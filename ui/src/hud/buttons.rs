//! The top-right buttons, ours (`[HUD] Buttons=1`; HUD.md, "The top-right buttons"):
//! a row of square buttons in the top-right corner, each showing the stock button's
//! own picture and pressing the stock button: menu, comm, idle unit, view, objectives.
//! Which are disabled (comm in single player, objectives where the mission has none)
//! is the stock panel's. Stock's help (the tooltip level) is left out, the palette
//! toggle is Game Settings' "Command Bar" (`prefs`, `menus::screens::game_settings`),
//! and so "show more", which only uncovered those two and the menu, is gone too: the
//! menu button always shows.
use crate::engine::topbar::{self, Which};
use crate::widgets::{Anchor, Ui};
use alloc::vec::Vec;

/// Left to right.
const ORDER: [Which; 5] = [Which::Menu, Which::Comm, Which::Idle, Which::View, Which::Objectives];

static mut LOGGED: bool = false;
/// The panel's foot in the last frame it drew (for the trade status under it).
static mut BOTTOM: Option<(f32, f64)> = None;

/// The foot of the buttons' panel, if it drew in the last half second.
pub fn bottom() -> Option<f32> {
    let now = crate::rt::now();
    unsafe { BOTTOM }.filter(|&(_, at)| now - at < 0.5).map(|(y, _)| y)
}

pub fn run(ui: &mut Ui) {
    if crate::engine::hud::hidden(topbar::BUTTON_PANEL) {
        return;
    }
    let list: Vec<(Which, usize)> = ORDER
        .iter()
        .map(|&w| (w, topbar::button(w)))
        .filter(|&(_, b)| b != 0)
        .collect();
    if list.is_empty() {
        return;
    }
    unsafe {
        if !LOGGED {
            let mut s = alloc::string::String::new();
            for &(w, b) in &list {
                s.push_str(&crate::sfmt!(" {}{}", w.name(), if crate::engine::buttons::enabled(b) { "" } else { "(off)" }));
            }
            crate::log!("buttons:{}", s);
            LOGGED = true;
        }
    }
    let (cs, gap) = (ui.u(44.0), ui.u(5.0));
    let w = list.len() as f32 * cs + (list.len() - 1) as f32 * gap;
    let (outer, inner) = super::panels::fit(ui, Anchor::TopRight, w, cs, ui.u(6.0));
    ui.panel(outer);
    unsafe { BOTTOM = Some((outer.bottom(), crate::rt::now())) };
    for (i, &(which, b)) in list.iter().enumerate() {
        let r = crate::gfx::R::new(inner.x + i as f32 * (cs + gap), inner.y, cs, cs);
        if super::panels::game_button(ui, r, b, which.name(), false) {
            unsafe { LOGGED = false };
        }
    }
}
