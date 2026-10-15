//! Effects: the game features that are effects (explosions, shield hits, the repair
//! welding), hull debris as our own simulation, and their lights. Ports
//! `Explosions.asi`, `Shields.asi` and `Repair.asi`, whose looks were settled with the
//! user (their READMEs); each part stands down while its plugin is loaded, so an
//! install that still has the plugin keeps its look.
//!
//! - `explosions`: `ExplosionFeature` taken through `feature()`: fireballs ray-marched
//!   as gas, shockwaves as a band of fire; any other explosion class (`OmegaExplosion`)
//!   is submitted to the engine as stock does.
//! - `shields`: `ShieldEffect` taken through `feature()`: the ordinary hits, the
//!   shields-down arcs and the special weapons' shells with a recipe are ours, every
//!   other shell is the engine's `ShieldHit::DrawShield`.
//! - `repair`: the welding sparks of `Craft::RepairSystem`; `NodeParticleEffect`'s
//!   `Render` runs, with the welds (and, with `sparks`, every particle) skipped.
//! - `sparks`: `NodeParticleEffect`'s other particles (systems blowing up, phaser
//!   flares, beam hits): stock's sprites, with sparks streaking off and a flash.
//! - `construction`: `BuildingEffect` through `feature()`: the station being built via
//!   the hulls module, welding arcs and cooling seams along its new edges.
//! - `bees`: `WorkerBee` and `DroneColony` through `feature()`: the bees as lit
//!   models, their welding beams as arcs of light. (`weldfx`: their shared quads.)
//! - `chunks`: `Step::Chunks`, hull debris of our own (`spec-chunks-anim-lod.md` §1).
//!
//! Everything noted while the features run is drawn at `Step::Chunks`, after the
//! debris: every opaque hull and chunk is then in the depth buffer, as the plugins
//! drew after `RenderChunks`. `Step::Specials` stays the engine's; `NaniteEffect` is
//! never made by the game.
pub mod bees;
pub mod chunks;
pub mod construction;
pub mod explosions;
pub mod repair;
pub mod shields;
pub mod sparks;
pub mod weldfx;

use super::{Module, Step};
use crate::d3d::*;
use crate::engine::{self, field, hook, rd};
use crate::frame::steps::View;
use crate::log;
use crate::math::{v3, M34, V3};
use alloc::vec::Vec;

const SITE_CHUNKS: usize = 0x598446; // ST3D_GraphicsEngine::RenderChunks(), thiscall
const FN_CHUNKS: usize = 0x62d470;
const VT_EXPLOSION_FEATURE: usize = 0x6afb30;
const VT_SHIELD_EFFECT: usize = 0x6b0134;
const VT_NODE_PARTICLE: usize = 0x6b001c;

const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), static, cdecl
const FN_NEW: usize = 0x652710; // operator new, cdecl
const FN_POINT_LIGHT: usize = 0x62f240; // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
const FN_REGISTER_LIGHT: usize = 0x62d250; // ST3D_GraphicsEngine::RegisterLight(light, colour&, Matrix34&), thiscall

/// A live game object by handle, as the plugins took one: `Entity::Get`, a game
/// object (flags bit 2), with an instance and a class. 0 if not.
pub fn object_of(handle: i32) -> usize {
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(FN_ENTITY_GET) };
    let o = unsafe { f(handle) };
    if o == 0 || field(o, 0x14) & 4 == 0 || field(o, 4) == 0 || field(o, 0x40) == 0 {
        return 0;
    }
    o
}

/// 32-bit integer hash (the plugins' `hash32`).
pub fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

/// 0..1 from a seed.
pub fn rnd(seed: u32) -> f32 {
    (hash32(seed) & 0xffffff) as f32 / 16777216.0
}

/// The camera of a view, as the plugins read it off the device's VIEW and PROJECTION.
#[derive(Clone, Copy)]
pub struct Cam {
    pub eye: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    /// Near plane's view depth.
    pub zn: f32,
    /// Projection `_11`, `_22`, `_33`, `_43`.
    pub px: f32,
    pub py: f32,
    pub p33: f32,
    pub p43: f32,
    /// A pixel's size at unit distance.
    pub pixel: f32,
    /// View x projection, transposed (the columns, for `dot(p, col)` in the shaders).
    pub vp: [f32; 16],
}

impl Cam {
    pub fn of(v: &View) -> Option<Cam> {
        let m = &v.view.0;
        let p = &v.proj.0;
        if p[2][3] == 0.0 || p[2][2] == 0.0 {
            return None;
        }
        let right = v3(m[0][0], m[1][0], m[2][0]);
        let up = v3(m[0][1], m[1][1], m[2][1]);
        let fwd = v3(m[0][2], m[1][2], m[2][2]);
        let t = v3(m[3][0], m[3][1], m[3][2]);
        let eye = -(right * t.x + up * t.y + fwd * t.z);
        let h = v.rect.h.max(1) as f32;
        Some(Cam {
            eye,
            right,
            up,
            fwd,
            zn: -p[3][2] / p[2][2],
            px: p[0][0],
            py: p[1][1],
            p33: p[2][2],
            p43: p[3][2],
            pixel: if p[1][1] > 1e-6 { 2.0 / (p[1][1] * h) } else { 0.001 },
            vp: crate::gfx::vp_constants(v),
        })
    }
}

