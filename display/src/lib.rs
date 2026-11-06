//! Display8: Star Trek: Armada II's `d3d8.dll`, ours. The engine gets Direct3D 8
//! objects of ours (`stub/`) that draw nothing themselves, and render3d and the UI draw
//! the whole frame on a device of Display8's own, on Vulkan (`vk/`) or Direct3D 12
//! (`dx12/`); the engine's own draws are drawn by the passthrough (`passthrough/`). On
//! the bench it traces every call (`Trace=1`) and drops the engine's draws by category
//! (`[Drop]`). When it cannot draw the game itself, the game gets the system's own
//! Direct3D 8 and the plugins stand down (`bail.rs`). See README.md beside this crate.
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
pub mod bail;
pub mod claims;
pub mod crash;
pub mod com;
pub mod d3d8;
pub mod detail;
pub mod drop;
#[allow(dead_code)]
#[rustfmt::skip]
pub mod engine_syms;
pub mod passthrough;
pub mod dx12;
pub mod trace;
pub mod stub;
pub mod user32;
pub mod vk;
pub mod window;
#[rustfmt::skip]
pub mod settings_table;

use core::ffi::c_void;
use rt::win::*;

#[global_allocator]
static HEAP: rt::mem::Heap = rt::mem::Heap;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    rt::log_str("Display8 stopped the game: a bug in d3d8.dll. Set [Display8] Backend=none in Display8.ini (or take d3d8.dll out) to play without it.");
    unsafe { ExitProcess(3) }
}

struct Config {
    log: bool,
    trace: bool,
}

static mut CONFIG: Option<Config> = None;
/// `Backend=`: the backends the stub tries, in order, at the engine's first
/// `CreateDevice` (`auto` by default, as the shipped `Display8.ini`; README.md,
/// "Choosing a backend"). Empty: `Backend=none`, the bail-out.
static mut ORDER: alloc::vec::Vec<stub::Backend> = alloc::vec::Vec::new();
/// `FailBackend=`: backends that fail on purpose once made (a test bench clone only).
static mut FAIL: alloc::vec::Vec<stub::Backend> = alloc::vec::Vec::new();
static mut VALIDATE: bool = false;
/// `Device=`: on Vulkan, only a physical device whose name contains this (any case);
/// empty: a discrete one first (README.md, "Vulkan").
static mut VK_DEVICE: alloc::string::String = alloc::string::String::new();

/// `PixelCentres=d3d9`: the plugins draw with Direct3D 9's pixel centres (a convention,
/// not the backend: kept for comparing frames with old ones).
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

/// The plugins' samplers and the passthrough's (README.md, "Samplers"): anisotropy and
/// LOD bias, from `Display8.ini`'s `[Samplers]` keys `Anisotropy`, `LodBias` and
/// `ClampNegativeLodBias`. `auto` (the shipped value) or no key is the default: 16, -0.5
/// and no clamp, what the game's `dxvk.conf` (stage 3) had DXVK make of every sampler on
/// Direct3D 9 before 15.0.0, so the pictures agree. A negative bias is held at 0 while the
/// clamp holds. One log line says where each value came from.
pub fn dxvk_samplers() -> (i32, f32) {
    let ini = rt::Ini::open(&rt::game_path("Display8.ini"));
    let own = |key: &str| ini.string("Samplers", key).filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("auto"));
    let (o_aniso, o_bias, o_clamp) = (own("Anisotropy"), own("LodBias"), own("ClampNegativeLodBias"));
    let aniso = o_aniso.as_deref().map_or(16, |v| rt::parse_f32(v).map_or(-1, |f| f as i32));
    let mut bias = o_bias.as_deref().map_or(-0.5, |v| rt::parse_f32(v).unwrap_or(0.0));
    let clamp = o_clamp.as_deref().is_some_and(|v| !(v == "0" || v.eq_ignore_ascii_case("false")));
    let from = |o: &Option<alloc::string::String>| if o.is_some() { "Display8.ini" } else { "default" };
    log!("samplers: Anisotropy={} ({}), LodBias={} ({}), ClampNegativeLodBias={} ({})",
        aniso, from(&o_aniso), bias, from(&o_bias), clamp as i32, from(&o_clamp));
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

