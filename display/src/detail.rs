//! A detailed look at the engine's draws, on demand: while `Display8.go` exists beside
//! the exe (looked for every 30 presents, deleted when seen) the next `Frames=` frames
//! (`[Trace]`, default 3) log one `DRAW` line per draw `Armada2.exe` makes, with the
//! state that decides what it puts on screen: the blend, the depth test, the texture,
//! the vertex format and, for vertices we can read, their box on the screen and
//! colours. That is how a draw the inventory counts is named without a picture: the
//! engine's 2D goes through one dynamic vertex buffer (its "trifan buffer") written by
//! lock, so the vertices are copied at `Unlock` while a capture runs.
use crate::com::Obj;
use crate::d3d8::describe;
use crate::trace;
use crate::{log, rt, sfmt};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, Ordering};

/// Frames still to capture.
static LEFT: AtomicU32 = AtomicU32::new(0);
static mut FRAMES: u32 = 3;
static mut PRESENTS: u32 = 0;

struct State {
    rs: [u32; 256],
    tex0: usize,
    stream0: (usize, u32),
    fvf: u32,
    /// Vertex buffers' contents as last unlocked during a capture, by object of ours.
    shadow: BTreeMap<usize, Vec<u8>>,
    /// Locks open: object -> (pointer, offset, size).
    open: BTreeMap<usize, (usize, u32, u32)>,
}

static mut ST: Option<State> = None;

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe {
        ST.get_or_insert_with(|| State {
            rs: [0; 256],
            tex0: 0,
            stream0: (0, 0),
            fvf: 0,
            shadow: BTreeMap::new(),
            open: BTreeMap::new(),
        })
    }
}

pub fn configure(ini: &rt::Ini) {
    unsafe { FRAMES = ini.int("Trace", "Frames", 3).max(1) as u32 };
}

pub fn on() -> bool {
    LEFT.load(Ordering::Relaxed) > 0
}

pub fn set_render_state(s: u32, v: u32) {
    if (s as usize) < 256 {
        st().rs[s as usize] = v;
    }
}
pub fn set_texture(stage: u32, t: *mut c_void) {
    if stage == 0 {
        st().tex0 = t as usize;
    }
}
pub fn set_stream(n: u32, vb: *mut c_void, stride: u32) {
    if n == 0 {
        st().stream0 = (vb as usize, stride);
    }
}
pub fn set_vertex_shader(h: u32) {
    st().fvf = h;
}

pub fn locked(vb: *mut Obj, data: *mut *mut u8, off: u32, size: u32) {
    if !on() || data.is_null() {
        return;
    }
    let p = unsafe { *data } as usize;
    st().open.insert(vb as usize, (p, off, size));
}

pub fn unlocked(vb: *mut Obj) {
    let s = st();
    let Some((p, off, size)) = s.open.remove(&(vb as usize)) else { return };
    if p == 0 {
        return;
    }
    let n = if size == 0 {
        // The whole buffer from `off`: its size from the description.
        let mut d = crate::d3d8::BufferDesc::default();
        vcall!(unsafe { (*vb).real }, 13, fn(*mut crate::d3d8::BufferDesc) -> i32, &mut d);
        d.size.saturating_sub(off)
    } else {
        size
    };
    let sh = s.shadow.entry(vb as usize).or_default();
    let end = (off + n) as usize;
    if sh.len() < end {
        sh.resize(end, 0);
    }
    unsafe { core::ptr::copy_nonoverlapping(p as *const u8, sh.as_mut_ptr().add(off as usize), n as usize) };
}

/// Vertex size and where the colour is, from an FVF.
fn layout(fvf: u32) -> Option<(usize, usize, bool)> {
    let (pos, rhw) = match fvf & 0x00e {
        0x002 => (12, false),
        0x004 => (16, true),
        0x006 => (16, false),
        0x008 => (20, false),
        0x00a => (24, false),
        0x00c => (28, false),
        0x00e => (32, false),
        _ => return None,
    };
    let mut at = pos;
    if fvf & 0x010 != 0 {
        at += 12;
    }
    if fvf & 0x020 != 0 {
        at += 4;
    }
    let diffuse = if fvf & 0x040 != 0 { Some(at) } else { None };
    Some((at, diffuse.unwrap_or(usize::MAX), rhw))
}

