//! Hull debris: our own simulation of the engine's chunks (`ST3D_ChunkManager`,
//! `spec-chunks-anim-lod.md` §1 in the notes), drawn with the scene layer's hull
//! shading, in place of `RenderChunks`' CPU-lit triangles.
//!
//! **The trigger.** `Craft::Explode` (0x4c6ab0) breaks a dying craft up with
//! `ST3D_Instance::Fragment(teamColour, velocity, smoking, faces, attractor)`
//! (0x62eb20, thiscall on the craft's instance), called at 0x4c6bfc. That one call is
//! chained: when the craft's SOD is one we can read, we note the craft (its ODF, its
//! transform, its animation clock, its team colour) and do not call the engine's, so
//! the engine makes no chunks of its own for it; otherwise the engine's is called. The
//! Evolver's cocoon bursting (`Evolver::mDoRemoveConstructionEffect` and the other three
//! construction-effect calls, 0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4, each breaking the
//! whole cocoon: faces -1) is taken the same way, each call only while it still calls
//! `Fragment` itself: the cocoon (`CocoonInstance`, of `8472_cocoon.sod`) at its
//! instance's transform and scale. The other callers (a cancelled construction,
//! `BuildingEffect::SafeDestroyConstruction`) stay the engine's: their chunks are drawn
//! by `RenderChunks`, which still runs.
//!
//! **Breaking a mesh** (`CreateChunks` 0x61b7c0, as the spec gives it): every mesh of
//! the LOD shown breaks into patches: a random unused face, its first corner the seed,
//! and every unused face with any corner within half the mesh's bounding radius
//! (`ST3D_CHUNK_SIZE_FACTOR` 0.5) of it. A chunk's origin is the mean of its corners.
//! It lives 4 ± 2 s (`ART_CFG.h`), flies off in a random direction at up to 90 units
//! a second (times the square root of the mesh's radius over 100 when that is over
//! 100), spins at up to 4 rad/s about each axis (times 100 over the chunk's radius when
//! that is over 100), with no drag, and vanishes at the end of its life without
//! fading, as stock's do. At most 1500 at once (the oldest go first). The ship's own
//! velocity is not inherited: stock passes it and never reads it.
//!
//! The simulation runs on the sim clock (`View::time`), once per frame however many
//! views draw, so it stops while the game is paused.
use super::hooks::chunks as hk;
use super::{blank, Sec};

use r3d_core::engine::{self, obj};
use crate::log;
use r3d_core::frame::steps::View;
use r3d_core::gfx;
use r3d_core::math::{cosf, sinf, sqrtf, v3, Lcg, M34, V3};
use r3d_core::scene::{self, pose, Blend, Group, Model, Part, Params};
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

/// A dying craft waiting to be broken up (at the next `Step::Chunks`, with a device).
struct Pending {
    /// The SOD (the ODF's `baseName`, else its name).
    sod: String,
    world: M34,
    clock: pose::Clock,
    team: V3,
    /// A construction's: only the faces built so far break (in the SOD's order).
    faces: Option<usize>,
}

static mut PENDING: Vec<Pending> = Vec::new();
/// The part is on and its hooks are in: the dying craft are ours (not the engine's).
static mut ACTIVE: bool = false;
static mut LAST_EYE: V3 = V3::ZERO;

#[allow(static_mut_refs)]
fn pending() -> &'static mut Vec<Pending> {
    unsafe { &mut PENDING }
}

/// The instance's object, by walking the object list.
fn object_of_instance(inst: usize) -> usize {
    let mut found = 0;
    engine::for_each_in_list(engine::OBJECT_LIST, |o| {
        if found == 0 && o != 0 && obj::instance(o) == inst {
            found = o;
        }
    });
    found
}

/// A dying craft (`Craft::Explode`'s call of `Fragment`, from `hooks.rs`): noted for
/// our own chunks when its SOD is one we can read. False: the engine breaks it up.
pub fn note_craft(inst: usize, faces: i32, team: V3) -> bool {
    unsafe {
        if ACTIVE && faces < 0 && pending().len() < 64 {
            let o = object_of_instance(inst);
            let name = if o != 0 { obj::odf_name(o) } else { "" };
            if !name.is_empty() && r3d_core::engine::instance::draws_plain_sod(o) && r3d_core::assets::sod(&r3d_core::assets::sod_of(name)).is_some() {
                let s = r3d_core::scene::instance::instance_scale(inst) * scale_sod(name);
                let mut world = obj::transform(o);
                world.r[0] = world.r[0] * s.x;
                world.r[1] = world.r[1] * s.y;
                world.r[2] = world.r[2] * s.z;
                pending().push(Pending { sod: r3d_core::assets::sod_of(name), world, clock: pose::Clock::of_instance(inst, engine::sim_time()), team, faces: None });
                return true;
            }
        }
        false
    }
}

