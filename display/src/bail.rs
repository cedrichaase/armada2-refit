//! The bail-out: when Display8 cannot draw the game itself, the game gets the system's
//! own Direct3D 8 (`GetSystemDirectoryA`, SysWOW64 under WoW64: Windows' `d3d8.dll`, or
//! Wine's builtin), and the plugins stand down, because `display8v1_backend` answers
//! none (README.md, "The bail-out"). It never loads a `d3d8_real.dll`.
//!
//! - **Early**, at `Direct3DCreate8`: `Backend=none`, an unknown build, a stood-down
//!   `display.modes` hook, a legacy plugin that changes what the engine draws, or neither
//!   Vulkan nor Direct3D 12 can start (`vk::probe`, `dx12::probe`). The engine gets the
//!   system's `IDirect3D8` itself.
//! - **Late**, at the engine's first `CreateDevice`, when every backend of `Backend=`'s
//!   order failed to start there (a swap chain the probe could not foresee, or
//!   `FailBackend=` on the bench): the stub's `IDirect3D8` becomes a thin forward of the
//!   system's, its 16 methods passed through, and the engine gets the system's device
//!   itself.
use crate::rt::win::*;
use crate::{log, rt};
use alloc::string::String;
use core::ffi::c_void;

type P = *mut c_void;
type HRESULT = i32;

static mut SYSTEM: HANDLE = core::ptr::null_mut();
static mut TAKEN: bool = false;
/// `Direct3DCreate8`'s SDK version, for the late path's own call.
static mut SDK: u32 = 220;
/// The system's `IDirect3D8` behind the stub's after a late bail-out.
static mut REAL: P = core::ptr::null_mut();

/// Whether the bail-out was taken.
pub fn taken() -> bool {
    unsafe { TAKEN }
}

/// The system's `d3d8.dll`: `GetSystemDirectoryA` and the name.
pub fn system_path() -> String {
    let mut buf = [0u8; 300];
    let n = unsafe { GetSystemDirectoryA(buf.as_mut_ptr(), 280) } as usize;
    let mut s = String::from(core::str::from_utf8(&buf[..n.min(280)]).unwrap_or("C:\\windows\\system32"));
    s.push_str("\\d3d8.dll");
    s
}

fn system() -> HANDLE {
    unsafe {
        if SYSTEM.is_null() {
            let p = system_path();
            SYSTEM = LoadLibraryA(rt::cstr(&p).as_ptr());
            if SYSTEM.is_null() {
                log!("bail-out: cannot load {}: no Direct3D for the game", p);
            }
        }
        SYSTEM
    }
}

/// An export of the system's `d3d8.dll`, or 0.
pub fn proc(name: &str) -> usize {
    let m = system();
    if m.is_null() {
        return 0;
    }
    unsafe { GetProcAddress(m, rt::cstr(name).as_ptr()) }
}

fn create(sdk: u32) -> P {
    let f = proc("Direct3DCreate8");
    if f == 0 {
        return core::ptr::null_mut();
    }
    let f: unsafe extern "system" fn(u32) -> P = unsafe { core::mem::transmute(f) };
    unsafe { f(sdk) }
}

fn say(why: &str) {
    unsafe { TAKEN = true };
    log!("bail-out: {}; the game draws through {}; the plugins stand down", why, system_path());
}

/// Remembers `Direct3DCreate8`'s SDK version for a late bail-out.
pub fn note_sdk(sdk: u32) {
    unsafe { SDK = sdk };
}

/// The early bail-out: the system's `IDirect3D8`, unwrapped. The engine's mode requests
/// are still answered where `display.modes` is ready (the shell's and the intro's raise,
/// `stub/modes.rs`), by the system's own list.
pub fn early(sdk: u32, why: &str) -> P {
    say(why);
    crate::stub::modes::install();
    let d = create(sdk);
    log!("Direct3DCreate8({}) -> the system's {:#x}", sdk, d as usize);
    d
}

/// A `Direct3DCreate8` after a late bail-out: the system's `IDirect3D8` itself. The
/// stub's vtables stay the forward `late` made of them.
pub fn again(sdk: u32) -> P {
    let d = create(sdk);
    log!("Direct3DCreate8({}) after the bail-out -> the system's {:#x}", sdk, d as usize);
    d
}

/// The late bail-out, from the stub's `CreateDevice`: the stub's `IDirect3D8` forwards
/// to the system's from now on, and the engine's `CreateDevice` is the system's.
pub fn late(adapter: u32, dt: u32, hwnd: P, flags: u32, pp: *mut crate::d3d8::PresentParams, out: *mut P) -> HRESULT {
    const D3DERR_NOTAVAILABLE: HRESULT = 0x8876_086au32 as i32;
    say("no backend could start at the engine's first CreateDevice");
    let real = create(unsafe { SDK });
    if real.is_null() {
        log!("bail-out: the system's Direct3DCreate8 failed: the engine's CreateDevice fails");
        return D3DERR_NOTAVAILABLE;
    }
    unsafe { REAL = real };
    crate::stub::obj::install(crate::com::Kind::D3D, &FORWARD.map(|(s, f)| (s, f as usize)));
    let hr = unsafe { create_device(core::ptr::null_mut(), adapter, dt, hwnd, flags, pp, out) };
    log!("bail-out: the system's CreateDevice -> {:#x}, device {:#x}", hr as u32, if out.is_null() { 0 } else { unsafe { *out as usize } });
    hr
}

macro_rules! forward {
    ($($slot:literal $name:ident($($a:ident: $t:ty),*) -> $r:ty;)*) => {
        $(
            unsafe extern "system" fn $name(_this: P $(, $a: $t)*) -> $r {
                vcall!(unsafe { REAL }, $slot, fn($($t),*) -> $r, $($a),*)
            }
        )*
        const FORWARD: [(usize, *const ()); 16] = [$(($slot, $name as *const ())),*];
    };
}

forward! {
    0 query_interface(iid: *const crate::com::Guid, out: *mut P) -> HRESULT;
    1 add_ref() -> u32;
    2 release() -> u32;
    3 register_software_device(f: P) -> HRESULT;
    4 get_adapter_count() -> u32;
    5 get_adapter_identifier(adapter: u32, flags: u32, id: P) -> HRESULT;
    6 get_adapter_mode_count(adapter: u32) -> u32;
    7 enum_adapter_modes(adapter: u32, mode: u32, out: P) -> HRESULT;
    8 get_adapter_display_mode(adapter: u32, out: P) -> HRESULT;
    9 check_device_type(adapter: u32, dt: u32, disp: u32, bb: u32, windowed: i32) -> HRESULT;
    10 check_device_format(adapter: u32, dt: u32, afmt: u32, usage: u32, rtype: u32, fmt: u32) -> HRESULT;
    11 check_device_multisample_type(adapter: u32, dt: u32, fmt: u32, windowed: i32, ms: u32) -> HRESULT;
    12 check_depth_stencil_match(adapter: u32, dt: u32, afmt: u32, rt: u32, ds: u32) -> HRESULT;
    13 get_device_caps(adapter: u32, dt: u32, caps: P) -> HRESULT;
    14 get_adapter_monitor(adapter: u32) -> usize;
    15 create_device(adapter: u32, dt: u32, hwnd: P, flags: u32, pp: *mut crate::d3d8::PresentParams, out: *mut P) -> HRESULT;
}
