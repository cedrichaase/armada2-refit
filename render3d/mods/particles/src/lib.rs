//! Particles: our own particle simulation and drawing, in place of the engine's
//! (`spec-particles.md` in the notes), and the stock nebula puffs.
//!
//! - `sim`: every emitter on every game object's SOD, simulated once per frame on the
//!   sim clock (damage vents, the drones' steam...), with the quirks of stock's look.
//! - `vent`: `Venting.asi`'s look for the vents (`ventplas`, `ventgas`), their glow,
//!   jet and the light each casts on its hull.
//! - `draw`, `look`, `anim`: the sprite batch, `@sprite_node`s and their animations.
//! - `nebula`: map nebulae's puffs, for classes without a volume.
//!
//! The engine keeps simulating the particle lists of the instances it still draws (the
//! hulls module may draw them instead), but its draw of a particle of an object we own
//! is dropped: `ST3D_DeviceDirectX8::RenderParticle` (vtable 0x6bc6ac, slot 50, 0x6262f0)
//! is chained, and while one of our views runs, a particle handed over while the
//! engine's current instance (`Storm3D+0x100`) is an owned object's is not drawn.
//! Particles of other instances (the emitters of weapons and special effects:
//! contrails, rockets, rifts, gates, clouds) and the global list (the chain pulsar's
//! and the Manheim trails, `Storm3D+0x68`, drawn at `Step::Particles` and by their
//! ordnance) are still the engine's, drawn into our target.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The runtime and its macros, as `crate::rt`, `crate::log` and `crate::sfmt` (r3d-core's).
#[allow(unused_imports)]
use r3d_core::{dump, log, rt, sfmt};

#[rustfmt::skip]
pub mod shaders;
pub mod anim;
pub mod billboard;
pub mod content;
pub mod draw;
pub mod hooks;
pub mod look;
pub mod nebula;
pub mod sim;
pub mod vent;

use r3d_core::module::{Module, Step};
use r3d_core::module::Changes;
use r3d_core::frame::steps::View;
use r3d_core::math::{v3, M34};
use r3d_core::scene;
use alloc::string::String;
use alloc::vec::Vec;
use draw::{Batch, Cam};

use r3d_core::engine::light::{FN_NEW, FN_POINT_LIGHT, FN_REGISTER_LIGHT};
use r3d_core::engine::storm3d;
const MAX_LIGHTS: usize = 16;
/// Whether the vents' lights are also added to the scene layer's list by hand (see
/// `Particles::lights`).
const SCENE_POINTS: bool = false;

static mut SYS: Option<sim::System> = None;

#[allow(static_mut_refs)]
fn sys() -> &'static mut sim::System {
    unsafe { SYS.get_or_insert_with(sim::System::new) }
}

/// The emitters other modules fed this view (`particles`), into the simulation: each
/// instance simulated once this frame and drawn with every other particle.
fn drain() {
    for f in r3d_core::particles::take() {
        sys().feed(f.key, &f.sod, &f.world, f.time, f.eye);
    }
}

pub struct Particles {
    batch: Batch,
    vdraw: vent::Drawer,
    vcfg: vent::Cfg,
    nebulae: nebula::Nebulae,
    vents: Vec<vent::VentAt>,
    frame: Option<u32>,
    /// The engine's light objects for the vents, made on first use.
    lights: Vec<usize>,
    engine_lights: bool,
    log_every: u32,
    frames: u32,
    /// Billboard effects drawn (for the log).
    billboards: u32,
    /// `[Particles] Enable=0`: drawn by the engine, as without the module (for a
    /// comparison at the same moment, with `Reload=1`).
    enabled: bool,
    reload: bool,
    /// `[Particles] CloudFps`, `PlasmaSprite`, `GasSprite`: given to the simulation
    /// whenever it is made.
    cloud_fps: f32,
    vent_sprites: [String; 2],
}

impl Default for Particles {
    fn default() -> Particles {
        Particles {
            batch: Batch::new(),
            vdraw: vent::Drawer::new(),
            // Defaults until `configure` (an ini with no file reads as every default).
            vcfg: vent::Cfg::read(&rt::Ini::open_plain("")),
            nebulae: nebula::Nebulae::default(),
            vents: Vec::new(),
            frame: None,
            lights: Vec::new(),
            engine_lights: true,
            log_every: 600,
            frames: 0,
            billboards: 0,
            enabled: true,
            reload: true,
            cloud_fps: 30.0,
            vent_sprites: [String::from(content::VENT_SPRITES[0]), String::from(content::VENT_SPRITES[1])],
        }
    }
}

