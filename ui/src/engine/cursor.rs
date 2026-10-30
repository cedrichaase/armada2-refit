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
/// Whether the latest `RefreshDisplay` (while it runs: this one) hides the engine's
/// cursor: what the UI claims (`CLAIM_CURSOR`), asked by Display8 at each scene, that
/// of the cursor inside this call included. Kept between presents, so the claim stays
/// put while the UI keeps drawing its cursor.
static mut HIDING: bool = false;
static mut INSTALLED: bool = false;
/// The probe on `RefreshDisplay`'s draw of the cursor (`install_probe`): the function
/// the call went to before it (stock `DrawScaled2D`, or HUD.asi's wrap of it).
static mut PROBE: Option<usize> = None;
/// Whether the probe keeps the engine's cursor square (`install_square`).
static mut SQUARE: bool = false;
/// Set by the probe when this present's draw of the cursor reached it.
static mut REACHED: bool = false;
/// Set for one present by `hide_this_frame`: the probe skips the draw.
static mut SUPPRESS: bool = false;
/// Whether, at the last present with a cursor sprite, the engine's draw of it got as
/// far as the probe: false while something earlier in the call's chain skips it
/// (Scene.asi's `cursor off`).
static mut SHOWN: bool = true;
/// The sprite at `+0x60` at the last present (0: none, or a hardware cursor).
static mut SPRITE: usize = 0;

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
    if dev == 0 {
        return unsafe { stock(dev) };
    }
    let p = (dev + DEV_CURSOR) as *mut usize;
    unsafe { HIDING = hide };
    unsafe {
        let sprite = *p;
        if PROBE.is_some() {
            // The draw is let through to the call site, where `cursor_draw` notes that it
            // was reached and, while hiding, skips it.
            REACHED = false;
            SUPPRESS = hide;
            stock(dev);
            SUPPRESS = false;
            let sync = rd::<u8>(dev + DEV_SYNCHRONOUS) != 0;
            let shown = sync && sprite != 0 && REACHED;
            if sync && sprite != 0 && shown != SHOWN {
                crate::log!("cursor: the engine's cursor is {}", if shown { "shown" } else { "held back (something before ours skips its draw)" });
            }
            SPRITE = if sync { sprite } else { 0 };
            if sync && sprite != 0 {
                SHOWN = shown;
            }
        } else if hide {
            let saved = sprite;
            *p = 0;
            stock(dev);
            *p = saved;
        } else {
            stock(dev);
        }
    }
}

/// Whether the engine's cursor is kept from drawing at this present (asked inside
/// `RefreshDisplay`) or was at the last: then the UI claims the cursor.
pub fn hiding() -> bool {
    unsafe { HIDING }
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
    element_name(field(device(), DEV_CURSOR))
}

/// A cursor sprite's `cursor.spr` name; None for no sprite.
pub fn element_name(sprite: usize) -> Option<&'static str> {
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

/// Whether the engine would show its cursor: false while a plugin that wraps
/// `RefreshDisplay`'s draw of it (Scene.asi's `cursor off`) skips that draw. Read from
/// the last present that had a cursor sprite, so it follows a change a frame late.
/// True when the probe is not in.
pub fn engine_shows() -> bool {
    unsafe { PROBE.is_none() || SHOWN }
}

/// Whether the engine had a cursor sprite to draw at the last present (false without
/// the probe, or with a hardware cursor).
pub fn engine_has_sprite() -> bool {
    unsafe { PROBE.is_some() && SPRITE != 0 }
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

/// Wraps the one `DrawScaled2D` call in `RefreshDisplay` that draws the synchronous
/// cursor (the probe). Through it the UI learns whether the engine's draw of its cursor
/// was reached (`engine_shows`): another plugin that wraps the same call later in the
/// game, Scene.asi with `cursor off`, skips it before it gets here, and then the UI
/// draws no cursor either. While the UI draws its own, the probe skips the engine's
/// draw (`hide_this_frame`), so the engine's choices are left as they are. Chains to
/// whatever the call went to (stock, or HUD.asi's wrap). Made before any mission.
pub fn install_probe() -> bool {
    let Some(next) = super::hook::call_target(SITE_CURSOR_DRAW) else {
        crate::log!("cursor: RefreshDisplay's cursor draw is not a call: the UI cannot see whether the engine's cursor is shown");
        return false;
    };
    let ok = super::hook::patch_call(SITE_CURSOR_DRAW, next, cursor_draw as *const () as usize);
    if ok {
        unsafe { PROBE = Some(next) };
    }
    crate::log!("cursor: RefreshDisplay's cursor draw {}", if ok { "is watched" } else { "could not be patched" });
    ok
}

/// Keeps the engine's own cursor square, as HUD.asi did (hud/README.md, "HUD.asi",
/// the cursors), for wherever the engine's cursor still shows (`[UI] Cursor=0`, or a
/// cursor ours cannot hide): the engine scales it by the back buffer over 800 x 600
/// on each axis, so on a wide screen it is drawn wide. The synchronous path's one draw
/// (through the probe) is made so that, for that draw only, the x scale is the y scale
/// and the position is moved so the hotspot stays on the pointer; the hardware path's
/// width is multiplied by the y scale. Not made while HUD.asi is loaded (it makes the
/// same two changes) or while `hud/cursor-aspect.py`'s squashed art is installed (it
/// would be squashed twice), or without the probe on stock `DrawScaled2D`.
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
    if unsafe { PROBE } != Some(DRAW_SCALED_2D) || !super::starts(SITE_HW_SCALE, &HW_SCALE_STOCK) {
        crate::log!("cursor: RefreshDisplay's or SetCursor's site is not stock: the engine's cursor is left as it is");
        return false;
    }
    let ok = super::hook::patch_bytes(SITE_HW_SCALE, &HW_SCALE_STOCK, &HW_SCALE_SQUARE);
    unsafe { SQUARE = ok };
    crate::log!("cursor: the engine's cursor {}", if ok { "is kept square" } else { "could not be patched" });
    ok
}

unsafe extern "thiscall" fn cursor_draw(sprite: usize, pos: *mut f32, w: f32, h: f32) {
    unsafe {
        REACHED = true;
        if SUPPRESS {
            return;
        }
        let Some(next) = PROBE else { return };
        let draw: unsafe extern "thiscall" fn(usize, *mut f32, f32, f32) = core::mem::transmute(next);
        let scale = SCALE_2D as *mut f32;
        let (sx, sy) = (*scale, *scale.add(1));
        if SQUARE && sx > 0.0 && sy > 0.0 && sx != sy && !pos.is_null() {
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
