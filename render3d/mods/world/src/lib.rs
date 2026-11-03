//! The world: planets, moons and asteroid fields, nebulae, drawn by render3d in place of
//! the engine's objects that override `Render` (`body-map.md` §2b) or draw a database of
//! their own. Ports `Surfaces.asi` (surfaces from recipes), `Lighting.asi`'s planets,
//! `Planets.asi` (a planet's tessellation, now our own sphere) and `Nebulae.asi`.
//!
//! Each part stands down on its own while the plugin that draws it is loaded, so an
//! install that still has the plugin keeps its look:
//! - planets, while `Lighting.asi` is loaded (it draws them through its `GroundMesh`
//!   hook);
//! - moons and asteroids, while `Lighting.asi` or `Surfaces.asi` is;
//! - nebulae, while `Nebulae.asi` is.
//!
//! Settings: `[World]` (and `[Nebulae]`) in Render3D.ini, read again every second with
//! `[World] Reload=1`; `[World] Enable=0` hands everything back to the engine at once,
//! for comparing with stock at the same moment on the bench.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The runtime and its macros, as `crate::rt`, `crate::log` and `crate::sfmt` (r3d-core's).
#[allow(unused_imports)]
use r3d_core::{dump, log, rt, sfmt};

#[rustfmt::skip]
pub mod shaders;
pub mod content;
pub mod hooks;
pub mod lights;
pub mod settings_table;
pub mod nebulae;
pub mod planets;
pub mod recipes;
pub mod rocks;
pub mod surfaces;

/// `Render3D.ini` as it is now, for `[World] Reload=1` (once a second, between settings
/// changes).
fn open_ini() -> Ini {
    Ini::open(&crate::rt::game_path("Render3D.ini"))
}

use r3d_core::module::{Module, Step};
use r3d_core::engine;
use r3d_core::frame::steps::View;
use crate::rt::settings;
use crate::rt::Ini;
use settings_table::{WORLD_NEBULAE, WORLD_PLANETS, WORLD_ROCKS, WORLD_SURFACES};

pub struct World {
    surfaces: surfaces::Surfaces,
    planets: planets::Planets,
    rocks: rocks::Rocks,
    nebulae: nebulae::Nebulae,
    lights: lights::Lights,
    on_planets: bool,
    on_rocks: bool,
    on_nebulae: bool,
    /// An old plugin draws that part (`settle`): the row cannot take it back.
    stand_planets: bool,
    stand_rocks: bool,
    stand_nebulae: bool,
    settled: bool,
    enable: bool,
    reload: bool,
    checked: f64,
    /// An old plugin draws the planets / the gas: the menu scene's part stands down
    /// too (`[World]` keys at 0 do not).
    plug_planets: bool,
    plug_nebulae: bool,
    /// Its place in the registry (set by `provide`), for asking about its own flag.
    id: r3d_core::module::ModuleId,
}

/// The module, for the overlays' ghosts (set at every view).
static mut ME: *mut World = core::ptr::null_mut();

/// A planet's fog-of-war ghost (`overlays::ghosts`), drawn on our sphere like the
/// planet itself; false if it is not a planet's or planets are the engine's.
fn planet_ghost(sp: usize, v: &View) -> bool {
    let me = unsafe { ME };
    if me.is_null() {
        return false;
    }
    let me = unsafe { &mut *me };
    if !(me.enable && me.on_planets) {
        return false;
    }
    me.planets.draw_ghost(sp, v, &mut me.surfaces)
}

impl Default for World {
    fn default() -> World {
        let ini = open_ini();
        World {
            surfaces: surfaces::Surfaces::new(&ini),
            planets: planets::Planets::new(&ini),
            rocks: rocks::Rocks::new(&ini),
            nebulae: nebulae::Nebulae::new(&ini),
            lights: lights::Lights::new(),
            on_planets: settings::on(&ini, WORLD_PLANETS),
            on_rocks: settings::on(&ini, WORLD_ROCKS),
            on_nebulae: settings::on(&ini, WORLD_NEBULAE),
            stand_planets: false,
            stand_rocks: false,
            stand_nebulae: false,
            settled: false,
            enable: ini.int("World", "Enable", 1) != 0,
            reload: ini.int("World", "Reload", 1) != 0,
            checked: 0.0,
            plug_planets: false,
            plug_nebulae: false,
            id: r3d_core::module::ModuleId(0),
        }
    }
}

