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
//! pointing at it says what opens it ("Complete the Federation Campaign to unlock"); its
//! scene still shows, locked (render3d). A click plays `[Menus] LockedSound` and still
//! goes to the stock panel, which has the last word (on a locked campaign it does
//! nothing).
use crate::gfx::{fade, Tex, R};
use crate::menus::screens::kit;
use crate::menus::{actions, Screen};
use crate::rt::win::{HWND, WM_MOUSEMOVE};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// What opens card `i`, while it is locked: "Complete the <name> to unlock".
pub fn lock_hint(i: usize, names: &[&str; 4]) -> Option<String> {
    let req = crate::menus::locks::locked(i)?;
    Some(crate::sfmt!("Complete the {} to unlock", names.get(req).copied().unwrap_or("")))
}

/// A click on the locked card `i`: the "unavailable" sound (`[Menus] LockedSound`),
/// logged, so a bench can tell it played (the bench has no audio).
pub fn locked_click(i: usize) {
    let snd = crate::config::get().locked_sound.clone();
    let played = crate::engine::api::sound::shell_named(&snd);
    let key = crate::content::get().campaigns.get(i).map_or("?", |c| c.key);
    crate::log!("menus: locked campaign {} clicked: sound {}", key, if played { snd.as_str() } else { "none" });
}

/// A padlock in `r` (its height sets its size): the icon atlas's `padlock`
/// (`ui/fontbake/src/icons.rs`), in the name's colour.
fn padlock(ui: &mut Ui, r: R, c: crate::gfx::Color) {
    crate::icons::draw(&mut ui.draw, "padlock", r, 1.0, |_| c);
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
            let file = crate::content::get().files.race_icons;
            let t = if file.is_empty() { None } else { crate::gfx::load_tga(file) };
            let name = file.rsplit(['/', '\\']).next().unwrap_or("");
            if t.is_some() {
                crate::log!("menus: campaign emblems from {}", name);
            } else {
                crate::log!("menus: campaign emblems not found ({}): names only", name);
            }
            ICONS = Some(t);
        }
        ICONS.flatten()
    }
}

/// Campaign `i`'s emblem picture (the content set's `Emblem = <file>`, or `[Menus]
/// Emblem<Id>=` in UI.ini: another file, `none` for the card's own art), read once. None:
/// no picture named or readable, so the caller draws what it drew before.
static mut PICTURES: Vec<Option<Tex>> = Vec::new();

#[allow(static_mut_refs)]
pub fn campaign_picture(i: usize) -> Option<Tex> {
    let camps = crate::content::get().campaigns;
    unsafe {
        if PICTURES.len() != camps.len() {
            let ini = crate::rt::Ini::open(&crate::rt::game_path("UI.ini"));
            PICTURES = camps
                .iter()
                .map(|c| {
                    let over = ini.string("Menus", &crate::sfmt!("Emblem{}", c.id)).unwrap_or_default();
                    let file = if over.trim().is_empty() { c.emblem_file } else { over.trim() };
                    if file.is_empty() || file.eq_ignore_ascii_case("none") {
                        return None;
                    }
                    let t = crate::gfx::load_tga(file);
                    crate::log!("menus: campaign {} emblem {}: {}", c.id, file, if t.is_some() { "read" } else { "not found, the card's art stands" });
                    t
                })
                .collect();
        }
        PICTURES.get(i).copied().flatten()
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

pub fn card_names<'a>(labels: &[&'a crate::menus::model::Control]) -> [&'a str; 4] {
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
    let names = card_names(&names);
    kit::backdrop(ui);
    let hints: [Option<String>; 4] = core::array::from_fn(|i| lock_hint(i, &names));
    let rows: [ColumnRow; 4] = core::array::from_fn(|i| ColumnRow {
        name: names[i],
        hint: hints[i].as_deref(),
        live: s.top && s.model.button_at(CARDS[i].at.0, CARDS[i].at.1).is_none_or(|c| c.enabled),
        here: false,
    });
    let arts: [[i32; 4]; 4] = core::array::from_fn(|i| CARDS[i].art);
    // The foot: Previous Menu at the left, Load Game at the right.
    let last = foot.len() - 1;
    let foot_controls = [foot[0], foot[last]];
    let foot_entries = [(foot[0].text.as_str(), foot[0].enabled), (foot[last].text.as_str(), foot[last].enabled)];
    let out = campaign_cards(ui, "SINGLE PLAYER", &rows, &arts, &mut |ui, i, dst, tint| s.mirror_part(ui, CARDS[i].art, dst, tint), foot_entries, s.top);
    if let Some(i) = out.clicked {
        if hints[i].is_some() {
            locked_click(i);
        }
        s.click(CARDS[i].at.0, CARDS[i].at.1);
    }
    // The stock screen hears of the hover, so its own animation and sound play.
    let hover = out.hover;
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 { CARDS[hover as usize].at } else { (2, 2) };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
        }
    }
    if let Some(i) = out.foot_clicked {
        s.click_control(foot_controls[i]);
    }
    true
}

