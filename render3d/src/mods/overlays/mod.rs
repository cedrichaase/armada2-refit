//! Overlays: what the engine draws over and about the map rather than as objects.
//!
//! - `Step::Terrain` (`Terrain_Geometry::Render` 0x593200): the build footprints, the
//!   tactical grid (`grid`), and the fog of war and the shroud with smooth edges
//!   (`fog`, the port of `Fog.asi`);
//! - `Step::Selection` (`RenderSelectionEffects` 0x599900): the selection bubbles
//!   (`selection`);
//! - the features `Ghost` (fog-of-war ghosts) and `PlaceholderFeature` (the build
//!   ghost) (`ghosts`).
//!
//! - the height posts (`DrawHeightPost`, `posts`), as anti-aliased lines.
//!
//! What it does not take stays the engine's: the shroud's `TreeGrid` path and the build
//! footprints' sprites are called through.
//! Nothing the simulation reads is written.
mod fog;
mod ghosts;
mod grid;
mod posts;
mod selection;

use super::{Module, Step};
use crate::d3d::Device;
use crate::engine::{self, field, rd};
use crate::frame::steps::View;
use crate::log;

// ---- Armada2.exe, GOG patch 1.1 ----------------------------------------------------

/// `g_terrain_geometry`: the `Terrain_Geometry` `Terrain_Geometry::Render` draws.
const G_TERRAIN: usize = 0x772140;
/// `Terrain_Geometry::m_initialized`: false until its first `Initialize`.
const TERRAIN_INITIALIZED: usize = 0x772180;
/// `View_Record` (`PresetView`): 0 the RTS camera, 1 the tactical one, 2 cineractive.
const VIEW_RECORD: usize = 0x76b610;
/// `UserProfile` (`GameConfiguration*`): `+0x84` the map editor is on, `+0x45` the
/// tactical shroud option, `+0x48` the selection bubbles' brightness.
const USER_PROFILE: usize = 0x76b5ac;
const G_TRANSPORT: usize = 0x76b8d4;
const FN_GET_GAME_SETUP: usize = 0x557940; // Transport::getGameSetup, thiscall
const FN_IS_FOG: usize = 0x5462f0; // GameSetup::isFog, thiscall
const FN_IS_SHROUD: usize = 0x546300; // GameSetup::isShroud, thiscall
const FN_GRID_VISIBLE: usize = 0x51e180; // GridVisible(), cdecl
const FN_DRAW_GRID: usize = 0x593480; // Terrain_Geometry::DrawGrid(cam), thiscall
const PLACEHOLDER: usize = 0x736d50; // placeholderFeature
const FN_FOOTPRINTS: usize = 0x473930; // PlaceholderFeature::RenderFootprints(cam, float), thiscall
const VT_GHOST: usize = 0x6aff8c;
const VT_PLACEHOLDER: usize = 0x6b005c;

pub(crate) unsafe fn this0_ret(f: usize, this: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> usize>(f)(this) }
}

fn game_setup_flag(f: usize) -> bool {
    unsafe {
        let t = rd::<usize>(G_TRANSPORT);
        if t == 0 {
            return false;
        }
        let gs = this0_ret(FN_GET_GAME_SETUP, t);
        gs != 0 && (core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(f)(gs)) != 0
    }
}

fn editor() -> bool {
    engine::field_u8(unsafe { rd::<usize>(USER_PROFILE) }, 0x84) != 0
}

fn view_record() -> u32 {
    unsafe { rd::<u32>(VIEW_RECORD) }
}

/// Settings from `Render3D.ini`, `[Overlays]`.
pub struct Settings {
    pub fog: bool,
    pub grid: bool,
    pub selection: bool,
    pub ghosts: bool,
    pub fog_layers: fog::Settings,
}

impl Settings {
    fn read() -> Settings {
        let ini = crate::config::ini();
        let on = |k: &str| ini.int("Overlays", k, 1) != 0;
        Settings {
            fog: on("Fog"),
            grid: on("Grid"),
            selection: on("Selection"),
            ghosts: on("Ghosts"),
            fog_layers: fog::Settings::read(&ini) }
    }
}

pub struct Overlays {
    s: Settings,
    fog: fog::Fog,
    grid: grid::Grid,
    sel: selection::Selection,
    ghosts: ghosts::Ghosts,
    posts: posts::Posts,
}

