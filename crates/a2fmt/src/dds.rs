//! DirectDraw Surface images, as Fleet Operations: Roots ships its textures.
//!
//! Only the base level is decoded: the engine's mip chains are either inside the file
//! (not read here) or named files beside it (`tga::TexIndex`). Compressed formats are
//! DXT1, DXT3 and DXT5; uncompressed ones are read by their channel masks (8, 16, 24
//! and 32 bits: A8R8G8B8, X8R8G8B8, A8B8G8R8, R8G8B8, R5G6B5, A1R5G5B5, X1R5G5B5,
//! A4R4G4B4, L8, A8L8 and the like). The result is the same BGRA8 top-down
//! [`Image`](crate::tga::Image) `tga::decode` returns. DX10 headers and any other
//! FourCC are refused.

use crate::tga::Image;
use alloc::vec::Vec;

/// Largest width or height accepted (Roots has 12600x280 sprite strips)...
pub const MAX_SIDE: u32 = 16384;
/// ...and the most pixels: 8192 x 8192.
pub const MAX_PIXELS: u64 = 8192 * 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Shorter than its header or its pixel data say.
    Truncated,
    /// No `DDS ` magic or a header of the wrong size.
    BadHeader,
    /// Zero width or height.
    Empty,
    /// Wider or taller than [`MAX_SIDE`], or more than [`MAX_PIXELS`].
    TooLarge,
    /// A `DX10` extension header.
    Dx10,
    /// A FourCC other than DXT1, DXT3, DXT5.
    UnsupportedFourCc(u32),
    /// An uncompressed format this reader does not take (bit count or masks).
    UnsupportedFormat,
    /// The pixel buffer could not be allocated.
    Alloc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Dxt1,
    Dxt3,
    Dxt5,
    /// Uncompressed, `bits` per pixel (8, 16, 24 or 32), channels by mask. A luminance
    /// format has its value in `r`, read as grey. A zero mask means the channel is
    /// absent (colour 0, alpha 255).
    Mask { bits: u8, r: u32, g: u32, b: u32, a: u32, luminance: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub width: u32,
    pub height: u32,
    /// Levels in the file (1 when the header does not say).
    pub mip_count: u32,
    pub format: Format,
}

const fn four_cc(s: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*s)
}

/// Offset of the base level's data.
const DATA: usize = 128;

impl Header {
    pub const SIZE: usize = DATA;

    pub fn parse(data: &[u8]) -> Result<Header, Error> {
        if data.len() < Self::SIZE {
            return Err(Error::Truncated);
        }
        let u32_at = |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        if &data[..4] != b"DDS " || u32_at(4) != 124 || u32_at(76) != 32 {
            return Err(Error::BadHeader);
        }
        let flags = u32_at(8);
        let (height, width) = (u32_at(12), u32_at(16));
        if width == 0 || height == 0 {
            return Err(Error::Empty);
        }
        if width > MAX_SIDE || height > MAX_SIDE || width as u64 * height as u64 > MAX_PIXELS {
            return Err(Error::TooLarge);
        }
        let mip_count = if flags & 0x2_0000 != 0 { u32_at(28).clamp(1, 32) } else { 1 };
        let pf_flags = u32_at(80);
        let format = if pf_flags & 4 != 0 {
            match u32_at(84) {
                f if f == four_cc(b"DXT1") => Format::Dxt1,
                f if f == four_cc(b"DXT3") => Format::Dxt3,
                f if f == four_cc(b"DXT5") => Format::Dxt5,
                f if f == four_cc(b"DX10") => return Err(Error::Dx10),
                f => return Err(Error::UnsupportedFourCc(f)),
            }
        } else if pf_flags & (0x40 | 0x2_0000 | 0x2) != 0 {
            let bits = u32_at(88);
            if !matches!(bits, 8 | 16 | 24 | 32) {
                return Err(Error::UnsupportedFormat);
            }
            let luminance = pf_flags & 0x2_0000 != 0;
            let alpha = pf_flags & (1 | 2) != 0;
            let (r, g, b) = (u32_at(92), u32_at(96), u32_at(100));
            let a = if alpha { u32_at(104) } else { 0 };
            let used = (r | g | b | a) as u64;
            if used >> bits != 0 {
                return Err(Error::UnsupportedFormat);
            }
            Format::Mask { bits: bits as u8, r, g, b, a, luminance }
        } else {
            return Err(Error::UnsupportedFormat);
        };
        Ok(Header { width, height, mip_count, format })
    }
}

