//! The engine's tooltips at the pointer, ours (`[HUD] Tooltips=1`; HUD.md, "The
//! tooltips at the pointer"): what `CursorInterface` would draw for the mouse's owner
//! (in play, an object's name and description over the 3D view), in the theme's
//! tooltip beside the pointer, at the screen's resolution. Stock's is drawn on the
//! HUD canvas (`api::CANVAS`) and so stretched on a wide screen. The text is the engine's own,
//! read as the cursor renders (`engine::hud::tooltip`); our panels' tooltips are their
//! own.
use crate::engine::api::hud;
use crate::gfx::R;
use crate::widgets::{theme, Ui};
use alloc::string::String;

static mut LAST: String = String::new();

pub fn run(ui: &mut Ui) {
    let Some(raw) = hud::tooltip() else { return };
    let mut text = clean(&raw);
    if super::bargrid::keys_under_pointer() {
        text = without_hotkey(&text);
    }
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

/// Roots' button tooltips name the bar's own hotkey (`(Hotkey: B)`), which does nothing while the
/// grid's position keys are on (QOL empties the bar's hotkey tables): the note goes, the cell's
/// badge names the key.
fn without_hotkey(s: &str) -> String {
    let Some(i) = s.find("(Hotkey") else { return String::from(s) };
    let end = s[i..].find(')').map_or(s.len(), |j| i + j + 1);
    let mut o = String::from(s[..i].trim_end());
    let rest = s[end..].trim_start_matches([' ', '\n']);
    if !rest.is_empty() {
        o.push('\n');
        o.push_str(rest);
    }
    o
}
