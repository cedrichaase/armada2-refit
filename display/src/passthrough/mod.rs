//! Passthrough (README.md, "Passthrough"): in stub mode, the engine's own Direct3D 8
//! draws are drawn after all, on Display8's device, the way Direct3D 8's fixed function
//! draws them. Whatever render3d or the UI does not draw itself (a module switched off,
//! a SOD render3d hands back, a HUD part the UI has not taken) then shows as stock
//! instead of vanishing.
//!
//! This half is the backend's neutral part: whether a draw is drawn at all, and what
//! it draws (its vertices and indices out of the stub's buffers, which are memory); the
//! translation of the engine's state (`state.rs`, host-tested) is neutral too. The
//! Vulkan backend records it (`vk/pass.rs`) and so does Direct3D 12 (`dx12/pass.rs`);
//! the stub's Direct3D 9 does not pass anything through yet.
pub mod state;

use crate::stub::device::Dev;
use crate::stub::obj::{Body, SObj};
use crate::trace::once;
use crate::{log, sfmt};
use alloc::vec::Vec;
use core::ffi::c_void;
pub use state::Layout;
use state::{bump_layout, fan, layout, vertices_of, Note};

static mut ON: bool = false;
/// The engine's scene now (told at its `BeginScene`) is one a plugin claims to draw over
/// (the loading loop's, the Credits', the cursor's: `drop::Scene`): its draws are not
/// drawn.
static mut CLAIMED: bool = false;

/// `Passthrough=` in `Display8.ini` (1, as the shipped file has it).
pub fn configure(ini: &crate::rt::Ini) {
    let on = ini.int("Display8", "Passthrough", 1) != 0;
    unsafe { ON = on };
    log!("Passthrough={}: {}", on as u8, if on { "the engine's own draws are drawn on Vulkan and Direct3D 12 in stub mode" } else { "the engine's own draws are dropped in stub mode" });
}

pub fn on() -> bool {
    unsafe { ON }
}

/// The engine's `BeginScene` (`fp` its frame pointer): every plugin's claims asked
/// (`claims::ask`), and whether the scene is one of the engine's that a plugin claims to
/// draw over (ROADMAP Phase 4, "Claims"): the loading loop's (`CLAIM_LOADING`, the UI's
/// loading screen), the Credits' (`CLAIM_CREDITS`) or the cursor's (`CLAIM_CURSOR`).
/// What nobody claims is drawn: without the UI, or with its part at 0, or with a UI that
/// stood down, the engine's loading screen is the only one.
pub fn begin_scene(fp: usize) {
    crate::api::scene_begun(fp);
    if !on() {
        return;
    }
    let claimed = crate::claims::ask();
    let want = match crate::drop::scene(fp) {
        crate::drop::Scene::Loading => gpu::display::CLAIM_LOADING,
        crate::drop::Scene::Credits => gpu::display::CLAIM_CREDITS,
        crate::drop::Scene::Cursor => gpu::display::CLAIM_CURSOR,
        crate::drop::Scene::Other => 0,
    };
    let drop = claimed & want != 0;
    if drop && unsafe { !CLAIMED } {
        once(&sfmt!("PASSTHROUGH claims: a scene of the engine's ({}) is claimed by a plugin: its draws are not drawn", crate::claims::names(want)));
    }
    unsafe { CLAIMED = drop };
}

/// The hash (`stub::device::token_hash`) of the engine's one vertex shader's function
/// tokens: the Borg hulls' bump pass, ported by hand as `vs_bump` in `shaders/ff.hlsl`
/// (ROADMAP 4d). Any other shader is refused and logged.
pub const BORG_BUMP: u32 = 0xaaa1_e806;

/// A vertex shader's layout, when it is one the passthrough has a port of: its
/// declaration read for where each input register is in stream 0 (`state::bump_layout`).
fn shader_layout(dv: &Dev) -> Option<Layout> {
    let sh = dv.shaders.iter().find(|s| s.handle == dv.vs)?;
    if sh.hash != BORG_BUMP {
        return None;
    }
    bump_layout(&sh.decl)
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

fn buf(p: *mut c_void) -> Option<&'static crate::stub::obj::Buf> {
    if !crate::stub::obj::is_stub(p) {
        return None;
    }
    match unsafe { &(*(p as *mut SObj)).body } {
        Body::Buf(b) => Some(b),
        _ => None,
    }
}

