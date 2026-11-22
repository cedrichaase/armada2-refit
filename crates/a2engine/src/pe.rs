//! The identity of an executable (phase 8, contract P1): `TimeDateStamp`, `SizeOfImage`
//! and an FNV-1a 32 over every section header in table order (the 8 name bytes, then
//! `VirtualAddress`, `VirtualSize` and `SizeOfRawData`, little-endian). Read from the
//! main module's headers in memory at run time, from the file offline; the file's
//! sha256 is the bench's, never a run-time key.

/// A PE image to read headers from: in memory (`this_process`) or a buffer (host tests;
/// the headers sit first, in a file and in a mapped image alike).
pub trait Image {
    /// Fills `out` from `at`; false if the image does not reach that far.
    fn read(&self, at: usize, out: &mut [u8]) -> bool;
    /// The address the image is mapped at, 0 for a buffer.
    fn base(&self) -> usize {
        0
    }
    /// The module's file name, without its directory, into `out`: its length, 0 if the
    /// image has none (a buffer). The identity line names it on an unknown build.
    fn name(&self, _out: &mut [u8]) -> usize {
        0
    }
    /// The CRC-32 of section `name`'s raw data in the executable's FILE on disk (see
    /// `section_crc_file`), None if it cannot be read. FleetOpsHook rewrites `.text` and
    /// `.rdata` in memory before a plugin of ours runs, so a build that sets file CRCs
    /// is told by the file, never by the mapped image.
    fn file_crc(&self, _section: &str) -> Option<u32> {
        None
    }
    /// The `.text` CRC-32, from its file on disk, of the companion module `name` loaded
    /// in the process (a profile's `[[module]]`).
    fn companion(&self, _name: &str) -> Companion {
        Companion::NotLoaded
    }
}

/// What the process holds of a companion module (`Image::companion`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Companion {
    /// No module of that name is loaded.
    NotLoaded,
    /// Loaded, but its file cannot be read.
    Unreadable,
    /// Loaded: the CRC-32 of its `.text` as the file has it.
    Crc(u32),
}

/// A file or buffer, offsets from its start.
pub struct Bytes<'a>(pub &'a [u8]);

impl Image for Bytes<'_> {
    fn read(&self, at: usize, out: &mut [u8]) -> bool {
        match at.checked_add(out.len()) {
            Some(end) if end <= self.0.len() => {
                out.copy_from_slice(&self.0[at..end]);
                true
            }
            _ => false,
        }
    }
    fn file_crc(&self, section: &str) -> Option<u32> {
        section_crc_file(&mut |at, out| self.read(at, out), section)
    }
}

const CRC_NIBBLE: [u32; 16] = [
    0x0000_0000, 0x1db7_1064, 0x3b6e_20c8, 0x26d9_30ac, 0x76dc_4190, 0x6b6b_51f4, 0x4db2_6158, 0x5005_713c,
    0xedb8_8320, 0xf00f_9344, 0xd6d6_a3e8, 0xcb61_b38c, 0x9b64_c2b0, 0x86d3_d2d4, 0xa00a_e278, 0xbdbd_f21c,
];

/// CRC-32 (IEEE 802.3, the zlib one) running state: start at `!0`, finish with `!`.
pub fn crc32_update(mut c: u32, data: &[u8]) -> u32 {
    for &b in data {
        c ^= b as u32;
        c = (c >> 4) ^ CRC_NIBBLE[(c & 15) as usize];
        c = (c >> 4) ^ CRC_NIBBLE[(c & 15) as usize];
    }
    c
}

/// The CRC-32 of section `name` in a PE FILE read through `read(offset, buf)`: its raw
/// data at `PointerToRawData`, the first `min(VirtualSize, SizeOfRawData)` bytes (the
/// section's content without the file alignment's padding). In 4 KiB steps, so the file
/// is never held. None when the section is not there or the file ends early.
pub fn section_crc_file(read: &mut dyn FnMut(usize, &mut [u8]) -> bool, name: &str) -> Option<u32> {
    let rd32 = |read: &mut dyn FnMut(usize, &mut [u8]) -> bool, at: usize| {
        let mut b = [0u8; 4];
        read(at, &mut b).then(|| u32::from_le_bytes(b))
    };
    let rd16 = |read: &mut dyn FnMut(usize, &mut [u8]) -> bool, at: usize| {
        let mut b = [0u8; 2];
        read(at, &mut b).then(|| u16::from_le_bytes(b))
    };
    let want = name.as_bytes();
    if want.len() > 8 {
        return None;
    }
    let nt = rd32(read, 0x3c)? as usize;
    let count = rd16(read, nt + 6)? as usize;
    let table = nt + 24 + rd16(read, nt + 20)? as usize;
    if count > MAX_SECTIONS {
        return None;
    }
    for i in 0..count {
        let mut s = [0u8; 40];
        if !read(table + i * 40, &mut s) {
            return None;
        }
        let n = s[..8].iter().position(|&c| c == 0).unwrap_or(8);
        if &s[..n] != want {
            continue;
        }
        let word = |o: usize| u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]) as usize;
        let (vsize, raw, ptr) = (word(8), word(16), word(20));
        let (mut left, mut at, mut c) = (vsize.min(raw), ptr, !0u32);
        let mut buf = [0u8; 4096];
        while left > 0 {
            let k = left.min(buf.len());
            if !read(at, &mut buf[..k]) {
                return None;
            }
            c = crc32_update(c, &buf[..k]);
            at += k;
            left -= k;
        }
        return Some(!c);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    /// `IMAGE_FILE_HEADER.TimeDateStamp`.
    pub timestamp: u32,
    /// `OptionalHeader.SizeOfImage`.
    pub image_size: u32,
    /// FNV-1a 32 over the section headers (see the module comment).
    pub sections: u32,
    /// Where the image is mapped (0 for a buffer).
    pub base: usize,
    /// The first executable section, as `base + VirtualAddress` and its end.
    pub code: (usize, usize),
}

