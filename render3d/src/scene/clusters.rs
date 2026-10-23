//! Clustered lights (`[Lights] Clustered=1`; README.md, "Clustered lights"): on Vulkan
//! and Direct3D 12, every light of the view lights the hulls, planets and rocks near it,
//! per pixel, with no cap per mesh.
//!
//! Once per view, after every module has registered its lights (`build`, from the
//! body after `mods::begin_view`): the engine's list as the scene read it
//! (`scene::lights()`: torpedoes, pulses, shield hits, welds, vents, fireballs, the
//! modules' own points) and the shapes render3d adds of its own (`add_line`: a phaser
//! beam as a line light; `add_spot`: a tractor or assimilation beam's wash, confined to
//! its cone) go into a storage buffer, and a compute pass (`shaders/clusters.hlsl`)
//! bins them into screen tiles by slices of view depth: each cluster lists the lights
//! that reach its box, at most `ClusterLights` and of one colour at most
//! `ClusterPerColour`, the strongest there. A lit draw binds the view's buffers
//! (`bind`) and draws with its shader's clustered twin (`shaders/clustered.hlsli`),
//! which walks its pixel's cluster and, of the lights of one colour (a volley), takes
//! only the strongest at that pixel, as `pick_points` took the strongest per mesh.
//!
//! Each view has buffers of its own (`View::key`: the HUD's action camera draws the
//! scene a second time each frame). Render only: nothing here is read by the game.
//! On Direct3D 9, or with `Clustered=0`, nothing of this runs and every draw takes its
//! lights from its constants as before.
use super::lights;
use crate::frame::steps::View;
use crate::gfx;
use crate::math::V3;
use crate::shaders::clusters::*;
use crate::shaders::clusters_dxil::*;
use crate::shaders::clusters_spv::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use gpu::{Block, Cmd, ComputeDesc, ComputePipeline, Storage, StorageDesc};

/// The most lights a view takes, and a cluster.
const MAX_LIGHTS: usize = 2048;
const MAX_PER_CLUSTER: u32 = 32;
/// The most tiles across and down.
const MAX_TILES: (u32, u32) = (64, 40);
/// Pixel constants c100..c103 (`clustered.hlsli`).
pub const PS_BASE: u32 = 100;

const KIND_LINE: u32 = 1;
const KIND_SPOT: u32 = 2;
const DAY_SIDE: u32 = 16;

/// `[Lights]` of Render3D.ini, clustered part.
pub struct Config {
    pub on: bool,
    pub tile: u32,
    pub slices: u32,
    pub per_cluster: u32,
    pub per_colour: u32,
    /// A phaser beam's line light (`add_line`): its brightness beside the middle of a
    /// long beam at `beam_core` from it (times the beam's colour, 0: none), falling as
    /// 1 / distance and gone at `beam_reach`; how far round the plating it wraps; its
    /// reach in the gas (the nebulae's march, as it was tuned).
    pub beam: f32,
    pub beam_core: f32,
    pub beam_reach: f32,
    pub beam_wrap: f32,
    pub beam_gas_reach: f32,
    /// The tractor and assimilation beams' wash as a spot in the cone (else a point).
    pub cone_spot: bool,
}

/// One light as the shaders read it (`ClLight`, 80 bytes).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct GpuLight {
    a: [f32; 4],
    c: [f32; 4],
    b: [f32; 4],
    k: [f32; 4],
    t: [u32; 4],
}

/// A light of render3d's own that is not a point (or a point that is not in the
/// engine's list), added for the view being drawn.
struct Extra {
    l: GpuLight,
    distinct: bool,
}

#[derive(Default)]
struct ViewBufs {
    lights: Option<Storage>,
    index: Option<Storage>,
    cap_lights: usize,
    cap_index: u32,
}

struct State {
    cfg: Option<Config>,
    pipe: Option<ComputePipeline>,
    views: gfx::PerView<ViewBufs>,
    extra: Vec<Extra>,
    /// The view built last: (frame, key, camera), its pixel constants.
    built: Option<(u32, usize, usize)>,
    ps: [f32; 16],
    said: bool,
    /// The lights and clusters of the last main view, for the log.
    last: (usize, u32),
}

