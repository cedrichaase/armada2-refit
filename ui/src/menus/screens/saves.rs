//! Load Game and Save Game (`do_loadGame`, `do_saveGame`): the stock rows
//! (`FileSelectButton`s labelled "<name> <date>") as our list, the stock buttons at the
//! foot, and for Save the stock name `Edit` as our text field.
//!
//! The stock list shows one page: its rows are re-labelled when its two small scroll
//! buttons are clicked, so our arrows and the wheel click those buttons. A row is picked
//! by a click on the stock row (which for Save copies its name into the edit); Load and
//! Save are clicks on the stock buttons, so a confirmation, the load itself and the
//! messages are the stock screen's. The name field holds its own text and, when it
//! changes, rewrites the stock edit with the keys a player would type (End, Backspace
//! for what was there, then the characters); when the stock edit changes under it (a row
//! was picked) the field takes the stock text.
use crate::gfx::{Weight, R};
use crate::menus::actions::{self, Action};
use crate::menus::model::{Class, Control, What};
use crate::menus::screens::kit::{self, Memory};
use crate::menus::Screen;
use crate::rt::win::{HWND, WM_CHAR, WM_KEYDOWN};
use crate::widgets::{theme, Align, FieldEvent, ListEvent, State, Ui};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Default)]
struct Sel(Option<usize>);

#[derive(Default)]
struct Name {
    /// What the field shows, and what the stock edit held when we last looked or wrote.
    local: String,
    stock: String,
}

static mut SEL: Memory<Sel> = Memory::new(Sel(None));
static mut NAME: Memory<Name> = Memory::new(Name { local: String::new(), stock: String::new() });

pub fn draw_load(ui: &mut Ui, s: &Screen) -> bool {
    draw(ui, s, false)
}

pub fn draw_save(ui: &mut Ui, s: &Screen) -> bool {
    draw(ui, s, true)
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// A row's label "<name> <Mon> <day> <time> <AM|PM>" as (name, date).
fn split_date(s: &str) -> (&str, &str) {
    let mut at = None;
    let mut pos = 0;
    for tok in s.split(' ') {
        if MONTHS.contains(&tok) {
            at = Some(pos);
        }
        pos += tok.len() + 1;
    }
    match at {
        Some(p) if p > 0 => (s[..p].trim_end(), &s[p..]),
        _ => (s, ""),
    }
}

/// The stock screen's parts, found in its shell buttons: the rows (wide, labelled, top
/// to bottom), the two scroll buttons (small, unlabelled), and the foot (Previous Menu,
/// the action, Delete), left to right. None when it is not the screen expected.
pub fn parts<'a>(buttons: &[&'a Control]) -> Option<(Vec<&'a Control>, Vec<&'a Control>, Vec<&'a Control>)> {
    let mut rows: Vec<&Control> = buttons.iter().copied().filter(|c| c.rect[2] >= 500 && !c.text.trim().is_empty()).collect();
    rows.sort_by_key(|c| c.rect[1]);
    let mut arrows: Vec<&Control> = buttons.iter().copied().filter(|c| c.text.is_empty() && c.rect[2] <= 40).collect();
    arrows.sort_by_key(|c| c.rect[1]);
    let mut foot: Vec<&Control> = buttons.iter().copied().filter(|c| !c.text.is_empty() && c.rect[2] < 500).collect();
    foot.sort_by_key(|c| c.rect[0]);
    if foot.len() < 3 || arrows.len() < 2 {
        return None;
    }
    Some((rows, arrows, foot))
}

