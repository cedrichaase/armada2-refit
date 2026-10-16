//! Our own particle simulation (`spec-particles.md`, sections 5, 6 and 11), on the sim
//! clock, once per displayed frame, whatever is on screen.
//!
//! Who owns emitters:
//!
//! - **Game objects** (`GameObject::objectList`): an object's emitters are its SOD's
//!   type-12 nodes (the SOD named after its ODF, as `mods::hulls` draws it), placed each
//!   frame by `scene::pose` with the object's transform scaled by its instance scale and
//!   the ODF's `ScaleSOD`. A craft shows or hides its damage nodes per system
//!   (`CraftInstance::RenderInternal` 0x4cb0e0, from the bytes at `+0x9c..+0xa1` of the
//!   instance, which `CraftInstance::Update`, slot 14, writes from the craft's systems);
//!   an emitter under a hidden node does not fire, as a node the engine does not
//!   traverse. The node of a system is the first in the SOD whose name equals the
//!   engine's (`CraftClass::InitializeDamageNodes` 0x4bfc70: `shields`, `engines`,
//!   `target`, `sensors`, `life`, `borg`, case-insensitive), so a node named `Shield` is
//!   never found and never hidden, as in stock.
//! - **The chain pulsar and the Manheim effect** (`Ordnance::ordnanceList` 0x771fa8):
//!   each shot carries a free emitter (`crpulsar` at `ChainPulsar+0xdc`, `gemini` at
//!   `ManheimEffect+0xd4`) that its `Submit` renders at the shot's transform, into the
//!   engine's global list (`Storm3D+0x68`): its particles outlive the shot.
//! - **Instances another module draws** (`feed`): a weapon's or an effect's SOD with
//!   emitters, fed once per frame by the module that draws it in the engine's place.
//!
//! Each owner's particles are one list, as an instance's is in stock: a cloud fills the
//! whole list to its count (Q11), and every particle is drawn as if submitted 1 + E
//! times (E: the owner's emitters shown this frame; for the global list, the global
//! emitters alive, Q1), as a colour factor. Simulated once per frame, not once per view
//! (stock's action camera ages what it shows twice, Q9: not reproduced), and for every
//! owner, drawn or not (stock froze an off-screen object's particles, Q10: fixed).
use super::look::{self, Look};
use crate::engine::{self, field, field_f32, field_u8, obj};
use crate::math::{v3, M34, V3};
use crate::scene::pose::{self, SodInfo};
use crate::{assets, log};
use a2fmt::sod::{NodeKind, Sod};
use a2fmt::spr::EmitterType;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

/// The engine's `RandomSequence` (`spec-particles.md`, section 3), our own instance:
/// the game's stream is never drawn from.
pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Rng {
        Rng(seed)
    }
    fn draw(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(2273).wrapping_add(31) % 429_397_397;
        self.0 % 100_001
    }
    /// `GetFloat()`: -1..1.
    pub fn signed(&mut self) -> f32 {
        (self.draw() as f32 - 50000.0) * 2e-5
    }
    /// `GetFloat(m)`: 0..m.
    pub fn up_to(&mut self, m: f32) -> f32 {
        self.draw() as f32 * m * 1e-5
    }
    pub fn bits(&mut self) -> u32 {
        let a = self.draw();
        let b = self.draw();
        a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b) ^ (a >> 7)
    }
}

/// Which of Venting.asi's looks a particle takes, by its emitter's sprite node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vent {
    Plasma = 0,
    Gas = 1,
}

/// An emitter template (`@emitter`): what every instance of it does.
pub struct Template {
    pub name: String,
    pub kind: EmitterType,
    pub look: Rc<Look>,
    pub speed: f32,
    pub growth: f32,
    pub life: f32,
    pub period: f32,
    /// Radians.
    pub spread: f32,
    pub count: i32,
    pub radius: f32,
    pub vent: Option<Vent>,
}

