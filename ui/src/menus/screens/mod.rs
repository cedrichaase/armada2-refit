//! Screen specs: a layout of our own for a stock screen, chosen by the screen's name
//! (`engine::dialogs::screen_name`, which names each shell dialog procedure).
//!
//! A screen of ours is a `Screen`: the key of the dialog it lays out and a draw that
//! lays it out from `menus::Screen` (the model, the mirror, the design fit) and acts
//! through it (`menus::Screen::click_control`, `menus::actions`). The draw returns false
//! to have the generic screen draw the dialog this frame instead (for one it does not
//! recognise, such as a model without the controls it expects). Most are a `Spec`, a
//! key and a function, in `SPECS`. Adding one: a file here, an entry in `SPECS`;
//! MENUS.md, "Writing a screen spec", has the pattern step by step.
use super::Screen as Dialog;
use crate::widgets::Ui;

pub mod admirals_log;
pub mod advanced_options;
pub mod common;
pub mod form;
pub mod forms;
pub mod game_settings;
pub mod game_setup;
pub mod graphics_settings;
pub mod kit;
// ui-multiplayer: the Internet lobby.
pub mod lobby;
pub mod main_menu;
pub mod map_select;
pub mod mission_select;
pub mod multiplayer;
pub mod notice;
pub mod options;
pub mod saves;
pub mod setup_kit;
pub mod single_player;
pub mod sound_settings;
pub mod tech_tree;
pub mod text_window;

/// A menu screen of ours: the stock dialog it lays out, and how.
pub trait Screen: Sync {
    /// The dialog it lays out, by its key: the screen's name, as
    /// `engine::api::dialogs::screen_name` names the dialog procedure.
    fn key(&self) -> &'static str;
    /// Draws the dialog `s` and reads its input; false to have the generic screen draw
    /// it this frame instead.
    fn draw(&self, ui: &mut Ui, s: &Dialog) -> bool;
}

/// A screen as a key and a function: what every screen of ours is today.
pub struct Spec {
    /// The screen it lays out, as `engine::api::dialogs::screen_name` names it.
    pub screen: &'static str,
    pub draw: fn(&mut Ui, &Dialog) -> bool,
}

impl Screen for Spec {
    fn key(&self) -> &'static str {
        self.screen
    }
    fn draw(&self, ui: &mut Ui, s: &Dialog) -> bool {
        (self.draw)(ui, s)
    }
}

pub const SPECS: &[Spec] = &[
    Spec { screen: "main menu", draw: main_menu::draw },
    Spec { screen: "single player", draw: single_player::draw },
    Spec { screen: "mission select", draw: mission_select::draw },
    Spec { screen: "load game", draw: saves::draw_load },
    Spec { screen: "save game", draw: saves::draw_save },
    Spec { screen: "admiral's log", draw: admirals_log::draw },
    Spec { screen: "technology tree", draw: tech_tree::draw },
    Spec { screen: "options", draw: options::draw },
    Spec { screen: "game settings", draw: game_settings::draw },
    Spec { screen: "graphics settings", draw: graphics_settings::draw },
    Spec { screen: "sound settings", draw: sound_settings::draw },
    Spec { screen: "message", draw: notice::draw },
    Spec { screen: "yes or no", draw: notice::draw },
    Spec { screen: "net waiting", draw: notice::draw },
    Spec { screen: "help", draw: text_window::draw },
    Spec { screen: "licence agreement", draw: text_window::draw },
    // Instant Action's and the multiplayer screens (menus-setup).
    Spec { screen: "game setup", draw: game_setup::draw },
    Spec { screen: "advanced options", draw: advanced_options::draw },
    Spec { screen: "map select", draw: map_select::draw },
    Spec { screen: "multiplayer connection", draw: multiplayer::connection },
    Spec { screen: "manual ip", draw: forms::manual_ip },
    Spec { screen: "game name", draw: forms::game_name },
    Spec { screen: "create room", draw: forms::game_name },
    Spec { screen: "password", draw: forms::password },
    Spec { screen: "map transfer", draw: forms::map_transfer },
    Spec { screen: "room select", draw: forms::gamespy },
    Spec { screen: "gamespy login", draw: forms::gamespy },
    Spec { screen: "create account", draw: forms::gamespy },
    Spec { screen: "change password", draw: forms::gamespy },
    // ui-multiplayer: the Internet lobby.
    Spec { screen: "internet game", draw: lobby::draw },
];

/// The screen of ours for the dialog keyed `screen`, if there is one.
pub fn find(screen: &str) -> Option<&'static dyn Screen> {
    if screen.is_empty() {
        return None;
    }
    SPECS.iter().find(|s| s.key() == screen).map(|s| s as &dyn Screen)
}
