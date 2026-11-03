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
use super::{eng, field, rd, storm3d};
use crate::engine::hook as patch;
use crate::syms_core::{field, hook, sym};

// The profile's (`ui.core.toml`, hooks `core.cursor`, `core.cursor_probe`,
// `core.cursor_square`): `ST3D_DeviceDirectX8`'s slot 26 (+0x68), `RefreshDisplay`,
// thiscall; `ST3D_DeviceDirectX8+0x60`, the cursor sprite `SetCursor` was last given;
// `+0xe0`, set when the engine draws the cursor itself (`SetSynchronousCursor`), clear
// for a Direct3D hardware cursor; and `ST3D_DatabaseElement+0x8`, the element's name (a
// cursor sprite's `cursor.spr` name).

static mut HIDE: bool = false;
/// Whether the latest `RefreshDisplay` (while it runs: this one) hides the engine's
/// cursor: what the UI claims (`CLAIM_CURSOR`), asked by Display8 at each scene, that
/// of the cursor inside this call included. Kept between presents, so the claim stays
/// put while the UI keeps drawing its cursor.
static mut HIDING: bool = false;
static mut INSTALLED: bool = false;
/// The probe on `RefreshDisplay`'s draw of the cursor (`install_probe`): the function
/// the call went to before it (stock `DrawScaled2D`).
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
    field(storm3d(), field::CORE_STORM3D_DEVICES)
}

/// Wraps `RefreshDisplay`. False, with nothing written, when its slot is not stock.
pub fn install() -> bool {
    if eng().ready(hook::CORE_CURSOR).is_err() {
        crate::log!("cursor: the engine's cursor cannot be hidden");
        return false;
    }
    let ok = patch::patch_slot(eng().addr(sym::CORE_SLOT_REFRESH), 0, refresh as *const () as usize).is_some();
    unsafe { INSTALLED = ok };
    ok
}

unsafe extern "thiscall" fn refresh(dev: usize) {
    let stock: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(eng().addr(sym::CORE_REFRESH_DISPLAY)) };
    let hide = unsafe { core::mem::take(&mut *core::ptr::addr_of_mut!(HIDE)) };
    if dev == 0 {
        return unsafe { stock(dev) };
    }
    let p = (dev + field::CORE_DX8_CURSOR) as *mut usize;
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
            let sync = rd::<u8>(dev + field::CORE_DX8_SYNCHRONOUS) != 0;
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
    d != 0 && unsafe { rd::<u8>(d + field::CORE_DX8_SYNCHRONOUS) } != 0
}

/// The name of the cursor the engine shows now, as `cursor.spr` names it (the move,
/// attack, select... cursors). Read after the HUD has rendered (the mission frame
/// hook): that is when this frame's choice has reached the device. None when no cursor
/// is shown.
pub fn shape() -> Option<&'static str> {
    element_name(field(device(), field::CORE_DX8_CURSOR))
}

/// A cursor sprite's `cursor.spr` name; None for no sprite.
pub fn element_name(sprite: usize) -> Option<&'static str> {
    let name = field(sprite, field::CORE_ELEMENT_NAME);
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

// In `RefreshDisplay`: the `call ST3D_Sprite::DrawScaled2D` that draws the synchronous
// cursor, after `SetScaleFactor2D(&device+0x18)`; `ST3D_Sprite`'s global 2D scale
// `{x, y}` that `DrawScaled2D` multiplies by; `ST3D_Sprite+0x30`, the sprite's hotspot x,
// in texels; and, in `SetCursor`, `fmuls 0x18(%esi)`, the hardware cursor's width times
// the x scale: its displacement byte (the third) becomes the y scale's, one float on.
const HW_SCALE_DISP: usize = 2;
const HW_SCALE_DISP_Y: u8 = 0x1c; // not an address

/// Wraps the one `DrawScaled2D` call in `RefreshDisplay` that draws the synchronous
/// cursor (the probe). Through it the UI learns whether the engine's draw of its cursor
/// was reached (`engine_shows`): another plugin that wraps the same call later in the
/// game, Scene.asi with `cursor off`, skips it before it gets here, and then the UI
/// draws no cursor either. While the UI draws its own, the probe skips the engine's
/// draw (`hide_this_frame`), so the engine's choices are left as they are. Chains to
/// whatever the call went to. Made before any mission.
pub fn install_probe() -> bool {
    if eng().ready(hook::CORE_CURSOR_PROBE).is_err() {
        crate::log!("cursor: the UI cannot see whether the engine's cursor is shown");
        return false;
    }
    let site = eng().addr(sym::CORE_SITE_CURSOR_DRAW);
    let Some(next) = patch::call_target(site) else {
        return false;
    };
    let ok = patch::patch_call(site, next, cursor_draw as *const () as usize);
    if ok {
        unsafe { PROBE = Some(next) };
    }
    crate::log!("cursor: RefreshDisplay's cursor draw {}", if ok { "is watched" } else { "could not be patched" });
    ok
}

/// Keeps the engine's own cursor square (ui/HUD.md, "The cursors"), for wherever the engine's cursor still shows (`[UI] Cursor=0`, or a
/// cursor ours cannot hide): the engine scales it by the back buffer over 800 x 600
/// on each axis, so on a wide screen it is drawn wide. The synchronous path's one draw
/// (through the probe) is made so that, for that draw only, the x scale is the y scale
/// and the position is moved so the hotspot stays on the pointer; the hardware path's
/// width is multiplied by the y scale. Not made without the probe on stock `DrawScaled2D`.
pub fn install_square() -> bool {
    if eng().ready(hook::CORE_CURSOR_SQUARE).is_err() {
        crate::log!("cursor: SetCursor's site is not stock: the engine's cursor is left as it is");
        return false;
    }
    if unsafe { PROBE } != Some(eng().addr(sym::CORE_DRAW_SCALED_2D)) {
        crate::log!("cursor: RefreshDisplay's draw is not stock: the engine's cursor is left as it is");
        return false;
    }
    let site = eng().addr(sym::CORE_SITE_HW_SCALE);
    let stock: [u8; 3] = unsafe { core::ptr::read_unaligned(site as *const [u8; 3]) };
    let mut square = stock;
    square[HW_SCALE_DISP] = HW_SCALE_DISP_Y;
    let ok = patch::patch_bytes(site, &stock, &square);
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
        let scale = eng().addr(sym::CORE_SCALE_2D) as *mut f32;
        let (sx, sy) = (*scale, *scale.add(1));
        if SQUARE && sx > 0.0 && sy > 0.0 && sx != sy && !pos.is_null() {
            // The engine passes pointer / scale - hotspot: the same point at the y scale.
            let ox: f32 = rd(sprite + field::CORE_SPRITE_ORIGIN_X);
            *pos = (*pos + ox) * sx / sy - ox;
            *scale = sy;
            draw(sprite, pos, w, h);
            *scale = sx;
        } else {
            draw(sprite, pos, w, h);
        }
    }
}
