//! Targa images, and the named mip chains stock ships beside them.
//!
//! Every stock texture is image type 2 (uncompressed true colour), 24 or 32 bits, with
//! no ID field and no colour map; the rows run bottom-up or top-down by the origin bit
//! of the descriptor byte, and stock mixes both, even within one skybox set. This
//! decoder also takes the run-length type 10, 8-bit greyscale (types 3 and 11) and 15/16
//! bits, which stock does not use, so a replacement written by another tool still
//! loads. Colour-mapped images are refused.

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Shorter than its header says.
    Truncated,
    /// A colour-mapped or unknown image type.
    UnsupportedType(u8),
    UnsupportedDepth(u8),
    /// Zero width or height.
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub id_length: u8,
    pub colour_map_type: u8,
    pub image_type: u8,
    pub colour_map_length: u16,
    pub colour_map_entry_bits: u8,
    pub width: u16,
    pub height: u16,
    pub bits_per_pixel: u8,
    /// Bits 0-3: alpha bits; bit 4: right-to-left; bit 5: top-down.
    pub descriptor: u8,
}

impl Header {
    pub const SIZE: usize = 18;

    pub fn parse(data: &[u8]) -> Result<Header, Error> {
        if data.len() < Self::SIZE {
            return Err(Error::Truncated);
        }
        let u16_at = |i: usize| u16::from_le_bytes([data[i], data[i + 1]]);
        Ok(Header {
            id_length: data[0],
            colour_map_type: data[1],
            image_type: data[2],
            colour_map_length: u16_at(5),
            colour_map_entry_bits: data[7],
            width: u16_at(12),
            height: u16_at(14),
            bits_per_pixel: data[16],
            descriptor: data[17],
        })
    }

    pub fn top_down(&self) -> bool {
        self.descriptor & 0x20 != 0
    }

    pub fn right_to_left(&self) -> bool {
        self.descriptor & 0x10 != 0
    }

    pub fn alpha_bits(&self) -> u8 {
        self.descriptor & 0x0f
    }

    pub fn is_rle(&self) -> bool {
        self.image_type & 8 != 0
    }

    /// Where the pixel data starts (after the ID field and any colour map).
    pub fn data_offset(&self) -> usize {
        let cmap = if self.colour_map_type != 0 {
            self.colour_map_length as usize * (self.colour_map_entry_bits as usize).div_ceil(8)
        } else {
            0
        };
        Self::SIZE + self.id_length as usize + cmap
    }
}

/// A decoded image: BGRA8, rows top-down, `width * height * 4` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// The file's depth: 24-bit files decode with alpha 255.
    pub bits_per_pixel: u8,
    pub pixels: Vec<u8>,
}

impl Image {
    pub fn has_alpha_channel(&self) -> bool {
        self.bits_per_pixel == 32
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2], self.pixels[i + 3]]
    }
}

pub fn decode(data: &[u8]) -> Result<Image, Error> {
    let h = Header::parse(data)?;
    let grey = match h.image_type {
        2 | 10 => false,
        3 | 11 => true,
        t => return Err(Error::UnsupportedType(t)),
    };
    let bpp = h.bits_per_pixel;
    let ok_depth = if grey { bpp == 8 } else { matches!(bpp, 15 | 16 | 24 | 32) };
    if !ok_depth {
        return Err(Error::UnsupportedDepth(bpp));
    }
    let (w, ht) = (h.width as usize, h.height as usize);
    if w == 0 || ht == 0 {
        return Err(Error::Empty);
    }
    let bytes_pp = (bpp as usize).div_ceil(8);
    let start = h.data_offset();
    if start > data.len() {
        return Err(Error::Truncated);
    }
    let src = &data[start..];
    let count = w * ht;

    // Raw pixels in file order, still in the file's bytes per pixel.
    let raw: Vec<u8>;
    let file_pixels: &[u8] = if h.is_rle() {
        // A run packet of 1 + bpp bytes expands to at most 128 pixels: refuse a header
        // the data cannot fill before allocating for it.
        if count / 128 > src.len() {
            return Err(Error::Truncated);
        }
        let mut out = Vec::with_capacity(count * bytes_pp);
        let mut i = 0;
        while out.len() < count * bytes_pp {
            let Some(&c) = src.get(i) else { return Err(Error::Truncated) };
            i += 1;
            let n = (c & 0x7f) as usize + 1;
            if c & 0x80 != 0 {
                let px = src.get(i..i + bytes_pp).ok_or(Error::Truncated)?;
                i += bytes_pp;
                for _ in 0..n {
                    out.extend_from_slice(px);
                }
            } else {
                let px = src.get(i..i + n * bytes_pp).ok_or(Error::Truncated)?;
                i += n * bytes_pp;
                out.extend_from_slice(px);
            }
        }
        // A run may cross the image's end; the excess is dropped.
        out.truncate(count * bytes_pp);
        raw = out;
        &raw
    } else {
        src.get(..count * bytes_pp).ok_or(Error::Truncated)?
    };

    let mut pixels = alloc::vec![0u8; count * 4];
    for fy in 0..ht {
        let y = if h.top_down() { fy } else { ht - 1 - fy };
        for fx in 0..w {
            let x = if h.right_to_left() { w - 1 - fx } else { fx };
            let s = &file_pixels[(fy * w + fx) * bytes_pp..][..bytes_pp];
            let px = match bpp {
                8 => [s[0], s[0], s[0], 255],
                15 | 16 => {
                    let v = u16::from_le_bytes([s[0], s[1]]);
                    let ex = |c: u16| ((c << 3) | (c >> 2)) as u8;
                    let a = if bpp == 16 && h.alpha_bits() > 0 && v & 0x8000 == 0 { 0 } else { 255 };
                    [ex(v & 31), ex((v >> 5) & 31), ex((v >> 10) & 31), a]
                }
                24 => [s[0], s[1], s[2], 255],
                _ => [s[0], s[1], s[2], s[3]],
            };
            pixels[(y * w + x) * 4..][..4].copy_from_slice(&px);
        }
    }
    Ok(Image { width: w as u32, height: ht as u32, bits_per_pixel: bpp, pixels })
}

