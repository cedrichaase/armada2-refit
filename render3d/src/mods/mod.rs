//! Modules: each takes over one or more of the body's steps, or the drawing of some
//! objects in the object loop, from the engine. A step no module takes is the
//! engine's own (`frame::steps`).
//!
//! A module is switched per category in `Render3D.ini` (`[Modules]`, `<name>=1`), and a
//! module that replaces a plugin's work stands down while that plugin is loaded (its
//! `superseded()`), so the plugin keeps drawing its part inside the engine's step.
//!
//! Every module is made at load and has an `on` flag (`Entry`): dispatch skips a module
//! that is off, and its `running()` is false, so the engine's own draw takes the part
//! back. `hulls`, `sky` and `specials` are read again when the settings change
//! (`settings_changed`); the others are latched at load, because they re-point engine
//! calls whose hooks are installed once (README.md, "Modules and settings"). A module
//! that is off at load and has no part in the menu scene (`in_menu`) is never given a
//! device, so it installs nothing, as before.
use crate::frame::steps::View;
use crate::log;
use crate::rt::settings;
use crate::settings_table::*;
use core::sync::atomic::{AtomicU32, Ordering};

pub mod effects;
pub mod hulls;
pub mod ordnance;
pub mod particles;
pub mod overlays;
pub mod sky;
pub mod specials;
pub mod world;
use alloc::boxed::Box;
use alloc::vec::Vec;

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

pub trait Module {
    fn name(&self) -> &'static str;
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
    /// A view of a mission while the module is off but `in_menu`: drop what the menu
    /// scene left that the mission's views must not see.
    fn idle_view(&mut self, _v: &View) {}
    /// A device to draw on (first frame, or after a new device): every plugin is
    /// loaded by now.
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

/// A module and whether it runs.
struct Entry {
    m: Box<dyn Module>,
    on: bool,
    /// Read again by `settings_changed`; else latched at load.
    live: bool,
    row: &'static settings::Setting,
    /// Let go of its GPU objects at the next view (it was just switched off).
    forget: bool,
}

impl Entry {
    /// Gets a device, resets and the menu scene's calls.
    fn present(&self) -> bool {
        self.on || self.m.in_menu()
    }
}

static mut MODS: Vec<Entry> = Vec::new();
static mut ACTIVE: bool = false;
/// Which modules are on (`NAMES`' bits), from load on; `running` adds the first frame.
static ON: AtomicU32 = AtomicU32::new(0);

/// The modules, in dispatch order.
const NAMES: [&str; 8] = ["world", "particles", "hulls", "effects", "ordnance", "specials", "sky", "overlays"];

fn bit(name: &str) -> u32 {
    NAMES.iter().position(|n| *n == name).map_or(0, |i| 1 << i)
}

/// Whether a module runs (asked by another module, e.g. whether a class of nebula is
/// drawn as a volume). False before the first frame, and for a module that is off
/// (one kept for the menu scene's part only is not running).
pub fn running(name: &str) -> bool {
    let active = unsafe { ACTIVE };
    active && ON.load(Ordering::Relaxed) & bit(name) != 0
}

/// Whether a module's flag is on (true from load on, unlike `running`).
pub fn enabled(name: &str) -> bool {
    ON.load(Ordering::Relaxed) & bit(name) != 0
}

#[allow(static_mut_refs)]
fn all() -> &'static mut Vec<Entry> {
    unsafe { &mut MODS }
}