/// One particle (`SprayParticle` / `RandomParticle`), in the world.
#[derive(Clone)]
pub struct Particle {
    /// Its emitter: an index into the owner's emitters (vents link by it).
    pub em: u16,
    pub tpl: Rc<Template>,
    pub pos: V3,
    /// Spray: the flight axis (unit length; see `spray`). Cloud: its drift per second.
    pub vel: V3,
    /// Cloud: the emitter's place at its birth, which it dies away from.
    pub centre: V3,
    pub age: f32,
    pub seed: u32,
}

/// A SOD's emitters, per SOD (or one free emitter, for the global ones).
pub struct EmitterSet {
    pub sod: Option<(Rc<Sod>, SodInfo)>,
    pub scale_sod: f32,
    pub emitters: Vec<EmitterNode>,
}

pub struct EmitterNode {
    pub node: usize,
    pub tpl: Rc<Template>,
    /// The damage systems among its ancestors (bit per `SYSTEMS` entry).
    pub systems: u8,
}

/// The engine's damage-node names and the instance byte each follows. A node is hidden
/// while its byte is non-zero (the system is online), except `borg`, shown while it is.
const SYSTEMS: [(&str, usize); 6] =
    [("shields", 0x9c), ("engines", 0x9d), ("target", 0x9e), ("sensors", 0x9f), ("life", 0xa0), ("borg", 0xa1)];
const VT_CRAFT_INSTANCE: usize = 0x6b3e2c;
/// `CraftInstance::Update(const GameObject*)` (0x4cb390): the systems into the bytes.
const SLOT_UPDATE: usize = 14;
const ORDNANCE_LIST: usize = 0x771fa8; // Ordnance::ordnanceList, a std::list (head node at +4)
const VT_CHAIN_PULSAR: usize = 0x6b7c74;
const VT_MANHEIM: usize = 0x6b89fc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Object,
    /// A global emitter (chain pulsar, Manheim): its particles outlive it.
    Global,
    /// An instance another module draws and feeds.
    Fed,
}

pub struct EmState {
    pub timer: f32,
    /// Shown this frame, and where its node is.
    pub shown: bool,
    pub world: M34,
}

pub struct Owner {
    pub kind: Kind,
    pub set: Option<Rc<EmitterSet>>,
    pub inst: usize,
    pub ems: Vec<EmState>,
    pub parts: Vec<Particle>,
    pub seen: u32,
    /// Drawn this frame: not expired and the player can see it.
    pub visible: bool,
    /// 1 + the emitters shown this frame (Q1).
    pub mult: f32,
    /// The instance's bounding radius (`+0x40`), for the vents' lights.
    pub radius: f32,
    /// The instance's scale (x), which stock's sprite draw multiplies particles by.
    pub scale: f32,
}

impl Owner {
    fn new(kind: Kind, set: Option<Rc<EmitterSet>>) -> Owner {
        let n = set.as_ref().map_or(0, |s| s.emitters.len());
        let ems = (0..n).map(|_| EmState { timer: 0.0, shown: false, world: M34::IDENTITY }).collect();
        Owner { kind, set, inst: 0, ems, parts: Vec::new(), seen: 0, visible: false, mult: 1.0, radius: 50.0, scale: 1.0 }
    }
}

pub struct System {
    pub owners: BTreeMap<(usize, u32), Owner>,
    /// The global list's particles whose shot is gone.
    pub orphans: Vec<Particle>,
    /// 1 + the global emitters alive (Q1 for the global list).
    pub global_mult: f32,
    sets: BTreeMap<String, Option<Rc<EmitterSet>>>,
    templates: BTreeMap<String, Option<Rc<Template>>>,
    rng: Rng,
    last: Option<f32>,
    pub frame: u32,
    dt: f32,
    rate: f32,
    /// Cloud steps per second (stock steps a cloud particle per displayed frame, Q8).
    pub cloud_fps: f32,
    pub vent_sprites: [String; 2],
    pub said: u32,
}

