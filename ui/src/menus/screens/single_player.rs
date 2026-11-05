//! The campaign selection (`do_single`), laid out our way: four cards, each a cut of the
//! stock screen's art (the faction's picture, which is information a theme cannot
//! draw) under its name, and the two buttons at the foot.
//!
//! In the stock screen the Tutorial and the Federation panel are hit areas of the
//! procedure itself, with Bink hover animations; the Klingon and Borg panels are
//! picture `ShellButton`s. None has a label: the words appear in the middle of the screen
//! on hover, from four hidden `TextLabel`s, which are where the names come from. A click
//! is a click on a point inside the stock panel (the same points `testbench/ui.json`
//! names), and a hover is a `WM_MOUSEMOVE` to the point, so the stock screen plays the
//! panel's animation and sound, which the cut of the art then shows.
//!
//! Over render3d's map (`menus::scene`) the screen is a column like the main menu's:
//! each campaign its faction's emblem (the HUD's race icons, `allrace` in
//! `gui_global.spr`; for the tutorials Starfleet Academy's patch, drawn) and its name,
//! and pointing at one moves the camera to its scene (`scene::choose`).
//!
//! A campaign not yet open (`menus::locks`) is greyed, with a padlock after its name, and
//! pointing at it says what opens it ("Finish the Federation Campaign to unlock"); its
//! scene still shows, locked (render3d). A click still goes to the stock panel, which
//! has the last word (on a locked campaign it does nothing).
use crate::gfx::{fade, Tex, R};
use crate::menus::screens::kit;
use crate::menus::{actions, Screen};
use crate::rt::win::{HWND, WM_MOUSEMOVE};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;

/// What opens card `i`, while it is locked: "Finish the <name> to unlock".
fn lock_hint(i: usize, names: &[&str; 4]) -> Option<String> {
    let req = crate::menus::locks::locked(i)?;
    Some(crate::sfmt!("Finish the {} to unlock", names.get(req).copied().unwrap_or("")))
}

/// A padlock in `r` (its height sets its size): a rounded body and its shackle.
fn padlock(ui: &mut Ui, r: R, c: crate::gfx::Color) {
    let h = r.h;
    let bw = h * 0.78;
    let x = r.x;
    let body = R::new(x, r.y + h * 0.42, bw, h * 0.58);
    ui.draw.rect_round(body, h * 0.08, c);
    let lw = (h * 0.12).max(1.5);
    let (cx, sw) = (x + bw * 0.5, bw * 0.30);
    let (top, base) = (r.y + h * 0.04 + lw * 0.5, r.y + h * 0.44);
    let mid = top + sw;
    // Two legs and a half circle of six segments over them.
    ui.draw.line((cx - sw, base), (cx - sw, mid), lw, c);
    ui.draw.line((cx + sw, base), (cx + sw, mid), lw, c);
    let mut last = (cx - sw, mid);
    for k in 1..=6 {
        let a = core::f32::consts::PI * (1.0 - k as f32 / 6.0);
        let p = (cx + sw * libm::cosf(a), mid - sw * libm::sinf(a));
        ui.draw.line(last, p, lw, c);
        last = p;
    }
}

/// A card: the stock panel's art (design rectangle) and a point inside the stock panel
/// to click. Its name, key, view and emblem are the content set's (`content.rs`,
/// `campaigns`, in the same order).
struct Card {
    art: [i32; 4],
    at: (i32, i32),
}

/// The race icons, loaded once (None inside: not found, the names stand alone).
static mut ICONS: Option<Option<Tex>> = None;

fn icons() -> Option<Tex> {
    unsafe {
        if ICONS.is_none() {
            let t = crate::gfx::load_tga("Textures\\RGB\\allrace.tga");
            crate::log!("menus: campaign emblems {}", if t.is_some() { "from allrace.tga" } else { "not found (allrace.tga): names only" });
            ICONS = Some(t);
        }
        ICONS.flatten()
    }
}

