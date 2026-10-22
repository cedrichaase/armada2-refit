//! The looks drawn with each kind of special-object effect, over stock's particle cloud
//! (which the particles module draws): a layer of light over the hull of the ship the
//! weapon has taken hold of (`shaders/specials.hlsl`), and a point light at the cloud.
//!
//! | Kind | Effect class | On the hull | Light |
//! |---|---|---|---|
//! | 0 | `TimedChangeTeam` (Computer Override) | green Borg circuitry creeping from the Borg ship's side | at the cloud, on the Borg ship |
//! | 1 | `TimedDrain` (Energy Dissipator) | a yellow-green film of spores and veins | at the cloud |
//! | 2 | `TimedCrewDisruption` (Psionic Disruption) | violet interference rippling | at the cloud |
//!
//! The hull is the hulls module's pose of the affected object (`hulls::posed`); an
//! object it does not draw (or a cloaked one) gets no layer.
use super::{Fx, VT_CHANGE_TEAM, VT_CREW_DISRUPTION, VT_DRAIN};
use crate::d3d::*;
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{v3, V3};
use crate::mods::effects::{rnd, Cam, LightPool, Sec};
use crate::rt::Ini;
use crate::scene;
use crate::shaders::specials::*;

pub const KINDS: usize = 3;
const NAMES: [&str; KINDS] = ["Override", "Drain", "Psionic"];

