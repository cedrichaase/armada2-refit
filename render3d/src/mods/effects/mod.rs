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
use crate::engine::{self, field, hook};
use crate::frame::steps::View;
use crate::log;

/// Positions alone (the fireballs' and the shields' quads and spheres).
pub static XYZ: [gpu::VertexAttr; 1] = [gpu::attr(0, gpu::AttrFormat::Float3, gpu::Semantic::Position, 0)];
/// A position and two float4 texture coordinates (the welds' and construction's quads).
pub static QUAD2: [gpu::VertexAttr; 3] = [
    gpu::attr(0, gpu::AttrFormat::Float3, gpu::Semantic::Position, 0),
    gpu::attr(12, gpu::AttrFormat::Float4, gpu::Semantic::TexCoord, 0),
    gpu::attr(28, gpu::AttrFormat::Float4, gpu::Semantic::TexCoord, 1),
];

pub use crate::gfx::fx::fx_desc;

/// Premultiplied over what is there: `ONE`, `INVSRCALPHA`.
pub const PREMUL: gpu::Blend =
    gpu::Blend { color: gpu::BlendEq { src: gpu::Factor::One, dst: gpu::Factor::InvSrcAlpha, op: gpu::BlendOp::Add }, alpha: None };

const SITE_CHUNKS: usize = 0x598446; // ST3D_GraphicsEngine::RenderChunks(), thiscall
const FN_CHUNKS: usize = 0x62d470;
const VT_EXPLOSION_FEATURE: usize = 0x6afb30;
const VT_SHIELD_EFFECT: usize = 0x6b0134;
const VT_NODE_PARTICLE: usize = 0x6b001c;

const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), static, cdecl

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

pub use crate::math::rand::{hash32, rnd};

pub use crate::engine::{ff, rv3};

/// This module's media (`scene::media`): a fireball's light to the gas
/// (`[Nebulae] LitExplosion=`), and a weld's, a bee's or a shields-down discharge's, a
/// spark (`LitSpark=`).
pub(crate) static FIREBALL: crate::scene::lights::Medium = crate::scene::lights::Medium::new("fireball", crate::scene::media::FIREBALL);
pub(crate) static SPARK: crate::scene::lights::Medium = crate::scene::lights::Medium::new("spark", crate::scene::media::SPARK);
pub use crate::scene::camera::Cam;
pub use crate::scene::lights::Pool as LightPool;

/// Settings from `Render3D.ini`, one section per part (`config::Section`).
pub use crate::config::Section as Sec;

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
    fn priority(&self) -> i32 {
        40
    }
    fn claims(&self) -> u32 {
        gpu::display::CLAIM_EFFECTS
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &[]
    }
    fn configure(&mut self, _ini: &crate::rt::Ini, _said: &mut crate::core_module::Changes) {}
    fn device(&mut self) {
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