/// Makes every module, with the flags `[Modules]` (and the overlay) says. Called once
/// at load.
pub fn init() {
    let ini = crate::config::ini();
    let mods = all();
    let mut add = |m: Box<dyn Module>, row: &'static settings::Setting, live: bool| {
        let on = settings::on(&ini, row);
        if on {
            log!("module {}: loaded", m.name());
        } else if m.in_menu() {
            log!("module {}: off for missions (Render3D.ini [Modules] {}=1 turns it on); loaded for the menu scene's part", m.name(), m.name());
        } else {
            log!("module {}: off (Render3D.ini [Modules] {}=1 turns it on)", m.name(), m.name());
        }
        mods.push(Entry { m, on, live, row, forget: false });
    };
    // One line per module; the first that takes a step or an object draws it.
    // particles before hulls: it takes the latinum nebulae, which draw through the
    // plain GameObject::Render as hulls do.
    // world first: it takes the nebulae (latinum included) and the rocks whose class has
    // a recipe, and leaves every other one to particles and hulls.
    // world=0, hulls=0 and sky=0 keep the menu scene's planets, nebulae, ships and sky.
    add(Box::new(world::World::default()), MODULES_WORLD, false);
    add(Box::new(particles::Particles::default()), MODULES_PARTICLES, false);
    add(Box::new(hulls::Hulls::default()), MODULES_HULLS, true);
    add(Box::new(effects::Effects::new()), MODULES_EFFECTS, false);
    add(Box::new(ordnance::Ordnance::new()), MODULES_ORDNANCE, false);
    add(Box::new(specials::Specials::default()), MODULES_SPECIALS, true);
    add(Box::new(sky::Sky::default()), MODULES_SKY, true);
    add(Box::new(overlays::Overlays::default()), MODULES_OVERLAYS, false);
    publish_flags();
}

fn publish_flags() {
    let mut m = 0;
    for e in all().iter() {
        m |= if e.on { bit(e.m.name()) } else { 0 };
    }
    ON.store(m, Ordering::Relaxed);
}

/// The flags of the modules `live` read again, after the ini or the overlay changed
/// (`Render3D_SettingsChanged`, on the game thread). A module switched off lets go of
/// its GPU objects at the next view, and the engine's own draw takes its part back at
/// once. Returns the changes as `Modules.<name>=<0|1>, ...`.
pub fn settings_changed() -> alloc::string::String {
    let ini = crate::config::ini();
    let mut said = alloc::string::String::new();
    for e in all().iter_mut() {
        if !e.live {
            continue;
        }
        let on = settings::on(&ini, e.row);
        if on == e.on {
            continue;
        }
        e.on = on;
        if !on {
            e.forget = true;
        }
        if !said.is_empty() {
            said.push_str(", ");
        }
        said.push_str(&crate::sfmt!("Modules.{}={}", e.m.name(), on as u32));
    }
    publish_flags();
    said
}

/// The modules that stood down for a superseded plugin: their row and the plugin's file
/// name (`Render3D_SettingsNote`, asked on the game thread).
static mut STOOD: Vec<(&'static settings::Setting, &'static str)> = Vec::new();

/// The plugin a module's row stood down for, if it did (`settle`).
pub fn stood_down_for(row: *const settings::Setting) -> Option<&'static str> {
    #[allow(static_mut_refs)]
    let v = unsafe { &STOOD };
    v.iter().find(|(r, _)| core::ptr::eq(*r, row)).map(|(_, p)| *p)
}

/// At the first frame, when every plugin is loaded: drop the modules whose plugin is.
pub fn settle() {
    let mods = all();
    mods.retain(|e| match e.m.superseded() {
        Some(p) if crate::plugin_loaded(p) => {
            #[allow(static_mut_refs)]
            unsafe { STOOD.push((e.row, p)) };
            log!("module {}: {} is loaded, which draws this part; the module stands down", e.m.name(), p);
            false
        }
        _ => true,
    });
    publish_flags();
    for e in mods.iter() {
        if e.on {
            log!("module {}: on", e.m.name());
        } else if e.m.in_menu() {
            log!("module {}: on, for the menu scene only", e.m.name());
        }
    }
    unsafe { ACTIVE = true };
}

pub fn device() {
    for e in all().iter_mut() {
        if e.present() {
            e.m.device();
        }
    }
}

pub fn reset() {
    for e in all().iter_mut() {
        if e.present() {
            e.m.reset();
        }
    }
}

pub fn forget() {
    for e in all().iter_mut() {
        if e.present() {
            e.m.forget();
        }
    }
}

/// Modules switched off since the last view let go of their GPU objects (they make
/// them again themselves when switched on, or for the menu scene).
fn let_go() {
    for e in all().iter_mut() {
        if e.forget {
            e.forget = false;
            e.m.forget();
        }
    }
}

