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
    /// Once per view, after the last flush, before the lights are deleted.
    fn end_view(&mut self, _v: &View) {}
}

static mut MODS: Vec<Box<dyn Module>> = Vec::new();
static mut ACTIVE: bool = false;

#[allow(static_mut_refs)]
fn all() -> &'static mut Vec<Box<dyn Module>> {
    unsafe { &mut MODS }
}

/// Builds the modules the configuration asks for. Called once at load.
pub fn init() {
    let ini = crate::config::ini();
    let mods = all();
    let mut add = |m: Box<dyn Module>| {
        if ini.int("Modules", m.name(), 1) != 0 {
            mods.push(m);
        } else {
            log!("module {}: off in Render3D.ini", m.name());
        }
    };
    let _ = &mut add;
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

pub fn end_view(v: &View) {
    for m in all().iter_mut() {
        m.end_view(v);
    }
}

pub fn any() -> bool {
    unsafe { ACTIVE && !MODS.is_empty() }
}
