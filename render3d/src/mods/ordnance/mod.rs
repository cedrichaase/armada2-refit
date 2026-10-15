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
mod eng;
mod fx;
mod ini;
mod lights;
mod models;
mod sprite;

use super::{Module, Step};
use crate::d3d::Device;
use crate::engine::{self, field, field_f32, field_u8, hook, rd};
use crate::frame::steps::View;
use crate::log;
use crate::math::V3;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

const CULL_DISTANCE: usize = 0x6fcac0; // cfgOBJECT_CULLING_DISTANCE, f32
const SLOT_SUBMIT: usize = 0x5c; // Ordnance vtable: Submit(ST3D_Camera*)

// Stock's Submit functions this module draws in their place (sprite.rs).
const SUB_PHASER: usize = 0x57edc0;
const SUB_PULSE: usize = 0x581cb0;
const SUB_PHOTON: usize = 0x57f100;
const SUB_PLASMA: usize = 0x57f580;
const SUB_POLERON: usize = 0x5800d0;
const SUB_ARTILLERY: usize = 0x5730a0;
const SUB_SRT: usize = 0x5853b0; // ShieldRemovingTorpedo
const SUB_CHAIN: usize = 0x574f20; // ChainPulsar: its sprite, and its emitter node's Render


// The warp flash: the call to ST3D_Sprite::DrawScaled3D in WarpSpriteEffect::Render.
const SITE_WARP_SPRITE: usize = 0x59cc6b;
const FN_DRAW_SCALED: usize = 0x63aa60;
const SITE_WARP: usize = 0x598423; // RenderWarpSpriteEffects(cam), cdecl, in the body
const FN_WARP: usize = 0x59c560;

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
    sprites: sprite::Sprites,
    fx: fx::Fx,
    models: models::Models,
    lights: lights::Lights,
    /// Stock's sprites drawn by us (0: the engine's Submit draws them).
    stock: bool,
    survey: bool,
    hooked: bool,
    warp_hooked: bool,
    /// Per ordnance class: its ODF and what draws it.
    classes: BTreeMap<usize, (String, Who)>,
    seen: Vec<String>,
    frames: u32,
}

static mut MODULE: *mut Ordnance = core::ptr::null_mut();
static mut VIEW: *const View = core::ptr::null();
static mut PREV_WARP_SPRITE: usize = FN_DRAW_SCALED;
/// Flashes noted by the warp hook in this view: (effect, position).
static mut NOTED: Vec<(usize, V3)> = Vec::new();

/// The module and the view being drawn, while one is (for the hooks).
fn current() -> Option<(&'static mut Ordnance, &'static View)> {
    unsafe {
        if MODULE.is_null() || VIEW.is_null() {
            None
        } else {
            Some((&mut *MODULE, &*VIEW))
        }
    }
}

impl Default for Ordnance {
    fn default() -> Ordnance {
        Self::new()
    }
}

impl Ordnance {
    pub fn new() -> Ordnance {
        let s = ini::Settings::load();
        let mut sprites = sprite::Sprites::new();
        sprites.hot = s.f("Ordnance", "Hot", 0.6);
        sprites.hot_from = s.f("Ordnance", "HotFrom", 0.7);
        let mut lights = lights::Lights::new();
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
        Ordnance {
            sprites,
            fx: fx::Fx::new(&s),
            models: models::Models::new(&s),
            lights,
            stock: s.i("Ordnance", "Stock", 1) != 0,
            survey: s.i("Ordnance", "Survey", 0) != 0,
            hooked: false,
            warp_hooked: false,
            classes: BTreeMap::new(),
            seen: Vec::new(),
            frames: 0,
        }
    }

    fn install(&mut self) {
        if self.hooked {
            return;
        }
        self.hooked = true;
        unsafe { MODULE = self as *mut Ordnance };
        if self.fx.warp_on() {
            self.warp_hooked = hook::patch_call(SITE_WARP_SPRITE, FN_DRAW_SCALED, warp_sprite as *const () as usize);
            log!(
                "ordnance: warp flash {}",
                if self.warp_hooked { "hooked (WarpSpriteEffect::Render's sprite)" } else { "NOT hooked: the site is not stock's, the engine draws it" }
            );
        }
    }

