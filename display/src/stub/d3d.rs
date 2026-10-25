//! The stub's `IDirect3D8`: one adapter, one mode (the desktop's), and the answers
//! DXVK behind crosire's d3d8to9 gave in phase 0 (display/README.md, "Stub mode"), so
//! the engine takes the code paths it takes on the real chain.
use super::obj::{self, SObj};
use super::stub;
use crate::com::{Guid, Kind, HRESULT, S_OK};
use crate::d3d8::{DisplayMode, PresentParams, CAPS8_SIZE, HWND};
use crate::trace::{self, once};
use crate::{log, sfmt};
use core::ffi::c_void;

pub const D3DERR_NOTAVAILABLE: HRESULT = 0x8876_086au32 as i32;
pub const D3DERR_INVALIDCALL: HRESULT = 0x8876_086cu32 as i32;

/// `IDirect3D8::GetDeviceCaps` for a HAL device as DXVK 3.1 behind crosire's d3d8to9
/// 1.16.0 answered it on the bench (Display8.log, `ANSWER IDirect3D8::GetDeviceCaps`),
/// `D3DCAPS8` as 53 little-endian words.
pub const CAPS: [u32; 53] = [
    0x00000001, 0x00000000, 0x00020000, 0x600a0000, 0x000003a0, 0x8000000f, 0x00000001, 0x001bbef0, 0x003fcff2,
    0x0f736191, 0x000000ff, 0x00003fff, 0x00003fff, 0x000000ff, 0x00084208, 0x0001ecc5, 0x07030700, 0x07030700,
    0x07030700, 0x0000003f, 0x0000003f, 0x0000003f, 0x00004000, 0x00004000, 0x00002000, 0x00002000, 0x00002000,
    0x00000010, 0x501502f9, 0xc7000000, 0xc7000000, 0x47000000, 0x47000000, 0x00000000, 0x000000ff, 0x00100008,
    0x03ffffff, 0x00000008, 0x00000008, 0x0000017b, 0x00000008, 0x00000006, 0x00000004, 0x00000000, 0x45ffff00,
    0x00555555, 0x00ffffff, 0x00000010, 0x000001fc, 0xfffe0101, 0x00000100, 0xffff0104, 0x7f7fffff,
];

/// Formats the stub takes as textures (`CheckDeviceFormat`): what the engine asked for
/// and the chain said yes to (A8R8G8B8, X8R8G8B8), and the rest it may ask that any
/// card has. R8G8B8 (20) is refused, as DXVK refused it.
fn texture_format_ok(fmt: u32) -> bool {
    matches!(fmt, 21 | 22 | 23 | 24 | 25 | 26 | 28 | 50 | 51 | 0x3154_5844 | 0x3254_5844 | 0x3354_5844 | 0x3454_5844 | 0x3554_5844)
}

unsafe extern "system" fn qi(this: *mut SObj, iid: *const Guid, out: *mut *mut c_void) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 0);
    obj::query(this, iid, out)
}
unsafe extern "system" fn add_ref(this: *mut SObj) -> u32 {
    obj::add_ref(this)
}
unsafe extern "system" fn release(this: *mut SObj) -> u32 {
    obj::release(this)
}

unsafe extern "system" fn register_software_device(_this: *mut SObj, _f: *mut c_void) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 3);
    D3DERR_INVALIDCALL
}

unsafe extern "system" fn get_adapter_count(_this: *mut SObj) -> u32 {
    trace::call(frame!(), Kind::D3D, 4);
    1
}

/// `D3DADAPTER_IDENTIFIER8` from Direct3D 9's: the same fields without `DeviceName[32]`.
unsafe extern "system" fn get_adapter_identifier(_this: *mut SObj, adapter: u32, flags: u32, id: *mut u8) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 5);
    if adapter != 0 || id.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let mut id9 = [0u8; 1100];
    if stub().d3d9.is_null() {
        // On Vulkan there is no Direct3D 9 to ask before the device exists: a name of
        // our own, and no vendor (nothing in the engine was seen to read it, phase 0).
        let name = b"Armada II Refit (Display8 on Vulkan)";
        id9[512..512 + name.len()].copy_from_slice(name);
        id9[..8].copy_from_slice(b"d3d8.dll");
    } else {
        let hr = vcall!(stub().d3d9, 5, fn(u32, u32, *mut u8) -> HRESULT, 0, flags, id9.as_mut_ptr());
        if hr < 0 {
            return hr;
        }
    }
    unsafe {
        core::ptr::copy_nonoverlapping(id9.as_ptr(), id, 1024);
        core::ptr::copy_nonoverlapping(id9.as_ptr().add(1056), id.add(1024), 1100 - 1056);
    }
    S_OK
}

unsafe extern "system" fn get_adapter_mode_count(_this: *mut SObj, adapter: u32) -> u32 {
    trace::call(frame!(), Kind::D3D, 6);
    if adapter == 0 { 1 } else { 0 }
}