/// One texture file in a directory listing.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    stem_lower: String,
    file: String,
}

/// An index over the file names of a texture directory (`Textures/RGB`), for looking
/// textures up the way the engine does: by name, case-insensitively, with either
/// extension case, and for finding the hand-authored mip levels named beside a base.
///
/// It holds names only. Widths come from a closure the caller supplies (reading the
/// 18-byte header, say), so a directory of 2,000 files is not opened to build it.
#[derive(Clone, Debug, Default)]
pub struct TexIndex {
    entries: Vec<Entry>,
}

impl TexIndex {
    /// From file names. Anything not ending in `.tga` (any case) is ignored, so a raw
    /// directory listing can be passed, `.a2neb-backup` files and all.
    pub fn new<I, S>(names: I) -> TexIndex
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut entries: Vec<Entry> = names
            .into_iter()
            .filter_map(|n| {
                let n = n.as_ref();
                if n.len() > 4 && n.get(n.len() - 4..).is_some_and(|e| e.eq_ignore_ascii_case(".tga")) {
                    Some(Entry {
                        stem_lower: n[..n.len() - 4].to_ascii_lowercase(),
                        file: String::from(n),
                    })
                } else {
                    None
                }
            })
            .collect();
        entries.sort_by(|a, b| a.stem_lower.cmp(&b.stem_lower).then(a.file.cmp(&b.file)));
        entries.dedup_by(|a, b| a.stem_lower == b.stem_lower);
        TexIndex { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The file name for a texture name (with or without an extension), any case.
    pub fn find(&self, name: &str) -> Option<&str> {
        let stem = match name.len() {
            n if n > 4 && name.get(n - 4..).is_some_and(|e| e.eq_ignore_ascii_case(".tga")) => {
                &name[..n - 4]
            }
            _ => name,
        };
        let want = stem.to_ascii_lowercase();
        self.entries
            .binary_search_by(|e| e.stem_lower.as_str().cmp(want.as_str()))
            .ok()
            .map(|i| self.entries[i].file.as_str())
    }

    /// Level `level` (1 = half size) of the chain below `base`: the sibling named
    /// `<base>_<level>` or else `<base><level>`, accepted only if its width is the
    /// base's width shifted right by `level`. `width_of` is given a file name from
    /// this index and returns its width. This is `mip_name()` of
    /// `textures/lib/common.sh`; the width test is what tells a real level from a
    /// texture that merely has a digit on its name (`fdestroy2` is another ship, not
    /// a level of `fdestroy`) or a stray placeholder.
    pub fn mip_level<F>(&self, base: &str, level: u32, mut width_of: F) -> Option<&str>
    where
        F: FnMut(&str) -> Option<u32>,
    {
        let base_file = self.find(base)?;
        let bw = width_of(base_file)?;
        if level == 0 || level >= 32 {
            return None;
        }
        let want = bw >> level;
        let stem = &base_file[..base_file.len() - 4];
        for sep in ["_", ""] {
            let mut cand = String::from(stem);
            cand.push_str(sep);
            push_u32(&mut cand, level);
            if let Some(f) = self.find(&cand) {
                if width_of(f) == Some(want) {
                    return Some(f);
                }
            }
        }
        None
    }

    /// The whole chain below `base`: levels 1, 2, ... while each resolves, at most 12.
    pub fn mip_chain<F>(&self, base: &str, mut width_of: F) -> Vec<&str>
    where
        F: FnMut(&str) -> Option<u32>,
    {
        let mut out = Vec::new();
        for level in 1..=12 {
            match self.mip_level(base, level, &mut width_of) {
                Some(f) => out.push(f),
                None => break,
            }
        }
        out
    }

    /// Siblings named like levels of `base` that are not levels of its chain (stock's
    /// placeholders, such as a level the same size as its base). The `_N` spelling is
    /// always checked; the bare `N` spelling only when level 1 is itself spelled bare,
    /// since otherwise `<base><digit>` is simply another texture. `mip_strays()` of
    /// `textures/lib/common.sh`.
    pub fn mip_strays<F>(&self, base: &str, mut width_of: F) -> Vec<&str>
    where
        F: FnMut(&str) -> Option<u32>,
    {
        let Some(base_file) = self.find(base) else { return Vec::new() };
        let stem = base_file[..base_file.len() - 4].to_ascii_lowercase();
        let chain = self.mip_chain(base, &mut width_of);
        let bare = chain
            .first()
            .map(|f| !f[..f.len() - 4].to_ascii_lowercase().ends_with("_1"))
            .unwrap_or(false);
        let mut out = Vec::new();
        for e in &self.entries {
            let Some(rest) = e.stem_lower.strip_prefix(stem.as_str()) else { continue };
            let digit = |s: &str| s.len() == 1 && s.as_bytes()[0].is_ascii_digit();
            let named = match rest.strip_prefix('_') {
                Some(d) => digit(d),
                None => bare && digit(rest),
            };
            if named && !chain.iter().any(|c| *c == e.file.as_str()) {
                out.push(e.file.as_str());
            }
        }
        out
    }
}

fn push_u32(s: &mut String, mut v: u32) {
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    for &b in &buf[i..] {
        s.push(b as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn header(t: u8, w: u16, h: u16, bpp: u8, desc: u8) -> Vec<u8> {
        let mut d = vec![0u8; 18];
        d[2] = t;
        d[12..14].copy_from_slice(&w.to_le_bytes());
        d[14..16].copy_from_slice(&h.to_le_bytes());
        d[16] = bpp;
        d[17] = desc;
        d
    }

    #[test]
    fn origin_is_honoured() {
        // 1x2, 24-bit: first stored row is the bottom one when bit 5 is clear.
        let mut d = header(2, 1, 2, 24, 0);
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        let img = decode(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [4, 5, 6, 255]);
        assert_eq!(img.pixel(0, 1), [1, 2, 3, 255]);
        d[17] = 0x20;
        let img = decode(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [1, 2, 3, 255]);
    }

    #[test]
    fn rle_and_errors() {
        let mut d = header(10, 3, 1, 32, 0x28);
        d.extend_from_slice(&[0x81, 9, 8, 7, 6, 0x00, 1, 2, 3, 4]);
        let img = decode(&d).unwrap();
        assert_eq!(img.pixel(1, 0), [9, 8, 7, 6]);
        assert_eq!(img.pixel(2, 0), [1, 2, 3, 4]);
        for n in 0..d.len() {
            assert!(decode(&d[..n]).is_err());
        }
        assert_eq!(decode(&header(1, 1, 1, 8, 0)), Err(Error::UnsupportedType(1)));
    }

    #[test]
    fn mip_names() {
        let idx = TexIndex::new([
            "fcruise1_B.tga", "fcruise1_B1.TGA", "fcruise1_B2.tga", "Fsensor_B.tga",
            "Fsensor_B_1.tga", "Fsensor_B_2.tga", "fdestroy.tga", "fdestroy2.tga",
            "fdestroy_1.tga", "notes.txt",
        ]);
        let w = |f: &str| -> Option<u32> {
            Some(match f {
                "fcruise1_B.tga" => 256,
                "fcruise1_B1.TGA" => 128,
                "fcruise1_B2.tga" => 64,
                "Fsensor_B.tga" | "Fsensor_B_1.tga" | "Fsensor_B_2.tga" => 128,
                "fdestroy.tga" | "fdestroy2.tga" => 256,
                "fdestroy_1.tga" => 128,
                _ => return None,
            })
        };
        assert_eq!(idx.find("FCRUISE1_b"), Some("fcruise1_B.tga"));
        assert_eq!(idx.mip_chain("fcruise1_B", w), ["fcruise1_B1.TGA", "fcruise1_B2.tga"]);
        assert!(idx.mip_chain("Fsensor_B", w).is_empty());
        assert_eq!(idx.mip_strays("Fsensor_B", w), ["Fsensor_B_1.tga", "Fsensor_B_2.tga"]);
        assert_eq!(idx.mip_chain("fdestroy", w), ["fdestroy_1.tga"]);
        assert!(idx.mip_strays("fdestroy", w).is_empty());
    }
}
