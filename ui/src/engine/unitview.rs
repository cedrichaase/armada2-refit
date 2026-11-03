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
use crate::syms_info::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

fn at(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

/// `g_cinematicView` (an instance): its address, or 0 while the unit view's hook is not
/// ready.
pub fn cinematic_view() -> usize {
    at(sym::UNITVIEW_CINEMATIC_VIEW)
}

/// The slots of `CinematicView`'s vtable (`Render`) and `StandardBackground`'s (`Render`)
/// are checked by the hook to be stock; the code never reads them.
#[allow(dead_code)]
const CHECKED: [a2engine::Sym; 2] = [sym::UNITVIEW_SLOT_CV_RENDER, sym::UNITVIEW_SLOT_SB_RENDER];
const VT_RENDER: usize = field::UNITVIEW_VT_RENDER;
const CV_RECT: usize = field::UNITVIEW_CV_RECT;
const CV_BACKGROUND_AREA: usize = field::UNITVIEW_CV_BACKGROUND_AREA;
const CV_DISPLAY_AREA: usize = field::UNITVIEW_CV_DISPLAY_AREA;
const CV_FRAME: usize = field::UNITVIEW_CV_FRAME;

static mut ON: bool = false;
static mut INERT_VT: [usize; 32] = [0; 32];
static mut INERT: [usize; 16] = [0; 16];
static mut LOGGED: [i32; 4] = [0; 4];

unsafe extern "thiscall" fn inert(_this: usize) {}

/// Whether the sites the placement relies on are stock (the unit view's hook is ready).
pub fn functions_stock() -> bool {
    eng().ready(hook::HUD_UNITVIEW).is_ok()
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
    let c = cinematic_view();
    if c == 0 {
        return None;
    }
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
    let c = cinematic_view();
    if c == 0 {
        return;
    }
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
        let sb_vtable = at(sym::UNITVIEW_SB_VTABLE);
        if frame == 0 || sb_vtable == 0 || field(frame, 0) != sb_vtable {
            return render(c);
        }
        *slot = core::ptr::addr_of!(INERT) as usize;
        render(c);
        *slot = frame;
    }
}
