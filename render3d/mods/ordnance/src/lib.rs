//! Ordnance: every live shot (`Step::Ordnance`, in place of `Ordnance::RenderAll`) and
//! the warp flash (`Step::Warp`), with the shots' light. Ports `Specials.asi` (the
//! special weapons' recipes and the warp flash) and `Lighting.asi`'s ordnance lights
//! (recolouring, phasers). README "Ordnance" holds the engine side.
//!
//! - `RenderAll`'s own loop is ours (the same tests, in the same order, and its one
//!   write); a shot it would submit is drawn by a recipe (`fx.rs`), by our copy of
//!   stock's sprite draw (`sprite.rs`), or by the engine's own `Submit` (vtable +0x5c)
//!   when we have neither: mines, pods, the tractor and mining beams' models, and the
//!   other special weapons that draw an instance.
//! - The warp flash: the engine's `RenderWarpSpriteEffects` still runs (it moves and
//!   removes the effects), with its one sprite draw re-pointed at us; we draw the flash.
//! - The special weapons' waves: `draw_wave`, which the module that draws the
//!   shockwaves (effects) asks first.
//!
//! Every address, hook and field is in `hooks.rs`; the built-in recipes in `content.rs`.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The runtime and its macros, as `crate::rt`, `crate::log` and `crate::sfmt` (r3d-core's).
#[allow(unused_imports)]
use r3d_core::{dump, log, rt, sfmt};

#[rustfmt::skip]
pub mod shaders;
mod content;
mod fx;
mod hooks;
mod ini;
mod lights;
mod models;

use r3d_core::module::{Module, Step};
use r3d_core::frame::steps::View;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// What draws a class's shots.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Who {
    Recipe(usize),
    Stock,
    /// The engine's `Submit`, with its call of `Entity::Render` ours (`models.rs`).
    Model,
    Engine,
}

pub struct Ordnance {
    sprites: r3d_core::scene::sprites::Sprites,
    fx: fx::Fx,
    models: models::Models,
    lights: lights::Lights,
    /// Stock's sprites drawn by us (0: the engine's Submit draws them).
    stock: bool,
    survey: bool,
    /// Per ordnance class: its ODF and what draws it.
    classes: BTreeMap<usize, (String, Who)>,
    seen: Vec<String>,
    frames: u32,
}

impl Default for Ordnance {
    fn default() -> Ordnance {
        Self::new()
    }
}

impl Ordnance {
    /// With the built-in recipes alone; `configure` reads the player's `Render3D.ini` over
    /// them (at load, and whenever the settings change).
    pub fn new() -> Ordnance {
        let none = crate::rt::Ini::open_plain("");
        let s = ini::Settings::load(&none);
        Ordnance {
            sprites: r3d_core::scene::sprites::Sprites::new(),
            fx: fx::Fx::new(&s),
            models: models::Models::new(&s),
            lights: lights::Lights::new(),
            stock: true,
            survey: false,
            classes: BTreeMap::new(),
            seen: Vec::new(),
            frames: 0,
        }
    }

    /// The settings (built-in recipes under `Render3D.ini`'s sections) into the module.
    /// What it holds that depends on the recipes (the classes' choice of who draws them)
    /// is forgotten only when they changed.
    fn apply(&mut self, s: &ini::Settings) {
        self.sprites.hot = s.f("Ordnance", "Hot", 0.6);
        self.sprites.hot_from = s.f("Ordnance", "HotFrom", 0.7);
        let lights = &mut self.lights;
        lights.recolour = s.i("Ordnance", "Recolour", 1) != 0;
        lights.phasers = s.i("Ordnance", "PhaserLights", 1) != 0;
        lights.ph = [
            s.f("Ordnance", "PhaserBrightness", 4.5),
            s.f("Ordnance", "PhaserStart", 0.0),
            s.f("Ordnance", "PhaserRange", 24.0),
            s.f("Ordnance", "PhaserLift", 3.0),
            s.f("Ordnance", "PhaserImpact", 2.0),
            s.f("Ordnance", "PhaserImpactStart", 6.0),
            s.f("Ordnance", "PhaserImpactRange", 70.0),
            s.f("Ordnance", "PhaserImpactLift", 8.0),
        ];
        lights.muzzle = [
            if s.i("Ordnance", "MuzzleLights", 1) != 0 { s.f("Ordnance", "MuzzleBrightness", 4.0) } else { 0.0 },
            s.f("Ordnance", "MuzzleTime", 0.5),
            s.f("Ordnance", "MuzzleReach", 1.0),
        ];
        let before = (self.stock, self.models.on, self.fx.specials);
        self.stock = s.i("Ordnance", "Stock", 1) != 0;
        self.survey = s.i("Ordnance", "Survey", 0) != 0;
        let recipes = self.fx.configure(s);
        self.models.configure(s);
        if recipes || before != (self.stock, self.models.on, self.fx.specials) {
            self.classes.clear();
        }
        hooks::set_warp_on(self.fx.warp_on());
    }

