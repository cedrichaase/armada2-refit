//! SOD instances, drawn by us: every module that shows a SOD (hulls, stations,
//! moons, asteroids, background planets, ghosts, the selection bubble, shield shells,
//! fireball flipbooks) goes through here.
//!
//! - `pose`: where each node is this frame (animation, level of detail);
//! - `model`: a SOD on the device, one vertex and index buffer per mesh;
//! - `draw`: one instance with the baseline shader (`shaders/sod.hlsl`), lit by the
//!   engine's registered lights; translucent parts wait in a queue drawn back to front
//!   just before the device's `Flush` (`Step::Translucent`), where stock draws its
//!   sorted triangles.
//!
//! A module with its own look binds its own shaders around `draw_part`, or walks the
//! model's parts itself.
pub mod lights;
pub mod pose;

use crate::assets;
use crate::d3d::*;
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{M34, M4, V3, v3};
use crate::shaders::sod::*;
use a2fmt::sod::{NodeKind, Sod};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use lights::Lights;
use pose::{Pose, SodInfo};

/// How a mesh's texture material blends (the SOD's texture material name).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blend {
    Opaque,
    /// `additive`: added, unlit, no depth writes.
    Additive,
    /// `alphathreshold`: cut out at alpha 0.5.
    Cutout,
    /// Alpha-blended (a faded instance, or a module's choice).
    Alpha,
}

/// One face group of one mesh: a draw call.
pub struct Group {
    pub start: u32,
    pub tris: u32,
    /// Its lighting material's name (`team`, `glow_white`...), lower case.
    pub material: String,
    pub self_illum: bool,
    /// The lighting model (0..3) of that material.
    pub model: u8,
}

/// One mesh node of a SOD on the device.
pub struct Part {
    pub node: usize,
    pub vb: Com,
    pub ib: Com,
    pub verts: u32,
    pub groups: Vec<Group>,
    pub texture: Option<String>,
    /// The height map of a bump-mapped mesh (the Borg's), by name.
    pub bump: Option<String>,
    pub blend: Blend,
    pub two_sided: bool,
    /// Model-space bounding sphere of the mesh (in its node's space).
    pub centre: V3,
    pub radius: f32,
}

pub struct Model {
    pub name: String,
    pub sod: Rc<Sod>,
    pub info: SodInfo,
    pub parts: Vec<Part>,
}

/// One vertex: position, normal, texture coordinate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
}

pub static VERTEX: [VertexElement; 4] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT3, DECLUSAGE_NORMAL, 0),
    ve(0, 24, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    VE_END,
];

fn blend_of(name: &str) -> Blend {
    match crate::rt::lower(name).as_str() {
        "additive" | "wormhole" => Blend::Additive,
        "alphathreshold" => Blend::Cutout,
        _ => Blend::Opaque,
    }
}

impl Model {
    fn build(dev: Device, name: &str, sod: Rc<Sod>) -> Model {
        let info = SodInfo::new(&sod);
        let mut parts = Vec::new();
        for (i, node) in sod.nodes.iter().enumerate() {
            let NodeKind::Mesh(mesh) = &node.kind else { continue };
            if mesh.positions.is_empty() || mesh.groups.is_empty() {
                continue;
            }
            let un = mesh.unindexed();
            let normals = mesh.smooth_normals();
            let verts: Vec<Vertex> = un
                .vertices
                .iter()
                .map(|c| Vertex {
                    pos: mesh.positions.get(c.position as usize).copied().unwrap_or([0.0; 3]),
                    nrm: normals.get(c.position as usize).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: mesh.uvs.get(c.uv as usize).copied().unwrap_or([0.0; 2]),
                })
                .collect();
            if verts.len() > 65535 {
                continue;
            }
            let mut idx: Vec<u16> = Vec::new();
            let mut groups = Vec::new();
            for (gi, g) in un.groups.iter().enumerate() {
                let start = idx.len() as u32;
                idx.extend(g.iter().map(|&v| v as u16));
                let mat_name = mesh.groups.get(gi).map(|fg| fg.material.as_str()).unwrap_or("");
                let lm = sod.lighting_material(mat_name);
                groups.push(Group {
                    start,
                    tris: g.len() as u32 / 3,
                    material: crate::rt::lower(mat_name),
                    self_illum: lm.is_some_and(|m| m.self_illuminating),
                    model: lm.map_or(1, |m| m.model),
                });
            }
            let vbytes = unsafe { core::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 32) };
            let ibytes = unsafe { core::slice::from_raw_parts(idx.as_ptr() as *const u8, idx.len() * 2) };
            let (Some(vb), Some(ib)) = (upload_buffer(dev, vbytes, false), upload_buffer(dev, ibytes, true)) else { continue };
            let texture = mesh
                .diffuse_texture()
                .map(String::from)
                .or_else(|| sod.texture_for_ident(&mesh.texture_material).map(String::from))
                .filter(|t| !t.is_empty());
            let (c, r) = mesh.bounds();
            parts.push(Part {
                node: i,
                vb,
                ib,
                verts: verts.len() as u32,
                groups,
                texture,
                bump: mesh.height_map().map(String::from),
                blend: blend_of(&mesh.texture_material),
                two_sided: mesh.cull == 0,
                centre: v3(c[0], c[1], c[2]),
                radius: r,
            });
        }
        Model { name: String::from(name), sod, info, parts }
    }
}

