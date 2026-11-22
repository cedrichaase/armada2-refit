//! PNG images (as Fleet Operations: Roots ships its form bitmaps), decoded to BGRA8, rows
//! top-down, straight (not premultiplied) alpha. Written here because the one use is a
//! title picture and a dependency for it would be larger than this file: non-interlaced
//! images of 8 or 16 bits per channel (greyscale, RGB, palette, with alpha or a `tRNS`
//! entry). The zlib stream is inflated by the small decoder below (stored, fixed and
//! dynamic blocks); the Adler-32 and CRCs are not checked, since every read is bounded.
//! No input panics: a bad file is an `Error`.
use crate::tga::Image;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotPng,
    Truncated,
    /// Interlaced, or a bit depth below 8.
    Unsupported,
    /// The deflate stream is damaged.
    Inflate,
    Empty,
}

pub fn is_png(data: &[u8]) -> bool {
    data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
}

impl Bits<'_> {
    fn need(&mut self, k: u32) -> Result<(), Error> {
        while self.n < k {
            let b = *self.d.get(self.pos).ok_or(Error::Inflate)?;
            self.pos += 1;
            self.acc |= (b as u32) << self.n;
            self.n += 8;
        }
        Ok(())
    }
    fn get(&mut self, k: u32) -> Result<u32, Error> {
        if k == 0 {
            return Ok(0);
        }
        self.need(k)?;
        let v = self.acc & ((1u32 << k) - 1);
        self.acc >>= k;
        self.n -= k;
        Ok(v)
    }
}

/// A canonical Huffman table: counts per length and the symbols in code order.
struct Huff {
    count: [u16; 16],
    sym: Vec<u16>,
}

