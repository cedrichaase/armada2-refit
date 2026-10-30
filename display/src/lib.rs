//! Display8: Star Trek: Armada II's `d3d8.dll`, ours. In forward mode (the default) it
//! stands in front of the real chain (`d3d8_real.dll`: crosire's d3d8to9, then DXVK's or
//! Windows' d3d9) and passes every call through, tracing it on the bench (`Trace=1`),
//! dropping the engine's draws by category (`[Drop]`), and telling the plugins of the
//! device's comings and goings (`api.rs`). In stub mode (`stub/`) the engine draws
//! nothing, and render3d and the UI draw the whole frame on a device of ours: Direct3D 9,
//! Vulkan (`vk/`) or Direct3D 12 (`dx12/`). See README.md beside this crate.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

#[macro_use]
pub mod macros;
pub mod rt {
    pub use a2rt::*;
}
pub use a2rt::{log, sfmt};
#[rustfmt::skip]
pub mod shaders;
pub mod api;
pub mod com;
pub mod d3d8;
pub mod detail;
pub mod drop;
pub mod passthrough;
pub mod dx12;
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
/// `Backend=`: the backends the stub tries, in order, at the engine's first
/// `CreateDevice` (d3d9 alone by default, until a backend is confirmed in game; README.md,
/// "Choosing a backend").
static mut ORDER: alloc::vec::Vec<stub::Backend> = alloc::vec::Vec::new();
/// `FailBackend=`: backends that fail on purpose once made (a test bench clone only).
static mut FAIL: alloc::vec::Vec<stub::Backend> = alloc::vec::Vec::new();
static mut VALIDATE: bool = false;
/// `Device=`: on Vulkan, only a physical device whose name contains this (any case);
/// empty: a discrete one first (README.md, "Vulkan").
static mut VK_DEVICE: alloc::string::String = alloc::string::String::new();

/// `PixelCentres=d3d9`: on Vulkan, the plugins draw with Direct3D 9's pixel centres.
static mut D3D9_CENTRES: bool = false;

pub fn d3d9_centres() -> bool {
    unsafe { D3D9_CENTRES }
}

/// `VSync=0`: present without waiting for the vertical blank, so the frame rate is not
/// held to the display's refresh rate (README.md, "VSync").
static mut VSYNC: bool = true;

/// `D3DPRESENT_INTERVAL_IMMEDIATE`.
const INTERVAL_IMMEDIATE: u32 = 0x8000_0000;

/// The presentation interval to make a device with, for the engine's `asked`: the
/// engine's own, or IMMEDIATE under `VSync=0`. A windowed Direct3D 8 device keeps the
/// engine's: Direct3D 8 takes only the default there.
pub fn present_interval(asked: u32, windowed: bool) -> u32 {
    if unsafe { VSYNC } || windowed { asked } else { INTERVAL_IMMEDIATE }
}

/// What the game's `dxvk.conf` makes of every sampler on Direct3D 9 (renderer/README.md):
/// `d3d9.samplerAnisotropy` (-1 when not set) and `d3d9.samplerLodBias` (0 when not set;
/// held at 0 or above when `d3d9.clampNegativeLodBias` is true, DXVK's default). On
/// Vulkan the plugins' samplers are given the same, so the two backends' pictures agree.
pub fn dxvk_samplers() -> (i32, f32) {
    let text = rt::read_file(&rt::game_path("dxvk.conf")).unwrap_or_default();
    let text = alloc::string::String::from_utf8_lossy(&text);
    let (mut aniso, mut bias, mut clamp) = (-1i32, 0f32, true);
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if k.eq_ignore_ascii_case("d3d9.samplerAnisotropy") {
            aniso = rt::parse_f32(v).map_or(-1, |f| f as i32);
        } else if k.eq_ignore_ascii_case("d3d9.samplerLodBias") {
            bias = rt::parse_f32(v).unwrap_or(0.0);
        } else if k.eq_ignore_ascii_case("d3d9.clampNegativeLodBias") {
            clamp = !v.eq_ignore_ascii_case("false");
        }
    }
    if clamp && bias < 0.0 {
        bias = 0.0;
    }
    (aniso, bias)
}

/// `Device=` (empty: any).
#[allow(static_mut_refs)]
pub fn vk_device() -> &'static str {
    unsafe { VK_DEVICE.as_str() }
}

/// Whether the game runs in a test bench clone (a game directory under
/// `a2test\sessions\`).
pub fn bench_clone() -> bool {
    rt::lower(rt::game_dir()).contains("\\a2test\\sessions\\")
}

/// `FailBackend=` names `b`: its start fails on purpose once everything is made.
pub fn fail_backend(b: stub::Backend) -> bool {
    unsafe { FAIL.contains(&b) }
}

