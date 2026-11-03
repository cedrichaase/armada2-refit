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
pub mod content;
pub mod explosions;
pub mod hooks;
pub mod repair;
pub mod shields;
pub mod sparks;
pub mod weldfx;

use super::{Module, Step};
use crate::core_hooks::Hooks;
use crate::core_module::Changes;
use crate::frame::steps::View;
use crate::log;
use crate::rt::Ini;
use hooks::{construction as hk_construction, Feature};

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

pub use crate::math::rand::{hash32, rnd};
pub use hooks::object_of;

/// An ini that holds nothing: what a part's `new()` reads its defaults from, before
/// `configure` gives it the real one.
pub fn blank() -> Ini {
    Ini::open_plain("")
}

/// This module's media (`scene::media`): a fireball's light to the gas
/// (`[Nebulae] LitExplosion=`), and a weld's, a bee's or a shields-down discharge's, a
/// spark (`LitSpark=`).
pub(crate) static FIREBALL: crate::scene::lights::Medium = crate::scene::lights::Medium::new("fireball", crate::scene::media::FIREBALL);
pub(crate) static SPARK: crate::scene::lights::Medium = crate::scene::lights::Medium::new("spark", crate::scene::media::SPARK);
pub use crate::scene::camera::Cam;
pub use crate::scene::lights::Pool as LightPool;

/// Settings from `Render3D.ini`, one section per part (`config::Section`).
pub use crate::config::Section as Sec;

/// Which parts' hooks went in (`Effects::hk`).
const HK_SHIELDS: u8 = 1;
const HK_REPAIR: u8 = 2;
const HK_DEBRIS: u8 = 4;

pub struct Effects {
    ex: explosions::Explosions,
    sh: shields::Shields,
    rp: repair::Repair,
    ch: chunks::Chunks,
    co: construction::Construction,
    bees: bees::Bees,
    sp: sparks::SparkFx,
    fx: weldfx::Fx,
    /// `hooks()` ran, and which parts' hooks are in (`HK_*`).
    hooked: bool,
    /// The hooks handle kept for `device()`.
    later: Option<Hooks>,
    hk: u8,
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
            later: None,
            hk: 0,
            ab: false,
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

impl Effects {
    /// What runs now: each part when it wants to (its `Enable`, and no plugin of its
    /// own loaded) and, for the parts with hooks, when their hooks are in. The
    /// statics the detours read follow.
    fn refresh(&mut self) {
        self.ex.on = self.ex.want;
        self.sh.on = self.sh.want && self.hk & HK_SHIELDS != 0;
        self.rp.on = self.rp.want && self.hk & HK_REPAIR != 0;
        self.ch.on = self.ch.want && self.hk & HK_DEBRIS != 0;
        self.co.on = self.co.want;
        self.bees.on = self.bees.want;
        // The sparks take over the particles only where repair's thunk is in place.
        self.sp.on = self.sp.want && hooks::repair::hooked();
        repair::Repair::set_taking(self.rp.on);
        chunks::set_active(self.ch.on);
        sparks::set_taking(self.sp.on && !self.ab_off);
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
    /// `[Explosions]`, `[Shields]` (and the `[Shell.<name>]` recipes), `[Repair]`,
    /// `[Debris]`, `[Construction]`, `[Bees]` and `[Sparks]`, one part each; each part
    /// stands down while its plugin is loaded, so an install that still has the plugin
    /// keeps its look.
    fn configure(&mut self, ini: &Ini, _said: &mut Changes) {
        self.ex.configure(ini);
        self.sh.configure(ini);
        self.rp.configure(ini);
        self.ch.configure(ini);
        self.co.configure(ini);
        self.bees.configure(ini);
        self.sp.configure(ini);
        self.ab = ini.int("Construction", "ABTest", 0) != 0;
        let loaded = |p: &str| {
            let l = crate::plugin_loaded(p);
            if l && !self.hooked {
                log!("effects: {} is loaded: it keeps drawing its part", p);
            }
            l
        };
        let (ex, li, sh, rp) = (loaded("Explosions.asi"), loaded("Lighting.asi"), loaded("Shields.asi"), loaded("Repair.asi"));
        self.ex.want = self.ex.want && !ex;
        self.ex.light_on = self.ex.light_on && !li;
        self.sh.want = self.sh.want && !sh;
        self.rp.want = self.rp.want && !rp;
        self.refresh();
    }
    /// The module's first switch-on: its hooks are put in at its first frame
    /// (`device()`), not now. Other plugins check the stock bytes of the sites they
    /// patch themselves when they load, which can be after this one (the bench's
    /// `Scene.asi` is one: it reads the shield hit sites), and the hooks chain onto
    /// whatever is there, as they always did.
    fn hooks(&mut self, h: &mut Hooks) {
        hooks::keep_gate(h);
        self.later = Some(h.clone());
    }
    /// The hooks of the parts that want them, once, at the first frame the module has a
    /// device; the sparks need repair's.
    fn device(&mut self) {
        let Some(mut h) = self.later.take() else { return };
        let h = &mut h;
        self.hooked = true;
        if self.sh.want {
            hooks::shields::install(h);
            self.hk |= HK_SHIELDS;
        }
        if self.rp.want && hooks::repair::install(h) {
            self.hk |= HK_REPAIR;
        }
        if self.ch.want && hooks::chunks::install(h) {
            self.hk |= HK_DEBRIS;
        }
        if self.sp.want && hooks::repair::hooked() == false {
            log!("effects: sparks: NodeParticleEffect's draw is not hooked (repair off): stock sparks");
        }
        self.refresh();
        log!(
            "effects: explosions {}, explosion light {}, shields {}, repair {}, debris {}, construction {}, bees {}, sparks {}",
            self.ex.on, self.ex.light_on, self.sh.on, self.rp.on, self.ch.on, self.co.on, self.bees.on, self.sp.on
        );
    }
    fn on(&mut self) {
        self.refresh();
    }
    /// Switched off live: the debris goes (the engine's draws take over through the
    /// hooks' gate).
    fn off(&mut self) {
        // The debris is our own simulation and goes; what the other parts hold (the
        // shield hits' births, the welds and construction edges noted) is keyed to the
        // engine's own records, which live on while the engine draws them, so it stays
        // and a switch-on picks the effects up where they are.
        self.ex.notes.clear();
        self.sp.clear();
        self.ch.clear();
        self.bees.clear();
        chunks::set_active(false);
        repair::Repair::set_taking(false);
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
        self.co.on && !self.ab_off && hk_construction::is_construction_object(o)
    }
    fn feature(&mut self, f: usize, v: &View) -> bool {
        match hooks::feature_kind(f) {
            Feature::Explosion if self.ex.on => self.ex.feature(v),
            Feature::Shield if self.sh.on => self.sh.feature(v),
            // The engine's Render draws every other spark; the welds are skipped there
            // (hooks.rs) and drawn by us at Step::Chunks.
            Feature::NodeParticle => false,
            Feature::Building if self.co.on && !self.ab_off => self.co.feature(v),
            Feature::WorkerBee if self.bees.on && !self.ab_off => self.bees.feature(f, false, v),
            Feature::DroneColony if self.bees.on && !self.ab_off => self.bees.feature(f, true, v),
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
            hooks::engine_chunks();
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
