//! The mission message box, ours (`[HUD] Messages=1`; HUD.md, "Messages and
//! objectives"): while the stock `MessageDisplay` shows, a window in the middle of a
//! dimmed screen with the message's title, its text and OK, which presses the stock
//! box's own OK (it hides the box and lets the game go on). Like stock's box it keeps
//! the mouse from everything under it; the keyboard stays the engine's, as in stock
//! (the stock box reads no keys).
use crate::engine::popups;
use crate::gfx::{Weight, R};
use crate::widgets::Ui;

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
    ui.overlay(move |ui| {
        let t = ui.theme;
        let w = 720.0;
        let px = ui.u(t.text_size);
        let pad = ui.u(t.pad);
        let (_, th) = crate::text::measure_wrapped(crate::text::font(), &m.body, px, ui.u(w) - 4.0 * pad);
        let bh = ui.u(t.button_h);
        // The window's height in units: the text, the button, the title and padding.
        let h = ((th + bh) / ui.scale + t.title_size + 6.0 * t.pad + 40.0).min(1000.0);
        let title = if m.title.is_empty() { " " } else { m.title.as_str() };
        let inner = super::panels::boxed(ui, title, w, h, 1.0);
        ui.draw.text_wrapped(&m.body, px, inner.w - pad, inner.x + pad * 0.5, inner.y + pad * 0.5, t.text, Weight::Regular);
        let bw = ui.u(180.0);
        let r = R::new(inner.right() - bw, inner.bottom() - bh, bw, bh);
        if m.ok != 0 {
            super::panels::game_text_button(ui, r, m.ok, "OK", &[]);
        }
    });
}
