//! Passthrough on Direct3D 12 (README.md, "Passthrough"): each of the engine's draws
//! that `passthrough/mod.rs` lets through, recorded into the frame's command list when
//! it is made, through `shaders/ff.hlsl` (DXIL), as `vk/pass.rs` does on Vulkan.
//!
//! - **Where it lands**: the pass a plugin published in this frame (`exports::published`:
//!   render3d's scene pass while its body runs, so what it hands back is drawn into its
//!   scene, tone-mapped and bloomed with the rest; the last one published is the one),
//!   else the engine's targets (Display8's colour and depth). A published colour that is
//!   Display8's own colour target is the engine's targets. Nothing is recorded while a
//!   plugin's `Cmd` is alive (a draw then is logged once and not drawn).
//! - **Nothing is open between draws.** Direct3D 12 has no render pass, so a draw sets
//!   what it needs and the next draw sets only what differs. What was set is forgotten
//!   (`forget`) whenever someone else may have recorded: a plugin's `Cmd` begins or ends
//!   (`display8v1_dx12_pass`), a flush, a new frame. Display8 never sets descriptor
//!   heaps (the frame's two are set when it begins), never samples, copies or changes
//!   the state of a pass it was given, and records into the list only what binds
//!   nothing between `Cmd`s (`gpu/src/display.rs`, "The rebind rule").
//! - **One root signature**: the vertex block as a root CBV at b0 in space 0 (`vs_ff`)
//!   and in space 1 (`vs_bump`), the pixel block as one at b1, a table of t0 and t1 and
//!   a table of s0 and s1 (`CB_SPACES`). The blocks are in the frame's ring at 256
//!   alignment, the pixel block at `PS_AT`; a block equal to the last is not sent
//!   again. The tables are copied into the frame's descriptor ranges, one per distinct
//!   pair in a frame; the samplers are made there directly.
//! - **Pipelines** by what Direct3D 12 bakes into one: the target's formats and sample
//!   count, topology type, vertex layout, blend, depth, rasterizer cull. What the
//!   vertices lack comes from a defaults record in slot 1, read once per draw as
//!   instance data (a stride of 0 is not asked for).
//! - **Textures** are the stub's mirrors (`mirror::passthrough`), or a white one;
//!   samplers by the stage's state as DXVK makes them under `[Samplers]` or `dxvk.conf` (`crate::dxvk_samplers`).
use super::build::{self, Key};
use super::exports::{cmd_alive, published};
use super::raw::*;
use super::{Dx, Img};
use crate::passthrough::state::{self, Blocks, SKey, PS_AT, PS_BYTES, VS_BYTES};
use crate::passthrough::{fixed, say, Draw};
use crate::stub::device::Dev;
use crate::stub::obj::SObj;
use crate::trace::once;
use crate::{log, sfmt};
use alloc::vec::Vec;
use gpu::display::{HEAP_CBV_SRV_UAV, HEAP_SAMPLER};

/// Where a draw lands.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    rtv: usize,
    dsv: usize,
    color: u32,
    depth: u32,
    samples: u32,
    w: u32,
    h: u32,
    own: bool,
}

/// What is set on the list now (all of it, after a `forget`: nothing).
#[derive(Clone, Copy, PartialEq)]
struct Bound {
    root: bool,
    pipeline: usize,
    target: Option<Target>,
    viewport: [u32; 6],
    topology: u32,
    blocks: u64,
    textures: u64,
    samplers: u64,
}

const NOTHING: Bound = Bound { root: false, pipeline: 0, target: None, viewport: [u32::MAX; 6], topology: 0, blocks: 0, textures: 0, samplers: 0 };

struct Pt {
    root: P,
    pipelines: Vec<(Key, P)>,
    white: Img,
    white_ready: bool,
    /// The frame (Display8's index) the per-frame state below belongs to.
    frame: u64,
    /// Table handles (GPU) of a frame's pairs of textures (their views) and samplers.
    tex_tables: Vec<([usize; 2], u64)>,
    samp_tables: Vec<([SKey; 2], u64)>,
    /// The defaults record's GPU address in the frame's ring.
    defaults: Option<u64>,
    /// The blocks last written in this frame, and where.
    last: Option<(Blocks, u64)>,
    bound: Bound,
    env: state::SamplerEnv,
    centres: bool,
    drawn: u64,
    failed: bool,
}

static mut PT: Option<Pt> = None;

#[allow(static_mut_refs)]
fn pt() -> Option<&'static mut Pt> {
    unsafe { PT.as_mut() }
}

