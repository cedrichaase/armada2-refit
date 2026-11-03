//! The module registry (README.md, "Module interface"): every module, made at load, in
//! priority order (`Module::priority`), each with its on flag. The interface is
//! `core_module`; a module's hooks go through `core_hooks`, what it offers the others
//! through `core_services`.
//!
//! A module is switched per category in `Render3D.ini` (`[Modules]`, `<name>=1`, read
//! through its row of the core's table), and a module that replaces a plugin's work
//! stands down while that plugin is loaded (its `superseded()`), so the plugin keeps
//! drawing its part inside the engine's step.
//!
//! Dispatch skips a module that is off, and its `running()` is false, so the engine's
//! own draw takes the part back. Every module's flag is read again when the settings
//! change (`settings_changed`): one switched off gets `off()` there and lets go of its
//! GPU objects at the next view; one switched on gets its `hooks()` the first time, then
//! `on()`, and its `device()` at its next view if it never had this device. A module
//! whose hooks are not yet gated by its flag (`NOT_YET_LIVE`) still refuses a live
//! change, logged, and changes at the next launch (README.md, "Modules and settings").
//! A module that is off at load and has no part in the menu scene (`in_menu`) is never
//! given a device, so it installs nothing, as before.
use crate::core_hooks::{Gate, Hooks};
use crate::core_module::Changes;
pub use crate::core_module::{Module, ModuleId, Step};
use crate::frame::steps::View;
use crate::log;
use crate::rt::settings::{self, Setting};
use crate::settings_table::SETTINGS;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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

/// The modules whose hooks are not yet installed through `core_hooks` behind their
/// flag: a live change of their `[Modules]` row is refused (logged) and applies at the
/// next launch. One line per module; each is taken out when its hooks hand back.
const NOT_YET_LIVE: &[&str] = &[
    "world",
    "effects",
    "overlays",
];

/// Every module built in, one line each (the crates' features choose them later).
fn built_in() -> Vec<Box<dyn Module>> {
    alloc::vec![
        Box::new(world::World::default()) as Box<dyn Module>,
        Box::new(particles::Particles::default()),
        Box::new(hulls::Hulls::default()),
        Box::new(effects::Effects::new()),
        Box::new(ordnance::Ordnance::new()),
        Box::new(specials::Specials::default()),
        Box::new(sky::Sky::default()),
        Box::new(overlays::Overlays::default()),
    ]
}

/// A module and whether it runs.
struct Entry {
    m: Box<dyn Module>,
    id: ModuleId,
    /// Its on flag: dispatch reads it, and its hooks' gate is it.
    flag: &'static AtomicBool,
    row: &'static Setting,
    /// Let go of its GPU objects at the next view (it was just switched off).
    forget: bool,
    /// Its `hooks()` ran.
    hooked: bool,
    /// A hook it cannot draw without did not install: never switched on.
    failed: bool,
    /// Stood down beside its superseded plugin (`settle`): never dispatched.
    down: bool,
    /// The device generation it last had a `device()` for.
    dev: u32,
}

impl Entry {
    fn on(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
    /// Gets a device, resets and the menu scene's calls.
    fn present(&self) -> bool {
        !self.down && (self.on() || self.m.in_menu())
    }
}

static mut MODS: Vec<Entry> = Vec::new();
static mut ACTIVE: bool = false;
/// Which modules are on (bit = registry index), from load on; `running` adds the
/// first frame.
static ON: AtomicU32 = AtomicU32::new(0);
/// Bumped at every `device()`: a module switched on later gets its own at its next view.
static mut DEV_GEN: u32 = 0;

fn index(name: &str) -> Option<usize> {
    all().iter().position(|e| e.m.name() == name)
}

/// Whether a module runs: false before the first frame, and for a module that is off
/// (one kept for the menu scene's part only is not running). A logging aid: module
/// code asks for a capability (`core_services`), never for a module by name.
pub fn running(name: &str) -> bool {
    index(name).is_some_and(|i| running_id(ModuleId(i as u8)))
}

/// `running`, by registry index.
pub fn running_id(id: ModuleId) -> bool {
    let active = unsafe { ACTIVE };
    active && (id.0 as u32) < 32 && ON.load(Ordering::Relaxed) & (1 << id.0) != 0
}

/// `enabled`, by registry index: for a module asking about itself.
pub fn enabled_id(id: ModuleId) -> bool {
    (id.0 as u32) < 32 && ON.load(Ordering::Relaxed) & (1 << id.0) != 0
}

/// Whether a module's flag is on (true from load on, unlike `running`). A logging aid,
/// as `running`.
pub fn enabled(name: &str) -> bool {
    index(name).is_some_and(|i| ON.load(Ordering::Relaxed) & (1 << i) != 0)
}

#[allow(static_mut_refs)]
fn all() -> &'static mut Vec<Entry> {
    unsafe { &mut MODS }
}

