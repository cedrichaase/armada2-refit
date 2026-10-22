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
//!   or anything in the default pool is held: `gpu` lets go of them first (`gfx::reset`),
//!   and the close-up drops its pictures, whose contents are lost.
//! - **`Present`** (slot 15, chained with `Reset`) is where a frame dump asked for with
//!   `UI.dump` reads the back buffer: the frame as presented, HUD, UI and the engine's
//!   cursor included (README.md, "Frame dumps").
use super::{field, hook};
use gpu::d3d9::raw::*;
use crate::log;
use core::ffi::c_void;

const DX8_VTABLE: usize = 0x6bc6ac;
const SLOT_DESTROY: usize = 28;
/// `ST3D_DeviceDirectX8::DestroyRenderSurfaces`.
const DESTROY: usize = 0x624760;
/// `IDirect3DDevice8::Reset`.
const D3D8_RESET: usize = 14;
/// `IDirect3DDevice8::Present`.
const D3D8_PRESENT: usize = 15;

static mut DESTROY_NEXT: usize = 0;
static mut RESET: usize = 0;
static mut RESET_VT: usize = 0;
static mut PRESENT: usize = 0;
static mut PRESENT_VT: usize = 0;
/// Frames presented through the chained `Present` (the dumps' frame number).
static mut PRESENTS: u32 = 0;

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

type Present8 = unsafe extern "system" fn(*mut c_void, *const c_void, *const c_void, *mut c_void, *const c_void) -> HRESULT;

/// Every present of the d3d8 device: a frame dump when `UI.dump` has appeared, read
/// from the back buffer just before it is presented.
unsafe extern "system" fn present(dev8: *mut c_void, src: *const c_void, dst: *const c_void, wnd: *mut c_void, dirty: *const c_void) -> HRESULT {
    let n = unsafe { PRESENTS };
    unsafe { PRESENTS = n.wrapping_add(1) };
    if let Some(name) = crate::dump::poll("UI.dump") {
        dump_back_buffer(dev8, name, n);
    }
    match unsafe { PRESENT } {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, Present8>(f)(dev8, src, dst, wnd, dirty) },
    }
}

fn dump_back_buffer(dev8: *mut c_void, name: alloc::string::String, frame: u32) {
    let mut d = crate::dump::Dump::new(name, frame, "ui");
    let bb = Device::from_d3d8(dev8).and_then(|dev| {
        crate::gfx::attach()?;
        dev.get_back_buffer()
    });
    match bb {
        Some(bb) => {
            let g = crate::gfx::gpu();
            d.note("the back buffer at Present: the frame as presented");
            d.surface(g, &g.wrap_d3d9_surface(bb), None, "present", crate::dump::Kind::Ppm);
        }
        None => {
            log!("dump: no Direct3D 9 device or back buffer behind the game's: nothing to read");
            d.note("no Direct3D 9 device behind the game's (DXVK's own d3d8?): nothing read");
        }
    }
    d.finish();
}

/// Chains the d3d8 device's `Reset` and `Present`, once per vtable (every device made
/// after has the same one).
pub fn hook_reset(dev8: *mut c_void) {
    let vt = field(dev8 as usize, 0);
    unsafe {
        if vt != 0 && vt != PRESENT_VT && field(vt, D3D8_PRESENT * 4) != present as *const () as usize {
            if let Some(old) = hook::patch_slot(vt, D3D8_PRESENT, present as *const () as usize) {
                PRESENT = old;
                PRESENT_VT = vt;
            }
        }
    }
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
