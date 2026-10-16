//! The mission message box, ours (`[HUD] Messages=1`; HUD.md, "Messages and
//! objectives"): while the stock `MessageDisplay` shows, a window in the middle of a
//! dimmed screen with the message's title, its text and OK. OK (or Enter, or Esc)
//! presses the stock box's own OK, which hides it and lets the game go on. Like stock's
//! box it is modal: nothing under it takes the mouse or the keys.
use crate::engine::popups;
use crate::gfx::{Weight, R};
use crate::widgets::Ui;

const VK_RETURN: u8 = 0x0d;
const VK_ESCAPE: u8 = 0x1b;

static mut LOGGED: bool = false;

pub fn run(ui: &mut Ui) {
    let Some(m) = popups::message() else {
        unsafe { LOGGED = false };
        return;
    };
    unsafe {
        if !LOGGED {
            crate::log!("messages: \"{}\" ({} chars), OK {}", m.title, m.body.len(), if m.ok != 0 { "found" } else { "NOT found" });
            LOGGED = true;
        }
    }
    let t = ui.theme;
    let w = ui.u(720.0);
    let px = ui.u(t.text_size);
    let pad = ui.u(t.pad);
    let (_, th) = crate::text::measure_wrapped(crate::text::font(), &m.body, px, w - 4.0 * pad);
    let bh = ui.u(t.button_h);
    // The window's height in units: the text, the button, the title and padding.
    let h_units = (th + bh) / ui.scale + t.title_size + 6.0 * t.pad + 40.0;
    let title = if m.title.is_empty() { alloc::string::String::from(" ") } else { m.title.clone() };
    let ok = m.ok;
    let body = m.body.clone();
    ui.modal(&title, 720.0, h_units.min(1000.0), move |ui, inner| {
        let t = ui.theme;
        ui.draw.text_wrapped(&body, px, inner.w - pad, inner.x + pad * 0.5, inner.y + pad * 0.5, t.text, Weight::Regular);
        let bw = ui.u(180.0);
        let r = R::new(inner.right() - bw, inner.bottom() - bh, bw, bh);
        super::panels::game_text_button(ui, r, ok, "OK", &[VK_RETURN, VK_ESCAPE]);
    });
}
