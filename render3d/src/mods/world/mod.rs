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
pub mod lights;
pub mod nebulae;
pub mod planets;
pub mod rocks;
pub mod surfaces;

use super::{Module, Step};
use crate::d3d::Device;
use crate::engine;
use crate::frame::steps::View;
use crate::log;

pub struct World {
    surfaces: surfaces::Surfaces,
    planets: planets::Planets,
    rocks: rocks::Rocks,
    nebulae: nebulae::Nebulae,
    lights: lights::Lights,
    on_planets: bool,
    on_rocks: bool,
    on_nebulae: bool,
    settled: bool,
    enable: bool,
    reload: bool,
    checked: f64,
}

impl Default for World {
    fn default() -> World {
        let ini = crate::config::ini();
        World {
            surfaces: surfaces::Surfaces::new(),
            planets: planets::Planets::new(),
            rocks: rocks::Rocks::new(),
            nebulae: nebulae::Nebulae::new(),
            lights: lights::Lights::new(),
            on_planets: ini.int("World", "Planets", 1) != 0,
            on_rocks: ini.int("World", "Rocks", 1) != 0,
            on_nebulae: ini.int("World", "Nebulae", 1) != 0,
            settled: false,
            enable: ini.int("World", "Enable", 1) != 0,
            reload: ini.int("World", "Reload", 1) != 0,
            checked: 0.0,
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
        let beside = crate::config::ini().int("World", "BesidePlugins", 0) != 0;
        let lighting = !beside && crate::plugin_loaded("Lighting.asi");
        let surfaces = !beside && crate::plugin_loaded("Surfaces.asi");
        if self.on_planets && lighting {
            log!("world: Lighting.asi is loaded, which draws the planets: planets stay its");
            self.on_planets = false;
        }
        if self.on_rocks && (lighting || surfaces) {
            log!("world: Lighting.asi or Surfaces.asi is loaded: moons and asteroids stay the engine's");
            self.on_rocks = false;
        }
        if self.on_nebulae && !beside && crate::plugin_loaded("Nebulae.asi") {
            log!("world: Nebulae.asi is loaded, which draws the gas: nebulae stay its");
            self.on_nebulae = false;
        }
        if self.on_nebulae {
            self.nebulae.install();
        }
        self.lights.install(self.on_planets);
        self.nebulae.set_active(self.on_nebulae && self.enable);
        log!(
            "world: planets {}, moons and asteroids {}, nebulae {}",
            on(self.on_planets), on(self.on_rocks), on(self.on_nebulae)
        );
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

impl Module for World {
    fn name(&self) -> &'static str {
        "world"
    }
    fn device(&mut self, _dev: Device) {
        self.settle();
    }
    fn reset(&mut self) {
        self.surfaces.reset();
        self.planets.reset();
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
        if v.main && self.reload {
            let now = crate::rt::now();
            if now - self.checked > 1.0 {
                self.checked = now;
                let ini = crate::config::ini();
                let e = ini.int("World", "Enable", 1) != 0;
                if e != self.enable {
                    self.enable = e;
                    log!("world: Enable={}", e as u32);
                }
                let s = ini.int("World", "Surfaces", 1) != 0;
                if s != self.surfaces.enable {
                    self.surfaces.enable = s;
                    log!("world: Surfaces={}", s as u32);
                }
                if self.on_nebulae {
                    self.nebulae.reload();
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
            planets::VT_PLANET if self.on_planets => self.planets.draw(o, v, &mut self.surfaces),
            rocks::VT_SCRAP if self.on_rocks => self.rocks.draw_scrap(o, v, &mut self.surfaces),
            rocks::VT_ASTEROID_FIELD if self.on_rocks => self.rocks.draw_field(o, v, &mut self.surfaces),
            nebulae::VT_NEBULA | nebulae::VT_LATINUM if self.on_nebulae => self.nebulae.object(o, v),
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
    use crate::engine::obj;
    if obj::expired(o) || !obj::can_user_see(o) {
        return;
    }
    if crate::mods::hulls::game::frustum_test(obj::instance(o), v.cam) {
        crate::mods::hulls::game::set_on_screen(o);
    }
}
