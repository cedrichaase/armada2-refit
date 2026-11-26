//! The Refit screen's Keys tab (`MENUS.md`, "The Keys tab"): every key of the game's
//! key map (`Input.map`) and the command bar's grid keys (QOL.asi), listed by what they
//! do, searchable, and rebound by pressing the new key. The model and every write are
//! `crate::keys`; this file only draws and asks it.
use crate::gfx::{Weight, R};
use crate::keys::{self, Role};
use crate::widgets::theme::{self, State};
use crate::widgets::{Align, Ui};
use crate::{log, sfmt};
use a2fmt::inputmap::{self, Binding, Mod};
use alloc::string::String;
use alloc::vec::Vec;

/// What a press is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    /// An entry of `keys::map().entries`.
    Entry(usize),
    /// A cell of the command bar's grid (0..15).
    Cell(usize),
}

/// The categories in the list, in order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cat {
    Grid,
    Map(usize),
}

struct St {
    cat: usize,
    search: String,
    capture: Option<Target>,
    /// Why the last press was not taken, shown in the capture box.
    refused: String,
    /// The search field held the keyboard at the start of this frame (its Esc is its own).
    search_focus: bool,
    /// The help strip's text for what the pointer is over.
    help: String,
}

static mut S: St = St { cat: 0, search: String::new(), capture: None, refused: String::new(), search_focus: false, help: String::new() };

fn st() -> &'static mut St {
    unsafe { &mut *core::ptr::addr_of_mut!(S) }
}

/// Whether the tab has anything to list.
pub fn available() -> bool {
    keys::map().is_some() || keys::has_grid()
}

/// The tab was opened: the player's keys are read again.
pub fn opened() {
    keys::refresh();
    let s = st();
    s.capture = None;
    s.refused.clear();
}

/// Esc this frame belongs to the tab (a capture, or the search field), not the modal.
pub fn takes_escape() -> bool {
    let s = st();
    s.capture.is_some() || s.search_focus
}

/// The modal's Esc, when `takes_escape`.
pub fn escape(ui: &mut Ui) {
    let s = st();
    if s.capture.take().is_some() {
        log!("refit: keys: capture cancelled");
        s.refused.clear();
    }
    s.search_focus = false;
    ui.blur();
}

/// A QOL row's value is on, or None when QOL has no such row.
fn qol_on(key: &str) -> Option<bool> {
    let t = crate::settings::tables().iter().find(|t| t.plugin == "QOL")?;
    let r = t.rows.iter().find(|r| r.key() == key)?;
    Some(crate::rt::parse_i32(&crate::settings::value(r)).unwrap_or(0) != 0)
}

/// The grid's keys are on: the bar's own keys do nothing.
fn grid_on() -> bool {
    keys::has_grid() && qol_on("BarKeys").unwrap_or(false)
}

fn cats(advanced: bool) -> Vec<Cat> {
    let mut out = Vec::new();
    if keys::has_grid() {
        out.push(Cat::Grid);
    }
    if let Some(m) = keys::map() {
        for (i, c) in m.cats.iter().enumerate() {
            if c.role != Role::Debug || advanced {
                out.push(Cat::Map(i));
            }
        }
    }
    out
}

fn cat_name(c: Cat) -> String {
    match c {
        Cat::Grid => String::from("Command bar grid"),
        Cat::Map(i) => keys::map().map_or(String::new(), |m| m.cats[i].name.clone()),
    }
}

/// How many of the player's choices a category holds.
fn cat_mine(c: Cat) -> usize {
    match c {
        Cat::Grid => (0..keys::CELLS).filter(|&c| keys::grid_is_mine(c)).count(),
        Cat::Map(i) => keys::map().map_or(0, |m| m.cats[i].entries.iter().filter(|&&e| keys::is_mine(&m.entries[e])).count()),
    }
}

/// A cell's place in words: "Top row, 1st".
fn cell_label(c: usize) -> String {
    const ROWS: [&str; 3] = ["Top row", "Middle row", "Bottom row"];
    const NTH: [&str; 5] = ["1st", "2nd", "3rd", "4th", "5th"];
    sfmt!("{}, {} button", ROWS[c / 5], NTH[c % 5])
}