/// The campaign selection as cards (2x2), for any front end (Armada II's, and Roots'
/// adopted Single Player, `menus::roots_dialogs`), when no scene stands behind it: a
/// title, each card its name in a bar over its picture (`art` for row `i` into a
/// rectangle shaped as `arts[i]`, in a tint: a locked campaign's dimmed, with what opens
/// it shown on hover), lit under the pointer; the foot's two buttons (label, enabled) at
/// the left and right, live only on the top dialog (`top`), as `kit::button` draws them.
pub fn campaign_cards(
    ui: &mut Ui,
    title: &str,
    rows: &[ColumnRow],
    arts: &[[i32; 4]],
    art: &mut dyn FnMut(&mut Ui, usize, R, u32),
    foot: [(&str, bool); 2],
    top: bool,
) -> ColumnOut {
    let t = ui.theme;
    let sc = ui.screen();
    // The screen: a title, the cards (2x2), the foot.
    let m = ui.u(40.0);
    let title_h = ui.u(74.0);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(24.0);
    ui.text_in(R::new(m, ui.u(18.0), sc.w - 2.0 * m, title_h), title, t.display_size, Align::Left, t.title, crate::gfx::Weight::Bold);
    let grid = R::new(m, ui.u(18.0) + title_h, sc.w - 2.0 * m, sc.h - ui.u(18.0) - title_h - foot_h - 2.0 * gap - ui.u(10.0));
    let cols = 2usize;
    let card_h = (grid.h - gap) * 0.5;
    let card_w = ((grid.w - gap) * 0.5).min(card_h * 1.5 + ui.u(24.0));
    let x0 = grid.x + (grid.w - cols as f32 * card_w - (cols - 1) as f32 * gap) * 0.5;
    let y0 = grid.y;
    let mut out = ColumnOut { clicked: None, hover: -1, foot_clicked: None, foot_hover: None };
    for (i, row) in rows.iter().enumerate() {
        let r = R::new(x0 + (i % cols) as f32 * (card_w + gap), y0 + (i / cols) as f32 * (card_h + gap), card_w, card_h);
        let id = ui.id(row.name);
        let (hot, held, clicked) = if row.live { ui.interact(id, r) } else { (false, false, false) };
        let st = theme::State::of(ui, id, hot, held);
        theme::panel(ui, r);
        let (bar, rest) = r.take_top(ui.u(46.0));
        let lock = row.hint.is_some();
        let col = if lock {
            crate::gfx::mix(t.text_off, t.text, 0.3 * st.hot)
        } else if hot {
            t.accent
        } else {
            t.title
        };
        let tw = ui.text_in(bar.inset(ui.u(8.0)), row.name, t.heading_size, Align::Left, col, crate::gfx::Weight::Bold);
        if lock {
            let ph = ui.u(24.0);
            padlock(ui, R::new(bar.x + ui.u(8.0) + tw + ui.u(12.0), bar.y + (bar.h - ph) * 0.5, ph, ph), col);
        }
        let dst = kit::contain(rest.inset(ui.u(8.0)), arts[i]);
        // A locked campaign's picture, dimmed.
        art(ui, i, dst, if lock { 0xff606060 } else { 0xffffffff });
        ui.draw.frame(dst, 1.0, fade(t.panel_edge, 0.8));
        if let Some(h) = row.hint.filter(|_| st.hot > 0.01) {
            let hb = R::new(dst.x, dst.bottom() - ui.u(40.0), dst.w, ui.u(40.0));
            ui.draw.rect(hb, fade(0xff000000, 0.7 * st.hot));
            ui.text_in(hb, h, t.text_size, Align::Centre, fade(t.text, st.hot), crate::gfx::Weight::Bold);
        }
        if st.hot > 0.01 {
            ui.draw.frame(r, ui.u(2.0).max(1.0), fade(if lock { t.text_off } else { t.accent }, st.hot));
        }
        if hot {
            out.hover = i as i32;
        }
        if clicked {
            out.clicked = Some(i);
        }
    }
    let fy = sc.h - foot_h - gap;
    let bw = ui.u(300.0);
    for (i, &(label, on)) in foot.iter().enumerate() {
        let r = if i == 0 { R::new(m, fy, bw, foot_h) } else { R::new(sc.w - m - bw, fy, bw, foot_h) };
        if top && on {
            if ui.button(r, label) {
                out.foot_clicked = Some(i);
            }
        } else {
            theme::button(ui, r, label, theme::State { off: !on, ..theme::State::default() });
        }
    }
    out
}

