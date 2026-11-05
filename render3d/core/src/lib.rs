//! `r3d-core`, the renderer core of Render3D.asi (README.md, "Crates"): the frame, gfx,
//! scene, assets, the shared engine glue, the menu scene, the module interface and its
//! registry, hooks, services and recipes. Each module is a crate of its own
//! (`render3d/mods/<m>/`) that depends on this one; the cdylib (`render3d/src/lib.rs`)
//! holds DllMain, the allocator, the panic handler and the exports, and gives `start`
//! the modules built in.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod rt {
    pub use a2rt::*;
}
pub use a2rt::{dump, log, sfmt};
pub mod assets;
pub mod files;
pub mod clock;
pub mod config;
pub mod hooks;
pub mod module;
pub mod particles;
pub mod recipes;
pub mod services;
pub mod engine;
pub mod frame;
pub mod gfx;
pub mod math;
pub mod menu;
pub mod registry;
pub mod scene;
pub mod settings_table;
pub mod syms;
#[rustfmt::skip]
pub mod shaders;
pub mod timing;

use rt::win::*;

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
        // A module not built in (a build without its feature) has no `[Modules]` row.
        if r.section() == "Modules" && !registry::built(r.key()) {
            continue;
        }
        // `Setting` is plain data (pointers to statics): copied bit for bit.
        v.push(unsafe { core::ptr::read(r) });
    }
    let core = v.len();
    registry::export_rows(&mut v);
    log!("settings: {} rows exported ({} the core's)", v.len(), core);
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

/// Render3D.asi's load (DllMain): the configuration, then, unless `Enable=0`, every
/// module built in (`built`, the cdylib's list) and the frame's hooks.
/// `version` is the DLL's (its package's).
pub fn start(version: &str, built: &'static [registry::BuiltIn]) {
    registry::set_built_in(built);
    rt::init();
    let cfg = config::load();
    rt::log_open("Render3D.log", cfg.log);
    log!("Render3D {} -- game dir {}", version, rt::game_dir());
    engine::start_table("core", &syms::TABLE);
    let reads = hooks::Hooks::core().ready(&syms::TABLE, syms::hook::CORE_READS);
    registry::log_built_in();
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
    if !reads {
        log!("core: the engine's reads are not stock's: standing down");
        DOWN.store(DOWN_SEAM, core::sync::atomic::Ordering::Relaxed);
        build_rows();
        return;
    }
    config::set(cfg);
    registry::init();
    build_rows();
    engine::viewdist::install();
    if frame::install() {
        // With Display8 (contract 1.1) the modules' claims are asked every frame
        // (README.md, "Claims"); a render3d that does not hook the frame claims nothing.
        gfx::gpu().register(registry::claims);
        log!("ready");
    } else {
        DOWN.store(DOWN_SEAM, core::sync::atomic::Ordering::Relaxed);
    }
}
