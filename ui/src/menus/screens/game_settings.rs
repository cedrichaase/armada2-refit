//! Options -> Game Settings (`do_gameSettings`): the scroll speeds, the hold level and
//! the AI handicaps as sliders (the stock `ShellScrollBar`s), the difficulty and the
//! interface mode as buttons that step through their choices (`form.rs`).
use super::form;
use crate::menus::Screen;
use crate::widgets::Ui;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let f = form::build(s, &[]);
    if f.cols.iter().all(|c| c.is_empty()) {
        return false;
    }
    form::draw(ui, s, &f);
    true
}
