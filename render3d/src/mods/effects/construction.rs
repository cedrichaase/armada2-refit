//! Construction: `BuildingEffect` taken through `feature()`, in place of
//! `BuildingEffect::Render` (0x469660). render3d/README.md, "effects", has the engine
//! findings.
//!
//! What stock draws, per construction (`BuildingEffect::m_list`, a `std::map` at
//! 0x736bd0 whose node `+0x14` holds the `Info`): the station being built, an
//! instance embedded in the `Info` (`+0x20`) drawn with a face limit (the faces built
//! so far, `+0xcc`), in its builder's team colour; and at every welding edge (the list
//! at `+0xfc`, made by `AddNewWeldingEdges` from the edges of the faces added since the
//! last frame, each with its start and end in the world and a progress 0..1 that
//! `BuildingEffect::Simulate` moves at `weldingSpeed` a second) one `xspark` sprite
//! sliding from the edge's start to its end.
//!
//! Ours: the station through the hulls module's hull look (`hulls::draw_instance`),
//! faces limited as stock limits them; and at each edge the welding as the repair
//! welding draws its arc (Repair.asi's look): a flickering arc with turning rays at the
//! weld, a hot spot, sparks streaking off, and behind it the seam it leaves, white-hot
//! at the weld and cooling along the edge to orange and dull red, which goes on cooling
//! after the edge is done. A few arcs per construction light the hull
//! (`Module::lights`).
use super::{object_of, rnd, Cam, LightPool, Sec};
use super::weldfx::{self, Fx};
use crate::engine::{self, field, rd};
use crate::frame::steps::View;
use crate::log;
use crate::math::{v3, M34, V3};
use crate::mods::hulls;
use alloc::vec::Vec;

pub const VT_BUILDING: usize = 0x6afe4c;
/// `ConstructionObject::Render`: draws nothing (the object loop's view of a station
/// being built).
pub const FN_CONSTRUCTION_OBJECT_RENDER: usize = 0x4a3790;
const BE_HEAD: usize = 0x736bd4; // BuildingEffect::m_list: its head node
const BE_NIL: usize = 0x736c1c; // that tree's _Nil

const I_BUILDER: usize = 0x0; // the builder's handle
const I_TEAMS: usize = 0x4; // teams that see it
const I_COLOUR: usize = 0x8; // its colour at the start
const I_INST: usize = 0x20; // the station's instance (a CraftInstance)
const I_SCALE_HANDLE: usize = 0xb4; // the handle ApplyScaleEffects is asked for
const I_FACES: usize = 0xcc; // faces built so far
const I_CLASS: usize = 0xd0; // the station's class
const I_SPEED: usize = 0xe0; // weldingSpeed: an edge's progress a second
const I_RADIUS: usize = 0xe8; // weldingRadius: the spark sprite's half size
const I_EDGES: usize = 0x100; // the welding edges' list head node
// A welding edge's list node: start +0x14, end +0x20, progress +0x2c.
const E_START: usize = 0x14;
const E_END: usize = 0x20;
const E_T: usize = 0x2c;

const FN_TEAMS_VISIBLE: usize = 0x4d0070; // Entity::GetTeamsVisibleToUser(), cdecl
const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
const FN_TEAM_COLOUR: usize = 0x4d5000; // GameObject::GetTeamColor(), thiscall
const FN_SCALE_EFFECTS: usize = 0x599760; // ApplyScaleEffects(ST3D_Instance*, handle), cdecl
const FN_FACE_LIMIT: usize = 0x62ea80; // ST3D_Instance::SetRenderFaceLimit(int), thiscall
const FN_CLASS_ODF: usize = 0x4ce370; // GameObjectClass::GetOdfName(), thiscall
const FN_GET_CO: usize = 0x4afe80; // ConstructionRig::GetConstructionObject(), thiscall
const USER_PROFILE: usize = 0x76b5ac; // +0x84: the map editor is on
const VIEW_RECORD: usize = 0x76b610;
const S3D_TEAM: usize = 0x34; // Storm3D: the team colour of the instance drawn

