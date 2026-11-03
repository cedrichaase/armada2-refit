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

/// The log line, identical in every plugin: `engine: ` + this. The name is fixed on
/// purpose: it is the line's key, not a reading of the file.
impl core::fmt::Display for Identity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Armada2.exe timestamp {:#010x}, image size {:#x}, sections {:#010x}",
            self.timestamp, self.image_size, self.sections
        )
    }
}

/// The main module (`Armada2.exe`) of this process, read from its mapped headers.
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
