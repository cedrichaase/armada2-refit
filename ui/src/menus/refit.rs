//! The Refit screen: every setting of every plugin that has a settings table, as rows on
//! five tabs, in a modal of ours over the hosted Options dialog (MENUS.md, "The Refit
//! screen"). The stock Options dialog keeps running under it; the modal owns the keys
//! (`menus::wants_keys`), so no Enter or Esc reaches it.
//!
//! The rows come from the settings hub (`settings::tables`); every change goes through
//! `settings::set_row` and nowhere else, so the hub is the only writer of
//! `RefitPlayer.ini` and tells the plugins. The words of the log lines and of the
//! badges are contracts the bench reads (`testbench/operating/refit-screen.md`).
use crate::engine::dialogs::Slot;
use crate::gfx::{Weight, R};
use crate::rt::settings::*;
use crate::rt::win::HWND;
use crate::settings::{self, Lock, Pending};
use crate::settings_table::MENUS_ENABLE;
use crate::widgets::theme::{self, State};
use crate::widgets::{Align, Ui};
use crate::{log, sfmt};
use alloc::string::String;
use alloc::vec::Vec;

/// The tabs, in the order they stand.
const TABS: [(u8, &str); 4] = [
    (TAB_GRAPHICS, "Graphics"),
    (TAB_INTERFACE, "Interface"),
    (TAB_GAMEPLAY, "Gameplay"),
    (TAB_MULTIPLAYER, "Multiplayer"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirm {
    None,
    Reset,
    MenusOff,
}

/// A note a plugin gave (`<Plugin>_SettingsNote`): on the plugin (`row` null) or a row.
struct Note {
    row: *const Setting,
    plugin: &'static str,
    text: String,
}

struct St {
    /// The Options dialog the modal is open over.
    open: Option<HWND>,
    /// The tab chosen last (kept for the session).
    tab: u8,
    advanced: bool,
    confirm: Confirm,
    ask: *const Setting,
    /// A dropdown's list was open when this frame began (its Esc is its own).
    dd_open: bool,
    /// The same, one frame earlier: the modal's body sees the pointer through an open
    /// list (`Ui::modal`), so while one is open every other widget is drawn inert.
    dd_prev: bool,
    hover: *const Setting,
    focus: *const Setting,
    done: bool,
    notes: Vec<Note>,
    notes_at: f64,
}

static mut S: St = St {
    open: None,
    tab: TAB_GRAPHICS,
    advanced: false,
    confirm: Confirm::None,
    ask: core::ptr::null(),
    dd_open: false,
    dd_prev: false,
    hover: core::ptr::null(),
    focus: core::ptr::null(),
    done: false,
    notes: Vec::new(),
    notes_at: 0.0,
};

fn st() -> &'static mut St {
    unsafe { &mut *core::ptr::addr_of_mut!(S) }
}

/// Whether a row has a place on the screen at all.
fn shown(r: &Setting) -> bool {
    r.flags & FLAG_NO_ROW == 0 && r.tab != TAB_NONE
}

/// Whether the screen has a row to show at all (the entry in Options needs one).
pub fn available() -> bool {
    settings::tables().iter().any(|t| t.rows.iter().any(|r| shown(r)))
}

pub fn is_open() -> bool {
    st().open.is_some()
}

/// Whether the modal is open over the dialog `h`.
pub fn is_host(h: HWND) -> bool {
    st().open == Some(h)
}

fn tab_name(id: u8) -> &'static str {
    TABS.iter().find(|t| t.0 == id).map_or("", |t| t.1)
}

fn tab_count(id: u8) -> usize {
    settings::tables().iter().map(|t| t.rows.iter().filter(|r| shown(r) && r.tab == id).count()).sum()
}

/// Opens the modal over the Options dialog `host`.
pub fn open(host: HWND) {
    let s = st();
    s.open = Some(host);
    s.confirm = Confirm::None;
    s.done = false;
    s.notes_at = 0.0;
    refresh_notes(0.0);
    let mut counts = String::new();
    for &(id, name) in TABS.iter() {
        let n = tab_count(id);
        if n > 0 {
            if !counts.is_empty() {
                counts.push_str(", ");
            }
            counts.push_str(&sfmt!("{} {}", name, n));
        }
    }
    if tab_count(s.tab) == 0 {
        s.tab = TABS.iter().map(|t| t.0).find(|&id| tab_count(id) > 0).unwrap_or(TAB_GRAPHICS);
    }
    log!("refit: opened in {} ({})", if crate::menus::backdrop::over_mission() { "a mission" } else { "the shell" }, counts);
    for n in s.notes.iter() {
        if n.row.is_null() {
            log!("refit: {}: {}", n.plugin, n.text);
        } else {
            let r = unsafe { &*n.row };
            log!("refit: [{}] {}: {}", crate::rt::overlay::section(r.file(), r.section()), r.key(), n.text);
        }
    }
}

