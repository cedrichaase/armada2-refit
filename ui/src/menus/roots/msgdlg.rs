//! The VCL's message box (`TMessageForm`, the form the library's `MessageDlg` builds) as
//! Armada II's message window: FleetOpsHook shows it where a task dialog would stand on a
//! system without one, which under Wine is everywhere, among others for the map editor's
//! question before Save Map overwrites a map (PLAN-FLEETOPS.md, decision 623). The title is
//! the form's caption, the text its `Message` label, the buttons its own windowed buttons in
//! the order it shows them, each pressed at its place. The keyboard stays on the form, so
//! Enter is its default button and Escape its cancel button, as on Roots' own box.
use super::kit;
use crate::engine::api::forms::Form;
use crate::menus::screens::notice::notice_window;
use crate::widgets::Ui;
use alloc::string::String;
use alloc::vec::Vec;

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let Some(text) = kit::shown(f, "Message").map(|c| c.text.as_str()).filter(|t| !t.trim().is_empty()) else {
        return false;
    };
    let mut buttons: Vec<_> = f.controls.iter().filter(|c| c.visible && c.hwnd != 0 && c.class == "TButton" && c.rect[2] > 0).collect();
    if buttons.is_empty() {
        return false;
    }
    buttons.sort_by_key(|c| (c.rect[0], c.rect[1]));
    static mut LOGGED: usize = 0;
    if unsafe { LOGGED } != f.hwnd {
        unsafe { LOGGED = f.hwnd };
        crate::log!("forms: message box {:?}: {:?}, {} buttons", f.caption, text, buttons.len());
    }
    let title = if f.caption.trim().is_empty() { String::from("WARNING") } else { f.caption.to_uppercase() };
    ui.draw.rect(ui.screen(), ui.theme.scrim);
    // The library breaks its lines with "\n\r" (the file name, then the question).
    let text: String = text.chars().filter(|&c| c != '\r').collect();
    let mut press = None;
    notice_window(ui, &title, &text, buttons.len(), &mut |ui, i, r| {
        let label = kit::caption(buttons[i]).replace('&', "");
        if ui.scope(&crate::sfmt!("mb{}", i), |ui| ui.button(r, &label)) {
            press = Some(i);
        }
    });
    if let Some(i) = press {
        kit::press(f, buttons[i]);
    }
    true
}