/// Engine point lights of our own, made on first use and registered each view (the
/// plugins' way: an ordinary `ST3D_Point_Light` registered after `GameObject_PreRenderAll`).
pub struct LightPool {
    name: &'static [u8],
    objs: Vec<usize>,
}

impl LightPool {
    pub const fn new(name: &'static [u8]) -> LightPool {
        LightPool { name, objs: Vec::new() }
    }
    /// Registers light `i` of the pool at `pos`: full colour to `start`, gone `range`
    /// further on. False if no light object could be made.
    pub fn register(&mut self, i: usize, pos: V3, colour: V3, start: f32, range: f32) -> bool {
        while self.objs.len() <= i {
            let l = unsafe {
                let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(FN_NEW);
                let l = new(0x138);
                if l == 0 {
                    return false;
                }
                let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(FN_POINT_LIGHT);
                ctor(l, 0, 0, self.name.as_ptr());
                l
            };
            self.objs.push(l);
        }
        let l = self.objs[i];
        unsafe {
            core::ptr::write_unaligned((l + 0x100) as *mut f32, start);
            core::ptr::write_unaligned((l + 0x104) as *mut f32, range);
        }
        let col = [colour.x, colour.y, colour.z, 1.0f32];
        let m = M34 { r: M34::IDENTITY.r, p: pos };
        unsafe {
            let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
            reg(engine::storm3d(), l, col.as_ptr(), &m);
        }
        true
    }
}