fn refresh_notes(now: f64) {
    let s = st();
    if now != 0.0 && now < s.notes_at {
        return;
    }
    s.notes_at = now + 1.0;
    s.notes.clear();
    for t in settings::tables() {
        if let Some(text) = settings::note(t.plugin, None) {
            s.notes.push(Note { row: core::ptr::null(), plugin: t.plugin, text });
        }
        for r in t.rows.iter() {
            if let Some(text) = settings::note(t.plugin, Some(r)) {
                s.notes.push(Note { row: *r as *const Setting, plugin: t.plugin, text });
            }
        }
    }
}

fn row_note(r: &Setting) -> Option<String> {
    st().notes.iter().find(|n| core::ptr::eq(n.row, r)).map(|n| n.text.clone())
}

fn close(ui: Option<&mut Ui>, what: &str) {
    let s = st();
    if s.open.take().is_some() {
        s.confirm = Confirm::None;
        s.dd_open = false;
        s.dd_prev = false;
        log!("refit: {}", what);
    }
    if let Some(ui) = ui {
        ui.blur();
    }
}

/// A hosted dialog ended (`menus::closed`): the modal over it goes with it.
pub fn host_ended(h: HWND) {
    if is_host(h) {
        close(None, "closed (Options ended)");
    }
}

/// A frame of the screen, after the dialog stack: the modal when open over a dialog
/// that is still the innermost one.
pub fn frame(ui: &mut Ui, stack: &[&'static Slot]) {
    let Some(host) = st().open else { return };
    match stack.iter().position(|s| s.hwnd == host) {
        None => return close(Some(ui), "closed (Options ended)"),
        Some(i) if i + 1 != stack.len() => return close(Some(ui), "closed (a dialog opened over Options)"),
        _ => {}
    }
    refresh_notes(ui.time());
    let s = st();
    s.dd_prev = s.dd_open;
    s.dd_open = false;
    s.hover = core::ptr::null();
    s.focus = core::ptr::null();
    let sw = ui.screen();
    let (wu, hu) = (1200.0f32.min(sw.w / ui.scale - 48.0), 820.0f32.min(sw.h / ui.scale - 48.0));
    let esc = ui.modal("REFIT", wu, hu, |ui, inner| body(ui, inner));
    let s = st();
    if s.done {
        s.done = false;
        return close(Some(ui), "closed");
    }
    if esc {
        if s.dd_open {
            // The dropdown closes with this Esc.
        } else if s.confirm != Confirm::None {
            answer(ui, false);
        } else {
            close(Some(ui), "closed");
        }
    }
}

/// One thing on a page.
enum Item {
    Sep,
    Row(&'static Setting),
}

fn items(tab: u8, advanced: bool) -> Vec<Item> {
    let mut out = Vec::new();
    for t in settings::tables() {
        let rows: Vec<&'static Setting> = t.rows.iter().copied().filter(|r| shown(r) && r.tab == tab && (advanced || r.flags & FLAG_ADVANCED == 0)).collect();
        if rows.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(Item::Sep);
        }
        out.extend(rows.into_iter().map(Item::Row));
    }
    out
}

/// The height of a column of items, in units.
fn col_h(items: &[Item], rh: f32, sep: f32) -> f32 {
    items.iter().map(|i| if matches!(i, Item::Sep) { sep } else { rh }).sum()
}

/// Where the two columns divide (the first item of the right one) and the taller one's height.
fn split(items: &[Item], rh: f32, sep: f32) -> (usize, f32) {
    let mut best = (items.len(), f32::MAX);
    for k in 0..=items.len() {
        let m = col_h(&items[..k], rh, sep).max(col_h(&items[k..], rh, sep));
        if m < best.1 {
            best = (k, m);
        }
    }
    best
}

fn body(ui: &mut Ui, inner: R) {
    let t = ui.theme;
    let s = st();
    let gap = ui.u(8.0);
    let ids: Vec<u8> = TABS.iter().map(|t| t.0).filter(|&id| tab_count(id) > 0).collect();
    if ids.is_empty() {
        ui.note(inner, "No settings found.");
        return;
    }
    if !ids.contains(&s.tab) {
        s.tab = ids[0];
    }
    let live = s.confirm == Confirm::None;
    // The banner: RESTART rows whose value differs from the start.
    let mut pend = String::new();
    for tb in settings::tables() {
        for r in tb.rows.iter() {
            if shown(r) && settings::pending(r) == Pending::Restart {
                if !pend.is_empty() {
                    pend.push_str(", ");
                }
                pend.push_str(r.label());
            }
        }
    }
    let foot = R::new(inner.x, inner.bottom() - ui.u(44.0), inner.w, ui.u(44.0));
    let mut bottom = foot.y - gap;
    let banner = R::new(inner.x, bottom - ui.u(26.0), inner.w, ui.u(26.0));
    if !pend.is_empty() {
        bottom = banner.y - gap;
    }
    let help = R::new(inner.x, bottom - ui.u(52.0), inner.w, ui.u(52.0));
    bottom = help.y - gap;
    let tabs = R::new(inner.x, inner.y, inner.w, ui.u(44.0));
    let mut y = tabs.bottom() + gap;
    // Plugin-wide notes for the plugins with rows on this tab.
    let mut lines: Vec<String> = Vec::new();
    for tb in settings::tables() {
        if tb.rows.iter().any(|r| shown(r) && r.tab == s.tab) {
            if let Some(n) = s.notes.iter().find(|n| n.row.is_null() && n.plugin == tb.plugin) {
                lines.push(sfmt!("{}: {}", tb.plugin, n.text));
            }
        }
    }
    // Without the rules layer (no QOLRules_Settings table) a game with anyone else plays
    // stock rules, and the PayOnQueue row is not there to say why (contract C7).
    if s.tab == TAB_GAMEPLAY && !settings::tables().iter().any(|tb| tb.plugin == "QOLRules") {
        lines.push(String::from("Pay on queue is not installed: network games you join play stock rules"));
    }
    let note_rects: Vec<R> = lines
        .iter()
        .map(|_| {
            let r = R::new(inner.x, y, inner.w, ui.u(24.0));
            y += ui.u(24.0);
            r
        })
        .collect();
    let page = R::new(inner.x, y, inner.w, (bottom - y).max(0.0));

    ui.inert_unless(live, |ui| {
        let names: Vec<&str> = ids.iter().map(|&id| tab_name(id)).collect();
        let mut sel = ids.iter().position(|&id| id == s.tab).unwrap_or(0);
        ui.inert_unless(!s.dd_prev, |ui| ui.tabs(tabs, "refit tabs", &names, &mut sel));
        s.tab = ids[sel];
        for (r, l) in note_rects.iter().zip(lines.iter()) {
            ui.text_in(*r, l, t.small_size, Align::Left, t.warn, Weight::Regular);
        }
        page_rows(ui, page);
        help_strip(ui, help);
        if !pend.is_empty() {
            ui.text_in(banner, &sfmt!("Restart the game to apply: {}", pend), t.text_size, Align::Left, t.warn, Weight::Bold);
        }
        ui.inert_unless(!s.dd_prev, |ui| foot_row(ui, foot));
    });
    if s.confirm != Confirm::None {
        confirm_box(ui, inner);
    }
}

fn page_rows(ui: &mut Ui, page: R) {
    let s = st();
    let its = items(s.tab, s.advanced);
    let sep = 14.0;
    let avail = page.h / ui.scale;
    let one = col_h(&its, 54.0, sep) <= avail;
    let mut rh = 54.0;
    let mut fits = one;
    if !one {
        let mut h = 54.0;
        while h >= 40.0 {
            if split(&its, h, sep).1 <= avail {
                rh = h;
                fits = true;
                break;
            }
            h -= 2.0;
        }
    }
    if fits {
        draw_cols(ui, page, &its, rh, sep, one);
    } else {
        // Two columns of 46-unit rows do not fit either: scroll.
        let rh = 46.0;
        let (k, mh) = split(&its, rh, sep);
        let _ = k;
        ui.scroll_area(page, "refit page", ui.u(mh), |ui, cr| draw_cols(ui, cr, &its, rh, sep, false));
    }
}

fn draw_cols(ui: &mut Ui, area: R, its: &[Item], rh: f32, sep: f32, one: bool) {
    let gapx = ui.u(24.0);
    if one {
        draw_col(ui, area, its, rh, sep);
        return;
    }
    let (k, _) = split(its, rh, sep);
    let w = (area.w - gapx) * 0.5;
    draw_col(ui, R::new(area.x, area.y, w, area.h), &its[..k], rh, sep);
    // A separator that would start the right column is dropped.
    let right = if matches!(its.get(k), Some(Item::Sep)) { &its[k + 1..] } else { &its[k..] };
    draw_col(ui, R::new(area.x + w + gapx, area.y, w, area.h), right, rh, sep);
}

fn draw_col(ui: &mut Ui, col: R, its: &[Item], rh: f32, sep: f32) {
    let mut y = col.y;
    let mut n = 0;
    for it in its {
        match it {
            Item::Sep => {
                let r = R::new(col.x, y, col.w, ui.u(sep));
                theme::separator(ui, r);
                y += r.h;
            }
            Item::Row(row) => {
                let r = R::new(col.x, y, col.w, ui.u(rh));
                draw_row(ui, r, n, row);
                n += 1;
                y += r.h;
            }
        }
    }
}

fn values_of(row: &Setting) -> Vec<&'static str> {
    row.values().split('|').collect()
}