/// Someone else may have recorded on the list (a plugin's `Cmd` began or ended, the
/// list was flushed): nothing of what was set can be assumed.
pub fn forget() {
    if let Some(p) = pt() {
        p.bound = NOTHING;
    }
}

fn make(d: &mut Dx) -> bool {
    if unsafe { PT.is_some() } {
        return true;
    }
    let note = |m: &str| log!("d3d12: D3D12SerializeRootSignature (passthrough): {}", m);
    let Some(root) = build::pass_root(d.dev, d.exported.serialize_root_signature, &note).inspect_err(|e| log!("d3d12: {} (passthrough): {:#x}", e.0, e.1 as u32)).ok() else {
        log!("passthrough: its root signature could not be made: the engine's draws are dropped");
        return false;
    };
    let white = match d.texture(1, 1, 1, FORMAT_B8G8R8A8_UNORM, 0, STATE_COPY_DEST, Some(DEFAULT_COMPONENT_MAPPING)) {
        Ok(w) => w,
        Err(e) => {
            log!("passthrough: its white texture could not be made ({}): the engine's draws are dropped", e);
            release(root);
            return false;
        }
    };
    let env = state::SamplerEnv {
        forced_anisotropy: d.exported.sampler_anisotropy,
        forced_bias: d.exported.sampler_lod_bias,
        have_anisotropy: d.exported.features & gpu::display::FEATURE_ANISOTROPY != 0,
        max_bias: 15.99,
        max_anisotropy: 16.0,
    };
    let centres = d.exported.features & gpu::display::FEATURE_D3D9_CENTRES != 0;
    unsafe {
        PT = Some(Pt {
            root,
            pipelines: Vec::new(),
            white,
            white_ready: false,
            frame: u64::MAX,
            tex_tables: Vec::new(),
            samp_tables: Vec::new(),
            defaults: None,
            last: None,
            bound: NOTHING,
            env,
            centres,
            drawn: 0,
            failed: false,
        })
    };
    log!("passthrough: on Direct3D 12; the engine's draws go into the pass render3d publishes, else the engine's targets");
    true
}

/// A new frame: what the last one made per frame is let go of (its descriptors and ring
/// are the frame slot's, which come round again).
fn new_frame(d: &Dx, p: &mut Pt) {
    if p.frame == d.index {
        return;
    }
    p.frame = d.index;
    p.tex_tables.clear();
    p.samp_tables.clear();
    p.defaults = None;
    p.last = None;
    p.bound = NOTHING;
}

fn pipeline(d: &mut Dx, p: &mut Pt, k: &Key) -> P {
    if let Some(x) = p.pipelines.iter().find(|x| x.0 == *k) {
        return x.1;
    }
    let pso = match build::pass_pipeline(d.dev, p.root, k) {
        Ok(o) => o,
        Err(hr) => {
            once(&sfmt!("PASSTHROUGH a pipeline could not be made ({:#x}): its draws are not drawn", hr as u32));
            core::ptr::null_mut()
        }
    };
    p.pipelines.push((*k, pso));
    if p.pipelines.len().is_power_of_two() {
        log!("passthrough: {} pipelines", p.pipelines.len());
    }
    pso
}

/// A sampler's description (`state::sampler_params`) in Direct3D 12's terms.
fn sampler_desc(q: &state::SamplerParams) -> SamplerDesc {
    // D3D12_FILTER: bit 4 minification, bit 2 magnification, bit 0 mip, linear when
    // set; 0x55 anisotropic.
    let filter = if q.anisotropy.is_some() { 0x55 } else { (q.min_linear as u32) << 4 | (q.mag_linear as u32) << 2 | q.mip_linear as u32 };
    SamplerDesc {
        filter,
        address_u: q.address_u,
        address_v: q.address_v,
        address_w: 1,
        mip_lod_bias: q.lod_bias,
        max_anisotropy: q.anisotropy.map_or(1, |a| a as u32),
        comparison: COMPARISON_FUNC_ALWAYS,
        border: [0.0; 4],
        min_lod: q.min_lod,
        max_lod: q.max_lod,
    }
}

/// The texture stage `s` reads (a mirror, made at its first draw): its view in Display8's
/// heap, or the white one's.
fn texture(d: &mut Dx, p: &mut Pt, dv: &Dev, s: usize) -> usize {
    let t = dv.textures[s];
    if !t.is_null() && crate::stub::obj::is_stub(t) {
        let o = t as *mut SObj;
        if unsafe { (*o).kind } == crate::com::Kind::Texture {
            if let Some(m) = super::mirror::passthrough(o) {
                return m.srv;
            }
        } else {
            once("PASSTHROUGH a cube or volume texture: drawn with a white one");
        }
    }
    white(d, p)
}

