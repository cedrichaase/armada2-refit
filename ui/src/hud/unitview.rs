//! The unit view, placed by us and framed in the theme (`[HUD] UnitView=1`; HUD.md,
//! "The unit view"). The engine keeps drawing `CinematicView`: its picture is a second
//! render of the scene (render3d/README.md, the action camera), or a video, which only
//! the engine makes. Ours says where (`engine::unitview::place`, through the
//! component's own rectangles, so the view's camera gets a square viewport at any
//! aspect) and draws the chrome around it, just outside, so nothing of the picture is
//! covered; the stock frame is left out.
use crate::engine::{hud, overlay, unitview};
use crate::gfx::{fade, Corners, R};
use crate::widgets::Ui;

/// Where the view goes: a square of `[HUD] UnitViewSize` units at the bottom right,
/// clear of the frame drawn around it.
pub fn place(ui: &Ui) -> R {
    let side = ui.u(crate::config::get().hud.unit_view_size).min(ui.h * 0.4);
    let m = ui.u(10.0) + frame_width(ui);
    R::new(libm::roundf(ui.w - m - side), libm::roundf(ui.h - m - side), libm::roundf(side), libm::roundf(side))
}

fn frame_width(ui: &Ui) -> f32 {
    ui.u(ui.theme.edge).max(1.0) + ui.u(3.0)
}

static mut SHOWN: bool = false;

pub fn run(ui: &mut Ui) {
    let c = overlay::CINEMATIC_VIEW;
    if unitview::enabled() {
        let r = place(ui);
        unitview::place([r.x, r.y, r.right(), r.bottom()], ui.w, ui.h);
    }
    let shown = !hud::hidden(c) && !crate::engine::ingame::hud_hidden();
    unsafe {
        if shown != SHOWN {
            SHOWN = shown;
            crate::log!("unit view: {}", if shown { "shown" } else { "hidden" });
        }
    }
    if !shown {
        return;
    }
    // In the cinematic view (a cutscene) the engine draws the unit view only while it
    // plays a video (`CinematicView::Render` returns at once otherwise): no frame then.
    if crate::engine::radar::view() == 2 && !hud::video_playing() {
        return;
    }
    let p = if unitview::enabled() { unitview::view_rect(ui.w, ui.h) } else { hud::panel_rect(c, ui.w, ui.h) };
    let Some(p) = p else { return };
    let r = R::new(p[0], p[1], p[2] - p[0], p[3] - p[1]);
    if r.w < 8.0 || r.h < 8.0 {
        return;
    }
    let t = ui.theme;
    let e = ui.u(t.edge).max(1.0);
    let g = ui.u(3.0);
    let outer = r.inset(-g - e);
    super::layout::note(outer);
    // Square, whatever the theme's corners: the picture is square, and a rounded
    // frame (the LCARS theme's) cut into its corners.
    ui.draw.stroke(outer, Corners::SQUARE, g + e, fade(t.panel, 0.95));
    ui.draw.stroke(outer, Corners::SQUARE, e, t.panel_edge);
    // The form's accent at two opposite corners.
    let (len, w) = (ui.u(18.0), ui.u(2.5).max(1.0));
    for (x, y, sx, sy) in [(outer.x, outer.y, 1.0, 1.0), (outer.right(), outer.bottom(), -1.0, -1.0)] {
        let hx = if sx > 0.0 { x } else { x - len };
        let vy = if sy > 0.0 { y } else { y - len };
        ui.draw.rect(R::new(hx, if sy > 0.0 { y } else { y - w }, len, w), t.accent);
        ui.draw.rect(R::new(if sx > 0.0 { x } else { x - w }, vy, w, len), t.accent);
    }
}
