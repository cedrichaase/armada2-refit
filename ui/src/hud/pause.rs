//! The pause box, ours (`[HUD] Pause=1`; HUD.md, "Pause and replay"): while the stock
//! `PauseBox` shows (the pause order shows and hides it), a small window in the middle
//! of a lightly dimmed screen with the game's "paused" text and its continue button,
//! which presses the stock unpause button (the unpause order). The keyboard stays the
//! engine's, so the pause key unpauses as in stock.
use crate::engine::{buttons, popups};
use crate::gfx::{Weight, R};
use crate::widgets::{Align, Anchor, Ui};
use alloc::string::String;

struct Words {
    text: String,
    cont: String,
}

static mut WORDS: Option<Words> = None;
static mut LOGGED: bool = false;

#[allow(static_mut_refs)]
fn words() -> &'static Words {
    unsafe {
        WORDS.get_or_insert_with(|| Words {
            text: buttons::localize(c"GUI_POP_PAUSE_TEXT"),
            cont: buttons::localize(c"GUI_POP_PAUSE_CONTINUE"),
        })
    }
}

pub fn run(ui: &mut Ui) {
    if !popups::pause_shown() {
        unsafe { LOGGED = false };
        return;
    }
    let b = popups::pause_button();
    let w = words();
    unsafe {
        if !LOGGED {
            crate::log!("pause: shown, \"{}\", button \"{}\" {}", w.text, buttons::label(b), if b != 0 { "found" } else { "NOT found" });
            LOGGED = true;
        }
    }
    ui.overlay(move |ui| {
        let t = ui.theme;
        let screen = ui.screen();
        ui.draw.rect(screen, crate::gfx::fade(t.scrim, 0.5));
        ui.claim(screen);
        let (bw, bh) = (ui.u(240.0), ui.u(t.button_h));
        let text_h = ui.u(t.display_size) * 1.4;
        let (outer, inner) = super::panels::fit(ui, Anchor::Centre, ui.u(420.0), text_h + ui.u(t.pad) + bh, 0.0);
        ui.panel(outer);
        ui.text_in(R::new(inner.x, inner.y, inner.w, text_h), &crate::widgets::theme::caps(&w.text), t.display_size, Align::Centre, t.title, Weight::Bold);
        let r = R::new(inner.x + (inner.w - bw) * 0.5, inner.bottom() - bh, bw, bh);
        if b != 0 {
            super::panels::game_text_button(ui, r, b, &w.cont, &[]);
        }
    });
}