    /// What draws a class's shots, by its ODF and its Submit (cached per class).
    fn who(&mut self, ord: usize) -> Who {
        let cls = hooks::shot_class(ord);
        let odf = hooks::ord_odf(ord);
        if let Some((name, w)) = self.classes.get(&cls) {
            if name.as_str() == odf {
                return *w;
            }
        }
        let sub = hooks::submit_addr(ord);
        let w = match self.fx.recipe_of(odf, false) {
            Some(rc) if self.fx.specials => Who::Recipe(rc),
            _ if self.stock && hooks::stock_submit(ord).is_some() => Who::Stock,
            _ if self.models.take_class(ord) => Who::Model,
            _ => Who::Engine,
        };
        if self.survey || !self.seen.iter().any(|s| s == odf) {
            if !self.seen.iter().any(|s| s == odf) && self.seen.len() < 256 {
                self.seen.push(String::from(odf));
                log!("ordnance: {} (vtable {:#x}, Submit {:#x}): {:?}", odf, r3d_core::engine::vtable(ord), sub, w);
            }
        }
        self.classes.insert(cls, (String::from(odf), w));
        w
    }

    /// Our copy of stock's `Submit` for the classes `who` calls Stock.
    fn draw_stock(&mut self, v: &View, ord: usize) {
        let cls = hooks::shot_class(ord);
        let kind = hooks::stock_submit(ord);
        // The chain pulsar: its sprite is ours while the particles module fires its
        // emitter (+0xdc) and draws the trail; otherwise the engine's Submit does both.
        if kind == Some(hooks::StockSubmit::Chain) && !r3d_core::particles::live() {
            hooks::submit(ord, v.cam);
            return;
        }
        let spr = hooks::class_sprite(cls);
        if spr == 0 {
            return;
        }
        let w = hooks::class_width(cls);
        match kind.unwrap_or(hooks::StockSubmit::Plasma) {
            hooks::StockSubmit::Phaser => self.sprites.polygon(v, spr, hooks::shot_start(ord), hooks::shot_end(ord), w, w, hooks::shot_tint(ord)),
            hooks::StockSubmit::Pulse => {
                // From the head back towards the start, at most the class's length.
                let (start, end) = (hooks::shot_start(ord), hooks::shot_end(ord));
                let d = start - end;
                let l = d.len();
                let len = hooks::class_length(cls);
                let tail = if l < len { end + d } else { end + d * (len / l) };
                self.sprites.polygon(v, spr, end, tail, w, w, hooks::shot_hue_b(ord));
            }
            sub => {
                let col = hooks::stock_colour(ord, sub, spr);
                self.sprites.scaled(v, spr, hooks::transform(ord).p, w, col);
            }
        }
    }

    /// `Ordnance::RenderAll`, as stock runs it, each shot drawn by us or the engine.
    fn render_all(&mut self, v: &View) {
        let cam = v.cam;
        let eye = hooks::camera_position(cam);
        let mut list: Vec<usize> = Vec::new();
        hooks::for_each_ordnance(|o| list.push(o));
        self.fx.begin_shots();
        for ord in list {
            if ord == 0 {
                continue;
            }
            let (owner, target) = hooks::shot_parties(ord);
            if !hooks::shot_shown(ord, owner, target, eye) {
                continue;
            }
            match self.who(ord) {
                Who::Recipe(rc) => {
                    self.fx.shot(v, ord, rc);
                    if self.fx.recipes[rc].kind == fx::Kind::Bolt {
                        self.models.feed_shot(v, ord);
                    }
                }
                Who::Stock => self.draw_stock(v, ord),
                Who::Model => {
                    hooks::set_in_submit(true);
                    hooks::submit(ord, cam);
                    hooks::set_in_submit(false);
                }
                Who::Engine => hooks::submit(ord, cam),
            }
        }
        self.sprites.draw(v);
        self.fx.draw(v);
    }

