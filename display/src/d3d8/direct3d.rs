//! `IDirect3D8`: the adapter's answers, which the engine branches on (phase 0, item 3,
//! logs each distinct one), and `CreateDevice`.
use super::{DisplayMode, PresentParams, CAPS8_SIZE, HWND};
use crate::com::{self, Kind, Obj, Role, HRESULT};
use crate::trace::{self, once};
use crate::{log, sfmt};
use core::ffi::c_void;

unknown!(D3D, qi, add_ref, release);
pass!(D3D 3 register_software_device(f: *mut c_void) -> HRESULT);
pass!(D3D 14 get_adapter_monitor(adapter: u32) -> usize);

unsafe extern "system" fn get_adapter_count(this: *mut Obj) -> u32 {
    trace::call(frame!(), Kind::D3D, 4);
    let n = vcall!(unsafe { (*this).real }, 4, fn() -> u32,);
    once(&sfmt!("ANSWER GetAdapterCount -> {}", n));
    n
}

unsafe extern "system" fn get_adapter_identifier(this: *mut Obj, adapter: u32, flags: u32, id: *mut u8) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 5);
    let hr = vcall!(unsafe { (*this).real }, 5, fn(u32, u32, *mut u8) -> HRESULT, adapter, flags, id);
    if hr >= 0 && !id.is_null() {
        // Driver[512], Description[512], DriverVersion (8), VendorId, DeviceId, SubSysId, Revision.
        let desc = unsafe { crate::rt::cstr_at(id.add(512), 512) };
        let ids = unsafe { core::slice::from_raw_parts(id.add(1032) as *const u32, 4) };
        once(&sfmt!(
            "ANSWER GetAdapterIdentifier({}, {:#x}) -> \"{}\" vendor {:#x} device {:#x}",
            adapter, flags, desc, ids[0], ids[1]
        ));
    }
    hr
}

unsafe extern "system" fn get_adapter_mode_count(this: *mut Obj, adapter: u32) -> u32 {
    let fp = frame!();
    trace::call(fp, Kind::D3D, 6);
    let n = vcall!(unsafe { (*this).real }, 6, fn(u32) -> u32, adapter);
    once(&sfmt!("ANSWER GetAdapterModeCount({}) -> {} by {}", adapter, n, trace::caller(fp)));
    n
}

unsafe extern "system" fn enum_adapter_modes(this: *mut Obj, adapter: u32, mode: u32, out: *mut DisplayMode) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 7);
    let hr = vcall!(unsafe { (*this).real }, 7, fn(u32, u32, *mut DisplayMode) -> HRESULT, adapter, mode, out);
    if hr >= 0 && !out.is_null() {
        let m = unsafe { *out };
        once(&sfmt!("ANSWER EnumAdapterModes({}, {}) -> {}x{} {} Hz fmt {}", adapter, mode, m.width, m.height, m.refresh, m.format));
    }
    hr
}

unsafe extern "system" fn get_adapter_display_mode(this: *mut Obj, adapter: u32, out: *mut DisplayMode) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::D3D, 8);
    let hr = vcall!(unsafe { (*this).real }, 8, fn(u32, *mut DisplayMode) -> HRESULT, adapter, out);
    if hr >= 0 && !out.is_null() {
        let m = unsafe { *out };
        once(&sfmt!(
            "ANSWER GetAdapterDisplayMode({}) -> {}x{} {} Hz fmt {} by {}",
            adapter, m.width, m.height, m.refresh, m.format, trace::chain(fp)
        ));
    }
    hr
}

unsafe extern "system" fn check_device_type(this: *mut Obj, adapter: u32, dt: u32, disp: u32, bb: u32, windowed: i32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 9);
    let hr = vcall!(unsafe { (*this).real }, 9, fn(u32, u32, u32, u32, i32) -> HRESULT, adapter, dt, disp, bb, windowed);
    once(&sfmt!("ANSWER CheckDeviceType({}, {}, {}, {}, {}) -> {:#x}", adapter, dt, disp, bb, windowed, hr as u32));
    hr
}