fn key_label(name: &str) -> &'static str {
    inputmap::key_by_name(name).map_or("?", |k| k.label)
}

/// What a grid key presses alongside: the map's entries a press of the key (with no
/// Control and no Alt, which the grid ignores) answers, without the bar's own.
fn grid_others(name: &str, skip: Option<usize>) -> Vec<String> {
    let Some(m) = keys::map() else { return Vec::new() };
    let mut out = Vec::new();
    for shift in [false, true] {
        let b = Binding { key: String::from(name), mods: [Mod::Off, if shift { Mod::On } else { Mod::Off }, Mod::Off] };
        for i in keys::others(&b, None, !grid_on()) {
            let l = m.entries[i].label.clone();
            if !out.contains(&l) {
                out.push(l);
            }
        }
    }
    for c in (0..keys::CELLS).filter(|&c| Some(c) != skip) {
        if keys::grid_key(c).is_some_and(|k| k.eq_ignore_ascii_case(name)) {
            out.push(sfmt!("grid: {}", cell_label(c)));
        }
    }
    out
}

/// What else a press of an entry's keys answers.
fn entry_others(i: usize, b: &Binding, bar: bool) -> Vec<String> {
    let Some(m) = keys::map() else { return Vec::new() };
    let mut out: Vec<String> = keys::others(b, Some(i), bar).into_iter().map(|j| m.entries[j].label.clone()).collect();
    // The grid's keys press with any Shift, never with Control or Alt.
    if grid_on() && b.mods[0] != Mod::On && b.mods[2] != Mod::On {
        for c in 0..keys::CELLS {
            if keys::grid_key(c).is_some_and(|k| k.eq_ignore_ascii_case(&b.key)) {
                out.push(sfmt!("grid: {}", cell_label(c)));
            }
        }
    }
    out
}

fn others_line(o: &[String]) -> String {
    let mut s = String::from("Same key as: ");
    for (n, l) in o.iter().take(2).enumerate() {
        if n > 0 {
            s.push_str(", ");
        }
        s.push_str(l);
    }
    if o.len() > 2 {
        s.push_str(&sfmt!(" and {} more", o.len() - 2));
    }
    s
}

fn lower(s: &str) -> String {
    crate::rt::lower(s)
}

/// One row the page shows.
enum Row {
    Heading(String),
    Entry(usize),
    Cell(usize),
}

fn rows(cat: Cat, search: &str, advanced: bool) -> Vec<Row> {
    let mut out = Vec::new();
    let q = lower(search.trim());
    if !q.is_empty() {
        // Every category, the matches under each one's name.
        if keys::has_grid() {
            let all = "command bar grid".contains(q.as_str());
            let cells: Vec<usize> = (0..keys::CELLS)
                .filter(|&c| {
                    let k = keys::grid_key(c).unwrap_or("");
                    all || lower(&cell_label(c)).contains(q.as_str()) || lower(k) == q || lower(key_label(k)) == q
                })
                .collect();
            if !cells.is_empty() {
                out.push(Row::Heading(String::from("Command bar grid")));
                out.extend(cells.into_iter().map(Row::Cell));
            }
        }
        let Some(m) = keys::map() else { return out };
        for (ci, c) in m.cats.iter().enumerate() {
            if c.role == Role::Debug && !advanced {
                continue;
            }
            let hits: Vec<usize> = c
                .entries
                .iter()
                .copied()
                .filter(|&e| {
                    let en = &m.entries[e];
                    let key = keys::binding(en).map(|b| lower(&b.label())).unwrap_or_default();
                    lower(&en.label).contains(q.as_str()) || lower(&en.help).contains(q.as_str()) || lower(&keys::action(en).name).contains(q.as_str()) || key == q
                })
                .collect();
            if !hits.is_empty() {
                out.push(Row::Heading(m.cats[ci].name.clone()));
                out.extend(hits.into_iter().map(Row::Entry));
            }
        }
        return out;
    }
    match cat {
        Cat::Grid => out.extend((0..keys::CELLS).map(Row::Cell)),
        Cat::Map(ci) => {
            let Some(m) = keys::map() else { return out };
            let mut group = String::new();
            for &e in &m.cats[ci].entries {
                let g = &keys::action(&m.entries[e]).group;
                if *g != group && !g.is_empty() {
                    out.push(Row::Heading(g.clone()));
                }
                group = g.clone();
                out.push(Row::Entry(e));
            }
        }
    }
    out
}

