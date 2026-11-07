//! Roots' Load Game and Save Game (`LoadGameDlgProc`, `SaveGameDlgProc`, adopted): Armada
//! II's screens (`screens::saves::saves_screen`) over Roots' dialogs, which keep the stock
//! parts at Roots' places: the rows (`FileSelectButton`s, one page of twelve, a hidden
//! row for an empty slot), the list's two small scroll buttons, the foot (Previous Menu,
//! the action, Delete), and for Save the name's `Edit`, a child window. A row picked, a
//! scroll, a foot button are clicks on the stock control; the name typed into our field
//! rewrites the stock edit with the keys a player would type, so the stock screen saves,
//! loads, asks and answers as it does under Roots.
use super::Screen;
use crate::engine::api::forms::{self, Action};
use crate::menus::model::{Class, What};
use crate::menus::screens::saves::{parts, saves_screen, SavesAct, SavesIn};
use crate::rt::win::{WM_CHAR, WM_KEYDOWN};
use crate::widgets::Ui;
use alloc::vec::Vec;

/// The row last clicked on the stock list: (dialog, its label).
static mut PICKED: Option<(usize, alloc::string::String)> = None;

pub fn draw_load(ui: &mut Ui, s: &Screen) -> bool {
    draw(ui, s, false)
}

pub fn draw_save(ui: &mut Ui, s: &Screen) -> bool {
    draw(ui, s, true)
}

fn draw(ui: &mut Ui, s: &Screen, save: bool) -> bool {
    let buttons = s.buttons();
    let Some((rows, arrows, foot)) = parts(&buttons) else { return false };
    let edit = s.model.controls.iter().find(|c| matches!(c.what, What::Window(Class::Edit)) && c.visible);
    if save && edit.is_none() {
        return false;
    }
    // Roots paints the title into the dialog; its label is empty.
    let title = s.labels().into_iter().find(|c| c.visible && !c.text.trim().is_empty()).map_or(if save { "Save Game" } else { "Load Game" }, |c| c.text.trim());
    crate::menus::screens::kit::backdrop(ui);
    let names: Vec<&str> = rows.iter().map(|c| c.text.trim()).collect();
    let (up, down) = (arrows[0], arrows[arrows.len() - 1]);
    let input = SavesIn {
        dlg: s.hwnd() as crate::rt::win::HWND,
        top: s.top,
        save,
        title,
        rows: &names,
        foot: [(foot[0].text.trim(), foot[0].enabled), (foot[1].text.trim(), foot[1].enabled), (foot[2].text.trim(), foot[2].enabled)],
        arrows: [up.enabled, down.enabled],
        edit: edit.map(|e| e.text.as_str()),
    };
    for a in saves_screen(ui, &input) {
        match a {
            SavesAct::Row(i) => {
                // A row already picked is not clicked again: two clicks on a stock row are
                // its double click, Roots' own load, whose question answered Yes then
                // loaded nothing (decision 255). A double click of ours is the pick and
                // then Load, as the foot's button.
                let key = (s.hwnd(), rows[i].text.clone());
                let last = unsafe { &mut *core::ptr::addr_of_mut!(PICKED) };
                if last.as_ref() != Some(&key) {
                    *last = Some(key);
                    s.click_control(rows[i]);
                }
            }
            SavesAct::Up => s.click_control(up),
            SavesAct::Down => s.click_control(down),
            SavesAct::Foot(i) => s.click_control(foot[i]),
            SavesAct::Type { erase, text } => {
                let Some(e) = edit else { continue };
                let h = e.hwnd as usize;
                // What it held goes (End, then Backspace), the new text is typed.
                forms::act(Action::Key(h, WM_KEYDOWN, 0x23));
                for _ in 0..erase {
                    forms::act(Action::Key(h, WM_CHAR, 8));
                }
                for c in text.chars() {
                    if (c as u32) < 256 {
                        forms::act(Action::Key(h, WM_CHAR, c as usize));
                    }
                }
            }
        }
    }
    true
}
