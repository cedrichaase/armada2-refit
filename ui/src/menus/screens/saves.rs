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
use crate::rt::win::{WM_CHAR, WM_KEYDOWN};
use crate::widgets::{Align, FieldEvent, ListEvent, Ui};
use alloc::string::String;

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

fn draw(ui: &mut Ui, s: &Screen, save: bool) -> bool {
    let buttons = kit::buttons(s);
    let mut rows: alloc::vec::Vec<&Control> = buttons.iter().copied().filter(|c| c.rect[2] >= 500 && !c.text.trim().is_empty()).collect();
    rows.sort_by_key(|c| c.rect[1]);
    let mut arrows: alloc::vec::Vec<&Control> = buttons.iter().copied().filter(|c| c.text.is_empty() && c.rect[2] <= 40).collect();
    arrows.sort_by_key(|c| c.rect[1]);
    let mut foot: alloc::vec::Vec<&Control> = buttons.iter().copied().filter(|c| !c.text.is_empty() && c.rect[2] < 500).collect();
    foot.sort_by_key(|c| c.rect[0]);
    if foot.len() < 3 || arrows.len() < 2 {
        return false;
    }
    let edit = s.model.controls.iter().find(|c| matches!(c.what, What::Window(Class::Edit)));
    if save && edit.is_none() {
        return false;
    }
    let t = ui.theme;
    let title = kit::labels(s).into_iter().find(|c| c.visible && !c.text.is_empty()).map_or(if save { "Save Game" } else { "Load Game" }, |c| c.text.as_str());

    kit::backdrop(ui);
    let win = kit::centred(ui, 1240.0, 900.0);
    let inner = ui.window(win, title);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let field_h = ui.u(t.row_h + 6.0);
    let bottom = foot_h + gap + if save { field_h + gap } else { 0.0 };
    let body = R::new(inner.x, inner.y, inner.w, inner.h - bottom);
    let arrow_w = ui.u(44.0);
    let list = R::new(body.x, body.y, body.w - arrow_w - gap, body.h);

    // The stock rows do not show which one is chosen: the choice is what our clicks made.
    let sel = unsafe { &mut SEL }.of(s.slot.hwnd);
    if sel.0.is_some_and(|i| i >= rows.len()) {
        sel.0 = None;
    }
    let mut chosen = sel.0;
    let row_h = ui.u(t.text_size + 22.0);
    let mut ev = ListEvent::None;
    let live = s.top;
    let draw_row = |ui: &mut Ui, i: usize, rr: R| {
        let (name, date) = split_date(&rows[i].text);
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
            s.click_control(rows[i]);
        }
        sel.0 = Some(i);
    }
    // The wheel pages the stock list.
    if live && ui.input.wheel != 0.0 && ui.input.mouse.is_some_and(|p| list.contains(p)) {
        s.click_control(if ui.input.wheel > 0.0 { arrows[0] } else { arrows[arrows.len() - 1] });
        sel.0 = None;
    }
    // Its two scroll buttons.
    let ar = R::new(list.right() + gap, body.y, arrow_w, ui.u(40.0));
    if kit::arrow(ui, s, ar, arrows[0], true) {
        sel.0 = None;
    }
    let ar = R::new(list.right() + gap, body.bottom() - ui.u(40.0), arrow_w, ui.u(40.0));
    if kit::arrow(ui, s, ar, arrows[arrows.len() - 1], false) {
        sel.0 = None;
    }

    let mut submit = false;
    if let (true, Some(e)) = (save, edit) {
        let name = unsafe { &mut NAME }.of(s.slot.hwnd);
        if name.stock != e.text {
            name.stock = e.text.clone();
            name.local = e.text.clone();
        }
        let fr = R::new(inner.x, body.bottom() + gap, inner.w, field_h);
        let lw = ui.u(150.0);
        ui.text_in(R::new(fr.x, fr.y, lw, fr.h), "Name", t.text_size, Align::Left, t.text_dim, Weight::Regular);
        let field = R::new(fr.x + lw, fr.y, fr.w - lw, fr.h);
        if s.top {
            match ui.text_field(field, "name", &mut name.local, 0) {
                FieldEvent::Changed => {
                    // Rewrite the stock edit: what it held goes, the new text is typed.
                    actions::push(Action::Key { h: e.hwnd, msg: WM_KEYDOWN, wp: 0x23 });
                    for _ in 0..name.stock.chars().count() {
                        actions::push(Action::Key { h: e.hwnd, msg: WM_CHAR, wp: 8 });
                    }
                    for c in name.local.chars() {
                        if (c as u32) < 256 {
                            actions::push(Action::Key { h: e.hwnd, msg: WM_CHAR, wp: c as usize });
                        }
                    }
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
    // Loading and deleting need a save picked (the stock buttons are drawn grey until
    // one is, but their flag is not).
    let picked = sel.0.is_some();
    kit::button(ui, s, cells[0], foot[0], &foot[0].text);
    kit::button_if(ui, s, cells[1], foot[1], &foot[1].text, foot[1].enabled && (save || picked));
    kit::button_if(ui, s, cells[2], foot[2], &foot[2].text, foot[2].enabled && picked);
    if (submit || ev == ListEvent::Activated && !save) && foot[1].enabled {
        s.click_control(foot[1]);
    }
    true
}