fn draw(ui: &mut Ui, s: &Screen, save: bool) -> bool {
    let buttons = kit::buttons(s);
    let Some((rows, arrows, foot)) = parts(&buttons) else { return false };
    let edit = s.model.controls.iter().find(|c| matches!(c.what, What::Window(Class::Edit)));
    if save && edit.is_none() {
        return false;
    }
    let title = kit::labels(s).into_iter().find(|c| c.visible && !c.text.is_empty()).map_or(if save { "Save Game" } else { "Load Game" }, |c| c.text.as_str());
    kit::backdrop(ui);
    let names: Vec<&str> = rows.iter().map(|c| c.text.as_str()).collect();
    let up = arrows[0];
    let down = arrows[arrows.len() - 1];
    let input = SavesIn {
        dlg: s.slot.hwnd,
        top: s.top,
        save,
        title,
        rows: &names,
        foot: [(foot[0].text.as_str(), foot[0].enabled), (foot[1].text.as_str(), foot[1].enabled), (foot[2].text.as_str(), foot[2].enabled)],
        arrows: [up.enabled, down.enabled],
        edit: edit.map(|e| e.text.as_str()),
    };
    for a in saves_screen(ui, &input) {
        match a {
            SavesAct::Row(i) => s.click_control(rows[i]),
            SavesAct::Up => s.click_control(up),
            SavesAct::Down => s.click_control(down),
            SavesAct::Foot(i) => s.click_control(foot[i]),
            SavesAct::Type { erase, text } => {
                let Some(e) = edit else { continue };
                // Rewrite the stock edit: what it held goes, the new text is typed.
                actions::push(Action::Key { h: e.hwnd, msg: WM_KEYDOWN, wp: 0x23 });
                for _ in 0..erase {
                    actions::push(Action::Key { h: e.hwnd, msg: WM_CHAR, wp: 8 });
                }
                for c in text.chars() {
                    if (c as u32) < 256 {
                        actions::push(Action::Key { h: e.hwnd, msg: WM_CHAR, wp: c as usize });
                    }
                }
            }
        }
    }
    true
}

