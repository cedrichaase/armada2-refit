//! Roots' screen specs: one file per form, each laid out as the Armada II screen it
//! corresponds to (MENUS.md, "Roots' forms"). A spec draws when the controls it expects
//! are on the form and returns false otherwise, so the generic widgets draw the form
//! instead.
use crate::engine::api::forms::Form;
use crate::widgets::Ui;

pub(super) fn draw(ui: &mut Ui, fit: &super::Fit, f: &Form) -> bool {
    match f.class.as_str() {
        "TGraphicOptionsForm" => super::graphics::draw(ui, f),
        "TSoundSettingsForm" => super::sound::draw(ui, f),
        "TModSettingsForm" => super::mods::draw(ui, f),
        "TAdvSettingsForm" => super::advsettings::draw(ui, f),
        _ => false,
    }
}
