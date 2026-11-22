//! Roots' text windows: the map editor's licence (`LicenseAgreementWindowDlgProc`, adopted,
//! opened by FleetOpsHook's import on the editor's first start) and the lobby's Help
//! (`HelpWindowDlgProc`, the executable's import). Armada II's text window
//! (`screens::text_window::text_window`): the agreement in our scroll area, the stock
//! buttons (ACCEPT, CANCEL) in a row at the foot, clicked as a player would. The text is
//! read whole from the stock edit that holds it (`WM_GETTEXT`); the edit itself is never
//! drawn, so its selection (Roots' picture shows the whole agreement selected, since the
//! edit has the focus) is not ours to show.
use super::Screen;
use crate::engine::api::shellui::Kind;
use crate::menus::model::{Class, Control, What};
use crate::menus::screens::text_window::{full_text, text_window};
use crate::widgets::Ui;
use alloc::vec::Vec;

/// The stock button under the pointer last frame (0: none).
static mut HOVER: (usize, usize) = (0, 0);

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let holder = s
        .model
        .controls
        .iter()
        .filter(|c| c.visible && matches!(c.what, What::Window(Class::Edit) | What::Window(Class::Static) | What::Shell(Kind::Label)) && c.text.len() > 40)
        .max_by_key(|c| c.rect[2] * c.rect[3]);
    let Some(holder) = holder else { return false };
    let text = match holder.what {
        What::Window(_) => full_text(holder.hwnd),
        _ => holder.text.clone(),
    };
    let mut btns: Vec<&Control> = s
        .model
        .controls
        .iter()
        .filter(|c| c.visible && !c.text.trim().is_empty() && c.key != holder.key && (matches!(c.what, What::Shell(k) if k != Kind::Label && k != Kind::Scroll) || matches!(c.what, What::Window(Class::Button))))
        .collect();
    if btns.is_empty() {
        return false;
    }
    btns.sort_by_key(|c| (c.rect[0], c.rect[1]));
    // Roots paints the title into the dialog; a label that holds it is used when there is one.
    let name = s
        .labels()
        .into_iter()
        .filter(|c| c.visible && c.key != holder.key && !c.text.trim().is_empty() && c.text.len() <= 60)
        .min_by_key(|c| c.rect[1])
        .map_or(if s.model.screen == "help" { "Help" } else { "License Agreement" }, |c| c.text.trim());
    let r = ui.screen();
    ui.draw.rect(r, ui.theme.scrim);
    let mut hot = 0usize;
    let mut clicked = None;
    text_window(ui, name, &text, btns.len(), &mut |ui, i, r| {
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
