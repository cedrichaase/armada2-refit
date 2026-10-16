//! The unit view placed by the UI (`[HUD] UnitView=1`; HUD.md, "The unit view").
//!
//! `CinematicView` (`g_cinematicView`) keeps drawing itself: a second render of the
//! scene, a Bink video, or its still picture. Where is read from the component's own
//! fields every frame, all relative to its rectangle `+0x04` (left, top, right,
//! bottom, inclusive, in the HUD's 1600 x 1200 space): `RenderView` (and the video)
//! take the view's rectangle as that corner plus `cinematicDisplayArea` at `+0x48`,
//! converted to the screen by `DisplayInterface::ConvertRectangle`, and give it to
//! the view's camera as its viewport; the still picture is drawn at the corner plus
//! `cinematicBackgroundArea` at `+0x38`; `CursorOver` tests the pointer against
//! `+0x04`. So the UI places the view by writing those three rectangles: the panel is
//! the view, with both areas at offset 0 and the panel's size.
//!
//! The stock frame around it (`cinematicBackgroundPanel`, a `StandardBackground` at
//! `+0x58`, its tiles placed from the same corner and sized for the stock 350 x 384
//! panel) is drawn by `CinematicView::Render` after the view. Ours draws a frame of the
//! theme instead (`hud/unitview.rs`), so for that one call the frame's pointer is
//! swapped for an inert object whose `Render` (vtable `+0x10`, the only call made on
//! it there) does nothing, and put back after.
use super::{field, rd};

/// `g_cinematicView` (an instance).
pub const CINEMATIC_VIEW: usize = 0x763758;
/// Its vtable, and its `Render` (slot `+0x58`), checked to be stock.
const CV_VTABLE: usize = 0x6b47b4;
const CV_RENDER: usize = 0x4e5d30;
const VT_RENDER: usize = 0x58;
const CV_RECT: usize = 0x04;
const CV_BACKGROUND_AREA: usize = 0x38;
const CV_DISPLAY_AREA: usize = 0x48;
const CV_FRAME: usize = 0x58;
/// `StandardBackground`'s vtable, and its `Render` at `+0x10`.
const SB_VTABLE: usize = 0x6b561c;
const SB_RENDER: usize = 0x50a820;
const SB_SLOT_RENDER: usize = 0x10;

static mut ON: bool = false;
static mut INERT_VT: [usize; 32] = [0; 32];
static mut INERT: [usize; 16] = [0; 16];
static mut LOGGED: [i32; 4] = [0; 4];

unsafe extern "thiscall" fn inert(_this: usize) {}

/// Whether the sites the placement relies on are stock.
pub fn functions_stock() -> bool {
    let slots = unsafe { rd::<usize>(CV_VTABLE + VT_RENDER) == CV_RENDER && rd::<usize>(SB_VTABLE + SB_SLOT_RENDER) == SB_RENDER };
    slots && super::starts(CV_RENDER, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x24])
}

/// Switches the placement on: from now on the panel switch draws the view through
/// `render`.
#[allow(static_mut_refs)]
pub fn enable() {
    unsafe {
        let f = inert as *const () as usize;
        for s in INERT_VT.iter_mut() {
            *s = f;
        }
        INERT[0] = core::ptr::addr_of!(INERT_VT) as usize;
        ON = true;
    }
}

pub fn enabled() -> bool {
    unsafe { ON }
}

/// The view's rectangle in back-buffer pixels as the engine draws it (the panel's
/// rectangle plus the display area), or None while there is none.
pub fn view_rect(w: f32, h: f32) -> Option<[f32; 4]> {
    let c = CINEMATIC_VIEW;
    let p: [i32; 4] = unsafe { rd(c + CV_RECT) };
    let a: [i32; 4] = unsafe { rd(c + CV_DISPLAY_AREA) };
    let (l, t, r, b) = (p[0] + a[0], p[1] + a[1], p[0] + a[2], p[1] + a[3]);
    if r <= l || b <= t {
        return None;
    }
    let (sx, sy) = (w / 1600.0, h / 1200.0);
    Some([l as f32 * sx, t as f32 * sy, (r + 1) as f32 * sx, (b + 1) as f32 * sy])
}

/// Places the view at `px` (left, top, right, bottom in back-buffer pixels of a `w` x
/// `h` buffer): the three rectangles written in the 1600 x 1200 space, whole units
/// covering the pixels.
pub fn place(px: [f32; 4], w: f32, h: f32) {
    if !enabled() || w < 1.0 || h < 1.0 {
        return;
    }
    let (sx, sy) = (1600.0 / w, 1200.0 / h);
    let l = libm::floorf(px[0] * sx) as i32;
    let t = libm::floorf(px[1] * sy) as i32;
    let r = (libm::ceilf(px[2] * sx) as i32 - 1).max(l + 1);
    let b = (libm::ceilf(px[3] * sy) as i32 - 1).max(t + 1);
    let c = CINEMATIC_VIEW;
    unsafe {
        *((c + CV_RECT) as *mut [i32; 4]) = [l, t, r, b];
        let area = [0, 0, r - l, b - t];
        *((c + CV_BACKGROUND_AREA) as *mut [i32; 4]) = area;
        *((c + CV_DISPLAY_AREA) as *mut [i32; 4]) = area;
        let now = [l, t, r, b];
        if *core::ptr::addr_of!(LOGGED) != now {
            LOGGED = now;
            crate::log!("unit view: placed at [{},{},{},{}] of 1600x1200 ({:.0},{:.0} {:.0}x{:.0} px)", l, t, r, b, px[0], px[1], px[2] - px[0], px[3] - px[1]);
        }
    }
}

/// `CinematicView::Render` with the stock frame left out (the panel switch calls this
/// in place of the component's `Render` while the placement is on).
pub fn render(c: usize) {
    unsafe {
        let render: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(rd::<usize>(c) + VT_RENDER));
        let slot = (c + CV_FRAME) as *mut usize;
        let frame = *slot;
        if frame == 0 || field(frame, 0) != SB_VTABLE {
            return render(c);
        }
        *slot = core::ptr::addr_of!(INERT) as usize;
        render(c);
        *slot = frame;
    }
}