pub fn lights(v: &View) {
    let_go();
    for e in all().iter_mut() {
        if !e.on {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        e.m.lights(v);
    }
}

pub fn begin_view(v: &View) {
    for e in all().iter_mut() {
        if !e.on {
            if e.m.in_menu() {
                e.m.idle_view(v);
            }
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        e.m.begin_view(v);
    }
}

pub fn step(s: Step, v: &View) -> bool {
    let mut done = false;
    for e in all().iter_mut() {
        if !e.on {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        if e.m.step(s, v) {
            done = true;
            if s != Step::Translucent {
                break;
            }
        }
    }
    done
}

pub fn object(o: usize, v: &View) -> bool {
    for e in all().iter_mut() {
        if !e.on {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        if e.m.object(o, v) {
            return true;
        }
    }
    survey(o);
    false
}

/// The survey of what is left to the engine: each class of object no module draws,
/// logged once (its vtable, `Render`, `RenderInternal`, its instance's database's
/// vtable and its ODF).
fn survey(o: usize) {
    static mut SEEN: Vec<usize> = Vec::new();
    let vt = crate::engine::vtable(o);
    #[allow(static_mut_refs)]
    let seen = unsafe { &mut SEEN };
    if vt == 0 || seen.len() >= 256 || seen.contains(&vt) {
        return;
    }
    seen.push(vt);
    let f = crate::engine::field;
    let db = f(f(o, 4), 0x80);
    log!(
        "survey: left to the engine: vtable {:#x} (Render {:#x}, RenderInternal {:#x}), database vtable {:#x}, odf '{}'",
        vt, f(vt, 11 * 4), f(vt, 22 * 4), crate::engine::vtable(db), crate::engine::obj::odf_name(o)
    );
}

pub fn feature(f: usize, v: &View) -> bool {
    for e in all().iter_mut() {
        if !e.on {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        if e.m.feature(f, v) {
            return true;
        }
    }
    false
}

pub fn menu_begin(shot: &crate::menu::Shot, v: &View) {
    for e in all().iter_mut() {
        if e.present() {
            e.m.menu_begin(shot, v);
        }
    }
}

pub fn menu(pass: crate::menu::Pass, shot: &crate::menu::Shot, v: &View) {
    for e in all().iter_mut() {
        if e.present() {
            e.m.menu(pass, shot, v);
        }
    }
}

pub fn end_view(v: &View) {
    for e in all().iter_mut() {
        if !e.on {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        e.m.end_view(v);
    }
}

/// What render3d claims to draw now (gpu/src/display.rs, "Claims"; README.md,
/// "Claims"): each module running, `[Overlays] Fog` and `Selection` apart. Nothing
/// before the first frame, and nothing for a module at 0 (kept for the menu scene only
/// is not running) or one standing down beside its old plugin. Display8 drops none of
/// the engine's 3D draws for these: a module that draws a category skips the engine's
/// step, so what the engine still issues (a hand-back) is drawn.
pub extern "C" fn claims() -> u32 {
    use gpu::display::*;
    let mut c = 0;
    for (name, bit) in [
        ("sky", CLAIM_SKY),
        ("hulls", CLAIM_HULLS),
        ("world", CLAIM_WORLD),
        ("particles", CLAIM_PARTICLES),
        ("effects", CLAIM_EFFECTS),
        ("ordnance", CLAIM_ORDNANCE),
        ("specials", CLAIM_SPECIALS),
    ] {
        if running(name) {
            c |= bit;
        }
    }
    if running("overlays") {
        let (fog, sel) = overlays::claims();
        if fog {
            c |= CLAIM_FOG;
        }
        if sel {
            c |= CLAIM_SELECTION;
        }
    }
    c
}

pub fn any() -> bool {
    unsafe { ACTIVE && !MODS.is_empty() }
}

/// `Render3D_SettingsChanged`'s body for the modules: the flags, then the keys the
/// modules read through rows. Returns what changed, for the log line.
pub fn apply_settings() -> alloc::string::String {
    let mut said = settings_changed();
    for part in [world::settings_changed(), overlays::settings_changed()] {
        if !part.is_empty() {
            if !said.is_empty() {
                said.push_str(", ");
            }
            said.push_str(&part);
        }
    }
    said
}
