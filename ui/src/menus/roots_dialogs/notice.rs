//! Roots' one-button message (`OneButtonDlgProc`, adopted, opened through the executable's
//! own import: "Save Game / Save Successful!" and the errors): Armada II's message window
//! (`screens::notice::notice_window`), its title and text the dialog's labels, its button
//! the stock one, clicked as a player would.
use super::Screen;
use crate::engine::api::shellui::Kind;
use crate::menus::model::{Control, What};
use crate::menus::screens::notice::{notice_window, parts};
use crate::widgets::Ui;
use alloc::vec::Vec;

/// The stock button under the pointer last frame (0: none).
static mut HOVER: (usize, usize) = (0, 0);

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let labels: Vec<&Control> = s.model.controls.iter().filter(|c| matches!(c.what, What::Shell(Kind::Label)) && c.visible && !c.text.trim().is_empty()).collect();
    let btns: Vec<&Control> = s.buttons().into_iter().filter(|c| !c.text.trim().is_empty()).collect();
    let Some((name, text, btns)) = parts(labels, btns) else { return false };
    let r = ui.screen();
    ui.draw.rect(r, ui.theme.scrim);
    let mut hot = 0usize;
    let mut clicked = None;
    notice_window(ui, name.trim(), &text, btns.len(), &mut |ui, i, r| {
        let c = btns[i];
        let label = c.text.trim();
        if !(s.top && c.enabled) {
            crate::menus::generic::shell_button_look(ui, r, label, false, false, false);
            return;
        }
        if ui.input.mouse.is_some_and(|p| r.contains(p)) {
            hot = c.key;
        }
        if ui.scope(&crate::sfmt!("b{:x}", c.key), |ui| ui.button(r, label)) {
            clicked = Some(i);
        }
    });
    if let Some(i) = clicked {
        s.click_control(btns[i]);
    }
    unsafe {
        if s.top && HOVER != (s.hwnd(), hot) {
            HOVER = (s.hwnd(), hot);
            s.hover(btns.iter().find(|c| c.key == hot).map(|c| (c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2)));
        }
    }
    true
}
