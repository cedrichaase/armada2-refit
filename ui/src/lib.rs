//! UI.asi: the runtime that draws Star Trek: Armada II's HUD, menus and loading screen
//! in our own code. See README.md and PLAN.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt;
#[macro_use]
pub mod d3d;
pub mod app;
pub mod config;
pub mod engine;
pub mod gfx;
pub mod input;
pub mod math;
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
    log!("UI {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    if !cfg.enable {
        log!("Enable=0: standing down");
        return;
    }
    config::set(cfg);
    // Each context stands down on its own: a hook that is not stock leaves that part
    // of the game as it was, and the others still work.
    log!("input: {}", if input::install() { "captured" } else { "not captured" });
    log!("mission: {}", if engine::ingame::install() { "hooked" } else { "standing down" });
    log!("shell: {}", if engine::shell::install() { "hooked" } else { "standing down" });
    log!("loading: {}", if engine::loading::install() { "hooked" } else { "standing down" });
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        startup();
    }
    1
}
