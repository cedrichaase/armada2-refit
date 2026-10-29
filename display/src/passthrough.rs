//! Passthrough (README.md, "Passthrough"): in stub mode, the engine's own Direct3D 8
//! draws are drawn after all, on Display8's device, the way Direct3D 8's fixed function
//! draws them. Whatever render3d or the UI does not draw itself (a module switched off,
//! a SOD render3d hands back, a HUD part the UI has not taken) then shows as stock
//! instead of vanishing.
//!
//! This half is the backend's neutral part: whether a draw is drawn at all, and what
//! it draws (its vertex layout from the FVF, its vertices and indices out of the
//! stub's buffers, which are memory). The Vulkan backend records it (`vk/pass.rs`);
//! Direct3D 12 and the stub's Direct3D 9 do not pass anything through yet.
use crate::stub::device::Dev;
use crate::stub::obj::{Body, SObj};
use crate::trace::once;
use crate::{log, sfmt};
use alloc::vec::Vec;
use core::ffi::c_void;

static mut ON: bool = false;
/// The frame now is one of the engine's loading loop (told at `BeginScene`), and the
/// UI's loading screen covers it: its draws are not drawn.
static mut LOADING: bool = false;

/// `Passthrough=` in `Display8.ini` (0 in the code; the shipped file has 1).
pub fn configure(ini: &crate::rt::Ini) {
    let on = ini.int("Display8", "Passthrough", 0) != 0;
    unsafe { ON = on };
    log!("Passthrough={}: {}", on as u8, if on { "the engine's own draws are drawn on Vulkan in stub mode" } else { "the engine's own draws are dropped in stub mode" });
}

pub fn on() -> bool {
    unsafe { ON }
}

/// The engine's `BeginScene` (`fp` its frame pointer): whether the frame is one of its
/// loading loop's, which the UI's loading screen draws over (ROADMAP phase 4: "keep one
/// drop for what the UI's loading screen overdraws"). Without `UI.asi` the engine's
/// loading screen is the only one, and is drawn.
pub fn begin_scene(fp: usize) {
    if !on() {
        return;
    }
    let ui = !unsafe { crate::rt::win::GetModuleHandleA(c"UI.asi".as_ptr() as *const u8) }.is_null();
    unsafe { LOADING = ui && crate::drop::in_loading(fp) };
}

/// Where a vertex's parts are, from its FVF.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Layout {
    /// The FVF's own size; the stream's stride is what the vertices are read at.
    pub size: u32,
    /// Pre-transformed (`XYZRHW`): screen pixels, z and 1/w.
    pub rhw: bool,
    pub normal: Option<u32>,
    pub diffuse: Option<u32>,
    pub specular: Option<u32>,
    /// Each texture coordinate set: (offset, floats).
    pub tex: [Option<(u32, u32)>; 8],
}

/// The FVF's layout. None: not one of fixed function's (a vertex shader's handle).
pub fn layout(fvf: u32) -> Option<Layout> {
    if fvf & 1 != 0 {
        return None;
    }
    let mut l = Layout::default();
    let mut at = match fvf & 0xe {
        0x2 => 12,
        0x4 => {
            l.rhw = true;
            16
        }
        p @ 0x6..=0xe => {
            once("PASSTHROUGH vertex blending (XYZB*): drawn unblended");
            12 + 4 * ((p - 0x4) / 2)
        }
        _ => return None,
    };
    if fvf & 0x10 != 0 {
        l.normal = Some(at);
        at += 12;
    }
    if fvf & 0x20 != 0 {
        at += 4; // point size
    }
    if fvf & 0x40 != 0 {
        l.diffuse = Some(at);
        at += 4;
    }
    if fvf & 0x80 != 0 {
        l.specular = Some(at);
        at += 4;
    }
    let sets = ((fvf >> 8) & 0xf).min(8);
    for i in 0..sets {
        let n = match (fvf >> (16 + 2 * i)) & 3 {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 1,
        };
        l.tex[i as usize] = Some((at, n));
        at += 4 * n;
    }
    l.size = at;
    Some(l)
}

/// One draw, as the backend records it.
pub struct Draw<'a> {
    /// `D3DPRIMITIVETYPE`; a fan comes as a list (`indices` made for it).
    pub prim: u32,
    pub layout: Layout,
    pub stride: u32,
    /// The vertices from the first the draw reads.
    pub verts: &'a [u8],
    /// The indices (16 or 32 bits), relative to `verts` after `base`.
    pub indices: Option<(&'a [u8], bool)>,
    /// Added to each index (or the first vertex of a draw without them).
    pub base: i32,
    /// Vertices drawn (no indices) or indices drawn.
    pub count: u32,
}

/// Vertices (or indices) a primitive count takes.
fn vertices_of(prim: u32, n: u32) -> u32 {
    match prim {
        1 => n,
        2 => 2 * n,
        3 => n + 1,
        4 => 3 * n,
        5 | 6 => n + 2,
        _ => 0,
    }
}

fn buf(p: *mut c_void) -> Option<&'static crate::stub::obj::Buf> {
    if !crate::stub::obj::is_stub(p) {
        return None;
    }
    match unsafe { &(*(p as *mut SObj)).body } {
        Body::Buf(b) => Some(b),
        _ => None,
    }
}

