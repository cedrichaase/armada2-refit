//! The Admiral's Log (`do_admiralsLog`), after a mission or from the in-mission menu: a
//! tab strip, the players down the left, the page, and Done. Stock's Save to text
//! (2211, which writes the log to a text file) is left out: not offered.
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
//! rows level with the players'. **The other seven pages** (Military, Economy,
//! Timeline, Battles, Ships, Build, Tally) are the pane's frame art with
//! `StdTextLabel`s over it, which the model also reads (each with its design
//! rectangle, colour, alignment and every line): `typeset` sets them again in the
//! theme's type, in their places in the pane mapped into the page, the header row in
//! the title colour and the rows striped, with no frame art. What is a picture and
//! not words is cropped from the mirror: Timeline's graph of the armadas' power, and
//! the Ships page's portraits beside their rows. The stock lists scroll by their own
//! two buttons (`Shell(Button)`s at the pane's right edge), which our arrows and the
//! wheel click. A list with no rows (no battle yet) draws an empty page.
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
    } else if sel > 0 && typeset(ui, s, page.inset(ui.u(10.0)), sel) {
        // drawn from the pane's labels
    } else {
        let src = PANE;
        let dst = kit::contain(page.inset(ui.u(6.0)), src);
        s.mirror_part(ui, src, dst, 0xffffffff);
        kit::forward_part(ui, s, dst, src);
    }

    // Done.
    let bw = ui.u(260.0);
    if let Some(c) = done {
        kit::button(ui, s, R::new(foot.right() - bw, foot.y, bw, foot.h), c, &c.text);
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

/// The pane's visible part, design coordinates: below this the lists are clipped (the
/// Ships page keeps labels of rows scrolled out of view, far below).
const PANE_BOTTOM: i32 = 566;

/// Timeline's graph, inside the pane (design coordinates): a picture, cropped.
const TIMELINE_GRAPH: [i32; 4] = [177, 125, 612, 442];
/// Its legend's two icons (a battle, a planet colonised), left of their words.
const TIMELINE_ICONS: [[i32; 4]; 2] = [[500, 85, 27, 26], [602, 85, 27, 26]];
/// The Ships page's portrait column (design x and width), left of each row's label.
const SHIP_PORTRAIT: (i32, i32) = (183, 216);

/// A page of the pane set from its labels (the header row, the rows, each label in
/// its place); the Timeline's graph and the Ships page's portraits cropped from the
/// mirror; the stock scroll buttons as our arrows. An empty list draws empty.
fn typeset(ui: &mut Ui, s: &Screen, page: R, sel: usize) -> bool {
    let mut labels: Vec<&Control> = kit::labels(s)
        .into_iter()
        .filter(|c| c.visible && !c.text.trim().is_empty() && c.rect[0] >= PANE[0] && c.rect[1] >= PANE[1] && c.rect[1] < PANE_BOTTOM)
        .collect();
    labels.sort_by_key(|c| (c.rect[1], c.rect[0]));
    let t = ui.theme;
    let dst = kit::contain(page, PANE);
    let k = dst.w / PANE[2] as f32;
    let map = |r: [i32; 4]| R::new(dst.x + (r[0] - PANE[0]) as f32 * k, dst.y + (r[1] - PANE[1]) as f32 * k, r[2] as f32 * k, r[3] as f32 * k);
    // Type in units from the design size: 12 design pixels a line, as stock's.
    let unit = ui.u(1.0).max(0.01);
    let body = (11.5 * k / unit).clamp(9.0, 22.0);
    let head = (13.0 * k / unit).clamp(10.0, 24.0);

    // Rows: labels whose tops are within a few pixels. A row of several one-line
    // labels without a digit is a header (Tally's stands half-way down, under its
    // summary; Build has none).
    let mut rows: Vec<(i32, i32, Vec<&Control>)> = Vec::new();
    for c in labels.iter() {
        match rows.iter_mut().find(|r| (r.0 - c.rect[1]).abs() <= 4) {
            Some(r) => {
                r.1 = r.1.max(c.rect[1] + c.rect[3]);
                r.2.push(c)
            }
            None => rows.push((c.rect[1], c.rect[1] + c.rect[3], alloc::vec![*c])),
        }
    }
    let is_header = |cells: &Vec<&Control>| cells.len() > 1 && cells.iter().all(|c| !c.text.trim().contains('\n') && !c.text.bytes().any(|b| b.is_ascii_digit()));
    // The Ships page: one label per row, beside its portrait; the rows are as tall as
    // the gap to the next.
    let ships = sel == 5 && labels.iter().all(|c| c.rect[0] > SHIP_PORTRAIT.0 + SHIP_PORTRAIT.1);

    if sel == 3 {
        for src in [TIMELINE_GRAPH, TIMELINE_ICONS[0], TIMELINE_ICONS[1]] {
            s.mirror_part(ui, src, map(src), 0xffffffff);
        }
        ui.draw.frame(map(TIMELINE_GRAPH), ui.u(1.0).max(1.0), t.panel_edge);
    }
    let mut stripe = 0usize;
    for (i, (top, bottom, cells)) in rows.iter().enumerate() {
        let is_head = is_header(cells);
        let next = rows.get(i + 1).map_or(PANE_BOTTOM, |r| r.0);
        let band_bottom = if ships { next.min(PANE_BOTTOM) } else { (*bottom).max(*top + 18).min(next) };
        let band = map([PANE[0], *top - 2, PANE[2], band_bottom - *top + 2]);
        if is_head && sel != 3 {
            ui.draw.rect(R::new(band.x, band.bottom() - 1.0, band.w, ui.u(1.0).max(1.0)), fade(t.accent, 0.7));
        } else if !is_head {
            if stripe % 2 == 1 && sel != 3 {
                ui.draw.rect(band, t.row_alt);
            }
            stripe += 1;
        }
        if ships {
            let h = (next.min(PANE_BOTTOM) - *top - 4).clamp(0, 132);
            if h > 8 {
                let src = [SHIP_PORTRAIT.0, *top, SHIP_PORTRAIT.1, h];
                let r = map(src).inset(ui.u(3.0));
                s.mirror_part(ui, src, r, 0xffffffff);
            }
        }
        for c in cells.iter() {
            let r = map(c.rect);
            let colour = if is_head {
                t.title
            } else if c.colour == 0xffffffff || c.colour == 0xff000000 || c.colour == 0 {
                t.text
            } else {
                c.colour | 0xff000000
            };
            let align = match c.align {
                -1 => Align::Left,
                1 => Align::Right,
                _ => Align::Centre,
            };
            let (mut size, weight) = if is_head { (head, Weight::Bold) } else { (body, if ships { Weight::Bold } else { Weight::Regular }) };
            // The lines fit the room the row has (Economy's cells are four lines in a
            // row of 55 design pixels), the type made smaller when they would not.
            let lines = c.text.trim_end().split('\n').count().max(1) as f32;
            let room = (next - c.rect[1] - 3).max(10) as f32 * k;
            let mut lh = ui.u(size * 1.25);
            if lh * lines > room {
                lh = room / lines;
                size = (lh / ui.u(1.2)).max(7.0);
            }
            let mut y = r.y + ui.u(2.0);
            for (n, line) in c.text.trim_end().split('\n').enumerate() {
                let line = line.trim_end_matches('\r');
                // The Ships page's second line (the stardate and what became of the
                // ship) in the quieter colour.
                let col = if ships && n > 0 { fade(t.text_dim, 1.0) } else { colour };
                let text = if is_head { theme::caps(line.trim()) } else { alloc::string::String::from(line.trim()) };
                ui.text_in(R::new(r.x + ui.u(4.0), y, r.w.max(ui.u(40.0)) - ui.u(8.0), lh), &text, size, align, col, weight);
                y += lh;
            }
        }
    }

    // The stock list's two scroll buttons, top and bottom of the pane's right edge.
    let mut arrows: Vec<&Control> = s
        .model
        .controls
        .iter()
        .filter(|c| c.visible && c.text.is_empty() && matches!(c.what, What::Shell(crate::engine::shellui::Kind::Button)) && c.rect[0] >= 700 && c.rect[1] >= PANE[1])
        .collect();
    arrows.sort_by_key(|c| c.rect[1]);
    if arrows.len() >= 2 {
        let (up, down) = (arrows[0], arrows[arrows.len() - 1]);
        let a = ui.u(30.0);
        let ur = R::new(page.right() - a, page.y, a, a);
        let dr = R::new(page.right() - a, page.bottom() - a, a, a);
        kit::arrow(ui, s, ur, up, true);
        kit::arrow(ui, s, dr, down, false);
        let over = ui.input.mouse.is_some_and(|p| page.contains(p));
        let wheel = ui.input.wheel;
        if s.top && over && wheel != 0.0 {
            s.click_control(if wheel > 0.0 { up } else { down });
        }
    }
    true
}