    /// A model-drawing class's `Entity::Render`, ours while the ordnance step submits it
    /// (the detour in `hooks.rs` asks).
    fn models_draw(&mut self, v: &View, ord: usize, cam: usize) -> bool {
        self.models.draw(v, &mut self.fx, ord, cam)
    }
}

/// The ordnance module's draws, offered to the others (`services`).
struct Provider;
static PROVIDER: Provider = Provider;

impl r3d_core::services::WaveDraw for Provider {
    fn draw_wave(&self, expl: usize, v: &View) -> bool {
        draw_wave(expl, v)
    }
}

impl Module for Ordnance {
    fn provide(&mut self, id: r3d_core::module::ModuleId, s: &mut r3d_core::services::Services) {
        s.wave(id, &PROVIDER);
    }
    fn name(&self) -> &'static str {
        "ordnance"
    }
    fn priority(&self) -> i32 {
        50
    }
    fn claims(&self) -> u32 {
        gpu::display::CLAIM_ORDNANCE
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &[]
    }
    /// `[Ordnance]`, `[Ordnance.<odf>]`, `[Model.<odf>]`, `[Wave.<odf>]` and `[Warp]`
    /// over the built-in recipes, and `[Lights] Beam*=`, `ConeSpot=`: the beams' lights,
    /// set per line (not rows).
    fn configure(&mut self, ini: &crate::rt::Ini, _said: &mut r3d_core::module::Changes) {
        let s = ini::Settings::load(ini);
        self.apply(&s);
        self.lights.beam = lights::Beam::read(ini);
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Specials.asi")
    }
    /// The warp flash's hook; the per-class slots follow when a class is first met.
    fn hooks(&mut self, h: &mut r3d_core::hooks::Hooks) {
        hooks::set_module(self as *mut Ordnance);
        hooks::install(h);
    }
    fn on(&mut self) {
        hooks::set_module(self as *mut Ordnance);
        hooks::set_warp_on(self.fx.warp_on());
    }
    /// Switched off live: its flashes, effect ages, muzzles and noted shots go, and the
    /// classes are asked again when it is switched on. The detours hand every call to
    /// the engine's (the hooks are behind the flag).
    fn off(&mut self) {
        self.fx.drop_state();
        self.lights.drop_state();
        self.classes.clear();
        hooks::set_view(None);
        hooks::clear_noted();
    }
    fn reset(&mut self) {
        self.sprites.reset();
        self.fx.reset();
    }
    fn begin_view(&mut self, v: &View) {
        hooks::set_view(Some(v));
        let ls = self.fx.take_lights(v.time, v.main).to_vec();
        if self.lights.begin_view(&ls, v.time, v.main) {
            r3d_core::scene::regather_lights();
        }
        // With clustered lights, this module's points are the scene's own (not
        // registered with the engine): added after the list is read again.
        for p in self.lights.take_points() {
            let m = p.medium;
            r3d_core::scene::add_point(p, m);
        }
        if v.main {
            self.frames = self.frames.wrapping_add(1);
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        match s {
            Step::Ordnance => {
                self.render_all(v);
                true
            }
            Step::Warp if hooks::warp_hooked() && self.fx.warp_on() => {
                hooks::clear_noted();
                hooks::run_warp(v.cam);
                for (eff, pos) in hooks::noted() {
                    self.fx.note_flash(eff, pos, v.time);
                }
                self.fx.warp_draw(v);
                true
            }
            _ => false,
        }
    }
    fn end_view(&mut self, _v: &View) {
        hooks::set_view(None);
    }
}

/// A special weapon's wave: `expl` is a `ShockwaveExplosion` (an `ExplosionFeature`'s
/// explosion). If its class names, through its `cPrjID`, an ODF with a `[Wave.<odf>]`
/// recipe, the wave is drawn now and true returned; otherwise nothing is drawn (the
/// caller draws its ring). The effects module calls this before it draws a shockwave.
/// False while the ordnance module is off or standing down.
fn draw_wave(expl: usize, v: &View) -> bool {
    let Some(m) = hooks::module() else { return false };
    if !m.fx.waves || expl == 0 {
        return false;
    }
    let odf = hooks::wave_odf(expl);
    match m.fx.recipe_of(odf, true) {
        Some(rc) => {
            m.fx.wave(v, expl, rc);
            true
        }
        None => false,
    }
}
