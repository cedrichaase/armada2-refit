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
//! - the height posts (`DrawHeightPost`, `posts`), as anti-aliased lines;
//! - the tactical camera's pivot marker (the feature `TacticalCamera`, `pivot`);
//! - the move paths, the sketched one and each ship's until flown (`paths`).
//!
//! What it does not take stays the engine's: the shroud's `TreeGrid` path and the build
//! footprints' sprites are called through.
//! Nothing the simulation reads is written.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The runtime and its macros, as `crate::rt`, `crate::log` and `crate::sfmt` (r3d-core's).
#[allow(unused_imports)]
use r3d_core::{dump, log, rt, sfmt};

#[rustfmt::skip]
pub mod shaders;
mod content;
mod fog;
pub mod hooks;
pub mod settings_table;
mod ghosts;
mod grid;
mod paths;
mod pivot;
mod posts;
mod selection;
mod syms;

use r3d_core::module::{Module, Step};
use r3d_core::engine;
use r3d_core::frame::steps::View;
use crate::rt::settings;
use crate::rt::Ini;
use settings_table::{OVERLAYS_FOG, OVERLAYS_GHOSTS, OVERLAYS_GRID, OVERLAYS_HEIGHTPOSTS, OVERLAYS_MOVEPATHS, OVERLAYS_PIVOT, OVERLAYS_SELECTION};

/// Settings from `Render3D.ini`, `[Overlays]`.
pub struct Settings {
    pub fog: bool,
    pub grid: bool,
    pub selection: bool,
    pub ghosts: bool,
    pub fog_layers: fog::Settings,
}

impl Settings {
    fn read(ini: &Ini) -> Settings {
        Settings {
            fog: settings::on(ini, OVERLAYS_FOG),
            grid: settings::on(ini, OVERLAYS_GRID),
            selection: settings::on(ini, OVERLAYS_SELECTION),
            ghosts: settings::on(ini, OVERLAYS_GHOSTS),
            fog_layers: fog::Settings::read(ini) }
    }
}

pub struct Overlays {
    s: Settings,
    fog: fog::Fog,
    grid: grid::Grid,
    sel: selection::Selection,
    ghosts: ghosts::Ghosts,
    posts: posts::Posts,
    pivot: pivot::Pivot,
    paths: paths::Paths,
}

/// `[Overlays] Fog` and `Selection` of the module made (`mods::claims`).
static mut CLAIMS: (bool, bool) = (false, false);

pub fn claims() -> (bool, bool) {
    unsafe { CLAIMS }
}

/// `Render3D.ini` as it is now, for the module's construction.
fn open_ini() -> Ini {
    Ini::open(&crate::rt::game_path("Render3D.ini"))
}

impl Default for Overlays {
    fn default() -> Overlays {
        let ini = open_ini();
        let s = Settings::read(&ini);
        unsafe { CLAIMS = (s.fog, s.selection) };
        log!(
            "overlays: fog {} (filter {}), grid {}, selection {}, ghosts {}",
            s.fog as u8,
            if s.fog_layers.catrom { "catmull-rom" } else { "bspline" },
            s.grid as u8,
            s.selection as u8,
            s.ghosts as u8
        );
        let posts = posts::Posts::new(&ini);
        let pivot = pivot::Pivot::new(&ini);
        log!("overlays: pivot marker {}", pivot.on as u8);
        let paths = paths::Paths::new(&ini);
        Overlays { s, fog: fog::Fog::new(), grid: grid::Grid::new(), sel: selection::Selection::new(), ghosts: ghosts::Ghosts::new(), posts, pivot, paths }
    }
}