/// The system the game runs on, for `Backend=auto`: Windows, or Wine on Linux or macOS
/// (`wine_get_host_version` in Wine's `ntdll.dll`).
fn platform() -> &'static str {
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr() as *const u8);
        if ntdll.is_null() {
            return "Windows";
        }
        let host = GetProcAddress(ntdll, c"wine_get_host_version".as_ptr() as *const u8);
        if host == 0 {
            return if GetProcAddress(ntdll, c"wine_get_version".as_ptr() as *const u8) != 0 { "Linux" } else { "Windows" };
        }
        let host: unsafe extern "C" fn(*mut *const u8, *mut *const u8) = core::mem::transmute(host);
        let (mut sys, mut rel) = (core::ptr::null::<u8>(), core::ptr::null::<u8>());
        host(&mut sys, &mut rel);
        if !sys.is_null() && rt::cstr_at(sys, 64) == "Darwin" { "macOS" } else { "Linux" }
    }
}

/// `Backend=auto`'s order: Vulkan, then Direct3D 12, then Direct3D 9, on every system (the
/// user's decision of 2026-10-23, over the display plan's Direct3D 12 first on Windows).
/// `platform` is only logged.
fn auto_order(_platform: &str) -> alloc::vec::Vec<stub::Backend> {
    use stub::Backend::*;
    alloc::vec![Vulkan, D3d12, D3d9]
}

/// `Backend=`: `auto`, one backend, or a list (`d3d12,vulkan`), tried in that order;
/// Direct3D 9 is always the last, named or not, so that a backend that cannot start never
/// fails the game. Unknown names are logged and skipped.
fn backends(value: &str) -> alloc::vec::Vec<stub::Backend> {
    let mut order = alloc::vec::Vec::new();
    for word in value.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        let more = if word.eq_ignore_ascii_case("auto") {
            let p = platform();
            let o = auto_order(p);
            log!("Backend=auto: on {}: {}", p, stub::names(&o));
            o
        } else {
            match stub::Backend::parse(word) {
                Some(b) => alloc::vec![b],
                None => {
                    log!("Backend={}: not a backend of this version (auto, d3d9, vulkan, d3d12); skipped", word);
                    alloc::vec::Vec::new()
                }
            }
        };
        for b in more {
            if !order.contains(&b) {
                order.push(b);
            }
        }
    }
    // Direct3D 9 last: what every system has while d3d9 lives.
    order.retain(|&b| b != stub::Backend::D3d9);
    order.push(stub::Backend::D3d9);
    order
}

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
            _ => bench_clone(),
        },
        real: real_path(ini.string("Display8", "Real").filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))),
    };
    rt::log_open("Display8.log", cfg.log || cfg.trace);
    let mode = ini.string("Display8", "Mode").unwrap_or_else(|| alloc::string::String::from("forward"));
    log!("Display8 {} -- game dir {}; mode {}; the real chain: {}", env!("CARGO_PKG_VERSION"), rt::game_dir(), mode, cfg.real);
    unsafe { STUB_ASKED = mode.eq_ignore_ascii_case("stub") };
    let backend = ini.string("Display8", "Backend").unwrap_or_else(|| alloc::string::String::from("d3d9"));
    let fail = ini.string("Display8", "FailBackend").unwrap_or_default();
    unsafe {
        if STUB_ASKED {
            ORDER = backends(&backend);
        } else {
            ORDER = alloc::vec![stub::Backend::D3d9];
        }
        for word in fail.split(',').map(str::trim).filter(|w| !w.is_empty()) {
            match stub::Backend::parse(word) {
                Some(b) if bench_clone() => FAIL.push(b),
                Some(_) => log!("FailBackend={}: for the test bench only; ignored outside a bench clone", word),
                None => log!("FailBackend={}: not a backend (d3d9, vulkan, d3d12); ignored", word),
            }
        }
        if !FAIL.is_empty() {
            log!("FailBackend={}: these fail on purpose once made (the test bench)", stub::names(&FAIL));
        }
        VALIDATE = ini.int("Display8", "Validate", 0) != 0;
        VK_DEVICE = ini.string("Display8", "Device").unwrap_or_default();
        D3D9_CENTRES = ini.string("Display8", "PixelCentres").is_some_and(|s| s.eq_ignore_ascii_case("d3d9"));
        VSYNC = ini.int("Display8", "VSync", 1) != 0;
        if !VSYNC {
            log!("VSync=0: frames are presented without waiting for the vertical blank");
        }
    }
    if STUB_ASKED_NOW() {
        log!("stub mode on {}{}", stub::names(unsafe { &ORDER }), if unsafe { VALIDATE } { ", with the validation layer" } else { "" });
    }
    if !mode.eq_ignore_ascii_case("forward") && !mode.eq_ignore_ascii_case("stub") {
        log!("Mode={}: not a mode of this version (forward, stub); forward it is", mode);
    }
    unsafe { CONFIG = Some(cfg) };
    d3d8::install();
    drop::load(&ini);
    if STUB_ASKED_NOW() {
        passthrough::configure(&ini);
    }
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
        if let Some(d) = stub::start(unsafe { ORDER.clone() }, unsafe { VALIDATE }) {
            log!("Direct3DCreate8({}) -> the stub: the engine draws nothing from here", sdk);
            return d;
        }
        log!("Mode=stub: the stub could not start on {}; forward mode instead", stub::names(unsafe { &ORDER }));
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
