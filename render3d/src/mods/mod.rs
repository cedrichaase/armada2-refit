//! Modules: each takes over one or more of the body's steps, or the drawing of some
//! objects in the object loop, from the engine. A step no module takes is the
//! engine's own (`frame::steps`).
//!
//! A module is switched per category in `Render3D.ini` (`[Modules]`, `<name>=1`), and a
//! module that replaces a plugin's work stands down while that plugin is loaded (its
//! `superseded()`), so the plugin keeps drawing its part inside the engine's step.
use crate::frame::steps::View;
use crate::log;

pub mod effects;
pub mod hulls;
pub mod ordnance;
pub mod particles;
pub mod overlays;
pub mod sky;
pub mod specials;
pub mod world;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// The steps of the body a module can take over (`body-map.md`, section 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// `Background_Render`: sky, stars, background objects, then the depth clear.
    Background,
    /// `Terrain_Geometry::Render`: footprints, tactical grid, fog, shroud.
    Terrain,
    /// The whole object loop (`Nebula::sCullOccludedNebula` and every `Render`).
    Objects,
    /// `Ordnance::RenderAll`.
    Ordnance,
    /// `SpecialObjectEffect::RenderAll`.
    Specials,
    /// `GameFeature_RenderAll`.
    Features,
    /// `RenderWarpSpriteEffects`.
    Warp,
    /// `RenderChunks`: debris.
    Chunks,
    /// `RenderParticleList`.
    Particles,
    /// Not an engine step: just before the device's `Flush` draws the sorted
    /// translucent triangles. Returning true does not skip the flush.
    Translucent,
    /// `RenderSelectionEffects`.
    Selection,
}

pub trait Module {
    fn name(&self) -> &'static str;
    /// The plugin whose work this module replaces; while it is loaded the module
    /// stands down.
    fn superseded(&self) -> Option<&'static str> {
        None
    }
    /// Draws only its part of the menu scene, switched off for missions
    /// (`hulls::MenuOnly`): not `running()`.
    fn menu_only(&self) -> bool {
        false
    }
    /// A device to draw on (first frame, or after a new device): every plugin is
    /// loaded by now.
    fn device(&mut self) {}
    /// Before a `Reset`. `gpu` lets go of its objects and makes them again itself; what
    /// a module does here is for what they held: a target's contents are lost (a bake to
    /// do again), and a texture or buffer made `Transient` cannot be made again (made
    /// anew by its owner).
    fn reset(&mut self) {}
    /// Before the engine lets go of its device (`frame::forget`): as `reset`, unless a
    /// module holds more.
    fn forget(&mut self) {
        self.reset();
    }
    /// Once per view, after `GameObject_PreRenderAll` and `Ordnance::PreRenderAll`
    /// registered the engine's lights: register any light of one's own with the engine
    /// (`ST3D_GraphicsEngine::RegisterLight`), so `scene::lights()` and the engine's own
    /// draws both see it. The engine deletes the records at the end of the view.
    fn lights(&mut self, _v: &View) {}
    /// Once per view, after the lights are registered and before any step.
    fn begin_view(&mut self, _v: &View) {}
    /// Draws a step in the engine's place; true if it did (the engine's is skipped).
    fn step(&mut self, _s: Step, _v: &View) -> bool {
        false
    }
    /// Draws an object in the object loop in the engine's place (its `Render`); true if
    /// it did. Called for every object that is not a background object and not culled.
    fn object(&mut self, _obj: usize, _v: &View) -> bool {
        false
    }
    /// Draws a game feature (`GameFeature_RenderAll`'s loop: explosions, shield hits,
    /// sparks, construction, ghosts, the build ghost...) in the engine's place; true if
    /// it did. Tell them apart by vtable.
    fn feature(&mut self, _f: usize, _v: &View) -> bool {
        false
    }
    /// Once per view, after the last flush, before the lights are deleted.
    fn end_view(&mut self, _v: &View) {}
    /// The menu scene (`crate::menu`): before its first pass that draws hulls (their
    /// poses, the shadow map).
    fn menu_begin(&mut self, _shot: &crate::menu::Shot, _v: &View) {}
    /// The menu scene: one pass of it, for the part this module draws.
    fn menu(&mut self, _pass: crate::menu::Pass, _shot: &crate::menu::Shot, _v: &View) {}
}

/// A module with `[Modules] <name>=0`, kept for the menu scene's part only (the sky and
/// the world; hulls has its own, `hulls::MenuOnly`). The front end has no game objects
/// and the engine draws nothing there, so nothing could take the part back. It forwards
/// the device, the resets and the menu scene's calls and nothing else: in a mission it
/// has no step, object, feature, light or view, and it is not `running()`, so that part
/// is the engine's, as with the module off.
pub struct MenuOnly<M: Module>(pub M);

impl<M: Module> Module for MenuOnly<M> {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn superseded(&self) -> Option<&'static str> {
        self.0.superseded()
    }
    fn menu_only(&self) -> bool {
        true
    }
    fn device(&mut self) {
        self.0.device();
    }
    fn reset(&mut self) {
        self.0.reset();
    }
    fn forget(&mut self) {
        self.0.forget();
    }
    fn menu_begin(&mut self, shot: &crate::menu::Shot, v: &View) {
        self.0.menu_begin(shot, v);
    }
    fn menu(&mut self, pass: crate::menu::Pass, shot: &crate::menu::Shot, v: &View) {
        self.0.menu(pass, shot, v);
    }
}

static mut MODS: Vec<Box<dyn Module>> = Vec::new();
static mut ACTIVE: bool = false;
/// The names of the modules running, from the first frame on.
static mut NAMES: Vec<&'static str> = Vec::new();