/// A channel of a masked pixel, scaled to 8 bits.
#[derive(Clone, Copy)]
struct Chan {
    mask: u32,
    shift: u32,
    bits: u32,
}

impl Chan {
    fn new(mask: u32) -> Chan {
        if mask == 0 {
            return Chan { mask: 0, shift: 0, bits: 0 };
        }
        let shift = mask.trailing_zeros();
        Chan { mask, shift, bits: (mask >> shift).trailing_ones() }
    }

    fn get(&self, px: u32, absent: u8) -> u8 {
        if self.bits == 0 {
            return absent;
        }
        let v = (px & self.mask) >> self.shift;
        if self.bits >= 8 {
            (v >> (self.bits - 8)) as u8
        } else {
            let max = (1u32 << self.bits) - 1;
            ((v * 255 + max / 2) / max) as u8
        }
    }
}

/// Decode the base level to BGRA8, rows top-down.
pub fn decode(data: &[u8]) -> Result<Image, Error> {
    let h = Header::parse(data)?;
    let (w, ht) = (h.width as usize, h.height as usize);
    let src = &data[DATA..];
    // The data is measured before anything is allocated, so a header that claims
    // 8192x8192 over a few bytes costs nothing.
    let (block, need) = match h.format {
        Format::Dxt1 | Format::Dxt3 | Format::Dxt5 => {
            let block = if h.format == Format::Dxt1 { 8 } else { 16 };
            (block, (w.div_ceil(4) * ht.div_ceil(4)).saturating_mul(block))
        }
        Format::Mask { bits, .. } => (bits as usize / 8, (w * ht).saturating_mul(bits as usize / 8)),
    };
    let src = src.get(..need).ok_or(Error::Truncated)?;
    let out_len = w * ht * 4;
    let mut pixels: Vec<u8> = Vec::new();
    pixels.try_reserve_exact(out_len).map_err(|_| Error::Alloc)?;
    pixels.resize(out_len, 0);

    match h.format {
        Format::Dxt1 | Format::Dxt3 | Format::Dxt5 => {
            let bw = w.div_ceil(4);
            for (n, b) in src.chunks_exact(block).enumerate() {
                let (bx, by) = (n % bw, n / bw);
                let mut tile = [[0u8; 4]; 16];
                let (alpha, colour) = b.split_at(block - 8);
                colour_block(colour, h.format == Format::Dxt1, &mut tile);
                match h.format {
                    Format::Dxt3 => {
                        for (i, t) in tile.iter_mut().enumerate() {
                            t[3] = ((alpha[i / 2] >> ((i & 1) * 4)) & 15) * 17;
                        }
                    }
                    Format::Dxt5 => dxt5_alpha(alpha, &mut tile),
                    _ => {}
                }
                for (i, px) in tile.iter().enumerate() {
                    let (x, y) = (bx * 4 + (i & 3), by * 4 + (i >> 2));
                    if x < w && y < ht {
                        pixels[(y * w + x) * 4..][..4].copy_from_slice(px);
                    }
                }
            }
        }
        Format::Mask { r, g, b, a, luminance, .. } => {
            let (cr, cg, cb, ca) = (Chan::new(r), Chan::new(g), Chan::new(b), Chan::new(a));
            for (s, d) in src.chunks_exact(block).zip(pixels.chunks_exact_mut(4)) {
                let mut px = 0u32;
                for (i, &byte) in s.iter().enumerate() {
                    px |= (byte as u32) << (8 * i);
                }
                let red = cr.get(px, 0);
                let (green, blue) = if luminance { (red, red) } else { (cg.get(px, 0), cb.get(px, 0)) };
                d.copy_from_slice(&[blue, green, red, ca.get(px, 255)]);
            }
        }
    }
    let bits_per_pixel = match h.format {
        Format::Dxt1 | Format::Dxt3 | Format::Dxt5 => 32,
        Format::Mask { a, .. } if a != 0 => 32,
        Format::Mask { bits, .. } => bits.max(24),
    };
    Ok(Image { width: h.width, height: h.height, bits_per_pixel, pixels })
}

