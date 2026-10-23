//! Display8: Star Trek: Armada II's `d3d8.dll`, ours. In forward mode, the only one so
//! far, it stands in front of the real chain (`d3d8_real.dll`: crosire's d3d8to9, then
//! DXVK's or Windows' d3d9) and passes every call through, tracing it on the bench
//! (`Trace=1`), dropping the engine's draws by category (`[Drop]`), and telling the
//! plugins of the device's comings and goings (`api.rs`). See README.md beside this
//! crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

#[macro_use]
pub mod macros;
pub mod rt;
#[rustfmt::skip]
pub mod shaders;
pub mod api;
pub mod com;
pub mod d3d8;
pub mod detail;
pub mod drop;
pub mod trace;
pub mod stub;
pub mod user32;
pub mod vk;

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
/// `Mode=stub` asked for; whether it runs is decided at `Direct3DCreate8`.
static mut STUB_ASKED: bool = false;
/// `Backend=`: what the stub draws on (d3d9 until every plugin draws on Vulkan).
static mut BACKEND: stub::Backend = stub::Backend::D3d9;
static mut VALIDATE: bool = false;

#[allow(non_snake_case)]
fn STUB_ASKED_NOW() -> bool {
    unsafe { STUB_ASKED }
}

/// The plugins that change what the engine draws, which in stub mode would draw
/// nothing (README.md, "Stub mode"): with any loaded, the stub stands down.
const ENGINE_PLUGINS: [&str; 3] = ["HUD.asi", "GridLayout.asi", "Menus.asi"];

fn stub_allowed() -> bool {
    for p in ENGINE_PLUGINS {
        if !unsafe { GetModuleHandleA(rt::cstr(p).as_ptr()) }.is_null() {
            log!("Mode=stub: {} is loaded and changes what the engine draws, which the stub drops; forward mode instead", p);
            return false;
        }
    }
    true
}

fn config() -> &'static Config {
    unsafe { CONFIG.as_ref().expect("config") }
}

fn startup() {
    rt::init();
    let ini = rt::Ini::open(&rt::game_path("Display8.ini"));
    let cfg = Config {
        log: ini.int("Display8", "Log", 1) != 0,
        trace: match ini.string("Display8", "Trace").as_deref().map(|s| s.to_ascii_lowercase()) {
            Some(ref s) if s == "1" => true,
            Some(ref s) if s == "0" => false,
            // auto: on in a test bench clone, which is where the inventory is wanted.
            _ => rt::lower(rt::game_dir()).contains("\\a2test\\sessions\\"),
        },
        real: real_path(ini.string("Display8", "Real").filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))),
    };
    rt::log_open("Display8.log", cfg.log || cfg.trace);
    let mode = ini.string("Display8", "Mode").unwrap_or_else(|| alloc::string::String::from("forward"));
    log!("Display8 {} -- game dir {}; mode {}; the real chain: {}", env!("CARGO_PKG_VERSION"), rt::game_dir(), mode, cfg.real);
    unsafe { STUB_ASKED = mode.eq_ignore_ascii_case("stub") };
    let backend = ini.string("Display8", "Backend").unwrap_or_else(|| alloc::string::String::from("d3d9"));
    unsafe {
        BACKEND = if backend.eq_ignore_ascii_case("vulkan") { stub::Backend::Vulkan } else { stub::Backend::D3d9 };
        VALIDATE = ini.int("Display8", "Validate", 0) != 0;
    }
    if STUB_ASKED_NOW() {
        log!("stub mode on {:?}{}", unsafe { BACKEND }, if unsafe { VALIDATE } { ", with the validation layer" } else { "" });
    }
    if !mode.eq_ignore_ascii_case("forward") && !mode.eq_ignore_ascii_case("stub") {
        log!("Mode={}: not a mode of this version (forward, stub); forward it is", mode);
    }
    unsafe { CONFIG = Some(cfg) };
    d3d8::install();
    drop::load(&ini);
    detail::configure(&ini);
    if config().trace {
        trace::start();
        user32::install();
    }
}

/// The real chain's `d3d8.dll`: `Real=` (a name in the game directory, or a full
/// path), else `d3d8_real.dll` beside the exe if there is one (what `display/install`
/// puts there), else the system's own (`GetSystemDirectory`: SysWOW64 for a 32-bit
/// process on 64-bit Windows, Wine's builtin under Wine).
fn real_path(set: Option<alloc::string::String>) -> alloc::string::String {
    if let Some(s) = set {
        return if s.contains(':') || s.starts_with('\\') { s } else { rt::game_path(&s) };
    }
    let local = rt::game_path("d3d8_real.dll");
    if rt::file_exists(&local) {
        return local;
    }
    let mut buf = [0u8; 300];
    let n = unsafe { GetSystemDirectoryA(buf.as_mut_ptr(), 280) } as usize;
    let mut s = alloc::string::String::from(core::str::from_utf8(&buf[..n.min(280)]).unwrap_or("C:\\windows\\system32"));
    s.push_str("\\d3d8.dll");
    s
}

/// The real chain's `d3d8.dll`, loaded on first use (not in `DllMain`).
fn real() -> HANDLE {
    unsafe {
        if REAL.is_null() {
            let p = rt::cstr(&config().real);
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
    if unsafe { STUB_ASKED } && stub_allowed() {
        if let Some(d) = stub::start(unsafe { BACKEND }, unsafe { VALIDATE }) {
            log!("Direct3DCreate8({}) -> the stub: the engine draws nothing from here", sdk);
            return d;
        }
        log!("Mode=stub: no Direct3D 9 for the stub; forward mode instead");
    }
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