/// `Backend=auto`'s order: Vulkan, then Direct3D 12, on every system (the user's
/// decision of 2026-10-23, over the display plan's Direct3D 12 first on Windows).
/// `platform` is only logged.
fn auto_order(_platform: &str) -> alloc::vec::Vec<stub::Backend> {
    use stub::Backend::*;
    alloc::vec![Vulkan, D3d12]
}

/// `Backend=`: `auto`, `vulkan`, `d3d12`, `none`, or a list (`d3d12,vulkan`), tried in
/// that order; when none of them starts, the bail-out (`bail.rs`). `none` ends the list:
/// nothing after it is tried, and `Backend=none` is the bail-out at once. The retired
/// `d3d9` (display 1.0.0) is skipped, as is any unknown name; a list left empty is
/// `auto`.
fn backends(value: &str) -> alloc::vec::Vec<stub::Backend> {
    let mut order = alloc::vec::Vec::new();
    let mut named = false;
    for word in value.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        if word.eq_ignore_ascii_case("none") {
            log!("Backend=none: {}the bail-out (the system's Direct3D 8)", if order.is_empty() { "" } else { "then " });
            return order;
        }
        let more = if word.eq_ignore_ascii_case("auto") {
            let p = platform();
            let o = auto_order(p);
            log!("Backend=auto: on {}: {}", p, stub::names(&o));
            o
        } else if word.eq_ignore_ascii_case("d3d9") {
            log!("settings: Backend token d3d9 retired (Direct3D 9 removed)");
            continue;
        } else {
            match stub::Backend::parse(word) {
                Some(b) => alloc::vec![b],
                None => {
                    log!("Backend={}: not a backend of this version (auto, vulkan, d3d12, none); skipped", word);
                    continue;
                }
            }
        };
        named = true;
        for b in more {
            if !order.contains(&b) {
                order.push(b);
            }
        }
    }
    if !named {
        let p = platform();
        order = auto_order(p);
        log!("Backend= names no backend: auto, on {}: {}", p, stub::names(&order));
    }
    order
}

/// The plugins that change what the engine draws, which in stub mode would draw
/// nothing (README.md, "Stub mode"): with any loaded, the stub stands down.
const ENGINE_PLUGINS: [&str; 3] = ["HUD.asi", "GridLayout.asi", "Menus.asi"];

fn engine_plugin() -> Option<&'static str> {
    ENGINE_PLUGINS.into_iter().find(|p| !unsafe { GetModuleHandleA(rt::cstr(p).as_ptr()) }.is_null())
}

/// The stub offers one mode, so it needs `display.modes` (stub/modes.rs): without it the
/// engine's own requests find no mode, the shell's display opens on none and the game
/// dies of a stack overflow before its first `CreateDevice` (measured with
/// `A2EngineTest.ini` `Mismatch=display.modes`, README.md, "Hooks"). So on a build this
/// Display8 does not know, or with that hook stood down, the bail-out (p8-e7): why, or
/// None when the stub can run.
fn stub_modes() -> Option<&'static str> {
    let e = engine();
    if e.build().is_none() {
        return Some("engine: unknown build");
    }
    if e.ready(engine_syms::hook::MODES).is_err() {
        return Some("without display.modes the stub could open no display");
    }
    None
}

fn engine_log(line: &str) {
    log!("{}", line);
}

