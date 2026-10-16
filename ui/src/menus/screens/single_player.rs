//! The campaign selection (`do_single`), laid out our way: four cards, each a cut of the
//! stock screen's art (the faction's picture, which is information a theme cannot
//! draw) under its name, and the two buttons at the foot.
//!
//! In the stock screen the Tutorial and the Federation panel are hit areas of the
//! procedure itself, with Bink hover animations; the Klingon and Borg panels are
//! picture `ShellButton`s. None has a label: the words appear in the middle of the screen
//! on hover, from four hidden `TextLabel`s, which are where the names come from. A click
//! is a click on a point inside the stock panel (the same points `testbench/ui.json`
//! names), and a hover is a `WM_MOUSEMOVE` to the point, so the stock screen plays the
//! panel's animation and sound, which the cut of the art then shows.
use crate::gfx::{fade, R};
use crate::menus::screens::kit;
use crate::menus::{actions, Screen};
use crate::rt::win::{HWND, WM_MOUSEMOVE};
use crate::widgets::{theme, Align, Ui};

/// A card: the stock panel's art (design rectangle), a point inside the stock panel to
/// click, the default name.
struct Card {
    art: [i32; 4],
    at: (i32, i32),
    name: &'static str,
}

const CARDS: [Card; 4] = [
    Card { art: [24, 16, 330, 205], at: (186, 111), name: "Tutorials" },
    Card { art: [436, 22, 310, 216], at: (589, 106), name: "Federation Campaign" },
    Card { art: [24, 338, 330, 222], at: (186, 444), name: "Klingon Campaign" },
    Card { art: [431, 340, 322, 222], at: (589, 439), name: "Borg Campaign" },
];

/// The card under the pointer last frame, to tell the stock screen when it changes.
static mut HOVER: (HWND, i32) = (core::ptr::null_mut(), -1);

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let names = kit::labels(s);
    if names.len() < 4 {
        return false;
    }
    let buttons = kit::buttons(s);
    // Previous menu and Load Game: the two with text, at the foot.
    let mut foot: alloc::vec::Vec<_> = buttons.iter().copied().filter(|c| !c.text.is_empty() && c.rect[1] > 500).collect();
    foot.sort_by_key(|c| c.rect[0]);
    if foot.len() < 2 {
        return false;
    }
    let t = ui.theme;
    let sc = ui.screen();
    kit::backdrop(ui);

    // The screen: a title, the 2x2 cards, the foot.
    let m = ui.u(40.0);
    let title_h = ui.u(74.0);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(24.0);
    ui.text_in(R::new(m, ui.u(18.0), sc.w - 2.0 * m, title_h), "SINGLE PLAYER", t.display_size, Align::Left, t.title, crate::gfx::Weight::Bold);
    let grid = R::new(m, ui.u(18.0) + title_h, sc.w - 2.0 * m, sc.h - ui.u(18.0) - title_h - foot_h - 2.0 * gap - ui.u(10.0));
    let card_h = (grid.h - gap) * 0.5;
    let card_w = ((grid.w - gap) * 0.5).min(card_h * 1.5 + ui.u(24.0));
    let x0 = grid.x + (grid.w - 2.0 * card_w - gap) * 0.5;
    let mut hover = -1i32;
    for (i, card) in CARDS.iter().enumerate() {
        let r = R::new(x0 + (i % 2) as f32 * (card_w + gap), grid.y + (i / 2) as f32 * (card_h + gap), card_w, card_h);
        let id = ui.id(card.name);
        let live = s.top && s.model.button_at(card.at.0, card.at.1).is_none_or(|c| c.enabled);
        let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
        let st = theme::State::of(ui, id, hot, held);
        theme::panel(ui, r);
        let (bar, rest) = r.take_top(ui.u(46.0));
        let name = names.get(i).map_or(card.name, |c| if c.text.is_empty() { card.name } else { c.text.as_str() });
        ui.text_in(bar.inset(ui.u(8.0)), name, t.heading_size, Align::Left, if hot { t.accent } else { t.title }, crate::gfx::Weight::Bold);
        let art = kit::contain(rest.inset(ui.u(8.0)), card.art);
        s.mirror_part(ui, card.art, art, 0xffffffff);
        ui.draw.frame(art, 1.0, fade(t.panel_edge, 0.8));
        if st.hot > 0.01 {
            ui.draw.frame(r, ui.u(2.0).max(1.0), fade(t.accent, st.hot));
        }
        if hot {
            hover = i as i32;
        }
        if clicked {
            s.click(card.at.0, card.at.1);
        }
    }
    // The stock screen hears of the hover, so its own animation and sound play.
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 { CARDS[hover as usize].at } else { (2, 2) };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
        }
    }
    // The foot: Previous Menu at the left, Load Game at the right.
    let fy = sc.h - foot_h - gap;
    let (bw, _) = (ui.u(300.0), 0);
    kit::button(ui, s, R::new(m, fy, bw, foot_h), foot[0], &foot[0].text);
    let last = foot.len() - 1;
    kit::button(ui, s, R::new(sc.w - m - bw, fy, bw, foot_h), foot[last], &foot[last].text);
    true
}