fn apply_word(row: &Setting) -> &'static str {
    match row.apply {
        APPLY_LIVE => "now",
        APPLY_MISSION => "next mission",
        _ => "restart",
    }
}

/// Writes a row through the hub, with the log line the bench reads.
fn apply_change(row: &Setting, old: &str, new: &str) {
    if core::ptr::eq(row, MENUS_ENABLE) && new.trim() == "0" {
        let s = st();
        s.confirm = Confirm::MenusOff;
        s.ask = row;
        return;
    }
    write_row(row, old, new);
}

fn write_row(row: &Setting, old: &str, new: &str) {
    let sec = crate::rt::overlay::section(row.file(), row.section());
    if settings::set_row(row, new) {
        log!("refit: [{}] {} {} -> {} ({})", sec, row.key(), old.trim(), new.trim(), apply_word(row));
    } else {
        log!("refit: [{}] {} could not be written", sec, row.key());
    }
}

fn log_locked(row: &Setting) {
    log!("refit: [{}] {} locked in a network game", crate::rt::overlay::section(row.file(), row.section()), row.key());
}

fn draw_row(ui: &mut Ui, rr: R, idx: usize, row: &'static Setting) {
    let key = sfmt!("{}.{}.{}", row.file(), row.section(), row.key());
    let own_open = {
        let id = ui.scope(&key, |ui| ui.id("dd"));
        ui.slot(id)[0] > 0.5
    };
    let live = !st().dd_prev || own_open;
    ui.inert_unless(live, |ui| ui.scope(&key, |ui| {
        let t = ui.theme;
        let s = st();
        let lock = settings::locked(row);
        let pend = settings::pending(row);
        let note = row_note(row);
        let cur = settings::value(row);
        let enum_row = row.kind == KIND_ENUM;
        let pad = ui.u(10.0);
        let over = ui.input.mouse.is_some_and(|p| rr.contains(p)) && ui.keys_free();
        let (cw, ch) = (ui.u(if enum_row { 260.0 } else { 120.0 }), (rr.h - ui.u(8.0)).min(ui.u(40.0)));
        let ctrl = R::new(rr.right() - pad - cw, rr.y + (rr.h - ch) * 0.5, cw, ch);
        let lbl = R::new(rr.x, rr.y, ctrl.x - rr.x, rr.h);

        // The control, and what a click does.
        let id = ui.id("row");
        let mut row_state = State { hot: over as u8 as f32, ..State::default() };
        if enum_row {
            let vals = values_of(row);
            let labs: Vec<&str> = row.value_labels().split('|').collect();
            let at = vals.iter().position(|v| v.trim().eq_ignore_ascii_case(cur.trim()));
            let mut names: Vec<String> = (0..vals.len()).map(|i| String::from(labs.get(i).copied().unwrap_or(vals[i]))).collect();
            if at.is_none() {
                names.push(sfmt!("Custom ({})", cur.trim()));
            }
            let refs: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
            let mut sel = at.unwrap_or(vals.len());
            let dd = ui.id("dd");
            if ui.slot(dd)[0] > 0.5 {
                s.dd_open = true;
            }
            theme::row(ui, rr, idx, row_state);
            let (_, _, clicked) = ui.interact(ui.id("lbl"), lbl);
            if lock != Lock::No {
                if clicked {
                    log_locked(row);
                }
                let fs = State { off: true, ..State::default() };
                theme::field(ui, ctrl, fs);
                let name = refs.get(sel).copied().unwrap_or("");
                ui.text_in(R::new(ctrl.x + pad, ctrl.y, ctrl.w - 2.0 * pad, ctrl.h), name, t.text_size, Align::Left, t.text_off, Weight::Regular);
            } else {
                if clicked {
                    let sl = ui.slot(dd);
                    sl[0] = 1.0;
                    sl[2] = sel as f32;
                    ui.focus(dd);
                }
                let changed = ui.dropdown(ctrl, "dd", &refs, &mut sel);
                if changed && sel < vals.len() {
                    apply_change(row, &cur, vals[sel]);
                }
                if ui.has_focus(dd) {
                    s.focus = row;
                }
            }
        } else {
            let on = crate::rt::parse_i32(&cur).unwrap_or(0) != 0;
            let focused = ui.focus_register(id);
            let (hot, held, clicked) = ui.interact(id, rr);
            row_state = State::of(ui, id, hot, held);
            row_state.focus = focused;
            theme::row(ui, rr, idx, row_state);
            let flip = clicked || (focused && ui.activate_pressed());
            if flip {
                if lock != Lock::No {
                    log_locked(row);
                } else {
                    apply_change(row, &cur, if on { "0" } else { "1" });
                }
            }
            let cs = State { off: lock != Lock::No, hot: row_state.hot, ..State::default() };
            let w = theme::check(ui, ctrl, on, cs);
            let col = if lock != Lock::No { t.text_off } else { t.text };
            ui.text_in(R::new(ctrl.x + w + ui.u(10.0), ctrl.y, ctrl.w - w, ctrl.h), if on { "On" } else { "Off" }, t.text_size, Align::Left, col, Weight::Regular);
            if focused {
                s.focus = row;
            }
        }
        if over {
            s.hover = row;
        }

        // The changed dot, the label, the badges.
        let dot = ui.u(8.0);
        if settings::overlay_holds(row) {
            ui.draw.rect_round(R::new(rr.x + pad * 0.5, rr.y + rr.h * 0.3 - dot * 0.5 + ui.u(2.0), dot, dot), dot * 0.5, t.accent);
        }
        let x0 = rr.x + pad + ui.u(10.0);
        let lw = (lbl.right() - ui.u(12.0) - x0).max(0.0);
        let dim = note.is_some() || lock != Lock::No;
        ui.text_in(R::new(x0, rr.y + ui.u(2.0), lw, rr.h * 0.55), row.label(), t.text_size, Align::Left, if dim { t.text_dim } else { t.text }, Weight::Regular);
        let bh = (rr.h * 0.38).min(ui.u(22.0));
        let by = rr.y + rr.h * 0.58;
        let mut x = x0;
        let apply: (&str, bool) = match pend {
            Pending::Restart => ("Restart needed", true),
            Pending::Mission => ("Next mission", true),
            Pending::No => (
                match row.apply {
                    APPLY_LIVE => "Now",
                    APPLY_MISSION => "Next mission",
                    _ => "Restart",
                },
                false,
            ),
        };
        let mut words: Vec<(&str, State)> = Vec::new();
        words.push((apply.0, State { on: apply.1, ..State::default() }));
        if row.scope == SCOPE_MATCH {
            words.push(("Must match", State::default()));
        }
        if lock != Lock::No {
            words.push(("Locked", State { off: true, ..State::default() }));
        }
        for (w, bs) in words {
            let bw = theme::status_badge_w(ui, w);
            if x + bw > lbl.right() {
                break;
            }
            theme::status_badge(ui, R::new(x, by, bw, bh), w, bs);
            x += bw + ui.u(6.0);
        }
    }));
}