/// Display8's symbols in this `Armada2.exe` (`engine_syms.rs`, from
/// `profiles/<build>/display.toml`; README.md, "Hooks").
pub fn engine() -> &'static a2engine::Engine {
    engine_syms::TABLE.engine()
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
    };
    rt::log_open("Display8.log", cfg.log || cfg.trace);
    log!("Display8 {} -- game dir {}", env!("CARGO_PKG_VERSION"), rt::game_dir());
    // The identity line, the build line and the symbols summary (crates/a2engine).
    a2engine::init(&engine_syms::TABLE, engine_log);
    // Retired in display 1.0.0 (forward mode and Direct3D 9 went): read past, said once.
    if let Some(m) = ini.string("Display8", "Mode") {
        log!("settings: Mode={} retired (forward mode removed); ignored", m);
    }
    if ini.string("Display8", "Real").is_some() {
        log!("settings: Real= retired (forward mode removed); ignored");
    }
    let backend = ini.string("Display8", "Backend").unwrap_or_else(|| alloc::string::String::from("auto"));
    let fail = ini.string("Display8", "FailBackend").unwrap_or_default();
    unsafe {
        ORDER = backends(&backend);
        for word in fail.split(',').map(str::trim).filter(|w| !w.is_empty()) {
            match stub::Backend::parse(word) {
                Some(b) if bench_clone() => FAIL.push(b),
                Some(_) => log!("FailBackend={}: for the test bench only; ignored outside a bench clone", word),
                None => log!("FailBackend={}: not a backend (vulkan, d3d12); ignored", word),
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
    if !unsafe { ORDER.is_empty() } {
        log!("stub on {}{}", stub::names(unsafe { &ORDER }), if unsafe { VALIDATE } { ", with the validation layer" } else { "" });
    }
    unsafe { CONFIG = Some(cfg) };
    drop::load(&ini);
    passthrough::configure(&ini);
    detail::configure(&ini);
    window::configure(&ini);
    crash::configure(&ini);
    if config().trace {
        trace::start();
        user32::install();
    }
}

/// Why the stub cannot run, or None: `Backend=none`, a build or hook it needs missing, a
/// legacy plugin loaded, or no backend of the order that can start (the probes). The
/// probes drop what cannot start from the order.
fn bail_reason() -> Option<alloc::string::String> {
    if unsafe { ORDER.is_empty() } {
        return Some(alloc::string::String::from("Backend=none"));
    }
    if let Some(p) = engine_plugin() {
        return Some(sfmt!("{} is loaded and changes what the engine draws, which the stub drops", p));
    }
    if let Some(why) = stub_modes() {
        return Some(alloc::string::String::from(why));
    }
    let mut failed = alloc::string::String::new();
    unsafe {
        ORDER.retain(|&b| {
            let r = match b {
                stub::Backend::Vulkan => vk::probe(),
                stub::Backend::D3d12 => dx12::probe(),
            };
            match r {
                Ok(()) => log!("probe: {} can start", b.name()),
                Err(e) => {
                    log!("probe: {} cannot start: {}", b.name(), e);
                    if !failed.is_empty() {
                        failed.push_str(", ");
                    }
                    failed.push_str(&sfmt!("{}: {}", b.name(), e));
                }
            }
            r.is_ok()
        });
        if ORDER.is_empty() {
            return Some(sfmt!("no backend can start ({})", failed));
        }
    }
    None
}

#[unsafe(no_mangle)]
pub extern "system" fn Direct3DCreate8(sdk: u32) -> *mut c_void {
    // A second call never starts the stub again: `stub::start` would put the stub's
    // methods back into the `IDirect3D8` vtable a late bail-out made a forward of, and
    // begin a new `Stub` over the one whose device exists.
    if bail::taken() && stub::active() {
        return bail::again(sdk);
    }
    if stub::active() {
        log!("Direct3DCreate8({}) again -> another of the stub's", sdk);
        return stub::another();
    }
    bail::note_sdk(sdk);
    if let Some(why) = bail_reason() {
        return bail::early(sdk, &why);
    }
    match stub::start(unsafe { ORDER.clone() }, unsafe { VALIDATE }) {
        Some(d) => {
            log!("Direct3DCreate8({}) -> the stub: the engine draws nothing from here", sdk);
            d
        }
        None => bail::early(sdk, "the stub could not start"),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn ValidatePixelShader(a: *const u32, b: *const u32, c: i32, d: *mut c_void) -> i32 {
    match bail::proc("ValidatePixelShader") {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, unsafe extern "system" fn(*const u32, *const u32, i32, *mut c_void) -> i32>(f)(a, b, c, d) },
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn ValidateVertexShader(a: *const u32, b: *const u32, c: *const u32, d: i32, e: *mut c_void) -> i32 {
    match bail::proc("ValidateVertexShader") {
        0 => -1,
        f => unsafe {
            core::mem::transmute::<usize, unsafe extern "system" fn(*const u32, *const u32, *const u32, i32, *mut c_void) -> i32>(f)(a, b, c, d, e)
        },
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DebugSetMute() {
    if let f @ 1.. = bail::proc("DebugSetMute") {
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
            }
        }
        _ => {}
    }
    1
}