    /// What draws a class's shots, by its ODF and its Submit (cached per class).
    fn who(&mut self, ord: usize) -> Who {
        let cls = field(ord, 0x34);
        let odf = eng::ord_odf(ord);
        if let Some((name, w)) = self.classes.get(&cls) {
            if name.as_str() == odf {
                return *w;
            }
        }
        let sub = field(engine::vtable(ord), SLOT_SUBMIT);
        let w = match self.fx.recipe_of(odf, false) {
            Some(rc) if self.fx.specials => Who::Recipe(rc),
            _ if self.stock && matches!(sub, SUB_PHASER | SUB_PULSE | SUB_PHOTON | SUB_PLASMA | SUB_POLERON | SUB_ARTILLERY | SUB_SRT | SUB_CHAIN) => Who::Stock,
            _ if self.models.take_class(ord) => Who::Model,
            _ => Who::Engine,
        };
        if self.survey || !self.seen.iter().any(|s| s == odf) {
            if !self.seen.iter().any(|s| s == odf) && self.seen.len() < 256 {
                self.seen.push(String::from(odf));
                log!("ordnance: {} (vtable {:#x}, Submit {:#x}): {:?}", odf, engine::vtable(ord), sub, w);
            }
        }
        self.classes.insert(cls, (String::from(odf), w));
        w
    }

    /// Our copy of stock's `Submit` for the classes `who` calls Stock.
    fn draw_stock(&mut self, v: &View, ord: usize) {
        let cls = field(ord, 0x34);
        // The chain pulsar: its sprite is ours while the particles module fires its
        // emitter (+0xdc) and draws the trail; otherwise the engine's Submit does both.
        if field(engine::vtable(ord), SLOT_SUBMIT) == SUB_CHAIN && !crate::mods::particles::live() {
            self.submit(v, ord);
            return;
        }
        let spr = field(cls, 0x12c);
        if spr == 0 {
            return;
        }
        let w = field_f32(cls, 0x148);
        let v3at = |off: usize| -> V3 { unsafe { rd(ord + off) } };
        match field(engine::vtable(ord), SLOT_SUBMIT) {
            SUB_PHASER => self.sprites.polygon(v, spr, v3at(0xbc), v3at(0xc8), w, w, v3at(0xf0)),
            SUB_PULSE => {
                // From the head (+0xc8) back towards the start, at most the class's
                // length (+0x144).
                let (start, end) = (v3at(0xbc), v3at(0xc8));
                let d = start - end;
                let l = d.len();
                let len = field_f32(cls, 0x144);
                let tail = if l < len { end + d } else { end + d * (len / l) };
                self.sprites.polygon(v, spr, end, tail, w, w, v3at(0xd4));
            }
            sub => {
                let col = match sub {
                    SUB_PHOTON | SUB_POLERON | SUB_SRT => v3at(0xd0),
                    SUB_ARTILLERY => v3at(0xd4),
                    // The plasma cannon sets no colour: the sprite's own.
                    _ => unsafe { rd::<V3>(spr + 0x24) },
                };
                self.sprites.scaled(v, spr, eng::transform(ord).p, w, col);
            }
        }
    }

