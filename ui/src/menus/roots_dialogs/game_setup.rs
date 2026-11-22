//! Roots' Game Setup (`MultiplayerSetupDlgProc`, adopted: Instant Action's setup and the
//! multiplayer games'): Armada II's Game Setup (`screens::game_setup::draw_with`, the
//! players' window with our drop-downs over the stock `ComboBox`es, the map's window with
//! the preview and the settings, the bottom row) over Roots' layout of the same stock
//! controls, at Roots' places in its 1200x900 design space (`ROOTS`, measured on the
//! bench's model of the dialog). FleetOpsHook moves the stock controls but keeps their
//! classes, so the shared code finds them by kind and place as on Armada II; Roots' column
//! headings are painted, not labels, so the columns are named by Armada II's own words.
//! Every choice is the stock control's clicks (a `ComboBox` opened and its item clicked,
//! `setup_kit::choose`), the pointer resting first; SELECT MAP opens Roots' Map Select
//! form, Advanced its Advanced Options form.
use super::Screen;
use crate::menus::screens::game_setup::{draw_with, Layout};
use crate::widgets::Ui;

/// Roots' screen: the table's rows at y 103 to 313 (30 apart), the player, side and team
/// combos at x 70, 219 and 536, a swatch (24 square) at x 45 before each row, the palette
/// at y 414 with Random Color at x 127, the chat log a multi-line edit at x 907 right of
/// the map's settings, the map's labels at x 744 from y 81, SELECT MAP and the toggles at
/// x 744 beside the preview (934,194), the other settings at x 589, the bottom row at
/// y 862 (decision 257).
pub const ROOTS: Layout = Layout {
    table_right: 700,
    table_bottom: 400,
    cols: [60, 200, 380, 520],
    armada: (380, 520),
    heads_y: (60, 100),
    swatch: (45, 24),
    palette_bottom: 450,
    random_right: 300,
    log_right: i32::MAX,
    target_y: 750,
    info_x: 700,
    info_bottom: 150,
    set_x: 580,
    set_top: 180,
    beside_x: 720,
    preview: [934, 194, 241, 239],
    bottom_row: 840,
    title_bottom: 45,
};

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    draw_with(ui, s, &ROOTS)
}