fn help_strip(ui: &mut Ui, r: R) {
    let t = ui.theme;
    let s = st();
    let row = if !s.hover.is_null() { s.hover } else { s.focus };
    ui.draw.rect(r, t.row_alt);
    let text;
    let mut dim = false;
    if row.is_null() {
        text = String::from("Point at a setting to see what it does. Choices are written to RefitPlayer.ini.");
        dim = true;
    } else {
        let row = unsafe { &*row };
        let mut x = String::new();
        match row_note(row) {
            Some(n) => {
                x.push_str(&n);
                dim = true;
            }
            None => x.push_str(row.help()),
        }
        x.push(' ');
        x.push_str(match row.apply {
            APPLY_LIVE => "Applies at once.",
            APPLY_MISSION => "Applies at the next mission.",
            _ => "Applies when the game is started again.",
        });
        if row.scope == SCOPE_MATCH {
            x.push_str(" Every player in a network game must have the same.");
        }
        match settings::locked(row) {
            Lock::Network => x.push_str(" Locked in a network game: every player must have the same."),
            Lock::Unknown => x.push_str(" Locked: the game's mode could not be read."),
            Lock::No => {}
        }
        text = x;
    }
    let p = ui.u(10.0);
    ui.draw.push_clip(r);
    ui.draw.text_wrapped(&text, ui.u(t.small_size), r.w - 2.0 * p, r.x + p, r.y + ui.u(5.0), if dim { t.text_dim } else { t.text }, Weight::Regular);
    ui.draw.pop_clip();
}