unsafe extern "system" fn enum_adapter_modes(_this: *mut SObj, adapter: u32, mode: u32, out: *mut DisplayMode) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 7);
    if adapter != 0 || mode != 0 || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = stub().mode };
    S_OK
}

unsafe extern "system" fn get_adapter_display_mode(_this: *mut SObj, adapter: u32, out: *mut DisplayMode) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::D3D, 8);
    if adapter != 0 || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = stub().mode };
    S_OK
}

unsafe extern "system" fn check_device_type(_this: *mut SObj, adapter: u32, dt: u32, disp: u32, bb: u32, windowed: i32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 9);
    let hr = if adapter == 0 && dt == 1 && disp == 22 && (bb == 22 || bb == 21) { S_OK } else { D3DERR_NOTAVAILABLE };
    once(&sfmt!("STUB CheckDeviceType({}, {}, {}, {}, {}) -> {:#x}", adapter, dt, disp, bb, windowed, hr as u32));
    hr
}

unsafe extern "system" fn check_device_format(_this: *mut SObj, adapter: u32, dt: u32, afmt: u32, usage: u32, rtype: u32, fmt: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 10);
    let ok = adapter == 0
        && dt == 1
        && match (rtype, usage) {
            (1, u) if u & 2 != 0 => matches!(fmt, 70 | 71 | 73 | 75 | 77 | 79 | 80), // a depth surface
            (1, u) if u & 1 != 0 => matches!(fmt, 21 | 22),                          // a render target
            (3 | 4 | 5, 0) | (1, 0) => texture_format_ok(fmt),
            (3, u) if u & 1 != 0 => matches!(fmt, 21 | 22),
            _ => false,
        };
    let hr = if ok { S_OK } else { D3DERR_NOTAVAILABLE };
    once(&sfmt!("STUB CheckDeviceFormat({}, {}, {}, {:#x}, {}, {}) -> {:#x}", adapter, dt, afmt, usage, rtype, fmt, hr as u32));
    hr
}

unsafe extern "system" fn check_device_multisample_type(_this: *mut SObj, _a: u32, _dt: u32, _fmt: u32, _w: i32, ms: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 11);
    if ms == 0 { S_OK } else { D3DERR_NOTAVAILABLE }
}

unsafe extern "system" fn check_depth_stencil_match(_this: *mut SObj, _a: u32, _dt: u32, _af: u32, _rt: u32, _ds: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 12);
    S_OK
}

unsafe extern "system" fn get_device_caps(_this: *mut SObj, _adapter: u32, _dt: u32, caps: *mut u8) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 13);
    if caps.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { core::ptr::copy_nonoverlapping(CAPS.as_ptr() as *const u8, caps, CAPS8_SIZE) };
    S_OK
}

unsafe extern "system" fn get_adapter_monitor(_this: *mut SObj, adapter: u32) -> usize {
    trace::call(frame!(), Kind::D3D, 14);
    if stub().d3d9.is_null() {
        return super::primary_monitor();
    }
    vcall!(stub().d3d9, 15, fn(u32) -> usize, adapter)
}

unsafe extern "system" fn create_device(
    _this: *mut SObj,
    adapter: u32,
    dt: u32,
    hwnd: HWND,
    flags: u32,
    pp: *mut PresentParams,
    out: *mut *mut c_void,
) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::D3D, 15);
    trace::check_mark(true);
    if pp.is_null() || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let asked = unsafe { *pp };
    let window = if asked.window != 0 { asked.window as *mut c_void } else { hwnd };
    let depth = if asked.auto_depth != 0 { asked.depth_format } else { 0 };
    // The stub's own device is windowed whatever the engine asks, so any interval holds.
    if !super::ensure_device(window, flags, depth, crate::present_interval(asked.interval, false)) {
        return D3DERR_NOTAVAILABLE;
    }
    let dev = super::device::make(asked);
    unsafe { *out = dev as *mut c_void };
    crate::api::device_made(dev as *mut c_void);
    trace::after_device();
    log!(
        "stub: CreateDevice: adapter {} type {} flags {:#x}; asked {}; given {}x{} by {}",
        adapter, dt, flags, asked.text(), stub().mode.width, stub().mode.height, trace::chain(fp)
    );
    S_OK
}

pub fn install() {
    obj::install(
        Kind::D3D,
        &[
            (0, qi as *const () as usize),
            (1, add_ref as *const () as usize),
            (2, release as *const () as usize),
            (3, register_software_device as *const () as usize),
            (4, get_adapter_count as *const () as usize),
            (5, get_adapter_identifier as *const () as usize),
            (6, get_adapter_mode_count as *const () as usize),
            (7, enum_adapter_modes as *const () as usize),
            (8, get_adapter_display_mode as *const () as usize),
            (9, check_device_type as *const () as usize),
            (10, check_device_format as *const () as usize),
            (11, check_device_multisample_type as *const () as usize),
            (12, check_depth_stencil_match as *const () as usize),
            (13, get_device_caps as *const () as usize),
            (14, get_adapter_monitor as *const () as usize),
            (15, create_device as *const () as usize),
        ],
    );
}
