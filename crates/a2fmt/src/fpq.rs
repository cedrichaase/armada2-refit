//! Fleet Operations: Roots' `.fpq` archive (`Data/ODF.fpq`): a reader.
//!
//! Layout (all integers little-endian), as `README.md` describes it:
//!
//! 1. a header of seven `u32`: the magic `FPQ\0`, the version (2), the folder count, the
//!    file count, the hash table's size (a power of two), the data blob's size and the
//!    names blob's size;
//! 2. the hash table: `hash_size` entries of 12 bytes (`hash_a`, `hash_b`, file index),
//!    an empty slot being all zero;
//! 3. the folder table: 12 bytes each (parent folder or `0xFFFFFFFF`, name offset, name
//!    length), names into the names blob;
//! 4. the file table: 25 bytes each (data offset, unpacked size, packed size, name
//!    offset, name length, method `u8`, folder index);
//! 5. the names blob, then the data blob.
//!
//! The five parts add up to the file's length exactly, which [`Archive::parse`] checks.
//! A file is stored as it is (method 0) or LZO1X-compressed (method 1, [`crate::lzo`]).
//! A name is found through the archive's string hash ([`hash`]) of the file's own name,
//! without its folders; the folders then say where it sits.

use alloc::string::String;
use alloc::vec::Vec;

use crate::lzo;

/// Why an archive or a file in it was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Not an `.fpq` file (the magic) or a version this reader does not know.
    Header,
    /// A count, size or offset that the file cannot hold.
    Size,
    /// A table entry that points outside its blob, a hash entry to no file, or a
    /// folder chain that loops.
    Table,
    /// A file stored by a method other than 0 and 1.
    Method(u8),
    /// The compressed data of a file did not decode to its size.
    Data(lzo::Error),
}

/// What the table says of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The file's own name, as the archive spells it.
    pub name: String,
    /// Index into [`Archive::folders`]; None for a file at the top.
    pub folder: Option<usize>,
    pub unpacked: u32,
    pub packed: u32,
    pub method: u8,
    offset: u32,
}

/// One folder: its name and its parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub parent: Option<usize>,
}

const MAGIC: u32 = u32::from_le_bytes(*b"FPQ\0");
const HEADER: usize = 28;
/// A deeper chain of folders is taken for a loop.
const DEPTH: usize = 64;

/// The archive's three string hashes of a name: the start slot (`Slot`), and the two
/// checks stored in a slot (`CheckA`, `CheckB`).
#[derive(Clone, Copy)]
pub enum Hash {
    Slot = 0,
    CheckA = 0x100,
    CheckB = 0x200,
}

/// The 0x500 words the hash draws on, from the generator seeded with 0x100001.
fn crypt_table() -> [u32; 0x500] {
    let mut t = [0u32; 0x500];
    let mut seed: u32 = 0x0010_0001;
    for i in 0..256 {
        let mut j = i;
        for _ in 0..5 {
            seed = (seed * 125 + 3) % 0x2A_AAAB;
            let hi = (seed & 0xFFFF) << 16;
            seed = (seed * 125 + 3) % 0x2A_AAAB;
            t[j] = hi | (seed & 0xFFFF);
            j += 256;
        }
    }
    t
}

/// The string hash of a name, upper-cased by ASCII. The slot is this hash of kind
/// [`Hash::Slot`] masked by the table size.
pub fn hash(name: &str, kind: Hash) -> u32 {
    hash_with(&crypt_table(), name, kind)
}