fn rgb565(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) as u32 & 31, (c >> 5) as u32 & 63, c as u32 & 31);
    [((r * 255 + 15) / 31) as u8, ((g * 255 + 31) / 63) as u8, ((b * 255 + 15) / 31) as u8]
}

/// The 8-byte colour block: two RGB565 endpoints and 2-bit indices. `dxt1` allows the
/// 3-colour mode with transparent black; otherwise the 4-colour mode always applies.
fn colour_block(b: &[u8], dxt1: bool, tile: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mut pal = [[0u8, 0, 0, 255]; 4];
    let mix = |a: u8, b: u8, wa: u32, wb: u32, d: u32| ((a as u32 * wa + b as u32 * wb) / d) as u8;
    for k in 0..3 {
        pal[0][k] = p0[k];
        pal[1][k] = p1[k];
        if !dxt1 || c0 > c1 {
            pal[2][k] = mix(p0[k], p1[k], 2, 1, 3);
            pal[3][k] = mix(p0[k], p1[k], 1, 2, 3);
        } else {
            pal[2][k] = mix(p0[k], p1[k], 1, 1, 2);
        }
    }
    if dxt1 && c0 <= c1 {
        pal[3] = [0, 0, 0, 0];
    }
    let idx = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    for (i, t) in tile.iter_mut().enumerate() {
        let p = pal[(idx >> (2 * i)) as usize & 3];
        // Palette entries are RGB; the image is BGRA.
        *t = [p[2], p[1], p[0], p[3]];
    }
}