/// A fan's triangles as a list's indices: (0, i, i + 1), through `ix` when it is
/// indexed.
fn fan(n: u32, ix: Option<(&[u8], bool)>) -> Vec<u32> {
    let at = |i: u32| match ix {
        Some((b, false)) => u16::from_le_bytes([b[2 * i as usize], b[2 * i as usize + 1]]) as u32,
        Some((b, true)) => u32::from_le_bytes([b[4 * i as usize], b[4 * i as usize + 1], b[4 * i as usize + 2], b[4 * i as usize + 3]]),
        None => i,
    };
    let mut v = Vec::with_capacity(3 * n as usize);
    for i in 1..=n {
        v.extend_from_slice(&[at(0), at(i), at(i + 1)]);
    }
    v
}

/// One of the device's draws (`slot`: `DrawPrimitive` 70, `DrawIndexedPrimitive` 71,
/// `DrawPrimitiveUP` 72, `DrawIndexedPrimitiveUP` 73; `a` its arguments), passed through
/// when it is the engine's to draw.
pub fn draw(this: *mut SObj, fp: usize, slot: u32, a: &[usize]) {
    if !on() || !(70..=73).contains(&slot) {
        return;
    }
    if !crate::stub::vulkan() {
        once("PASSTHROUGH: the engine's draws are passed through on Vulkan only in this version: dropped");
        return;
    }
    if crate::drop::dropped(fp) || unsafe { LOADING } {
        return;
    }
    let dv: &Dev = crate::stub::device::state(this);
    if dv.target != dv.back_buffer {
        once(&sfmt!("PASSTHROUGH a draw into a target of the engine's own (not its back buffer): not drawn, by {}", crate::trace::caller(fp)));
        return;
    }
    let Some(lay) = layout(dv.vs) else {
        once(&sfmt!("PASSTHROUGH a draw through a vertex shader ({:#x}): not drawn (ROADMAP 4d), by {}", dv.vs, crate::trace::caller(fp)));
        return;
    };
    let prim = a[0] as u32;
    if !(1..=6).contains(&prim) {
        return;
    }
    let bad = |what: &str| once(&sfmt!("PASSTHROUGH {} out of its buffer: not drawn, by {}", what, crate::trace::caller(fp)));
    let mut made: Vec<u32>;
    let mut d = match slot {
        70 | 71 => {
            let Some(vb) = buf(dv.stream0.0) else { return };
            let stride = if dv.stream0.1 != 0 { dv.stream0.1 } else { lay.size };
            if slot == 70 {
                let (start, n) = (a[1] as u32, a[2] as u32);
                let nv = vertices_of(prim, n);
                let (s, e) = (start as usize * stride as usize, (start + nv) as usize * stride as usize);
                if e > vb.data.len() || nv == 0 {
                    bad("vertices");
                    return;
                }
                Draw { prim, layout: lay, stride, verts: &vb.data[s..e], indices: None, base: 0, count: nv }
            } else {
                let (min, nv, start, n) = (a[1] as u32, a[2] as u32, a[3] as u32, a[4] as u32);
                let Some(ib) = buf(dv.indices.0) else { return };
                let wide = ib.format == 102;
                let isz = if wide { 4 } else { 2 };
                let ni = vertices_of(prim, n);
                let (is, ie) = (start as usize * isz, (start + ni) as usize * isz);
                let first = dv.indices.1 + min;
                let (s, e) = (first as usize * stride as usize, (first + nv) as usize * stride as usize);
                if ie > ib.data.len() || e > vb.data.len() || ni == 0 {
                    bad("vertices or indices");
                    return;
                }
                Draw { prim, layout: lay, stride, verts: &vb.data[s..e], indices: Some((&ib.data[is..ie], wide)), base: -(min as i32), count: ni }
            }
        }
        72 => {
            let (n, v, stride) = (a[1] as u32, a[2] as *const u8, a[3] as u32);
            let nv = vertices_of(prim, n);
            if v.is_null() || stride == 0 || nv == 0 {
                return;
            }
            let verts = unsafe { core::slice::from_raw_parts(v, (nv * stride) as usize) };
            Draw { prim, layout: lay, stride, verts, indices: None, base: 0, count: nv }
        }
        _ => {
            let (min, nv, n, ix, fmt, v, stride) = (a[1] as u32, a[2] as u32, a[3] as u32, a[4] as *const u8, a[5] as u32, a[6] as *const u8, a[7] as u32);
            let ni = vertices_of(prim, n);
            if v.is_null() || ix.is_null() || stride == 0 || ni == 0 || nv == 0 {
                return;
            }
            let wide = fmt == 102;
            let isz = if wide { 4 } else { 2 };
            let verts = unsafe { core::slice::from_raw_parts(v.add((min * stride) as usize), (nv * stride) as usize) };
            let idx = unsafe { core::slice::from_raw_parts(ix, ni as usize * isz) };
            Draw { prim, layout: lay, stride, verts, indices: Some((idx, wide)), base: -(min as i32), count: ni }
        }
    };
    if d.stride < lay.size {
        bad("a stride under its FVF's size");
        return;
    }
    if prim == 6 {
        // Triangle fans as lists: a portability device (MoltenVK) has no fans.
        let n = d.count - 2;
        made = fan(n, d.indices);
        let bytes = unsafe { core::slice::from_raw_parts(made.as_mut_ptr() as *const u8, made.len() * 4) };
        d = Draw { prim: 4, indices: Some((bytes, true)), count: made.len() as u32, ..d };
    }
    if let Some(v) = crate::vk::get() {
        crate::vk::pass::draw(v, dv, &d, fp);
    }
}
