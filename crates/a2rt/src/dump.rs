//! Frame dumps on request: render targets read back byte for byte and written as PFM
//! (float RGB) or binary PPM (8-bit RGB) into `Dumps\` in the game directory
//! (render3d's and ui's README.md, "Frame dumps"). Behind the `dump` feature, which
//! brings in `gpu`: Display8 has no targets of its own to dump.
//!
//! A dump is asked for with a trigger file beside `Armada2.exe` (`Render3D.dump`,
//! `UI.dump`), looked for every `EVERY` frames as D3DTrace does, and deleted when seen.
//! Its text, if any, is the name the files start with. Each file is
//! `<name>-<frame>-<what>.pfm|.ppm`, and a sidecar `<name>-<frame>-<plugin>.txt`
//! lists them with the frame number, each target's size, format and samples.
//!
//! A target is read back through `gpu` (`Gpu::read_back`: on Direct3D 9 copied with
//! `GetRenderTargetData` into a system-memory surface of its size and format, a
//! multisampled one resolved with `StretchRect` first) and converted row by row on the
//! CPU, so nothing of the device's state changes. A format with no conversion
//! (`gpu::Format::readable`) is logged and skipped.
use crate as rt;
use gpu::d3d9::format_code;
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

/// The sidecar's samples: 0 for a target of one, as Direct3D 9 counts them.
fn ms(samples: u32) -> u32 {
    if samples > 1 { samples } else { 0 }
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

    /// Reads `src` (a colour target) back and writes `area` of it (None: the whole)
    /// as `what`. Logs and notes what it wrote, or why not.
    pub fn surface(&mut self, gpu: &gpu::Gpu, src: &gpu::Texture, area: Option<Area>, what: &str, kind: Kind) {
        let d = src.desc();
        let a = area.unwrap_or(Area { x: 0, y: 0, w: d.width, h: d.height });
        let ext = if kind == Kind::Pfm { "pfm" } else { "ppm" };
        let file = self.file(what, ext);
        let r = self.read_write(gpu, src, &d, a, &file, kind);
        let (code, name) = (format_code(d.format), d.format.name());
        let line = match r {
            Ok(()) => {
                self.written += 1;
                crate::sfmt!(
                    "{} {}x{} at {},{} of {}x{}, format {} ({}), {} samples",
                    file, a.w, a.h, a.x, a.y, d.width, d.height, code, name, ms(d.samples)
                )
            }
            Err(why) => {
                self.skipped += 1;
                crate::sfmt!("skipped {}: {} (format {} ({}), {}x{}, {} samples)", file, why, code, name, d.width, d.height, ms(d.samples))
            }
        };
        crate::log!("dump: {}", line);
        self.note(&line);
    }

    fn read_write(&self, gpu: &gpu::Gpu, src: &gpu::Texture, d: &gpu::TextureDesc, a: Area, file: &str, kind: Kind) -> Result<(), &'static str> {
        if !d.format.readable() {
            return Err("a format this dump cannot read");
        }
        if a.w == 0 || a.h == 0 || a.x + a.w > d.width || a.y + a.h > d.height {
            return Err("the area is outside the target");
        }
        let path = rt::game_path(&crate::sfmt!("Dumps\\{}", file));
        gpu.read_back(src, |px| {
            let bpp = px.format.bytes().unwrap_or(4);
            let fmt = px.format;
            let mut w = rt::Writer::create(&path).ok_or("the file could not be made")?;
            match kind {
                Kind::Pfm => {
                    w.write(crate::sfmt!("PF\n{} {}\n-1.0\n", a.w, a.h).as_bytes());
                    let mut buf: Vec<u8> = Vec::with_capacity(a.w as usize * 12);
                    for y in (a.y..a.y + a.h).rev() {
                        buf.clear();
                        for t in px.row(a.x, y, a.w).chunks_exact(bpp) {
                            for v in fmt.rgb_f32(t) {
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
                        for t in px.row(a.x, y, a.w).chunks_exact(bpp) {
                            buf.extend_from_slice(&fmt.rgb_u8(t));
                        }
                        w.write(&buf);
                    }
                }
            }
            if w.ok { Ok(()) } else { Err("writing the file failed") }
        })?
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
