//! Modules: each takes over one or more of the body's steps, or the drawing of some
//! objects in the object loop, from the engine. A step no module takes is the
//! engine's own (`frame::steps`).
//!
//! A module is switched per category in `Render3D.ini` (`[Modules]`, `<name>=1`), and a
//! module that replaces a plugin's work stands down while that plugin is loaded (its
//! `superseded()`), so the plugin keeps drawing its part inside the engine's step.
use crate::d3d::Device;
use crate::frame::steps::View;
use crate::log;

pub mod effects;
pub mod hulls;
pub mod ordnance;
pub mod particles;
pub mod overlays;
pub mod sky;
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
    /// A device to draw on (first frame, or after a new device).
    fn device(&mut self, _dev: Device) {}
    /// Before a `Reset`: drop everything in the default pool.
    fn reset(&mut self) {}
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
        if ini.int("Modules", m.name(), on as i32) != 0 {
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
    add(Box::new(world::World::default()), true);
    add(Box::new(particles::Particles::default()), true);
    add(Box::new(hulls::Hulls::default()), true);
    add(Box::new(effects::Effects::new()), true);
    add(Box::new(ordnance::Ordnance::new()), true);
    add(Box::new(sky::Sky::default()), true);
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
        log!("module {}: on", m.name());
    }
    unsafe { NAMES = mods.iter().map(|m| m.name()).collect() };
    unsafe { ACTIVE = true };
}

pub fn device(dev: Device) {
    for m in all().iter_mut() {
        m.device(dev);
    }
}

pub fn reset() {
    for m in all().iter_mut() {
        m.reset();
    }
}

pub fn lights(v: &View) {
    for m in all().iter_mut() {
        m.lights(v);
    }
}

pub fn begin_view(v: &View) {
    for m in all().iter_mut() {
        m.begin_view(v);
    }
}

pub fn step(s: Step, v: &View) -> bool {
    let mut done = false;
    for m in all().iter_mut() {
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
        if m.object(o, v) {
            return true;
        }
    }
    false
}

pub fn feature(f: usize, v: &View) -> bool {
    for m in all().iter_mut() {
        if m.feature(f, v) {
            return true;
        }
    }
    false
}

pub fn end_view(v: &View) {
    for m in all().iter_mut() {
        m.end_view(v);
    }
}

pub fn any() -> bool {
    unsafe { ACTIVE && !MODS.is_empty() }
}