fn foot_row(ui: &mut Ui, f: R) {
    let t = ui.theme;
    let s = st();
    let (bw, gap) = (ui.u(200.0), ui.u(16.0));
    if ui.button(R::new(f.x, f.y, bw, f.h), "Reset tab") {
        s.confirm = Confirm::Reset;
    }
    let tg = R::new(f.x + bw + gap, f.y, ui.u(260.0), f.h);
    let mut adv = s.advanced;
    if ui.toggle(tg, "Show advanced", &mut adv) {
        s.advanced = adv;
    }
    let done = R::new(f.right() - bw, f.y, bw, f.h);
    if s.tab == TAB_GRAPHICS {
        let name = crate::gfx::gpu().backend_name();
        let r = R::new(tg.right() + gap, f.y, done.x - gap - tg.right() - gap, f.h);
        ui.text_in(r, &sfmt!("Drawn on {}", name), t.small_size, Align::Right, t.text_dim, Weight::Regular);
    }
    if ui.button(done, "Done") {
        s.done = true;
    }
}

/// The question over the page: Reset tab, or switching the redrawn menus off.
fn confirm_box(ui: &mut Ui, inner: R) {
    let t = ui.theme;
    let s = st();
    ui.draw.rect(inner.inset(-ui.u(t.pad)), t.scrim);
    let r = crate::widgets::Anchor::Centre.place(inner, ui.u(700.0), ui.u(240.0), 0.0);
    ui.panel(r);
    let p = ui.u(20.0);
    let text = match s.confirm {
        Confirm::Reset => String::from("Remove your choices on this tab from RefitPlayer.ini? The ini files' settings apply again."),
        _ => String::from("The Refit screen is part of the redrawn menus. To get them back, delete the line Enable=0 under [UI.Menus] in RefitPlayer.ini in the game folder. Switch them off?"),
    };
    ui.draw.text_wrapped(&text, ui.u(t.text_size), r.w - 2.0 * p, r.x + p, r.y + p, t.text, Weight::Regular);
    let (bw, bh) = (ui.u(160.0), ui.u(44.0));
    let by = r.bottom() - p - bh;
    ui.scope("confirm", |ui| {
        if ui.button(R::new(r.right() - p - 2.0 * bw - ui.u(12.0), by, bw, bh), "Yes") {
            answer(ui, true);
        }
        if ui.button(R::new(r.right() - p - bw, by, bw, bh), "No") {
            answer(ui, false);
        }
    });
}

fn answer(ui: &mut Ui, yes: bool) {
    let s = st();
    let what = s.confirm;
    s.confirm = Confirm::None;
    ui.blur();
    match what {
        Confirm::Reset => {
            if !yes {
                return;
            }
            // Reset covers the tab's advanced rows too, shown or not.
            let its = items(s.tab, true);
            let keys: Vec<(&str, &str, &str)> = its
                .iter()
                .filter_map(|i| match i {
                    Item::Row(r) => Some((r.file(), r.section(), r.key())),
                    Item::Sep => None,
                })
                .collect();
            let n = settings::remove_many(&keys);
            log!("refit: {} reset ({} keys removed)", tab_name(s.tab), n);
        }
        Confirm::MenusOff => {
            log!("refit: menus off asked; {}", if yes { "yes" } else { "no" });
            if yes && !s.ask.is_null() {
                let row = unsafe { &*s.ask };
                write_row(row, &settings::value(row), "0");
            }
            s.ask = core::ptr::null();
        }
        Confirm::None => {}
    }
}
