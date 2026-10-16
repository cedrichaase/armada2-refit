//! Options -> Graphics Settings (`do_graphicsOptions`): the display mode and the display
//! driver as drop-downs over the stock combo boxes (ids 2102 and 2103: the screen draws
//! them itself and keeps the windows hidden, so a choice is the combo's own selection
//! and the notification a user's choice sends), gamma, the selection effect's opacity
//! and graphics detail as sliders, the switches as buttons (`form.rs`).
use super::form;
use crate::menus::Screen;
use crate::widgets::Ui;

/// The display mode and the display driver: the combo boxes' ids, and the place of the
/// caption that names each among those left of the middle (0 is the heading).
const COMBOS: &[(i32, usize)] = &[(2102, 1), (2103, 2)];

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let f = form::build(s, COMBOS);
    if f.cols.iter().all(|c| c.is_empty()) {
        return false;
    }
    form::draw(ui, s, &f);
    true
}
