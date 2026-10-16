//! Options -> Game Settings (`do_gameSettings`): the scroll speeds, the hold level and
//! the AI handicaps as sliders (the stock `ShellScrollBar`s), the difficulty and the
//! interface mode as buttons that step through their choices (`form.rs`).
//!
//! One row is ours: "Command Bar", shown or hidden, the toggle that was the top-right
//! palette button in a mission (`hud/buttons.rs`), kept between sessions in `prefs`
//! and applied by the HUD at the next mission frame.
use super::form;
use crate::menus::Screen;
use crate::widgets::Ui;

fn toggle_bar() {
    crate::prefs::set_bar_hidden(!crate::prefs::bar_hidden());
    crate::hud::bar_pref_changed();
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let f = form::build(s, &[]);
    if f.cols.iter().all(|c| c.is_empty()) {
        return false;
    }
    let own = [form::Own { label: "Command Bar", value: if crate::prefs::bar_hidden() { "Hidden" } else { "Shown" }, press: toggle_bar }];
    form::draw_with(ui, s, &f, &own);
    true
}