static mut STATE: State = State {
    cfg: None,
    pipe: None,
    views: gfx::PerView::new(),
    extra: Vec::new(),
    built: None,
    ps: [0.0; 16],
    said: false,
    last: (0, 0),
};

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

pub fn config() -> &'static Config {
    st().cfg.get_or_insert_with(|| {
        let ini = crate::config::ini();
        let s = "Lights";
        Config {
            on: ini.int(s, "Clustered", 1) != 0,
            tile: ini.int(s, "ClusterTile", 64).clamp(16, 256) as u32,
            slices: ini.int(s, "ClusterSlices", 24).clamp(4, 64) as u32,
            per_cluster: (ini.int(s, "ClusterLights", 24).clamp(1, MAX_PER_CLUSTER as i32)) as u32,
            per_colour: ini.int(s, "ClusterPerColour", 2).clamp(1, MAX_PER_CLUSTER as i32) as u32,
            beam: ini.float(s, "BeamLight", BEAM_LIGHT).max(0.0),
            beam_core: ini.float(s, "BeamCore", 8.0).max(0.5),
            beam_reach: ini.float(s, "BeamReach", 200.0).max(1.0),
            beam_wrap: ini.float(s, "BeamWrap", 0.4).clamp(0.0, 1.0),
            beam_gas_reach: ini.float(s, "BeamGasReach", 40.0).max(1.0),
            cone_spot: ini.int(s, "ConeSpot", 1) != 0,
        }
    })
}

/// Whether the lit draws take their lights from the clusters: `Clustered=1`, on a
/// backend with compute (Vulkan, Direct3D 12), its compute shader made.
pub fn on() -> bool {
    let c = config();
    if !c.on {
        return false;
    }
    let g = gfx::gpu();
    if !g.compute() {
        return false;
    }
    let s = st();
    let p = s.pipe.get_or_insert_with(|| g.compute_pipeline(ComputeDesc { cs: gpu::code!(CS_CLUSTER, CS_CLUSTER_SPV, CS_CLUSTER_DXIL) }));
    let ok = p.ready(g);
    if !s.said {
        s.said = true;
        crate::log!(
            "lights: clustered lights {} on {} (tiles of {} px, {} slices, {} lights a cluster, {} of a colour)",
            if ok { "on" } else { "could not start" },
            g.backend_name(),
            c.tile,
            c.slices,
            c.per_cluster,
            c.per_colour
        );
    }
    ok
}

/// The view's lights start again (`scene::begin_view`).
pub fn begin_view() {
    st().extra.clear();
    st().built = None;
}

/// A view that is not the engine's (the menu scene) has no clusters.
pub fn forget_view() {
    st().extra.clear();
    st().built = None;
}

/// `BeamLight`'s default.
pub const BEAM_LIGHT: f32 = 4.0;

/// A phaser beam from `a` to `b` as a line light for the view being drawn (call from a
/// module's `begin_view`), in the beam's `colour` (its peak 1 at full strength): every
/// unit of it emits alike (`clustered.hlsli`, `cl_line`), `BeamLight` beside the middle
/// of a long beam at `BeamCore` from it, falling as 1 / distance, gone at `BeamReach`,
/// wrapped `BeamWrap` round the plating. In the gas it is the beam's colour from its
/// nearest point out to `BeamGasReach`. A light of its own (never merged).
pub fn add_line(a: V3, b: V3, colour: V3) {
    let c = config();
    if c.beam <= 0.0 {
        return;
    }
    let core = c.beam_core.min(c.beam_reach * 0.5);
    let col = colour * c.beam;
    let l = GpuLight {
        a: [a.x, a.y, a.z, core],
        c: [col.x, col.y, col.z, 1.0 / (c.beam_reach - core).max(1e-3)],
        b: [b.x, b.y, b.z, c.beam_gas_reach],
        k: [c.beam_wrap, 2.0, 0.0, 1.0 / c.beam],
        t: [KIND_LINE, 0, 0, 0],
    };
    st().extra.push(Extra { l, distinct: true });
}

