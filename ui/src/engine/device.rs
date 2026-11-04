//! The engine's Direct3D device coming and going.
//!
//! - **A new device.** `ST3D_DeviceDirectX8::DestroyRenderSurfaces` (slot 28 of its
//!   vtable, 0x6bc6ac) releases the engine's device at every change of display: between
//!   the shell's and a mission's, and in Graphics Settings. The slot is chained: the
//!   close-up lets go of its pictures before the engine releases the device.
//! - **`Reset`** (slot 14 of the d3d8 device's vtable, chained): the close-up drops its
//!   pictures, whose contents are lost.
//! - **`Present`** (slot 15, chained with `Reset`) is where a frame dump asked for with
//!   `UI.dump` reads the back buffer: the frame as presented, HUD, UI and the engine's
//!   cursor included (README.md, "Frame dumps").
use super::{eng, field, hook as patch};
/// A Direct3D `HRESULT`.
type HRESULT = i32;
use crate::log;
use crate::syms_core::{hook, sym};
use core::ffi::c_void;

// Slot 28 of `ST3D_DeviceDirectX8`'s vtable is `DestroyRenderSurfaces` (`ui.core.toml`,
// hook `core.device`).
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
    if eng().ready(hook::CORE_DEVICE).is_err() {
        return false;
    }
    match patch::patch_slot(eng().addr(sym::CORE_SLOT_DESTROY), 0, destroy as *const () as usize) {
        Some(old) => {
            unsafe { DESTROY_NEXT = old };
            true
        }
        None => false,
    }
}

unsafe extern "thiscall" fn destroy(this: usize) {
    super::closeup::release();
    let next: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(DESTROY_NEXT) };
    unsafe { next(this) };
}

unsafe extern "system" fn reset(dev8: *mut c_void, params: *mut c_void) -> HRESULT {
    super::closeup::release();
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
    crate::movie::present(dev8);
    if let Some(name) = crate::dump::poll("UI.dump") {
        dump_back_buffer(dev8, name, n);
    }
    // A recorded frame (gpu/README.md, "Record and replay") ends here: its streams are
    // written, and the frame as presented is dumped beside them, for the replay to be
    // compared with.
    if let Some(name) = crate::gfx::gpu().record_present() {
        if crate::rt::create_dir(&crate::rt::game_path("Dumps")) {
            dump_back_buffer(dev8, crate::sfmt!("{}-game", name), n);
        }
    }
    match unsafe { PRESENT } {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, Present8>(f)(dev8, src, dst, wnd, dirty) },
    }
}

fn dump_back_buffer(dev8: *mut c_void, name: alloc::string::String, frame: u32) {
    let mut d = crate::dump::Dump::new(name, frame, "ui");
    let g = crate::gfx::gpu();
    let _ = dev8;
    // Display8's image of the engine's back buffer, before its gamma pass.
    let bb = crate::gfx::attach().and_then(|_| g.engine_target());
    match bb {
        Some(bb) => {
            d.note("the back buffer at Present: the frame as presented");
            d.surface(g, &bb, None, "present", crate::dump::Kind::Ppm);
        }
        None => {
            log!("dump: no Display8 device or back buffer behind the game's: nothing to read");
            d.note("no Display8 device behind the game's: nothing read");
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
            if let Some(old) = patch::patch_slot(vt, D3D8_PRESENT, present as *const () as usize) {
                PRESENT = old;
                PRESENT_VT = vt;
            }
        }
    }
    unsafe {
        if vt == 0 || vt == RESET_VT || field(vt, D3D8_RESET * 4) == reset as *const () as usize {
            return;
        }
        if let Some(old) = patch::patch_slot(vt, D3D8_RESET, reset as *const () as usize) {
            RESET = old;
            RESET_VT = vt;
        }
    }
}