/// Ages and moves a list (`SimulateParticleList`); the dead go.
fn age(parts: &mut Vec<Particle>, dt: f32) {
    parts.retain_mut(|p| {
        let t = &p.tpl;
        p.age += dt;
        match t.kind {
            EmitterType::Cloud => {
                p.pos = p.pos + p.vel * dt;
                p.age < t.life && (p.pos - p.centre).len() <= t.radius
            }
            _ => {
                p.pos = p.pos + p.vel * (t.speed * dt);
                p.age < t.life
            }
        }
    });
}

impl System {
    pub fn new() -> System {
        System {
            owners: BTreeMap::new(),
            orphans: Vec::new(),
            global_mult: 1.0,
            sets: BTreeMap::new(),
            templates: BTreeMap::new(),
            rng: Rng::new(12345),
            last: None,
            frame: 0,
            dt: 0.0,
            rate: 1.0,
            cloud_fps: 30.0,
            vent_sprites: [String::from("ventplas"), String::from("ventgas")],
            said: 0,
        }
    }

    pub fn template(&mut self, name: &str) -> Option<Rc<Template>> {
        let key = crate::rt::lower(name);
        if let Some(t) = self.templates.get(&key) {
            return t.clone();
        }
        let t = (|| {
            let table = assets::sprites();
            let e = table.emitter(&key)?;
            // No sprite node, no emitter (the engine warns and creates none).
            let look = look::get(e.sprite.as_deref()?)?;
            let vent = if look.id.eq_ignore_ascii_case(&self.vent_sprites[0]) {
                Some(Vent::Plasma)
            } else if look.id.eq_ignore_ascii_case(&self.vent_sprites[1]) {
                Some(Vent::Gas)
            } else {
                None
            };
            Some(Rc::new(Template {
                name: key.clone(),
                kind: e.emitter_type,
                look,
                speed: e.velocity,
                growth: e.expansion_rate,
                life: e.lifespan,
                period: e.firing_period,
                spread: e.spread_angle * core::f32::consts::PI / 180.0,
                count: e.particle_count,
                radius: e.cloud_radius,
                vent,
            }))
        })();
        if t.is_none() {
            log!("particles: emitter {} is not made (no such @emitter, or no sprite node)", key);
        }
        self.templates.insert(key, t.clone());
        t
    }