/// Which look an effect takes, by its class.
pub fn kind(vt: usize) -> Option<usize> {
    match vt {
        VT_CHANGE_TEAM => Some(0),
        VT_DRAIN => Some(1),
        VT_CREW_DISRUPTION => Some(2),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Kind {
    on: bool,
    /// Light colour of the layer (above 1 blooms).
    colour: V3,
    strength: f32,
    /// Seconds to cover the hull.
    creep: f32,
    /// Pattern cells per radius.
    scale: f32,
    rim: f32,
    light: V3,
    light_brightness: f32,
    /// Light reach, in the affected ship's radii.
    light_range: f32,
    /// Seconds over which it fades out at its end.
    fade: f32,
}

pub struct Looks {
    overlay: bool,
    lights_on: bool,
    kinds: [Kind; KINDS],
    rise: f32,
    fade: f32,
    push: f32,
    rim_power: f32,
    white: f32,
    vs: Shader,
    ps: Shader,
    decl: Decl,
    pool: LightPool,
    drawn: u32,
    said: bool,
}

impl Looks {
    pub fn new(_ini: &Ini) -> Looks {
        let s = Sec::new("Specials");
        // Defaults: (colour 0..255, gain, strength, creep s, scale, rim, light colour, brightness, range)
        let defs: [([f32; 3], f32, f32, f32, f32, f32, [f32; 3], f32, f32); KINDS] = [
            ([60.0, 255.0, 110.0], 1.6, 1.0, 2.5, 14.0, 0.15, [80.0, 255.0, 120.0], 1.6, 3.0),
            ([235.0, 80.0, 190.0], 1.4, 0.9, 1.5, 10.0, 0.3, [235.0, 90.0, 200.0], 1.0, 2.5),
            ([150.0, 110.0, 255.0], 1.6, 1.0, 0.8, 3.0, 0.6, [160.0, 110.0, 255.0], 1.4, 3.0),
        ];
        let mut kinds = [Kind { on: true, colour: V3::ZERO, strength: 0.0, creep: 1.0, scale: 1.0, rim: 0.0, light: V3::ZERO, light_brightness: 0.0, light_range: 0.0, fade: 1.0 }; KINDS];
        for (k, d) in defs.iter().enumerate() {
            let n = NAMES[k];
            let key = |suffix: &str| {
                let mut s = alloc::string::String::from(n);
                s.push_str(suffix);
                s
            };
            kinds[k] = Kind {
                on: s.i(&key(""), 1) != 0,
                colour: s.rgb(&key("Colour"), d.0) * s.f(&key("Gain"), d.1),
                strength: s.f(&key("Strength"), d.2),
                creep: s.f(&key("Creep"), d.3).max(0.05),
                scale: s.f(&key("Scale"), d.4),
                rim: s.f(&key("Rim"), d.5),
                light: s.rgb(&key("LightColour"), d.6),
                light_brightness: s.f(&key("LightBrightness"), d.7),
                light_range: s.f(&key("LightRange"), d.8),
                // The psionic disruption is renewed every half second while its beam
                // holds: it fades over its last quarter second only.
                fade: s.f(&key("Fade"), if k == 2 { 0.25 } else { 1.5 }).max(0.01),
            };
        }
        Looks {
            overlay: s.i("Overlay", 1) != 0,
            lights_on: s.i("Lights", 1) != 0,
            kinds,
            rise: s.f("Rise", 0.4).max(0.01),
            fade: 1.5,
            push: s.f("Push", 0.004),
            rim_power: s.f("RimPower", 2.5),
            white: s.f("CoreWhite", 0.35),
            vs: Shader::vertex(&VS_FX),
            ps: Shader::pixel(&PS_FX),
            decl: Decl::new(&scene::VERTEX),
            pool: LightPool::new(b"special\0"),
            drawn: 0,
            said: false,
        }
    }
    /// `Render3D.ini` read again (`[Specials] Reload=1`): the settings, keeping what is
    /// on the device.
    pub fn reload(&mut self, ini: &Ini) {
        let n = Looks::new(ini);
        self.overlay = n.overlay;
        self.lights_on = n.lights_on;
        self.kinds = n.kinds;
        self.rise = n.rise;
        self.fade = n.fade;
        self.push = n.push;
        self.rim_power = n.rim_power;
        self.white = n.white;
    }
    pub fn device(&mut self, _dev: Device) {}
    pub fn reset(&mut self) {}

    /// How strong an effect is now: up over `Rise` seconds from its start, down over
    /// its last `Fade` seconds.
    fn envelope(&self, f: &Fx) -> f32 {
        let up = (f.age / self.rise).clamp(0.0, 1.0);
        let fade = kind(f.vt).map(|k| self.kinds[k].fade).unwrap_or(self.fade);
        let down = (f.left / fade).clamp(0.0, 1.0);
        up * down
    }

    pub fn lights(&mut self, _v: &View, fx: &[Fx]) {
        if !self.lights_on {
            return;
        }
        let mut n = 0;
        for f in fx.iter().take(8) {
            let Some(k) = kind(f.vt) else { continue };
            let kd = self.kinds[k];
            if !kd.on || kd.light_brightness <= 0.0 {
                continue;
            }
            let e = self.envelope(f);
            if e <= 0.01 {
                continue;
            }
            let r = radius_of(f.target);
            let flick = 0.85 + 0.15 * libm::sinf(libm::fmod(crate::clock::anim_now(), 10000.0) as f32 * 9.0 + (f.effect as f32) * 0.37);
            let col = kd.light * (kd.light_brightness * e * flick);
            if self.pool.register(n, f.world.p, col, kd.light_range * r, kd.light_range * r * 0.5) {
                n += 1;
            }
        }
    }

    pub fn draw(&mut self, v: &View, fx: &[Fx]) {
        if !self.overlay || fx.is_empty() {
            return;
        }
        let Some(cam) = Cam::of(v) else { return };
        let dev = v.dev;
        let mut began = false;
        let _g = gfx::Guard::new(dev);
        let wall = libm::fmod(crate::clock::anim_now(), 10000.0) as f32;
        for f in fx {
            let Some(k) = kind(f.vt) else { continue };
            let kd = self.kinds[k];
            if !kd.on || f.victim == 0 {
                continue;
            }
            let e = self.envelope(f) * kd.strength;
            if e <= 0.005 {
                continue;
            }
            let Some(p) = crate::mods::hulls::posed(f.victim) else { continue };
            if !p.drawn || p.cloak.is_some_and(|c| c.0 != 0) {
                continue;
            }
            if !began {
                if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
                    return;
                }
                for (st, val) in [
                    (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 0), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHATESTENABLE, 0),
                    (RS_ALPHABLENDENABLE, 1), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD),
                    (RS_SEPARATEALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_FILLMODE, 3),
                    (RS_STENCILENABLE, 0), (RS_COLORWRITEENABLE, 7), (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
                ] {
                    dev.set_rs(st, val);
                }
                dev.set_stream_freq(0, 1);
                dev.vs_const(4, &gfx::vp_constants(v));
                began = true;
            }
            // The object's frame: its axes, unit; its centre and radius.
            let ax = |i: usize| crate::mods::hulls::norm_or(p.world.r[i], v3((i == 0) as u32 as f32, (i == 1) as u32 as f32, (i == 2) as u32 as f32));
            let (x, y, z) = (ax(0), ax(1), ax(2));
            let radius = p.radius.max(1.0);
            let c = p.centre;
            // Where the creep starts: towards the cloud when it is off the hull (the
            // override's, on the Borg ship), else a direction of the effect's own.
            let to = f.world.p - c;
            let seed = (f.effect as u32 >> 4) ^ (f.vt as u32);
            let dir = if to.len() > radius * 0.3 {
                to.norm()
            } else {
                v3(rnd(seed) * 2.0 - 1.0, rnd(seed ^ 0x9e37) * 2.0 - 1.0, rnd(seed ^ 0x51ed) * 2.0 - 1.0).norm()
            };
            let from = v3(dir.dot(x), dir.dot(y), dir.dot(z));
            let mut kv = [0.0f32; 20];
            kv[0..4].copy_from_slice(&[self.push * radius, scene::NORMAL_SIGN, 0.0, 0.0]);
            kv[4..8].copy_from_slice(&[c.x, c.y, c.z, 1.0 / radius]);
            kv[8..12].copy_from_slice(&[x.x, x.y, x.z, 0.0]);
            kv[12..16].copy_from_slice(&[y.x, y.y, y.z, 0.0]);
            kv[16..20].copy_from_slice(&[z.x, z.y, z.z, 0.0]);
            dev.vs_const(8, &kv);
            let mode = [(k == 0) as u32 as f32, (k == 1) as u32 as f32, (k == 2) as u32 as f32, 0.0];
            let pk: [f32; 24] = [
                kd.colour.x, kd.colour.y, kd.colour.z, e,
                mode[0], mode[1], mode[2], mode[3],
                wall, f.age / kd.creep, rnd(seed) * 10.0, kd.scale,
                from.x, from.y, from.z, 0.0,
                cam.eye.x, cam.eye.y, cam.eye.z, 0.0,
                kd.rim, self.rim_power, 0.08, self.white,
            ];
            dev.ps_const(0, &pk);
            for part in p.model.parts.iter() {
                if !p.pose.shown.get(part.node).copied().unwrap_or(false) || part.blend == scene::Blend::Additive {
                    continue;
                }
                let world = p.pose.world[part.node];
                dev.vs_const(0, world.to_m4().transpose().as_floats());
                let mirrored = world.det3() < 0.0;
                dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else if mirrored { CULL_CCW } else { CULL_CW });
                dev.set_stream(0, part.vb.ptr(), 0, 32);
                dev.set_indices(part.ib.ptr());
                for g in &part.groups {
                    dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                }
            }
            self.drawn += 1;
            if !self.said {
                self.said = true;
                crate::log!("specials: first layer drawn: {} over {} (radius {:.0})", NAMES[k], crate::engine::obj::odf_name(f.victim), radius);
            }
        }
    }
}

/// An object's bounding radius (its instance's `+0x40`), or 50.
fn radius_of(o: usize) -> f32 {
    let inst = crate::engine::obj::instance(o);
    let r = if inst != 0 { crate::engine::field_f32(inst, 0x40) } else { 0.0 };
    if r > 1.0 && r < 20000.0 { r } else { 50.0 }
}