impl Default for Overlays {
    fn default() -> Overlays {
        let s = Settings::read();
        log!(
            "overlays: fog {} (filter {}), grid {}, selection {}, ghosts {}",
            s.fog as u8,
            if s.fog_layers.catrom { "catmull-rom" } else { "bspline" },
            s.grid as u8,
            s.selection as u8,
            s.ghosts as u8
        );
        let posts = posts::Posts::new(&crate::config::ini());
        Overlays { s, fog: fog::Fog::new(), grid: grid::Grid::new(), sel: selection::Selection::new(), ghosts: ghosts::Ghosts::new(), posts }
    }
}

impl Overlays {
    /// `Terrain_Geometry::Render`, as stock runs it, with the grid and the two layers
    /// ours. False to leave the whole step to the engine.
    fn terrain(&mut self, v: &View) -> bool {
        let tg = G_TERRAIN;
        if unsafe { rd::<u8>(TERRAIN_INITIALIZED) } == 0 {
            // The engine's first call makes its meshes and finds its textures.
            return false;
        }
        let ed = editor();
        let vr = view_record();
        let shroud_on = !ed && game_setup_flag(FN_IS_SHROUD);
        // In the tactical view with the profile's shroud option on, stock draws the
        // shroud as a `TreeGrid` (`+0x28`): not seen on the bench, left to the engine.
        if shroud_on && vr == 1 {
            let opt = engine::field_u8(unsafe { rd::<usize>(USER_PROFILE) }, 0x45);
            if (opt == 1 || opt == 2) && field(tg, 0x28) != 0 {
                return false;
            }
        }
        let shroud = shroud_on && vr == 0;
        let fog = !ed && vr == 0 && game_setup_flag(FN_IS_FOG);
        // 1: the build footprints, the engine's sprites.
        unsafe {
            let f: unsafe extern "thiscall" fn(usize, usize, f32) = core::mem::transmute(FN_FOOTPRINTS);
            f(PLACEHOLDER, v.cam, engine::field_f32(tg, 0x1c));
        }
        // 2: the tactical grid.
        if unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(FN_GRID_VISIBLE)() } != 0 {
            if self.s.grid {
                self.grid.draw(v, tg);
            } else {
                unsafe {
                    let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(FN_DRAW_GRID);
                    f(tg, v.cam);
                }
            }
        }
        // 3, 4: the fog of war, then the shroud over it.
        if fog {
            self.fog.draw(v, tg, fog::Layer::Fog, &self.s.fog_layers);
        }
        if shroud {
            self.fog.draw(v, tg, fog::Layer::Shroud, &self.s.fog_layers);
        }
        true
    }
}

impl Module for Overlays {
    fn name(&self) -> &'static str {
        "overlays"
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Fog.asi")
    }
    fn device(&mut self, _dev: Device) {
        self.posts.install();
    }
    fn reset(&mut self) {
        self.grid.reset();
        self.fog.reset();
        self.posts.reset();
    }
    fn begin_view(&mut self, _v: &View) {
        self.ghosts.begin_view();
        self.posts.begin_view();
    }
    fn end_view(&mut self, _v: &View) {
        self.posts.end_view();
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        match s {
            Step::Terrain if self.s.fog => self.terrain(v),
            Step::Selection if self.s.selection => self.sel.draw(v),
            Step::Translucent => {
                self.posts.draw(v);
                false
            }
            _ => false,
        }
    }
    fn feature(&mut self, f: usize, v: &View) -> bool {
        if !self.s.ghosts {
            return false;
        }
        match engine::vtable(f) {
            VT_GHOST => self.ghosts.ghosts(f, v),
            VT_PLACEHOLDER => self.ghosts.placeholder(f, v),
            _ => false,
        }
    }
}

/// A height post from `top` to the grid's plane, drawn with this view's posts; the
/// engine's `DrawHeightPost` when posts are not ours.
pub fn height_post(v: &View, top: crate::math::V3) {
    if !posts::note(top) {
        unsafe {
            let f: unsafe extern "C" fn(*const crate::math::V3, usize) = core::mem::transmute(0x593840usize);
            f(&top, v.cam);
        }
    }
}