    /// The emitters of a SOD (`odf` names both the SOD and the ODF with its
    /// `ScaleSOD`, as for game objects; `scaled` false for a SOD fed with its own
    /// world transform). None when it has none.
    fn set(&mut self, name: &str, scaled: bool) -> Option<Rc<EmitterSet>> {
        let mut key = crate::rt::lower(name);
        if let Some(k) = key.strip_suffix(".sod") {
            key = String::from(k);
        }
        let mut ck = key.clone();
        if !scaled {
            ck.push_str("|fed");
        }
        if let Some(s) = self.sets.get(&ck) {
            return s.clone();
        }
        let s = (|| {
            let sod = assets::sod(&key)?;
            // The first node of each system's name, as FindElement finds it.
            let sys_nodes: Vec<Option<usize>> =
                SYSTEMS.iter().map(|(n, _)| sod.nodes.iter().position(|nd| nd.name.eq_ignore_ascii_case(n))).collect();
            let mut emitters = Vec::new();
            for (i, nd) in sod.nodes.iter().enumerate() {
                let NodeKind::Emitter { emitter } = &nd.kind else { continue };
                let Some(tpl) = self.template(emitter) else { continue };
                if tpl.kind == EmitterType::Beam {
                    continue; // dead in stock (spec 6.4)
                }
                let mut systems = 0u8;
                let mut at = Some(i);
                let mut guard = 0;
                while let Some(a) = at {
                    for (s, n) in sys_nodes.iter().enumerate() {
                        if *n == Some(a) {
                            systems |= 1 << s;
                        }
                    }
                    at = sod.nodes[a].parent;
                    guard += 1;
                    if guard > sod.nodes.len() {
                        break;
                    }
                }
                emitters.push(EmitterNode { node: i, tpl, systems });
            }
            if emitters.is_empty() {
                return None;
            }
            let scale_sod = if scaled {
                assets::odf(&key).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0)
            } else {
                1.0
            };
            let info = SodInfo::new(&sod);
            log!("particles: {}: {} emitter(s)", key, emitters.len());
            Some(Rc::new(EmitterSet { sod: Some((sod, info)), scale_sod, emitters }))
        })();
        self.sets.insert(ck, s.clone());
        s
    }

    /// A set of one free emitter (the global ones).
    fn free_set(&mut self, emitter: &str) -> Option<Rc<EmitterSet>> {
        let mut ck = String::from(emitter);
        ck.push_str("|free");
        if let Some(s) = self.sets.get(&ck) {
            return s.clone();
        }
        let s = self.template(emitter).map(|tpl| Rc::new(EmitterSet { sod: None, scale_sod: 1.0, emitters: alloc::vec![EmitterNode { node: 0, tpl, systems: 0 }] }));
        self.sets.insert(ck, s.clone());
        s
    }

    /// One displayed frame: every owner's emitters and particles.
    pub fn simulate(&mut self, now: f32, eye: V3) {
        self.frame = self.frame.wrapping_add(1);
        let dt = match self.last {
            Some(l) if now >= l => (now - l).min(1.0),
            Some(_) => {
                // The clock went back: a new game. Nothing carries over.
                self.owners.clear();
                self.orphans.clear();
                0.0
            }
            None => 0.0,
        };
        self.last = Some(now);
        self.dt = dt;
        let detail = pose::LodSettings::read().detail;
        self.rate = if detail >= 4 { 1.0 } else { (detail as f32 + 1.0) * 0.2 };
        let lod = pose::auto(eye);
        let frame = self.frame;
        let mut objs: Vec<usize> = Vec::new();
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if o != 0 && obj::flags(o) & 0x10000 == 0 && obj::instance(o) != 0 {
                objs.push(o);
            }
        });
        for o in objs {
            let key = (o, field(o, 0x28) as u32);
            if !self.owners.contains_key(&key) {
                let name = obj::odf_name(o);
                let set = if name.is_empty() { None } else { self.set(&assets::sod_of(name), true) };
                self.owners.insert(key, Owner::new(Kind::Object, set));
            }
            let Some(ow) = self.owners.get_mut(&key) else { continue };
            ow.seen = frame;
            let Some(set) = ow.set.clone() else { continue };
            let inst = obj::instance(o);
            ow.inst = inst;
            let expired = obj::expired(o);
            ow.visible = !expired && obj::can_user_see(o);
            let r = field_f32(inst, 0x40);
            ow.radius = if r > 1.0 && r < 20000.0 { r } else { 50.0 };
            let s3 = crate::mods::hulls::instance_scale(inst);
            ow.scale = if s3.x > 0.0 { s3.x } else { 1.0 };
            // Age and move what there is (SimulateParticleList comes first in stock).
            age(&mut ow.parts, dt);
            // Which emitters show: a hidden damage node hides its subtree.
            let hidden = if engine::vtable(inst) == VT_CRAFT_INSTANCE {
                // The bytes are written by the instance's Update (slot 14), which
                // GameObject::Render calls before drawing; an object a module draws
                // instead never gets it, so it is called here, through the vtable (the
                // bench's Scene.asi wraps it). It only reads the craft.
                unsafe {
                    let up: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(inst), SLOT_UPDATE * 4));
                    up(inst, o);
                }
                // Another database shown (not the class's main one): none of its nodes.
                let class = field(inst, 0x98);
                if field(inst, 0x80) != field(class, 0x1dc) {
                    0xff
                } else {
                    let mut h = 0u8;
                    for (s, (_, off)) in SYSTEMS.iter().enumerate() {
                        let b = field_u8(inst, *off);
                        let hide = if s == 5 { b == 0 } else { b != 0 };
                        if hide {
                            h |= 1 << s;
                        }
                    }
                    h
                }
            } else {
                0
            };
            let mut world = obj::transform(o);
            let s = s3 * set.scale_sod;
            world.r[0] = world.r[0] * s.x;
            world.r[1] = world.r[1] * s.y;
            world.r[2] = world.r[2] * s.z;
            let shown = place(ow, &set, &world, if expired { 0xff } else { hidden }, pose::Clock::of_instance(inst, now), lod);
            fire(ow, &set, dt, self.rate, &mut self.rng, self.cloud_fps);
            if self.said < 6 && shown > 0 {
                self.said += 1;
                log!(
                    "particles: {} emitter(s) of {} shown, {} particle(s), dt {:.3}, instance scale {:.2}, ScaleSOD {:.2}",
                    shown, obj::odf_name(o), ow.parts.len(), dt, s3.x, set.scale_sod
                );
            }
        }
        // The chain pulsar's and the Manheim effect's trails.
        let mut globals = 0u32;
        let mut ords: Vec<(usize, &'static str)> = Vec::new();
        let head = field(ORDNANCE_LIST, 4);
        if head != 0 {
            let mut n = field(head, 0);
            let mut guard = 0;
            while n != head && n != 0 && guard < 100_000 {
                guard += 1;
                let o = field(n, 8);
                n = field(n, 0);
                match engine::vtable(o) {
                    VT_CHAIN_PULSAR if field(o, 0xdc) != 0 => ords.push((o, "crpulsar")),
                    VT_MANHEIM if field(o, 0xd4) != 0 => ords.push((o, "gemini")),
                    _ => {}
                }
            }
        }
        for (o, em) in ords {
            let key = (o, 0xffff_fffd);
            if !self.owners.contains_key(&key) {
                let set = self.free_set(em);
                self.owners.insert(key, Owner::new(Kind::Global, set));
            }
            let Some(ow) = self.owners.get_mut(&key) else { continue };
            ow.seen = frame;
            ow.visible = true;
            let Some(set) = ow.set.clone() else { continue };
            age(&mut ow.parts, dt);
            ow.ems[0].shown = true;
            ow.ems[0].world = obj::transform(o);
            fire(ow, &set, dt, self.rate, &mut self.rng, self.cloud_fps);
            globals += 1;
        }
        self.global_mult = 1.0 + globals as f32;
        age(&mut self.orphans, dt);
        // Gone objects take their particles with them (the instance's destructor); a gone
        // shot leaves its trail in the global list; a fed instance not fed last frame is
        // gone.
        let orphans = &mut self.orphans;
        self.owners.retain(|_, ow| {
            let keep = match ow.kind {
                Kind::Fed => ow.seen.wrapping_add(1) >= frame,
                _ => ow.seen == frame,
            };
            if !keep && ow.kind == Kind::Global {
                orphans.append(&mut ow.parts);
            }
            keep
        });
        for ow in self.owners.values_mut() {
            if ow.kind == Kind::Global {
                ow.mult = self.global_mult;
            }
        }
    }

    /// An instance another module draws in the engine's place: its SOD's emitters at
    /// `world` (the transform the module draws it with, scale included), simulated once
    /// this frame. Call it every frame the instance exists; one not fed for a frame is
    /// gone, with its particles.
    pub fn feed(&mut self, key: usize, sod: &str, world: &M34, now: f32, eye: V3) {
        let k = (key, 0xffff_fffe);
        if !self.owners.contains_key(&k) {
            let set = self.set(sod, false);
            self.owners.insert(k, Owner::new(Kind::Fed, set));
        }
        let frame = self.frame;
        let (dt, rate, fps) = (self.dt, self.rate, self.cloud_fps);
        let Some(ow) = self.owners.get_mut(&k) else { return };
        if ow.seen == frame {
            return;
        }
        ow.seen = frame;
        ow.visible = true;
        let Some(set) = ow.set.clone() else { return };
        age(&mut ow.parts, dt);
        place(ow, &set, world, 0, pose::Clock::looping(now), pose::auto(eye));
        fire(ow, &set, dt, rate, &mut self.rng, fps);
    }

    pub fn count(&self) -> usize {
        self.owners.values().map(|o| o.parts.len()).sum::<usize>() + self.orphans.len()
    }
}

