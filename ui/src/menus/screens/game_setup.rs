//! Game Setup (`do_multiplayerSetup`): Instant Action's setup and every multiplayer
//! game's, laid out our way (MENUS.md, "Game Setup").
//!
//! - **Players**, a window on the left: a row per slot, with the slot's colour (the
//!   stock swatch, cropped from the mirror: a click on it is the stock click), and its
//!   player, side and team as our drop-downs over the stock `ComboBox`es
//!   (`engine::shellcombo`). A choice is the two clicks a player gives the stock one.
//!   Under the rows, the colour palette (the stock swatches) and its Random button.
//! - **Messages and chat**, under the players: the stock message log (a read-only
//!   edit, which also tells of each setting changed) and, in a multiplayer game, the
//!   chat line being typed (the stock edit, as our field).
//! - **The map**, a window on the right: its name and players and what the screen says
//!   of it (the stock labels), the minimap preview (cropped from the mirror; a click on
//!   it is a click on the stock preview, which picks a start location), and the
//!   settings: SELECT MAP, the toggles as check boxes, the other buttons (whose labels
//!   say their state) as buttons.
//! - **Along the bottom**, Previous Menu at the left and the rest (LAUNCH, Load Game) at
//!   the right.
//!
//! Every control is found in the model by kind and place, not by label, so the screen
//! works in every language and in each of its states (host, joiner, Instant Action).
use super::setup_kit as kit;
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Control, What};
use crate::menus::Screen;
use crate::widgets::{theme, Align, Ui};
use alloc::vec::Vec;

/// The stock layout's parts, in design coordinates: the players' table is left of
/// x 450 and above y 300; the minimap preview's frame; the bottom row of buttons.
const TABLE_RIGHT: i32 = 450;
const TABLE_BOTTOM: i32 = 260;
const PREVIEW: [i32; 4] = [627, 129, 155, 155];
const BOTTOM_ROW: i32 = 560;

fn is_combo(c: &Control) -> bool {
    matches!(c.what, What::Shell(Kind::Combo))
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let combos = kit::visible(s, is_combo);
    let nav: Vec<&Control> = kit::visible(s, |c| kit::is_button(c) && c.rect[1] >= BOTTOM_ROW && !c.text.is_empty());
    if combos.is_empty() || nav.is_empty() {
        return false;
    }
    let t = ui.theme;
    kit::backdrop(ui);
    let screen = ui.screen();
    let m = ui.u(32.0);
    let gap = ui.u(t.gap * 2.0);

    // The title: the screen's own (its first label, "GAME SETUP").
    let title = s
        .model
        .controls
        .iter()
        .find(|c| kit::is_label(c) && c.rect[1] < 30 && !c.text.trim().is_empty())
        .map(|c| c.text.trim())
        .unwrap_or("GAME SETUP");
    let title_r = R::new(m, m * 0.5, screen.w - 2.0 * m, ui.u(t.display_size + 16.0));
    ui.text_in(title_r, &theme::caps(title), t.display_size, Align::Left, t.title, Weight::Bold);

    let bottom_h = ui.u(t.button_h);
    let body = R::new(m, title_r.bottom() + ui.u(8.0), screen.w - 2.0 * m, screen.h - title_r.bottom() - ui.u(8.0) - bottom_h - m - gap);
    let left_w = libm::floorf((body.w - gap) * 0.56);
    let left = R::new(body.x, body.y, left_w, body.h);
    let right = R::new(left.right() + gap, body.y, body.w - left_w - gap, body.h);

    players(ui, s, &combos, left);
    map(ui, s, right);

    // The bottom row: the first button (Previous Menu) at the left, the rest at the right.
    let mut nav = nav;
    nav.sort_by_key(|c| c.rect[0]);
    let bw = ui.u(260.0);
    let y = screen.h - m - bottom_h;
    for (i, c) in nav.iter().enumerate() {
        let x = if i == 0 { m } else { screen.w - m - (nav.len() - i) as f32 * (bw + gap) + gap };
        kit::button(ui, s, c, R::new(x, y, bw, bottom_h), None);
    }
    true
}

