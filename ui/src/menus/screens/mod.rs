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

pub mod admirals_log;
pub mod kit;
pub mod main_menu;
pub mod mission_select;
pub mod saves;
pub mod single_player;
pub mod tech_tree;

pub struct Spec {
    /// The screen it lays out, as `engine::dialogs::screen_name` names it.
    pub screen: &'static str,
    pub draw: fn(&mut Ui, &Screen) -> bool,
}

pub const SPECS: &[Spec] = &[
    Spec { screen: "main menu", draw: main_menu::draw },
    Spec { screen: "single player", draw: single_player::draw },
    Spec { screen: "mission select", draw: mission_select::draw },
    Spec { screen: "load game", draw: saves::draw_load },
    Spec { screen: "save game", draw: saves::draw_save },
    Spec { screen: "admiral's log", draw: admirals_log::draw },
    Spec { screen: "technology tree", draw: tech_tree::draw },
];

pub fn find(screen: &str) -> Option<&'static Spec> {
    if screen.is_empty() {
        return None;
    }
    SPECS.iter().find(|s| s.screen == screen)
}
