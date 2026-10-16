//! The Admiral's Log (`do_admiralsLog`), after a mission or from the in-mission menu: a
//! tab strip, the players down the left, the page, and Save to text / Done.
//!
//! The stock dialog is a frame of art with owner-drawn buttons (the eight tabs, the
//! players, Save and Done: windows, which the model reads by id and text) and eight panes
//! made with `CreateDialogParamA`, one per tab, of which one shows. Their frames are art,
//! so ours is a window of the theme's; the tabs and players are our widgets, a click on
//! one a click on the stock button (the pane then switches by itself, which is how our
//! tab knows which one shows: the visible pane's place among the eight, the panes being
//! listed in the order the tabs are, backwards).
//!
//! **Score** (the first page) is a table of labels, which the model reads (the column
//! headers and 8 rows of values, in the game's colours): it is drawn as our table, its
//! rows level with the players'. **The other pages** are graphs and lists painted by
//! GDI inside the pane (and a scroll bar on the lists): the model cannot read them, so
//! the pane is cropped from the mirror into the page, and the pointer, the wheel and the
//! scroll bar go to the stock pane (`kit::forward_part`).
use crate::gfx::{fade, Weight, R};
use crate::menus::model::{Class, Control, What};
use crate::menus::screens::kit::{self, Memory};
use crate::menus::Screen;
use crate::widgets::{theme, Align, Stack, Ui};
use alloc::vec::Vec;

/// The pane's place in the dialog, design coordinates.
const PANE: [i32; 4] = [170, 60, 630, 512];

#[derive(Default)]
struct Tab(usize);

static mut TAB: Memory<Tab> = Memory::new(Tab(0));

fn window_ctl<'a>(s: &'a Screen, id: i32) -> Option<&'a Control> {
    s.model.controls.iter().find(|c| c.id == id && matches!(c.what, What::Window(Class::OwnerDrawn)))
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let mut tabs: Vec<&Control> = s.model.controls.iter().filter(|c| c.id < 8 && matches!(c.what, What::Window(Class::OwnerDrawn)) && c.rect[1] < 80).collect();
    tabs.sort_by_key(|c| c.id);
    let mut players: Vec<&Control> = s.model.controls.iter().filter(|c| (2100..2300).contains(&c.id) && c.rect[0] < 100 && matches!(c.what, What::Window(Class::OwnerDrawn))).collect();
    players.sort_by_key(|c| c.id);
    let done = window_ctl(s, 2212);
    let save_text = window_ctl(s, 2211);
    let panes: Vec<&Control> = s.model.controls.iter().filter(|c| matches!(c.what, What::Window(Class::Pane))).collect();
    if tabs.len() < 2 || done.is_none() {
        return false;
    }
    let t = ui.theme;
    kit::backdrop(ui);
    let win = kit::centred(ui, 1760.0, 980.0);
    let inner = ui.window(win, "Admiral's Log");

    // Which tab shows: the visible pane's place, the panes being in the tabs' order
    // backwards; else the last one we clicked.
    let mem = unsafe { &mut TAB }.of(s.slot.hwnd);
    let shown: Vec<usize> = panes.iter().enumerate().filter(|(_, c)| c.visible).map(|(i, _)| i).collect();
    if shown.len() == 1 && panes.len() == tabs.len() {
        mem.0 = panes.len() - 1 - shown[0];
    }
    let mut sel = mem.0.min(tabs.len() - 1);

    let gap = ui.u(t.gap);
    let foot_h = ui.u(t.button_h);
    let mut st = Stack::new(inner, gap);
    let tab_row = st.row(ui.u(46.0));
    let body_h = (inner.h - ui.u(46.0) - foot_h - 2.0 * gap).max(0.0);
    let body = st.row(body_h);
    let foot = st.row(foot_h);

    let names: Vec<&str> = tabs.iter().map(|c| c.text.as_str()).collect();
    if s.top {
        if ui.tabs(tab_row, "tabs", &names, &mut sel) {
            s.click_control(tabs[sel]);
            mem.0 = sel;
        }
    } else {
        let mut keep = sel;
        ui.tabs(tab_row, "tabs", &names, &mut keep);
    }

    // The players down the left, the page to their right.
    let (left, page) = body.take_left(ui.u(300.0).min(body.w * 0.3));
    let page = R::new(page.x + gap, page.y, page.w - gap, page.h);
    let header_h = ui.u(44.0);
    let rows = players.len().max(8);
    let row_h = ((page.h - header_h) / rows as f32).min(ui.u(70.0));
    for (i, c) in players.iter().enumerate() {
        if c.text.trim().is_empty() {
            continue;
        }
        let r = R::new(left.x, page.y + header_h + i as f32 * row_h, left.w, row_h - ui.u(4.0));
        let id = ui.id(&crate::sfmt!("player{}", c.id));
        let (hot, held, clicked) = if s.top { ui.interact(id, r) } else { (false, false, false) };
        let state = theme::State::of(ui, id, hot, held);
        theme::button_frame(ui, r, state);
        let mut lines = c.text.split('\n');
        let name = lines.next().unwrap_or("");
        let team = lines.next().unwrap_or("");
        let pad = ui.u(t.pad);
        ui.text_in(R::new(r.x + pad, r.y, r.w - 2.0 * pad, r.h * 0.55), name, t.text_size, Align::Left, t.text, Weight::Bold);
        ui.text_in(R::new(r.x + pad, r.y + r.h * 0.5, r.w - 2.0 * pad, r.h * 0.45), team, t.small_size, Align::Left, t.text_dim, Weight::Regular);
        if clicked {
            s.click_control(c);
        }
    }

    // The page.
    ui.draw.rect_round(page, ui.u(6.0), fade(t.track, 0.9));
    ui.draw.frame(page, ui.u(t.edge).max(1.0), t.panel_edge);
    if sel == 0 && score_table(ui, s, page, header_h, row_h, &panes) {
        // drawn from the model
    } else {
        let src = PANE;
        let dst = kit::contain(page.inset(ui.u(6.0)), src);
        s.mirror_part(ui, src, dst, 0xffffffff);
        kit::forward_part(ui, s, dst, src);
    }

    // Save to text, Done.
    let bw = ui.u(260.0);
    if let Some(c) = done {
        kit::button(ui, s, R::new(foot.right() - bw, foot.y, bw, foot.h), c, &c.text);
    }
    if let Some(c) = save_text {
        kit::button(ui, s, R::new(foot.right() - 2.0 * bw - gap, foot.y, bw, foot.h), c, &c.text);
    }
    true
}

