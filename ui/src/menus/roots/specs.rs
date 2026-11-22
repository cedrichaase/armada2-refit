//! Roots' screen specs: one file per form, each laid out as the Armada II screen it
//! corresponds to (MENUS.md, "Roots' forms"). A spec draws when the controls it expects
//! are on the form and returns false otherwise, so the generic widgets draw the form
//! instead.
use crate::engine::api::forms::Form;
use crate::widgets::Ui;

pub(super) fn draw(ui: &mut Ui, _fit: &super::Fit, f: &Form) -> bool {
    match f.class.as_str() {
        "TGraphicOptionsForm" => super::graphics::draw(ui, f),
        "TSoundSettingsForm" => super::sound::draw(ui, f),
        "TModSettingsForm" => super::mods::draw(ui, f),
        "TMapSelectForm" => super::mapselect::draw(ui, f),
        "TAdvSettingsForm" => super::advsettings::draw(ui, f),
        "TMapEditForm" => super::mapedit::draw(ui, f),
        "TMapEditCreateForm" => super::mapeditcreate::draw(ui, f),
        "TMapSaveForm" => super::mapsave::draw(ui, f),
        "TGameObjectDialogForm" => super::objdlg::draw(ui, f),
        "TMapOptionsDialogForm" => super::mapopts::draw(ui, f),
        "TMessageForm" => super::msgdlg::draw(ui, f),
        "TMultiplayerConnectionForm" => super::multiplayer::draw(ui, f),
        "TManualIPForm" => super::manualip::draw(ui, f),
        "TAdvancedOptionsForm" => super::advanced::draw(ui, f),
        _ => false,
    }
}
