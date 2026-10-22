//! Frame dumps on request: render targets read back byte for byte and written as PFM
//! (float RGB) or binary PPM (8-bit RGB) into `Dumps\` in the game directory
//! (README.md, "Frame dumps"). The same file is in `render3d/src/dump.rs` and
//! `ui/src/dump.rs`; keep them alike.
//!
//! A dump is asked for with a trigger file beside `Armada2.exe` (`Render3D.dump`,
//! `UI.dump`), looked for every `EVERY` frames as D3DTrace does, and deleted when seen.
//! Its text, if any, is the name the files start with. Each file is
//! `<name>-<frame>-<what>.pfm|.ppm`, and a sidecar `<name>-<frame>-<plugin>.txt`
//! lists them with the frame number, each target's size, format and samples.
//!
//! A target is copied with `GetRenderTargetData` into a system-memory surface of its
//! size and format (a multisampled one is first resolved with `StretchRect` into a
//! plain target), locked and converted row by row on the CPU, so nothing of the
//! device's state changes. A format with no conversion here is logged and skipped.
use crate::d3d::*;
use crate::rt;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// Frames between two looks for the trigger file.
const EVERY: u32 = 30;
static mut POLLS: u32 = 0;

/// Called once a frame: every `EVERY` calls looks for `trigger` beside the exe, and
/// when it is there deletes it and returns the name the dump's files start with (its
/// text, made safe for a file name; `dump` when empty). `Dumps\` is made then.
#[allow(static_mut_refs)]
pub fn poll(trigger: &str) -> Option<String> {
    unsafe {
        POLLS = POLLS.wrapping_add(1);
        if POLLS % EVERY != 0 {
            return None;
        }
    }
    let p = rt::game_path(trigger);
    if !rt::file_exists(&p) {
        return None;
    }
    let text = rt::read_file(&p).unwrap_or_default();
    rt::delete_file(&p);
    let raw = String::from_utf8_lossy(&text);
    let mut name = String::new();
    for c in raw.trim().chars().take(64) {
        name.push(if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' { c } else { '-' });
    }
    if name.is_empty() {
        name.push_str("dump");
    }
    if !rt::create_dir(&rt::game_path("Dumps")) {
        crate::log!("dump: {} seen, but Dumps\\ could not be made: nothing written", trigger);
        return None;
    }
    Some(name)
}

/// A rectangle of a target, in pixels.
#[derive(Clone, Copy)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// PFM: float RGB, rows bottom-up, little-endian (scale -1).
    Pfm,
    /// PPM (P6): 8-bit RGB, rows top-down. Float formats are clamped to [0, 1].
    Ppm,
}

/// One dump: the files written so far and the sidecar's lines.
pub struct Dump {
    pub name: String,
    pub frame: u32,
    plugin: &'static str,
    side: String,
    written: u32,
    skipped: u32,
}

pub fn format_name(f: u32) -> &'static str {
    match f {
        FMT_A16B16G16R16F => "A16B16G16R16F",
        FMT_A32B32G32R32F => "A32B32G32R32F",
        FMT_A8R8G8B8 => "A8R8G8B8",
        FMT_X8R8G8B8 => "X8R8G8B8",
        FMT_R5G6B5 => "R5G6B5",
        FMT_A2R10G10B10 => "A2R10G10B10",
        FMT_A2B10G10R10 => "A2B10G10R10",
        _ => "?",
    }
}

const FMT_A2B10G10R10: u32 = 31;
const FMT_A2R10G10B10: u32 = 35;

/// Bytes per pixel of the formats read here; None for the rest.
fn bpp(f: u32) -> Option<usize> {
    match f {
        FMT_A16B16G16R16F => Some(8),
        FMT_A32B32G32R32F => Some(16),
        FMT_A8R8G8B8 | FMT_X8R8G8B8 | FMT_A2R10G10B10 | FMT_A2B10G10R10 => Some(4),
        FMT_R5G6B5 => Some(2),
        _ => None,
    }
}

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