/// The players' window, and the chat under it when the screen has one.
fn players(ui: &mut Ui, s: &Screen, combos: &[&Control], r: R) {
    let t = ui.theme;
    // Group the combos into rows by their top.
    let mut rows: Vec<(i32, Vec<&Control>)> = Vec::new();
    for c in combos.iter().filter(|c| c.rect[1] < TABLE_BOTTOM && c.rect[0] < TABLE_RIGHT) {
        match rows.iter_mut().find(|r| (r.0 - c.rect[1]).abs() < 6) {
            Some(r) => r.1.push(c),
            None => rows.push((c.rect[1], alloc::vec![*c])),
        }
    }
    rows.sort_by_key(|r| r.0);
    for r in rows.iter_mut() {
        r.1.sort_by_key(|c| c.rect[0]);
    }
    // The chat: a multi-line edit (the log) and a single-line one (the line typed).
    let edits = kit::visible(s, kit::is_edit);
    // A multiplayer game has a second multi-line edit at the right (x 600, the
    // observers); the log is the one under the table.
    let log = edits.iter().find(|c| c.style & 0x4 != 0 && c.rect[0] < TABLE_RIGHT).copied();
    let line = edits.iter().find(|c| c.style & 0x4 == 0).copied();
    // The log also says what the host changed ("Resources: Lots"), in Instant Action too.
    let chat = line.is_some() || log.is_some();
    let row_h = ui.u(t.row_h + 6.0);
    let head_h = ui.u(t.small_size + 10.0);
    let palette_h = ui.u(t.button_h);
    let want = ui.u(t.title_size + 2.0 * t.pad + 8.0) + head_h + rows.len() as f32 * row_h + palette_h + ui.u(t.pad * 3.0);
    let ph = if chat { want.min(r.h * 0.62) } else { want.min(r.h) };
    let pr = R::new(r.x, r.y, r.w, ph);
    let inner = ui.window(pr, "Players");

    // The column headings (the screen's own labels over the table).
    let heads: Vec<&Control> = kit::visible(s, |c| kit::is_label(c) && c.rect[1] < 80 && c.rect[1] > 40 && c.rect[0] < TABLE_RIGHT && !c.text.trim().is_empty());
    // A multiplayer game's rows have a fourth combo, the "Armada" column (x 295:
    // Personal, Allied, ...), between the side (x 203) and the team (x 375), and a
    // ready light left of the swatch (a 10x10 picture at x 14, under the "Comm"
    // heading), green once the player has pressed LAUNCH.
    let armada = rows.iter().any(|r| r.1.iter().any(|c| c.rect[0] >= 290 && c.rect[0] < 370));
    let lights = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Button)) && c.rect[0] < 30 && c.rect[2] == 10 && c.rect[3] == 10 && c.rect[1] < TABLE_BOTTOM);
    let light = if lights.is_empty() { 0.0 } else { ui.u(t.row_h * 0.6) + ui.u(t.gap) };
    let chip = ui.u(t.row_h) + light;
    let ncol = if armada { 4 } else { 3 };
    let cols = column_rects(ui, R::new(inner.x + chip + ui.u(t.gap), inner.y, inner.w - chip - ui.u(t.gap), head_h), ncol);
    let head = |x0: i32, x1: i32, d: &'static str| heads.iter().find(|h| h.rect[0] >= x0 && h.rect[0] < x1).map_or(d, |h| h.text.as_str());
    let names: [&str; 4] = if armada {
        [head(41, 200, "Player"), head(200, 290, "Side"), head(290, 370, "Armada"), head(370, 450, "Team")]
    } else {
        [head(41, 200, "Player"), head(200, 290, "Side"), head(360, 450, "Team"), ""]
    };
    for i in 0..ncol {
        kit::caption(ui, cols[i], names[i]);
    }
    let mut y = inner.y + head_h;
    for (ry, cs) in rows.iter() {
        let rr = R::new(inner.x, y, inner.w, row_h - ui.u(4.0));
        y += row_h;
        // The slot's colour: the stock swatch, left of the row.
        if let Some(l) = lights.iter().find(|l| (l.rect[1] - (*ry + 3)).abs() < 6) {
            let lz = ui.u(t.row_h * 0.6);
            s.mirror_part(ui, l.rect, R::new(rr.x, rr.y + (rr.h - lz) * 0.5, lz, lz), 0xffffffff);
        }
        let cz = ui.u(t.row_h) * 0.7;
        let sw = R::new(rr.x + light, rr.y + (rr.h - cz) * 0.5, cz, cz);
        kit::picture(ui, s, [30, *ry, 16, 16], sw, true);
        let cols = column_rects(ui, R::new(inner.x + chip + ui.u(t.gap), rr.y, inner.w - chip - ui.u(t.gap), rr.h), ncol);
        for c in cs.iter() {
            let col = if c.rect[0] < 200 { 0 } else if c.rect[0] < 290 { 1 } else if c.rect[0] < 370 && armada { 2 } else if armada { 3 } else { 2 };
            kit::combo(ui, s, c, cols[col]);
        }
    }
    // The palette: the stock Random button and the colour swatches, by place.
    let pal_y = inner.bottom() - palette_h;
    let random = kit::visible(s, |c| kit::is_button(c) && !c.text.is_empty() && c.rect[1] > TABLE_BOTTOM && c.rect[1] < 320 && c.rect[0] < 250);
    let swatches = kit::visible(s, |c| kit::is_button(c) && c.text.is_empty() && c.rect[2] == 16 && c.rect[1] > TABLE_BOTTOM && c.rect[1] < 320);
    let mut x = inner.x;
    if let Some(c) = random.first() {
        let br = R::new(x, pal_y, ui.u(180.0), palette_h);
        kit::button(ui, s, c, br, None);
        x = br.right() + ui.u(t.gap * 2.0);
    }
    let sz = palette_h * 0.8;
    for c in swatches.iter() {
        let sr = R::new(x, pal_y + (palette_h - sz) * 0.5, sz, sz);
        kit::picture(ui, s, c.rect, sr, c.enabled);
        x += sz + ui.u(t.gap);
    }

    if chat {
        let cr = R::new(r.x, pr.bottom() + ui.u(t.gap * 2.0), r.w, r.bottom() - pr.bottom() - ui.u(t.gap * 2.0));
        let ci = kit::panel(ui, cr);
        let fh = if line.is_some() { ui.u(t.row_h + 4.0) } else { -ui.u(t.gap) };
        if let Some(lc) = log {
            chat_log(ui, &lc.text, R::new(ci.x, ci.y, ci.w, ci.h - fh - ui.u(t.gap)));
        }
        if let Some(lc) = line {
            // Beside the line, the toggle that says who it goes to ("All"), when the
            // screen has one (a multiplayer game: a toggle right of the line).
            let target = kit::visible(s, |c| matches!(c.what, What::Shell(Kind::Toggle)) && c.rect[1] > 500 && c.rect[0] < TABLE_RIGHT);
            let tw = if target.is_empty() { 0.0 } else { ui.u(120.0) };
            kit::edit(ui, s, lc, R::new(ci.x, ci.bottom() - fh, ci.w - tw - if tw > 0.0 { ui.u(t.gap) } else { 0.0 }, fh));
            if let Some(tc) = target.first() {
                kit::button(ui, s, tc, R::new(ci.right() - tw, ci.bottom() - fh, tw, fh), None);
            }
        }
    }
}

