//! UI.asi: the runtime that draws Star Trek: Armada II's HUD, menus and loading screen
//! in our own code. See README.md and PLAN.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt;
pub mod dump;
pub mod app;
pub mod clipboard;
pub mod clock;
pub mod config;
pub mod credits;
pub mod cursor;
pub mod engine;
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
#[rustfmt::skip]
pub mod shaders;
pub mod style;
pub mod surface;
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
    clock::load();
    log!("UI {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    if !cfg.enable {
        log!("Enable=0: standing down");
        return;
    }
    config::set(cfg);
    // Each context stands down on its own: a hook that is not stock leaves that part
    // of the game as it was, and the others still work.
    if config::get().native {
        log!("display: {}", if engine::display::install() { "a stock mode becomes the desktop's" } else { "the call is not stock: the game's mode is as set" });
    }
    log!("input: {}", if input::install() { "captured" } else { "not captured" });
    let mission = engine::ingame::install();
    log!("mission: {}", if mission { "hooked" } else { "standing down" });
    let h = &config::get().hud;
    if mission && h.enable {
        // Drawn in the mission frame: without its hook they stay the engine's.
        let n = engine::leftovers::install(h.overview, h.fade, h.unit_video);
        log!("leftovers: {} of the engine's last draws are ours", n);
    }
    log!("shell: {}", if engine::shell::install() { "hooked" } else { "standing down" });
    log!("loading: {}", if engine::loading::install(config::get().loading) { "hooked" } else { "standing down" });
    log!("hud: {}", if hud::install() { "panels of ours" } else { "stock" });
    log!("device: {}", if engine::device::install() { "DestroyRenderSurfaces chained: the UI lets go of the device with the engine" } else { "slot not stock: the UI lets go of the device when it sees the next one" });
    // The menus: [Menus] Enable=1 hosts the stock shell's dialogs (ui/MENUS.md). Patched
    // here, at DllMain, because the shell's display mode must be raised before the
    // engine first sets it.
    if config::get().menus {
        log!("menus: {}", if engine::dialogs::install() { "hosted by the UI" } else { "standing down: the shell runs stock" });
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        startup();
    }
    1
}
