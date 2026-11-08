//! Roots' mission list (`MissionSelectDlgProc`, adopted, opened over Single Player through
//! the executable's own import): Armada II's list (`screens::mission_select::mission_list`)
//! over the campaign selection. The rows are the stock `ShellButton`s, one per mission,
//! labelled by the game; picking one is a click on the stock row, so the stock screen
//! keeps the choice; OK and Cancel are the stock buttons (a double click on a row is OK).
use super::Screen;
use crate::menus::screens::mission_select::mission_list;
use crate::widgets::Ui;
use alloc::vec::Vec;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let buttons = s.buttons();
    if buttons.len() < 3 {
        return false;
    }
    // The two at the foot are OK and Cancel; the others are the rows, top to bottom.
    let foot_y = buttons.iter().map(|c| c.rect[1]).max().unwrap_or(0);
    let mut foot: Vec<_> = buttons.iter().copied().filter(|c| c.rect[1] >= foot_y - 2).collect();
    foot.sort_by_key(|c| c.rect[0]);
    let mut rows: Vec<_> = buttons.iter().copied().filter(|c| c.rect[1] < foot_y - 2 && !c.text.trim().is_empty()).collect();
    rows.sort_by_key(|c| c.rect[1]);
    if foot.len() != 2 || rows.is_empty() {
        return false;
    }
    // Roots paints the title into the dialog; its label is empty.
    let title = s.labels().into_iter().find(|c| c.visible && !c.text.trim().is_empty()).map_or("Select Mission", |c| c.text.trim());
    let names: Vec<&str> = rows.iter().map(|c| c.text.trim()).collect();
    let foot_entries = [(foot[0].text.trim(), foot[0].enabled), (foot[1].text.trim(), foot[1].enabled)];
    let out = mission_list(ui, s.hwnd() as crate::rt::win::HWND, title, &names, foot_entries, s.top);
    if let Some(i) = out.picked {
        s.click_control(rows[i]);
    }
    if let Some(i) = out.foot {
        s.click_control(foot[i]);
    }
    if out.activated && foot[0].enabled {
        s.click_control(foot[0]);
    }
    true
}