fn dxt5_alpha(b: &[u8], tile: &mut [[u8; 4]; 16]) {
    let (a0, a1) = (b[0] as u32, b[1] as u32);
    let mut pal = [0u8; 8];
    pal[0] = a0 as u8;
    pal[1] = a1 as u8;
    if a0 > a1 {
        for k in 1..7u32 {
            pal[k as usize + 1] = (((7 - k) * a0 + k * a1) / 7) as u8;
        }
    } else {
        for k in 1..5u32 {
            pal[k as usize + 1] = (((5 - k) * a0 + k * a1) / 5) as u8;
        }
        pal[6] = 0;
        pal[7] = 255;
    }
    let mut bits = 0u64;
    for (i, &v) in b[2..8].iter().enumerate() {
        bits |= (v as u64) << (8 * i);
    }
    for (i, t) in tile.iter_mut().enumerate() {
        t[3] = pal[(bits >> (3 * i)) as usize & 7];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn header(w: u32, h: u32, pf_flags: u32, fourcc: &[u8; 4], bits: u32, m: [u32; 4]) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[..4].copy_from_slice(b"DDS ");
        let mut put = |at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(4, 124);
        put(8, 0x1007 | 0x2_0000);
        put(12, h);
        put(16, w);
        put(28, 3);
        put(76, 32);
        put(80, pf_flags);
        put(84, u32::from_le_bytes(*fourcc));
        put(88, bits);
        for (i, v) in m.iter().enumerate() {
            put(92 + 4 * i, *v);
        }
        d
    }

    fn dxt(w: u32, h: u32, cc: &[u8; 4], blocks: &[u8]) -> Vec<u8> {
        let mut d = header(w, h, 4, cc, 0, [0; 4]);
        d.extend_from_slice(blocks);
        d
    }

    // Red (0xF800) and blue (0x001F) endpoints, little endian.
    const RB: [u8; 4] = [0x00, 0xF8, 0x1F, 0x00];

    #[test]
    fn dxt1_four_colour_and_one_bit_alpha() {
        let mut b = RB.to_vec();
        b.extend_from_slice(&[0b11_10_01_00, 0, 0, 0]);
        let img = decode(&dxt(4, 4, b"DXT1", &b)).unwrap();
        assert_eq!(img.pixel(0, 0), [0, 0, 255, 255]);
        assert_eq!(img.pixel(1, 0), [255, 0, 0, 255]);
        assert_eq!(img.pixel(2, 0), [85, 0, 170, 255]);
        assert_eq!(img.pixel(3, 0), [170, 0, 85, 255]);
        assert_eq!(img.bits_per_pixel, 32);
        // c0 <= c1: index 2 is the midpoint, 3 is transparent black.
        let mut b = vec![RB[2], RB[3], RB[0], RB[1]];
        b.extend_from_slice(&[0b11_10_00_00, 0, 0, 0]);
        let img = decode(&dxt(4, 4, b"DXT1", &b)).unwrap();
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.pixel(2, 0), [127, 0, 127, 255]);
        assert_eq!(img.pixel(3, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn dxt3_explicit_alpha() {
        let mut b = vec![0u8; 8];
        b[0] = 0xF0; // pixel 0: 0, pixel 1: 15
        b[7] = 0x80; // pixel 15: 8
        b.extend_from_slice(&RB);
        b.extend_from_slice(&[0; 4]);
        let img = decode(&dxt(4, 4, b"DXT3", &b)).unwrap();
        assert_eq!(img.pixel(0, 0), [0, 0, 255, 0]);
        assert_eq!(img.pixel(1, 0)[3], 255);
        assert_eq!(img.pixel(3, 3)[3], 136);
    }

    #[test]
    fn dxt5_both_alpha_modes() {
        let mut b = vec![255, 0, 0, 0, 0, 0, 0, 0];
        // indices: pixel 0 = 0 (a0), pixel 1 = 1 (a1), pixel 2 = 2, pixel 3 = 7
        let bits: u64 = 1 << 3 | 2 << 6 | 7 << 9;
        b[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
        b.extend_from_slice(&RB);
        b.extend_from_slice(&[0; 4]);
        let img = decode(&dxt(4, 4, b"DXT5", &b)).unwrap();
        assert_eq!(
            [img.pixel(0, 0)[3], img.pixel(1, 0)[3], img.pixel(2, 0)[3], img.pixel(3, 0)[3]],
            [255, 0, 218, 36]
        );
        // a0 <= a1: 6-value mode, index 6 = 0, index 7 = 255.
        b[0] = 0;
        b[1] = 100;
        let bits: u64 = 2 << 6 | 6 << 9;
        b[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
        let img = decode(&dxt(4, 4, b"DXT5", &b)).unwrap();
        assert_eq!(img.pixel(2, 0)[3], 20);
        assert_eq!(img.pixel(3, 0)[3], 0);
        b[2..8].copy_from_slice(&(7u64 << 9).to_le_bytes()[..6]);
        assert_eq!(decode(&dxt(4, 4, b"DXT5", &b)).unwrap().pixel(3, 0)[3], 255);
    }

    #[test]
    fn partial_blocks_and_odd_sizes() {
        // 6x5: 2x2 blocks. Left blocks all red, right blocks all blue (index 1).
        let red = [RB[0], RB[1], RB[2], RB[3], 0, 0, 0, 0];
        let blue = [RB[0], RB[1], RB[2], RB[3], 0x55, 0x55, 0x55, 0x55];
        let mut b = Vec::new();
        for _ in 0..2 {
            b.extend_from_slice(&red);
            b.extend_from_slice(&blue);
        }
        let img = decode(&dxt(6, 5, b"DXT1", &b)).unwrap();
        assert_eq!((img.width, img.height, img.pixels.len()), (6, 5, 6 * 5 * 4));
        assert_eq!(img.pixel(3, 4), [0, 0, 255, 255]);
        assert_eq!(img.pixel(5, 4), [255, 0, 0, 255]);
        assert_eq!(img.pixel(0, 4), [0, 0, 255, 255]);
        // 768x768: not a power of two.
        let blocks = vec![0u8; 192 * 192 * 8];
        let img = decode(&dxt(768, 768, b"DXT1", &blocks)).unwrap();
        assert_eq!((img.width, img.height), (768, 768));
        assert_eq!(decode(&dxt(768, 768, b"DXT1", &blocks[1..])), Err(Error::Truncated));
    }

    #[test]
    fn uncompressed() {
        let mut d = header(2, 1, 0x41, b"\0\0\0\0", 32, [0xFF0000, 0xFF00, 0xFF, 0xFF00_0000]);
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let img = decode(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [1, 2, 3, 4]);
        assert_eq!(img.pixel(1, 0), [5, 6, 7, 8]);
        // A8B8G8R8 swaps red and blue.
        let mut d = header(1, 1, 0x41, b"\0\0\0\0", 32, [0xFF, 0xFF00, 0xFF0000, 0xFF00_0000]);
        d.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [3, 2, 1, 4]);
        // R5G6B5: pure green, then white.
        let mut d = header(2, 1, 0x40, b"\0\0\0\0", 16, [0xF800, 0x07E0, 0x1F, 0]);
        d.extend_from_slice(&[0xE0, 0x07, 0xFF, 0xFF]);
        let img = decode(&d).unwrap();
        assert_eq!(img.pixel(0, 0), [0, 255, 0, 255]);
        assert_eq!(img.pixel(1, 0), [255, 255, 255, 255]);
        assert_eq!(img.bits_per_pixel, 24);
        // R8G8B8.
        let mut d = header(1, 1, 0x40, b"\0\0\0\0", 24, [0xFF0000, 0xFF00, 0xFF, 0]);
        d.extend_from_slice(&[10, 20, 30]);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [10, 20, 30, 255]);
        // A4R4G4B4: blue 15, alpha 8.
        let mut d = header(1, 1, 0x41, b"\0\0\0\0", 16, [0xF00, 0xF0, 0xF, 0xF000]);
        d.extend_from_slice(&[0x0F, 0x80]);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [255, 0, 0, 136]);
        // A1R5G5B5: alpha clear, red full.
        let mut d = header(1, 1, 0x41, b"\0\0\0\0", 16, [0x7C00, 0x3E0, 0x1F, 0x8000]);
        d.extend_from_slice(&[0x00, 0x7C]);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [0, 0, 255, 0]);
        // L8 and A8L8.
        let mut d = header(1, 1, 0x2_0000, b"\0\0\0\0", 8, [0xFF, 0, 0, 0]);
        d.push(77);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [77, 77, 77, 255]);
        let mut d = header(1, 1, 0x2_0001, b"\0\0\0\0", 16, [0xFF, 0, 0, 0xFF00]);
        d.extend_from_slice(&[77, 5]);
        assert_eq!(decode(&d).unwrap().pixel(0, 0), [77, 77, 77, 5]);
    }

    #[test]
    fn errors_never_panic() {
        let mut b = RB.to_vec();
        b.extend_from_slice(&[0; 4]);
        let d = dxt(4, 4, b"DXT1", &b);
        assert!(decode(&d).is_ok());
        for n in 0..d.len() {
            assert!(decode(&d[..n]).is_err());
        }
        assert_eq!(Header::parse(&d).unwrap().mip_count, 3);
        let dx10 = header(4, 4, 4, b"DX10", 0, [0; 4]);
        assert_eq!(Header::parse(&dx10), Err(Error::Dx10));
        assert!(matches!(
            Header::parse(&header(4, 4, 4, b"ATI2", 0, [0; 4])),
            Err(Error::UnsupportedFourCc(_))
        ));
        assert_eq!(Header::parse(&header(0, 4, 4, b"DXT1", 0, [0; 4])), Err(Error::Empty));
        assert_eq!(Header::parse(&header(20000, 4, 4, b"DXT1", 0, [0; 4])), Err(Error::TooLarge));
        assert_eq!(Header::parse(&header(16384, 16384, 4, b"DXT1", 0, [0; 4])), Err(Error::TooLarge));
        assert!(Header::parse(&header(12600, 280, 4, b"DXT1", 0, [0; 4])).is_ok());
        assert_eq!(
            Header::parse(&header(4, 4, 0x40, b"\0\0\0\0", 12, [0; 4])),
            Err(Error::UnsupportedFormat)
        );
        let mut bad = d.clone();
        bad[0] = b'X';
        assert_eq!(decode(&bad), Err(Error::BadHeader));
        // A huge claimed size with no data fails before anything is allocated.
        assert_eq!(decode(&dxt(8192, 8192, b"DXT5", &[])), Err(Error::Truncated));
    }
}