/// Where each emitter of an owner is this frame and whether it shows; the number shown.
fn place(ow: &mut Owner, set: &EmitterSet, world: &M34, hidden: u8, clock: pose::Clock, lod: pose::Lod) -> u32 {
    let mut shown = 0u32;
    let any = set.emitters.iter().any(|e| e.systems & hidden == 0) && hidden != 0xff;
    match (&set.sod, any) {
        (Some((sod, info)), true) => {
            let p = pose::pose(sod, info, world, clock, lod);
            for (i, e) in set.emitters.iter().enumerate() {
                let st = &mut ow.ems[i];
                st.shown = e.systems & hidden == 0 && p.shown.get(e.node).copied().unwrap_or(false);
                if st.shown {
                    st.world = p.world[e.node];
                    shown += 1;
                }
            }
        }
        _ => {
            for st in ow.ems.iter_mut() {
                st.shown = false;
            }
        }
    }
    ow.mult = 1.0 + shown as f32;
    shown
}

/// Every shown emitter of an owner fires (after its list has moved, as each emitter
/// node's `RenderInternal` does).
fn fire(ow: &mut Owner, set: &EmitterSet, dt: f32, rate: f32, rng: &mut Rng, fps: f32) {
    for (i, e) in set.emitters.iter().enumerate() {
        if !ow.ems[i].shown {
            continue;
        }
        let t = &e.tpl;
        let w = ow.ems[i].world;
        match t.kind {
            EmitterType::Spray => {
                // ST3D_ParticleEmitter_Spray::Simulate: at most one per call (Q12).
                let dte = rate * dt;
                let st = &mut ow.ems[i];
                st.timer -= dte;
                if st.timer < 0.0 {
                    st.timer = if dte < t.period { st.timer + t.period } else { t.period };
                    ow.parts.push(spray(rng, t, &w, i as u16));
                }
            }
            EmitterType::Cloud => {
                let mut guard = 0;
                while (ow.parts.len() as i32) < t.count && guard < 4096 {
                    guard += 1;
                    ow.parts.push(cloud(rng, t, &w, i as u16, fps));
                }
            }
            EmitterType::Beam => {}
        }
    }
}