/// The `[Modules] <name>` row of the core's table.
fn module_row(name: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|r| r.section() == "Modules" && r.key() == name)
}

/// Makes every module, with the flags `[Modules]` (and the overlay) says. Called once
/// at load.
pub fn init() {
    let ini = crate::config::ini();
    let mut made = built_in();
    made.sort_by_key(|m| m.priority());
    for w in made.windows(2) {
        assert!(w[0].priority() != w[1].priority(), "modules {} and {} share a priority", w[0].name(), w[1].name());
    }
    let mods = all();
    for (i, m) in made.into_iter().enumerate() {
        let Some(row) = module_row(m.name()) else {
            panic!("module {} has no [Modules] row in render3d/settings.toml", m.name());
        };
        let on = settings::on(&ini, row);
        if on {
            log!("module {}: loaded", m.name());
        } else if m.in_menu() {
            log!("module {}: off for missions (Render3D.ini [Modules] {}=1 turns it on); loaded for the menu scene's part", m.name(), m.name());
        } else {
            log!("module {}: off (Render3D.ini [Modules] {}=1 turns it on)", m.name(), m.name());
        }
        let flag: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(on)));
        mods.push(Entry { m, id: ModuleId(i as u8), flag, row, forget: false, hooked: false, failed: false, down: false, dev: 0 });
    }
    for e in mods.iter_mut() {
        e.m.provide(e.id, crate::core_services::services_mut());
    }
    for e in mods.iter_mut() {
        e.m.configure(&ini, &mut Changes::default());
    }
    for e in mods.iter_mut() {
        if e.on() && switch_on(e) {
            e.m.on();
        }
    }
    publish_flags();
}

/// A module being switched on: its hooks, the first time. False (and the flag shut) if
/// one it cannot draw without did not install.
fn switch_on(e: &mut Entry) -> bool {
    if e.failed {
        e.flag.store(false, Ordering::Relaxed);
        return false;
    }
    if !e.hooked {
        e.hooked = true;
        let mut h = Hooks::for_module(e.id, e.m.name(), Gate(e.flag));
        e.m.hooks(&mut h);
        if let Some(why) = h.down() {
            log!("module {}: stands down: {}", e.m.name(), why);
            e.failed = true;
            e.flag.store(false, Ordering::Relaxed);
            return false;
        }
    }
    e.flag.store(true, Ordering::Relaxed);
    true
}

/// The `[Modules]` row of a module, for the registry's callers.
pub fn row_of(name: &str) -> Option<&'static Setting> {
    index(name).map(|i| all()[i].row)
}

/// Each module's `rows()`, in priority order, appended to the core's for
/// `Render3D_Settings` (`crate::settings_rows`). With `Enable=0` no module is made,
/// so only the core's rows are exported.
pub fn export_rows(out: &mut Vec<Setting>) {
    for e in all().iter() {
        for r in e.m.rows() {
            // `Setting` is plain data (pointers to statics): copied bit for bit.
            out.push(unsafe { core::ptr::read(r) });
        }
    }
}

fn publish_flags() {
    let mut m = 0;
    for e in all().iter() {
        if e.on() && !e.down {
            m |= 1 << e.id.0;
        }
    }
    ON.store(m, Ordering::Relaxed);
}

/// The flags read again, after the ini or the overlay changed
/// (`Render3D_SettingsChanged`, on the game thread). A module switched off lets go of
/// its GPU objects at the next view, and the engine's own draw takes its part back at
/// once. Returns the changes as `Modules.<name>=<0|1>, ...`.
fn settings_changed(ini: &crate::rt::Ini) -> Changes {
    let mut said = Changes::default();
    for e in all().iter_mut() {
        if e.down {
            continue;
        }
        let want = settings::on(ini, e.row);
        if want == e.on() {
            continue;
        }
        let name = e.m.name();
        if NOT_YET_LIVE.contains(&name) {
            log!("settings: Modules.{} changes at the next launch (hooks not yet live)", name);
            continue;
        }
        if want {
            if !switch_on(e) {
                continue;
            }
            e.m.on();
        } else {
            e.flag.store(false, Ordering::Relaxed);
            e.m.off();
            e.forget = true;
        }
        said.note(&crate::sfmt!("Modules.{}={}", name, want as u32));
    }
    publish_flags();
    said
}