/// The Evolver's cocoon breaking up (faces -1, the whole of it): the cocoon
/// (`CocoonInstance`, of `8472_cocoon.sod`) at its instance's transform and scale.
pub fn note_cocoon(inst: usize, faces: i32, team: V3) -> bool {
    unsafe {
        if ACTIVE && faces < 0 && pending().len() < 64 && hk::is_cocoon(inst) && r3d_core::assets::sod("8472_cocoon").is_some() {
            let s = r3d_core::scene::instance::instance_scale(inst);
            let mut world = super::hooks::instance_world(inst);
            world.r[0] = world.r[0] * s.x;
            world.r[1] = world.r[1] * s.y;
            world.r[2] = world.r[2] * s.z;
            pending().push(Pending { sod: String::from("8472_cocoon"), world, clock: pose::Clock::of_instance(inst, engine::sim_time()), team, faces: None });
            return true;
        }
        false
    }
}

/// A construction cancelled or destroyed (`BuildingEffect::SafeDestroyConstruction`):
/// `faces` the faces built so far, which are the ones that break.
pub fn note_building(inst: usize, faces: i32, team: V3) -> bool {
    unsafe {
        if ACTIVE && faces > 0 && pending().len() < 64 {
            let name = hk::building_odf(inst);
            if !name.is_empty() && r3d_core::assets::sod(&r3d_core::assets::sod_of(name)).is_some() {
                let s = r3d_core::scene::instance::instance_scale(inst) * scale_sod(name);
                let mut world = super::hooks::instance_world(inst);
                world.r[0] = world.r[0] * s.x;
                world.r[1] = world.r[1] * s.y;
                world.r[2] = world.r[2] * s.z;
                pending().push(Pending { sod: r3d_core::assets::sod_of(name), world, clock: pose::Clock::of_instance(inst, engine::sim_time()), team, faces: Some(faces as usize) });
                return true;
            }
        }
        false
    }
}

/// Whether the dying craft are ours (set by the module when the part is on and its
/// hooks are in; cleared with the module's gate, so the engine breaks them up again).
pub fn set_active(on: bool) {
    unsafe { ACTIVE = on };
}

/// Forgets the craft waiting to be broken up (the module is switched off).
pub fn drop_pending() {
    pending().clear();
}

fn scale_sod(odf: &str) -> f32 {
    r3d_core::assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0)
}

struct Chunk {
    model: Rc<Model>,
    part: usize,
    /// The chunk's origin in its mesh's space (the mean of its corners).
    centre: V3,
    /// Its frame: the mesh's world axes, at the origin's place.
    m: M34,
    vel: V3,
    spin: V3,
    life: f32,
    team: V3,
}

struct Set {
    size_factor: f32,
    life: f32,
    life_dev: f32,
    max_vel: f32,
    max_spin: f32,
    max_chunks: usize,
}

pub struct Chunks {
    pub on: bool,
    /// `Enable=`: the part wants to run.
    pub want: bool,
    s: Set,
    chunks: VecDeque<Chunk>,
    rng: Lcg,
    last: f32,
    made: u32,
}

impl Chunks {
    pub fn new() -> Chunks {
        let c = Sec::with(blank(), "Debris");
        Chunks {
            on: false,
            want: false,
            s: Self::read(&c),
            chunks: VecDeque::new(),
            rng: Lcg(r3d_core::clock::seed() | 1),
            last: -1.0,
            made: 0,
        }
    }

    fn read(c: &Sec) -> Set {
        Set {
            size_factor: c.f("SizeFactor", 0.5).max(0.05),
            life: c.f("Lifetime", 4.0),
            life_dev: c.f("LifetimeDeviation", 2.0),
            max_vel: c.f("MaxVelocity", 90.0),
            max_spin: c.f("MaxAngularVelocity", 4.0),
            max_chunks: c.i("MaxChunks", 1500).max(1) as usize,
        }
    }

    /// `[Debris]`.
    pub fn configure(&mut self, ini: &crate::rt::Ini) {
        let c = Sec::of(ini, "Debris");
        self.want = c.i("Enable", 1) != 0;
        self.s = Self::read(&c);
    }

