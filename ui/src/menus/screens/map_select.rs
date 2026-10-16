//! Map Select (`do_mapSelect`), opened by Game Setup's SELECT MAP: the maps as our list.
//!
//! The stock screen is a window of fourteen `FileSelectButton` rows, whose labels are
//! the maps at the list's scroll position, two arrow buttons and a scroll bar beside
//! them, a filter (`ComboBox`: All Maps, 2 Players, ...) and Select Map and Cancel. The
//! model reads the rows' labels, so our list shows the same fourteen maps; the wheel
//! and our arrows are clicks on the stock arrows (the list scrolls the stock way), a
//! click on a row is the stock click on that row (which selects it), a double click is
//! that click and Select Map, as in stock. The selected row is the one the stock screen
//! draws held (its button state).
//!
//! The minimap preview is Game Setup's, under this dialog: the map chosen here shows
//! there once Select Map closes it.
use super::kit;
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Control, What};
use crate::menus::Screen;
use crate::widgets::{theme, Align, Anchor, State, Ui};
use alloc::vec::Vec;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    // The rows: wide labelled buttons, one under another.
    let mut rows: Vec<&Control> = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Button)) && c.rect[2] > 200 && c.rect[3] < 20);
    rows.sort_by_key(|c| c.rect[1]);
    let arrows: Vec<&Control> = {
        let mut v = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Button)) && c.text.is_empty() && c.rect[2] < 30);
        v.sort_by_key(|c| c.rect[1]);
        v
    };
    let filter = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Combo))).first().copied();
    let mut nav: Vec<&Control> = kit::visible(s, |c| kit::is_button(c) && !c.text.is_empty() && c.rect[3] >= 20);
    nav.sort_by_key(|c| c.rect[0]);
    if nav.len() < 2 {
        return false;
    }
    let t = ui.theme;
    let screen = ui.screen();
    ui.draw.rect(screen, t.scrim);
    let w = ui.u(900.0).min(screen.w - ui.u(32.0));
    let h = ui.u(820.0).min(screen.h - ui.u(32.0));
    let r = Anchor::Centre.place(screen, w, h, 0.0);
    let inner = ui.window(r, "Select Map");
    let bh = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let mut y = inner.y;
    if let Some(f) = filter {
        let fr = R::new(inner.right() - ui.u(300.0), y, ui.u(300.0), bh);
        ui.text_in(R::new(inner.x, y, inner.w - fr.w, bh), "Show", t.text_size, Align::Left, t.text_dim, Weight::Regular);
        kit::combo(ui, s, f, fr);
        y += bh + gap;
    }
    let list = R::new(inner.x, y, inner.w - ui.u(48.0), inner.bottom() - bh - ui.u(t.pad) - y);
    let row_h = (list.h / rows.len().max(1) as f32).min(ui.u(t.text_size + 16.0));
    theme::field(ui, list, State::default());
    let li = list.inset(ui.u(3.0));
    let mut double = false;
    for (i, c) in rows.iter().enumerate() {
        let rr = R::new(li.x, li.y + i as f32 * row_h, li.w, row_h);
        if rr.bottom() > li.bottom() + 0.5 {
            break;
        }
        let id = ui.id(&crate::sfmt!("row{}", i));
        let live = s.top && c.enabled && !c.text.is_empty();
        let (hot, held, clicked) = if live { ui.interact(id, rr) } else { (false, false, false) };
        let mut st = State::of(ui, id, hot, held);
        // The stock row stays drawn held while it is the selection.
        st.on = c.hover == 2 || c.checked;
        theme::row(ui, rr, i, st);
        let col = theme::row_text(ui, st);
        ui.text_in(rr.inset(ui.u(8.0)), &c.text, t.text_size, Align::Left, col, Weight::Regular);
        if clicked {
            s.click_control(c);
            double = ui.input.double[0];
        }
    }
    // The arrows, as our buttons beside the list; the wheel clicks them too.
    let ax = list.right() + gap;
    let aw = inner.right() - ax;
    if let (Some(up), Some(down)) = (arrows.first(), arrows.last()) {
        if arrows.len() >= 2 {
            kit::button(ui, s, up, R::new(ax, list.y, aw, aw), Some("^"));
            kit::button(ui, s, down, R::new(ax, list.bottom() - aw, aw, aw), Some("v"));
            let over = ui.input.mouse.is_some_and(|p| list.contains(p));
            if s.top && over && ui.input.wheel != 0.0 {
                let c = if ui.input.wheel > 0.0 { up } else { down };
                for _ in 0..((libm::fabsf(ui.input.wheel) * 3.0) as i32).max(1) {
                    s.click_control(c);
                }
            }
            ui.claim(list);
        }
    }
    let by = inner.bottom() - bh;
    let bw = ui.u(220.0);
    for (i, c) in nav.iter().enumerate() {
        let x = if i == 0 { inner.x } else { inner.right() - (nav.len() - i) as f32 * (bw + gap) + gap };
        kit::button(ui, s, c, R::new(x, by, bw, bh), None);
    }
    // A double click on a row is that row and Select Map (the first button).
    if double {
        s.click_control(nav[0]);
    }
    true
}