/// A welding edge, live (the engine's) or done and cooling (ours).
#[derive(Clone, Copy)]
struct Edge {
    node: usize,
    info: usize,
    s: V3,
    e: V3,
    t: f32,
    /// Its progress a second.
    speed: f32,
    seed: u32,
    /// Sim time it was found done (gone from the engine's list), or < 0 while live.
    done: f32,
    /// The station's centre and radius, for the sparks' and the light's reach.
    centre: V3,
    radius: f32,
    /// Its construction was visible to the player when last seen.
    visible: bool,
    seen: bool,
    /// Its construction had all its faces when last seen (a construction that is gone
    /// without, cancelled or destroyed, takes its seams with it: its chunks fly off).
    complete: bool,
}

struct Set {
    sparks: usize,
    brightness: f32,
    flicker_rate: f32,
    stutter: f32,
    flare: f32,
    flare_min_px: f32,
    arc_share: f32,
    arc: V3,
    hot_col: V3,
    light_col: V3,
    hot: f32,
    seam: f32,
    seam_width: f32,
    seam_min_px: f32,
    cool: f32,
    spark_speed: f32,
    spark_spread: f32,
    spark_life: [f32; 2],
    spark_drag: f32,
    spark_width: f32,
    spark_min_px: f32,
    spark_bright: f32,
    streak_time: f32,
    lights: usize,
    light: f32,
    light_reach: f32,
}

pub struct Construction {
    pub on: bool,
    s: Set,
    edges: Vec<Edge>,
    made: u32,
    lights: LightPool,
    said: u32,
    said_fallback: bool,
}

/// Walks `BuildingEffect::m_list` in order: each `Info`.
fn walk(mut f: impl FnMut(usize)) {
    let head = unsafe { rd::<usize>(BE_HEAD) };
    let nil = unsafe { rd::<usize>(BE_NIL) };
    if head == 0 || nil == 0 {
        return;
    }
    let mut node = field(head, 0);
    let mut guard = 0;
    while node != 0 && node != head && guard < 1024 {
        guard += 1;
        let info = field(node, 0x14);
        if info != 0 {
            f(info);
        }
        let mut next = field(node, 8);
        if next != nil {
            while field(next, 0) != nil {
                next = field(next, 0);
            }
            node = next;
        } else {
            let mut p = field(node, 4);
            while node == field(p, 8) {
                node = p;
                p = field(p, 4);
            }
            if field(node, 8) != p {
                node = p;
            }
        }
    }
}

/// `BuildingEffect::Render`'s test: the builder is seen, or its team is one the player
/// sees, or the editor is on, or the view is the one that shows everything.
fn visible(info: usize) -> bool {
    let b = object_of(field(info, I_BUILDER) as i32);
    if b != 0 && crate::engine::obj::can_user_see(b) {
        return true;
    }
    let teams = unsafe {
        let f: unsafe extern "C" fn() -> u32 = core::mem::transmute(FN_TEAMS_VISIBLE);
        f()
    };
    if teams & field(info, I_TEAMS) as u32 != 0 {
        return true;
    }
    unsafe { engine::field_u8(rd::<usize>(USER_PROFILE), 0x84) != 0 || rd::<u32>(VIEW_RECORD) == 2 }
}

fn class_odf(class: usize) -> &'static str {
    if class == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_CLASS_ODF);
        let s = crate::rt::cstr_at(f(class), 260);
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

fn scale_sod(odf: &str) -> f32 {
    crate::assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0)
}

