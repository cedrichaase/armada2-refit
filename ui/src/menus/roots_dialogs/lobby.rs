//! Roots' lobby (`InternetGameDlgProc`, adopted, opened by FleetOpsHook's import after
//! Local Area Network or Manual IP) and the small dialogs it opens through the executable's
//! own import (the game's name for Create Game, Help, a game's password, the map transfer).
//! Roots' lobby is Armada II's screen at 1.5 times its design size (1200x900 against
//! 800x600: the same widgets at the same places, scaled), so Armada II's spec draws it
//! (`screens::lobby::draw_with`, scale 1.5); the small dialogs are Armada II's plain forms
//! (`setup_kit::form`). Every action is the stock control's click, as a player's.
use super::Screen;
use crate::menus::screens::setup_kit;
use crate::widgets::Ui;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let (dw, _) = crate::engine::api::forms::design();
    crate::menus::screens::lobby::draw_with(ui, s, dw as f32 / 800.0, "LOBBY")
}

/// The game's name (Create Game): the name and the password fields, OK and Cancel, in a
/// window of ours over the lobby. Roots' two labels ("Game Name", "Password") are of a
/// label class the widget reader does not read (UI.log: `reads empty`), so their words are
/// ours, given in the order the fields stand (as Game Settings' captions, decision 256);
/// the fields are the stock edits, typed into as a player would.
pub fn game_name(ui: &mut Ui, s: &Screen) -> bool {
    let mut edits = setup_kit::visible(s, setup_kit::is_edit);
    let mut btns = setup_kit::visible(s, |c| setup_kit::is_button(c) && !c.text.trim().is_empty());
    if edits.len() != 2 || btns.len() != 2 {
        return setup_kit::form(ui, s, "");
    }
    edits.sort_by_key(|c| c.rect[1]);
    btns.sort_by_key(|c| c.rect[0]);
    let t = ui.theme;
    ui.draw.rect(ui.screen(), t.scrim);
    let row = ui.u(t.row_h + 4.0);
    let cap = ui.u(t.text_size + 8.0);
    let w = ui.u(560.0).min(ui.w - ui.u(32.0));
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + 2.0 * (cap + row + ui.u(t.gap * 2.0)) + ui.u(t.button_h + t.pad * 2.0);
    let r = crate::widgets::Anchor::Centre.place(ui.screen(), w, h, 0.0);
    let inner = ui.window(r, "CREATE GAME");
    let mut y = inner.y;
    for (c, word) in edits.iter().zip(["Game Name", "Password"]) {
        setup_kit::caption(ui, crate::gfx::R::new(inner.x, y, inner.w, cap), word);
        y += cap;
        setup_kit::edit(ui, s, c, crate::gfx::R::new(inner.x, y, inner.w, row));
        y += row + ui.u(t.gap * 2.0);
    }
    let bh = ui.u(t.button_h);
    let bw = ui.u(180.0);
    let by = inner.bottom() - bh;
    setup_kit::button(ui, s, btns[0], crate::gfx::R::new(inner.right() - 2.0 * bw - ui.u(t.gap), by, bw, bh), None);
    setup_kit::button(ui, s, btns[1], crate::gfx::R::new(inner.right() - bw, by, bw, bh), None);
    true
}

/// A game's password, asked of a joiner (`PasswordDlgProc`, FO-8): Armada II's small form
/// (`setup_kit::form`), the stock field and buttons, titled PASSWORD when the dialog's own
/// label does not give a title.
pub fn password(ui: &mut Ui, s: &Screen) -> bool {
    setup_kit::form(ui, s, "PASSWORD")
}
