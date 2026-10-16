//! The Internet Game screen (`do_internetGame`, `InternetGameDlgProc`): the lobby every
//! network connection type comes to (*Internet - Manual IP*, *Local Area Network*,
//! *Internet - Online*) before Game Setup (MENUS.md, "The Internet lobby").
//!
//! The stock screen is shell widgets over a few windows: the title, the "current games"
//! rows (labelled `ShellButton`s in a `RadioBox`, one per game the search has found:
//! `name (players/max) ping "map"`) with Create Game and Join Game under them, the
//! players in the room (`PlayerListButton`s, right), the game-type filter (a shell
//! `ComboBox`), the chat (a read-only multi-line edit, the log, which also carries the
//! "Local IP Address" line and `Online.asi`'s join-code messages, and a single-line
//! edit, the line typed), and the way out (Previous Menu, Main Menu, Help). The
//! scroll bars are the shell's `ScrollBar` class, which the model does not read; their
//! arrow buttons are picture buttons and are kept as our arrows.
//!
//! Found by kind and place, not by label, so it works in every language and with the
//! controls the connection type hides; false (the generic screen draws) when there is
//! no chat line or no button along the bottom. Every action is the stock click.
use super::setup_kit as kit;
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Control, What};
use crate::menus::Screen;
use crate::widgets::{theme, Align, Ui};
use alloc::vec::Vec;

/// The stock layout, design coordinates: the players' column starts at x 560; the game
/// rows lie between the "current games" caption and Create Game (y 258); the chat from
/// y 290; the way out along the bottom (y 562).
const RIGHT: i32 = 560;
const LIST_TOP: i32 = 66;
const ACTIONS: i32 = 250;
const BOTTOM: i32 = 540;

fn labelled(c: &Control) -> bool {
    kit::is_button(c) && !c.text.trim().is_empty() && !c.text.eq_ignore_ascii_case("NULL")
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let edits = kit::visible(s, kit::is_edit);
    let log = edits.iter().find(|c| c.style & 0x4 != 0).copied();
    let line = edits.iter().find(|c| c.style & 0x4 == 0).copied();
    let mut nav: Vec<&Control> = kit::visible(s, |c| labelled(c) && c.rect[1] >= BOTTOM);
    if line.is_none() || nav.is_empty() {
        return false;
    }
    let t = ui.theme;
    kit::backdrop(ui);
    let screen = kit::wide_box(ui);
    let m = ui.u(32.0);
    let gap = ui.u(t.gap * 2.0);

    let labels = kit::visible(s, |c| kit::is_label(c) && !c.text.trim().is_empty());
    let title = labels.iter().filter(|c| c.rect[1] < 40).min_by_key(|c| c.rect[1]).map_or("INTERNET", |c| c.text.trim());
    let cap = |x0: i32, x1: i32, y0: i32, y1: i32, d: &'static str| -> &str {
        labels.iter().find(|c| c.rect[0] >= x0 && c.rect[0] < x1 && c.rect[1] >= y0 && c.rect[1] < y1).map_or(d, |c| c.text.trim())
    };
    let games_cap = cap(0, RIGHT, 40, LIST_TOP, "Current Games");
    let players_cap = cap(RIGHT, 800, 40, LIST_TOP, "Players");
    let chat_cap = cap(0, 800, ACTIONS + 20, 320, "Chat");

    let title_r = R::new(screen.x + m, screen.y + m * 0.5, screen.w - 2.0 * m, ui.u(t.display_size + 16.0));
    ui.text_in(title_r, &theme::caps(title), t.display_size, Align::Left, t.title, Weight::Bold);
    let bh = ui.u(t.button_h);
    let body = R::new(screen.x + m, title_r.bottom() + ui.u(8.0), screen.w - 2.0 * m, screen.bottom() - title_r.bottom() - ui.u(8.0) - bh - m - gap);
    let top_h = libm::floorf((body.h - gap) * 0.55);
    let left_w = libm::floorf((body.w - gap) * 0.64);
    let games_r = R::new(body.x, body.y, left_w, top_h);
    let players_r = R::new(games_r.right() + gap, body.y, body.w - left_w - gap, top_h);
    let chat_r = R::new(body.x, games_r.bottom() + gap, body.w, body.bottom() - games_r.bottom() - gap);

    // ---- the games, the filter, Create and Join
    let mut rows: Vec<&Control> = kit::visible(s, |c| labelled(c) && c.rect[0] < RIGHT && c.rect[1] >= LIST_TOP && c.rect[1] < ACTIONS && !matches!(c.what, What::Shell(Kind::Combo)));
    rows.sort_by_key(|c| c.rect[1]);
    let mut actions: Vec<&Control> = kit::visible(s, |c| labelled(c) && c.rect[0] < RIGHT && c.rect[1] >= ACTIONS && c.rect[1] < BOTTOM - 260);
    kit::dedupe(&mut actions);
    actions.sort_by_key(|c| c.rect[0]);
    let combos = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Combo)));
    let gi = ui.window(games_r, games_cap);
    let row_h = ui.u(t.row_h + 6.0);
    let mut lr = R::new(gi.x, gi.y, gi.w, gi.h - bh - ui.u(t.gap));
    if let Some(cb) = combos.first() {
        let fr = R::new(gi.right() - ui.u(260.0), gi.y, ui.u(260.0), ui.u(t.row_h + 4.0));
        kit::combo(ui, s, cb, fr);
        lr = R::new(lr.x, fr.bottom() + ui.u(t.gap), lr.w, lr.h - fr.h - ui.u(t.gap));
    }
    rows_list(ui, s, &rows, lr, row_h, "game");
    let arrows = arrows_in(s, RIGHT, 700, ACTIONS + 10);
    arrow_buttons(ui, s, &arrows, lr);
    let aw = ui.u(220.0);
    let ay = gi.bottom() - bh;
    let mut ax = gi.right() - aw;
    for c in actions.iter().rev() {
        kit::button(ui, s, c, R::new(ax, ay, aw, bh), None);
        ax -= aw + ui.u(t.gap);
    }

    // ---- the players in the room
    let mut players: Vec<&Control> = kit::visible(s, |c| labelled(c) && c.rect[0] >= RIGHT && c.rect[1] >= LIST_TOP && c.rect[1] < BOTTOM - 260);
    players.sort_by_key(|c| c.rect[1]);
    let pi = ui.window(players_r, players_cap);
    rows_list(ui, s, &players, pi, row_h, "player");
    arrow_buttons(ui, s, &arrows_in(s, 700, 800, BOTTOM), pi);

    // ---- the chat
    let ci = ui.window(chat_r, chat_cap);
    let fh = ui.u(t.row_h + 4.0);
    if let Some(lc) = log {
        chat_log(ui, &lc.text, R::new(ci.x, ci.y, ci.w, ci.h - fh - ui.u(t.gap)));
    }
    if let Some(lc) = line {
        kit::edit(ui, s, lc, R::new(ci.x, ci.bottom() - fh, ci.w, fh));
    }

    // ---- the way out: the first at the left, the rest at the right
    nav.sort_by_key(|c| c.rect[0]);
    kit::dedupe(&mut nav);
    let bw = ui.u(260.0);
    let y = screen.bottom() - m - bh;
    for (i, c) in nav.iter().enumerate() {
        let x = if i == 0 { screen.x + m } else { screen.right() - m - (nav.len() - i) as f32 * (bw + gap) + gap };
        kit::button(ui, s, c, R::new(x, y, bw, bh), None);
    }
    true
}