/// The columns of a player row in `r`: player, side, team; or, with `n` 4 (a
/// multiplayer game), player, side, armada, team.
fn column_rects(ui: &Ui, r: R, n: usize) -> [R; 4] {
    let g = ui.u(8.0);
    let w = r.w - (n as f32 - 1.0) * g;
    let parts: &[f32] = if n == 4 { &[0.36, 0.24, 0.20, 0.20] } else { &[0.46, 0.32, 0.22] };
    let mut out = [R::new(r.x, r.y, 0.0, r.h); 4];
    let mut x = r.x;
    for (i, f) in parts.iter().enumerate() {
        let cw = if i + 1 == parts.len() { r.right() - x } else { libm::floorf(w * f) };
        out[i] = R::new(x, r.y, cw, r.h);
        x += cw + g;
    }
    out
}

/// The last lines of a chat log that fit in `r`, newest at the bottom.
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

/// The map's window: what the screen says of the map, the preview, the settings.
fn map(ui: &mut Ui, s: &Screen, r: R) {
    let t = ui.theme;
    // The map's own labels (name, players, "Teams are locked."), top to bottom.
    let mut info: Vec<&Control> = kit::visible(s, |c| kit::is_label(c) && c.rect[0] >= 480 && c.rect[1] < 125 && !c.text.trim().is_empty());
    info.sort_by_key(|c| c.rect[1]);
    // The settings: every labelled shell button right of the table and above the bottom row.
    let mut set: Vec<&Control> = kit::visible(s, |c| {
        kit::is_button(c) && !c.text.is_empty() && c.rect[0] >= 380 && c.rect[1] > TABLE_BOTTOM - 140 && c.rect[1] < BOTTOM_ROW && !matches!(c.what, What::Shell(Kind::Combo))
    });
    kit::dedupe(&mut set);
    set.sort_by_key(|c| (c.rect[0] / 100, c.rect[1]));
    let inner = ui.window(r, info.first().map_or("Map", |c| c.text.trim()));
    // Left: the remaining map labels over the preview; right of them nothing (the
    // preview is square, the window is not).
    let pv = (inner.w * 0.5).min(inner.h * 0.5);
    let mut y = inner.y;
    for c in info.iter().skip(1) {
        let lr = R::new(inner.x, y, inner.w, ui.u(t.text_size + 8.0));
        let col = if c.text.trim_end().ends_with('.') { t.warn } else { t.text };
        ui.text_in(lr, c.text.trim(), t.text_size, Align::Left, col, Weight::Regular);
        y += lr.h;
    }
    let pr = R::new(inner.x, y + ui.u(t.gap), pv, pv);
    ui.draw.rect(pr.inset(-ui.u(2.0)), t.panel_edge);
    kit::picture(ui, s, PREVIEW, pr, true);
    // Beside the preview: SELECT MAP and the toggles; under it the rest, two columns.
    let bh = ui.u(t.button_h);
    let sx = pr.right() + ui.u(t.pad * 1.5);
    let sw = inner.right() - sx;
    let mut sy = pr.y;
    let mut rest: Vec<&Control> = Vec::new();
    for c in set.iter() {
        let beside = matches!(c.what, What::Shell(Kind::SelBox | Kind::Toggle)) || c.rect[0] >= 480;
        if beside && sy + bh <= pr.bottom() + 0.5 {
            let br = R::new(sx, sy, sw, bh);
            match c.what {
                What::Shell(Kind::Toggle) => {
                    kit::check(ui, s, c, br);
                }
                _ => {
                    kit::button(ui, s, c, br, None);
                }
            }
            sy += bh + ui.u(t.gap);
        } else {
            rest.push(c);
        }
    }
    let gy = pr.bottom() + ui.u(t.pad * 1.5);
    let cw = (inner.w - ui.u(t.gap)) * 0.5;
    for (i, c) in rest.iter().enumerate() {
        let (col, row) = (i % 2, i / 2);
        let br = R::new(inner.x + col as f32 * (cw + ui.u(t.gap)), gy + row as f32 * (bh + ui.u(t.gap)), cw, bh);
        if br.bottom() > inner.bottom() + 0.5 {
            break;
        }
        if matches!(c.what, What::Shell(Kind::Toggle)) {
            kit::check(ui, s, c, br);
        } else {
            kit::button(ui, s, c, br, None);
        }
    }
}