/// The emblem of card `i` in `r` (a square), tinted.
fn emblem(ui: &mut Ui, i: usize, r: R, c: crate::gfx::Color) {
    if let Some(t) = campaign_picture(i) {
        ui.draw.image(t, r, c);
        return;
    }
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
    let shown = crate::menus::scene::chosen();
    let hints: [Option<String>; 4] = core::array::from_fn(|i| lock_hint(i, names));
    let rows: [ColumnRow; 4] = core::array::from_fn(|i| ColumnRow {
        name: names[i],
        hint: hints[i].as_deref(),
        live: s.top && s.model.button_at(CARDS[i].at.0, CARDS[i].at.1).is_none_or(|c| c.enabled),
        here: shown.is_some_and(|v| v == crate::content::get().campaigns[i].scene) || (shown.is_none() && i == 1),
    });
    // Load Game, then Previous Menu, quieter, below a gap.
    let last = foot.len() - 1;
    let foot_entries = [(foot[last].text.as_str(), s.top && foot[last].enabled), (foot[0].text.as_str(), s.top && foot[0].enabled)];
    let out = campaign_column(ui, "STAR TREK ARMADA II", "SINGLE PLAYER", &rows, &mut |ui, i, r, c| emblem(ui, i, r, c), &foot_entries);
    if out.hover >= 0 {
        crate::menus::scene::choose(crate::content::get().campaigns[out.hover as usize].scene);
    }
    if let Some(i) = out.clicked {
        if hints[i].is_some() {
            locked_click(i);
        }
        s.click(CARDS[i].at.0, CARDS[i].at.1);
    }
    // The stock screen hears of the hover, so its own sound plays.
    let hover = out.hover;
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 { CARDS[hover as usize].at } else { (2, 2) };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
            // The Tutorial and Federation panels are hit areas of the procedure, which
            // plays no sound for them: ours plays the panels' buttons' own.
            if hover >= 0 && s.model.button_at(x, y).is_none() {
                crate::engine::api::sound::shell_hover(like_button(s));
            }
        }
    }
    // The stock screen is not told of the pointer over the foot (its buttons stand
    // elsewhere), so ours plays the sound.
    let foot_controls = [foot[last], foot[0]];
    if let Some(i) = out.foot_clicked {
        s.click_control(foot_controls[i]);
    }
    let foot_hover = out.foot_hover.map_or(0, |i| foot_controls[i].key);
    unsafe {
        if s.top && FOOT_HOVER != foot_hover {
            FOOT_HOVER = foot_hover;
            if foot_hover != 0 {
                crate::engine::api::sound::shell_hover(Some(foot_hover));
            }
        }
    }
    true
}

/// A row of the campaign column (`campaign_column`): the name, what opens it while it
/// is locked, whether it takes the pointer, and whether its scene is the one shown.
pub struct ColumnRow<'a> {
    pub name: &'a str,
    pub hint: Option<&'a str>,
    pub live: bool,
    pub here: bool,
}

/// What the campaign column was told this frame: the row clicked, the row under the
/// pointer (-1: none), and the same for the foot's entries.
pub struct ColumnOut {
    pub clicked: Option<usize>,
    pub hover: i32,
    pub foot_clicked: Option<usize>,
    pub foot_hover: Option<usize>,
}

