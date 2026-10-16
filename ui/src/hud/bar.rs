//! The button bar, ours: the stock bar's buttons (`engine::palette`) as a 5x3 grid of
//! position keys, drawn with the game's icons in our style, with the game's tooltips
//! (HUD.md, "The button bar").
//!
//! The stock bar keeps running: its `Update` decides which buttons the selection and
//! the open menu show, and a press goes through the button's own press function, so
//! every menu (orders, build, special weapons, AI, formations, trade, back and close)
//! works as stock. It is neither drawn nor given the mouse (`engine::hud`).
//!
//! The grid is grid/README.md's, to the cell:
//!
//! ```text
//!  Q  W  E  R | T      T  cancel, in every build menu
//!  A  S  D  F | G      G  back (or close), in every submenu
//!  Z  X  C  V | B      B  the 13th item of a construction ship's build list
//! ```
//!
//! Keys are scancodes, so they are positions on any layout. A key does what clicking
//! its button does, and nothing while chat has the keyboard, Ctrl or Alt is held, or
//! the game is not in the foreground. Clicks and keys are both pressed in the bar's
//! own keyboard step (the game's simulate), not in the UI's frame.
use crate::engine::palette::{self, Kind};
use crate::engine::{hud, sound, sprite};
use crate::gfx::{fade, mix, Weight, R};
use crate::rt::win::*;
use crate::widgets::{Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

const COLS: usize = 5;
const ROWS: usize = 3;
const CELLS: usize = COLS * ROWS;
const fn cell(c: usize, r: usize) -> usize {
    r * COLS + c
}
const CELL_CANCEL: usize = cell(4, 0);
const CELL_BACK: usize = cell(4, 1);
const CELL_OVERFLOW: usize = cell(4, 2);

/// Set-1 scancodes, row by row: Q W E R T / A S D F G / Z X C V B.
const SCAN: [u8; CELLS] = [0x10, 0x11, 0x12, 0x13, 0x14, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x2c, 0x2d, 0x2e, 0x2f, 0x30];
const LABEL: &[u8; CELLS] = b"QWERTASDFGZXCVB";
/// Free cells are taken down each of the four left columns, then the fifth column's
/// spare: the left of the keyboard fills first.
const FILL: [usize; CELLS] = [
    cell(0, 0), cell(0, 1), cell(0, 2),
    cell(1, 0), cell(1, 1), cell(1, 2),
    cell(2, 0), cell(2, 1), cell(2, 2),
    cell(3, 0), cell(3, 1), cell(3, 2),
    CELL_OVERFLOW, CELL_CANCEL, CELL_BACK,
];

/// `eMenuType` 0 and 8 are the top level.
fn top_level(menu: u32) -> bool {
    menu == 0 || menu == 8
}

struct Item {
    btn: usize,
    desc: palette::Desc,
}

/// The bar's buttons and the cell each goes in: `cells[c]` is the button under cell
/// `c`, 0 for an empty one.
fn layout(p: usize, menu: u32) -> ([usize; CELLS], Vec<Item>) {
    let mut items = Vec::new();
    for i in 0..palette::N_SLOTS {
        let b = palette::slot(p, i);
        if palette::valid(b) {
            items.push(Item { btn: b, desc: palette::describe(b) });
        }
    }
    let at = assign(&items, menu);
    let mut cells = [0usize; CELLS];
    for (it, c) in items.iter().zip(at) {
        if let Some(c) = c {
            cells[c] = it.btn;
        }
    }
    (cells, items)
}

/// grid.c's `assign`: the cell each item goes in, None only if the grid is full.
fn assign(it: &[Item], menu: u32) -> Vec<Option<usize>> {
    let mut used = [false; CELLS];
    let mut cell_of: Vec<Option<usize>> = alloc::vec![None; it.len()];
    let take = |used: &mut [bool; CELLS], c: usize| -> Option<usize> {
        if c < CELLS && !used[c] {
            used[c] = true;
            Some(c)
        } else {
            None
        }
    };
    let take_in_row = |used: &mut [bool; CELLS], r: usize| (0..4).find_map(|c| take(used, cell(c, r)));
    let take_free = |used: &mut [bool; CELLS]| FILL.iter().find_map(|&c| take(used, c));
    let is = |i: usize, k: Kind, n: &str| it[i].desc.kind == k && it[i].desc.name == n;

    // 1. the two keys that never move
    for i in 0..it.len() {
        if is(i, Kind::Command, "cancel") {
            cell_of[i] = take(&mut used, CELL_CANCEL);
        } else if is(i, Kind::Menu, "back") || is(i, Kind::Menu, "close") {
            cell_of[i] = take(&mut used, CELL_BACK);
        }
    }
    // T and G mean cancel and back everywhere: nothing else goes there unless the grid
    // would otherwise overflow.
    used[CELL_CANCEL] = true;
    used[CELL_BACK] = true;
    if top_level(menu) {
        // 2a. the build opener first in the top row, the other menus after it, special
        //     weapons along the bottom, commands between
        let opener = |i: usize| is(i, Kind::Menu, "build") || is(i, Kind::Menu, "research") || is(i, Kind::Menu, "evolve");
        for i in 0..it.len() {
            if cell_of[i].is_none() && opener(i) {
                cell_of[i] = take_in_row(&mut used, 0);
            }
        }
        for (pass, kind, row) in [(0, Kind::Menu, 0), (1, Kind::Weapon, 2), (2, Kind::Command, 1)] {
            let _ = pass;
            for i in 0..it.len() {
                if cell_of[i].is_none() && it[i].desc.kind == kind {
                    cell_of[i] = take_in_row(&mut used, row);
                }
            }
        }
    } else {
        // 2b. a submenu: commands where their preferred position puts them, a
        //     three-column plan (the AI menu is a 3x3)
        for i in 0..it.len() {
            if cell_of[i].is_none() && it[i].desc.kind == Kind::Command {
                if let Some((c, r)) = it[i].desc.pos {
                    if (0..3).contains(&c) && (0..ROWS as i32).contains(&r) {
                        cell_of[i] = take(&mut used, cell(c as usize, r as usize));
                    }
                }
            }
        }
    }
    // 3. everything else in stock's order, down the left columns first
    for c in cell_of.iter_mut() {
        if c.is_none() {
            *c = take_free(&mut used);
        }
    }
    // 4. a grid still too small gets T and G after all
    used[CELL_CANCEL] = false;
    used[CELL_BACK] = false;
    for c in cell_of.iter_mut() {
        if c.is_none() {
            *c = take_free(&mut used);
        }
    }
    cell_of
}

// ---- state kept between frames -------------------------------------------------------

struct State {
    /// The button a click released over, pressed at the bar's next keyboard step.
    pending: usize,
    /// UI frames since `pending` was set: a press the game did not take in time (the
    /// game paused, the selection gone) is dropped rather than made late.
    pending_age: u32,
    /// Grid keys: virtual key per cell, and held last time.
    vk: [u8; CELLS],
    down: [bool; CELLS],
    /// The button under the pointer, and since when (for the tooltip).
    hover: usize,
    hover_since: f64,
    tip: Option<(usize, bool, f64, String)>,
    /// The last layout logged, to log each change once.
    last_log: String,
    /// The menu the buttons were last built for, and when (see `run`).
    built_menu: u32,
    built_at: f64,
}

static mut STATE: State = State {
    pending: 0,
    pending_age: 0,
    vk: [0; CELLS],
    down: [false; CELLS],
    hover: 0,
    hover_since: 0.0,
    tip: None,
    last_log: String::new(),
    built_menu: 0,
    built_at: -1.0,
};

fn st() -> &'static mut State {
    unsafe { &mut *core::ptr::addr_of_mut!(STATE) }
}