/// The 1x1 white texture a stage without one reads.
fn white(d: &mut Dx, p: &mut Pt) -> usize {
    if !p.white_ready {
        p.white_ready = true;
        let res = p.white.res;
        d.upload(res, FORMAT_B8G8R8A8_UNORM, 0, 1, 1, &[255, 255, 255, 255], 4, 4, STATE_COPY_DEST, STATE_SHADER_RESOURCE);
        p.white.state = STATE_SHADER_RESOURCE;
    }
    p.white.srv
}


/// Records one draw of the engine's (`passthrough::draw`).
pub fn draw(d: &mut Dx, dv: &Dev, dr: &Draw, fp: usize) {
    if pt().is_none() && !make(d) {
        return;
    }
    let Some(p) = pt() else { return };
    if p.failed || d.lost || d.frame().is_none() {
        return;
    }
    new_frame(d, p);
    if cmd_alive() {
        once(&sfmt!("PASSTHROUGH an engine draw while a plugin records: not drawn, by {}", crate::trace::caller(fp)));
        return;
    }
    let fx = fixed(dv);
    state::notes(&fx, &mut say);

    // Where it goes: the pass published in this frame, unless that is the engine's own
    // back buffer (then the engine's targets, with their depth).
    let own = Target { rtv: d.color.rtv, dsv: d.depth.dsv, color: d.color.format, depth: d.depth.format, samples: 1, w: d.color.w, h: d.color.h, own: true };
    let target = match published() {
        Some(x) if x.colour.resource != d.color.res as usize => {
            let has_depth = x.depth.resource != 0 && x.dsv != 0;
            Target {
                rtv: x.rtv,
                dsv: if has_depth { x.dsv } else { 0 },
                color: x.colour.format,
                depth: if has_depth { x.depth.format } else { FORMAT_UNKNOWN },
                samples: x.samples.max(1),
                w: x.width,
                h: x.height,
                own: false,
            }
        }
        _ => own,
    };
    if target.w == 0 || target.h == 0 || target.rtv == 0 {
        return;
    }

    // Textures first: a mirror made or brought up to date records copies and barriers.
    let stage_on = |s: usize| dv.tss[s][1] > 1 && (s == 0 || dv.tss[0][1] > 1);
    let mut views = [0usize; 2];
    for s in 0..2 {
        views[s] = if stage_on(s) { texture(d, p, dv, s) } else { white(d, p) };
    }
    let skeys = [state::sampler_key(&dv.tss[0]), state::sampler_key(&dv.tss[1])];

    let vp = state::clamp_viewport(dv.viewport, target.w, target.h);
    let key = Key {
        color: target.color,
        depth: target.depth,
        samples: target.samples,
        stride: dr.stride,
        layout: dr.layout,
        st: state::pipe_state(&fx, dr.prim),
        no_dst_alpha: target.own,
    };
    let pso = pipeline(d, p, &key);
    if pso.is_null() {
        return;
    }

    // The constant blocks, unless the same as the last draw's in this frame.
    let bl = state::blocks(&fx, &dr.layout, vp, p.centres, &mut say);
    let ubuf = match &p.last {
        Some((b, gpu)) if b[..] == bl[..] => *gpu,
        _ => {
            let Some(a) = d.alloc(PS_AT + 256, 256) else { return };
            unsafe {
                core::ptr::copy_nonoverlapping(bl.as_ptr(), a.cpu, VS_BYTES);
                core::ptr::copy_nonoverlapping(bl.as_ptr().add(VS_BYTES), a.cpu.add(PS_AT as usize), PS_BYTES);
            }
            p.last = Some((bl, a.gpu));
            a.gpu
        }
    };
    // The vertices, the indices, the defaults.
    let Some(va) = d.alloc(dr.verts.len() as u32, 16) else { return };
    unsafe { core::ptr::copy_nonoverlapping(dr.verts.as_ptr(), va.cpu, dr.verts.len()) };
    let ib = match dr.indices {
        Some((ix, wide)) => {
            let Some(a) = d.alloc(ix.len() as u32, 4) else { return };
            unsafe { core::ptr::copy_nonoverlapping(ix.as_ptr(), a.cpu, ix.len()) };
            Some(IndexBufferView { location: a.gpu, size: ix.len() as u32, format: if wide { FORMAT_R32_UINT } else { FORMAT_R16_UINT } })
        }
        None => None,
    };
    let defaults = match p.defaults {
        Some(x) => x,
        None => {
            let Some(a) = d.alloc(state::DEFAULTS_BYTES as u32, 16) else { return };
            let b = state::defaults();
            unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), a.cpu, b.len()) };
            p.defaults = Some(a.gpu);
            a.gpu
        }
    };

    // The tables: the pair of views and the pair of samplers, one per distinct pair in a
    // frame.
    let tex = match p.tex_tables.iter().find(|x| x.0 == views) {
        Some(x) => x.1,
        None => {
            let Some(t) = d.descriptors(HEAP_CBV_SRV_UAV, 2) else {
                once("PASSTHROUGH the frame's descriptors are used up: draws are not drawn");
                return;
            };
            for (i, v) in views.iter().enumerate() {
                vcall!(d.dev, slot::DEV_COPY_DESCRIPTORS_SIMPLE, fn(u32, usize, usize, u32) -> (), 1, t.cpu + i * d.inc[0] as usize, *v, DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
            }
            p.tex_tables.push((views, t.gpu));
            t.gpu
        }
    };
    let smp = match p.samp_tables.iter().find(|x| x.0 == skeys) {
        Some(x) => x.1,
        None => {
            let Some(t) = d.descriptors(HEAP_SAMPLER, 2) else {
                once("PASSTHROUGH the frame's sampler descriptors are used up: draws are not drawn");
                return;
            };
            for (i, k) in skeys.iter().enumerate() {
                let desc = sampler_desc(&state::sampler_params(k, &p.env));
                vcall!(d.dev, slot::DEV_CREATE_SAMPLER, fn(*const SamplerDesc, usize) -> (), &desc, t.cpu + i * d.inc[1] as usize);
            }
            p.samp_tables.push((skeys, t.gpu));
            t.gpu
        }
    };

    // Set what differs from what is set.
    let l = d.list();
    let b = &mut p.bound;
    if !b.root {
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), p.root);
        *b = Bound { root: true, ..NOTHING };
    }
    if b.pipeline != pso as usize {
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), pso);
        b.pipeline = pso as usize;
    }
    if b.target != Some(target) {
        let dsv = target.dsv;
        let rtv = target.rtv;
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 1, &rtv, 0, if dsv != 0 { &dsv as *const usize } else { core::ptr::null() });
        b.target = Some(target);
    }
    if b.viewport != vp {
        let v = Viewport { x: vp[0] as f32, y: vp[1] as f32, w: vp[2] as f32, h: vp[3] as f32, min_z: f32::from_bits(vp[4]).clamp(0.0, 1.0), max_z: f32::from_bits(vp[5]).clamp(0.0, 1.0) };
        let sc = Rect { left: vp[0] as i32, top: vp[1] as i32, right: (vp[0] + vp[2]) as i32, bottom: (vp[1] + vp[3]) as i32 };
        vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport) -> (), 1, &v);
        vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const Rect) -> (), 1, &sc);
        b.viewport = vp;
    }
    if b.topology != key.st.topology {
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), key.st.topology);
        b.topology = key.st.topology;
    }
    if b.blocks != ubuf {
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 0, ubuf);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 1, ubuf);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 2, ubuf + PS_AT as u64);
        b.blocks = ubuf;
    }
    if b.textures != tex {
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 3, tex);
        b.textures = tex;
    }
    if b.samplers != smp {
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 4, smp);
        b.samplers = smp;
    }
    let vbs = [
        VertexBufferView { location: va.gpu, size: dr.verts.len() as u32, stride: dr.stride },
        VertexBufferView { location: defaults, size: state::DEFAULTS_BYTES as u32, stride: state::DEFAULTS_BYTES as u32 },
    ];
    vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 0, 2, vbs.as_ptr());
    match ib {
        Some(v) => {
            vcall!(l, slot::LIST_IA_SET_INDEX_BUFFER, fn(*const IndexBufferView) -> (), &v);
            vcall!(l, slot::LIST_DRAW_INDEXED_INSTANCED, fn(u32, u32, u32, i32, u32) -> (), dr.count, 1, 0, dr.base, 0);
        }
        None => {
            vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), dr.count, 1, 0, 0);
        }
    }
    p.drawn += 1;
    crate::host::engine_drew();
    if p.drawn.is_power_of_two() && p.drawn >= 1024 {
        log!("passthrough: {} draws of the engine's so far", p.drawn);
    } else if p.drawn == 1 {
        log!("passthrough: the engine's first draw, into {}", if target.own { "the engine's targets" } else { "a plugin's pass" });
    }
}