impl Huff {
    fn new(lens: &[u8]) -> Huff {
        let mut count = [0u16; 16];
        for &l in lens {
            count[l as usize] += 1;
        }
        count[0] = 0;
        let mut offs = [0u16; 16];
        for i in 1..16 {
            offs[i] = offs[i - 1] + count[i - 1];
        }
        let mut sym = vec![0u16; lens.len()];
        for (s, &l) in lens.iter().enumerate() {
            if l != 0 {
                sym[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Huff { count, sym }
    }
    fn decode(&self, b: &mut Bits) -> Result<u16, Error> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= b.get(1)? as i32;
            let c = self.count[len] as i32;
            if code - c < first {
                return self.sym.get((index + (code - first)) as usize).copied().ok_or(Error::Inflate);
            }
            index += c;
            first = (first + c) << 1;
            code <<= 1;
        }
        Err(Error::Inflate)
    }
}

const LBASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEXT: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DBASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DEXT: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// Inflates a zlib stream; `cap` bounds the output.
fn inflate(z: &[u8], cap: usize) -> Result<Vec<u8>, Error> {
    let mut b = Bits { d: z.get(2..).ok_or(Error::Inflate)?, pos: 0, acc: 0, n: 0 };
    let mut out: Vec<u8> = Vec::with_capacity(cap);
    loop {
        let last = b.get(1)?;
        match b.get(2)? {
            0 => {
                b.acc = 0;
                b.n = 0;
                let h = b.d.get(b.pos..b.pos + 4).ok_or(Error::Inflate)?;
                let len = u16::from_le_bytes([h[0], h[1]]) as usize;
                b.pos += 4;
                let s = b.d.get(b.pos..b.pos + len).ok_or(Error::Inflate)?;
                if out.len() + len > cap {
                    return Err(Error::Inflate);
                }
                out.extend_from_slice(s);
                b.pos += len;
            }
            t @ (1 | 2) => {
                let (lit, dist);
                if t == 1 {
                    let mut l = [8u8; 288];
                    l[144..256].fill(9);
                    l[256..280].fill(7);
                    lit = Huff::new(&l);
                    dist = Huff::new(&[5u8; 30]);
                } else {
                    let (nlen, ndist, ncode) = (b.get(5)? as usize + 257, b.get(5)? as usize + 1, b.get(4)? as usize + 4);
                    const ORD: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                    let mut cl = [0u8; 19];
                    for &o in ORD.iter().take(ncode) {
                        cl[o] = b.get(3)? as u8;
                    }
                    let ch = Huff::new(&cl);
                    let mut lens = vec![0u8; nlen + ndist];
                    let mut i = 0;
                    while i < nlen + ndist {
                        let s = ch.decode(&mut b)?;
                        let (v, rep) = match s {
                            0..=15 => (s as u8, 1),
                            16 => (*lens.get(i.wrapping_sub(1)).ok_or(Error::Inflate)?, 3 + b.get(2)? as usize),
                            17 => (0, 3 + b.get(3)? as usize),
                            _ => (0, 11 + b.get(7)? as usize),
                        };
                        if i + rep > lens.len() {
                            return Err(Error::Inflate);
                        }
                        lens[i..i + rep].fill(v);
                        i += rep;
                    }
                    lit = Huff::new(&lens[..nlen]);
                    dist = Huff::new(&lens[nlen..]);
                }
                loop {
                    let s = lit.decode(&mut b)? as usize;
                    if s < 256 {
                        if out.len() >= cap {
                            return Err(Error::Inflate);
                        }
                        out.push(s as u8);
                    } else if s == 256 {
                        break;
                    } else {
                        let s = s - 257;
                        if s >= 29 {
                            return Err(Error::Inflate);
                        }
                        let len = LBASE[s] as usize + b.get(LEXT[s] as u32)? as usize;
                        let ds = dist.decode(&mut b)? as usize;
                        if ds >= 30 {
                            return Err(Error::Inflate);
                        }
                        let d = DBASE[ds] as usize + b.get(DEXT[ds] as u32)? as usize;
                        if d > out.len() || out.len() + len > cap {
                            return Err(Error::Inflate);
                        }
                        for _ in 0..len {
                            out.push(out[out.len() - d]);
                        }
                    }
                }
            }
            _ => return Err(Error::Inflate),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

/// Decode to BGRA8, rows top-down.
pub fn decode(data: &[u8]) -> Result<Image, Error> {
    if !is_png(data) {
        return Err(Error::NotPng);
    }
    let (mut p, mut ihdr, mut plte, mut trns) = (8usize, None, &[][..], &[][..]);
    let mut idat: Vec<u8> = Vec::new();
    while let Some(h) = data.get(p..p + 8) {
        let len = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as usize;
        let body = data.get(p + 8..p + 8 + len).ok_or(Error::Truncated)?;
        match &h[4..8] {
            b"IHDR" => ihdr = Some(body),
            b"PLTE" => plte = body,
            b"tRNS" => trns = body,
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        p += 12 + len;
    }
    let h = ihdr.filter(|h| h.len() >= 13).ok_or(Error::Truncated)?;
    let (w, ht) = (u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as usize, u32::from_be_bytes([h[4], h[5], h[6], h[7]]) as usize);
    let (depth, ctype, interlace) = (h[8], h[9], h[12]);
    if w == 0 || ht == 0 {
        return Err(Error::Empty);
    }
    if interlace != 0 || (depth != 8 && depth != 16) || (ctype == 3 && depth != 8) {
        return Err(Error::Unsupported);
    }
    let ch = match ctype {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err(Error::Unsupported),
    };
    let bpp = ch * (depth as usize / 8);
    let stride = w.checked_mul(bpp).ok_or(Error::Unsupported)?;
    let total = (stride + 1).checked_mul(ht).filter(|&t| t <= 1 << 28).ok_or(Error::Unsupported)?;
    let mut raw = inflate(&idat, total)?;
    if raw.len() < total {
        return Err(Error::Truncated);
    }
    // Unfilter in place; `prev` is the row above, already unfiltered.
    for y in 0..ht {
        let (before, row) = raw.split_at_mut(y * (stride + 1) + 1);
        let row = &mut row[..stride];
        let f = before.last().copied().unwrap_or(0);
        let prev: Option<&[u8]> = (y > 0).then(|| &before[before.len() - 1 - stride..before.len() - 1]);
        for i in 0..stride {
            let a = if i >= bpp { row[i - bpp] as i32 } else { 0 };
            let b = prev.map_or(0, |r| r[i] as i32);
            let c = if i >= bpp { prev.map_or(0, |r| r[i - bpp] as i32) } else { 0 };
            let pred = match f {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                4 => {
                    let pp = a + b - c;
                    let (pa, pb, pc) = ((pp - a).abs(), (pp - b).abs(), (pp - c).abs());
                    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
                }
                _ => return Err(Error::Inflate),
            };
            row[i] = row[i].wrapping_add(pred as u8);
        }
    }
    let step = depth as usize / 8;
    let mut px = vec![0u8; w * ht * 4];
    for y in 0..ht {
        let row = &raw[y * (stride + 1) + 1..][..stride];
        for x in 0..w {
            let s = |c: usize| row[x * bpp + c * step];
            let (r, g, b, a) = match ctype {
                0 => (s(0), s(0), s(0), 255),
                2 => (s(0), s(1), s(2), 255),
                3 => {
                    let i = s(0) as usize;
                    let c = plte.get(i * 3..i * 3 + 3).ok_or(Error::Truncated)?;
                    (c[0], c[1], c[2], trns.get(i).copied().unwrap_or(255))
                }
                4 => (s(0), s(0), s(0), s(1)),
                _ => (s(0), s(1), s(2), s(3)),
            };
            px[(y * w + x) * 4..][..4].copy_from_slice(&[b, g, r, a]);
        }
    }
    Ok(Image { width: w as u32, height: ht as u32, bits_per_pixel: 32, pixels: px })
}

/// The bounding box (x, y, w, h) of a title picture's lettering: the first band of rows,
/// from the top, that holds ink. Ink is a pixel of alpha 128 or more, outside a margin of
/// 3% of the picture all round (a frame line stands there) and not blue-dominant (the
/// frame and the translucent panel Roots draws are blue, the lettering gold and silver);
/// rows closer than 6% of the height belong to one band (the rule, the second line). The
/// box is padded by 2% of the height for the drop shadow, which is fainter than the ink.
/// None when there is no ink.
pub fn title_box(img: &Image) -> Option<[u32; 4]> {
    let (w, h) = (img.width as usize, img.height as usize);
    let (mx, my) = (w * 3 / 100, h * 3 / 100);
    let gap = (h * 6 / 100).max(1);
    let (mut y0, mut y1) = (usize::MAX, 0usize);
    let mut last = 0usize;
    for y in my..h.saturating_sub(my) {
        let mut any = false;
        for x in mx..w.saturating_sub(mx) {
            let q = &img.pixels[(y * w + x) * 4..][..4];
            let (b, r, a) = (q[0] as i32, q[2] as i32, q[3]);
            if a >= 128 && b <= r + 24 {
                any = true;
            }
        }
        if any {
            if y0 == usize::MAX {
                y0 = y;
            } else if y - last > gap {
                // A second band: not the title.
                break;
            }
            y1 = y;
            last = y;
        }
    }
    if y0 == usize::MAX {
        return None;
    }
    // The columns, over the title's rows alone.
    let (mut cx0, mut cx1) = (usize::MAX, 0usize);
    for y in y0..=y1 {
        for x in mx..w.saturating_sub(mx) {
            let q = &img.pixels[(y * w + x) * 4..][..4];
            if q[3] >= 128 && (q[0] as i32) <= q[2] as i32 + 24 {
                cx0 = cx0.min(x);
                cx1 = cx1.max(x);
            }
        }
    }
    let pad = h * 2 / 100;
    let (ax, ay) = (cx0.saturating_sub(pad), y0.saturating_sub(pad));
    let (bx, by) = ((cx1 + pad + 1).min(w), (y1 + pad + 1).min(h));
    Some([ax as u32, ay as u32, (bx - ax) as u32, (by - ay) as u32])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2x1 RGBA image, one stored deflate block, filter 0.
    #[test]
    fn stored_block() {
        let mut f = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10];
        let mut chunk = |t: &[u8; 4], b: &[u8]| {
            f.extend_from_slice(&(b.len() as u32).to_be_bytes());
            f.extend_from_slice(t);
            f.extend_from_slice(b);
            f.extend_from_slice(&[0; 4]);
        };
        chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        let raw = [0u8, 1, 2, 3, 4, 5, 6, 7, 8];
        let mut z = vec![0x78, 0x01, 1, 9, 0, !9, 0xff];
        z.extend_from_slice(&raw);
        z.extend_from_slice(&[0; 4]);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        let i = decode(&f).unwrap();
        assert_eq!((i.width, i.height), (2, 1));
        assert_eq!(i.pixels, [3, 2, 1, 4, 7, 6, 5, 8]);
        assert_eq!(decode(&f[..20]), Err(Error::Truncated));
        assert_eq!(decode(b"x"), Err(Error::NotPng));
    }

    /// Reads the file named by `A2_TEST_TITLE_PNG` (Roots' own art, never in the repo).
    #[test]
    #[ignore]
    fn title_box_of_a_real_file() {
        let Ok(p) = std::env::var("A2_TEST_TITLE_PNG") else { return };
        let i = decode(&std::fs::read(p).unwrap()).unwrap();
        std::eprintln!("{}x{} box {:?}", i.width, i.height, title_box(&i));
    }

    #[test]
    fn title_box_synthetic() {
        // 100x100: frame line in blue at the border, a gold bar at 30..40 x 20..60, a
        // blue panel edge lower down.
        let mut px = vec![0u8; 100 * 100 * 4];
        let mut put = |x: usize, y: usize, c: [u8; 4]| px[(y * 100 + x) * 4..][..4].copy_from_slice(&c);
        for i in 0..100 {
            put(i, 1, [255, 100, 50, 255]);
            put(1, i, [255, 100, 50, 255]);
        }
        for y in 30..40 {
            for x in 20..60 {
                put(x, y, [30, 140, 200, 255]);
            }
        }
        for x in 30..70 {
            put(x, 70, [255, 100, 50, 255]);
        }
        let i = Image { width: 100, height: 100, bits_per_pixel: 32, pixels: px };
        assert_eq!(title_box(&i), Some([18, 28, 44, 14]));
    }

    #[test]
    fn dynamic_block_real_file() {
        // A 1x1 grey PNG as written by common encoders (fixed Huffman block).
        let d = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0, 0x3a,
            0x7e, 0x9b, 0x55, 0, 0, 0, 10, 0x49, 0x44, 0x41, 0x54, 0x78, 0x01, 0x63, 0x68, 0x00, 0x00, 0x00, 0x82, 0x00, 0x81, 0xcb, 0x2c, 0x0c,
            0x1c, 0, 0, 0, 0, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let i = decode(&d).unwrap();
        assert_eq!(i.pixels[3], 255);
    }
}
