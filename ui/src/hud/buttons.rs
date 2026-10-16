//! The top-right buttons, ours (`[HUD] Buttons=1`; HUD.md, "The top-right buttons"):
//! a row of square buttons in the top-right corner, each showing the stock button's
//! own picture and pressing the stock button (menu, comm, idle unit, view, objectives,
//! and, while "show more" is set, help and palette; "more" last). Which show, and
//! which are disabled (comm in single player, objectives where the mission has none),
//! is the stock panel's.
use crate::engine::topbar::{self, Which};
use crate::widgets::{Anchor, Ui};
use alloc::vec::Vec;

/// Left to right; the extra ones only with "show more".
const ORDER: [Which; 8] = [Which::Menu, Which::Help, Which::Palette, Which::Comm, Which::Idle, Which::View, Which::Objectives, Which::More];

static mut LOGGED: bool = false;

pub fn run(ui: &mut Ui) {
    if crate::engine::hud::hidden(topbar::BUTTON_PANEL) {
        return;
    }
    let more = topbar::show_more();
    let list: Vec<(Which, usize)> = ORDER
        .iter()
        .filter(|w| more || !w.extra())
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
    for (i, &(which, b)) in list.iter().enumerate() {
        let r = crate::gfx::R::new(inner.x + i as f32 * (cs + gap), inner.y, cs, cs);
        let on = match which {
            Which::More => more,
            Which::Help => topbar::tooltip_state() != 0,
            _ => false,
        };
        if super::panels::game_button(ui, r, b, which.name(), on) {
            unsafe { LOGGED = false };
        }
    }
}
