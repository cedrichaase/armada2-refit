//! The engine's tooltips at the pointer, ours (`[HUD] Tooltips=1`; HUD.md, "The
//! tooltips at the pointer"): what `CursorInterface` would draw for the mouse's owner
//! (in play, an object's name and description over the 3D view), in the theme's
//! tooltip beside the pointer, at the screen's resolution. Stock's is drawn on the
//! 1600 x 1200 canvas and so stretched on a wide screen. The text is the engine's own,
//! read as the cursor renders (`engine::hud::tooltip`); our panels' tooltips are their
//! own.
use crate::engine::hud;
use crate::gfx::R;
use crate::widgets::{theme, Ui};
use alloc::string::String;

static mut LAST: String = String::new();

pub fn run(ui: &mut Ui) {
    let Some(raw) = hud::tooltip() else { return };
    let text = clean(&raw);
    if text.is_empty() {
        return;
    }
    #[allow(static_mut_refs)]
    unsafe {
        if LAST != text {
            crate::log!("tooltip: {:?}", text);
            LAST = text.clone();
        }
    }
    let Some((mx, my)) = ui.input.mouse else { return };
    ui.overlay(move |ui| {
        let (w, h) = theme::tooltip_size(ui, &text, ui.u(420.0));
        // Below and right of the pointer, clear of the cursor's art; kept on screen.
        let mut x = mx + ui.u(18.0);
        let mut y = my + ui.u(26.0);
        if x + w > ui.w - ui.u(4.0) {
            x = (mx - w - ui.u(8.0)).max(ui.u(4.0));
        }
        if y + h > ui.h - ui.u(4.0) {
            y = (my - h - ui.u(8.0)).max(ui.u(4.0));
        }
        theme::tooltip(ui, R::new(libm::roundf(x), libm::roundf(y), w, h), &text, 1.0);
    });
}

/// The game's strings mark a hotkey's letter with `^` and keep printf's `%%`.
fn clean(s: &str) -> String {
    let o: String = s.chars().filter(|&c| c != '^' && c != '\r' && c != '\0').collect();
    String::from(o.replace("%%", "%").trim())
}
