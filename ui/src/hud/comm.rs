//! The multiplayer HUD boxes, ours (HUD.md, "Comms, the drop box and the load wait"):
//!
//! - **`[HUD] Comm=1`**: the comm panel (`CommDisplay`), while the stock one shows (the
//!   top-right comm button opens it). A window over a dimmed screen: the players, each
//!   with their colour, team, chat switch and, when the setup lets alliances change,
//!   enemy, neutral and ally; the message buttons (who the next chat line goes to);
//!   giving the selected units or an amount of a resource to the selected player;
//!   allied victory and high latency; OK. Every control presses the stock panel's own
//!   button, so the orders and chat targets are the game's.
//! - **`[HUD] DropPlayer=1`**: the box that shows while the game waits on a player who
//!   stopped answering (`DropPlayerBox`): its text, and the votes to drop the player or
//!   let the computer take over.
//! - **`[HUD] LoadWait=1`**: the box that shows when this player's load is done and the
//!   others' are not (`LoadPercentageBox`): the wait and each player's progress. The
//!   loading screen of ours (`LOADING.md`) shows the same progress while this player
//!   is still loading; this is the time after it.
//!
//! Like the stock boxes (whose `CursorOver` is always true) each keeps the mouse from
//! everything under it while it shows; the keyboard stays the engine's, so the chat
//! line still opens on Return over the comm panel.
use crate::config::Hud;
use crate::engine::{self, buttons, comm, hud, setup, sound};
use crate::gfx::{Weight, R};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

static mut ON: (bool, bool, bool) = (false, false, false);

pub fn any(c: &Hud) -> bool {
    c.comm || c.drop_player || c.load_wait
}

pub fn install(c: &Hud) {
    if !any(c) {
        return;
    }
    if !comm::functions_stock() {
        crate::log!("comm: an engine function is not stock: the multiplayer boxes stay stock");
        return;
    }
    engine::buttons::install_presses();
    let take = |flag: bool, at: usize, name: &str| {
        if flag {
            hud::set_taken(at, false, true);
            crate::log!("comm: {} ours", name);
        }
        flag
    };
    let on = (take(c.comm, comm::COMM, "CommDisplay"), take(c.drop_player, comm::DROP, "DropPlayerBox"), take(c.load_wait, comm::LOAD, "LoadPercentageBox"));
    unsafe { ON = on };
}

/// Whether one of these boxes shows (and so covers the screen).
pub fn blocking() -> bool {
    let on = unsafe { ON };
    (on.0 && comm::comm_shown()) || (on.1 && comm::drop_shown()) || (on.2 && comm::load_shown())
}

static mut LOGGED: [bool; 3] = [false; 3];

fn log_once(i: usize, shown: bool, f: impl FnOnce()) {
    unsafe {
        if shown && !LOGGED[i] {
            f();
        }
        LOGGED[i] = shown;
    }
}

pub fn run(ui: &mut Ui) {
    let on = unsafe { ON };
    if on.0 {
        let c = if comm::comm_shown() { comm::comm() } else { None };
        log_once(0, c.is_some(), || {
            let c = c.as_ref().unwrap();
            let mut s = String::new();
            for b in &c.bars {
                s.push_str(&crate::sfmt!(" [{} \"{}\" team {:?} chat {}{} rel {}{}{}{}]", b.slot, b.name, b.team, if b.chat != 0 { "" } else { "-" }, b.chat_on,
                    b.alliance[0].1 as u8, b.alliance[1].1 as u8, b.alliance[2].1 as u8, if b.selected { " selected" } else { "" }));
            }
            crate::log!("comm: shown \"{}\", teams {} alliances {}, {} bars:{}; {} resources; msg {:?}; give units {}, okay {}, victory {:?}, latency {:?}",
                c.title, c.teams, c.alliances, c.bars.len(), s, c.rows.len(), c.msg.map(|b| b != 0), c.give_units != 0, c.okay != 0, c.victory, c.latency);
        });
        if let Some(c) = c {
            ui.overlay(move |ui| draw_comm(ui, c));
        }
    }
    if on.1 {
        let d = comm::drop_box();
        log_once(1, d.is_some(), || {
            let d = d.as_ref().unwrap();
            crate::log!("drop: shown \"{}\"; drop {} ({}), AI {} ({})", d.text.replace('\n', " / "), buttons::label(d.drop), buttons::enabled(d.drop), buttons::label(d.ai), buttons::enabled(d.ai));
        });
        if let Some(d) = d {
            ui.overlay(move |ui| draw_drop(ui, d));
        }
    }
    if on.2 {
        let l = comm::load_wait();
        log_once(2, l.is_some(), || crate::log!("load wait: shown \"{}\"", l.as_ref().unwrap().1.replace('\n', " / ")));
        if let Some((secs, text)) = l {
            ui.overlay(move |ui| draw_load(ui, secs, text));
        }
    }
}