/// Patches the bar's keyboard step. False: the bar stays stock.
pub fn install() -> bool {
    let s = st();
    for (c, &sc) in SCAN.iter().enumerate() {
        s.vk[c] = unsafe { MapVirtualKeyA(sc as u32, 1) } as u8;
    }
    palette::install_keys(crate::config::get().hud.bar_keys, on_keys)
}

// ---- the game's simulate: presses ----------------------------------------------------

fn game_has_focus() -> bool {
    let w = unsafe { GetForegroundWindow() };
    if w.is_null() {
        return false;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(w, &mut pid) };
    pid == unsafe { GetCurrentProcessId() }
}

fn held(vk: u8) -> bool {
    vk != 0 && (unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000) != 0
}

/// The cell whose key went down since the last step.
fn key_pressed() -> Option<usize> {
    let s = st();
    let ok = game_has_focus() && !hud::typing() && !held(0x11) && !held(0x12);
    let mut hit = None;
    for c in 0..CELLS {
        let d = held(s.vk[c]);
        if d && !s.down[c] && ok && hit.is_none() {
            hit = Some(c);
        }
        s.down[c] = d;
    }
    hit
}

/// After the stock keyboard step, in the bar's `Update`: the buttons are those of this
/// update.
fn on_keys(p: usize) {
    let s = st();
    if hud::hidden(p) {
        s.pending = 0;
        return;
    }
    // A press below changes the menu at once, and the buttons only at the bar's next
    // update; the UI's frames in between lay out by the menu these buttons are for.
    s.built_menu = palette::menu(p);
    s.built_at = crate::rt::now();
    let (cells, items) = layout(p, s.built_menu);
    if s.pending != 0 {
        let b = core::mem::take(&mut s.pending);
        if let Some(it) = items.iter().find(|it| it.btn == b) {
            crate::log!("bar: click {}{}", it.desc.name, if palette::state(b) == 0 { " (disabled)" } else { "" });
            palette::press(b);
        }
    }
    if crate::config::get().hud.bar_keys {
        if let Some(c) = key_pressed() {
            let b = cells[c];
            if b != 0 {
                let name = items.iter().find(|it| it.btn == b).map_or("?", |it| it.desc.name.as_str());
                crate::log!("bar: key {}={}{}", LABEL[c] as char, name, if palette::state(b) == 0 { " (disabled)" } else { "" });
                palette::press(b);
            }
        }
    }
}