impl World {
    /// At the first device, when every plugin is loaded: each part stands down beside
    /// the plugin that draws it.
    fn settle(&mut self) {
        if self.settled {
            return;
        }
        self.settled = true;
        // `[World] BesidePlugins=1` (bench only): draw beside the plugins anyway, so that
        // `Enable=0`/`1` flips between the plugins' look and ours at the same moment.
        let ini = open_ini();
        let beside = ini.int("World", "BesidePlugins", 0) != 0;
        let lighting = !beside && r3d_core::plugin_loaded("Lighting.asi");
        let surfaces = !beside && r3d_core::plugin_loaded("Surfaces.asi");
        self.plug_planets = lighting;
        self.plug_nebulae = !beside && r3d_core::plugin_loaded("Nebulae.asi");
        self.stand_planets = lighting;
        self.stand_rocks = lighting || surfaces;
        self.stand_nebulae = self.plug_nebulae;
        if self.on_planets && lighting {
            log!("world: Lighting.asi is loaded, which draws the planets: planets stay its");
        }
        if self.on_rocks && (lighting || surfaces) {
            log!("world: Lighting.asi or Surfaces.asi is loaded: moons and asteroids stay the engine's");
        }
        if self.on_nebulae && self.stand_nebulae {
            log!("world: Nebulae.asi is loaded, which draws the gas: nebulae stay its");
        }
        self.rows(&ini);
        unsafe { ME = self as *mut World };
        self.parts();
        log!(
            "world: planets {}, moons and asteroids {}, nebulae {}",
            on(self.on_planets), on(self.on_rocks), on(self.on_nebulae)
        );
    }

    /// The three part rows, read again: a part is ours when its row is on and no old
    /// plugin has it. Returns the notes of what changed.
    fn rows(&mut self, ini: &Ini) -> alloc::vec::Vec<&'static str> {
        let mut changed = alloc::vec::Vec::new();
        let (p, r, n) = (
            settings::on(ini, WORLD_PLANETS) && !self.stand_planets,
            settings::on(ini, WORLD_ROCKS) && !self.stand_rocks,
            settings::on(ini, WORLD_NEBULAE) && !self.stand_nebulae,
        );
        if p != self.on_planets {
            self.on_planets = p;
            changed.push(if p { "World.Planets=1" } else { "World.Planets=0" });
        }
        if r != self.on_rocks {
            self.on_rocks = r;
            changed.push(if r { "World.Rocks=1" } else { "World.Rocks=0" });
        }
        if n != self.on_nebulae {
            self.on_nebulae = n;
            changed.push(if n { "World.Nebulae=1" } else { "World.Nebulae=0" });
        }
        changed
    }

    /// What follows from the parts and `Enable`: the planets' glows, and whether the
    /// nebulae's ghost hook hides a fogged nebula's billboards.
    fn parts(&mut self) {
        self.lights.set_planets(self.on_planets);
        self.nebulae.set_active(self.on_nebulae && self.enable);
    }
}

/// `a` then `b` then the digit `d` (if any): a texture or key name. (Not `format!`: the
/// precompiled `alloc::fmt::format` needs the C++ unwinder, which this crate does not
/// link.)
pub fn name(a: &str, b: &str, d: Option<usize>) -> alloc::string::String {
    let mut s = alloc::string::String::from(a);
    s.push_str(b);
    if let Some(d) = d {
        s.push((b'0' + (d % 10) as u8) as char);
    }
    s
}

