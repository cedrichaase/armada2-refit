//! Render3D.asi: Star Trek: Armada II's 3D view drawn by our own renderer, in place of
//! the engine's `Armada_RenderAllOurStuff`. See README.md and PLAN.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt;
pub mod assets;
#[macro_use]
pub mod d3d;
pub mod config;
pub mod engine;
pub mod frame;
pub mod gfx;
pub mod math;
pub mod mods;
pub mod scene;
#[rustfmt::skip]
pub mod shaders;

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

fn startup() {
    rt::init();
    let cfg = config::load();
    rt::log_open("Render3D.log", cfg.log);
    log!("Render3D {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    if !cfg.enable {
        log!("Enable=0: standing down, the engine draws the 3D view");
        return;
    }
    config::set(cfg);
    mods::init();
    if frame::install() {
        log!("ready");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        startup();
    }
    1
}
