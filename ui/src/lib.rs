//! UI.asi: the runtime that draws Star Trek: Armada II's HUD, menus and loading screen
//! in our own code. See README.md and PLAN.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt {
    pub use a2rt::*;
    pub mod win;
}
pub use a2rt::{dump, log, sfmt};
pub mod app;
pub mod claims;
pub mod clipboard;
pub mod clock;
pub mod config;
pub mod content;
pub mod content_sets;
pub mod credits;
pub mod cursor;
pub mod engine;
pub mod files;
pub mod gallery;
pub mod gfx;
pub mod hud;
pub mod icons;
pub mod input;
pub mod loading;
pub mod math;
pub mod menus;
pub mod movie;
pub mod prefs;
pub mod settings;
mod settings_table;
#[rustfmt::skip]
pub mod shaders;
pub mod style;
pub mod surface;
pub mod syms;
#[rustfmt::skip]
pub mod syms_core;
#[rustfmt::skip]
pub mod syms_dialogs;
#[rustfmt::skip]
pub mod syms_hud;
#[rustfmt::skip]
pub mod syms_info;
#[rustfmt::skip]
pub mod syms_loading;
#[rustfmt::skip]
pub mod syms_panels;
#[rustfmt::skip]
pub mod syms_radar;
#[rustfmt::skip]
pub mod syms_shell;
pub mod text;
pub mod widgets;

use rt::win::*;

#[global_allocator]
static HEAP: rt::mem::Heap = rt::mem::Heap;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    rt::log_str("UI stopped the game: a bug in the UI runtime. Set Enable=0 in UI.ini to play without it.");
    unsafe { ExitProcess(3) }
}

fn startup() {
    rt::init();
    let cfg = config::load();
    rt::log_open("UI.log", cfg.log);
    // The overlay's values read by `load` came before the log was open and were not
    // noted (a2rt `overlay::note`): read once more, from the cache, so the log has them.
    let _ = config::load();
    clock::load();
    log!("UI {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    // The identity, the build and each part's symbols summary (crates/a2engine).
    engine::api::tables::init();
    if !cfg.enable {
        log!("Enable=0: standing down");
        return;
    }
    config::set(cfg);
    config::remember();
    // With Display8 (contract 1.1) the parts' claims route its passthrough: registered
    // before any part is patched (README.md, "Claims").
    claims::register();
    // Each context stands down on its own: a hook that is not stock leaves that part
    // of the game as it was, and the others still work.
    // The display mode (the profile's, the shell's, the intro's) and the pointer's clip
    // are Display8's (`[Window]` in Display8.ini).
    if config::get().movies {
        engine::api::intro::route_frames();
    }
    log!("input: {}", if input::install() { "captured" } else { "not captured" });
    let mission = engine::api::ingame::install();
    log!("mission: {}", if mission { "hooked" } else { "standing down" });
    let h = &config::get().hud;
    if mission && h.enable {
        // Drawn in the mission frame: without its hook they stay the engine's.
        let n = engine::api::leftovers::install(h.overview, h.fade, h.unit_video, h.frame_rate);
        log!("leftovers: {} of the engine's last draws are ours", n);
    }
    log!("shell: {}", if engine::api::shell::install() { "hooked" } else { "standing down" });
    log!("loading: {}", if engine::api::loading::install(config::get().loading) { "hooked" } else { "standing down" });
    let hud = hud::install();
    claims::set_hud(hud);
    log!("hud: {}", if hud { "panels of ours" } else { "stock" });
    log!("device: {}", if engine::api::device::install() { "DestroyRenderSurfaces chained: the UI lets go of the device with the engine" } else { "slot not stock: the UI lets go of the device when it sees the next one" });
    // The menus: [Menus] Enable=1 hosts the stock shell's dialogs (ui/MENUS.md). Patched
    // here, at DllMain, because the shell's display mode must be raised before the
    // engine first sets it.
    if config::get().menus {
        log!("menus: {}", if engine::api::dialogs::install() { "hosted by the UI" } else { "standing down: the shell runs stock" });
    }
}

/// `UI_SettingsNote` (tools/settings/README.md, C5): why a row's part is not running now.
/// The HUD's rows while the HUD stood down. NULL for the plugin as a whole and for any other row.
#[unsafe(no_mangle)]
pub extern "C" fn UI_SettingsNote(row: *const rt::settings::Setting) -> *const u8 {
    use settings_table::{HUD_BARHIDDEN, HUD_ENABLE};
    if row.is_null() {
        // The plugin as a whole: an Armada2.exe Refit does not know.
        return engine::api::tables::note_plugin().unwrap_or(core::ptr::null());
    }
    let r = unsafe { &*row };
    // Why a hook that feeds this row stood down, in the engine's own words.
    if let Some(n) = engine::api::tables::note_for(r.section(), r.key()) {
        return n;
    }
    if core::ptr::eq(r, HUD_ENABLE) || core::ptr::eq(r, HUD_BARHIDDEN) {
        return core::ptr::null();
    }
    if r.section() == "HUD" {
        let why: &'static [u8] = match unsafe { hud::STOOD_DOWN } {
            1 => b"The HUD is switched off (Enable=0): the game's own HUD is drawn\0",
            2 => b"No HUD panel is switched on: the game's own HUD is drawn\0",
            3 => b"An engine function is not stock: the game's own HUD is drawn\0",
            4 => b"The HUD could not be installed: the game's own HUD is drawn\0",
            _ => b"\0",
        };
        if why[0] != 0 {
            return why.as_ptr();
        }
    }
    core::ptr::null()
}

/// `UI_BarCell` (qol/bar.c, QOL-1): the stock button in cell 0..14 of the bar's menu now
/// shown, or NULL when the cell is empty or the bar is not drawn. QOL's position keys
/// work only while this gives a cell.
#[unsafe(no_mangle)]
pub extern "C" fn UI_BarCell(cell: u32) -> usize {
    hud::bar::cell_button(cell)
}

/// `UI_BarStep` (qol/bar.c): the bar's update has run; QOL calls it on the game's thread
/// after the stock keyboard step, so the UI's clicks are pressed with this update's buttons.
#[unsafe(no_mangle)]
pub extern "C" fn UI_BarStep(palette: usize) {
    hud::bar::step_from_qol(palette);
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        startup();
    }
    1
}