// ---- the comm panel -------------------------------------------------------------------

/// The word for a stock button: its label, the first line of its tooltip, or `fallback`.
fn word(b: usize, fallback: &str) -> String {
    let l = buttons::label(b);
    if !l.is_empty() {
        return l;
    }
    buttons::tooltip(b, false).and_then(|t| t.lines().next().map(String::from)).filter(|s| !s.is_empty()).unwrap_or_else(|| String::from(fallback))
}

/// A check box of ours for a stock toggle `b`, `on` its state, with `label` after it.
/// A click presses the toggle (its own press flips it and runs its callback).
fn toggle(ui: &mut Ui, r: R, b: usize, on: bool, label: &str, key: &str) {
    let id = ui.id(key);
    let live = b != 0 && buttons::enabled(b);
    let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !live;
    let w = theme::check(ui, r, on, st);
    if !label.is_empty() {
        let t = ui.theme;
        let lr = R::new(r.x + w + ui.u(t.gap), r.y, (r.w - w - ui.u(t.gap)).max(0.0), r.h);
        ui.text_in(lr, label, t.text_size, Align::Left, if live { t.text } else { t.text_off }, Weight::Regular);
    }
    if clicked && live {
        crate::log!("comm: toggle {} ({})", key, if on { "off" } else { "on" });
        buttons::queue(b);
    }
}

/// A small button of ours that presses the stock button `b`: true when clicked.
fn press(ui: &mut Ui, r: R, b: usize, label: &str, key: &str, on: bool) -> bool {
    let id = ui.id(key);
    let live = b != 0 && buttons::enabled(b);
    let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !live;
    st.on = on;
    theme::button(ui, r, label, st);
    if hot && live {
        if let Some(tip) = buttons::tooltip(b, false) {
            if !tip.is_empty() && tip != label {
                ui.tooltip(r, &tip);
            }
        }
    }
    if clicked && live {
        crate::log!("comm: press {}", key);
        buttons::queue(b);
        return true;
    }
    false
}

