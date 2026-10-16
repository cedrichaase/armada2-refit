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

// ---- the engine's cursor, square at any aspect --------------------------------------

/// In `RefreshDisplay`: the `call ST3D_Sprite::DrawScaled2D` that draws the
/// synchronous cursor, after `SetScaleFactor2D(&device+0x18)`.
const SITE_CURSOR_DRAW: usize = 0x6246fa;
const DRAW_SCALED_2D: usize = 0x63ada0;
/// `ST3D_Sprite`'s global 2D scale `{x, y}` that `DrawScaled2D` multiplies by.
const SCALE_2D: usize = 0x7ad6e8;
/// `ST3D_Sprite+0x30`: the sprite's hotspot x, in texels.
const SPRITE_ORIGIN_X: usize = 0x30;
/// In `SetCursor`: `fmuls 0x18(%esi)`, the hardware cursor's width times the x scale.
const SITE_HW_SCALE: usize = 0x625dd9;
const HW_SCALE_STOCK: [u8; 3] = [0xd8, 0x4e, 0x18];
const HW_SCALE_SQUARE: [u8; 3] = [0xd8, 0x4e, 0x1c];

/// Keeps the engine's own cursor square, as HUD.asi did (hud/README.md, "HUD.asi",
/// the cursors), for wherever the engine's cursor still shows (`[UI] Cursor=0`, or a
/// cursor ours cannot hide): the engine scales it by the back buffer over 800 x 600
/// on each axis, so on a wide screen it is drawn wide. The synchronous path's one draw
/// is wrapped so that, for that draw only, the x scale is the y scale and the
/// position is moved so the hotspot stays on the pointer; the hardware path's width is
/// multiplied by the y scale. Not made while HUD.asi is loaded (it makes the same two
/// changes) or while `hud/cursor-aspect.py`'s squashed art is installed (it would be
/// squashed twice).
pub fn install_square() -> bool {
    let hud_asi = unsafe { !crate::rt::win::GetModuleHandleA(c"HUD.asi".as_ptr() as *const u8).is_null() };
    if hud_asi {
        crate::log!("cursor: HUD.asi is loaded and keeps the engine's cursor square");
        return false;
    }
    if crate::rt::file_exists(&crate::rt::game_path("Textures\\RGB\\Curs_Move.tga.a2neb-backup")) {
        crate::log!("cursor: cursor-aspect.py's squashed art is installed: the engine's cursor is left as it is");
        return false;
    }
    if super::hook::call_target(SITE_CURSOR_DRAW) != Some(DRAW_SCALED_2D) || !super::starts(SITE_HW_SCALE, &HW_SCALE_STOCK) {
        crate::log!("cursor: RefreshDisplay's or SetCursor's site is not stock: the engine's cursor is left as it is");
        return false;
    }
    let ok = super::hook::patch_call(SITE_CURSOR_DRAW, DRAW_SCALED_2D, draw_square as *const () as usize)
        && super::hook::patch_bytes(SITE_HW_SCALE, &HW_SCALE_STOCK, &HW_SCALE_SQUARE);
    crate::log!("cursor: the engine's cursor {}", if ok { "is kept square" } else { "could not be patched" });
    ok
}

unsafe extern "thiscall" fn draw_square(sprite: usize, pos: *mut f32, w: f32, h: f32) {
    let draw: unsafe extern "thiscall" fn(usize, *mut f32, f32, f32) = unsafe { core::mem::transmute(DRAW_SCALED_2D) };
    let scale = SCALE_2D as *mut f32;
    unsafe {
        let (sx, sy) = (*scale, *scale.add(1));
        if sx > 0.0 && sy > 0.0 && sx != sy && !pos.is_null() {
            // The engine passes pointer / scale - hotspot: the same point at the y scale.
            let ox: f32 = rd(sprite + SPRITE_ORIGIN_X);
            *pos = (*pos + ox) * sx / sy - ox;
            *scale = sy;
            draw(sprite, pos, w, h);
            *scale = sx;
        } else {
            draw(sprite, pos, w, h);
        }
    }
}
