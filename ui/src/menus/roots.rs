//! A frame of ours over one of Roots' front-end forms (PLAN-FLEETOPS.md, FO-6 step 7,
//! option C). Step 0, the probe: the form's controls as read from its live objects, each
//! button a button of ours that posts a press to the form; nothing of Roots' art.
use crate::engine::api::forms;
use crate::gfx::R;
use crate::widgets::Ui;

pub fn frame(ui: &mut Ui) {
    let ((w, h), controls) = forms::snapshot();
    if w <= 0 || h <= 0 {
        return;
    }
    // The form's client area, fitted to the back buffer as the host fits its design space.
    let s = (ui.w / w as f32).min(ui.h / h as f32);
    let (ox, oy) = (libm::floorf((ui.w - w as f32 * s) * 0.5), libm::floorf((ui.h - h as f32 * s) * 0.5));
    let at = |r: [i32; 4]| R::new(ox + r[0] as f32 * s, oy + r[1] as f32 * s, r[2] as f32 * s, r[3] as f32 * s);
    let area = R::new(ox, oy, w as f32 * s, h as f32 * s);
    let inner = ui.window(area, "ROOTS FORM (PROBE)");
    ui.note(R::new(inner.x, inner.y, inner.w, ui.u(24.0)), &crate::sfmt!("{} controls read from the live form", controls.len()));
    for (i, c) in controls.iter().enumerate() {
        if !c.visible || c.rect[2] <= 0 || c.rect[3] <= 0 || c.class == "TImage" {
            continue;
        }
        let r = at(c.rect);
        let label = if c.text.is_empty() { c.name.as_str() } else { c.text.as_str() };
        if c.class == "TJetButton" {
            if ui.scope(&crate::sfmt!("c{}", i), |ui| ui.button(r, label)) {
                forms::press(c);
            }
        } else {
            ui.note(r, label);
        }
    }
}