fn draw_comm(ui: &mut Ui, c: comm::Comm) {
    let t = ui.theme;
    let title = if c.title.is_empty() { String::from("Comm") } else { c.title.clone() };
    let row_h = ui.u(t.row_h + 6.0);
    let bh = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let head_h = ui.u(t.heading_size) * 1.6;
    // The height: the players' table or the right column, whichever is taller, and
    // the bottom row.
    let left_h = head_h + c.bars.len().max(1) as f32 * row_h;
    let right_h = head_h + 4.0 * (bh + gap) + head_h + (bh + gap) + c.rows.len() as f32 * (row_h + gap);
    let content = left_h.max(right_h) + ui.u(t.pad * 2.0) + bh;
    let h_units = (content / ui.scale + t.title_size + 6.0 * t.pad + 40.0).min(1000.0);
    let inner = super::panels::boxed(ui, &title, 1240.0, h_units, 1.0);
    let pad = ui.u(t.pad);
    let right_w = ui.u(380.0).min(inner.w * 0.4);
    let left = R::new(inner.x, inner.y, inner.w - right_w - pad * 2.0, inner.h - bh - pad * 2.0);
    let right = R::new(left.right() + pad * 2.0, inner.y, right_w, left.h);

    // ---- the players
    let swatch = ui.u(t.row_h * 0.7);
    let chat_w = ui.u(90.0);
    let team_w = if c.teams { ui.u(140.0) } else { 0.0 };
    let all_w = if c.alliances { ui.u(330.0).min(left.w * 0.45) } else { 0.0 };
    let name_x = left.x + swatch + gap * 2.0;
    let all_x = left.right() - all_w;
    let chat_x = all_x - chat_w - if c.alliances { gap * 2.0 } else { 0.0 };
    let team_x = chat_x - team_w - gap;
    let name_w = (team_x - name_x - gap).max(ui.u(80.0));
    let hy = left.y;
    let small = |ui: &mut Ui, x: f32, w: f32, s: &str, al: Align| {
        let t = ui.theme;
        ui.text_in(R::new(x, hy, w, head_h), s, t.small_size, al, t.text_dim, Weight::Bold);
    };
    small(ui, name_x, name_w, &theme::caps(&c.head_player), Align::Left);
    if c.teams {
        small(ui, team_x, team_w, &theme::caps(&c.head_team), Align::Left);
    }
    small(ui, chat_x, chat_w, &theme::caps(&c.head_message), Align::Left);
    if c.alliances {
        small(ui, all_x, all_w, &theme::caps(&c.head_alliance), Align::Left);
    }
    let mut y = hy + head_h;
    for (i, b) in c.bars.iter().enumerate() {
        let rr = R::new(left.x, y, left.w, row_h - ui.u(2.0));
        y += row_h;
        // The row selects the player (the colour button's press, as clicking the stock
        // swatch or name does); the chat and alliance controls on it act on their own.
        let rid = ui.id(&crate::sfmt!("comm.row{}", b.slot));
        let sel_r = R::new(rr.x, rr.y, team_x - rr.x + team_w, rr.h);
        let live = b.select != 0 && buttons::enabled(b.select);
        let (hot, held, clicked) = if live { ui.interact(rid, sel_r) } else { (false, false, false) };
        let mut st = theme::State::of(ui, rid, hot, held);
        st.on = b.selected;
        theme::row(ui, rr, i, st);
        let sw = R::new(rr.x + gap * 0.5, rr.y + (rr.h - swatch) * 0.5, swatch, swatch);
        ui.draw.rect(sw.inset(-ui.u(1.5)), t.panel_edge);
        ui.draw.rect(sw, b.colour);
        let tc = theme::row_text(ui, st);
        ui.text_in(R::new(name_x, rr.y, name_w, rr.h), &b.name, t.text_size, Align::Left, tc, Weight::Regular);
        if let Some(team) = b.team {
            let s = crate::sfmt!("{} {}", buttons::localize(comm::TEAM_PREFIX), team);
            ui.text_in(R::new(team_x, rr.y, team_w, rr.h), &s, t.text_size, Align::Left, t.text_dim, Weight::Regular);
        }
        if clicked && live {
            crate::log!("comm: select {} \"{}\"", b.slot, b.name);
            sound::click();
            buttons::queue(b.select);
        }
        if b.chat != 0 {
            toggle(ui, R::new(chat_x, rr.y, chat_w, rr.h), b.chat, b.chat_on, "", &crate::sfmt!("chat{}", b.slot));
        }
        if c.alliances {
            const WORDS: [&str; 3] = ["Enemy", "Neutral", "Ally"];
            let cw = (all_w - 2.0 * gap) / 3.0;
            for k in 0..3 {
                let (ab, active, pending) = b.alliance[k];
                let br = R::new(all_x + k as f32 * (cw + gap), rr.y + ui.u(2.0), cw, rr.h - ui.u(4.0));
                let label = if ab != 0 { word(ab, WORDS[k]) } else { String::from(WORDS[k]) };
                press(ui, br, ab, &label, &crate::sfmt!("rel{}.{}", b.slot, k), active);
                if pending {
                    // Proposed, not answered yet: a dashed hint in the warning colour.
                    ui.draw.frame(br.inset(-ui.u(2.0)), ui.u(1.5).max(1.0), t.warn);
                }
            }
        }
    }
    if c.bars.is_empty() {
        let t = ui.theme;
        ui.text_in(R::new(name_x, y, name_w, row_h), "-", t.text_size, Align::Left, t.text_dim, Weight::Regular);
    }

    // ---- messages and giving
    theme::separator(ui, R::new(right.x - pad, right.y, ui.u(2.0), right.h));
    let mut y = right.y;
    theme::heading(ui, R::new(right.x, y, right.w, head_h), &c.group_message);
    y += head_h;
    const MSG_WORDS: [&str; 4] = ["Message all", "Message allies", "Message enemies", "Message player"];
    for (k, &b) in c.msg.iter().enumerate() {
        press(ui, R::new(right.x, y, right.w, bh), b, &word(b, MSG_WORDS[k]), &crate::sfmt!("msg{}", k), false);
        y += bh + gap;
    }
    theme::heading(ui, R::new(right.x, y, right.w, head_h), &c.group_give);
    y += head_h;
    press(ui, R::new(right.x, y, right.w, bh), c.give_units, &word(c.give_units, "Give units"), "give.units", false);
    y += bh + gap;
    for (k, r) in c.rows.iter().enumerate() {
        let rr = R::new(right.x, y, right.w, row_h);
        y += row_h + gap;
        let step = ui.u(t.row_h);
        let give_w = ui.u(90.0);
        let name_w = rr.w - give_w - 2.0 * step - ui.u(70.0) - 3.0 * gap;
        ui.text_in(R::new(rr.x, rr.y, name_w, rr.h), &r.name, t.text_size, Align::Left, t.text, Weight::Regular);
        let mut x = rr.x + name_w + gap;
        let sb = |x: f32| R::new(x, rr.y + (rr.h - step) * 0.5, step, step);
        press(ui, sb(x), r.down, "-", &crate::sfmt!("res{}.down", k), false);
        x += step;
        let ar = R::new(x, rr.y, ui.u(70.0), rr.h);
        let col = if r.amount <= r.min { t.text_dim } else if r.amount >= r.max { t.warn } else { t.text };
        ui.text_in(ar, &crate::sfmt!("{}", r.amount), t.text_size, Align::Centre, col, Weight::Bold);
        x += ar.w;
        press(ui, sb(x), r.up, "+", &crate::sfmt!("res{}.up", k), false);
        x += step + gap * 2.0;
        // The stock give buttons are pictures; their tooltip names the resource again,
        // which the row already does.
        let gl = buttons::label(r.give);
        let gl = if gl.is_empty() { String::from("Give") } else { gl };
        press(ui, R::new(x, rr.y + ui.u(2.0), rr.right() - x, rr.h - ui.u(4.0)), r.give, &gl, &crate::sfmt!("res{}.give", k), false);
    }

    // ---- the bottom row: the game's switches, OK
    let by = inner.bottom() - bh;
    let mut x = inner.x;
    for (k, (b, on, label)) in [(c.victory.0, c.victory.1, &c.label_victory), (c.latency.0, c.latency.1, &c.label_latency)].iter().enumerate() {
        if *b == 0 {
            continue;
        }
        let w = ui.u(300.0);
        let fallback = if k == 0 { "Allied victory" } else { "High latency" };
        let l = if label.is_empty() { fallback } else { label.as_str() };
        toggle(ui, R::new(x, by, w, bh), *b, *on, l, if k == 0 { "victory" } else { "latency" });
        x += w + gap * 2.0;
    }
    let ow = ui.u(180.0);
    if c.okay != 0 {
        super::panels::game_text_button(ui, R::new(inner.right() - ow, by, ow, bh), c.okay, "OK", &[]);
    }
}

