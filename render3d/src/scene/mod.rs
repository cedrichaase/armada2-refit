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
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{M34, M4, V3, v3};
use gpu::{
    attr, Address, AlphaTest, AttrFormat, Blend as GBlend, Block, Buffer, BufferKind, Cmd, Compare, Cull, Depth,
    Filter, Format, MipFilter, Pipeline, PipelineDesc, Sampler, Semantic, Topology, VertexAttr,
};
use crate::shaders::sod::*;
use crate::shaders::sod_spv::*;
use crate::shaders::sod_dxil::*;
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
    /// Its vertices (`Vertex`, 32 bytes) and 16-bit indices.
    pub vb: Buffer,
    pub ib: Buffer,
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
    /// Its box, in the same space: how wide the mesh really is along each axis.
    pub lo: V3,
    pub hi: V3,
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

pub static VERTEX: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float3, Semantic::Normal, 0),
    attr(24, AttrFormat::Float2, Semantic::TexCoord, 0),
];

impl Part {
    /// Draws `tris` triangles of the part's indices from `start` (a group's), with the
    /// bound pipeline.
    pub fn draw(&self, cmd: &mut Cmd, start: u32, tris: u32) {
        let (Some(vb), Some(ib)) = (cmd.span_of(&self.vb, 32), cmd.span_of(&self.ib, 2)) else { return };
        cmd.draw_indexed(&vb, 0, self.verts, &ib, start, tris * 3);
    }
}

fn blend_of(name: &str) -> Blend {
    match crate::rt::lower(name).as_str() {
        "additive" | "wormhole" => Blend::Additive,
        "alphathreshold" => Blend::Cutout,
        _ => Blend::Opaque,
    }
}

/// A mesh's blend: its texture material's name from version 1.7 on. Before, the name
/// is an ident in the file's texture-material table, and `ST3D_Stream::ReadTextureMaterial`
/// (0x63eb10) makes the material alpha-tested unless both of the entry's first two mode
/// bytes are 2 (the engine's two `ST3D_BlendMode`s): the texture's alpha cuts holes, as
/// in the Klingon and Romulan freighters' cargo pods.
fn mesh_blend(sod: &a2fmt::sod::Sod, name: &str) -> Blend {
    if sod.texture_materials.is_empty() {
        return blend_of(name);
    }
    match sod.texture_materials.iter().find(|t| t.ident.eq_ignore_ascii_case(name)) {
        Some(t) if !(t.modes[0] == 2 && t.modes[1] == 2) => Blend::Cutout,
        _ => blend_of(name),
    }
}

impl Model {
    fn build(name: &str, sod: Rc<Sod>) -> Model {
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
            let (vb, ib) = (gfx::buffer_once(&verts, BufferKind::Vertex), gfx::buffer_once(&idx, BufferKind::Index16));
            let texture = mesh
                .diffuse_texture()
                .map(String::from)
                .or_else(|| sod.texture_for_ident(&mesh.texture_material).map(String::from))
                .filter(|t| !t.is_empty());
            let (c, r) = mesh.bounds();
            let (mut lo, mut hi) = (v3(f32::MAX, f32::MAX, f32::MAX), v3(f32::MIN, f32::MIN, f32::MIN));
            for p in &mesh.positions {
                lo = v3(lo.x.min(p[0]), lo.y.min(p[1]), lo.z.min(p[2]));
                hi = v3(hi.x.max(p[0]), hi.y.max(p[1]), hi.z.max(p[2]));
            }
            parts.push(Part {
                node: i,
                vb,
                ib,
                verts: verts.len() as u32,
                groups,
                texture,
                bump: mesh.height_map().map(String::from),
                blend: mesh_blend(&sod, &mesh.texture_material),
                two_sided: mesh.cull == 0,
                centre: v3(c[0], c[1], c[2]),
                radius: r,
                lo,
                hi,
            });
        }
        Model { name: String::from(name), sod, info, parts }
    }
}

struct State {
    models: BTreeMap<String, Option<Rc<Model>>>,
    lights: Lights,
    queue: Vec<Queued>,
    pipes: Pipes,
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
    models: BTreeMap::new(),
    lights: Lights { dirs: Vec::new(), points: Vec::new() },
    queue: Vec::new(),
    pipes: Pipes::new(),
};

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

/// A SOD on the device, by name; built on first use. None if it does not load.
pub fn model(name: &str) -> Option<Rc<Model>> {
    let s = st();
    let key = crate::rt::lower(name);
    if let Some(m) = s.models.get(&key) {
        return m.clone();
    }
    let m = assets::sod(&key).map(|sod| Rc::new(Model::build(&key, sod)));
    s.models.insert(key, m.clone());
    m
}

/// Drops everything on the device (before a `Reset`): the models' buffers keep no copy
/// of their vertices (`gfx::buffer_once`), so they are built again.
pub fn reset() {
    let s = st();
    s.models.clear();
    s.queue.clear();
}

/// Once per view, before any module draws: the lights of this view.
pub fn begin_view(_v: &View) {
    st().lights = Lights::gather();
    st().queue.clear();
}

