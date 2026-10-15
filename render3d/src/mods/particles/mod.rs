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
pub mod anim;
pub mod billboard;
pub mod draw;
pub mod look;
pub mod nebula;
pub mod sim;
pub mod vent;

use super::{Module, Step};
use crate::engine::{self, hook, rd, storm3d};
use crate::frame::steps::View;
use crate::math::{v3, M34};
use crate::scene;
use crate::{log, rt};
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use draw::{Batch, Cam};

const VT_DEVICE_DX8: usize = 0x6bc6ac;
const SLOT_RENDER_PARTICLE: usize = 50;
const FN_RENDER_PARTICLE: usize = 0x6262f0;
const ENG_CURRENT_INSTANCE: usize = 0x100;
const FN_NEW: usize = 0x652710; // operator new, cdecl
const FN_POINT_LIGHT: usize = 0x62f240; // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
const FN_REGISTER_LIGHT: usize = 0x62d250; // ST3D_GraphicsEngine::RegisterLight(light, colour*, Matrix34*), thiscall
const MAX_LIGHTS: usize = 16;
/// Whether the vents' lights are also added to the scene layer's list by hand (see
/// `Particles::lights`).
const SCENE_POINTS: bool = false;

static mut PREV_RP: usize = 0;
static mut SUPPRESS: bool = false;
static mut OWNED: BTreeSet<usize> = BTreeSet::new();
static mut DROPPED: u32 = 0;

#[allow(static_mut_refs)]
unsafe extern "thiscall" fn render_particle(dev: usize, pt: usize) {
    unsafe {
        if SUPPRESS {
            let inst = engine::field(storm3d(), ENG_CURRENT_INSTANCE);
            // An owned object's list, or the global list (inst 0: the chain pulsar's and
            // Manheim trails, ours too).
            if inst == 0 || OWNED.contains(&inst) {
                DROPPED = DROPPED.wrapping_add(1);
                return;
            }
        }
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(PREV_RP);
        f(dev, pt)
    }
}

static mut SYS: Option<sim::System> = None;
static mut LIVE: bool = false;

#[allow(static_mut_refs)]
fn sys() -> &'static mut sim::System {
    unsafe { SYS.get_or_insert_with(sim::System::new) }
}

/// For a module that draws an instance with emitters in the engine's place (a weapon's
/// SOD, a special effect's): its emitters at `world` (the transform it is drawn with,
/// scale included), simulated once this frame and drawn with every other particle.
/// `key` names the instance (its address); call it every frame it exists.
pub fn feed(key: usize, sod: &str, world: &M34, v: &View) {
    if unsafe { LIVE } {
        sys().feed(key, sod, world, v.time, v.eye);
    }
}

pub struct Particles {
    batch: Batch,
    vdraw: vent::Drawer,
    vcfg: vent::Cfg,
    nebulae: nebula::Nebulae,
    vents: Vec<vent::VentAt>,
    frame: Option<u32>,
    hooked: bool,
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
}

impl Default for Particles {
    fn default() -> Particles {
        let ini = crate::config::ini();
        let sys = sys();
        sys.cloud_fps = ini.float("Particles", "CloudFps", 30.0).max(1.0);
        if let Some(s) = ini.string("Particles", "PlasmaSprite") {
            sys.vent_sprites[0] = s;
        }
        if let Some(s) = ini.string("Particles", "GasSprite") {
            sys.vent_sprites[1] = s;
        }
        Particles {
            batch: Batch::new(),
            vdraw: vent::Drawer::new(),
            vcfg: vent::Cfg::read(&ini),
            nebulae: nebula::Nebulae::default(),
            vents: Vec::new(),
            frame: None,
            hooked: false,
            lights: Vec::new(),
            engine_lights: ini.int("Particles", "EngineLights", 1) != 0,
            log_every: ini.int("Particles", "LogEvery", 600).max(0) as u32,
            frames: 0,
            billboards: 0,
            enabled: ini.int("Particles", "Enable", 1) != 0,
            reload: ini.int("Particles", "Reload", 1) != 0,
        }
    }
}

impl Particles {
    fn hook(&mut self) {
        if self.hooked {
            return;
        }
        self.hooked = true;
        billboard::install();
        let prev = unsafe { rd::<usize>(VT_DEVICE_DX8 + SLOT_RENDER_PARTICLE * 4) };
        match hook::patch_slot(VT_DEVICE_DX8, SLOT_RENDER_PARTICLE, render_particle as *const () as usize) {
            Some(p) => {
                unsafe { PREV_RP = p };
                if p != FN_RENDER_PARTICLE {
                    log!("particles: RenderParticle chained to {:#x} (not the engine's)", p);
                }
            }
            None => log!("particles: RenderParticle not patched (was {:#x})", prev),
        }
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
                scene::add_point(scene::lights::Point { pos, colour: col, start, range, light });
            }
            n += 1;
        }
    }
}

impl Module for Particles {
    fn name(&self) -> &'static str {
        "particles"
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Venting.asi")
    }
    fn reset(&mut self) {
        self.batch.reset();
        self.vdraw.reset();
    }
    /// Before the scene layer reads the light list (`Module::lights`): the frame's
    /// simulation, then the vents' lights, so this frame's vents light this frame.
    fn lights(&mut self, v: &View) {
        self.hook();
        if self.frame != Some(v.frame) {
            self.frame = Some(v.frame);
            self.frames += 1;
            let t0 = rt::now();
            sys().simulate(v.time, v.eye);
            self.vents = vent::vents(sys(), &self.vcfg);
            let owned = unsafe { &mut *core::ptr::addr_of_mut!(OWNED) };
            owned.clear();
            for ow in sys().owners.values() {
                if ow.set.is_some() && ow.inst != 0 {
                    owned.insert(ow.inst);
                }
            }
            if self.reload && self.frames % 120 == 0 {
                let ini = crate::config::ini();
                let on = ini.int("Particles", "Enable", 1) != 0;
                if on != self.enabled {
                    log!("particles: Enable={} (Render3D.ini read again)", on as u32);
                }
                self.enabled = on;
                self.vcfg = vent::Cfg::read(&ini);
            }
            if self.log_every > 0 && self.frames % self.log_every == 1 {
                log!(
                    "particles: {} object(s) with emitters, {} particle(s), {} vent(s), {} nebula draw(s), {} billboard effect(s), {} engine particle draw(s) dropped, sim {:.2} ms",
                    sys().owners.values().filter(|o| o.set.is_some()).count(),
                    sys().count(),
                    self.vents.len(),
                    self.nebulae.drawn,
                    self.billboards,
                    unsafe { DROPPED },
                    (rt::now() - t0) * 1000.0
                );
            }
        }
        unsafe {
            SUPPRESS = self.enabled;
            LIVE = self.enabled;
        }
        billboard::skip(self.enabled);
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
        self.billboards += billboard::draw(v, &cam, &mut self.batch);
        self.batch.draw(v);
        self.vdraw.draw(v, &cam, sys, &self.vents, &self.vcfg);
        true
    }
    fn end_view(&mut self, _v: &View) {
        unsafe {
            SUPPRESS = false;
            LIVE = false;
        }
        billboard::skip(false);
    }
}
