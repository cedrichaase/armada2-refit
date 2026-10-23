//! The worker bees and the drones: `WorkerBee` (a construction ship's bees) and
//! `DroneColony` (the drones of the `Drones` special weapon) taken through
//! `feature()`, in place of `BeeColony::Render` (0x4688a0). render3d/README.md,
//! "effects", has the engine findings.
//!
//! Stock: each bee (the colony's multimap, from its `GetBeeList`, vtable `+0x40`; the
//! bee at node `+0x10`) is an `ST3D_Instance` of the bee's SOD drawn by
//! `ST3D_Instance::Render` while its owner is seen; one that welds (state `+0x8c` = 4)
//! draws its welding beam, the sprite `+0x84` (`weldingBeamSprite`) as a ribbon
//! (`ST3D_Sprite::RenderAsPolygon`) from the bee (`+0x68`) to the spot it welds
//! (`+0xac`), `+0x88` wide.
//!
//! Ours: the bee through the hulls module (lit as a hull, its strobes drawn as
//! running lights), its SOD named by the owner's ODF (`workerBeeName`, the
//! construction ship's; the drones' from `gdrones`), its emitters (the drones' steam)
//! fed to the particles module; the beam as an arc of light (`weldfx` kind 4) with
//! the repair welding's flare, hot spot and sparks at the spot, and a light on the hull.
use super::{object_of, rnd, Cam, LightPool, Sec};
use super::weldfx::{self, Fx};
use crate::engine::{self, field, rd};
use crate::frame::steps::View;
use crate::log;
use crate::math::{v3, M34, V3};
use crate::mods::hulls;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub const VT_WORKER_BEE: usize = 0x6b017c;
pub const VT_DRONE_COLONY: usize = 0x6afe98;
const BEE_NIL: usize = 0x736bc4; // the bees' multimap's _Nil
const SLOT_BEE_LIST: usize = 0x40; // the colony's GetBeeList()

const B_POS: usize = 0x68;
const B_WIDTH: usize = 0x88;
const B_STATE: usize = 0x8c;
const B_SPOT: usize = 0xac;
const B_OWNER: usize = 0xbc;
const STATE_WELDING: u32 = 4;

const USER_PROFILE: usize = 0x76b5ac;
const VIEW_RECORD: usize = 0x76b610;

/// A bee welding this frame.
#[derive(Clone, Copy)]
struct Weld {

    from: V3,
    spot: V3,
    width: f32,
    seed: u32,
}

struct Set {
    beam: f32,
    beam_col: V3,
    beam_min_px: f32,
    flare: f32,
    flare_min_px: f32,
    hot: f32,
    hot_col: V3,
    sparks: usize,
    spark_every: f32,
    spark_life: [f32; 2],
    spark_speed: f32,
    spark_spread: f32,
    spark_bright: f32,
    spark_min_px: f32,
    flicker_rate: f32,
    light: f32,
    light_col: V3,
    light_reach: f32,
    brightness: f32,
}

pub struct Bees {
    pub on: bool,
    s: Set,
    welds: Vec<Weld>,
    /// By the owner's ODF: the bee's SOD.
    names: BTreeMap<String, String>,
    drone_sod: Option<String>,
    lights: LightPool,
    said: u32,
}

/// `BeeColony::ShouldDrawBee`: the owner is a live game object the player sees, or
/// the editor is on, or the view shows everything.
fn should_draw(bee: usize) -> usize {
    let o = object_of(field(bee, B_OWNER) as i32);
    let seen = o != 0 && engine::field_u8(o, 0x113) == 0 && crate::engine::obj::can_user_see(o);
    if seen || unsafe { engine::field_u8(rd::<usize>(USER_PROFILE), 0x84) != 0 || rd::<u32>(VIEW_RECORD) == 2 } {
        if o != 0 { o } else { 1 }
    } else {
        0
    }
}