impl Construction {
    pub fn new() -> Construction {
        let c = Sec::new("Construction");
        let life = c.pair("SparkLife", [0.2, 0.6]);
        let l0 = life[0].max(0.02);
        Construction {
            on: c.i("Enable", 1) != 0,
            s: Set {
                sparks: c.i("Sparks", 3).clamp(0, 16) as usize,
                brightness: c.f("Brightness", 1.2),
                flicker_rate: c.f("FlickerRate", 24.0).max(1.0),
                stutter: c.f("Stutter", 0.2).clamp(0.0, 0.95),
                flare: c.f("Flare", 2.0),
                flare_min_px: c.f("FlareMinPixels", 12.0),
                arc_share: c.f("ArcShare", 0.3).clamp(0.0, 1.0),
                arc: c.rgb("ArcColour", [120.0, 165.0, 255.0]),
                hot_col: c.rgb("HotColour", [255.0, 120.0, 30.0]),
                light_col: c.rgb("LightColour", [150.0, 185.0, 255.0]),
                hot: c.f("HotSpot", 1.0),
                seam: c.f("Seam", 1.6),
                seam_width: c.f("SeamWidth", 0.35),
                seam_min_px: c.f("SeamMinPixels", 1.5),
                cool: c.f("CoolTime", 0.8).max(0.05),
                spark_speed: c.f("SparkSpeed", 0.12),
                spark_spread: c.f("SparkSpread", 0.9),
                spark_life: [l0, life[1].max(l0)],
                spark_drag: c.f("SparkDrag", 0.8).clamp(0.0, 1.9),
                spark_width: c.f("SparkWidth", 0.002),
                spark_min_px: c.f("SparkMinPixels", 1.4),
                spark_bright: c.f("SparkBrightness", 2.0),
                streak_time: c.f("StreakTime", 0.035),
                lights: c.i("Lights", 4).clamp(0, 8) as usize,
                light: c.f("Light", 1.5),
                light_reach: c.f("LightReach", 0.35),
            },
            edges: Vec::new(),
            made: 0,
            lights: LightPool::new(b"construction weld\0"),
            said: 0,
            said_fallback: false,
        }
    }

    /// The engine's edges into ours: new ones taken, gone ones marked done.
    fn gather(&mut self, now: f32) {
        for e in &mut self.edges {
            e.seen = false;
        }
        let mut infos = Vec::new();
        walk(|i| infos.push(i));
        for &info in &infos {
            let vis = visible(info);
            let complete = field(info, I_FACES) >= field(info, 0xc8);
            for e in self.edges.iter_mut().filter(|e| e.info == info) {
                e.complete = complete;
            }
            let speed = engine::field_f32(info, I_SPEED).max(0.01);
            let inst = info + I_INST;
            let mut r = engine::field_f32(inst, 0x40);
            if !(r > 1.0 && r < 1e5) {
                r = 100.0;
            }
            let centre = engine::field_v3(inst, 0x68);
            let head = field(info, I_EDGES);
            if head == 0 {
                continue;
            }
            let mut n = field(head, 0);
            let mut guard = 0;
            while n != 0 && n != head && guard < 4096 {
                guard += 1;
                let s = super::rv3(n + E_START);
                let e = super::rv3(n + E_END);
                let t = engine::field_f32(n, E_T).clamp(0.0, 1.0);
                match self.edges.iter_mut().find(|x| x.done < 0.0 && x.node == n && (x.s - s).len() < 1e-3 && (x.e - e).len() < 1e-3) {
                    Some(x) => {
                        x.t = t;
                        x.seen = true;
                        x.visible = vis;
                        x.centre = centre;
                        x.radius = r;
                    }
                    None => {
                        if self.edges.len() < 4096 {
                            self.made = self.made.wrapping_add(1);
                            self.edges.push(Edge {
                                node: n,
                                info,
                                s,
                                e,
                                t,
                                speed,
                                seed: super::hash32(self.made.wrapping_mul(2654435761) ^ n as u32),
                                done: -1.0,
                                centre,
                                radius: r,
                                visible: vis,
                                seen: true,
                                complete,
                            });
                        }
                    }
                }
                n = field(n, 0);
            }
        }
        self.edges.retain(|e| e.complete || infos.contains(&e.info));
        let keep = self.s.cool * 5.0 + self.s.spark_life[1];
        for e in &mut self.edges {
            if !e.seen && e.done < 0.0 {
                e.done = now;
                e.t = 1.0;
            }
        }
        self.edges.retain(|e| e.done < 0.0 || (now - e.done < keep && now >= e.done));
    }

    fn arc_now(&self, e: &Edge, now: f32) -> f32 {
        if e.done >= 0.0 {
            return 0.0;
        }
        // Of the many edges welded at once only some arc at a time, each in bursts.
        let burst = (now * 8.0) as i64 as u32;
        if rnd(e.seed ^ burst.wrapping_mul(2654435761)) >= self.s.arc_share {
            return 0.0;
        }
        let tick = (now * self.s.flicker_rate) as i64 as u32;
        let on = if rnd(e.seed ^ ((now * 9.0) as i64 as u32).wrapping_mul(2246822519)) < self.s.stutter { 0.15 } else { 1.0 };
        on * (0.45 + 0.55 * rnd(e.seed.wrapping_add(tick.wrapping_mul(747796405))))
    }

