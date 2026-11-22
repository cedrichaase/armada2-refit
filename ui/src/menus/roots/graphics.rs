//! Roots' Graphics Settings (`TGraphicOptionsForm`) as Armada II's graphics screen
//! (`screens::graphics_settings`): the display driver, the resolution and the
//! anti-aliasing as dropdowns over the form's combo boxes, the graphics detail and the
//! gamma as sliders, the switches as check boxes showing the state Roots' own controls
//! hold. Previous Menu is Roots' own (it saves the profile).
use super::kit::{self, Column};
use crate::engine::api::forms::Form;
use crate::menus::screens::form as sform;
use crate::widgets::Ui;

/// The switches, in the order they stand on our screen.
const CHECKS: &[&str] =
    &["WindowModeCheck", "TeamColorsCheck", "CinematicWindowCheck", "HardwareVertexCheck", "BumpMappingCheck", "AsyncCursorCheck", "AsyncCursorWindowCheck"];

pub fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(driver), Some(modes), Some(aa)) =
        (kit::shown(f, "comboDevicePrimary"), kit::shown(f, "comboDevicePrimaryResolutions"), kit::shown(f, "comboDevicePrimaryAA"))
    else {
        return false;
    };
    let checks: alloc::vec::Vec<_> = CHECKS.iter().filter_map(|n| kit::shown(f, n)).collect();
    let sliders = [("labelGraphicDetail", "GraphicDetailSlider"), ("labelGamma", "GammaCorrectionSlider")];
    let combo_h = kit::LABEL + kit::FIELD + kit::GAP;
    let slider_h = kit::LABEL + kit::SLIDER + kit::GAP;
    let left = 3.0 * combo_h + sliders.len() as f32 * slider_h;
    let note = kit::text(f, "labelUnsupported");
    let right = checks.len() as f32 * (kit::CHECK + kit::CHECK_GAP) + if note.is_empty() { 0.0 } else { 48.0 };
    let (body, foot) = sform::frame(ui, 1180.0, left.max(right), "GRAPHICS");
    let gap = ui.u(40.0);
    let cw = (body.w - gap) * 0.5;
    let top = body.y + ui.u(6.0);

    let mut l = Column::new(body.x, top, cw);
    for (label, c) in [("labelDisplayDriver", driver), ("labelDisplayMode", modes), ("labelAA", aa)] {
        let r = l.field(ui, kit::text(f, label), kit::FIELD);
        kit::combo(ui, f, c, r);
    }
    for (label, name) in sliders {
        let Some(c) = kit::shown(f, name) else { continue };
        let r = l.field(ui, kit::text(f, label), kit::SLIDER);
        kit::slider(ui, f, c, r);
    }

    let mut r = Column::new(body.x + cw + gap, top, cw);
    for c in checks {
        let row = r.row(ui, kit::CHECK, kit::CHECK_GAP);
        kit::check(ui, f, c, row);
    }
    if !note.is_empty() {
        r.skip(ui, 8.0);
        let row = r.row(ui, 40.0, 0.0);
        kit::note(ui, row, note);
    }
    kit::back(ui, f, foot, "buttonBack");
    true
}