/// A spot from `apex` along `axis` (unit) for the view being drawn: lights what lies
/// inside the cone (half width `hw0` at the apex, growing by `slope` per unit along
/// it, soft at its side) out to `reach` along the axis, fading over its last `fade`.
#[allow(clippy::too_many_arguments)]
pub fn add_spot(apex: V3, axis: V3, reach: f32, hw0: f32, slope: f32, colour: V3, fade: f32, wrap: f32) {
    let l = GpuLight {
        a: [apex.x, apex.y, apex.z, 0.0],
        c: [colour.x, colour.y, colour.z, 1.0 / fade.max(1e-3)],
        b: [axis.x, axis.y, axis.z, reach],
        k: [wrap, 1.0, hw0, slope],
        t: [KIND_SPOT, 0, 0, 0],
    };
    st().extra.push(Extra { l, distinct: true });
}

/// The view's lights, in groups: each a light of its own or the lights of one colour
/// (`Style::distinct` makes one its own, as `pick_points` keeps them apart).
fn gather() -> Vec<GpuLight> {
    let s = st();
    let ls = super::lights();
    let key = ls.key();
    let mut v: Vec<(u32, GpuLight)> = Vec::with_capacity(ls.points.len() + s.extra.len());
    let mut groups: BTreeMap<[u32; 3], u32> = BTreeMap::new();
    let mut next = 0u32;
    let mut group = |col: V3, distinct: bool| -> u32 {
        next += 1;
        if distinct {
            return next - 1;
        }
        let k = [col.x.to_bits(), col.y.to_bits(), col.z.to_bits()];
        *groups.entry(k).or_insert(next - 1)
    };
    for p in &ls.points {
        if v.len() >= MAX_LIGHTS {
            break;
        }
        let day = lights::is_day_side(p.light);
        let fade = if p.range > 1e-3 { 1.0 / p.range } else { 1000.0 };
        let kd = key.map_or(V3::ZERO, |k| k.dir);
        let g = group(p.colour, p.style.distinct || day);
        v.push((
            g,
            GpuLight {
                a: [p.pos.x, p.pos.y, p.pos.z, p.start],
                c: [p.colour.x, p.colour.y, p.colour.z, fade],
                b: [kd.x, kd.y, kd.z, 0.0],
                k: [p.style.wrap, p.style.power, 0.0, 0.0],
                t: [if day && key.is_some() { DAY_SIDE } else { 0 }, g, lights::gas_of(p.light) as u32, 0],
            },
        ));
    }
    for e in &s.extra {
        if v.len() >= MAX_LIGHTS {
            break;
        }
        let g = group(crate::math::v3(e.l.c[0], e.l.c[1], e.l.c[2]), e.distinct);
        let mut l = e.l;
        l.t[1] = g;
        v.push((g, l));
    }
    v.sort_by_key(|e| e.0);
    v.into_iter().map(|e| e.1).collect()
}

