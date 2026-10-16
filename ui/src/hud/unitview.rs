//! Our frame around the unit view (`[HUD] UnitView=1`; HUD.md, "The unit view"). The
//! engine keeps drawing `CinematicView`: its picture is a second render of the scene
//! (render3d/README.md, the action camera), or a video, which only the engine makes.
//! Ours is the chrome around it, in the race's theme, drawn just outside its
//! rectangle so nothing of the picture is covered.
use crate::engine::{hud, overlay};
use crate::gfx::{fade, R};
use crate::widgets::Ui;

pub fn run(ui: &mut Ui) {
    let c = overlay::CINEMATIC_VIEW;
    if hud::hidden(c) || crate::engine::ingame::hud_hidden() {
        return;
    }
    let Some(p) = hud::panel_rect(c, ui.w, ui.h) else { return };
    let r = R::new(p[0], p[1], p[2] - p[0], p[3] - p[1]);
    if r.w < 8.0 || r.h < 8.0 {
        return;
    }
    let t = ui.theme;
    let e = ui.u(t.edge).max(1.0);
    let g = ui.u(3.0);
    let outer = r.inset(-g - e);
    let corners = t.small.scaled(ui.scale);
    ui.draw.shadow(outer, corners, (0.0, ui.u(4.0)), ui.u(18.0), fade(t.shadow, 0.6));
    ui.draw.stroke(outer, corners, g + e, fade(t.panel, 0.95));
    ui.draw.stroke(outer, corners, e, t.panel_edge);
    // The form's accent at two opposite corners.
    let (len, w) = (ui.u(18.0), ui.u(2.5).max(1.0));
    for (x, y, sx, sy) in [(outer.x, outer.y, 1.0, 1.0), (outer.right(), outer.bottom(), -1.0, -1.0)] {
        let hx = if sx > 0.0 { x } else { x - len };
        let vy = if sy > 0.0 { y } else { y - len };
        ui.draw.rect(R::new(hx, if sy > 0.0 { y } else { y - w }, len, w), t.accent);
        ui.draw.rect(R::new(if sx > 0.0 { x } else { x - w }, vy, w, len), t.accent);
    }
}