impl Particles {
    /// The simulation gets the configured values (it is made again after a switch-off).
    fn apply_to_sys(&self) {
        let sys = sys();
        sys.cloud_fps = self.cloud_fps;
        sys.vent_sprites = self.vent_sprites.clone();
    }

    /// The vents' lights: an engine `ST3D_Point_Light` per vent registered for the
    /// view (so the hulls the engine still draws take it, as Venting.asi's did), and the
    /// same light in the scene's list for ours. Full colour out to `LightReach` radii of
    /// the ship, gone a fifth of that further: a hard light at its place.
    fn register_lights(&mut self) {
        let eng = storm3d();
        let mut n = 0;
        for vt in self.vents.iter().take(MAX_LIGHTS) {
            let k = vt.kind as usize;
            let cfg = &self.vcfg;
            if cfg.light[k] <= 0.0 {
                continue;
            }
            let fl = vent::flicker(vt.seed, cfg.flicker[k]);
            let col = cfg.light_col[k] * (cfg.light[k] * fl);
            let start = cfg.light_reach[k] * vt.r;
            let range = start * 0.2;
            let pos = vt.pos + vt.dir * (cfg.light_lift[k] * vt.r);
            let mut light = 0usize;
            if self.engine_lights && eng != 0 {
                if n >= self.lights.len() {
                    unsafe {
                        let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(FN_NEW);
                        let l = new(0x138);
                        if l == 0 {
                            break;
                        }
                        let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(FN_POINT_LIGHT);
                        ctor(l, 0, 0, b"vent\0".as_ptr());
                        self.lights.push(l);
                    }
                }
                light = self.lights[n];
                unsafe {
                    core::ptr::write_unaligned((light + 0x100) as *mut f32, start);
                    core::ptr::write_unaligned((light + 0x104) as *mut f32, range);
                    let c = [col.x, col.y, col.z];
                    let m = M34 { r: [v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0)], p: pos };
                    let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
                    reg(eng, light, c.as_ptr(), &m);
                }
            }
            // Registered from `begin_view`, after the scene layer read the engine's list:
            // so the same light goes into the scene's list by hand. Where modules have a
            // `lights()` hook that runs before the scene reads the list, call this from
            // there and set SCENE_POINTS false, or the scene would see the light twice.
            if SCENE_POINTS {
                let medium = scene::lights::medium_of(light);
                scene::add_point(scene::lights::Point { pos, colour: col, start, range, light, style: scene::lights::Style::default(), medium }, medium);
            }
            n += 1;
        }
    }
}