/// The line under the search field: what this category's keys are and when a change
/// applies.
fn cat_note(cat: Cat) -> String {
    match cat {
        Cat::Grid => {
            if grid_on() {
                String::from("Which key presses which button of the command bar, by its place. A change applies at once.")
            } else {
                String::from("Grid keys are off (Interface tab): the command bar takes the game's own keys, under Bar.")
            }
        }
        Cat::Map(i) => {
            let role = keys::map().map_or(Role::General, |m| m.cats[i].role);
            let mut s = String::new();
            if role == Role::Bar && grid_on() {
                s.push_str("Grid keys are on (Interface tab): the command bar takes the grid's keys and these do nothing. ");
            }
            if cat_name(cat) == "Selection" && qol_on("ShiftAddsToGroup") == Some(true) {
                s.push_str("Ctrl+number stores a group, Shift+number adds to it, Alt+number selects it and centres the view. ");
            }
            s.push_str("A change applies when the game is started again.");
            s
        }
    }
}

/// The page: the categories down the left, the search field and the keys on the right.
pub fn page(ui: &mut Ui, page: R, advanced: bool) {
    let t = ui.theme;
    let s = st();
    s.help.clear();
    if !available() {
        ui.note(R::new(page.x, page.y, page.w, ui.u(30.0)), keys::why());
        return;
    }
    let live = s.capture.is_none();
    let list = cats(advanced);
    if s.cat >= list.len() {
        s.cat = 0;
    }
    let gap = ui.u(16.0);
    let lw = ui.u(270.0).min(page.w * 0.3);
    let left = R::new(page.x, page.y, lw, page.h);
    let right = R::new(page.x + lw + gap, page.y, page.w - lw - gap, page.h);

    // The categories.
    let ch = ui.u(36.0);
    let total = ch * list.len() as f32;
    ui.inert_unless(live, |ui| {
        ui.scroll_area(left, "keys cats", total, |ui, area| {
            for (i, &c) in list.iter().enumerate() {
                let r = R::new(area.x, area.y + ch * i as f32, area.w, ch);
                let id = ui.id(&sfmt!("keys cat {}", i));
                let (hot, held, clicked) = ui.interact(id, r);
                let mut stt = State::of(ui, id, hot, held);
                stt.on = i == s.cat && s.search.trim().is_empty();
                theme::row(ui, r, i, stt);
                let name = cat_name(c);
                let n = cat_mine(c);
                let col = theme::row_text(ui, stt);
                ui.text_in(R::new(r.x + ui.u(12.0), r.y, r.w - ui.u(40.0), r.h), &name, t.text_size, Align::Left, col, Weight::Regular);
                if n > 0 {
                    let d = ui.u(8.0);
                    ui.draw.rect_round(R::new(r.right() - ui.u(18.0), r.y + (r.h - d) * 0.5, d, d), d * 0.5, t.accent);
                }
                if clicked {
                    s.cat = i;
                    s.search.clear();
                }
            }
        });
    });

    // The search field and the category's note.
    let fh = ui.u(38.0);
    let field = R::new(right.right() - ui.u(320.0).min(right.w * 0.45), right.y, ui.u(320.0).min(right.w * 0.45), fh);
    let title = R::new(right.x, right.y, field.x - right.x - gap, fh);
    let cat = list[s.cat];
    let searching = !s.search.trim().is_empty();
    let heading = if searching { String::from("Search") } else { cat_name(cat) };
    theme::heading(ui, title, &heading);
    let fid = ui.id("keys search");
    s.search_focus = ui.has_focus(fid);
    ui.inert_unless(live, |ui| {
        let _ = ui.text_field(field, "keys search", &mut s.search, 40);
    });
    if s.search.is_empty() && !ui.has_focus(fid) {
        ui.text_in(R::new(field.x + ui.u(12.0), field.y, field.w, field.h), "Search keys", t.small_size, Align::Left, t.text_dim, Weight::Regular);
    }
    let note = if searching { String::from("Every key whose name, help or key matches. A click on a row binds it.") } else { cat_note(cat) };
    let nr = R::new(right.x, right.y + fh + ui.u(4.0), right.w, ui.u(24.0));
    ui.draw.push_clip(nr);
    ui.text_in(nr, &note, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    ui.draw.pop_clip();

    // The rows.
    let body = R::new(right.x, nr.bottom() + ui.u(6.0), right.w, (right.bottom() - nr.bottom() - ui.u(6.0)).max(0.0));
    let rs = rows(cat, &s.search, advanced);
    let (rh, hh) = (ui.u(48.0), ui.u(30.0));
    let total: f32 = rs.iter().map(|r| if matches!(r, Row::Heading(_)) { hh } else { rh }).sum();
    let bar_role = |e: usize| keys::map().is_some_and(|m| m.cats[m.entries[e].cat].role == Role::Bar);
    ui.inert_unless(live, |ui| {
        ui.scroll_area(body, &sfmt!("keys rows {}", if searching { 999 } else { s.cat }), total, |ui, area| {
            let mut y = area.y;
            let mut n = 0;
            for row in rs.iter() {
                let h = if matches!(row, Row::Heading(_)) { hh } else { rh };
                let r = R::new(area.x, y, area.w, h);
                y += h;
                // Only what is in view is drawn and asked.
                if r.bottom() < body.y || r.y > body.bottom() {
                    if !matches!(row, Row::Heading(_)) {
                        n += 1;
                    }
                    continue;
                }
                match *row {
                    Row::Heading(ref g) => {
                        ui.text_in(R::new(r.x + ui.u(4.0), r.y + ui.u(4.0), r.w, r.h - ui.u(4.0)), g, t.small_size, Align::Left, t.title, Weight::Bold);
                    }
                    Row::Entry(e) => {
                        let m = keys::map().unwrap();
                        let en = &m.entries[e];
                        let a = keys::action(en);
                        let b = keys::binding(en);
                        let off = bar_role(e) && grid_on();
                        let others = b.as_ref().map(|b| entry_others(e, b, bar_role(e) || !grid_on())).unwrap_or_default();
                        let fixed = !a.rebindable();
                        let shown = b.as_ref().map(|b| b.label()).unwrap_or_else(|| {
                            let ks = a.keys();
                            ks.iter().map(|k| key_label(k.trim_start_matches('*'))).collect::<Vec<_>>().join("+")
                        });
                        if key_row(ui, r, n, &sfmt!("keys e {}", e), &en.label, &shown, &others, keys::is_mine(en), keys::pending(en), off, fixed) {
                            if fixed {
                                log!("refit: keys: {} is fixed ({})", a.name, shown);
                            } else {
                                s.capture = Some(Target::Entry(e));
                                s.refused.clear();
                                ui.blur();
                                log!("refit: keys: press a key for {}", a.name);
                            }
                        }
                        if ui.input.mouse.is_some_and(|p| r.contains(p) && body.contains(p)) {
                            let mut h = String::from(en.label.as_str());
                            if !en.help.is_empty() {
                                h.push_str(". ");
                                h.push_str(&en.help);
                            }
                            h.push_str(&sfmt!(" ({} in Input.map", a.name));
                            if let Some(sb) = keys::stock_binding(en)
                                && keys::is_mine(en)
                            {
                                h.push_str(&sfmt!("; stock: {}", sb.label()));
                            }
                            h.push_str(if fixed { "; this one cannot be changed here.)" } else { ".) A click binds it; applies when the game is started again." });
                            s.help = h;
                        }
                    }
                    Row::Cell(c) => {
                        let name = keys::grid_key(c).unwrap_or("");
                        let others = grid_others(name, Some(c));
                        if key_row(ui, r, n, &sfmt!("keys c {}", c), &sfmt!("Command bar: {}", cell_label(c)), key_label(name), &others, keys::grid_is_mine(c), false, !grid_on(), false) {
                            s.capture = Some(Target::Cell(c));
                            s.refused.clear();
                            ui.blur();
                            log!("refit: keys: press a key for grid cell {}", c + 1);
                        }
                        if ui.input.mouse.is_some_and(|p| r.contains(p) && body.contains(p)) {
                            s.help = sfmt!("The key that presses the command bar's button in this place (QOL.ini [Keys] Cell{}). A click binds it; applies at once.", c + 1);
                        }
                    }
                }
                if !matches!(row, Row::Heading(_)) {
                    n += 1;
                }
            }
        });
    });

    if s.capture.is_some() {
        capture_box(ui, page);
    }
}

/// One key's row: the label (and what else the key does) on the left, the key on the
/// right. True when it was clicked.
#[allow(clippy::too_many_arguments)]
fn key_row(ui: &mut Ui, r: R, idx: usize, id: &str, label: &str, key: &str, others: &[String], mine: bool, pending: bool, off: bool, fixed: bool) -> bool {
    let t = ui.theme;
    let id = ui.id(id);
    let (hot, held, clicked) = ui.interact(id, r);
    let stt = State::of(ui, id, hot, held);
    theme::row(ui, r, idx, stt);
    let pad = ui.u(10.0);
    let kw = ui.u(190.0).min(r.w * 0.35);
    let cap = R::new(r.right() - pad - kw, r.y + ui.u(6.0), kw, r.h - ui.u(12.0));
    let mut lr = R::new(r.x + pad + ui.u(10.0), r.y, cap.x - r.x - pad * 3.0, r.h);
    if pending {
        let w = theme::status_badge_w(ui, "Restart needed");
        let br = R::new(cap.x - w - pad, r.y + (r.h - ui.u(22.0)) * 0.5, w, ui.u(22.0));
        theme::status_badge(ui, br, "Restart needed", State { on: true, ..State::default() });
        lr.w = (br.x - lr.x - pad).max(0.0);
    }
    if mine {
        let d = ui.u(8.0);
        ui.draw.rect_round(R::new(r.x + pad * 0.5, r.y + r.h * 0.3 - d * 0.5 + ui.u(2.0), d, d), d * 0.5, t.accent);
    }
    let col = if off { t.text_dim } else { t.text };
    ui.draw.push_clip(lr);
    if others.is_empty() {
        ui.text_in(lr, label, t.text_size, Align::Left, col, Weight::Regular);
    } else {
        ui.text_in(R::new(lr.x, lr.y + ui.u(2.0), lr.w, lr.h * 0.55), label, t.text_size, Align::Left, col, Weight::Regular);
        ui.text_in(R::new(lr.x, lr.y + lr.h * 0.55, lr.w, lr.h * 0.4), &others_line(others), t.small_size, Align::Left, if off { t.text_dim } else { t.warn }, Weight::Regular);
    }
    ui.draw.pop_clip();
    let fs = State { off: fixed || off, hot: stt.hot, ..State::default() };
    theme::field(ui, cap, fs);
    ui.text_in(cap, key, t.text_size, Align::Centre, if fixed || off { t.text_off } else { t.text }, Weight::Bold);
    clicked
}

/// The box over the page while a key is awaited: the presses of this frame are read.
fn capture_box(ui: &mut Ui, page: R) {
    let t = ui.theme;
    let s = st();
    let Some(target) = s.capture else { return };
    let (what, now) = match target {
        Target::Entry(e) => {
            let en = &keys::map().unwrap().entries[e];
            (en.label.clone(), keys::binding(en).map(|b| b.label()).unwrap_or_default())
        }
        Target::Cell(c) => (sfmt!("Command bar: {}", cell_label(c)), String::from(key_label(keys::grid_key(c).unwrap_or("")))),
    };
    // A press binds; Esc is the modal's (`escape`).
    let presses: Vec<u16> = ui.input.presses().to_vec();
    for p in presses {
        let scan = crate::input::scan_of(p);
        if scan == 0x01 {
            continue;
        }
        let held = crate::input::mods_of(p);
        let Some(k) = inputmap::key_by_scan(scan) else {
            // A modifier alone waits for the key it goes with.
            if ![0x1d, 0x11d, 0x2a, 0x36, 0x38, 0x138].contains(&scan) {
                s.refused = String::from("That key is not one the game's key map names. Try another.");
            }
            continue;
        };
        match target {
            Target::Entry(e) => {
                let en = &keys::map().unwrap().entries[e];
                let b = Binding { key: String::from(k.name), mods: held.map(|h| if h { Mod::On } else { Mod::Off }) };
                keys::set(en, Some(&b));
            }
            Target::Cell(c) => {
                if held[0] || held[2] {
                    s.refused = String::from("The grid's keys take no Ctrl or Alt: press the key alone.");
                    continue;
                }
                if !keys::grid_can_take(k) {
                    s.refused = sfmt!("{} cannot press the grid: take a key of the main block, the F-keys or the keypad.", k.label);
                    continue;
                }
                keys::set_grid(c, Some(k.name));
            }
        }
        s.capture = None;
        s.refused.clear();
        return;
    }

    ui.draw.rect(page.inset(-ui.u(t.pad)), t.scrim);
    let r = crate::widgets::Anchor::Centre.place(page, ui.u(720.0).min(page.w), ui.u(280.0), 0.0);
    ui.panel(r);
    let p = ui.u(20.0);
    ui.text_in(R::new(r.x + p, r.y + p, r.w - 2.0 * p, ui.u(30.0)), &sfmt!("Press the new key for: {}", what), t.text_size, Align::Left, t.text, Weight::Bold);
    ui.text_in(R::new(r.x + p, r.y + p + ui.u(34.0), r.w - 2.0 * p, ui.u(26.0)), &sfmt!("Now: {}", now), t.small_size, Align::Left, t.text_dim, Weight::Regular);
    let help = match target {
        Target::Entry(_) => "Hold Ctrl, Shift or Alt with it for a combination. Esc cancels.",
        Target::Cell(_) => "One key, without Ctrl or Alt. Esc cancels.",
    };
    ui.text_in(R::new(r.x + p, r.y + p + ui.u(64.0), r.w - 2.0 * p, ui.u(26.0)), help, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    if !s.refused.is_empty() {
        ui.text_in(R::new(r.x + p, r.y + p + ui.u(96.0), r.w - 2.0 * p, ui.u(26.0)), &s.refused, t.small_size, Align::Left, t.warn, Weight::Regular);
    }
    let (bw, bh) = (ui.u(180.0), ui.u(44.0));
    let by = r.bottom() - p - bh;
    ui.scope("keys capture", |ui| {
        if ui.button(R::new(r.right() - p - 2.0 * bw - ui.u(12.0), by, bw, bh), "Stock key") {
            match target {
                Target::Entry(e) => {
                    let en = &keys::map().unwrap().entries[e];
                    keys::set(en, None);
                }
                Target::Cell(c) => {
                    keys::set_grid(c, None);
                }
            }
            s.capture = None;
        }
        if ui.button(R::new(r.right() - p - bw, by, bw, bh), "Cancel") {
            log!("refit: keys: capture cancelled");
            s.capture = None;
        }
    });
}

/// The help strip's text, when the pointer is over a row.
pub fn help() -> Option<String> {
    let s = st();
    (!s.help.is_empty()).then(|| s.help.clone())
}

/// Reset tab on the Keys tab: every choice of the player's for keys goes.
pub fn reset() -> usize {
    st().capture = None;
    keys::reset()
}