    /// The engine's own `Submit` of a shot (vtable +0x5c).
    fn submit(&mut self, v: &View, ord: usize) {
        unsafe {
            let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(ord), SLOT_SUBMIT));
            f(ord, v.cam);
        }
    }

    /// `Ordnance::RenderAll`, as stock runs it, each shot drawn by us or the engine.
    fn render_all(&mut self, v: &View) {
        let cam = v.cam;
        let cull = unsafe { rd::<f32>(CULL_DISTANCE) };
        let eye = eng::camera_position(cam);
        let mut list: Vec<usize> = Vec::new();
        eng::for_each_ordnance(|o| list.push(o));
        self.fx.begin_shots();
        for ord in list {
            if ord == 0 {
                continue;
            }
            // The owner, if it is a game object and not +0x113 (gone).
            let mut owner = eng::entity_get(unsafe { rd::<i32>(ord + 0x5c) });
            if owner != 0 && (field(owner, 0x14) & 4 == 0 || field_u8(owner, 0x113) != 0) {
                owner = 0;
            }
            let mut target = eng::entity_get(unsafe { rd::<i32>(ord + 0x60) });
            if target != 0 && field(target, 0x14) & 4 == 0 {
                target = 0;
            } else if target != 0 && field_u8(target, 0x10f) != 0 {
                // Stock's one write in RenderAll: a shot whose target has +0x10f set
                // forgets it. Kept, so the game runs as it does without us.
                unsafe { core::ptr::write_unaligned((ord + 0x60) as *mut i32, 0) };
                target = 0;
            } else if target != 0 {
                let h = unsafe { rd::<i32>(ord + 0x5c) };
                let o2 = eng::obj_even_if_destroyed(h);
                if o2 != 0 && field_u8(o2, 0x113) == 0 && field_u8(o2, 0x112) == 0 && field_u8(target, 0x112) != 0 {
                    // ... and so does one whose owner is not +0x112 while its target is.
                    unsafe { core::ptr::write_unaligned((ord + 0x60) as *mut i32, 0) };
                    target = 0;
                }
            }
            let shown = (owner != 0 && field_u8(owner, 0x183) == 0 && eng::can_see(owner))
                || (target != 0 && field_u8(target, 0x183) == 0 && eng::can_see(target))
                || ((eye - eng::transform(ord).p).len() <= cull && eng::can_see(ord));
            if !shown {
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
                Who::Model => unsafe {
                    models::IN_SUBMIT = true;
                    self.submit(v, ord);
                    models::IN_SUBMIT = false;
                },
                Who::Engine => self.submit(v, ord),
            }
        }
        self.sprites.draw(v);
        self.fx.draw(v);
    }
}

impl Module for Ordnance {
    fn name(&self) -> &'static str {
        "ordnance"
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Specials.asi")
    }
    fn device(&mut self, _dev: Device) {
        self.install();
    }
    fn reset(&mut self) {
        self.sprites.reset();
        self.fx.reset();
    }
    fn begin_view(&mut self, v: &View) {
        unsafe { VIEW = v as *const View };
        let ls = self.fx.take_lights(v.time, v.main).to_vec();
        if self.lights.begin_view(&ls) {
            crate::scene::regather_lights();
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
            Step::Warp if self.warp_hooked => {
                unsafe {
                    NOTED.clear();
                    let f: unsafe extern "C" fn(usize) = core::mem::transmute(hook::call_target(SITE_WARP).unwrap_or(FN_WARP));
                    f(v.cam);
                    for &(eff, pos) in NOTED.iter() {
                        self.fx.note_flash(eff, pos, v.time);
                    }
                }
                self.fx.warp_draw(v);
                true
            }
            _ => false,
        }
    }
    fn end_view(&mut self, _v: &View) {
        unsafe { VIEW = core::ptr::null() };
    }
}

/// In `WarpSpriteEffect::Render`, in place of its `DrawScaled3D` of stock's warp
/// sprite: notes the flash (at `pos`, the effect's +4) for our warp step.
unsafe extern "thiscall" fn warp_sprite(sprite: usize, cam: usize, pos: *const V3, size: f32) {
    unsafe {
        if !VIEW.is_null() && !pos.is_null() && NOTED.len() < 64 {
            let eff = pos as usize - 4;
            if eng::object_of(rd::<i32>(eff)) != 0 {
                NOTED.push((eff, core::ptr::read_unaligned(pos)));
                return;
            }
        }
        let f: unsafe extern "thiscall" fn(usize, usize, *const V3, f32) = core::mem::transmute(PREV_WARP_SPRITE);
        f(sprite, cam, pos, size);
    }
}

/// A special weapon's wave: `expl` is a `ShockwaveExplosion` (an `ExplosionFeature`'s
/// explosion). If its class (+0x30) names, through its `cPrjID` (+0x24), an ODF with a
/// `[Wave.<odf>]` recipe, the wave is drawn now and true returned; otherwise nothing
/// is drawn (the caller draws its ring). The effects module calls this before it draws
/// a shockwave. False while the ordnance module is off or standing down.
pub fn draw_wave(expl: usize, v: &View) -> bool {
    unsafe {
        let m = MODULE;
        if m.is_null() || !(*m).fx.waves || expl == 0 {
            return false;
        }
        let odf = eng::prj_odf(field(field(expl, 0x30), 0x24));
        match (*m).fx.recipe_of(odf, true) {
            Some(rc) => {
                (*m).fx.wave(v, expl, rc);
                true
            }
            None => false,
        }
    }
}