/// The colony's bees, in order.
fn walk(colony: usize, mut f: impl FnMut(usize)) {
    let list = unsafe {
        let g: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(field(engine::vtable(colony), SLOT_BEE_LIST));
        g(colony)
    };
    let head = field(list, 4);
    let nil = unsafe { rd::<usize>(BEE_NIL) };
    if head == 0 || nil == 0 {
        return;
    }
    let mut node = field(head, 0);
    let mut guard = 0;
    while node != 0 && node != head && guard < 1024 {
        guard += 1;
        f(node + 0x10);
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

impl Bees {
    pub fn new() -> Bees {
        let c = Sec::new("Bees");
        let life = c.pair("SparkLife", [0.15, 0.45]);
        let l0 = life[0].max(0.02);
        Bees {
            on: c.i("Enable", 1) != 0,
            s: Set {
                beam: c.f("Beam", 0.9),
                beam_col: c.rgb("BeamColour", [110.0, 160.0, 255.0]),
                beam_min_px: c.f("BeamMinPixels", 2.5),
                flare: c.f("Flare", 3.0),
                flare_min_px: c.f("FlareMinPixels", 12.0),
                hot: c.f("HotSpot", 1.0),
                hot_col: c.rgb("HotColour", [255.0, 120.0, 30.0]),
                sparks: c.i("Sparks", 6).clamp(0, 24) as usize,
                spark_every: c.f("SparkEvery", 0.06).max(0.01),
                spark_life: [l0, life[1].max(l0)],
                spark_speed: c.f("SparkSpeed", 20.0),
                spark_spread: c.f("SparkSpread", 0.8),
                spark_bright: c.f("SparkBrightness", 2.0),
                spark_min_px: c.f("SparkMinPixels", 1.4),
                flicker_rate: c.f("FlickerRate", 24.0).max(1.0),
                light: c.f("Light", 1.5),
                light_col: c.rgb("LightColour", [150.0, 185.0, 255.0]),
                light_reach: c.f("LightReach", 40.0),
                brightness: c.f("Brightness", 1.3),
            },
            welds: Vec::new(),
            names: BTreeMap::new(),
            drone_sod: None,
            lights: LightPool::new(b"bee weld\0"),
            said: 0,
        }
    }

    /// The bee's SOD: the owner's `workerBeeName` for a worker bee, the drones' weapon's
    /// for a drone.
    fn sod(&mut self, drone: bool, owner: usize) -> String {
        if drone {
            if self.drone_sod.is_none() {
                let n = crate::assets::odf("gdrones").and_then(|o| o.get_str("workerBeeName").map(String::from)).unwrap_or_else(|| String::from("drones"));
                self.drone_sod = Some(n);
            }
            return self.drone_sod.clone().unwrap_or_default();
        }
        let odf = if owner > 1 { crate::engine::obj::odf_name(owner) } else { "" };
        if let Some(n) = self.names.get(odf) {
            return n.clone();
        }
        let n = crate::assets::odf(odf).and_then(|o| o.get_str("workerBeeName").map(String::from)).unwrap_or_else(|| String::from("fbee"));
        self.names.insert(String::from(odf), n.clone());
        n
    }

    pub fn begin_view(&mut self) {
        self.welds.clear();
    }

    /// `BeeColony::Render` in our place. False (the engine draws the colony) when the
    /// hulls module does not run.
    pub fn feature(&mut self, colony: usize, drone: bool, v: &View) -> bool {
        if !crate::mods::running("hulls") {
            return false;
        }
        let mut bees = Vec::new();
        walk(colony, |b| bees.push(b));
        for bee in bees {
            let owner = should_draw(bee);
            if owner == 0 {
                continue;
            }
            if hulls::game::frustum_test(bee, v.cam) {
                let name = self.sod(drone, owner);
                let world: M34 = engine::field_m34(bee, 0x44);
                let team = if owner > 1 { Some(hulls::game::team_colour(owner)) } else { None };
                if hulls::draw_instance(v, bee, bee, false, &name, &world, team) {
                    crate::mods::particles::feed(bee, &name, &world, v);
                    if self.said < 3 {
                        self.said += 1;
                        log!("effects: bees: {} drawn ({})", name, if drone { "drone" } else { "worker bee" });
                    }
                } else {
                    unsafe {
                        let ri: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(bee), 0x14));
                        ri(bee, v.cam);
                    }
                }
            }
            if field(bee, B_STATE) as u32 == STATE_WELDING && self.welds.len() < 64 {
                self.welds.push(Weld {

                    from: engine::field_v3(bee, B_POS),
                    spot: engine::field_v3(bee, B_SPOT),
                    width: engine::field_f32(bee, B_WIDTH),
                    seed: super::hash32(bee as u32),
                });
            }
        }
        true
    }

    fn flicker(&self, seed: u32, now: f32) -> f32 {
        let tick = (now * self.s.flicker_rate) as i64 as u32;
        0.6 + 0.4 * rnd(seed.wrapping_add(tick.wrapping_mul(747796405)))
    }

    /// The welds of the last frame light the hull.
    pub fn lights(&mut self, v: &View) {
        if !self.on || self.s.light <= 0.0 {
            return;
        }
        let ws = self.welds.clone();
        for (n, w) in ws.iter().take(12).enumerate() {
            let k = self.flicker(w.seed, v.time);
            let back = (w.from - w.spot).norm();
            self.lights.register_spark(n, w.spot + back * 2.0, self.s.light_col * (self.s.light * k), self.s.light_reach * 0.3, self.s.light_reach * 0.7);
        }
    }

    /// The beams, the welds' flares and sparks, at `Step::Chunks`.
    pub fn draw(&mut self, v: &View, c: &Cam, fx: &mut Fx) {
        let now = v.time;
        let s = &self.s;
        for w in &self.welds {
            let k = self.flicker(w.seed, now);
            let px = c.pixel * (w.spot - c.eye).len();
            let hw = (w.width * 0.5).max(s.beam_min_px * px * 0.5);
            let phase = libm::fmodf(now * 0.37 + rnd(w.seed) * 10.0, 10.0);
            fx.ribbon(c, w.from, w.spot, hw, weldfx::BEAM, phase, s.beam_col, s.beam * s.brightness * k);
            let m = s.flare_min_px * px * 0.5;
            let h = (s.flare * w.width * 0.5).max(m);
            if s.hot > 0.0 {
                fx.billboard(c, w.spot, h * 0.4, h * 0.4, weldfx::HOT, 0.0, s.hot_col, s.hot * s.brightness);
            }
            let ang = rnd(w.seed.wrapping_add(((now * s.flicker_rate) as i64 as u32).wrapping_mul(2891336453))) * core::f32::consts::PI;
            fx.billboard(c, w.spot, h, h * 0.5, weldfx::FLARE, ang, s.beam_col, s.brightness * k);
            // Sparks: one every `spark_every` seconds, off the hull towards the bee.
            let back = (w.from - w.spot).norm();
            let slot = (now / s.spark_every) as i64;
            for j in 0..s.sparks as i64 {
                let i = slot - j;
                let sd = w.seed ^ (i as u32).wrapping_mul(0x9e3779b9);
                let te = (i as f32 + rnd(sd)) * s.spark_every;
                let life = s.spark_life[0] + (s.spark_life[1] - s.spark_life[0]) * rnd(sd.wrapping_add(1));
                let dir = (back + weldfx::rand_dir(rnd(sd.wrapping_add(2)), rnd(sd.wrapping_add(3))) * s.spark_spread).norm();
                let sp = s.spark_speed * (0.3 + 0.7 * rnd(sd.wrapping_add(5)));
                if let Some((tail, head, kh)) = weldfx::spark_at(w.spot, dir, sp, now - te, life, 0.8, 0.035) {
                    let hw = (s.spark_min_px * px * 0.5).max(0.05);
                    fx.streak(c, tail, head, hw, weldfx::heat_colour(kh), s.spark_bright * s.brightness * kh);
                }
            }
        }
        let _ = v3;
    }
}

impl Default for Bees {
    fn default() -> Bees {
        Bees::new()
    }
}