impl Module for Particles {
    fn name(&self) -> &'static str {
        "particles"
    }
    fn priority(&self) -> i32 {
        20
    }
    fn claims(&self) -> u32 {
        gpu::display::CLAIM_PARTICLES
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &[]
    }
    /// `[Particles]`: the module's own switch (`Enable`, `Reload`), the simulation's
    /// (`CloudFps`, the vent sprites), the vents' look and lights, and the log.
    fn configure(&mut self, ini: &crate::rt::Ini, said: &mut Changes) {
        self.cloud_fps = ini.float("Particles", "CloudFps", 30.0).max(1.0);
        self.vent_sprites = [
            ini.string("Particles", "PlasmaSprite").unwrap_or_else(|| String::from(content::VENT_SPRITES[0])),
            ini.string("Particles", "GasSprite").unwrap_or_else(|| String::from(content::VENT_SPRITES[1])),
        ];
        self.apply_to_sys();
        self.vcfg = vent::Cfg::read(ini);
        self.engine_lights = ini.int("Particles", "EngineLights", 1) != 0;
        self.log_every = ini.int("Particles", "LogEvery", 600).max(0) as u32;
        let on = ini.int("Particles", "Enable", 1) != 0;
        if on != self.enabled {
            said.note(&crate::sfmt!("Particles.Enable={}", on as u32));
        }
        self.enabled = on;
        self.reload = ini.int("Particles", "Reload", 1) != 0;
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Venting.asi")
    }
    fn hooks(&mut self, h: &mut r3d_core::hooks::Hooks) {
        hooks::install(h);
    }
    /// Switched off live: the simulation, what it owns and what others fed it are gone,
    /// so a later switch-on starts clean (effects in flight end with the switch). The
    /// engine's draws are let through (the hooks are behind the flag).
    fn off(&mut self) {
        unsafe { SYS = None };
        hooks::clear();
        let _ = r3d_core::particles::take();
        r3d_core::particles::set_live(false);
        self.vents.clear();
        self.frame = None;
        self.batch.clear();
        self.apply_to_sys();
    }
    fn reset(&mut self) {
        self.batch.reset();
        self.vdraw.reset();
    }
    /// Before the scene layer reads the light list (`Module::lights`): the frame's
    /// simulation, then the vents' lights, so this frame's vents light this frame.
    fn lights(&mut self, v: &View) {
        if self.frame != Some(v.frame) {
            self.frame = Some(v.frame);
            self.frames += 1;
            let t0 = rt::now();
            sys().simulate(v.time, v.eye);
            self.vents = vent::vents(sys(), &self.vcfg);
            hooks::set_owned(sys().owners.values().filter(|ow| ow.set.is_some() && ow.inst != 0).map(|ow| ow.inst));
            if self.reload && self.frames % 120 == 0 {
                let was = self.enabled;
                self.configure(&r3d_core::config::ini(), &mut Changes::default());
                if self.enabled != was {
                    log!("particles: Enable={} (Render3D.ini read again)", self.enabled as u32);
                }
            }
            if self.log_every > 0 && self.frames % self.log_every == 1 {
                log!(
                    "particles: {} object(s) with emitters, {} particle(s), {} vent(s), {} nebula draw(s), {} billboard effect(s), {} engine particle draw(s) dropped, sim {:.2} ms",
                    sys().owners.values().filter(|o| o.set.is_some()).count(),
                    sys().count(),
                    self.vents.len(),
                    self.nebulae.drawn,
                    self.billboards,
                    hooks::dropped(),
                    (rt::now() - t0) * 1000.0
                );
            }
        }
        hooks::suppress(self.enabled);
        r3d_core::particles::set_live(self.enabled);
        self.batch.clear();
        self.register_lights();
    }
    fn object(&mut self, o: usize, v: &View) -> bool {
        if !self.enabled {
            return false;
        }
        let cam = Cam::of(v);
        self.nebulae.object(o, v, &cam, &mut self.batch)
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s != Step::Translucent || !self.enabled {
            return false;
        }
        drain();
        let cam = Cam::of(v);
        // Every other particle in the stock look (the vents in theirs, below).
        let sys = sys();
        let lists = sys.owners.values().filter(|ow| ow.visible).map(|ow| (&ow.parts, ow.mult, ow.scale));
        for (parts, mult, scale) in lists.chain(core::iter::once((&sys.orphans, sys.global_mult, 1.0))) {
            for p in parts {
                let t = &p.tpl;
                if t.vent.is_some_and(|k| self.vcfg.kind_on[k as usize]) {
                    continue;
                }
                let lk = &t.look;
                let Some(tex) = lk.texture.as_deref() else { continue };
                // RenderParticle: the size grows with age; the node data's animations
                // run on the particle's age; the draw flag is not looked at.
                let hx = (lk.size[0] + t.growth * p.age) * scale;
                let hy = (lk.size[1] + t.growth * p.age) * scale;
                if hx <= 0.0 || hy <= 0.0 {
                    continue;
                }
                let (c, _) = lk.state(p.age, 3);
                let c = c * mult;
                let uv = lk.uv_at(p.age, 3);
                self.batch.billboard(&cam, lk.material, tex, p.pos, hx, hy, uv, [c.x, c.y, c.z, 1.0]);
            }
        }
        if hooks::billboards_hooked() {
            self.billboards += billboard::draw(v, &cam, &mut self.batch);
        }
        self.batch.draw(v);
        self.vdraw.draw(v, &cam, sys, &self.vents, &self.vcfg);
        true
    }
    fn end_view(&mut self, _v: &View) {
        drain();
        hooks::suppress(false);
        r3d_core::particles::set_live(false);
    }
}