const MAX_SECTIONS: usize = 96;
const SCN_MEM_EXECUTE: u32 = 0x2000_0000;
const SCN_CNT_CODE: u32 = 0x0000_0020;

fn u16_at(m: &dyn Image, at: usize) -> Option<u16> {
    let mut b = [0u8; 2];
    m.read(at, &mut b).then(|| u16::from_le_bytes(b))
}

fn u32_at(m: &dyn Image, at: usize) -> Option<u32> {
    let mut b = [0u8; 4];
    m.read(at, &mut b).then(|| u32::from_le_bytes(b))
}

fn fnv(mut h: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// `identify` on a buffer: the offline reader.
pub fn identify_bytes(b: &[u8]) -> Option<Identity> {
    identify(&Bytes(b))
}

/// Reads the identity of the image, or None if it is not a PE the reader understands.
pub fn identify(mem: &dyn Image) -> Option<Identity> {
    if u16_at(mem, 0)? != 0x5a4d {
        return None;
    }
    let nt = u32_at(mem, 0x3c)? as usize;
    if u32_at(mem, nt)? != 0x0000_4550 {
        return None;
    }
    let count = u16_at(mem, nt + 6)? as usize;
    let timestamp = u32_at(mem, nt + 8)?;
    let opt_size = u16_at(mem, nt + 20)? as usize;
    let opt = nt + 24;
    // PE32 and PE32+ both keep SizeOfImage 56 bytes into the optional header.
    if !matches!(u16_at(mem, opt)?, 0x10b | 0x20b) {
        return None;
    }
    let image_size = u32_at(mem, opt + 56)?;
    if count == 0 || count > MAX_SECTIONS {
        return None;
    }
    let base = mem.base();
    let table = opt + opt_size;
    let mut h: u32 = 0x811c_9dc5;
    let mut code = None;
    for i in 0..count {
        let mut s = [0u8; 40];
        if !mem.read(table + i * 40, &mut s) {
            return None;
        }
        let word = |o: usize| u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
        let (vsize, va, raw, flags) = (word(8), word(12), word(16), word(36));
        h = fnv(h, &s[0..8]);
        h = fnv(h, &va.to_le_bytes());
        h = fnv(h, &vsize.to_le_bytes());
        h = fnv(h, &raw.to_le_bytes());
        if code.is_none() && flags & (SCN_MEM_EXECUTE | SCN_CNT_CODE) != 0 {
            code = Some((base + va as usize, base + va as usize + vsize as usize));
        }
    }
    Some(Identity { timestamp, image_size, sections: h, base, code: code.unwrap_or((0, 0)) })
}

/// The section named `name` (its 8 header bytes, NUL-padded): (RVA, VirtualSize).
pub fn section(mem: &dyn Image, name: &str) -> Option<(usize, usize)> {
    let nt = u32_at(mem, 0x3c)? as usize;
    let count = u16_at(mem, nt + 6)? as usize;
    let opt_size = u16_at(mem, nt + 20)? as usize;
    let table = nt + 24 + opt_size;
    let want = name.as_bytes();
    if want.len() > 8 || count > MAX_SECTIONS {
        return None;
    }
    for i in 0..count {
        let mut s = [0u8; 40];
        if !mem.read(table + i * 40, &mut s) {
            return None;
        }
        let n = s[..8].iter().position(|&c| c == 0).unwrap_or(8);
        if &s[..n] == want {
            let word = |o: usize| u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]) as usize;
            return Some((word(12), word(8)));
        }
    }
    None
}

/// The log line, identical in every plugin: `engine: <exe> ` + this, where `<exe>` is
/// the matched build's executable (build.toml `exe`), else the module's own file name
/// (`init_with` writes it; `is_identity_line` recognises it).
impl core::fmt::Display for Identity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "timestamp {:#010x}, image size {:#x}, sections {:#010x}",
            self.timestamp, self.image_size, self.sections
        )
    }
}

/// The main module (the game's executable) of this process, read from its mapped headers.
#[cfg(windows)]
pub fn this_process() -> Option<Identity> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleA(name: *const u8) -> usize;
    }
    struct Mapped(usize);
    impl Image for Mapped {
        fn read(&self, at: usize, out: &mut [u8]) -> bool {
            // The headers stay mapped for the life of the process, and `identify`
            // reads only the offsets the headers themselves name.
            unsafe { core::ptr::copy_nonoverlapping((self.0 + at) as *const u8, out.as_mut_ptr(), out.len()) };
            true
        }
        fn base(&self) -> usize {
            self.0
        }
    }
    let base = unsafe { GetModuleHandleA(core::ptr::null()) };
    if base == 0 {
        return None;
    }
    identify(&Mapped(base))
}