fn vertices(data: &[u8], stride: usize, fvf: u32, first: usize, count: usize) -> String {
    let Some((_, col, rhw)) = layout(fvf) else { return String::from("fvf ?") };
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut colours: Vec<u32> = Vec::new();
    for i in first..first + count {
        let at = i * stride;
        if at + stride > data.len() || stride < 8 {
            break;
        }
        let f = |k: usize| f32::from_le_bytes([data[at + k], data[at + k + 1], data[at + k + 2], data[at + k + 3]]);
        let (x, y) = (f(0), f(4));
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
        if col != usize::MAX && colours.len() < 4 {
            let c = u32::from_le_bytes([data[at + col], data[at + col + 1], data[at + col + 2], data[at + col + 3]]);
            if !colours.contains(&c) {
                colours.push(c);
            }
        }
    }
    if x0 > x1 {
        return String::from("no vertices read");
    }
    let mut t = sfmt!("{} {:.0},{:.0}..{:.0},{:.0}", if rhw { "screen" } else { "model" }, x0, y0, x1, y1);
    for c in colours {
        t.push_str(&sfmt!(" {:#010x}", c));
    }
    t
}

fn vertex_count(pt: u32, n: u32) -> usize {
    (match pt {
        1 => n,
        2 => n * 2,
        3 => n + 1,
        4 => n * 3,
        5 | 6 => n + 2,
        _ => n,
    }) as usize
}

/// A draw on the device: `a` its arguments, in order.
pub fn draw(fp: usize, slot: u8, a: &[usize]) {
    if !on() {
        return;
    }
    let ra = unsafe { *((fp + 4) as *const usize) };
    let exe = trace::exe_range();
    if ra < exe.0 || ra >= exe.1 {
        return;
    }
    let s = st();
    let rs = &s.rs;
    let what = match slot {
        70 => {
            let (pt, start, n) = (a[0] as u32, a[1], a[2] as u32);
            let (vb, stride) = s.stream0;
            let v = s.shadow.get(&vb).map_or(String::from("vertices not seen"), |d| vertices(d, stride as usize, s.fvf, start, vertex_count(pt, n)));
            sfmt!("DrawPrimitive pt {} n {} | {}", pt, n, v)
        }
        72 => {
            let (pt, n, p, stride) = (a[0] as u32, a[1] as u32, a[2], a[3]);
            let count = vertex_count(pt, n);
            let d = unsafe { core::slice::from_raw_parts(p as *const u8, count * stride) };
            sfmt!("DrawPrimitiveUP pt {} n {} | {}", pt, n, vertices(d, stride, s.fvf, 0, count))
        }
        71 => sfmt!("DrawIndexedPrimitive pt {} n {}", a[0], a[4]),
        _ => sfmt!("DrawIndexedPrimitiveUP pt {} n {}", a[0], a[3]),
    };
    let tex = if s.tex0 == 0 || !crate::com::is_ours(s.tex0 as *mut c_void) { String::from("none") } else { describe(s.tex0 as *mut Obj) };
    log!(
        "DRAW {} | fvf {:#x} | blend {} {}/{} | test {} ref {:#x} | z {},{} | tex {} | by {}",
        what,
        s.fvf,
        rs[27],
        rs[19],
        rs[20],
        rs[15],
        rs[24],
        rs[7],
        rs[14],
        tex,
        trace::chain(fp)
    );
}

/// At each `Present`: ends a capture's frame, or starts one if `Display8.go` is there.
pub fn present() {
    let left = LEFT.load(Ordering::Relaxed);
    if left > 0 {
        log!("DRAW --- end of frame ({} to go)", left - 1);
        LEFT.store(left - 1, Ordering::Relaxed);
        return;
    }
    unsafe {
        PRESENTS = PRESENTS.wrapping_add(1);
        if PRESENTS % 30 != 0 {
            return;
        }
    }
    let go = rt::game_path("Display8.go");
    if rt::file_exists(&go) {
        rt::delete_file(&go);
        let n = unsafe { FRAMES };
        log!("DRAW --- capture of {} frames", n);
        LEFT.store(n, Ordering::Relaxed);
    }
}
