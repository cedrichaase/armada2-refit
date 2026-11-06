//! The module interface, version 2 (README.md, "Module interface"): what a module of
//! render3d is to the registry (`core/src/registry.rs`) and to the frame. A module takes over
//! one or more of the body's steps, or the drawing of some objects in the object loop,
//! from the engine; a step no module takes is the engine's own (`frame::steps`).
//!
//! Lives in `r3d-core`; each module is a crate under `mods/` that depends on this one
//! alone. Nothing here knows a module by name.
use crate::frame::steps::View;
use crate::rt::Ini;
use crate::rt::settings::Setting;
use alloc::string::String;

/// A module's index in the registry, stable within a run (a module that stands down
/// keeps its index).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModuleId(pub u8);

impl ModuleId {
    /// The renderer core's own hooks (the frame's), which no module owns.
    pub const CORE: ModuleId = ModuleId(u8::MAX);
}

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

/// What `configure` changed, for `Render3D_SettingsChanged`'s one log line
/// (`settings: applied <a>, <b>`): each part as `Section.Key=value`.
#[derive(Default)]
pub struct Changes(pub String);

impl Changes {
    pub fn note(&mut self, part: &str) {
        if part.is_empty() {
            return;
        }
        if !self.0.is_empty() {
            self.0.push_str(", ");
        }
        self.0.push_str(part);
    }
}

pub trait Module {
    /// Its `[Modules]` key and log prefix: "hulls".
    fn name(&self) -> &'static str;
    /// Dispatch order, ascending (README.md, "Module interface"); two modules never
    /// share one (asserted at `init`).
    fn priority(&self) -> i32;
    /// The `gpu::display::CLAIM_*` this module draws while it runs (asked every frame,
    /// only while it is running).
    fn claims(&self) -> u32;
    /// Its settings rows (its `settings.toml` fragment's `ROWS`), copied into
    /// `Render3D_Settings`' table after the core's, in priority order; `&[]` if none.
    fn rows(&self) -> &'static [Setting];
    /// Reads every section it owns (README.md, "Modules and settings"): at load, and
    /// again at every `Render3D_SettingsChanged`, whether it runs or not. What changed
    /// goes into `said` (`Section.Key=value`) for the log line.
    fn configure(&mut self, ini: &Ini, said: &mut Changes);
    /// The plugin whose work this module replaces; while it is loaded the module
    /// stands down.
    fn superseded(&self) -> Option<&'static str> {
        None
    }
    /// Has a part in the menu scene (the front end has no game objects and the engine
    /// draws nothing there, so nothing could take that part back): it gets its device,
    /// resets and menu calls even while its `[Modules]` flag is 0. In a mission it does
    /// nothing then, and is not `running()`.
    fn in_menu(&self) -> bool {
        false
    }
    /// Registers what it provides to other modules (`services`), once, at init,
    /// under its own id (`s.hulls(id, ..)`).
    fn provide(&mut self, _id: ModuleId, _s: &mut crate::services::Services) {}
    /// Installs its hooks through the registry (`hooks`), once, when it is first
    /// on (at load, or at its first live switch-on: off at load installs nothing). A
    /// hook it cannot draw without that does not install: `h.stand_down(..)`, and the
    /// module is not switched on.
    fn hooks(&mut self, _h: &mut crate::hooks::Hooks) {}
    /// Switched on, at load or live: after `configure` and `hooks()`.
    fn on(&mut self) {}
    /// Switched off live (on the game thread, in `Render3D_SettingsChanged`): drop its
    /// cosmetic simulation state. `forget()` follows at the next view.
    fn off(&mut self) {}
    /// A view of a mission while the module is off but `in_menu`: drop what the menu
    /// scene left that the mission's views must not see.
    fn idle_view(&mut self, _v: &View) {}
    /// A device to draw on (first frame, or after a new device): every plugin is
    /// loaded by now. A module switched on later than its device gets one at its next
    /// view.
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
    /// The scene target's format or samples changed (MSAA or HDR switched live): what
    /// it made for the old ones goes. As `reset`, unless a module holds more.
    fn formats_changed(&mut self) {
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