/// The campaign selection over the scene, for any front end that has one (Armada II's,
/// and Roots' adopted Single Player, `menus::roots_dialogs`): the shade, the title
/// (`small` over `big`), a column of campaigns, each its emblem (drawn by `emblem` for
/// row `i` in a square, in a colour) and its name, lit under the pointer, a locked one
/// greyed with a padlock and what opens it; then the foot's entries (label, enabled),
/// quieter, below a gap.
pub fn campaign_column(
    ui: &mut Ui,
    small: &str,
    big: &str,
    rows: &[ColumnRow],
    emblem: &mut dyn FnMut(&mut Ui, usize, R, crate::gfx::Color),
    foot: &[(&str, bool)],
) -> ColumnOut {
    use crate::gfx::Weight;
    let sc = ui.screen();
    let t = ui.theme;
    ui.draw.rect_hgrad(R::new(0.0, 0.0, sc.w * 0.22, sc.h), 0xd8010205, 0xa0010205);
    ui.draw.rect_hgrad(R::new(sc.w * 0.22, 0.0, sc.w * 0.3, sc.h), 0xa0010205, 0x00010205);
    ui.draw.rect_vgrad(R::new(0.0, 0.0, sc.w, sc.h * 0.12), 0x80000000, 0x00000000);
    ui.draw.rect_vgrad(R::new(0.0, sc.h * 0.84, sc.w, sc.h * 0.16), 0x00000000, 0x90000000);

    let n = rows.len() as f32;
    let (w, eh, gap) = (ui.u(420.0), ui.u(68.0), ui.u(10.0));
    let (fh, title_h) = (ui.u(46.0), ui.u(90.0));
    let total = title_h + n * eh + (n - 1.0) * gap + ui.u(36.0) + foot.len() as f32 * fh;
    let x = libm::roundf(sc.w * 0.08).max(ui.u(40.0));
    let top = libm::roundf((sc.h - total) * 0.5);
    ui.text_in(R::new(x, top - ui.u(40.0), w, ui.u(40.0)), small, 24.0, Align::Left, fade(t.text, 0.85), Weight::Bold);
    ui.text_in(R::new(x - ui.u(2.0), top - ui.u(8.0), w * 1.5, ui.u(76.0)), big, 56.0, Align::Left, t.title, Weight::Bold);
    ui.draw.rect_hgrad(R::new(x, top + ui.u(74.0), ui.u(260.0), ui.u(2.0).max(1.0)), t.accent, fade(t.accent, 0.0));
    let mut y = top + title_h;
    let mut out = ColumnOut { clicked: None, hover: -1, foot_clicked: None, foot_hover: None };
    for (i, row) in rows.iter().enumerate() {
        let r = R::new(x, y, w, eh);
        y += eh + gap;
        let id = ui.id(row.name);
        let live = row.live;
        let (hot, held, clicked) = if live { ui.interact(id, r) } else { (false, false, false) };
        let st = theme::State::of(ui, id, hot, held);
        let k = st.hot.max(if row.here { 0.45 } else { 0.0 });
        let lock = row.hint.is_some();
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
        let tr = R::new(r.x + ui.u(20.0) + is + ui.u(18.0) + nudge, r.y, r.w - is - ui.u(44.0), r.h);
        // Locked and pointed at: the name moves up and what opens it shows below.
        let hk = if lock { st.hot } else { 0.0 };
        let name_r = R::new(tr.x, tr.y - ui.u(11.0) * hk, tr.w, tr.h);
        let tw = ui.text_in(name_r, row.name, 24.0, Align::Left, c, Weight::Bold);
        if let Some(h) = row.hint {
            let ph = ui.u(22.0);
            padlock(ui, R::new(tr.x + tw + ui.u(12.0), name_r.y + (name_r.h - ph) * 0.5, ph, ph), c);
            if hk > 0.01 {
                let hr = R::new(tr.x, tr.y + ui.u(15.0), tr.w, tr.h);
                ui.text_in(hr, h, t.small_size, Align::Left, fade(t.text, 0.85 * hk), Weight::Regular);
            }
        }
        if hot {
            out.hover = i as i32;
        }
        if clicked {
            out.clicked = Some(i);
        }
    }
    y += ui.u(26.0);
    for (i, &(label, on)) in foot.iter().enumerate() {
        let r = R::new(x, y, w, fh);
        y += fh;
        let id = ui.id(label);
        let (hot, held, clicked) = if on { ui.interact(id, r) } else { (false, false, false) };
        if hot {
            out.foot_hover = Some(i);
        }
        let st = theme::State::of(ui, id, hot, held);
        if st.hot > 0.01 {
            ui.draw.rect(R::new(r.x, r.y + ui.u(10.0), ui.u(3.0).max(2.0), r.h - ui.u(20.0)), fade(t.accent, st.hot));
        }
        let col = if !on { t.text_off } else if held { t.accent } else { crate::gfx::mix(fade(t.text, 0.8), 0xffffffff, st.hot) };
        ui.text_in(R::new(r.x + ui.u(20.0) + ui.u(8.0) * st.hot, r.y, r.w, r.h), label, 19.0, Align::Left, col, Weight::Bold);
        if clicked {
            out.foot_clicked = Some(i);
        }
    }
    out
}

/// The foot entry under the pointer last frame (its widget; 0 for none).
static mut FOOT_HOVER: usize = 0;

/// A picture button of the stock screen (the Klingon or Borg panel), whose hover sound
/// the panels that are hit areas borrow.
fn like_button(s: &Screen) -> Option<usize> {
    CARDS.iter().find_map(|c| s.model.button_at(c.at.0, c.at.1)).map(|c| c.key)
}
