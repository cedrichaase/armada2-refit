//! The top-right buttons, ours (`[HUD] Buttons=1`; HUD.md, "The top-right buttons"):
//! a row of square buttons in the top-right corner, each showing the stock button's
//! own picture and pressing the stock button: menu, comm, idle unit, view, objectives.
//! Which are disabled (comm in single player, objectives where the mission has none)
//! is the stock panel's. Stock's help (the tooltip level) is left out, the palette
//! toggle is Game Settings' "Command Bar" (`prefs`, `menus::screens::game_settings`),
//! and so "show more", which only uncovered those two and the menu, is gone too: the
//! menu button always shows.
use crate::engine::api::topbar::{self, Which};
use crate::widgets::{Anchor, Ui};
use alloc::vec::Vec;

/// The top-right buttons, a panel of the table (`panels::PANELS`).
pub struct Buttons;

impl super::panels::Panel for Buttons {
    fn name(&self) -> &'static str {
        "buttons"
    }
    fn scope(&self) -> &'static str {
        "hud.buttons"
    }
    fn key(&self, c: &crate::config::Hud) -> bool {
        c.buttons
    }
    fn component(&self) -> (usize, &'static str) {
        (topbar::button_panel(), "ButtonPanel")
    }
    fn with_hud(&self) -> bool {
        true
    }
    fn draw(&self, ui: &mut Ui) {
        run(ui)
    }
}

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
    if crate::engine::api::hud::hidden(topbar::button_panel()) {
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
                s.push_str(&crate::sfmt!(" {}{}", w.name(), if crate::engine::api::buttons::enabled(b) { "" } else { "(off)" }));
            }
            crate::log!("buttons:{}", s);
            crate::log!("buttons: panel{}", topbar::measure());
            LOGGED = true;
        }
    }
    let (cs, gap) = (ui.u(44.0), ui.u(5.0));
    let w = list.len() as f32 * cs + (list.len() - 1) as f32 * gap;
    let (mut outer, mut inner) = super::panels::fit(ui, Anchor::TopRight, w, cs, ui.u(6.0));
    // The map editor's own menu ("Edit Menu" and its F-key list) is drawn by the game in
    // the top-right corner on Roots, where these stand: in the editor they stand beside
    // the resource strip at the top left instead (PLAN-FLEETOPS.md, decision 504).
    if let Some(strip) = editor_place(ui, outer) {
        let (dx, dy) = (strip.0 - outer.x, strip.1 - outer.y);
        outer = crate::gfx::R::new(outer.x + dx, outer.y + dy, outer.w, outer.h);
        inner = crate::gfx::R::new(inner.x + dx, inner.y + dy, inner.w, inner.h);
    }
    ui.panel(outer);
    unsafe { BOTTOM = Some((outer.bottom(), crate::rt::now())) };
    for (i, &(which, b)) in list.iter().enumerate() {
        let r = crate::gfx::R::new(inner.x + i as f32 * (cs + gap), inner.y, cs, cs);
        if super::panels::game_button(ui, r, b, which.name(), false) {
            unsafe { LOGGED = false };
        }
    }
}

/// Where the panel `outer` stands in the map editor: just right of the resource strip,
/// at its top, when the strip stands at the top left (Roots); None in a game, or where
/// the strip is at the top centre (Armada II, whose editor menu is at the top left).
fn editor_place(ui: &Ui, outer: crate::gfx::R) -> Option<(f32, f32)> {
    static mut SAID: bool = false;
    if !crate::engine::api::radar::editor() {
        return None;
    }
    let strip = super::resources::left_strip()?;
    let x = strip.right() + ui.u(8.0);
    if x + outer.w > ui.w * 0.6 {
        return None;
    }
    unsafe {
        if !SAID {
            SAID = true;
            crate::log!("buttons: in the map editor, beside the resources at {:.0},{:.0} (the editor's menu keeps the top right)", x, strip.y);
        }
    }
    Some((x, strip.y))
}