/// Stock rows (games or players) as our rows in `r`: a click is the stock row's click
/// (which selects it); the row the stock screen draws held is shown selected.
fn rows_list(ui: &mut Ui, s: &Screen, rows: &[&Control], r: R, row_h: f32, what: &str) {
    let t = ui.theme;
    ui.draw.push_clip(r);
    let mut y = r.y;
    for (i, c) in rows.iter().enumerate() {
        if y + row_h > r.bottom() + 0.5 {
            break;
        }
        let rr = R::new(r.x, y, r.w - ui.u(28.0), row_h - ui.u(2.0));
        y += row_h;
        let id = ui.id(&crate::sfmt!("{}{:x}", what, c.key));
        let live = s.top && c.enabled;
        let (hot, held, clicked) = if live { ui.interact(id, rr) } else { (false, false, false) };
        let mut st = theme::State::of(ui, id, hot, held);
        st.on = c.hover == 2 || c.checked;
        st.off = !c.enabled;
        theme::row(ui, rr, i, st);
        let col = theme::row_text(ui, st);
        ui.text_in(rr.inset(ui.u(t.pad * 0.5)), c.text.trim(), t.text_size, Align::Left, col, Weight::Regular);
        if clicked {
            crate::log!("lobby: {} \"{}\"", what, c.text.trim());
            kit::press(s, c);
        }
    }
    ui.draw.pop_clip();
}

/// The picture buttons (no label) of a column, top to bottom: the stock scroll bars'
/// arrows.
fn arrows_in<'a>(s: &'a Screen, x0: i32, x1: i32, y1: i32) -> Vec<&'a Control> {
    let mut v = kit::visible(s, |c| {
        matches!(c.what, What::Shell(Kind::Button)) && (c.text.trim().is_empty() || c.text.eq_ignore_ascii_case("NULL"))
    });
    v.retain(|c| c.rect[0] >= x0 && c.rect[0] < x1 && c.rect[1] >= LIST_TOP && c.rect[1] < y1 && c.rect[2] <= 32 && c.rect[3] <= 32);
    v.sort_by_key(|c| c.rect[1]);
    v
}

/// The first and last of `arrows` (up and down) as two small buttons at the right edge
/// of `r`.
fn arrow_buttons(ui: &mut Ui, s: &Screen, arrows: &[&Control], r: R) {
    if arrows.len() < 2 {
        return;
    }
    let w = ui.u(24.0);
    let up = arrows[0];
    let down = arrows[arrows.len() - 1];
    kit::button(ui, s, up, R::new(r.right() - w, r.y, w, w), Some("^"));
    kit::button(ui, s, down, R::new(r.right() - w, r.bottom() - w, w, w), Some("v"));
}

/// The last lines of the chat log that fit in `r`, newest at the bottom.
fn chat_log(ui: &mut Ui, text: &str, r: R) {
    let t = ui.theme;
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).filter(|l| !l.is_empty()).collect();
    let lh = ui.u(t.small_size * 1.35);
    let n = ((r.h / lh) as usize).max(1);
    let first = lines.len().saturating_sub(n);
    ui.draw.push_clip(r);
    for (i, l) in lines[first..].iter().enumerate() {
        ui.text_in(R::new(r.x, r.y + i as f32 * lh, r.w, lh), l, t.small_size, Align::Left, t.text, Weight::Regular);
    }
    ui.draw.pop_clip();
}
