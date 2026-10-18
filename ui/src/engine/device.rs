//! The engine's Direct3D device coming and going. Under native Direct3D 9 (crosire's
//! d3d8to9 on Windows) both need everything the UI made on the device let go of first;
//! DXVK forgives either.
//!
//! - **A new device.** `ST3D_DeviceDirectX8::DestroyRenderSurfaces` (slot 28 of its
//!   vtable, 0x6bc6ac) releases the engine's device at every change of display: between
//!   the shell's and a mission's, and in Graphics Settings. A device lives while any
//!   object made on it is held, and the engine's next `CreateDevice` then fails ("Fatal
//!   error in Direct3D. Please reboot."). The slot is chained: the UI lets go of all it
//!   holds on the device, in every pool, before the engine releases it.
//! - **`Reset`** (slot 14 of the d3d8 device's vtable, chained) fails while a state block
//!   or anything in the default pool is held: the UI's state block and the close-up's
//!   pictures go first.
use super::{field, hook};
use crate::d3d::*;
use crate::log;
use core::ffi::c_void;

const DX8_VTABLE: usize = 0x6bc6ac;
const SLOT_DESTROY: usize = 28;
/// `ST3D_DeviceDirectX8::DestroyRenderSurfaces`.
const DESTROY: usize = 0x624760;
/// `IDirect3DDevice8::Reset`.
const D3D8_RESET: usize = 14;

static mut DESTROY_NEXT: usize = 0;
static mut RESET: usize = 0;
static mut RESET_VT: usize = 0;

/// Chains `DestroyRenderSurfaces`. False, with nothing written, when the slot holds
/// neither the stock function nor another plugin's (one outside `Armada2.exe`'s code).
pub fn install() -> bool {
    let cur = field(DX8_VTABLE, SLOT_DESTROY * 4);
    if cur != DESTROY && (0x401000..0x6ae000).contains(&cur) {
        return false;
    }
    match hook::patch_slot(DX8_VTABLE, SLOT_DESTROY, destroy as *const () as usize) {
        Some(old) => {
            unsafe { DESTROY_NEXT = old };
            true
        }
        None => false,
    }
}

unsafe extern "thiscall" fn destroy(this: usize) {
    // Hold the d3d9 device across the engine's release, to count who else still holds
    // it: 0 once the engine has let go and so have we (and any plugin after us).
    let dev8 = field(this, 0x90) as *mut c_void;
    let held = if dev8.is_null() { None } else { Device::from_d3d8(dev8).map(|d| Com::from_borrowed(d.p())) };
    super::closeup::release();
    crate::gfx::forget();
    let left = live();
    if !left.is_empty() {
        let mut t = alloc::string::String::new();
        for (k, n) in left.iter() {
            let _ = core::fmt::Write::write_fmt(&mut t, format_args!(" {} {}", n, k));
        }
        log!("device: still held after letting go:{}", t);
    }
    let next: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(DESTROY_NEXT) };
    unsafe { next(this) };
    if let Some(h) = held {
        let p = h.ptr();
        core::mem::forget(h);
        let left = vcall!(p, 2, fn() -> u32,);
        log!("device: the engine let go of its device; {} references to it left (0: it is gone)", left);
    }
}

unsafe extern "system" fn reset(dev8: *mut c_void, params: *mut c_void) -> HRESULT {
    super::closeup::release();
    crate::gfx::reset();
    let left = live_default();
    if !left.is_empty() {
        let mut t = alloc::string::String::new();
        for (k, n) in left.iter() {
            let _ = core::fmt::Write::write_fmt(&mut t, format_args!(" {} {}", n, k));
        }
        log!("device: still held at Reset:{}", t);
    }
    log!("device: Reset");
    match unsafe { RESET } {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT>(f)(dev8, params) },
    }
}

/// Chains the d3d8 device's `Reset`, once per vtable (every device made after has the
/// same one).
pub fn hook_reset(dev8: *mut c_void) {
    let vt = field(dev8 as usize, 0);
    unsafe {
        if vt == 0 || vt == RESET_VT || field(vt, D3D8_RESET * 4) == reset as *const () as usize {
            return;
        }
        if let Some(old) = hook::patch_slot(vt, D3D8_RESET, reset as *const () as usize) {
            RESET = old;
            RESET_VT = vt;
        }
    }
}