// ---- the UI's frame: drawing and the mouse -----------------------------------------

/// Where the bar goes: at the bottom of the screen, between the minimap and the info
/// panel when there is room, else above the info panel, aligned to its left edge.
fn place(ui: &Ui, w: f32, h: f32) -> R {
    let m = ui.u(10.0);
    let info = if hud::hidden(hud::SHIP_DISPLAY) { None } else { hud::panel_rect(hud::SHIP_DISPLAY, ui.w, ui.h) };
    let mini = hud::panel_rect(hud::MAP_RADAR, ui.w, ui.h);
    match (info, mini) {
        (Some(i), Some(mm)) if i[0] - mm[2] >= w + 2.0 * m => {
            R::new(mm[2] + ((i[0] - mm[2]) - w) * 0.5, ui.h - m - h, w, h)
        }
        (Some(i), _) => R::new(i[0], (i[1] - m - h).max(m), w, h),
        _ => R::new((ui.w - w) * 0.5, ui.h - m - h, w, h),
    }
}

pub fn run(ui: &mut Ui) {
    let s = st();
    if s.pending != 0 {
        s.pending_age += 1;
        if s.pending_age > 3 {
            s.pending = 0;
        }
    }
    let p = palette::palette();
    if p == 0 || hud::hidden(p) || crate::engine::ingame::hud_hidden() {
        s.hover = 0;
        s.tip = None;
        return;
    }
    // The menu the buttons were built for, while the bar's updates run (they do not
    // while the game is paused or nothing of the player's is selected).
    let menu = if crate::rt::now() - s.built_at < 0.25 { s.built_menu } else { palette::menu(p) };
    let (cells, items) = layout(p, menu);
    if items.is_empty() {
        s.hover = 0;
        s.tip = None;
        return;
    }
    log_layout(p, menu, &cells, &items);

    let cfg = crate::config::get();
    let t = ui.theme;
    let (cs, gap, pad) = (ui.u(62.0), ui.u(5.0), ui.u(8.0));
    let gw = COLS as f32 * cs + (COLS - 1) as f32 * gap;
    let gh = ROWS as f32 * cs + (ROWS - 1) as f32 * gap;
    let outer = place(ui, gw + 2.0 * pad, gh + 2.0 * pad);
    ui.panel(outer);
    // The fifth column, cancel/back, set off from the grid proper.
    let col4 = R::new(outer.x + pad + 4.0 * (cs + gap) - gap * 0.5 - ui.u(1.0), outer.y + pad, ui.u(1.0).max(1.0), gh);
    ui.draw.rect(col4, fade(t.panel_edge, 0.5));

    let mut hovered = 0usize;
    let mut hovered_rect = R::new(0.0, 0.0, 0.0, 0.0);
    for c in 0..CELLS {
        let (col, row) = (c % COLS, c / COLS);
        let r = R::new(outer.x + pad + col as f32 * (cs + gap), outer.y + pad + row as f32 * (cs + gap), cs, cs);
        let b = cells[c];
        let rad = ui.u(5.0);
        if b == 0 {
            ui.draw.rect_round(r, rad, fade(t.track, 0.55));
            continue;
        }
        let enabled = palette::state(b) != 0;
        let active = palette::state(b) == 2;
        let id = ui.id(&crate::sfmt!("bar.cell.{}", c));
        let (hot, held, clicked) = ui.interact(id, r);
        if hot {
            hovered = b;
            hovered_rect = r;
        }
        if clicked && enabled {
            s.pending = b;
            s.pending_age = 0;
        }
        let edge = if active { t.accent } else if hot && enabled { mix(t.accent, t.panel_edge, 0.35) } else { t.panel_edge };
        let bg = if held && enabled { t.button_down } else if hot && enabled { t.button_hot } else { t.button };
        ui.draw.rect_round(r, rad, edge);
        let e = ui.u(if active { 2.5 } else { 1.5 }).max(1.0);
        ui.draw.rect_round(r.inset(e), (rad - e).max(0.0), bg);
        // The game's icon, inside the edge.
        let icon = r.inset(e + ui.u(2.0));
        match sprite::art(palette::sprite(b)) {
            Some((tex, uv)) => {
                let tint = if enabled { 0xffffffff } else { 0xff6a6a6a };
                ui.draw.image_uv(tex, icon, uv, tint);
            }
            None => {
                // No art: the button's name.
                let name = items.iter().find(|it| it.btn == b).map_or("", |it| it.desc.name.as_str());
                let tc = if enabled { t.text } else { t.text_dim };
                ui.text_in(icon, name, 13.0, Align::Centre, tc, Weight::Regular);
            }
        }
        if !enabled {
            ui.draw.rect_round(r.inset(e), (rad - e).max(0.0), 0x60000000);
        }
        if cfg.hud.bar_keys && cfg.hud.bar_labels {
            let k = [LABEL[c]];
            let label = core::str::from_utf8(&k).unwrap_or("?");
            let sz = ui.u(15.0);
            let lr = R::new(r.x + ui.u(3.0), r.y + ui.u(2.0), sz * 1.1, sz * 1.15);
            ui.draw.rect_round(lr, ui.u(3.0), 0xb0000000);
            let col = if enabled { 0xffffe48f } else { 0xff8c8c8c };
            ui.text_in(lr, label, 15.0, Align::Centre, col, Weight::Bold);
        }
    }

    // Hover: the rollover sound on entering an enabled button, then the tooltip.
    let now = ui.time();
    if hovered != s.hover {
        if hovered != 0 && palette::state(hovered) != 0 {
            sound::rollover();
        }
        s.hover = hovered;
        s.hover_since = now;
        s.tip = None;
    }
    if hovered != 0 && now - s.hover_since > 0.35 {
        let verbose = now - s.hover_since > 1.5;
        let stale = match &s.tip {
            Some((b, v, at, _)) => *b != hovered || *v != verbose || now - at > 0.5,
            None => true,
        };
        if stale {
            let text = palette::tooltip(hovered, verbose).or_else(|| palette::tooltip(hovered, false)).unwrap_or_default();
            s.tip = Some((hovered, verbose, now, text));
        }
        if let Some((_, _, _, text)) = &s.tip {
            tooltip(ui, hovered_rect, outer, text);
        }
    }
}

