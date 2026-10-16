//! The engine's mouse cursor: which one it shows, and hiding it for a frame so the UI
//! can draw its own (README.md, "The cursor").
//!
//! Each frame `CursorInterface::Render` hands the cursor sprite the HUD chose to the
//! device's `SetCursor`, which keeps it at `ST3D_DeviceDirectX8+0x60`. Under DXVK the
//! cursor is the synchronous one (`+0xe0` set): `RefreshDisplay` draws that sprite as
//! a 2D quad in a scene of its own just before it presents, and draws nothing when
//! `+0x60` is empty. Hiding it for a frame therefore needs no change to what the engine
//! chose: `RefreshDisplay` is wrapped, through its vtable slot, to empty `+0x60` for
//! the one call and put it back.
use super::{field, rd, storm3d};
use crate::engine::hook;

/// `ST3D_DeviceDirectX8`'s vtable.
const DEVICE_VTABLE: usize = 0x6bc6ac;
/// Its slot 26 (+0x68): `ST3D_DeviceDirectX8::RefreshDisplay`, thiscall.
const SLOT_REFRESH: usize = 0x68 / 4;
const REFRESH_DISPLAY: usize = 0x624630;
/// `ST3D_DeviceDirectX8+0x60`: the cursor sprite `SetCursor` was last given.
const DEV_CURSOR: usize = 0x60;
/// `ST3D_DeviceDirectX8+0xe0`: set when the engine draws the cursor itself
/// (`SetSynchronousCursor`), clear for a Direct3D hardware cursor.
const DEV_SYNCHRONOUS: usize = 0xe0;
/// `ST3D_DatabaseElement+0x8`: the element's name (a cursor sprite's `cursor.spr` name).
const ELEMENT_NAME: usize = 0x8;

static mut HIDE: bool = false;
static mut INSTALLED: bool = false;

/// The engine's current device (`Storm3D+0xcc`), 0 before there is one.
fn device() -> usize {
    field(storm3d(), 0xcc)
}

/// Wraps `RefreshDisplay`. False, with nothing written, when its slot is not stock.
pub fn install() -> bool {
    let slot = DEVICE_VTABLE + SLOT_REFRESH * 4;
    if unsafe { rd::<usize>(slot) } != REFRESH_DISPLAY {
        crate::log!("cursor: RefreshDisplay's slot is not stock: the engine's cursor cannot be hidden");
        return false;
    }
    let ok = hook::patch_slot(DEVICE_VTABLE, SLOT_REFRESH, refresh as *const () as usize).is_some();
    unsafe { INSTALLED = ok };
    ok
}

unsafe extern "thiscall" fn refresh(dev: usize) {
    let stock: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(REFRESH_DISPLAY) };
    let hide = unsafe { core::mem::take(&mut *core::ptr::addr_of_mut!(HIDE)) };
    if !hide || dev == 0 {
        return unsafe { stock(dev) };
    }
    let p = (dev + DEV_CURSOR) as *mut usize;
    unsafe {
        let saved = *p;
        *p = 0;
        stock(dev);
        *p = saved;
    }
}

/// Keeps the engine from drawing its cursor at the next present. Call it every frame
/// the UI draws a cursor of its own; it lapses after one present. Works for the
/// synchronous cursor only (`synchronous()`), the one DXVK uses.
pub fn hide_this_frame() {
    if unsafe { INSTALLED } {
        unsafe { HIDE = true };
    }
}

/// Whether the engine draws the cursor itself (true under DXVK). When false it is a
/// Direct3D hardware cursor, which `hide_this_frame` does not reach.
pub fn synchronous() -> bool {
    let d = device();
    d != 0 && unsafe { rd::<u8>(d + DEV_SYNCHRONOUS) } != 0
}

/// The name of the cursor the engine shows now, as `cursor.spr` names it (the move,
/// attack, select... cursors). Read after the HUD has rendered (the mission frame
/// hook): that is when this frame's choice has reached the device. None when no cursor
/// is shown.
pub fn shape() -> Option<&'static str> {
    let sprite = field(device(), DEV_CURSOR);
    let name = field(sprite, ELEMENT_NAME);
    if name == 0 {
        return None;
    }
    let p = name as *const u8;
    let mut n = 0;
    while n < 64 && unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, n) }).ok()
}

/// Whether the engine's cursor sprite can be hidden (the `RefreshDisplay` wrap is in).
pub fn installed() -> bool {
    unsafe { INSTALLED }
}
