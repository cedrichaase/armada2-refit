//! Render3D.asi: Star Trek: Armada II's 3D view drawn by our own renderer, in place of
//! the engine's `Armada_RenderAllOurStuff`. See README.md and PLAN.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt {
    pub use a2rt::*;
}
pub use a2rt::{dump, log, sfmt};
pub mod assets;
pub mod clock;
pub mod config;
pub mod core_hooks;
pub mod core_module;
pub mod core_particles;
pub mod core_recipes;
pub mod core_services;
pub mod engine;
pub mod frame;
pub mod gfx;
pub mod math;
pub mod menu;
pub mod mods;
pub mod scene;
pub mod settings_table;
#[rustfmt::skip]
pub mod shaders;
pub mod timing;

use rt::win::*;

#[global_allocator]
static HEAP: rt::mem::Heap = rt::mem::Heap;

/// The plugins whose work moves into render3d's modules. While the engine's own step
/// draws a category, the plugin hooked into that step keeps working there; a module
/// of ours that takes a category over stands down while its plugin is loaded.
/// MSAA.asi only wastes work beside us: our scene target has samples of its own.
pub const SUPERSEDED: &[&str] = &[
    "Lighting.asi", "Sky.asi", "Nebulae.asi", "Surfaces.asi", "Explosions.asi", "Shields.asi",
    "Repair.asi", "Venting.asi", "Specials.asi", "Fog.asi", "Planets.asi", "MSAA.asi",
];

/// Whether a plugin is loaded, by file name. Asked at the first frame, not at load:
/// the ASI loader may not have loaded the others yet when our DllMain runs.
pub fn plugin_loaded(name: &str) -> bool {
    let c = rt::cstr(name);
    !unsafe { GetModuleHandleA(c.as_ptr()) }.is_null()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    rt::log_str("Render3D stopped the game: a bug in the renderer. Set Enable=0 in Render3D.ini to play without it.");
    unsafe { ExitProcess(3) }
}

/// Why render3d is not drawing, for `Render3D_SettingsNote` (0: it is, or not known yet).
pub static DOWN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub const DOWN_DISABLED: u32 = 1;
pub const DOWN_SEAM: u32 = 2;
/// The 3D view's targets or post shaders could not be made (no usable device).
pub const DOWN_DEVICE: u32 = 3;

/// `Render3D_Settings`' one table (A2SET_ABI 1.1, contiguous rows): the core's rows,
/// then each built-in module's in priority order (README.md, "Modules and settings").
/// Built once at load; the row constants the code reads with stay the tables' own.
static mut ROWS: alloc::vec::Vec<rt::settings::Setting> = alloc::vec::Vec::new();
static ROWS_BUILT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

fn build_rows() {
    let mut v = alloc::vec::Vec::new();
    for r in settings_table::SETTINGS.iter() {
        // `Setting` is plain data (pointers to statics): copied bit for bit.
        v.push(unsafe { core::ptr::read(r) });
    }
    mods::export_rows(&mut v);
    log!("settings: {} rows exported ({} the core's)", v.len(), settings_table::SETTINGS.len());
    unsafe { ROWS = v };
    ROWS_BUILT.store(true, core::sync::atomic::Ordering::Release);
}

/// The rows `Render3D_Settings` returns (`settings_table.rs`).
pub fn settings_rows() -> &'static [rt::settings::Setting] {
    if !ROWS_BUILT.load(core::sync::atomic::Ordering::Acquire) {
        build_rows();
    }
    unsafe { &*core::ptr::addr_of!(ROWS) }
}

fn startup() {
    rt::init();
    let cfg = config::load();
    rt::log_open("Render3D.log", cfg.log);
    log!("Render3D {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    timing::configure(cfg.timing, cfg.timing_every);
    clock::load();
    if !cfg.enable {
        log!("Enable=0: standing down, the engine draws the 3D view");
        DOWN.store(DOWN_DISABLED, core::sync::atomic::Ordering::Relaxed);
        if cfg.timing && frame::install_measure() {
            log!("timing: the engine's own body is timed (Enable=0)");
        }
        build_rows();
        return;
    }
    config::set(cfg);
    mods::init();
    build_rows();
    if frame::install() {
        // With Display8 (contract 1.1) the modules' claims are asked every frame
        // (README.md, "Claims"); a render3d that does not hook the frame claims nothing.
        gfx::gpu().register(mods::claims);
        log!("ready");
    } else {
        DOWN.store(DOWN_SEAM, core::sync::atomic::Ordering::Relaxed);
    }
}

/// `Render3D_SettingsNote` (`tools/settings/README.md`, C5): why a row is not running,
/// or NULL. `row` NULL asks about the plugin as a whole. The text lives in a static
/// buffer, valid until the next call; nothing is written but that buffer.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_SettingsNote(row: *const rt::settings::Setting) -> *const u8 {
    use core::sync::atomic::Ordering;
    static mut BUF: [u8; 128] = [0; 128];
    let b = unsafe { &mut *core::ptr::addr_of_mut!(BUF) };
    let mut n = 0;
    let mut put = |s: &str| {
        for &c in s.as_bytes() {
            if n < 126 {
                b[n] = c;
                n += 1;
            }
        }
    };
    if row.is_null() {
        match DOWN.load(Ordering::Relaxed) {
            DOWN_DISABLED => put("Render3D is off (Enable=0 in Render3D.ini): the engine draws the 3D view"),
            DOWN_SEAM => put("Render3D stood down: the game's 3D call is not the stock one"),
            DOWN_DEVICE => put("Render3D stood down: no usable graphics device, the engine draws the 3D view"),
            _ => return core::ptr::null(),
        }
    } else {
        let Some(p) = mods::stood_down_for(row) else { return core::ptr::null() };
        put("Stands down while ");
        put(p);
        put(" is loaded");
    }
    b[n] = 0;
    b.as_ptr()
}

/// The hub's call after the overlay or an ini changed (`tools/settings/README.md`, C5):
/// the live rows are read again and applied on the game thread, in one log line. Rows
/// that are not live (`apply` restart) are latched at load.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_SettingsChanged() {
    // Stood down (Enable=0, or the seam is not stock's): nothing of ours to change.
    if config::get().is_none() {
        return;
    }
    rt::ini_refresh_all();
    let mut said = mods::apply_settings();
    // The frame's own keys: swapped in whole; MSAA or HDR moving rebuilds the scene
    // target (and the post passes made for its format) at the next view.
    if let Some((parts, formats)) = config::reload() {
        if !parts.is_empty() {
            if !said.is_empty() {
                said.push_str(", ");
            }
            said.push_str(&parts);
        }
        if formats {
            frame::formats_changed();
        }
    }
    if said.is_empty() {
        log!("settings: re-read, nothing changed");
    } else {
        log!("settings: applied {}", said);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        startup();
    }
    1
}