// ---- the dropped player ---------------------------------------------------------------

fn draw_drop(ui: &mut Ui, d: comm::Drop) {
    let t = ui.theme;
    let mut lines: Vec<&str> = d.text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let title = if lines.is_empty() { String::from(" ") } else { String::from(lines.remove(0)) };
    let body: String = lines.join("\n");
    let w = 760.0;
    let px = ui.u(t.text_size);
    let pad = ui.u(t.pad);
    let (_, th) = crate::text::measure_wrapped(crate::text::font(), &body, px, ui.u(w) - 4.0 * pad);
    let bh = ui.u(t.button_h);
    let h = ((th + bh) / ui.scale + t.title_size + 6.0 * t.pad + 40.0).min(1000.0);
    let inner = super::panels::boxed(ui, &title, w, h, 1.0);
    ui.draw.text_wrapped(&body, px, inner.w - pad, inner.x + pad * 0.5, inner.y + pad * 0.5, t.text, Weight::Regular);
    let bw = ui.u(260.0);
    let gap = ui.u(t.gap);
    let mut x = inner.right() - bw;
    for (b, fallback) in [(d.ai, "AI takeover"), (d.drop, "Drop player")] {
        if b != 0 {
            super::panels::game_text_button(ui, R::new(x, inner.bottom() - bh, bw, bh), b, fallback, &[]);
            x -= bw + gap;
        }
    }
}

