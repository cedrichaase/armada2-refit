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

pub mod common;
pub mod form;
pub mod game_settings;
pub mod graphics_settings;
pub mod main_menu;
pub mod notice;
pub mod options;
pub mod sound_settings;
pub mod text_window;

pub struct Spec {
    /// The screen it lays out, as `engine::dialogs::screen_name` names it.
    pub screen: &'static str,
    pub draw: fn(&mut Ui, &Screen) -> bool,
}

pub const SPECS: &[Spec] = &[
    Spec { screen: "main menu", draw: main_menu::draw },
    Spec { screen: "options", draw: options::draw },
    Spec { screen: "game settings", draw: game_settings::draw },
    Spec { screen: "graphics settings", draw: graphics_settings::draw },
    Spec { screen: "sound settings", draw: sound_settings::draw },
    Spec { screen: "message", draw: notice::draw },
    Spec { screen: "yes or no", draw: notice::draw },
    Spec { screen: "net waiting", draw: notice::draw },
    Spec { screen: "help", draw: text_window::draw },
    Spec { screen: "licence agreement", draw: text_window::draw },
];

pub fn find(screen: &str) -> Option<&'static Spec> {
    if screen.is_empty() {
        return None;
    }
    SPECS.iter().find(|s| s.screen == screen)
}