/// The modules that stood down for a superseded plugin: their row and the plugin's file
/// name (`Render3D_SettingsNote`, asked on the game thread).
static mut STOOD: Vec<(&'static Setting, &'static str)> = Vec::new();

/// The plugin a module's row stood down for, if it did (`settle`). The row is the
/// exported copy's or the table's: compared by section and key.
pub fn stood_down_for(row: *const Setting) -> Option<&'static str> {
    if row.is_null() {
        return None;
    }
    let row = unsafe { &*row };
    #[allow(static_mut_refs)]
    let v = unsafe { &STOOD };
    v.iter().find(|(r, _)| core::ptr::eq(r.section, row.section) && core::ptr::eq(r.key, row.key)).map(|(_, p)| *p)
}

/// At the first frame, when every plugin is loaded: the modules whose plugin is stand
/// down (they keep their place in the registry, and are never dispatched).
pub fn settle() {
    let mods = all();
    for e in mods.iter_mut() {
        if let Some(p) = e.m.superseded() {
            if crate::plugin_loaded(p) {
                #[allow(static_mut_refs)]
                unsafe { STOOD.push((e.row, p)) };
                log!("module {}: {} is loaded, which draws this part; the module stands down", e.m.name(), p);
                e.down = true;
                e.flag.store(false, Ordering::Relaxed);
            }
        }
    }
    publish_flags();
    for e in mods.iter() {
        if e.down {
            continue;
        }
        if e.on() {
            log!("module {}: on", e.m.name());
        } else if e.m.in_menu() {
            log!("module {}: on, for the menu scene only", e.m.name());
        }
    }
    unsafe { ACTIVE = true };
}

pub fn device() {
    let g = unsafe {
        DEV_GEN = DEV_GEN.wrapping_add(1).max(1);
        DEV_GEN
    };
    for e in all().iter_mut() {
        if e.present() {
            e.m.device();
            e.dev = g;
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
/// them again themselves when switched on, or for the menu scene); modules switched on
/// since their device's `device()` get it now.
fn let_go() {
    let g = unsafe { DEV_GEN };
    for e in all().iter_mut() {
        if e.forget {
            e.forget = false;
            e.m.forget();
        }
        if g != 0 && e.present() && e.dev != g {
            e.m.device();
            e.dev = g;
        }
    }
}

pub fn lights(v: &View) {
    let_go();
    for e in all().iter_mut() {
        if !e.on() || e.down {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        e.m.lights(v);
    }
}

pub fn begin_view(v: &View) {
    for e in all().iter_mut() {
        if e.down {
            continue;
        }
        if !e.on() {
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
        if !e.on() || e.down {
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
        if !e.on() || e.down {
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
        if !e.on() || e.down {
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
        if !e.on() || e.down {
            continue;
        }
        let _z = crate::timing::zone(e.m.name());
        e.m.end_view(v);
    }
}

/// What render3d claims to draw now (gpu/src/display.rs, "Claims"; README.md,
/// "Claims"): the OR of every running module's `claims()`. Nothing before the first
/// frame, and nothing for a module at 0 (kept for the menu scene only is not running)
/// or one standing down beside its old plugin. Display8 drops none of the engine's 3D
/// draws for these: a module that draws a category skips the engine's step, so what
/// the engine still issues (a hand-back) is drawn.
pub extern "C" fn claims() -> u32 {
    // Read only: Display8 may ask while the game thread is inside the registry.
    #[allow(static_mut_refs)]
    let mods = unsafe { &*core::ptr::addr_of!(MODS) };
    let mut c = 0;
    for e in mods.iter() {
        if running_id(e.id) {
            c |= e.m.claims();
        }
    }
    c
}

pub fn any() -> bool {
    unsafe { ACTIVE && all().iter().any(|e| !e.down) }
}

/// `Render3D_SettingsChanged`'s body for the modules: every module's `configure`
/// (whether it runs or not), in priority order, then the flags (a module switched on
/// gets `hooks()` and `on()` with its configuration read). Returns what changed, for
/// the log line: the flags first, then the keys.
pub fn apply_settings() -> alloc::string::String {
    let ini = crate::config::ini();
    let mut parts = Changes::default();
    for e in all().iter_mut() {
        if !e.down {
            e.m.configure(&ini, &mut parts);
        }
    }
    let mut said = settings_changed(&ini);
    said.note(&parts.0);
    said.0
}