// ---- the wait for the other players ---------------------------------------------------

fn draw_load(ui: &mut Ui, secs: f32, text: String) {
    let t = ui.theme;
    let players = setup::players();
    let fr = setup::load_fractions();
    let title = buttons::localize(comm::LOAD_WAIT);
    let title = if title.is_empty() || title.starts_with("GUI_") { String::from("Waiting for players") } else { title };
    let row_h = ui.u(t.row_h + 4.0);
    let rows: Vec<(String, Option<f32>, bool)> = match fr {
        Some(f) => players.iter().filter_map(|p| f.get(p.slot).copied().flatten().map(|v| (p.name.clone(), Some(v), p.local))).collect(),
        None => text.lines().skip(1).map(|l| l.trim()).filter(|l| !l.is_empty()).map(|l| (String::from(l), None, false)).collect(),
    };
    let content = row_h * (rows.len() as f32 + 1.0);
    let h = (content / ui.scale + t.title_size + 6.0 * t.pad + 40.0).min(800.0);
    let inner = super::panels::boxed(ui, &title, 640.0, h, 0.6);
    let pad = ui.u(t.pad);
    ui.text_in(R::new(inner.x, inner.y, inner.w, row_h), &crate::sfmt!("{} s", secs as i32), t.text_size, Align::Right, t.text_dim, Weight::Regular);
    let mut y = inner.y + row_h;
    for (name, f, local) in rows {
        let nw = inner.w * 0.45;
        ui.text_in(R::new(inner.x, y, nw, row_h), &name, t.text_size, Align::Left, if local { t.title } else { t.text }, Weight::Regular);
        if let Some(f) = f {
            let br = R::new(inner.x + nw + pad, y + row_h * 0.3, inner.w - nw - pad * 2.0 - ui.u(60.0), row_h * 0.4);
            theme::bar(ui, br, f, None);
            ui.text_in(R::new(br.right() + pad, y, ui.u(60.0), row_h), &crate::sfmt!("{}%", (f * 100.0) as i32), t.small_size, Align::Right, t.text_dim, Weight::Regular);
        }
        y += row_h;
    }
}
