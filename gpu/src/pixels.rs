//! Texels read back to the CPU (`Cmd::read_back`), and their conversion to RGB for
//! the frame dumps (render3d/README.md, "Frame dumps").
use crate::Format;

/// A target's texels, mapped for reading: valid only inside `Cmd::read_back`'s call.
pub struct Mapped<'a> {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    /// The samples of the target that was read (it was resolved first if more than 1).
    pub samples: u32,
    pub(crate) pitch: usize,
    pub(crate) bits: *const u8,
    pub(crate) _life: core::marker::PhantomData<&'a [u8]>,
}

impl Mapped<'_> {
    /// The first row's first texel; rows are `pitch` bytes apart.
    pub fn ptr(&self) -> *const u8 {
        self.bits
    }

    pub fn pitch(&self) -> usize {
        self.pitch
    }

    /// Row `y` from column `x` on, `w` texels of it. Empty when the format's size is
    /// unknown or the span is outside the target.
    pub fn row(&self, x: u32, y: u32, w: u32) -> &[u8] {
        let Some(b) = self.format.bytes() else { return &[] };
        if y >= self.height || x + w > self.width {
            return &[];
        }
        unsafe { core::slice::from_raw_parts(self.bits.add(y as usize * self.pitch + x as usize * b), w as usize * b) }
    }
}

/// A half float's value.
pub fn half(h: u16) -> f32 {
    let s = ((h >> 15) & 1) as u32;
    let e = ((h >> 10) & 0x1f) as u32;
    let m = (h & 0x3ff) as u32;
    let bits = if e == 0 {
        if m == 0 {
            s << 31
        } else {
            // Subnormal: m * 2^-24.
            let v = m as f32 * (1.0 / 16_777_216.0);
            return if s != 0 { -v } else { v };
        }
    } else if e == 31 {
        (s << 31) | 0x7f80_0000 | (m << 13)
    } else {
        (s << 31) | ((e + 112) << 23) | (m << 13)
    };
    f32::from_bits(bits)
}

impl Format {
    /// Whether `rgb_f32` and `rgb_u8` read this format.
    pub fn readable(self) -> bool {
        matches!(self, Format::Rgba16F | Format::Rgba32F | Format::Bgra8 | Format::Bgrx8 | Format::B5G6R5 | Format::Bgr10A2 | Format::Rgb10A2)
    }

    /// One texel as float RGB. `p` is the texel's bytes.
    pub fn rgb_f32(self, p: &[u8]) -> [f32; 3] {
        let u16_at = |i: usize| u16::from_le_bytes([p[i], p[i + 1]]);
        let f32_at = |i: usize| f32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
        let u32_at = |i: usize| u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
        match self {
            Format::Rgba16F => [half(u16_at(0)), half(u16_at(2)), half(u16_at(4))],
            Format::Rgba32F => [f32_at(0), f32_at(4), f32_at(8)],
            Format::Bgra8 | Format::Bgrx8 => [p[2] as f32 / 255.0, p[1] as f32 / 255.0, p[0] as f32 / 255.0],
            Format::B5G6R5 => {
                let v = u16_at(0) as u32;
                [((v >> 11) & 31) as f32 / 31.0, ((v >> 5) & 63) as f32 / 63.0, (v & 31) as f32 / 31.0]
            }
            Format::Bgr10A2 => {
                let v = u32_at(0);
                [((v >> 20) & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, (v & 1023) as f32 / 1023.0]
            }
            Format::Rgb10A2 => {
                let v = u32_at(0);
                [(v & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, ((v >> 20) & 1023) as f32 / 1023.0]
            }
            _ => [0.0; 3],
        }
    }

    /// One texel as 8-bit RGB: the bytes as they are for 8-bit formats (byte-exact),
    /// else the float value clamped and rounded.
    pub fn rgb_u8(self, p: &[u8]) -> [u8; 3] {
        match self {
            Format::Bgra8 | Format::Bgrx8 => [p[2], p[1], p[0]],
            _ => {
                let c = self.rgb_f32(p);
                let q = |v: f32| if v.is_nan() || v <= 0.0 { 0 } else if v >= 1.0 { 255 } else { (v * 255.0 + 0.5) as u8 };
                [q(c[0]), q(c[1]), q(c[2])]
            }
        }
    }
}