    /// A few arcs per construction light its hull.
    pub fn lights(&mut self, v: &View) {
        if !self.on || self.s.lights == 0 || self.s.light <= 0.0 {
            return;
        }
        let now = v.time;
        let mut n = 0;
        let mut infos: Vec<usize> = Vec::new();
        for e in &self.edges {
            if e.done < 0.0 && e.visible && !infos.contains(&e.info) {
                infos.push(e.info);
            }
        }
        for info in infos {
            let live: Vec<Edge> = self.edges.iter().filter(|e| e.info == info && e.done < 0.0 && e.visible).copied().collect();
            let k = live.len().min(self.s.lights);
            for j in 0..k {
                if n >= 16 {
                    return;
                }
                let e = live[j * live.len() / k];
                let arc = self.arc_now(&e, now);
                if arc < 0.02 {
                    continue;
                }
                let p = e.s + (e.e - e.s) * e.t;
                let out = (p - e.centre).norm();
                let reach = self.s.light_reach * e.radius;
                if self.lights.register(n, p + out * (0.03 * e.radius), self.s.light_col * (self.s.light * arc), reach * 0.3, reach * 0.7) {
                    n += 1;
                }
            }
        }
    }

    /// `BuildingEffect::Render` in our place: the stations being built, through the
    /// hulls module; the welds are noted for `draw`. False (the engine draws the whole
    /// feature) when the hulls module does not run.
    pub fn feature(&mut self, v: &View) -> bool {
        if !crate::mods::running("hulls") {
            if !self.said_fallback {
                self.said_fallback = true;
                log!("effects: construction: the hulls module does not run: the engine draws construction");
            }
            return false;
        }
        self.gather(v.time);
        let mut infos = Vec::new();
        walk(|i| infos.push(i));
        for info in infos {
            if !visible(info) {
                continue;
            }
            let faces = field(info, I_FACES) as i32;
            let inst = info + I_INST;
            if faces <= 0 {
                continue;
            }
            let pos = engine::field_v3(inst, 0x68);
            let culled = unsafe {
                let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
                f(inst, &v.eye, &pos) != 0
            };
            if culled {
                continue;
            }
            let b = object_of(field(info, I_BUILDER) as i32);
            let colour = if b != 0 {
                unsafe {
                    let f: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(FN_TEAM_COLOUR);
                    let p = f(b);
                    if p.is_null() { engine::field_v3(info, I_COLOUR) } else { core::ptr::read_unaligned(p) }
                }
            } else {
                engine::field_v3(info, I_COLOUR)
            };
            // As stock: the scale effects into the instance, its frustum test, its face
            // limit. Each writes only the effect's own instance.
            let shown = unsafe {
                let se: unsafe extern "C" fn(usize, usize) = core::mem::transmute(FN_SCALE_EFFECTS);
                se(inst, field(info, I_SCALE_HANDLE));
                let fl: unsafe extern "thiscall" fn(usize, i32) = core::mem::transmute(FN_FACE_LIMIT);
                fl(inst, faces);
                hulls::game::frustum_test(inst, v.cam)
            };
            if shown {
                let name = class_odf(field(info, I_CLASS));
                let s = hulls::instance_scale(inst) * scale_sod(name);
                let mut world: M34 = engine::field_m34(inst, 0x44);
                world.r[0] = world.r[0] * s.x;
                world.r[1] = world.r[1] * s.y;
                world.r[2] = world.r[2] * s.z;
                let ours = !name.is_empty() && hulls::draw_instance(v, inst, inst, true, &crate::assets::sod_of(name), &world, Some(colour));
                if ours {
                    crate::mods::particles::feed(inst, name, &world, v);
                    self.said += 1;
                    if self.said <= 4 || self.said % 600 == 0 {
                        log!(
                            "effects: construction: {} at {:.0},{:.0},{:.0}: {} of {} faces, {} welding edge(s) live",
                            name, world.p.x, world.p.y, world.p.z, faces, field(info, 0xc8),
                            self.edges.iter().filter(|e| e.info == info && e.done < 0.0).count()
                        );
                    }
                } else {
                    // The engine's own draw of the instance, as stock makes it.
                    unsafe {
                        core::ptr::write_unaligned((engine::storm3d() + S3D_TEAM) as *mut V3, colour);
                        let ri: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(inst), 0x14));
                        ri(inst, v.cam);
                    }
                }
            }
            // The construction object's on-screen byte, which the HUD reads.
            if b != 0 && hulls::game::render_fn(b) == hulls::game::FN_RIG_RENDER {
                unsafe {
                    let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(FN_GET_CO);
                    let co = f(b);
                    if co != 0 {
                        core::ptr::write_volatile((co + 0x25) as *mut u8, shown as u8);
                    }
                }
            }
        }
        true
    }

    /// The welds, the seams and the sparks, at `Step::Chunks` (every hull in depth).
    pub fn draw(&mut self, v: &View, c: &Cam, fx: &mut Fx) {
        let now = v.time;
        let s = &self.s;
        for e in self.edges.iter().filter(|e| e.visible) {
            let len = (e.e - e.s).len();
            if len < 1e-4 {
                continue;
            }
            let wr = engine::field_f32(e.info, I_RADIUS);
            let wr = if e.done < 0.0 && wr > 0.0 && wr < 100.0 { wr } else { 1.0 };
            let edge_time = 1.0 / e.speed;
            let (end, age, fade) = if e.done < 0.0 {
                (e.s + (e.e - e.s) * e.t, e.t * edge_time, 1.0)
            } else {
                (e.e, edge_time + (now - e.done), libm::expf(-(now - e.done) / s.cool))
            };
            let px = c.pixel * (end - c.eye).len();
            // The seam: from the edge's start to the weld, cooling behind it.
            if s.seam > 0.0 && fade > 0.01 {
                let hw = (s.seam_width * wr).max(s.seam_min_px * px * 0.5);
                let k = (e.t * edge_time) / s.cool;
                let thin = libm::sqrtf((s.seam_width * wr / hw).min(1.0));
                fx.ribbon(c, e.s, end, hw, weldfx::SEAM, k, v3(1.0, 1.0, 1.0), s.seam * s.brightness * fade * thin);
            }
            // The sparks, emitted while the weld ran.
            let out = (end - e.centre).norm();
            for k in 0..s.sparks {
                let sd = e.seed.wrapping_add((k as u32).wrapping_mul(0x9e3779b9));
                let te = rnd(sd) * edge_time;
                let life = s.spark_life[0] + (s.spark_life[1] - s.spark_life[0]) * rnd(sd.wrapping_add(1));
                let from = e.s + (e.e - e.s) * (te / edge_time).clamp(0.0, 1.0);
                let dir = (out + weldfx::rand_dir(rnd(sd.wrapping_add(2)), rnd(sd.wrapping_add(3))) * s.spark_spread).norm();
                let sp = s.spark_speed * e.radius * (0.3 + 0.7 * rnd(sd.wrapping_add(5)));
                if let Some((tail, head, kh)) = weldfx::spark_at(from, dir, sp, age - te, life, s.spark_drag, s.streak_time) {
                    let phys = s.spark_width * e.radius;
                    let hw = phys.max(s.spark_min_px * px * 0.5);
                    let b = s.spark_bright * s.brightness * kh * (0.5 + 0.5 * kh) * libm::sqrtf(phys / hw);
                    fx.streak(c, tail, head, hw, weldfx::heat_colour(kh), b);
                }
            }
            if e.done >= 0.0 {
                continue;
            }
            // The weld: a hot spot and the arc's flare.
            let m = s.flare_min_px * px * 0.5;
            if s.hot > 0.0 {
                let h = (wr * 0.6).max(m * 0.5);
                fx.billboard(c, end, h, h, weldfx::HOT, 0.0, s.hot_col, s.hot * s.brightness);
            }
            let arc = self.arc_now(e, now);
            if arc > 0.01 {
                let h = (s.flare * wr).max(m);
                let ang = rnd(e.seed.wrapping_add(((now * s.flicker_rate) as i64 as u32).wrapping_mul(2891336453))) * core::f32::consts::PI;
                fx.billboard(c, end, h, h, weldfx::FLARE, ang, s.arc, s.brightness * arc);
            }
        }
    }
}

impl Default for Construction {
    fn default() -> Construction {
        Construction::new()
    }
}

