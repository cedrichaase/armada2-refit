//! Roots' Advanced Settings (`TAdvSettingsForm`: the two log switches) as a small settings
//! window of Armada II's kind: the log check boxes showing the state Roots' controls hold,
//! the form's note under them, Previous Menu at the foot. No button of Roots' Options
//! opens this form (Roots keeps it for a build that does), so the bench cannot reach it.
use super::kit::{self, Column};
use crate::engine::api::forms::Form;
use crate::menus::screens::form as sform;
use crate::widgets::Ui;

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let checks: alloc::vec::Vec<_> = ["Armada2LogCheck", "FleetOpsLogCheck"].iter().filter_map(|n| kit::shown(f, n)).collect();
    if checks.is_empty() {
        return false;
    }
    let note = kit::text(f, "labelSettings");
    let (body, foot) = sform::frame(ui, 640.0, checks.len() as f32 * (kit::CHECK + kit::CHECK_GAP) + 60.0, "ADVANCED SETTINGS");
    let mut c = Column::new(body.x, body.y + ui.u(6.0), body.w);
    for k in checks {
        let row = c.row(ui, kit::CHECK, kit::CHECK_GAP);
        kit::check(ui, f, k, row);
    }
    if !note.is_empty() {
        c.skip(ui, 8.0);
        let row = c.row(ui, 48.0, 0.0);
        kit::note(ui, row, note);
    }
    kit::back(ui, f, foot, "buttonBack");
    true
}