const CARDS: [Card; 4] = [
    Card { art: [24, 16, 330, 205], at: (186, 111) },
    Card { art: [436, 22, 310, 216], at: (589, 106) },
    Card { art: [24, 338, 330, 222], at: (186, 444) },
    Card { art: [431, 340, 322, 222], at: (589, 439) },
];

/// Each card's name, from the stock screen's hidden labels, matched by the word that
/// names the campaign (`KEYS`), never by the labels' order: the dialog's controls do
/// not always come in the same order (after an aborted mission the rows once read in
/// the wrong order). A card no label names keeps its own name; when no label names any
/// card (another language), the labels are taken in order, as before.
/// The labels were seen out of order (logged once).
static mut SAID_ORDER: bool = false;

fn card_names<'a>(labels: &[&'a crate::menus::model::Control]) -> [&'a str; 4] {
    let camps = crate::content::get().campaigns;
    let mut out = [camps[0].name, camps[1].name, camps[2].name, camps[3].name];
    let mut any = false;
    for (i, key) in camps.iter().map(|c| c.key).enumerate() {
        let hit = labels.iter().position(|c| crate::rt::lower(&c.text).contains(key));
        if let Some(j) = hit {
            out[i] = labels[j].text.as_str();
            any = true;
            if j != i && unsafe { !SAID_ORDER } {
                unsafe { SAID_ORDER = true };
                crate::log!("menus: campaign labels out of order (\"{}\" is label {}); matched by name", labels[j].text, j);
            }
        }
    }
    if !any {
        for (i, c) in labels.iter().take(4).enumerate() {
            if !c.text.is_empty() {
                out[i] = c.text.as_str();
            }
        }
    }
    out
}