/// Settings from `Render3D.ini`, one section per part.
pub struct Sec(&'static str, crate::rt::Ini);

impl Sec {
    pub fn new(name: &'static str) -> Sec {
        Sec(name, crate::config::ini())
    }
    pub fn f(&self, key: &str, def: f32) -> f32 {
        self.1.float(self.0, key, def)
    }
    pub fn i(&self, key: &str, def: i32) -> i32 {
        self.1.int(self.0, key, def)
    }
    /// A colour as `r,g,b` in 0..255, returned in 0..1.
    pub fn rgb(&self, key: &str, def: [f32; 3]) -> V3 {
        let c = self.1.floats(self.0, key, def);
        v3(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0)
    }
    /// Three plain floats.
    pub fn v3(&self, key: &str, def: [f32; 3]) -> V3 {
        let c = self.1.floats(self.0, key, def);
        v3(c[0], c[1], c[2])
    }
    pub fn pair(&self, key: &str, def: [f32; 2]) -> [f32; 2] {
        self.1.floats(self.0, key, def)
    }
    pub fn s(&self, key: &str) -> Option<alloc::string::String> {
        self.1.string(self.0, key)
    }
}

pub struct Effects {
    ex: explosions::Explosions,
    sh: shields::Shields,
    rp: repair::Repair,
    ch: chunks::Chunks,
    co: construction::Construction,
    bees: bees::Bees,
    sp: sparks::SparkFx,
    fx: weldfx::Fx,
    hooked: bool,
    /// Bench A/B (`[Construction] ABTest=1`): while `Render3D.construction-off` exists in
    /// the game directory, construction, the bees and the sparks are the engine's.
    ab: bool,
    ab_off: bool,
    ab_checked: f64,
}

impl Effects {
    pub fn new() -> Effects {
        Effects {
            ex: explosions::Explosions::new(),
            sh: shields::Shields::new(),
            rp: repair::Repair::new(),
            ch: chunks::Chunks::new(),
            co: construction::Construction::new(),
            bees: bees::Bees::new(),
            sp: sparks::SparkFx::new(),
            fx: weldfx::Fx::new(),
            hooked: false,
            ab: crate::config::ini().int("Construction", "ABTest", 0) != 0,
            ab_off: false,
            ab_checked: 0.0,
        }
    }
}

impl Default for Effects {
    fn default() -> Effects {
        Effects::new()
    }
}

/// The engine's own `RenderChunks`, through its call site (a plugin chained there
/// runs too).
fn engine_chunks() {
    let f = hook::call_target(SITE_CHUNKS).unwrap_or(FN_CHUNKS);
    unsafe {
        let r: unsafe extern "thiscall" fn(usize) = core::mem::transmute(f);
        r(engine::storm3d());
    }
}

impl Module for Effects {
    fn name(&self) -> &'static str {
        "effects"
    }
    fn device(&mut self, _dev: Device) {
        if self.hooked {
            return;
        }
        self.hooked = true;
        // Each part stands down beside the plugin it ports.
        let loaded = |p: &str| {
            let l = crate::plugin_loaded(p);
            if l {
                log!("effects: {} is loaded: it keeps drawing its part", p);
            }
            l
        };
        self.ex.on = self.ex.on && !loaded("Explosions.asi");
        self.ex.light_on = self.ex.light_on && !crate::plugin_loaded("Lighting.asi");
        self.sh.on = self.sh.on && !loaded("Shields.asi");
        self.rp.on = self.rp.on && !loaded("Repair.asi");
        if self.sh.on {
            self.sh.install();
        }
        if self.rp.on {
            self.rp.install();
        }
        if self.ch.on {
            self.ch.install();
        }
        if self.sp.on {
            self.sp.install();
        }
        log!(
            "effects: explosions {}, explosion light {}, shields {}, repair {}, debris {}, construction {}, bees {}, sparks {}",
            self.ex.on, self.ex.light_on, self.sh.on, self.rp.on, self.ch.on, self.co.on, self.bees.on, self.sp.on
        );
    }
    fn reset(&mut self) {
        self.ex.reset();
        self.ch.reset();
        self.sp.reset();
    }
    fn lights(&mut self, v: &View) {
        self.ex.lights(v);
        self.sh.lights(v);
        self.rp.lights(v);
        if !self.ab_off {
            if self.co.on {
                self.co.lights(v);
            }
            if self.bees.on {
                self.bees.lights(v);
            }
            if self.sp.on {
                self.sp.lights(v);
            }
        }
    }
    fn begin_view(&mut self, _v: &View) {
        self.ex.notes.clear();
        self.bees.begin_view();
        if self.ab {
            let now = crate::rt::now();
            if now - self.ab_checked > 0.5 {
                self.ab_checked = now;
                let off = crate::rt::file_exists(&crate::rt::game_path("Render3D.construction-off"));
                if off != self.ab_off {
                    self.ab_off = off;
                    log!("effects: construction, bees, sparks: {}", if off { "the engine's (A/B)" } else { "ours (A/B)" });
                    sparks::set_taking(self.sp.on && !off);
                }
            }
        }
    }
    /// The station under construction as an object (`ConstructionObject`, `mconstph`):
    /// its `Render` (0x4a3790) draws nothing; the station is `BuildingEffect`'s, drawn in
    /// `feature()`. Taken here so the object loop leaves nothing of it to the engine.
    fn object(&mut self, o: usize, _v: &View) -> bool {
        self.co.on && !self.ab_off && field(engine::vtable(o), 11 * 4) == construction::FN_CONSTRUCTION_OBJECT_RENDER
    }
    fn feature(&mut self, f: usize, v: &View) -> bool {
        match engine::vtable(f) {
            VT_EXPLOSION_FEATURE if self.ex.on => self.ex.feature(v),
            VT_SHIELD_EFFECT if self.sh.on => self.sh.feature(v),
            // The engine's Render draws every other spark; the welds are skipped there
            // (repair.rs) and drawn by us at Step::Chunks.
            VT_NODE_PARTICLE => false,
            construction::VT_BUILDING if self.co.on && !self.ab_off => self.co.feature(v),
            bees::VT_WORKER_BEE if self.bees.on && !self.ab_off => self.bees.feature(f, false, v),
            bees::VT_DRONE_COLONY if self.bees.on && !self.ab_off => self.bees.feature(f, true, v),
            _ => false,
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s != Step::Chunks {
            return false;
        }
        if self.ch.on {
            let _z = crate::timing::zone("effects: debris");
            self.ch.simulate(v.time);
            self.ch.draw(v);
        }
        // The engine's chunks (construction's, and a plugin's work chained there).
        {
            let _z = crate::timing::zone("effects: engine chunks");
            engine_chunks();
        }
        if let Some(c) = Cam::of(v) {
            if self.ex.on {
                let _z = crate::timing::zone("effects: explosions");
                self.ex.draw(v, &c);
            }
            if self.sh.on {
                let _z = crate::timing::zone("effects: shields");
                self.sh.draw(v, &c);
            }
            if self.rp.on {
                let _z = crate::timing::zone("effects: repair");
                self.rp.draw(v, &c);
            }
            if self.co.on && !self.ab_off {
                self.co.draw(v, &c, &mut self.fx);
            }
            if self.bees.on && !self.ab_off {
                self.bees.draw(v, &c, &mut self.fx);
            }
            if self.sp.on && !self.ab_off {
                let _z = crate::timing::zone("effects: sparks");
                self.sp.draw(v, &c, &mut self.fx);
            }
            let _z = crate::timing::zone("effects: quads");
            self.fx.draw(v, &c);
        }
        self.ex.notes.clear();
        true
    }
}

/// Reads an `f32` field.
pub fn ff(base: usize, off: usize) -> f32 {
    engine::field_f32(base, off)
}

/// Reads a `V3` at an address.
pub fn rv3(a: usize) -> V3 {
    unsafe { rd::<V3>(a) }
}