struct State {
    dev: Option<Device>,
    models: BTreeMap<String, Option<Rc<Model>>>,
    lights: Lights,
    queue: Vec<Queued>,
    vs: Shader,
    ps: Shader,
    decl: Decl,
}

/// A translucent part waiting for the flush.
struct Queued {
    model: Rc<Model>,
    part: usize,
    world: M34,
    p: Params,
    depth: f32,
}

static mut STATE: State = State {
    dev: None,
    models: BTreeMap::new(),
    lights: Lights { dirs: Vec::new(), points: Vec::new() },
    queue: Vec::new(),
    vs: Shader::vertex(&VS_SOD),
    ps: Shader::pixel(&PS_SOD),
    decl: Decl::new(&VERTEX),
};

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

/// A SOD on `dev`, by name; built on first use. None if it does not load.
pub fn model(dev: Device, name: &str) -> Option<Rc<Model>> {
    let s = st();
    if s.dev != Some(dev) {
        s.models.clear();
        s.dev = Some(dev);
    }
    let key = crate::rt::lower(name);
    if let Some(m) = s.models.get(&key) {
        return m.clone();
    }
    let m = assets::sod(&key).map(|sod| Rc::new(Model::build(dev, &key, sod)));
    s.models.insert(key, m.clone());
    m
}

/// Drops everything on the device (before a `Reset`).
pub fn reset() {
    let s = st();
    s.models.clear();
    s.queue.clear();
    s.dev = None;
}

/// Once per view, before any module draws: the lights of this view.
pub fn begin_view(_v: &View) {
    st().lights = Lights::gather();
    st().queue.clear();
}

/// Reads the engine's registered lights again, for a module that registered lights of
/// its own in its `begin_view` (the ordnance module: shots, warp flashes).
pub fn regather_lights() {
    st().lights = Lights::gather();
}

/// The registered lights of the view being drawn.
pub fn lights() -> &'static Lights {
    &st().lights
}

/// How to draw an instance.
#[derive(Clone, Copy)]
pub struct Params {
    /// Fade (1 = opaque). Below 1 the instance is drawn blended at the flush.
    pub alpha: f32,
    /// Multiplies the texture colour of the groups named `team*`.
    pub team: Option<V3>,
    /// Ambient light; and how strongly self-illuminated texels (alpha) glow.
    pub ambient: V3,
    pub self_illum: f32,
    pub lit: bool,
}

impl Default for Params {
    fn default() -> Params {
        Params { alpha: 1.0, team: None, ambient: v3(0.12, 0.12, 0.14), self_illum: 1.0, lit: true }
    }
}

/// Draws every shown mesh of a posed model: opaque parts now, translucent ones (and
/// everything when faded) queued for the flush.
pub fn draw(v: &View, m: &Rc<Model>, pose: &Pose, p: &Params) {
    let mut opaque = Vec::new();
    for (pi, part) in m.parts.iter().enumerate() {
        if !pose.shown.get(part.node).copied().unwrap_or(false) {
            continue;
        }
        let world = pose.world[part.node];
        if p.alpha < 0.999 || part.blend == Blend::Additive || part.blend == Blend::Alpha {
            let c = world.transform_point(part.centre);
            let vp = v.view.transform_point(c);
            st().queue.push(Queued { model: m.clone(), part: pi, world, p: *p, depth: vp.z });
        } else {
            opaque.push((pi, world));
        }
    }
    if opaque.is_empty() {
        return;
    }
    let _g = gfx::Guard::new(v.dev);
    if !begin(v) {
        return;
    }
    for (pi, world) in opaque {
        draw_part(v, m, pi, &world, p);
    }
}