/// The lights of a view that is not the engine's (the menu scene, `crate::menu`): no
/// light is registered in the front end.
pub fn set_lights(l: Lights) {
    st().lights = l;
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

/// Adds a point light to the view being drawn, for the modules that draw after this
/// (a module's own light source: the particles module's vents). Call from
/// `begin_view`; the list is gathered again for the next view.
pub fn add_point(p: lights::Point) {
    st().lights.points.push(p);
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
    let Some(mut cmd) = gfx::cmd() else { return };
    if !begin(&mut cmd, v) {
        return;
    }
    for (pi, world) in opaque {
        draw_part(&mut cmd, v, m, pi, &world, p);
    }
}

/// The baseline's texture sampler: wrapped, filtered through the levels.
pub const SAMPLER: Sampler = gfx::sampler(Filter::Linear, MipFilter::Linear, Address::Wrap, Address::Wrap);

/// How a part blends, for its pipeline (`pipeline`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Added, no depth written.
    Add,
    /// Alpha-blended, no depth written.
    Alpha,
    /// Alpha-tested at one half, depth written.
    Cutout,
    Opaque,
}

/// The cull for a part: none for a two-sided mesh; the engine culls clockwise (its
/// front faces run anticlockwise on screen), and a mirrored node reverses the winding.
pub fn cull_of(part: &Part, world: &M34) -> Cull {
    if part.two_sided {
        Cull::None
    } else if world.det3() < 0.0 {
        Cull::CounterClockwise
    } else {
        Cull::Clockwise
    }
}

/// The baseline's pipeline for a cull and a blend: depth-tested against the scene.
fn pipeline(cull: Cull, mode: Mode) -> Option<Pipeline> {
    st().pipes.view(cull as u64 * 4 + mode as u64, || {
        let mut d = PipelineDesc::new(gpu::code!(VS_SOD, VS_SOD_SPV, VS_SOD_DXIL), gpu::code!(PS_SOD, PS_SOD_SPV, PS_SOD_DXIL, PS_SOD_AT_DXIL), &VERTEX, 32, Topology::TriangleList, Format::Unknown);
        let write = matches!(mode, Mode::Cutout | Mode::Opaque);
        d.depth = Depth { test: true, write, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
        d.cull = cull;
        d.blend = match mode {
            Mode::Add => Some(GBlend::ADD),
            Mode::Alpha => Some(GBlend::ALPHA),
            _ => None,
        };
        d.alpha_test = if mode == Mode::Cutout { Some(AlphaTest::HALF) } else { None };
        d
    })
}

/// Binds what every part drawn with the baseline shader shares (the view's matrix).
/// False if its shaders cannot be made.
pub fn begin(cmd: &mut Cmd, v: &View) -> bool {
    if !pipeline(Cull::Clockwise, Mode::Opaque).is_some_and(|p| p.ready(gfx::gpu())) {
        return false;
    }
    cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
    true
}

/// Draws one part with the baseline shader, after `begin`.
pub fn draw_part(cmd: &mut Cmd, v: &View, m: &Model, pi: usize, world: &M34, p: &Params) {
    let _ = v;
    let part = &m.parts[pi];
    let w = world.to_m4();
    let blended = p.alpha < 0.999 || matches!(part.blend, Blend::Additive | Blend::Alpha);
    let mode = match part.blend {
        Blend::Additive => Mode::Add,
        _ if blended => Mode::Alpha,
        Blend::Cutout => Mode::Cutout,
        _ => Mode::Opaque,
    };
    let Some(pipe) = pipeline(cull_of(part, world), mode) else { return };
    if !cmd.set_pipeline(&pipe) {
        return;
    }
    cmd.vs_block(Block::new(0, w.transpose().as_floats()));
    let tex = part.texture.as_deref().and_then(assets::texture);
    cmd.bind_texture(0, tex.as_ref(), &SAMPLER);
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
    // A normal taken through a mirrored node stays outward (a mirror is its own
    // inverse transpose), so it needs no flip of its own.
    set(&mut k, 1, [if p.lit && !additive { 1.0 } else { 0.0 }, if blended && !additive { 1.0 } else { 0.0 }, NORMAL_SIGN, p.alpha]);
    for g in &part.groups {
        let team = if g.material.starts_with("team") { p.team } else { None };
        let t = team.unwrap_or(v3(1.0, 1.0, 1.0));
        let selfi = if g.self_illum { p.self_illum } else { 0.0 };
        set(&mut k, 0, [p.ambient.x, p.ambient.y, p.ambient.z, selfi]);
        set(&mut k, 2, [t.x, t.y, t.z, 1.0]);
        cmd.ps_block(Block::new(0, &k));
        part.draw(cmd, g.start, g.tris);
    }
}

/// Which way `smooth_normals` points: a2fmt takes them from the file's winding, and
/// the engine's front faces run anticlockwise on screen (it culls clockwise), so they
/// point inward, as the engine's own do; -1 turns them out. (With the opposite cull, every hull showed its back
/// faces, which is why +1 once looked right.)
pub const NORMAL_SIGN: f32 = -1.0;

/// At `Step::Translucent`: every queued part, back to front.
pub fn flush(v: &View) {
    let s = st();
    if s.queue.is_empty() {
        return;
    }
    let mut q = core::mem::take(&mut s.queue);
    q.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(core::cmp::Ordering::Equal));
    let Some(mut cmd) = gfx::cmd() else { return };
    if begin(&mut cmd, v) {
        for e in &q {
            draw_part(&mut cmd, v, &e.model, e.part, &e.world, &e.p);
        }
    }
}

/// World matrix of a part's node as a Direct3D matrix (for modules with their own
/// shaders).
pub fn world_m4(pose: &Pose, part: &Part) -> M4 {
    pose.world[part.node].to_m4()
}