/// After every module has added its lights for the view: the view's lights uploaded and
/// binned (a compute pass), for the draws of this view to `bind`.
pub fn build(v: &View) {
    if !on() {
        return;
    }
    let _z = crate::timing::zone("light clusters");
    let s = st();
    let c = config();
    let list = gather();
    let (w, h) = (v.rect.w.max(1), v.rect.h.max(1));
    let tile = c.tile.max(w.div_ceil(MAX_TILES.0)).max(h.div_ceil(MAX_TILES.1));
    let (nx, ny, ns) = (w.div_ceil(tile), h.div_ceil(tile), c.slices);
    let k = c.per_cluster.min(MAX_PER_CLUSTER);
    let clusters = nx * ny * ns;
    let words = clusters * (k + 1);
    // Depth: exponential slices between the projection's near and far planes.
    let p = &v.proj.0;
    let (a, b) = (p[2][2], p[3][2]);
    let near = if a.abs() > 1e-9 { (-b / a).max(0.1) } else { 1.0 };
    let far = if a > 1.0001 { (a * near / (a - 1.0)).min(near * 1.0e5) } else { near * 1.0e4 };
    let span = libm::log2f(far / near).max(1.0);
    let zscale = ns as f32 / span;
    let zbias = -libm::log2f(near) * zscale;
    let g = gfx::gpu();
    let bufs = s.views.get(v.key());
    let n = list.len().max(1);
    if bufs.lights.is_none() || bufs.cap_lights < n {
        let cap = n.next_power_of_two().max(64);
        bufs.lights = Some(g.storage(StorageDesc { size: (cap * core::mem::size_of::<GpuLight>()) as u32, stride: core::mem::size_of::<GpuLight>() as u32 }));
        bufs.cap_lights = cap;
    }
    if bufs.index.is_none() || bufs.cap_index < words {
        bufs.index = Some(g.storage(StorageDesc { size: words * 4, stride: 4 }));
        bufs.cap_index = words;
    }
    let (Some(lb), Some(ib)) = (bufs.lights.clone(), bufs.index.clone()) else { return };
    let Some(pipe) = s.pipe.clone() else { return };
    let Some(mut cmd) = gfx::cmd() else { return };
    if !list.is_empty() {
        let bytes = unsafe { core::slice::from_raw_parts(list.as_ptr() as *const u8, core::mem::size_of_val(list.as_slice())) };
        cmd.write_storage(&lb, 0, bytes);
    }
    let vw = &v.view.0;
    let row = |i: usize| [vw[0][i], vw[1][i], vw[2][i], vw[3][i]];
    let mut ck = [0.0f32; 7 * 4];
    ck[0..4].copy_from_slice(&[nx as f32, ny as f32, ns as f32, tile as f32]);
    ck[4..8].copy_from_slice(&[w as f32, h as f32, list.len() as f32, k as f32]);
    ck[8..12].copy_from_slice(&[p[0][0], p[1][1], p[2][0], p[2][1]]);
    ck[12..16].copy_from_slice(&[zscale, zbias, c.per_colour as f32, 0.0]);
    ck[16..20].copy_from_slice(&row(0));
    ck[20..24].copy_from_slice(&row(1));
    ck[24..28].copy_from_slice(&row(2));
    cmd.cs_block(Block::new(0, &ck));
    cmd.bind_storage(0, Some(&lb));
    cmd.bind_storage_rw(0, Some(&ib));
    if !cmd.dispatch(&pipe, clusters.div_ceil(64), 1, 1) {
        return;
    }
    let mut ps = [0.0f32; 16];
    ps[0..4].copy_from_slice(&[nx as f32, ny as f32, ns as f32, tile as f32]);
    ps[4..8].copy_from_slice(&[v.rect.x as f32, v.rect.y as f32, zscale, zbias]);
    ps[8..12].copy_from_slice(&row(2));
    ps[12..16].copy_from_slice(&[(k + 1) as f32, 1.0, 0.0, 0.0]);
    s.ps = ps;
    s.built = Some((v.frame, v.key(), v.cam));
    if v.main {
        s.last = (list.len(), clusters);
        if v.frame % 600 == 1 {
            crate::log!("lights: clusters: {} lights ({} of render3d's own shapes) in {}x{}x{} clusters of {} px", list.len(), s.extra.len(), nx, ny, ns, tile);
        }
    }
}

/// Binds the view's clusters to a draw (storage slots 0 and 1, pixel constants
/// c100..c103): true if this view has them, so the draw may use a clustered twin.
pub fn bind(cmd: &mut Cmd, v: &View) -> bool {
    let s = st();
    if s.built != Some((v.frame, v.key(), v.cam)) {
        return false;
    }
    let bufs = s.views.get(v.key());
    let (Some(lb), Some(ib)) = (bufs.lights.as_ref(), bufs.index.as_ref()) else { return false };
    cmd.bind_storage(0, Some(lb));
    cmd.bind_storage(1, Some(ib));
    cmd.ps_block(Block::new(PS_BASE, &s.ps));
    true
}

/// The lights and clusters of the last main view (the log).
pub fn last() -> (usize, u32) {
    st().last
}
