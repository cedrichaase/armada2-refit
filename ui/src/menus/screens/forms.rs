//! The small multiplayer dialogs, each as one window of ours (`kit::form`): their
//! labels, edit boxes, lists and buttons, read from the model, in the stock order.
//!
//! - **manual ip** (`do_manual_ip`): the address prompt and its field, O.K. and Cancel.
//!   With `Online.asi`, *Internet - Online* opens the same dialog with its own prompt
//!   ("Join code or host address"), read from the dialog's own label like any other.
//! - **game name**, **create room** (`do_gameName`, `do_createRoom`): a name.
//! - **password** (`do_password`): a game's password.
//! - **map transfer** (`do_mapTransfer`): what is being sent, and Cancel.
//! - **room select** (`do_roomSelect`) and the GameSpy account screens
//!   (`do_gamespyLogin`, `do_createNewAccount`, `do_changePassword`): GameSpy is gone,
//!   so these are not reached any more; they are plain forms so that they still work
//!   if something reaches them.
//!
//! Every one acts only through the stock controls: a button is the stock button's
//! click, typing is sent to the stock edit, Enter and Esc reach the dialog as the
//! stock edit would give them (`IDOK`, `IDCANCEL`).
use super::kit;
use crate::menus::Screen;
use crate::widgets::Ui;

pub fn manual_ip(ui: &mut Ui, s: &Screen) -> bool {
    kit::form(ui, s, "")
}

pub fn game_name(ui: &mut Ui, s: &Screen) -> bool {
    kit::form(ui, s, "")
}

pub fn password(ui: &mut Ui, s: &Screen) -> bool {
    kit::form(ui, s, "")
}

pub fn map_transfer(ui: &mut Ui, s: &Screen) -> bool {
    kit::form(ui, s, "")
}

pub fn gamespy(ui: &mut Ui, s: &Screen) -> bool {
    kit::form(ui, s, "GameSpy")
}