/// One of the device's draws (`slot`: `DrawPrimitive` 70, `DrawIndexedPrimitive` 71,
/// `DrawPrimitiveUP` 72, `DrawIndexedPrimitiveUP` 73; `a` its arguments), passed through
/// when it is the engine's to draw.
pub fn draw(this: *mut SObj, fp: usize, slot: u32, a: &[usize]) {
    if !on() || !(70..=73).contains(&slot) {
        return;
    }
    let backend = if crate::stub::vulkan() {
        Backend::Vulkan
    } else if crate::stub::d3d12() {
        Backend::D3d12
    } else {
        once("PASSTHROUGH: the engine's draws are passed through on Vulkan and Direct3D 12 only in this version: dropped");
        return;
    };
    if crate::drop::dropped(fp) || unsafe { CLAIMED } {
        return;
    }
    let dv: &Dev = crate::stub::device::state(this);
    if dv.target != dv.back_buffer {
        once(&sfmt!("PASSTHROUGH a draw into a target of the engine's own (not its back buffer): not drawn, by {}", crate::trace::caller(fp)));
        return;
    }
    let Some(lay) = layout(dv.vs).or_else(|| shader_layout(dv)) else {
        once(&sfmt!("PASSTHROUGH a draw through a vertex shader ({:#x}): not one the passthrough has a port of: not drawn, by {}", dv.vs, crate::trace::caller(fp)));
        return;
    };
    if lay.blended {
        once("PASSTHROUGH vertex blending (XYZB*): drawn unblended");
    }
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
    match backend {
        Backend::Vulkan => {
            if let Some(v) = crate::vk::get() {
                crate::vk::pass::draw(v, dv, &d, fp);
            }
        }
        Backend::D3d12 => {
            if let Some(x) = crate::dx12::get() {
                crate::dx12::pass::draw(x, dv, &d, fp);
            }
        }
    }
}

enum Backend {
    Vulkan,
    D3d12,
}

/// The device state a draw reads, borrowed for `state`.
pub fn fixed(dv: &Dev) -> state::Fixed<'_> {
    state::Fixed {
        rs: &dv.rs,
        tss: &dv.tss,
        transforms: &dv.transforms,
        material: &dv.material,
        lights: &dv.lights,
        vs_const: &dv.vs_const,
        ps: dv.ps,
        zbuffer: !dv.zbuffer.is_null(),
    }
}

static mut BUMP_LIT: bool = false;

/// Says once what `state` found of state it does not draw as Direct3D 8 would.
pub fn say(n: Note) {
    match n {
        Note::Fog => once("PASSTHROUGH fog is on: drawn without fog"),
        Note::FillMode(m) => once(&sfmt!("PASSTHROUGH FILLMODE {}: drawn solid", m)),
        Note::ZBias(z) => once(&sfmt!("PASSTHROUGH ZBIAS {}: drawn without a depth bias", z)),
        Note::ColorOp(s, op) => once(&sfmt!("PASSTHROUGH stage {} COLOROP {}: drawn as SELECTARG1", s, op)),
        Note::TextureTransform => once("PASSTHROUGH a texture transform: drawn without it"),
        Note::GeneratedCoordinates => once("PASSTHROUGH generated texture coordinates: drawn with the set's own"),
        Note::ThirdStage => once("PASSTHROUGH a third texture stage: drawn with two"),
        Note::PixelShader => once("PASSTHROUGH a pixel shader: drawn with fixed function"),
        Note::SpotLight => once("PASSTHROUGH a spot light: lit as a point light"),
        // Logged once whether tracing or not: the engine's bump passes with LIGHTING on.
        Note::BumpLit => unsafe {
            if !BUMP_LIT {
                BUMP_LIT = true;
                log!("passthrough: the Borg bump shader drawn with LIGHTING on: the lights left out, as Direct3D ignores them under a vertex shader");
            }
        },
    }
}