/// Whether a module runs (asked by another module, e.g. whether a class of nebula is
/// drawn as a volume). False before the first frame.
#[allow(static_mut_refs)]
pub fn running(name: &str) -> bool {
    unsafe { NAMES.iter().any(|n| *n == name) }
}

#[allow(static_mut_refs)]
fn all() -> &'static mut Vec<Box<dyn Module>> {
    unsafe { &mut MODS }
}

/// Builds the modules the configuration asks for. Called once at load.
pub fn init() {
    let ini = crate::config::ini();
    let mods = all();
    // `on`: whether the module runs when Render3D.ini does not say; a module that is
    // not finished yet is off until asked for.
    let mut add = |m: Box<dyn Module>, on: bool| {
        if m.menu_only() {
            log!("module {}: off for missions (Render3D.ini [Modules] {}=1 turns it on); loaded for the menu scene's part", m.name(), m.name());
            mods.push(m);
        } else if ini.int("Modules", m.name(), on as i32) != 0 {
            log!("module {}: loaded", m.name());
            mods.push(m);
        } else {
            log!("module {}: off (Render3D.ini [Modules] {}=1 turns it on)", m.name(), m.name());
        }
    };
    // One line per module; the first that takes a step or an object draws it.
    // particles before hulls: it takes the latinum nebulae, which draw through the
    // plain GameObject::Render as hulls do.
    // world first: it takes the nebulae (latinum included) and the rocks whose class has
    // a recipe, and leaves every other one to particles and hulls.
    // world=0 and sky=0 likewise keep the menu scene's planets, nebulae and sky.
    if ini.int("Modules", "world", 1) != 0 {
        add(Box::new(world::World::default()), true);
    } else {
        add(Box::new(MenuOnly(world::World::menu_scene())), true);
    }
    add(Box::new(particles::Particles::default()), true);
    // hulls=0: the menu scene is render3d's own and the engine draws nothing in the
    // front end, so its ships have no stock draw to fall back to and stay ours there.
    if ini.int("Modules", "hulls", 1) != 0 {
        add(Box::new(hulls::Hulls::default()), true);
    } else {
        add(Box::new(hulls::MenuOnly::new()), true);
    }
    add(Box::new(effects::Effects::new()), true);
    add(Box::new(ordnance::Ordnance::new()), true);
    add(Box::new(specials::Specials::default()), true);
    if ini.int("Modules", "sky", 1) != 0 {
        add(Box::new(sky::Sky::default()), true);
    } else {
        add(Box::new(MenuOnly(sky::Sky::default())), true);
    }
    add(Box::new(overlays::Overlays::default()), true);
}

/// At the first frame, when every plugin is loaded: drop the modules whose plugin is.
pub fn settle() {
    let mods = all();
    mods.retain(|m| match m.superseded() {
        Some(p) if crate::plugin_loaded(p) => {
            log!("module {}: {} is loaded, which draws this part; the module stands down", m.name(), p);
            false
        }
        _ => true,
    });
    for m in mods.iter() {
        if m.menu_only() {
            log!("module {}: on, for the menu scene only", m.name());
        } else {
            log!("module {}: on", m.name());
        }
    }
    unsafe { NAMES = mods.iter().filter(|m| !m.menu_only()).map(|m| m.name()).collect() };
    unsafe { ACTIVE = true };
}

pub fn device() {
    for m in all().iter_mut() {
        m.device();
    }
}

pub fn reset() {
    for m in all().iter_mut() {
        m.reset();
    }
}

pub fn forget() {
    for m in all().iter_mut() {
        m.forget();
    }
}

pub fn lights(v: &View) {
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        m.lights(v);
    }
}

pub fn begin_view(v: &View) {
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        m.begin_view(v);
    }
}

pub fn step(s: Step, v: &View) -> bool {
    let mut done = false;
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        if m.step(s, v) {
            done = true;
            if s != Step::Translucent {
                break;
            }
        }
    }
    done
}

pub fn object(o: usize, v: &View) -> bool {
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        if m.object(o, v) {
            return true;
        }
    }
    survey(o);
    false
}

/// The survey of what is left to the engine: each class of object no module draws,
/// logged once (its vtable, `Render`, `RenderInternal`, its instance's database's
/// vtable and its ODF).
fn survey(o: usize) {
    static mut SEEN: Vec<usize> = Vec::new();
    let vt = crate::engine::vtable(o);
    #[allow(static_mut_refs)]
    let seen = unsafe { &mut SEEN };
    if vt == 0 || seen.len() >= 256 || seen.contains(&vt) {
        return;
    }
    seen.push(vt);
    let f = crate::engine::field;
    let db = f(f(o, 4), 0x80);
    log!(
        "survey: left to the engine: vtable {:#x} (Render {:#x}, RenderInternal {:#x}), database vtable {:#x}, odf '{}'",
        vt, f(vt, 11 * 4), f(vt, 22 * 4), crate::engine::vtable(db), crate::engine::obj::odf_name(o)
    );
}

pub fn feature(f: usize, v: &View) -> bool {
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        if m.feature(f, v) {
            return true;
        }
    }
    false
}

pub fn menu_begin(shot: &crate::menu::Shot, v: &View) {
    for m in all().iter_mut() {
        m.menu_begin(shot, v);
    }
}

pub fn menu(pass: crate::menu::Pass, shot: &crate::menu::Shot, v: &View) {
    for m in all().iter_mut() {
        m.menu(pass, shot, v);
    }
}

pub fn end_view(v: &View) {
    for m in all().iter_mut() {
        let _z = crate::timing::zone(m.name());
        m.end_view(v);
    }
}

pub fn any() -> bool {
    unsafe { ACTIVE && !MODS.is_empty() }
}