fn hash_with(t: &[u32; 0x500], name: &str, kind: Hash) -> u32 {
    let mut s1: u32 = 0x7FED_7FED;
    let mut s2: u32 = 0xEEEE_EEEE;
    for c in name.bytes() {
        let c = c.to_ascii_uppercase() as u32;
        s1 = t[kind as usize + c as usize] ^ s1.wrapping_add(s2);
        s2 = c.wrapping_add(s1).wrapping_add(s2).wrapping_add(s2 << 5).wrapping_add(3);
    }
    s1
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// An opened archive: the whole file in memory, its tables parsed and checked.
pub struct Archive {
    data: Vec<u8>,
    data_start: usize,
    crypt: [u32; 0x500],
    /// (hash_a, hash_b, file index); all zero is an empty slot.
    slots: Vec<(u32, u32, u32)>,
    folders: Vec<Folder>,
    files: Vec<Entry>,
}

impl Archive {
    /// Parses the tables of an archive held in `data`. Every offset and length is
    /// checked against the blob it points into, so [`Archive::read`] cannot go out of
    /// range afterwards.
    pub fn parse(data: Vec<u8>) -> Result<Archive, Error> {
        if data.len() < HEADER || u32_at(&data, 0) != MAGIC || u32_at(&data, 4) < 2 {
            return Err(Error::Header);
        }
        let n_folders = u32_at(&data, 8) as u64;
        let n_files = u32_at(&data, 12) as u64;
        let hash_size = u32_at(&data, 16) as u64;
        let data_size = u32_at(&data, 20) as u64;
        let names_size = u32_at(&data, 24) as u64;
        if hash_size == 0 || !hash_size.is_power_of_two() || hash_size < n_files {
            return Err(Error::Size);
        }
        // u64 sums of u32-sized parts cannot wrap; the whole must be the file.
        let total = HEADER as u64 + hash_size * 12 + n_folders * 12 + n_files * 25 + names_size + data_size;
        if total != data.len() as u64 {
            return Err(Error::Size);
        }
        let (n_folders, n_files, hash_size) = (n_folders as usize, n_files as usize, hash_size as usize);
        let hash_at = HEADER;
        let folder_at = hash_at + hash_size * 12;
        let file_at = folder_at + n_folders * 12;
        let names_at = file_at + n_files * 25;
        let data_start = names_at + names_size as usize;
        let names = &data[names_at..data_start];
        let blob = data_size as usize;

        let name_of = |off: u32, len: u32| -> Result<String, Error> {
            let end = (off as u64) + len as u64;
            let b = names.get(off as usize..).filter(|_| end <= names.len() as u64).ok_or(Error::Table)?;
            // Latin-1: a byte is the character of its value.
            Ok(b[..len as usize].iter().map(|&c| c as char).collect())
        };

        let mut folders = Vec::with_capacity(n_folders);
        for i in 0..n_folders {
            let at = folder_at + i * 12;
            let parent = u32_at(&data, at);
            let parent = if parent == u32::MAX { None } else { Some(parent as usize) };
            if parent.is_some_and(|p| p >= n_folders) {
                return Err(Error::Table);
            }
            folders.push(Folder { name: name_of(u32_at(&data, at + 4), u32_at(&data, at + 8))?, parent });
        }
        for i in 0..n_folders {
            let mut at = Some(i);
            let mut depth = 0;
            while let Some(f) = at {
                depth += 1;
                if depth > DEPTH {
                    return Err(Error::Table);
                }
                at = folders[f].parent;
            }
        }

        let mut files = Vec::with_capacity(n_files);
        for i in 0..n_files {
            let at = file_at + i * 25;
            let offset = u32_at(&data, at);
            let unpacked = u32_at(&data, at + 4);
            let packed = u32_at(&data, at + 8);
            let method = data[at + 20];
            let folder = u32_at(&data, at + 21);
            if offset as u64 + packed as u64 > blob as u64 {
                return Err(Error::Table);
            }
            let folder = if folder == u32::MAX { None } else { Some(folder as usize) };
            if folder.is_some_and(|f| f >= n_folders) {
                return Err(Error::Table);
            }
            files.push(Entry {
                name: name_of(u32_at(&data, at + 12), u32_at(&data, at + 16))?,
                folder,
                unpacked,
                packed,
                method,
                offset,
            });
        }

        let mut slots = Vec::with_capacity(hash_size);
        for i in 0..hash_size {
            let at = hash_at + i * 12;
            let s = (u32_at(&data, at), u32_at(&data, at + 4), u32_at(&data, at + 8));
            if (s.0 != 0 || s.1 != 0) && s.2 as usize >= n_files {
                return Err(Error::Table);
            }
            slots.push(s);
        }
        Ok(Archive { data, data_start, crypt: crypt_table(), slots, folders, files })
    }

    /// The folders, in the archive's order.
    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    /// The files, in the archive's order (the index [`Archive::read`] takes).
    pub fn entries(&self) -> &[Entry] {
        &self.files
    }

    /// A folder's names from the top down (`["odf", "system"]`).
    pub fn folder_names(&self, folder: usize) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        let mut at = Some(folder);
        while let Some(f) = at.and_then(|f| self.folders.get(f)) {
            names.push(&f.name);
            at = f.parent;
        }
        names.reverse();
        names
    }

    /// A folder's path from the top, `/`-joined (`odf/system`).
    pub fn folder_path(&self, folder: usize) -> String {
        let mut s = String::new();
        for n in self.folder_names(folder) {
            if !s.is_empty() {
                s.push('/');
            }
            s.push_str(n);
        }
        s
    }

    /// A file's path from the top, `/`-joined (`odf/system/x.odf`).
    pub fn path(&self, index: usize) -> Option<String> {
        let e = self.files.get(index)?;
        let mut s = e.folder.map(|f| self.folder_path(f)).unwrap_or_default();
        if !s.is_empty() {
            s.push('/');
        }
        s.push_str(&e.name);
        Some(s)
    }

    /// The index of the file a name or path names, or None. Only the last component is
    /// hashed (the archive keys files by their own names); a path's folders, when it
    /// has any, must be the end of the file's own folder path, compared without regard
    /// to case, so `x.odf`, `system/x.odf` and `odf\system\x.odf` find the same file.
    pub fn find(&self, path: &str) -> Option<usize> {
        let parts = crate::vfs::components(path)?;
        let (name, dirs) = parts.split_last()?;
        let a = hash_with(&self.crypt, name, Hash::CheckA);
        let b = hash_with(&self.crypt, name, Hash::CheckB);
        let mask = self.slots.len() - 1;
        let mut i = hash_with(&self.crypt, name, Hash::Slot) as usize & mask;
        for _ in 0..self.slots.len() {
            let (sa, sb, idx) = self.slots[i];
            if sa == 0 && sb == 0 {
                return None;
            }
            if sa == a && sb == b && self.has_folders(idx as usize, dirs) {
                return Some(idx as usize);
            }
            i = (i + 1) & mask;
        }
        None
    }

    /// Whether `dirs` is the end of the file's folder path.
    fn has_folders(&self, index: usize, dirs: &[&str]) -> bool {
        let mut at = self.files[index].folder;
        for d in dirs.iter().rev() {
            match at.map(|f| &self.folders[f]) {
                Some(f) if f.name.eq_ignore_ascii_case(d) => at = f.parent,
                _ => return false,
            }
        }
        true
    }

    /// A file's bytes, decompressed and checked against its unpacked size.
    pub fn read(&self, index: usize) -> Result<Vec<u8>, Error> {
        let e = self.files.get(index).ok_or(Error::Table)?;
        let start = self.data_start + e.offset as usize;
        // parse() checked offset + packed against the data blob.
        let packed = &self.data[start..start + e.packed as usize];
        match e.method {
            0 if e.packed == e.unpacked => Ok(packed.to_vec()),
            0 => Err(Error::Data(lzo::Error::Short)),
            1 => lzo::decompress(packed, e.unpacked as usize).map_err(Error::Data),
            m => Err(Error::Method(m)),
        }
    }

    /// The file at a path, or None when there is none or it does not decode.
    pub fn open(&self, path: &str) -> Option<Vec<u8>> {
        self.read(self.find(path)?).ok()
    }
}