    /// Switched off: the chunks in flight and the craft waiting go (the engine breaks
    /// the next one up itself).
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.last = -1.0;
        drop_pending();
    }

    pub fn reset(&mut self) {
        // The chunks hold the scene's buffers: they go before the device resets.
        self.chunks.clear();
    }

    pub fn simulate(&mut self, now: f32) {
        if self.last < 0.0 || now < self.last {
            self.last = now;
            return;
        }
        let dt = (now - self.last).min(1.0);
        self.last = now;
        if dt <= 0.0 {
            return;
        }
        for c in self.chunks.iter_mut() {
            let r = euler(-c.spin.x * dt, -c.spin.y * dt, -c.spin.z * dt);
            let p = c.m.p + c.vel * dt;
            c.m = M34 { r: r.then(&M34 { r: c.m.r, p: V3::ZERO }).r, p };
            c.life -= dt;
        }
        self.chunks.retain(|c| c.life >= 0.0);
    }

    fn spawn(&mut self, _v: &View, p: Pending) {
        let Some(model) = scene::model(&p.sod) else { return };
        let eye = unsafe { LAST_EYE };
        let pz = pose::pose(&model.sod, &model.info, &p.world, p.clock, pose::auto(eye));
        let mut n = 0;
        let mut budget = p.faces.unwrap_or(usize::MAX);
        for (pi, part) in model.parts.iter().enumerate() {
            if !pz.shown.get(part.node).copied().unwrap_or(false) {
                continue;
            }
            if budget == 0 {
                break;
            }
            let Some(mesh) = model.sod.nodes[part.node].mesh() else { continue };
            let (k, used) = self.break_part(&model, pi, mesh, &pz.world[part.node], p.team, budget);
            n += k;
            budget = budget.saturating_sub(used);
        }
        self.made += 1;
        if self.made <= 6 {
            log!("effects: debris: {} broke into {} chunks ({} live)", p.sod, n, self.chunks.len());
        }
    }

    /// Breaks one mesh into chunks, at most its first `limit` faces: how many chunks,
    /// and how many faces it took.
    #[allow(clippy::too_many_arguments)]
    fn break_part(&mut self, model: &Rc<Model>, pi: usize, mesh: &a2fmt::sod::Mesh, w0: &M34, team: V3, limit: usize) -> (usize, usize) {
        let part = &model.parts[pi];
        let un = mesh.unindexed();
        let idx: Vec<u32> = un.groups.iter().flatten().copied().collect();
        let ntri = (idx.len() / 3).min(limit);
        if ntri == 0 {
            return (0, 0);
        }
        let pos = |vi: u32| -> V3 {
            let c = un.vertices[vi as usize];
            let p = mesh.positions.get(c.position as usize).copied().unwrap_or([0.0; 3]);
            v3(p[0], p[1], p[2])
        };
        // Each triangle's group.
        let mut tri_group = alloc::vec![0usize; ntri];
        for (gi, g) in part.groups.iter().enumerate() {
            let t0 = (g.start / 3) as usize;
            for t in t0..(t0 + g.tris as usize).min(ntri) {
                tri_group[t] = gi;
            }
        }
        let rm = part.radius.max(1e-3);
        let r = self.s.size_factor * rm;
        let r2 = r * r;
        let mut unused: Vec<usize> = (0..ntri).collect();
        // Every chunk's triangles, by group, into one index buffer for the mesh.
        let mut all_idx: Vec<u16> = Vec::new();
        let mut made: Vec<(Vec<Group>, V3, f32)> = Vec::new();
        while !unused.is_empty() {
            let k = (self.rng.next() as usize) % unused.len();
            let seed = pos(idx[unused[k] * 3]);
            let mut sel = Vec::new();
            let mut i = 0;
            while i < unused.len() {
                let t = unused[i];
                let near = (0..3).any(|c| {
                    let d = pos(idx[t * 3 + c]) - seed;
                    d.dot(d) < r2
                });
                if near {
                    sel.push(t);
                    unused.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            if sel.is_empty() {
                // Cannot happen (the seed's own face is within reach); be safe.
                let t = unused.swap_remove(k);
                sel.push(t);
            }
            // The origin: the mean of the distinct positions.
            let mut seen: Vec<u16> = Vec::new();
            let mut sum = V3::ZERO;
            for &t in &sel {
                for c in 0..3 {
                    let pidx = un.vertices[idx[t * 3 + c] as usize].position;
                    if !seen.contains(&pidx) {
                        seen.push(pidx);
                        sum = sum + pos(idx[t * 3 + c]);
                    }
                }
            }
            let centre = sum * (1.0 / seen.len().max(1) as f32);
            let mut rc = 0.0f32;
            for &t in &sel {
                for c in 0..3 {
                    rc = rc.max((pos(idx[t * 3 + c]) - centre).len());
                }
            }
            sel.sort_by_key(|&t| tri_group[t]);
            let mut groups: Vec<Group> = Vec::new();
            let mut gi_cur = usize::MAX;
            for &t in &sel {
                let gi = tri_group[t];
                if gi != gi_cur {
                    gi_cur = gi;
                    let g = &part.groups[gi];
                    groups.push(Group { start: all_idx.len() as u32, tris: 0, material: g.material.clone(), self_illum: g.self_illum, model: g.model });
                }
                for c in 0..3 {
                    all_idx.push(idx[t * 3 + c] as u16);
                }
                if let Some(g) = groups.last_mut() {
                    g.tris += 1;
                }
            }
            made.push((groups, centre, rc));
        }
        let ib = gfx::buffer_once(&all_idx, gpu::BufferKind::Index16);
        let parts: Vec<Part> = made
            .iter()
            .map(|(groups, centre, rc)| Part {
                node: 0,
                vb: part.vb.clone(),
                ib: ib.clone(),
                verts: part.verts,
                groups: groups.iter().map(|g| Group { start: g.start, tris: g.tris, material: g.material.clone(), self_illum: g.self_illum, model: g.model }).collect(),
                texture: part.texture.clone(),
                bump: None,
                blend: part.blend,
                two_sided: part.two_sided,
                centre: *centre,
                radius: *rc,
                lo: *centre - v3(*rc, *rc, *rc),
                hi: *centre + v3(*rc, *rc, *rc),
            })
            .collect();
        drop(ib);
        let cm = Rc::new(Model {
            name: model.name.clone(),
            sod: model.sod.clone(),
            info: pose::SodInfo { channel_node: Vec::new(), lods: Vec::new() },
            parts,
        });
        let n = made.len();
        for (ci, (_, centre, rc)) in made.into_iter().enumerate() {
            while self.chunks.len() >= self.s.max_chunks {
                self.chunks.pop_front();
            }
            let s = &self.s;
            let rng = &mut self.rng;
            let life = s.life + s.life_dev * rng.signed();
            let y = rng.signed();
            let th = core::f32::consts::PI * rng.signed();
            let q = sqrtf((1.0 - y * y).max(0.0));
            let dir = v3(sinf(th) * q, y, cosf(th) * q);
            let vmax = if rm > 100.0 { s.max_vel * sqrtf(rm / 100.0) } else { s.max_vel };
            let speed = rng.range(vmax);
            let wmax = if rc > 100.0 { s.max_spin * 100.0 / rc } else { s.max_spin };
            let spin = v3(wmax * rng.signed(), wmax * rng.signed(), wmax * rng.signed());
            self.chunks.push_back(Chunk {
                model: cm.clone(),
                part: ci,
                centre,
                m: M34 { r: w0.r, p: w0.transform_point(centre) },
                vel: dir * speed,
                spin,
                life,
                team,
            });
        }
        (n, ntri)
    }

    pub fn draw(&mut self, v: &View) {
        if v.main {
            unsafe { LAST_EYE = v.eye };
        }
        let p = core::mem::take(pending());
        for e in p {
            self.spawn(v, e);
        }
        if self.chunks.is_empty() {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        if !scene::begin(&mut cmd, v) {
            return;
        }
        for c in self.chunks.iter() {
            let world = M34 { r: M34::IDENTITY.r, p: -c.centre }.then(&c.m);
            let params = Params { team: Some(c.team), ..Params::default() };
            if c.model.parts[c.part].blend == Blend::Opaque || c.model.parts[c.part].blend == Blend::Cutout {
                scene::draw_part(&mut cmd, v, &c.model, c.part, &world, &params);
            }
        }
        // The additive parts (glows), after every opaque one.
        for c in self.chunks.iter() {
            let world = M34 { r: M34::IDENTITY.r, p: -c.centre }.then(&c.m);
            let params = Params { team: Some(c.team), ..Params::default() };
            if matches!(c.model.parts[c.part].blend, Blend::Additive | Blend::Alpha) {
                scene::draw_part(&mut cmd, v, &c.model, c.part, &world, &params);
            }
        }
    }
}

impl Default for Chunks {
    fn default() -> Chunks {
        Chunks::new()
    }
}


/// Rotation by Euler angles about x, then y, then z (row vectors).
fn euler(a: f32, b: f32, c: f32) -> M34 {
    let (sa, ca) = (sinf(a), cosf(a));
    let (sb, cb) = (sinf(b), cosf(b));
    let (sc, cc) = (sinf(c), cosf(c));
    let rx = M34 { r: [v3(1.0, 0.0, 0.0), v3(0.0, ca, sa), v3(0.0, -sa, ca)], p: V3::ZERO };
    let ry = M34 { r: [v3(cb, 0.0, -sb), v3(0.0, 1.0, 0.0), v3(sb, 0.0, cb)], p: V3::ZERO };
    let rz = M34 { r: [v3(cc, sc, 0.0), v3(-sc, cc, 0.0), v3(0.0, 0.0, 1.0)], p: V3::ZERO };
    rx.then(&ry).then(&rz)
}