/// What the save list shows (`saves_screen`): the dialog it keys its memory by, whether
/// it takes input, Load or Save, the title, the rows' labels, the foot's three buttons
/// (label, enabled), whether the two scroll buttons are enabled, and Save's name as the
/// stock edit holds it.
pub struct SavesIn<'a> {
    pub dlg: HWND,
    pub top: bool,
    pub save: bool,
    pub title: &'a str,
    pub rows: &'a [&'a str],
    pub foot: [(&'a str, bool); 3],
    pub arrows: [bool; 2],
    pub edit: Option<&'a str>,
}

/// What the player did on the save list this frame, in order.
pub enum SavesAct {
    /// A row picked: a click on the stock row.
    Row(usize),
    /// The stock list's scroll buttons.
    Up,
    Down,
    /// A foot button (0 Previous Menu, 1 the action, 2 Delete).
    Foot(usize),
    /// The name field changed: the stock edit's `erase` characters to go, `text` typed.
    Type { erase: usize, text: String },
}

/// Load Game and Save Game laid out, for any front end (Armada II's hosted screens, and
/// Roots' adopted ones, `menus::roots_dialogs`): a window with the list (name and date),
/// its two scroll buttons and the wheel, for Save the name field, and the foot. Load and
/// Delete need a save picked (the stock buttons are drawn grey until one is, but their
/// flag is not).
pub fn saves_screen(ui: &mut Ui, inp: &SavesIn) -> Vec<SavesAct> {
    let mut acts = Vec::new();
    let (save, rows) = (inp.save, inp.rows);
    let t = ui.theme;
    let win = kit::centred(ui, 1240.0, 900.0);
    let inner = ui.window(win, inp.title);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let field_h = ui.u(t.row_h + 6.0);
    let bottom = foot_h + gap + if save { field_h + gap } else { 0.0 };
    let body = R::new(inner.x, inner.y, inner.w, inner.h - bottom);
    let arrow_w = ui.u(44.0);
    let list = R::new(body.x, body.y, body.w - arrow_w - gap, body.h);

    // The stock rows do not show which one is chosen: the choice is what our clicks made.
    let sel = unsafe { &mut *core::ptr::addr_of_mut!(SEL) }.of(inp.dlg);
    if sel.0.is_some_and(|i| i >= rows.len()) {
        sel.0 = None;
    }
    let mut chosen = sel.0;
    let row_h = ui.u(t.text_size + 22.0);
    let mut ev = ListEvent::None;
    let live = inp.top;
    let draw_row = |ui: &mut Ui, i: usize, rr: R| {
        let (name, date) = split_date(rows[i]);
        let pad = ui.u(t.pad);
        ui.text_in(R::new(rr.x + pad, rr.y, rr.w * 0.62, rr.h), name, t.text_size, Align::Left, t.text, Weight::Regular);
        ui.text_in(R::new(rr.x + rr.w * 0.6, rr.y, rr.w * 0.4 - pad, rr.h), date, t.small_size, Align::Right, t.text_dim, Weight::Regular);
    };
    if live {
        ev = ui.list(list, "saves", rows.len(), row_h, &mut chosen, |ui, i, rr, _s, _h| draw_row(ui, i, rr));
    } else {
        let mut none = chosen;
        ui.list(list, "saves", rows.len(), row_h, &mut none, |ui, i, rr, _s, _h| draw_row(ui, i, rr));
    }
    if rows.is_empty() {
        ui.text_in(list, if save { "No saved games" } else { "No saved games to load" }, t.text_size, Align::Centre, t.text_dim, Weight::Regular);
    }
    if let Some(i) = chosen {
        if ev != ListEvent::None && i < rows.len() {
            acts.push(SavesAct::Row(i));
        }
        sel.0 = Some(i);
    }
    // The wheel pages the stock list.
    if live && ui.input.wheel != 0.0 && ui.input.mouse.is_some_and(|p| list.contains(p)) {
        acts.push(if ui.input.wheel > 0.0 { SavesAct::Up } else { SavesAct::Down });
        sel.0 = None;
    }
    // Its two scroll buttons.
    let ar = R::new(list.right() + gap, body.y, arrow_w, ui.u(40.0));
    if kit::arrow_at(ui, ar, live, inp.arrows[0], true) {
        acts.push(SavesAct::Up);
        sel.0 = None;
    }
    let ar = R::new(list.right() + gap, body.bottom() - ui.u(40.0), arrow_w, ui.u(40.0));
    if kit::arrow_at(ui, ar, live, inp.arrows[1], false) {
        acts.push(SavesAct::Down);
        sel.0 = None;
    }

    let mut submit = false;
    if let (true, Some(e)) = (save, inp.edit) {
        let name = unsafe { &mut *core::ptr::addr_of_mut!(NAME) }.of(inp.dlg);
        if name.stock != e {
            name.stock = String::from(e);
            name.local = String::from(e);
        }
        let fr = R::new(inner.x, body.bottom() + gap, inner.w, field_h);
        let lw = ui.u(150.0);
        ui.text_in(R::new(fr.x, fr.y, lw, fr.h), "Name", t.text_size, Align::Left, t.text_dim, Weight::Regular);
        let field = R::new(fr.x + lw, fr.y, fr.w - lw, fr.h);
        if inp.top {
            match ui.text_field(field, "name", &mut name.local, 0) {
                FieldEvent::Changed => {
                    acts.push(SavesAct::Type { erase: name.stock.chars().count(), text: name.local.clone() });
                    name.stock = name.local.clone();
                }
                FieldEvent::Submit => submit = true,
                _ => {}
            }
        } else {
            let shown = name.local.clone();
            ui.text_in(field.inset(ui.u(8.0)), &shown, t.text_size, Align::Left, t.text, Weight::Regular);
        }
    }
    // The foot: Previous Menu, the action, Delete Game.
    let cells = {
        let mut st = crate::widgets::Stack::new(R::new(inner.x, inner.bottom() - foot_h, inner.w, foot_h), gap);
        st.cells::<3>(foot_h)
    };
    let picked = sel.0.is_some();
    let on = [inp.foot[0].1, inp.foot[1].1 && (save || picked), inp.foot[2].1 && picked];
    for i in 0..3 {
        // As `kit::button_if`: only the top dialog acts; a disabled one is drawn so.
        if inp.top && on[i] {
            if ui.button(cells[i], inp.foot[i].0) {
                acts.push(SavesAct::Foot(i));
            }
        } else {
            theme::button(ui, cells[i], inp.foot[i].0, State { off: !on[i], ..State::default() });
        }
    }
    if (submit || ev == ListEvent::Activated && !save) && inp.foot[1].1 {
        acts.push(SavesAct::Foot(1));
    }
    acts
}