impl Overlays {
    /// `Terrain_Geometry::Render`, as stock runs it, with the grid and the two layers
    /// ours. False to leave the whole step to the engine.
    fn terrain(&mut self, v: &View) -> bool {
        let tg = hooks::g_terrain();
        if !hooks::terrain_initialized() {
            // The engine's first call makes its meshes and finds its textures.
            return false;
        }
        let ed = hooks::editor();
        let vr = hooks::view_record();
        let shroud_on = !ed && hooks::is_shroud();
        // In the tactical view with the profile's shroud option on, stock draws the
        // shroud as a `TreeGrid` (`+0x28`): not seen on the bench, left to the engine.
        if shroud_on && vr == 1 {
            let opt = hooks::tactical_shroud_option();
            if (opt == 1 || opt == 2) && hooks::shroud_tree_grid(tg) {
                return false;
            }
        }
        let shroud = shroud_on && vr == 0;
        let fog = !ed && vr == 0 && hooks::is_fog();
        // 1: the build footprints, the engine's sprites.
        hooks::render_footprints(v.cam, tg);
        // 2: the tactical grid.
        if hooks::grid_visible() {
            if self.s.grid {
                self.grid.draw(v, tg);
            } else {
                hooks::draw_engine_grid(tg, v.cam);
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
    fn provide(&mut self, id: r3d_core::module::ModuleId, s: &mut r3d_core::services::Services) {
        s.post(id, &PROVIDER);
    }
    fn name(&self) -> &'static str {
        "overlays"
    }
    fn priority(&self) -> i32 {
        80
    }
    fn claims(&self) -> u32 {
        // [Overlays] Fog and Selection, as they are now.
        let (fog, sel) = crate::claims();
        (if fog { gpu::display::CLAIM_FOG } else { 0 }) | (if sel { gpu::display::CLAIM_SELECTION } else { 0 })
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &settings_table::ROWS
    }
    /// `[Overlays]`, at load and at every settings change: every key with a row is read
    /// again (the fog's filter and thresholds with them), and the claims follow.
    fn configure(&mut self, ini: &Ini, said: &mut r3d_core::module::Changes) {
        let mut note = |k: &str, v: bool| said.note(&crate::sfmt!("Overlays.{}={}", k, v as u32));
        let new = Settings::read(ini);
        if new.fog != self.s.fog {
            note("Fog", new.fog);
        }
        if new.grid != self.s.grid {
            note("Grid", new.grid);
        }
        if new.selection != self.s.selection {
            note("Selection", new.selection);
        }
        if new.ghosts != self.s.ghosts {
            note("Ghosts", new.ghosts);
        }
        self.s = new;
        unsafe { CLAIMS = (self.s.fog, self.s.selection) };
        let hp = settings::on(ini, OVERLAYS_HEIGHTPOSTS);
        if self.posts.set_on(hp) {
            note("HeightPosts", hp);
        }
        let pv = settings::on(ini, OVERLAYS_PIVOT);
        if self.pivot.set_on(pv) {
            note("Pivot", pv);
        }
        let mp = settings::on(ini, OVERLAYS_MOVEPATHS);
        if self.paths.set_on(mp) {
            note("MovePaths", mp);
        }
    }
    /// The height posts' five calls and stock's sketch line, at the first enable;
    /// gated by the module's flag.
    fn hooks(&mut self, h: &mut r3d_core::hooks::Hooks) {
        if !h.ready(&syms::TABLE, syms::hook::OVERLAYS_READS) {
            h.stand_down("the engine's reads are not stock");
            return;
        }
        self.posts.install(h);
        self.paths.install(h);
    }
    /// Switched off live: the posts noted in a view are dropped; the hook hands its
    /// calls to the engine through its gate.
    fn off(&mut self) {
        self.posts.end_view();
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Fog.asi")
    }
    fn reset(&mut self) {
        self.grid.reset();
        self.fog.reset();
        self.posts.reset();
        self.paths.reset();
        // The queued ghosts hold their models, and the models their buffers, which are
        // in the default pool.
        self.ghosts.begin_view();
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
                self.paths.draw_all(v);
                false
            }
            _ => false,
        }
    }
    fn feature(&mut self, f: usize, v: &View) -> bool {
        let vt = engine::vtable(f);
        if vt == hooks::vt_tactical_camera() {
            return self.pivot.feature(f, v);
        }
        if !self.s.ghosts {
            return false;
        }
        match vt {
            x if x != 0 && x == hooks::vt_ghost() => self.ghosts.ghosts(f, v),
            x if x != 0 && x == hooks::vt_placeholder() => self.ghosts.placeholder(f, v),
            _ => false,
        }
    }
}

/// A height post from `top` to the grid's plane, drawn with this view's posts; the
/// engine's `DrawHeightPost` when posts are not ours.
fn height_post(v: &View, top: r3d_core::math::V3) {
    if !posts::note(top) {
        engine::instance::draw_height_post(top, v.cam);
    }
}

/// The overlays module's draws, offered to the others (`services`).
struct Provider;
static PROVIDER: Provider = Provider;

impl r3d_core::services::PostDraw for Provider {
    fn height_post(&self, v: &View, top: r3d_core::math::V3) {
        height_post(v, top)
    }
}
