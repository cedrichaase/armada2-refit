//! The world's lights, as `Lighting.asi` gave them, while `Lighting.asi` is not loaded (it adds its own):
//!
//! - **Planet glows.** A planet in sunlight lights what is near its day side in the
//!   colour of its ground: once per view (`Module::lights`), a point light per planet at
//!   its centre (the instance's transform +0x44 plus its bounding sphere +0x34, whose
//!   centre is in object space), the Key's colour times the ground's mean colour times
//!   `PlanetGlow`, full at the surface and gone at `PlanetGlowRange` radii. It is marked
//!   a day side (`scene::lights::set_day_side`): a lit draw takes it times the share of
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
use crate::engine::{self, field, field_f32, hook, obj, rd};
use crate::frame::steps::View;
use crate::math::{M34, V3, v3};
use crate::{log, scene};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

const FN_NEW: usize = 0x652710; // operator new, cdecl
const FN_POINT_LIGHT: usize = 0x62f240; // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
const FN_REGISTER_LIGHT: usize = 0x62d250; // ST3D_GraphicsEngine::RegisterLight(light, colour&, Matrix34&), thiscall
const SITE_NEBULA_LIGHTS: usize = 0x4a4a7a; // in Nebula::Simulate
const FN_NEBULA_LIGHTS: usize = 0x4a5160; // Nebula::Simulate_Nebula_Lights(float), thiscall
const FN_ENTITY_TRANSFORM: usize = 0x4cfd50; // Entity::GetTransform() const, thiscall
const VT_NEBULA_INSTANCE: usize = 0x6b1448;
const FN_FRUSTUM_TEST: usize = 0x62e990; // ST3D_Instance::FrustumTest(cam), thiscall

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

    pub fn read() {
        let ini = crate::config::ini();
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

    /// At settle: the nebulae's light and cull are ours unless `Lighting.asi` has them;
    /// the planets' glows when the module draws the planets.
    pub fn install(&mut self, planets: bool) {
        Self::read();
        self.planets = planets;
        if crate::plugin_loaded("Lighting.asi") {
            log!("world: lights: Lighting.asi is loaded: the nebulae's and planets' lights are its");
            self.planets = false;
            return;
        }
        let c = cfg();
        if c.nebulae {
            if hook::patch_call(SITE_NEBULA_LIGHTS, FN_NEBULA_LIGHTS, nebula_lights as *const () as usize) {
                log!("world: lights: the nebulae's lights: brightness {:.2}, range x{:.1}", c.neb_bright, c.neb_range);
            } else {
                log!("world: lights: Nebula::Simulate's light call is not stock's: the nebulae's lights stay as they are");
            }
        }
        if c.neb_cull > 1.0 {
            if field(VT_NEBULA_INSTANCE, 3 * 4) == FN_FRUSTUM_TEST && hook::patch_slot(VT_NEBULA_INSTANCE, 3, nebula_frustum as *const () as usize).is_some() {
                log!("world: lights: a nebula is on screen while its sphere times {:.1} is", c.neb_cull);
            } else {
                log!("world: lights: NebulaInstance's frustum test is not stock's: stock's nebula cull");
            }
        }
    }

    fn light(&mut self, i: usize) -> Option<usize> {
        while self.pool.len() <= i {
            let l = unsafe {
                let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(FN_NEW);
                let l = new(0x138);
                if l == 0 {
                    return None;
                }
                let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(FN_POINT_LIGHT);
                ctor(l, 0, 0, b"planet glow\0".as_ptr());
                l
            };
            scene::lights::set_day_side(l);
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
            if o != 0 && engine::vtable(o) == super::planets::VT_PLANET {
                planets.push(o);
            }
        });
        for (i, o) in planets.into_iter().enumerate() {
            let inst = obj::instance(o);
            if inst == 0 || obj::class(o) == 0 {
                continue;
            }
            let sph: [f32; 4] = unsafe { rd(inst + 0x34) };
            if !(sph[3] > 0.0) {
                continue;
            }
            let Some(l) = self.light(i) else { break };
            let g = ground(o);
            let col = [g[0] * key.colour.x * c.glow, g[1] * key.colour.y * c.glow, g[2] * key.colour.z * c.glow, 1.0];
            let xf: M34 = unsafe { rd(inst + 0x44) };
            let m = M34 { r: M34::IDENTITY.r, p: xf.p + v3(sph[0], sph[1], sph[2]) };
            unsafe {
                core::ptr::write_unaligned((l + 0x100) as *mut f32, sph[3]);
                core::ptr::write_unaligned((l + 0x104) as *mut f32, sph[3] * if c.glow_range > 1.0 { c.glow_range - 1.0 } else { 0.01 });
                let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
                reg(engine::storm3d(), l, col.as_ptr(), &m);
            }
            if self.said & 1 == 0 {
                self.said |= 1;
                log!("world: lights: planet glows (ground {:.2} {:.2} {:.2})", g[0], g[1], g[2]);
            }
        }
    }
}

/// `Nebula::Simulate_Nebula_Lights`, re-pointed (see the module's notes).
#[allow(static_mut_refs)]
unsafe extern "thiscall" fn nebula_lights(neb: usize, _dt: f32) {
    let light = field(neb, 0x1ac);
    let cls = obj::class(neb);
    if light == 0 || cls == 0 {
        return;
    }
    let c = cfg();
    let mut col: [f32; 3] = unsafe { rd(cls + 0x22c) };
    if c.neb_recipe {
        let name = crate::rt::lower(obj::odf_name(neb));
        if let Some(h) = unsafe { HUES.get(&name) } {
            let s = col[0].max(col[1]).max(col[2]);
            col = [h.x * s, h.y * s, h.z * s];
        }
    }
    let rgb = [col[0] * c.neb_bright, col[1] * c.neb_bright, col[2] * c.neb_bright, 1.0];
    // It lights the hulls near it, never the gas (`[Nebulae] Lit=`).
    scene::lights::set_gas(light, scene::lights::Gas::OwnGlow);
    unsafe {
        core::ptr::write_unaligned((light + 0x100) as *mut f32, field_f32(cls, 0x244) * c.neb_range);
        core::ptr::write_unaligned((light + 0x104) as *mut f32, field_f32(cls, 0x248) * c.neb_range);
        let tf: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(FN_ENTITY_TRANSFORM);
        let m = tf(neb);
        let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
        reg(engine::storm3d(), light, rgb.as_ptr(), m);
    }
}

/// `NebulaInstance` slot 3: the frustum test on the sphere times `NebulaCull`.
unsafe extern "thiscall" fn nebula_frustum(inst: usize, cam: usize) -> u32 {
    let node = field(inst, 0x80);
    let r = ((if node != 0 { node + 0x1c } else { inst + 0x34 }) + 0xc) as *mut f32;
    unsafe {
        let keep = core::ptr::read_unaligned(r);
        core::ptr::write_unaligned(r, keep * cfg().neb_cull);
        // Its answer is a bool in al: the rest of eax is not to be relied on.
        let f: unsafe extern "thiscall" fn(usize, usize) -> u32 = core::mem::transmute(FN_FRUSTUM_TEST);
        let inn = f(inst, cam) & 0xff;
        core::ptr::write_unaligned(r, keep);
        inn
    }
}
