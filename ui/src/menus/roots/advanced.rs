//! Roots' Advanced Options (`TAdvancedOptionsForm`, "Game Settings" on its own picture) as
//! Armada II's Advanced Options (`screens::advanced_options`): the four choices (starting
//! resources, cease fire, AI personality, battleship limit) as dropdowns over the form's
//! combo boxes, the switches that show as check boxes with the state Roots' controls hold,
//! and the six sliders with their end captions at their values. OK, Cancel and Default are
//! Roots' own buttons (OK writes the game setup). The four choices' captions are painted
//! into Roots' picture, not controls: they are the words Roots paints there.
use super::kit::{self, Column};
use crate::engine::api::forms::Form;
use crate::gfx::{Weight, R};
use crate::menus::screens::form as sform;
use crate::widgets::{Align, Ui};

const COMBOS: &[(&str, &str)] =
    &[("cbbStartingResource", "Starting Resources"), ("cbbCeaseFire", "Cease Fire"), ("cbbTechlevel", "AI Personality Type"), ("cbbOfficer", "Battleship Build Limit")];
const CHECKS: &[&str] = &[
    "chkGive", "chkFerengi", "chkSpecialWeapons", "chkRandomMap", "chkObservers", "chkGameLocked", "chkInstantTrade", "chkMothership", "chkWarpSpeed", "chkInfiniteMoons",
    "chkTrekPhysics",
];
/// Each slider: its title, its end captions (low, high) and itself.
const SLIDERS: &[(&str, &str, &str, &str)] = &[
    ("lblGameSpeedSliderTitle", "lblGameSpeedSliderMinus", "lblGameSpeedSliderPlus", "sldrGameSpeed"),
    ("lblDamageModifierSliderTitle", "lblDamageModifierSliderMinus", "lblDamageModifierSliderPlus", "sldrDamageModifier"),
    ("lblBuildTimeModifierSliderTitle", "lblBuildTimeModifierSliderMinus", "lblBuildTimeModifierSliderPlus", "sldrBuildTimeModifier"),
    ("lblBuildTimeSliderTitle", "lblBuildTimeSliderMinus", "lblBuildTimeSliderPlus", "sldrBuildTime"),
    ("lblResourceCostSliderTitle", "lblResourceCostSliderMinus", "lblResourceCostSliderPlus", "sldrResourceCost"),
    ("lblUnitCostSliderTitle", "lblUnitCostSliderMinus", "lblUnitCostSliderPlus", "sldrUnitCost"),
    ("lblMapHeightTitle", "lblMapHeightMinus", "lblMapHeightPlus", "sldrMapHeight"),
];

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(ok), Some(cancel), Some(defaults)) = (kit::shown(f, "btnAccept"), kit::shown(f, "btnClose"), kit::shown(f, "btnLoadDefaults")) else { return false };
    let combos: alloc::vec::Vec<_> = COMBOS.iter().filter_map(|(n, l)| kit::shown(f, n).map(|c| (c, *l))).collect();
    let checks: alloc::vec::Vec<_> = CHECKS.iter().filter_map(|n| kit::shown(f, n)).collect();
    let sliders: alloc::vec::Vec<_> = SLIDERS.iter().filter_map(|s| kit::shown(f, s.3).filter(|c| c.value.is_some()).map(|c| (c, *s))).collect();
    if combos.is_empty() || sliders.is_empty() {
        return false;
    }
    // A modal over the setup screen: that one is dimmed.
    let full = ui.screen();
    let scrim = ui.theme.scrim;
    ui.draw.rect(full, scrim);
    let combo_h = kit::LABEL + kit::FIELD + kit::GAP;
    let slider_h = kit::LABEL + kit::SLIDER + sform::CAP_H + kit::GAP;
    let left = combos.len() as f32 * combo_h + checks.len() as f32 * (kit::CHECK + kit::CHECK_GAP);
    let right = sliders.len() as f32 * slider_h;
    let (body, foot) = sform::frame(ui, 1180.0, left.max(right), "GAME SETTINGS");
    let gap = ui.u(40.0);
    let cw = (body.w - gap) * 0.5;
    let top = body.y + ui.u(6.0);
    let t = ui.theme;

    let mut l = Column::new(body.x, top, cw);
    for (c, label) in combos {
        let r = l.field(ui, label, kit::FIELD);
        kit::combo(ui, f, c, r);
    }
    for c in checks {
        let row = l.row(ui, kit::CHECK, kit::CHECK_GAP);
        kit::check(ui, f, c, row);
    }

    let mut r = Column::new(body.x + cw + gap, top, cw);
    for (c, (title, low, high, _)) in sliders {
        let row = r.field(ui, kit::text(f, title), kit::SLIDER);
        kit::slider(ui, f, c, row);
        let cap = R::new(row.x, row.bottom(), row.w, ui.u(sform::CAP_H));
        ui.text_in(cap, kit::text(f, low), t.small_size, Align::Left, t.text_dim, Weight::Regular);
        ui.text_in(cap, kit::text(f, high), t.small_size, Align::Right, t.text_dim, Weight::Regular);
        r.skip(ui, sform::CAP_H);
    }

    let (w, h, g) = (ui.u(220.0), ui.u(44.0), ui.u(12.0));
    let y = foot.bottom() - h;
    kit::button(ui, f, ok, R::new(foot.x, y, w, h));
    kit::button(ui, f, cancel, R::new(foot.x + w + g, y, w, h));
    kit::button(ui, f, defaults, R::new(foot.right() - w, y, w, h));
    true
}
