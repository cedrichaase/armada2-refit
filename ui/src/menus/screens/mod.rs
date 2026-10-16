//! Screen specs: a layout of our own for a stock screen, chosen by the screen's name
//! (`engine::dialogs::screen_name`, which names each shell dialog procedure).
//!
//! A spec is a function that draws the screen from `Screen` (the model, the mirror,
//! the design fit) and acts through it (`Screen::click_control`, `menus::actions`). It
//! returns false to have the generic screen draw the dialog this frame instead (for
//! one it does not recognise, such as a model without the controls it expects). Adding
//! one: a file here, an entry in `SPECS`; MENUS.md, "Writing a screen spec", has the
//! pattern step by step.
use super::Screen;
use crate::widgets::Ui;

pub mod advanced_options;
pub mod forms;
pub mod game_setup;
pub mod kit;
pub mod main_menu;
pub mod map_select;
pub mod multiplayer;

pub struct Spec {
    /// The screen it lays out, as `engine::dialogs::screen_name` names it.
    pub screen: &'static str,
    pub draw: fn(&mut Ui, &Screen) -> bool,
}

pub const SPECS: &[Spec] = &[
    Spec { screen: "main menu", draw: main_menu::draw },
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
];

pub fn find(screen: &str) -> Option<&'static Spec> {
    if screen.is_empty() {
        return None;
    }
    SPECS.iter().find(|s| s.screen == screen)
}