/// The tooltip, above the bar (below it if there is no room), at the button's left.
fn tooltip(ui: &mut Ui, at: R, bar: R, text: &str) {
    let text = text.trim_matches(|c: char| c == '\n' || c == '\r' || c == ' ');
    if text.is_empty() {
        return;
    }
    // '^' marks the hotkey letter in the game's strings, and "<...>" names stock's
    // hotkey, which does nothing while the grid's keys are on.
    let keys = crate::config::get().hud.bar_keys;
    let mut clean = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '\r' | '^' => {}
            '<' if keys => depth += 1,
            '>' if keys && depth > 0 => depth -= 1,
            _ if depth == 0 => clean.push(c),
            _ => {}
        }
    }
    let clean = String::from(clean.replace(" :", ":").trim());
    let t = ui.theme;
    let size = ui.u(19.0);
    let maxw = ui.u(460.0);
    let (tw, th) = crate::text::measure_wrapped(crate::text::font(), &clean, size, maxw);
    let pad = ui.u(9.0);
    let (w, h) = (tw + 2.0 * pad, th + 2.0 * pad);
    let mut x = at.x.min(ui.w - w - ui.u(4.0)).max(ui.u(4.0));
    let mut y = bar.y - h - ui.u(6.0);
    if y < 0.0 {
        y = bar.bottom() + ui.u(6.0);
        x = x.max(0.0);
    }
    let r = R::new(x, y, w, h);
    let rad = ui.u(5.0);
    ui.draw.rect_round(r, rad, t.panel_edge);
    ui.draw.rect_round(r.inset(1.0), rad - 1.0, 0xf00a1220);
    ui.draw.text_wrapped(&clean, size, maxw, r.x + pad, r.y + pad, t.text, Weight::Regular);
}

/// `bar: menu=0 | Q=build W=order ...`, once per change, as GridLayout.log does.
fn log_layout(p: usize, menu: u32, cells: &[usize; CELLS], items: &[Item]) {
    let mut line = crate::sfmt!("bar: menu={} mode={} |", menu, palette::mode(p));
    for (c, &b) in cells.iter().enumerate() {
        if b == 0 {
            continue;
        }
        let it = items.iter().find(|it| it.btn == b);
        line.push_str(&crate::sfmt!(" {}={}{}", LABEL[c] as char, it.map_or("?", |it| it.desc.name.as_str()),
            if palette::state(b) == 0 { "(off)" } else { "" }));
    }
    let s = st();
    if s.last_log != line {
        crate::log!("{}", line);
        s.last_log = line;
    }
}