/// One pixel as float RGB. `p` points at the pixel in format `f`.
fn rgb_f32(f: u32, p: &[u8]) -> [f32; 3] {
    let u16_at = |i: usize| u16::from_le_bytes([p[i], p[i + 1]]);
    let f32_at = |i: usize| f32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
    let u32_at = |i: usize| u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
    match f {
        FMT_A16B16G16R16F => [half(u16_at(0)), half(u16_at(2)), half(u16_at(4))],
        FMT_A32B32G32R32F => [f32_at(0), f32_at(4), f32_at(8)],
        FMT_A8R8G8B8 | FMT_X8R8G8B8 => [p[2] as f32 / 255.0, p[1] as f32 / 255.0, p[0] as f32 / 255.0],
        FMT_R5G6B5 => {
            let v = u16_at(0) as u32;
            [((v >> 11) & 31) as f32 / 31.0, ((v >> 5) & 63) as f32 / 63.0, (v & 31) as f32 / 31.0]
        }
        FMT_A2R10G10B10 => {
            let v = u32_at(0);
            [((v >> 20) & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, (v & 1023) as f32 / 1023.0]
        }
        FMT_A2B10G10R10 => {
            let v = u32_at(0);
            [(v & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, ((v >> 20) & 1023) as f32 / 1023.0]
        }
        _ => [0.0; 3],
    }
}

/// One pixel as 8-bit RGB: the bytes as they are for 8-bit formats (byte-exact), else
/// the float value clamped and rounded.
fn rgb_u8(f: u32, p: &[u8]) -> [u8; 3] {
    match f {
        FMT_A8R8G8B8 | FMT_X8R8G8B8 => [p[2], p[1], p[0]],
        _ => {
            let c = rgb_f32(f, p);
            let q = |v: f32| if v.is_nan() || v <= 0.0 { 0 } else if v >= 1.0 { 255 } else { (v * 255.0 + 0.5) as u8 };
            [q(c[0]), q(c[1]), q(c[2])]
        }
    }
}

impl Dump {
    pub fn new(name: String, frame: u32, plugin: &'static str) -> Dump {
        let mut side = String::new();
        let _ = writeln!(side, "plugin {}\r\nframe {}\r\nname {}\r", plugin, frame, name);
        Dump { name, frame, plugin, side, written: 0, skipped: 0 }
    }

    /// A line of its own in the sidecar.
    pub fn note(&mut self, line: &str) {
        self.side.push_str(line);
        self.side.push_str("\r\n");
    }

    fn file(&self, what: &str, ext: &str) -> String {
        crate::sfmt!("{}-{}-{}.{}", self.name, self.frame, what, ext)
    }

    /// Reads `src` (a render target surface) back and writes `area` of it (None: the
    /// whole) as `what`. Logs and notes what it wrote, or why not.
    pub fn surface(&mut self, dev: Device, src: &Com, area: Option<Area>, what: &str, kind: Kind) {
        let d = src.surf_desc();
        let a = area.unwrap_or(Area { x: 0, y: 0, w: d.width, h: d.height });
        let ext = if kind == Kind::Pfm { "pfm" } else { "ppm" };
        let file = self.file(what, ext);
        let r = self.read_write(dev, src, &d, a, &file, kind);
        let line = match r {
            Ok(()) => {
                self.written += 1;
                crate::sfmt!(
                    "{} {}x{} at {},{} of {}x{}, format {} ({}), {} samples",
                    file, a.w, a.h, a.x, a.y, d.width, d.height, d.format, format_name(d.format), d.ms_type
                )
            }
            Err(why) => {
                self.skipped += 1;
                crate::sfmt!("skipped {}: {} (format {} ({}), {}x{}, {} samples)", file, why, d.format, format_name(d.format), d.width, d.height, d.ms_type)
            }
        };
        crate::log!("dump: {}", line);
        self.note(&line);
    }

    fn read_write(&self, dev: Device, src: &Com, d: &SurfaceDesc, a: Area, file: &str, kind: Kind) -> Result<(), &'static str> {
        let bpp = bpp(d.format).ok_or("a format this dump cannot read")?;
        if a.w == 0 || a.h == 0 || a.x + a.w > d.width || a.y + a.h > d.height {
            return Err("the area is outside the target");
        }
        // A multisampled target cannot be read back: resolve it into a plain one first.
        let plain;
        let from = if d.ms_type != 0 {
            plain = dev.create_render_target(d.width, d.height, d.format, 0, false).ok_or("no plain target to resolve into")?;
            if dev.stretch_rect(src, None, &plain, None, TEXF_NONE) < 0 {
                return Err("the resolve (StretchRect) failed");
            }
            &plain
        } else {
            src
        };
        let sys = dev.create_offscreen_surface(d.width, d.height, d.format, POOL_SYSTEMMEM).ok_or("no system-memory surface")?;
        if dev.get_render_target_data(from, &sys) < 0 {
            return Err("GetRenderTargetData failed");
        }
        let lr = sys.surf_lock(LOCK_READONLY).ok_or("the copy could not be locked")?;
        let path = rt::game_path(&crate::sfmt!("Dumps\\{}", file));
        let res = (|| {
            let mut w = rt::Writer::create(&path).ok_or("the file could not be made")?;
            let row_at = |y: u32| unsafe {
                core::slice::from_raw_parts(lr.bits.add(y as usize * lr.pitch as usize + a.x as usize * bpp), a.w as usize * bpp)
            };
            match kind {
                Kind::Pfm => {
                    w.write(crate::sfmt!("PF\n{} {}\n-1.0\n", a.w, a.h).as_bytes());
                    let mut buf: Vec<u8> = Vec::with_capacity(a.w as usize * 12);
                    for y in (a.y..a.y + a.h).rev() {
                        buf.clear();
                        for px in row_at(y).chunks_exact(bpp) {
                            for v in rgb_f32(d.format, px) {
                                buf.extend_from_slice(&v.to_le_bytes());
                            }
                        }
                        w.write(&buf);
                    }
                }
                Kind::Ppm => {
                    w.write(crate::sfmt!("P6\n{} {}\n255\n", a.w, a.h).as_bytes());
                    let mut buf: Vec<u8> = Vec::with_capacity(a.w as usize * 3);
                    for y in a.y..a.y + a.h {
                        buf.clear();
                        for px in row_at(y).chunks_exact(bpp) {
                            buf.extend_from_slice(&rgb_u8(d.format, px));
                        }
                        w.write(&buf);
                    }
                }
            }
            if w.ok { Ok(()) } else { Err("writing the file failed") }
        })();
        sys.surf_unlock();
        res
    }

    /// Writes the sidecar and logs the dump.
    pub fn finish(self) {
        let file = self.file(self.plugin, "txt");
        let ok = rt::write_file(&rt::game_path(&crate::sfmt!("Dumps\\{}", file)), self.side.as_bytes());
        crate::log!(
            "dump: frame {} as {}: {} files written, {} skipped; sidecar {}{}",
            self.frame, self.name, self.written, self.skipped, file, if ok { "" } else { " could not be written" }
        );
    }
}
