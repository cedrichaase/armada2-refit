//! Options -> Sound Settings (`do_soundSettings`): the four volumes as sliders driving
//! the stock `ShellScrollBar`s, the channel count, unit speech and hardware
//! acceleration as buttons that step through their choices (`form.rs`).
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
