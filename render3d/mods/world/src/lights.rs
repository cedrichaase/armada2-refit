//! The world's lights, as `Lighting.asi` gave them, while `Lighting.asi` is not loaded (it adds its own):
//!
//! - **Planet glows.** A planet in sunlight lights what is near its day side in the
//!   colour of its ground: once per view (`Module::lights`), a point light per planet at
//!   its centre (the instance's transform +0x44 plus its bounding sphere +0x34, whose
//!   centre is in object space), the Key's colour times the ground's mean colour times
//!   `PlanetGlow`, full at the surface and gone at `PlanetGlowRange` radii. It is marked
//!   a day side (`FLAG_SHARE`, `is_day_side`): a lit draw takes it times the share of
//!   the day side facing it, and the planets themselves never take one.
//! - **Nebula lights.** Every nebula carries a point light (+0x1ac, made by
//!   `Nebula::InitializeGeometry`), which `Nebula::Simulate` registers while the nebula is
//!   on screen (+0x25) through `Simulate_Nebula_Lights` (0x4a5160, called at 0x4a4a7a):
//!   stock's colour is the class's glow (`red_glow`.., NebulaClass +0x22c) swung by a
//!   noise term, which came out black frame after frame, and its falloff the class's
//!   (+0x244, +0x248: 60 and 60 on every stock nebula). The call is re-pointed here: the
//!   glow colour times `NebulaBrightness`, steady, the falloff times `NebulaRange`; a
//!   class drawn as gas takes its recipe's hue at the glow's strength
//!   (`NebulaRecipeColour`). Only the light object's falloff is written.
//! - **Nebula cull.** Whether a nebula is on screen is `ST3D_Instance::FrustumTest`
//!   (0x62e990), slot 3 of the `NebulaInstance` vtable (0x6b1448), on its bounding
//!   sphere (the node's at +0x1c when the instance has one at +0x80, else its own at
//!   +0x34; radius at +0xc). The sphere is smaller than the cloud and its light, so a
//!   nebula beside a ship the camera is zoomed in on went, light and all; the slot tests
//!   the sphere with its radius times `NebulaCull` and puts it back.
use super::hooks;
use r3d_core::engine::{self, obj};
use r3d_core::frame::steps::View;
use r3d_core::math::{M34, V3, v3};
use crate::rt::Ini;
use r3d_core::scene;
use crate::log;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use r3d_core::engine::light;
/// A nebula's own glow to the gas: it lights what is near it, never the gas
/// (`[Nebulae] Lit=`).
static OWN_GLOW: scene::lights::Medium = scene::lights::Medium::new("own_glow", scene::media::OWN_GLOW);

/// Whether a light is a planet's day side (`Lights::view`): `FLAG_SHARE`, the one flag
/// this module gives its planet glows.
pub fn is_day_side(light: usize) -> bool {
    scene::lights::has_flag(light, scene::lights::FLAG_SHARE)
}

#[derive(Clone, Copy)]
struct Cfg {
    planet_glows: bool,
    glow: f32,
    glow_range: f32,
    nebulae: bool,
    neb_bright: f32,
    neb_range: f32,
    neb_recipe: bool,
    neb_cull: f32,
}

static mut CFG: Cfg = Cfg {
    planet_glows: true,
    glow: 1.5,
    glow_range: 6.0,
    nebulae: true,
    neb_bright: 1.4,
    neb_range: 8.0,
    neb_recipe: true,
    neb_cull: 3.0,
};

/// Per nebula class drawn as gas: its recipe's hue, brightest channel 1.
static mut HUES: BTreeMap<String, V3> = BTreeMap::new();

/// What the nebulae part says a class's gas looks like (None: not drawn as gas).
#[allow(static_mut_refs)]
pub fn set_hue(odf: &str, hue: Option<V3>) {
    unsafe {
        match hue {
            Some(h) => {
                HUES.insert(String::from(odf), h);
            }
            None => {
                HUES.remove(odf);
            }
        }
    }
}

fn cfg() -> Cfg {
    unsafe { CFG }
}

pub struct Lights {
    /// Planet glows: our own point lights, one per planet seen this view.
    pool: Vec<usize>,
    planets: bool,
    said: u32,
}

impl Lights {
    pub fn new() -> Lights {
        Lights { pool: Vec::new(), planets: false, said: 0 }
    }