/// The Score page from the pane's labels: false when it has none (not this page).
fn score_table(ui: &mut Ui, s: &Screen, page: R, header_h: f32, row_h: f32, panes: &[&Control]) -> bool {
    let _ = panes;
    let mut labels: Vec<&Control> = kit::labels(s).into_iter().filter(|c| c.visible && c.rect[0] >= PANE[0]).collect();
    if labels.len() < 6 {
        return false;
    }
    labels.sort_by_key(|c| (c.rect[1], c.rect[0]));
    let top = labels[0].rect[1];
    let cols = labels.iter().filter(|c| c.rect[1] == top).count();
    if cols == 0 || labels.len() % cols != 0 {
        return false;
    }
    let t = ui.theme;
    let col_w = page.w / cols as f32;
    for (n, c) in labels.iter().enumerate() {
        let (col, row) = (n % cols, n / cols);
        let (y, h) = if row == 0 { (page.y, header_h) } else { (page.y + header_h + (row - 1) as f32 * row_h, row_h) };
        let r = R::new(page.x + col as f32 * col_w, y, col_w, h);
        if row > 0 && row % 2 == 0 {
            ui.draw.rect(R::new(page.x, y, page.w, h), t.row_alt);
        }
        let text = c.text.trim();
        if row == 0 {
            ui.text_in(r, text, t.small_size, Align::Centre, t.title, Weight::Bold);
        } else {
            // Numbers in the player's colour from the game.
            let colour = if c.colour == 0xffffffff || c.colour == 0xff000000 { t.text } else { c.colour };
            ui.text_in(r, text, t.text_size + 4.0, Align::Centre, colour, Weight::Regular);
        }
    }
    true
}