/// The card under the pointer last frame, to tell the stock screen when it changes.
static mut HOVER: (HWND, i32) = (core::ptr::null_mut(), -1);

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let names = kit::labels(s);
    // A set without the four campaigns leaves the screen to the engine's own.
    if names.len() < 4 || crate::content::get().campaigns.len() != CARDS.len() {
        return false;
    }
    let buttons = kit::buttons(s);
    // Previous menu and Load Game: the two with text, at the foot.
    let mut foot: alloc::vec::Vec<_> = buttons.iter().copied().filter(|c| !c.text.is_empty() && c.rect[1] > 500).collect();
    foot.sort_by_key(|c| c.rect[0]);
    if foot.len() < 2 {
        return false;
    }
    if crate::menus::scene::shown() {
        return over_scene(ui, s, &card_names(&names), &foot);
    }
    let t = ui.theme;
    let sc = ui.screen();
    let names = card_names(&names);
    kit::backdrop(ui);

    // The screen: a title, the cards (2x2), the foot.
    let m = ui.u(40.0);
    let title_h = ui.u(74.0);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(24.0);
    ui.text_in(R::new(m, ui.u(18.0), sc.w - 2.0 * m, title_h), "SINGLE PLAYER", t.display_size, Align::Left, t.title, crate::gfx::Weight::Bold);
    let grid = R::new(m, ui.u(18.0) + title_h, sc.w - 2.0 * m, sc.h - ui.u(18.0) - title_h - foot_h - 2.0 * gap - ui.u(10.0));
    let cols = 2usize;
    let card_h = (grid.h - gap) * 0.5;
    let card_w = ((grid.w - gap) * 0.5).min(card_h * 1.5 + ui.u(24.0));
    let x0 = grid.x + (grid.w - cols as f32 * card_w - (cols - 1) as f32 * gap) * 0.5;
    let y0 = grid.y;
    let mut hover = -1i32;
    for (i, card) in CARDS.iter().enumerate() {
        let r = R::new(x0 + (i % cols) as f32 * (card_w + gap), y0 + (i / cols) as f32 * (card_h + gap), card_w, card_h);
        let id = ui.id(names[i]);
        let live = s.top && s.model.button_at(card.at.0, card.at.1).is_none_or(|c| c.enabled);
        let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
        let st = theme::State::of(ui, id, hot, held);
        theme::panel(ui, r);
        let (bar, rest) = r.take_top(ui.u(46.0));
        let name = names[i];
        let hint = lock_hint(i, &names);
        let lock = hint.is_some();
        let col = if lock {
            crate::gfx::mix(t.text_off, t.text, 0.3 * st.hot)
        } else if hot {
            t.accent
        } else {
            t.title
        };
        let tw = ui.text_in(bar.inset(ui.u(8.0)), name, t.heading_size, Align::Left, col, crate::gfx::Weight::Bold);
        if lock {
            let ph = ui.u(20.0);
            padlock(ui, R::new(bar.x + ui.u(8.0) + tw + ui.u(12.0), bar.y + (bar.h - ph) * 0.5, ph, ph), col);
        }
        let art = kit::contain(rest.inset(ui.u(8.0)), card.art);
        // A locked campaign's picture, dimmed.
        s.mirror_part(ui, card.art, art, if lock { 0xff606060 } else { 0xffffffff });
        ui.draw.frame(art, 1.0, fade(t.panel_edge, 0.8));
        if let Some(h) = hint.as_deref().filter(|_| st.hot > 0.01) {
            let hb = R::new(art.x, art.bottom() - ui.u(40.0), art.w, ui.u(40.0));
            ui.draw.rect(hb, fade(0xff000000, 0.7 * st.hot));
            ui.text_in(hb, h, t.text_size, Align::Centre, fade(t.text, st.hot), crate::gfx::Weight::Bold);
        }
        if st.hot > 0.01 {
            ui.draw.frame(r, ui.u(2.0).max(1.0), fade(if lock { t.text_off } else { t.accent }, st.hot));
        }
        if hot {
            hover = i as i32;
        }
        if clicked {
            s.click(card.at.0, card.at.1);
        }
    }
    // The stock screen hears of the hover, so its own animation and sound play.
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 { CARDS[hover as usize].at } else { (2, 2) };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
        }
    }
    // The foot: Previous Menu at the left, Load Game at the right.
    let fy = sc.h - foot_h - gap;
    let (bw, _) = (ui.u(300.0), 0);
    kit::button(ui, s, R::new(m, fy, bw, foot_h), foot[0], &foot[0].text);
    let last = foot.len() - 1;
    kit::button(ui, s, R::new(sc.w - m - bw, fy, bw, foot_h), foot[last], &foot[last].text);
    true
}

/// The emblem of card `i` in `r` (a square), tinted.
fn emblem(ui: &mut Ui, i: usize, r: R, c: crate::gfx::Color) {
    match crate::content::get().campaigns[i].emblem {
        Some((x, y)) => {
            let Some(t) = icons() else { return };
            let k = 1.0 / 256.0;
            ui.draw.image_uv(t, r, [x as f32 * k, y as f32 * k, (x + 84) as f32 * k, (y + 84) as f32 * k], c);
        }
        None => academy(ui, r, c),
    }
}

/// Starfleet Academy's patch, drawn: a triangle standing on its base, its corners
/// rounded, round the Federation's delta.
fn academy(ui: &mut Ui, r: R, c: crate::gfx::Color) {
    let (cx, w) = (r.x + r.w * 0.5, r.w * 0.94);
    let h = w * 0.88;
    let top = r.y + (r.h - h) * 0.5;
    let pts = [(cx, top), (cx + w * 0.5, top + h), (cx - w * 0.5, top + h)];
    let lw = (r.w * 0.075).max(1.5);
    for i in 0..3 {
        ui.draw.line(pts[i], pts[(i + 1) % 3], lw, c);
        ui.draw.rect_round(R::new(pts[i].0 - lw * 0.5, pts[i].1 - lw * 0.5, lw, lw), lw * 0.5, c);
    }
    if let Some(t) = icons() {
        // The delta of the Federation's icon, without the bar behind it.
        let k = 1.0 / 256.0;
        let d = R::new(cx - w * 0.2, top + h * 0.36, w * 0.4, w * 0.48);
        ui.draw.image_uv(t, d, [24.0 * k, 2.0 * k, 60.0 * k, 80.0 * k], c);
    }
}