    /// `[World]`'s light keys (`configure`).
    pub fn read(ini: &Ini) {
        let s = "World";
        unsafe {
            CFG = Cfg {
                planet_glows: ini.int(s, "PlanetGlows", 1) != 0,
                glow: ini.float(s, "PlanetGlow", 1.5),
                glow_range: ini.float(s, "PlanetGlowRange", 6.0),
                nebulae: ini.int(s, "NebulaLights", 1) != 0,
                neb_bright: ini.float(s, "NebulaBrightness", 1.4),
                neb_range: ini.float(s, "NebulaRange", 8.0),
                neb_recipe: ini.int(s, "NebulaRecipeColour", 1) != 0,
                neb_cull: ini.float(s, "NebulaCull", 3.0).max(1.0),
            };
        }
    }

    /// Whether the planets' glows are ours (the module draws the planets and no
    /// `Lighting.asi` adds its own).
    pub fn set_planets(&mut self, planets: bool) {
        self.planets = planets && !r3d_core::plugin_loaded("Lighting.asi");
    }

    fn light(&mut self, i: usize) -> Option<usize> {
        while self.pool.len() <= i {
            let l = light::new_point_light(b"planet glow\0");
            if l == 0 {
                return None;
            }
            scene::lights::set_flag(l, scene::lights::FLAG_SHARE);
            self.pool.push(l);
        }
        Some(self.pool[i])
    }

    /// Once per view: a glow per planet (`ground` gives a planet's ground colour).
    pub fn view(&mut self, _v: &View, mut ground: impl FnMut(usize) -> [f32; 3]) {
        let c = cfg();
        if !self.planets || !c.planet_glows {
            return;
        }
        let Some(key) = scene::lights::Lights::gather().key() else { return };
        let mut planets = Vec::new();
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if o != 0 && engine::vtable(o) == hooks::VT_PLANET {
                planets.push(o);
            }
        });
        for (i, o) in planets.into_iter().enumerate() {
            let inst = obj::instance(o);
            if inst == 0 || obj::class(o) == 0 {
                continue;
            }
            let sph = hooks::instance_sphere(inst);
            if !(sph[3] > 0.0) {
                continue;
            }
            let Some(l) = self.light(i) else { break };
            let g = ground(o);
            let col = [g[0] * key.colour.x * c.glow, g[1] * key.colour.y * c.glow, g[2] * key.colour.z * c.glow, 1.0];
            let xf = hooks::instance_world(inst);
            let m = M34 { r: M34::IDENTITY.r, p: xf.p + v3(sph[0], sph[1], sph[2]) };
            light::set_reach(l, sph[3], sph[3] * if c.glow_range > 1.0 { c.glow_range - 1.0 } else { 0.01 });
            light::register(l, &col, &m);
            if self.said & 1 == 0 {
                self.said |= 1;
                log!("world: lights: planet glows (ground {:.2} {:.2} {:.2})", g[0], g[1], g[2]);
            }
        }
    }
}

/// Whether the nebulae's lights are ours (`NebulaLights`), for the detour.
pub fn nebula_lights_on() -> bool {
    cfg().nebulae
}

/// `NebulaCull`: the factor on a nebula's sphere in the frustum test.
pub fn cull_scale() -> f32 {
    cfg().neb_cull
}

/// `Nebula::Simulate_Nebula_Lights`, replaced (see the module's notes): the detour in
/// `hooks.rs` calls this while the module is on.
#[allow(static_mut_refs)]
pub fn nebula_light(neb: usize) {
    let light = hooks::nebula_light_object(neb);
    let cls = obj::class(neb);
    if light == 0 || cls == 0 {
        return;
    }
    let c = cfg();
    let mut col = hooks::nebula_glow(cls);
    if c.neb_recipe {
        let name = crate::rt::lower(obj::odf_name(neb));
        if let Some(h) = unsafe { HUES.get(&name) } {
            let s = col[0].max(col[1]).max(col[2]);
            col = [h.x * s, h.y * s, h.z * s];
        }
    }
    let rgb = [col[0] * c.neb_bright, col[1] * c.neb_bright, col[2] * c.neb_bright, 1.0];
    // It lights the hulls near it, never the gas (`[Nebulae] Lit=`).
    scene::lights::set_medium(light, OWN_GLOW.id());
    let (start, range) = hooks::nebula_falloff(cls);
    hooks::set_light_reach(light, start * c.neb_range, range * c.neb_range);
    hooks::register_nebula_light(neb, light, &rgb);
}