/// A spray particle at the emitter node `w` (`ST3D_Particle` constructor, spec 6.1):
/// the node's +Z tilted by a uniform pitch and yaw within the spread.
fn spray(rng: &mut Rng, t: &Rc<Template>, w: &M34, em: u16) -> Particle {
    let s = t.spread;
    let d = if s > 0.0 {
        let a = s * rng.signed();
        let b = s * rng.signed();
        let _c = s * rng.signed();
        let (sa, ca) = (libm::sinf(a), libm::cosf(a));
        let (sb, cb) = (libm::sinf(b), libm::cosf(b));
        w.r[0] * (ca * sb) - w.r[1] * sa + w.r[2] * (ca * cb)
    } else {
        w.r[2]
    };
    // The axis is a unit one: stock's plumes are speed x life long whatever the model's
    // ScaleSOD (the Galaxy's, at 2.78, is 24 units: 8 a second for 3 s). The engine puts
    // ScaleSOD into the model's positions, not into its node matrices.
    let d = if d.len() > 1e-6 { d.norm() } else { v3(0.0, 0.0, 1.0) };
    Particle { em, tpl: t.clone(), pos: w.p, vel: d, centre: w.p, age: 0.0, seed: rng.bits() }
}

/// A cloud particle (`RandomParticle`, spec 6.3): a random step (stock's per frame,
/// here per second at `fps`), a start in a cube of half-width speed x radius about the
/// emitter, and a random age.
fn cloud(rng: &mut Rng, t: &Rc<Template>, w: &M34, em: u16, fps: f32) -> Particle {
    let v = v3(rng.signed(), rng.signed(), rng.signed()) * t.speed;
    let o = v3(rng.signed(), rng.signed(), rng.signed()) * (t.speed * t.radius);
    let age = rng.up_to(t.life);
    Particle { em, tpl: t.clone(), pos: w.p + o, vel: v3(v.z, v.y, v.x) * fps, centre: w.p, age, seed: rng.bits() }
}