/// Binds the baseline shaders and the states every part shares. False if the shaders
/// cannot be made.
pub fn begin(v: &View) -> bool {
    let s = st();
    let dev = v.dev;
    if !(s.vs.bind(dev) && s.ps.bind(dev) && s.decl.bind(dev)) {
        return false;
    }
    dev.vs_const(4, &gfx::vp_constants(v));
    for (st_, val) in [
        (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
        (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_BLENDOP, BLENDOP_ADD),
        (RS_SEPARATEALPHABLENDENABLE, 0), (RS_SRGBWRITEENABLE, 0),
    ] {
        dev.set_rs(st_, val);
    }
    dev.set_ss(0, SAMP_ADDRESSU, TADDRESS_WRAP);
    dev.set_ss(0, SAMP_ADDRESSV, TADDRESS_WRAP);
    dev.set_ss(0, SAMP_MAGFILTER, TEXF_LINEAR);
    dev.set_ss(0, SAMP_MINFILTER, TEXF_LINEAR);
    dev.set_ss(0, SAMP_MIPFILTER, TEXF_LINEAR);
    dev.set_stream_freq(0, 1);
    true
}

/// Draws one part with the baseline shader bound by `begin`.
pub fn draw_part(v: &View, m: &Model, pi: usize, world: &M34, p: &Params) {
    let dev = v.dev;
    let part = &m.parts[pi];
    let w = world.to_m4();
    dev.vs_const(0, w.transpose().as_floats());
    let mirrored = world.det3() < 0.0;
    // Which side faces out: the winding stock's meshes are authored with, flipped
    // under a mirrored node.
    dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else if mirrored { CULL_CW } else { CULL_CCW });
    let tex = part.texture.as_deref().and_then(|t| assets::texture(dev, t)).unwrap_or(core::ptr::null_mut());
    dev.set_texture(0, tex);
    let blended = p.alpha < 0.999 || matches!(part.blend, Blend::Additive | Blend::Alpha);
    match part.blend {
        Blend::Additive => {
            dev.set_rs(RS_ALPHABLENDENABLE, 1);
            dev.set_rs(RS_SRCBLEND, BLEND_ONE);
            dev.set_rs(RS_DESTBLEND, BLEND_ONE);
            dev.set_rs(RS_ZWRITEENABLE, 0);
            dev.set_rs(RS_ALPHATESTENABLE, 0);
        }
        _ if blended => {
            dev.set_rs(RS_ALPHABLENDENABLE, 1);
            dev.set_rs(RS_SRCBLEND, BLEND_SRCALPHA);
            dev.set_rs(RS_DESTBLEND, BLEND_INVSRCALPHA);
            dev.set_rs(RS_ZWRITEENABLE, 0);
            dev.set_rs(RS_ALPHATESTENABLE, 0);
        }
        Blend::Cutout => {
            dev.set_rs(RS_ALPHABLENDENABLE, 0);
            dev.set_rs(RS_ZWRITEENABLE, 1);
            dev.set_rs(RS_ALPHATESTENABLE, 1);
            dev.set_rs(RS_ALPHAREF, 128);
            dev.set_rs(RS_ALPHAFUNC, CMP_GREATEREQUAL);
        }
        _ => {
            dev.set_rs(RS_ALPHABLENDENABLE, 0);
            dev.set_rs(RS_ZWRITEENABLE, 1);
            dev.set_rs(RS_ALPHATESTENABLE, 0);
        }
    }
    // Lights: the strongest that reach this part.
    let ls = lights();
    let c = world.transform_point(part.centre);
    let r = part.radius * world.r[0].len().max(world.r[1].len()).max(world.r[2].len());
    let mut k = [0.0f32; 4 * 28];
    let set = |k: &mut [f32], reg: usize, a: [f32; 4]| k[(reg) * 4..(reg) * 4 + 4].copy_from_slice(&a);
    for (i, d) in ls.dirs.iter().take(4).enumerate() {
        set(&mut k, 4 + i, [d.dir.x, d.dir.y, d.dir.z, 0.0]);
        set(&mut k, 8 + i, [d.colour.x, d.colour.y, d.colour.z, 0.0]);
    }
    for (i, pt) in ls.points_near(c, r, 8).iter().enumerate() {
        set(&mut k, 12 + i, [pt.pos.x, pt.pos.y, pt.pos.z, pt.start]);
        set(&mut k, 20 + i, [pt.colour.x, pt.colour.y, pt.colour.z, pt.range]);
    }
    let additive = part.blend == Blend::Additive;
    set(&mut k, 1, [if p.lit && !additive { 1.0 } else { 0.0 }, if blended && !additive { 1.0 } else { 0.0 }, NORMAL_SIGN * if mirrored { -1.0 } else { 1.0 }, p.alpha]);
    dev.set_stream(0, part.vb.ptr(), 0, 32);
    dev.set_indices(part.ib.ptr());
    for g in &part.groups {
        let team = if g.material.starts_with("team") { p.team } else { None };
        let t = team.unwrap_or(v3(1.0, 1.0, 1.0));
        let selfi = if g.self_illum { p.self_illum } else { 0.0 };
        set(&mut k, 0, [p.ambient.x, p.ambient.y, p.ambient.z, selfi]);
        set(&mut k, 2, [t.x, t.y, t.z, 1.0]);
        dev.ps_const(0, &k);
        dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
    }
}

/// Which way `smooth_normals` points: outward as a2fmt computes them. Seen on the
/// bench: with -1 every hull was lit from behind.
pub const NORMAL_SIGN: f32 = 1.0;

/// At `Step::Translucent`: every queued part, back to front.
pub fn flush(v: &View) {
    let s = st();
    if s.queue.is_empty() {
        return;
    }
    let mut q = core::mem::take(&mut s.queue);
    q.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(core::cmp::Ordering::Equal));
    let _g = gfx::Guard::new(v.dev);
    if begin(v) {
        for e in &q {
            draw_part(v, &e.model, e.part, &e.world, &e.p);
        }
    }
}

/// World matrix of a part's node as a Direct3D matrix (for modules with their own
/// shaders).
pub fn world_m4(pose: &Pose, part: &Part) -> M4 {
    pose.world[part.node].to_m4()
}