/// Over render3d's map: the column at the left, as the main menu's, under a title;
/// each campaign its emblem and its name, lit as the pointer comes over it, which
/// moves the camera to its scene; the row whose scene shows keeps its bar. Load Game
/// and Previous Menu follow in the column.
fn over_scene(ui: &mut Ui, s: &Screen, names: &[&str; 4], foot: &[&crate::menus::model::Control]) -> bool {
    use crate::gfx::Weight;
    let sc = ui.screen();
    let t = ui.theme;
    ui.draw.rect_hgrad(R::new(0.0, 0.0, sc.w * 0.22, sc.h), 0xd8010205, 0xa0010205);
    ui.draw.rect_hgrad(R::new(sc.w * 0.22, 0.0, sc.w * 0.3, sc.h), 0xa0010205, 0x00010205);
    ui.draw.rect_vgrad(R::new(0.0, 0.0, sc.w, sc.h * 0.12), 0x80000000, 0x00000000);
    ui.draw.rect_vgrad(R::new(0.0, sc.h * 0.84, sc.w, sc.h * 0.16), 0x00000000, 0x90000000);

    let (w, eh, gap) = (ui.u(420.0), ui.u(68.0), ui.u(10.0));
    let (fh, title_h) = (ui.u(46.0), ui.u(90.0));
    let total = title_h + 4.0 * eh + 3.0 * gap + ui.u(36.0) + 2.0 * fh;
    let x = libm::roundf(sc.w * 0.08).max(ui.u(40.0));
    let top = libm::roundf((sc.h - total) * 0.5);
    ui.text_in(R::new(x, top - ui.u(40.0), w, ui.u(40.0)), "STAR TREK ARMADA II", 24.0, Align::Left, fade(t.text, 0.85), Weight::Bold);
    ui.text_in(R::new(x - ui.u(2.0), top - ui.u(8.0), w * 1.5, ui.u(76.0)), "SINGLE PLAYER", 56.0, Align::Left, t.title, Weight::Bold);
    ui.draw.rect_hgrad(R::new(x, top + ui.u(74.0), ui.u(260.0), ui.u(2.0).max(1.0)), t.accent, fade(t.accent, 0.0));
    let shown = crate::menus::scene::chosen();
    let mut y = top + title_h;
    let mut hover = -1i32;
    for (i, card) in CARDS.iter().enumerate() {
        let r = R::new(x, y, w, eh);
        y += eh + gap;
        let id = ui.id(names[i]);
        let live = s.top && s.model.button_at(card.at.0, card.at.1).is_none_or(|c| c.enabled);
        let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
        let st = theme::State::of(ui, id, hot, held);
        let here = shown.is_some_and(|v| v == crate::content::get().campaigns[i].scene) || (shown.is_none() && i == 1);
        let k = st.hot.max(if here { 0.45 } else { 0.0 });
        let hint = lock_hint(i, names);
        let lock = hint.is_some();
        // A locked campaign's bar is grey, not the accent: it is pointed at, not offered.
        let bar = if lock { t.text_off } else { t.accent };
        if k > 0.01 {
            ui.draw.rect_hgrad(r, fade(bar, 0.26 * st.hot), fade(bar, 0.0));
            ui.draw.rect(R::new(r.x, r.y + ui.u(10.0), ui.u(4.0).max(2.0), r.h - ui.u(20.0)), fade(bar, k));
        }
        let c = if !live {
            t.text_off
        } else if lock {
            crate::gfx::mix(t.text_off, t.text, 0.25 + 0.25 * k)
        } else if held {
            t.accent
        } else {
            crate::gfx::mix(t.text, 0xffffffff, k)
        };
        let nudge = ui.u(8.0) * st.hot;
        let is = ui.u(46.0);
        emblem(ui, i, R::new(r.x + ui.u(20.0) + nudge, r.y + (r.h - is) * 0.5, is, is), c);
        let name = names[i];
        let tr = R::new(r.x + ui.u(20.0) + is + ui.u(18.0) + nudge, r.y, r.w - is - ui.u(44.0), r.h);
        // Locked and pointed at: the name moves up and what opens it shows below.
        let hk = if lock { st.hot } else { 0.0 };
        let name_r = R::new(tr.x, tr.y - ui.u(11.0) * hk, tr.w, tr.h);
        let tw = ui.text_in(name_r, name, 24.0, Align::Left, c, Weight::Bold);
        if let Some(h) = hint.as_deref() {
            let ph = ui.u(18.0);
            padlock(ui, R::new(tr.x + tw + ui.u(12.0), name_r.y + (name_r.h - ph) * 0.5, ph, ph), c);
            if hk > 0.01 {
                let hr = R::new(tr.x, tr.y + ui.u(15.0), tr.w, tr.h);
                ui.text_in(hr, h, t.small_size, Align::Left, fade(t.text, 0.85 * hk), Weight::Regular);
            }
        }
        if hot {
            hover = i as i32;
            crate::menus::scene::choose(crate::content::get().campaigns[i].scene);
        }
        if clicked {
            s.click(card.at.0, card.at.1);
        }
    }
    // The stock screen hears of the hover, so its own sound plays.
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 { CARDS[hover as usize].at } else { (2, 2) };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
            // The Tutorial and Federation panels are hit areas of the procedure, which
            // plays no sound for them: ours plays the panels' buttons' own.
            if hover >= 0 && s.model.button_at(x, y).is_none() {
                crate::engine::sound::shell_hover(like_button(s));
            }
        }
    }
    // Load Game, then Previous Menu, quieter, below a gap. The stock screen is not told
    // of the pointer over them (its buttons stand elsewhere), so ours plays the sound.
    y += ui.u(26.0);
    let last = foot.len() - 1;
    let mut foot_hover = 0usize;
    for c in [foot[last], foot[0]] {
        let r = R::new(x, y, w, fh);
        y += fh;
        let id = ui.id(c.text.as_str());
        let on = s.top && c.enabled;
        let (hot, held, clicked) = if on { ui.interact(id, r) } else { (false, false, false) };
        if hot {
            foot_hover = c.key;
        }
        let st = theme::State::of(ui, id, hot, held);
        if st.hot > 0.01 {
            ui.draw.rect(R::new(r.x, r.y + ui.u(10.0), ui.u(3.0).max(2.0), r.h - ui.u(20.0)), fade(t.accent, st.hot));
        }
        let col = if !on { t.text_off } else if held { t.accent } else { crate::gfx::mix(fade(t.text, 0.8), 0xffffffff, st.hot) };
        ui.text_in(R::new(r.x + ui.u(20.0) + ui.u(8.0) * st.hot, r.y, r.w, r.h), &c.text, 19.0, Align::Left, col, Weight::Bold);
        if clicked {
            s.click_control(c);
        }
    }
    unsafe {
        if s.top && FOOT_HOVER != foot_hover {
            FOOT_HOVER = foot_hover;
            if foot_hover != 0 {
                crate::engine::sound::shell_hover(Some(foot_hover));
            }
        }
    }
    true
}

/// The foot entry under the pointer last frame (its widget; 0 for none).
static mut FOOT_HOVER: usize = 0;

/// A picture button of the stock screen (the Klingon or Borg panel), whose hover sound
/// the panels that are hit areas borrow.
fn like_button(s: &Screen) -> Option<usize> {
    CARDS.iter().find_map(|c| s.model.button_at(c.at.0, c.at.1)).map(|c| c.key)
}