fn on(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

/// The world module's draws, offered to the others (`services`).
struct Provider;
static PROVIDER: Provider = Provider;

impl r3d_core::services::GhostDraw for Provider {
    fn planet_ghost(&self, sp: usize, v: &View) -> bool {
        planet_ghost(sp, v)
    }
}

impl Module for World {
    fn name(&self) -> &'static str {
        "world"
    }
    fn priority(&self) -> i32 {
        10
    }
    fn claims(&self) -> u32 {
        gpu::display::CLAIM_WORLD
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &settings_table::ROWS
    }
    /// `[World]` and `[Nebulae]`, at load and at every settings change: the rows
    /// `Surfaces`, `Planets`, `Rocks`, `Nebulae` and `Nebulae.Lit` are honoured at the
    /// next view, and the keys without a row are read again with them.
    fn configure(&mut self, ini: &Ini, said: &mut r3d_core::module::Changes) {
        lights::Lights::read(ini);
        self.enable = ini.int("World", "Enable", 1) != 0;
        self.reload = ini.int("World", "Reload", 1) != 0;
        let sf = settings::on(ini, WORLD_SURFACES);
        if sf != self.surfaces.enable {
            self.surfaces.enable = sf;
            said.note(&crate::sfmt!("World.Surfaces={}", sf as u32));
        }
        if let Some(l) = self.nebulae.reread(ini) {
            said.note(&crate::sfmt!("Nebulae.Lit={}", l as u32));
        }
        for part in self.rows(ini) {
            said.note(part);
        }
        self.planets.configure(ini);
        self.rocks.reload(ini);
        self.parts();
    }
    /// Its hooks, at the first enable: the nebulae's lights and cull (unless
    /// `Lighting.asi` has them) and the ghosts' `Render` (unless `Nebulae.asi` draws the
    /// gas). Gated by the module's flag; shut, each hands the call to what it replaced.
    fn hooks(&mut self, h: &mut r3d_core::hooks::Hooks) {
        let beside = open_ini().int("World", "BesidePlugins", 0) != 0;
        let lights = !r3d_core::plugin_loaded("Lighting.asi");
        if !lights {
            log!("world: lights: Lighting.asi is loaded: the nebulae's and planets' lights are its");
        }
        let spirit = beside || !r3d_core::plugin_loaded("Nebulae.asi");
        hooks::install(h, lights, spirit);
    }
    fn on(&mut self) {
        unsafe { ME = self as *mut World };
        self.parts();
    }
    /// Switched off live: the engine's own code takes the hooks back through their
    /// gate; the ghosts' hiding stops, and the planets' glows with it.
    fn off(&mut self) {
        self.nebulae.set_active(false);
        self.lights.set_planets(false);
    }
    fn provide(&mut self, id: r3d_core::module::ModuleId, s: &mut r3d_core::services::Services) {
        self.id = id;
        s.ghost(id, &PROVIDER);
        s.volumes(id);
    }
    /// `[Modules] world=0` keeps the menu scene's planet and gas.
    fn in_menu(&self) -> bool {
        true
    }
    fn device(&mut self) {
        self.settle();
    }
    fn reset(&mut self) {
        self.surfaces.reset();
        self.rocks.reset();
        self.nebulae.reset();
    }
    fn lights(&mut self, v: &View) {
        if self.enable {
            let planets = &mut self.planets;
            self.lights.view(v, |o| planets.glow_colour(o));
        }
    }
    fn begin_view(&mut self, v: &View) {
        unsafe { ME = self as *mut World };
        if v.main && self.reload {
            let now = crate::rt::now();
            if now - self.checked > 1.0 {
                self.checked = now;
                let ini = open_ini();
                let e = ini.int("World", "Enable", 1) != 0;
                if e != self.enable {
                    self.enable = e;
                    log!("world: Enable={}", e as u32);
                }
                let s = settings::on(&ini, WORLD_SURFACES);
                if s != self.surfaces.enable {
                    self.surfaces.enable = s;
                    log!("world: Surfaces={}", s as u32);
                }
                if self.on_rocks {
                    self.rocks.reload(&ini);
                }
                if self.on_nebulae {
                    self.nebulae.reload(&ini);
                }
                self.nebulae.set_active(self.on_nebulae && self.enable);
            }
        }
    }
    fn object(&mut self, o: usize, v: &View) -> bool {
        if !self.enable {
            return false;
        }
        let ours = match engine::vtable(o) {
            hooks::VT_PLANET if self.on_planets => self.planets.draw(o, v, &mut self.surfaces),
            hooks::VT_SCRAP if self.on_rocks => self.rocks.draw_scrap(o, v, &mut self.surfaces),
            hooks::VT_ASTEROID_FIELD if self.on_rocks => self.rocks.draw_field(o, v, &mut self.surfaces),
            hooks::VT_NEBULA | hooks::VT_LATINUM if self.on_nebulae => self.nebulae.object(o, v),
            _ => false,
        };
        if ours {
            mark_on_screen(o, v);
        }
        ours
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s == Step::Translucent && self.enable {
            if self.on_rocks {
                self.rocks.translucent(v);
            }
            if self.on_nebulae {
                self.nebulae.draw(v);
            }
        }
        false
    }
    fn menu(&mut self, pass: r3d_core::menu::Pass, shot: &r3d_core::menu::Shot, v: &View) {
        // The menu scene ignores `[World] Enable`, `Planets` and `Nebulae`: they are a
        // mission's, and the engine draws nothing in the front end to take it back.
        match pass {
            r3d_core::menu::Pass::Planet if !self.plug_planets => {
                if let Some(p) = &shot.planet {
                    self.planets.draw_menu(v, &mut self.surfaces, &p.ground, &p.atmo, &p.world, p.radius, p.cloud_yaw);
                }
            }
            r3d_core::menu::Pass::Gas if !self.plug_nebulae => {
                let key = shot.key.map(|k| [k.x, k.y, k.z]);
                self.nebulae.draw_menu(v, &shot.gas, key);
            }
            _ => {}
        }
    }
    fn end_view(&mut self, v: &View) {
        if v.main {
            self.planets.end_frame(v.frame);
        }
    }
}

/// What `Entity::Render` (0x4cfda0) writes as the engine draws an object: its on-screen
/// byte (+0x25), set when the instance's frustum test (its vtable slot 3) passes, and
/// cleared for every object each frame by `Entity::ResetVisibility`. The pointer's
/// pick (`DisplayInterface::s_PrecomputeMouseOver`) passes over any object whose byte is
/// clear, so an object drawn here in the engine's place must have it written, or it
/// cannot be clicked: planets could be selected only by a drag box and not
/// right-clicked to colonise. As `GameObject::Render`, nothing for an object that is
/// going or that the player cannot see.
fn mark_on_screen(o: usize, v: &View) {
    use r3d_core::engine::obj;
    if obj::expired(o) || !obj::can_user_see(o) {
        return;
    }
    if r3d_core::scene::instance::frustum_test(obj::instance(o), v.cam) {
        r3d_core::scene::instance::set_on_screen(o);
    }
}
