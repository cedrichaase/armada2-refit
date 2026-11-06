//! Options (`do_escapeMenu`), from the main menu and in a mission: a window with a
//! column of the stock screen's buttons, in the order they stand on the stock screen
//! (the mission's has more: save, load, abort, the log, the tech tree, exit). The
//! words are the stock buttons' own; the one at the foot (Previous Menu, or Return to
//! Game in a mission) stands in the window's footer. A click is the stock button's
//! click, so its handler, sound and next screen are the stock ones.
use super::common::*;
use crate::gfx::{Weight, R};
use crate::menus::model::Control;
use crate::menus::{refit, Screen};
use crate::widgets::{Align, Anchor, Ui};
use alloc::vec::Vec;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let mut btns: Vec<&Control> = s.model.controls.iter().filter(|c| is_button(c) && c.visible && !is_footer(c) && !c.text.is_empty()).collect();
    if btns.is_empty() {
        return false;
    }
    btns.sort_by_key(|c| (c.rect[1], c.rect[0]));
    backdrop_for(ui, s);
    let t = ui.theme;
    let (bh, gap) = (46.0, 10.0);
    // One entry of ours, "Refit settings", directly under Game Settings (at the foot
    // when this screen has no such button): the Refit screen.
    let entry = refit::available();
    let at = btns.iter().position(|c| c.text.eq_ignore_ascii_case("game settings")).unwrap_or(btns.len() - 1);
    let n = btns.len() as f32 + if entry { 1.0 } else { 0.0 };
    let h = 64.0 + 2.0 * t.pad + n * bh + (n - 1.0) * gap + 24.0 + 44.0 + 20.0;
    let r = Anchor::Centre.place(ui.screen(), ui.u(560.0), ui.u(h.min(1040.0)), 0.0);
    let name = title(s).map_or("", |c| c.text.as_str());
    let inner = ui.window(r, name);
    let (body, foot) = inner.take_top(inner.h - ui.u(44.0 + 12.0));
    let mut hot = None;
    let mut y = body.y + ui.u(12.0);
    for (i, c) in btns.into_iter().enumerate() {
        stock_button(ui, s, c, R::new(body.x + ui.u(40.0), y, body.w - ui.u(80.0), ui.u(bh)), &mut hot);
        y += ui.u(bh + gap);
        if entry && i == at {
            let er = R::new(body.x + ui.u(40.0), y, body.w - ui.u(80.0), ui.u(bh));
            let clicked = ui.scope("refit entry", |ui| ui.button(er, "Refit settings"));
            if clicked && s.top && !refit::is_open() {
                crate::engine::api::sound::shell_click();
                refit::open(s.slot.hwnd);
            }
            if ui.input.mouse.is_some_and(|p| er.contains(p)) {
                hot = None;
            }
            y += ui.u(bh + gap);
        }
    }
    footer(ui, s, foot, &mut hot);
    if let Some(n) = footnote(s) {
        let nr = R::new(foot.right() - ui.u(120.0), foot.y, ui.u(120.0), foot.h);
        ui.text_in(nr, &n.text, t.small_size, Align::Right, t.text_dim, Weight::Regular);
    }
    hover_stock(s, if refit::is_open() { None } else { hot });
    true
}
