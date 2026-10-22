//! Display8: Star Trek: Armada II's `d3d8.dll`, ours. In forward mode, the only one so
//! far, it stands in front of the real chain (`d3d8_real.dll`: crosire's d3d8to9, then
//! DXVK's or Windows' d3d9) and passes every call through, tracing it on the bench
//! (`Trace=1`). See README.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

#[macro_use]
pub mod macros;
pub mod rt;
pub mod com;
pub mod d3d8;
pub mod detail;
pub mod drop;
pub mod trace;
pub mod user32;

use core::ffi::c_void;
use rt::win::*;

#[global_allocator]
static HEAP: rt::mem::Heap = rt::mem::Heap;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    rt::log_str("Display8 stopped the game: a bug in d3d8.dll. Take it out (or put d3d8_real.dll back as d3d8.dll) to play without it.");
    unsafe { ExitProcess(3) }
}

struct Config {
    log: bool,
    trace: bool,
    real: alloc::string::String,
}

static mut CONFIG: Option<Config> = None;
static mut REAL: HANDLE = core::ptr::null_mut();

fn config() -> &'static Config {
    unsafe { CONFIG.as_ref().expect("config") }
}

fn startup() {
    rt::init();
    let ini = rt::Ini::open(&rt::game_path("Display8.ini"));
    let cfg = Config {
        log: ini.int("Display8", "Log", 1) != 0,
        trace: ini.int("Display8", "Trace", 0) != 0,
        real: ini.string("Display8", "Real").filter(|s| !s.is_empty()).unwrap_or_else(|| alloc::string::String::from("d3d8_real.dll")),
    };
    rt::log_open("Display8.log", cfg.log || cfg.trace);
    log!("Display8 {} -- game dir {}; forward to {}", env!("CARGO_PKG_VERSION"), rt::game_dir(), cfg.real);
    unsafe { CONFIG = Some(cfg) };
    d3d8::install();
    drop::load(&ini);
    detail::configure(&ini);
    if config().trace {
        trace::start();
        user32::install();
    }
}

/// The real chain's `d3d8.dll`, loaded on first use (not in `DllMain`).
fn real() -> HANDLE {
    unsafe {
        if REAL.is_null() {
            let p = rt::cstr(&rt::game_path(&config().real));
            REAL = LoadLibraryA(p.as_ptr());
            if REAL.is_null() {
                log!("cannot load {}: no Direct3D for the game", config().real);
            }
        }
        REAL
    }
}

fn real_proc(name: &str) -> usize {
    let m = real();
    if m.is_null() {
        return 0;
    }
    let c = rt::cstr(name);
    unsafe { GetProcAddress(m, c.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "system" fn Direct3DCreate8(sdk: u32) -> *mut c_void {
    let f = real_proc("Direct3DCreate8");
    if f == 0 {
        return core::ptr::null_mut();
    }
    let f: unsafe extern "system" fn(u32) -> *mut c_void = unsafe { core::mem::transmute(f) };
    let d3d = unsafe { f(sdk) };
    log!("Direct3DCreate8({}) -> {:#x}", sdk, d3d as *const () as usize);
    com::wrap(d3d, com::Kind::D3D, com::Role::Plain) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "system" fn ValidatePixelShader(a: *const u32, b: *const u32, c: i32, d: *mut c_void) -> i32 {
    match real_proc("ValidatePixelShader") {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, unsafe extern "system" fn(*const u32, *const u32, i32, *mut c_void) -> i32>(f)(a, b, c, d) },
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn ValidateVertexShader(a: *const u32, b: *const u32, c: *const u32, d: i32, e: *mut c_void) -> i32 {
    match real_proc("ValidateVertexShader") {
        0 => -1,
        f => unsafe {
            core::mem::transmute::<usize, unsafe extern "system" fn(*const u32, *const u32, *const u32, i32, *mut c_void) -> i32>(f)(a, b, c, d, e)
        },
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DebugSetMute() {
    if let f @ 1.. = real_proc("DebugSetMute") {
        unsafe { core::mem::transmute::<usize, unsafe extern "system" fn()>(f)() }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    match reason {
        1 => startup(),
        0 => {
            if config().trace {
                trace::write_inventory();
                log!("exit: {} objects of ours alive", com::live());
            }
        }
        _ => {}
    }
    1
}