unsafe extern "system" fn check_device_format(this: *mut Obj, adapter: u32, dt: u32, afmt: u32, usage: u32, rtype: u32, fmt: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 10);
    let hr = vcall!(unsafe { (*this).real }, 10, fn(u32, u32, u32, u32, u32, u32) -> HRESULT, adapter, dt, afmt, usage, rtype, fmt);
    once(&sfmt!(
        "ANSWER CheckDeviceFormat({}, {}, {}, {:#x}, {}, {}) -> {:#x}",
        adapter, dt, afmt, usage, rtype, fmt, hr as u32
    ));
    hr
}

unsafe extern "system" fn check_device_multisample_type(this: *mut Obj, adapter: u32, dt: u32, fmt: u32, windowed: i32, ms: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 11);
    let hr = vcall!(unsafe { (*this).real }, 11, fn(u32, u32, u32, i32, u32) -> HRESULT, adapter, dt, fmt, windowed, ms);
    once(&sfmt!("ANSWER CheckDeviceMultiSampleType({}, {}, {}, {}, {}) -> {:#x}", adapter, dt, fmt, windowed, ms, hr as u32));
    hr
}

unsafe extern "system" fn check_depth_stencil_match(this: *mut Obj, adapter: u32, dt: u32, afmt: u32, rt: u32, ds: u32) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 12);
    let hr = vcall!(unsafe { (*this).real }, 12, fn(u32, u32, u32, u32, u32) -> HRESULT, adapter, dt, afmt, rt, ds);
    once(&sfmt!("ANSWER CheckDepthStencilMatch({}, {}, {}, {}, {}) -> {:#x}", adapter, dt, afmt, rt, ds, hr as u32));
    hr
}

unsafe extern "system" fn get_device_caps(this: *mut Obj, adapter: u32, dt: u32, caps: *mut u8) -> HRESULT {
    trace::call(frame!(), Kind::D3D, 13);
    let hr = vcall!(unsafe { (*this).real }, 13, fn(u32, u32, *mut u8) -> HRESULT, adapter, dt, caps);
    if hr >= 0 && !caps.is_null() {
        once(&sfmt!("ANSWER IDirect3D8::GetDeviceCaps({}, {}) -> {}", adapter, dt, trace::hex(caps as *const c_void, CAPS8_SIZE)));
    }
    hr
}

/// Devices made so far: the epoch the plugins will be told of (phase 3).
pub static DEVICES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

unsafe extern "system" fn create_device(
    this: *mut Obj,
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
    let asked = if pp.is_null() { PresentParams::default() } else { unsafe { *pp } };
    let hr = vcall!(
        unsafe { (*this).real },
        15,
        fn(u32, u32, HWND, u32, *mut PresentParams, *mut *mut c_void) -> HRESULT,
        adapter, dt, hwnd, flags, pp, out
    );
    com::wrap_out(out, hr, Kind::Device, Role::Plain);
    if hr >= 0 && !out.is_null() {
        crate::api::device_made(unsafe { *out });
    }
    let n = DEVICES.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;
    trace::after_device();
    if hr >= 0 {
        trace::MAIN_THREAD.store(unsafe { crate::rt::win::GetCurrentThreadId() }, core::sync::atomic::Ordering::Relaxed);
    }
    log!(
        "CreateDevice #{}: adapter {} type {} hwnd {:#x} flags {:#x} -> {:#x}; asked {}; by {}",
        n, adapter, dt, hwnd as *const () as usize, flags, hr as u32, asked.text(), trace::chain(fp)
    );
    if !pp.is_null() && hr >= 0 {
        let got = unsafe { *pp };
        if got.text() != asked.text() {
            log!("CreateDevice #{}: parameters as the chain left them: {}", n, got.text());
        }
    }
    hr
}

pub fn install() {
    com::install(
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
