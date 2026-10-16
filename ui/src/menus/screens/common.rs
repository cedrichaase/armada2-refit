//! What the spec screens of the Options family share: the shell's backdrop, the stock
//! controls found by kind and place, the footer of "back" buttons, the stock button
//! drawn as our button, and the hover the stock screen is told about.
//!
//! The stock screens are laid out in 800x600 design pixels, with the words of a screen
//! as `TextLabel`s and the choices as `ShellButton`s, so a spec finds them by place
//! (a label above a button names it) rather than by text: the text is the game's own,
//! localised, and is never written here.
use crate::engine::dialogs::DESIGN_W;
use crate::engine::shellui::Kind;
use crate::menus::model::{Control, What};
use crate::menus::{actions, backdrop, Screen};
use crate::rt::win::{HWND, WM_MOUSEMOVE};
use crate::widgets::{theme, Ui};
use crate::gfx::R;

/// A shell button of any class (not a label or a scroll bar).
pub fn is_button(c: &Control) -> bool {
    matches!(c.what, What::Shell(k) if k != Kind::Label && k != Kind::Scroll)
}

pub fn is_label(c: &Control) -> bool {
    matches!(c.what, What::Shell(Kind::Label))
}

pub fn is_scroll(c: &Control) -> bool {
    matches!(c.what, What::Shell(Kind::Scroll))
}

/// A button at the foot of the screen: Previous Menu, Return to Game, the OK of a
/// dialog. The stock screens put them in the bottom-left corner of the design area.
pub fn is_footer(c: &Control) -> bool {
    is_button(c) && c.rect[1] >= 500 && c.rect[0] < 300
}

/// What a screen is drawn over: the shell plate for a full-screen dialog, the mission's
/// dimmed picture (already drawn by the menus' backdrop) for one over a mission, a
/// dimming for a dialog smaller than the screen over another.
pub fn backdrop_for(ui: &mut Ui, s: &Screen) {
    if s.model.rect[2] >= DESIGN_W && !backdrop::over_mission() {
        let r = ui.screen();
        theme::backdrop(ui, r);
    } else {
        let r = ui.screen();
        ui.draw.rect(r, ui.theme.scrim);
    }
}

/// The screen's title: the label at the top of the design area.
pub fn title<'a>(s: &'a Screen) -> Option<&'a Control> {
    s.model.controls.iter().filter(|c| is_label(c) && c.visible && !c.text.is_empty() && c.rect[1] < 90).min_by_key(|c| c.rect[1])
}

/// The small label at the foot of the screen (the version).
pub fn footnote<'a>(s: &'a Screen) -> Option<&'a Control> {
    s.model.controls.iter().find(|c| is_label(c) && c.visible && !c.text.is_empty() && c.rect[1] >= 500 && c.text.len() <= 12)
}

/// The last stock button the pointer was told about, per dialog, so the stock screen
/// (its hover sound and picture) hears the pointer arrive on and leave a button once.
static mut HOVER: (HWND, usize) = (core::ptr::null_mut(), 0);

/// Tells the stock screen the pointer is over `c` (or over none, `None`).
pub fn hover_stock(s: &Screen, c: Option<&Control>) {
    if !s.top {
        return;
    }
    let key = c.map_or(0, |c| c.key);
    unsafe {
        if HOVER == (s.slot.hwnd, key) {
            return;
        }
        HOVER = (s.slot.hwnd, key);
    }
    let (x, y) = match c {
        Some(c) => (c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2),
        None => (2, 2),
    };
    actions::push(actions::Action::Mouse { h: s.slot.hwnd, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
}

/// A stock button as our button, in `r`; true when clicked (the click is then sent to
/// the stock button, whose own handler does what the stock screen does). Returns
/// whether the pointer is over it through `hot`.
pub fn stock_button<'a>(ui: &mut Ui, s: &Screen, c: &'a Control, r: R, hot: &mut Option<&'a Control>) -> bool {
    let label = if c.text.is_empty() { " " } else { c.text.as_str() };
    if !(s.top && c.enabled) {
        crate::menus::generic::shell_button_look(ui, r, label, false, false, false);
        return false;
    }
    let clicked = ui.scope(&crate::sfmt!("b{:x}", c.key), |ui| {
        let over = ui.input.mouse.is_some_and(|p| r.contains(p));
        if over {
            *hot = Some(c);
        }
        ui.button(r, label)
    });
    if clicked {
        s.click_control(c);
    }
    clicked
}

/// The buttons at the foot of the screen, in a row from the left of `area`, each
/// `w` x `h` pixels, `gap` apart.
pub fn footer<'a>(ui: &mut Ui, s: &'a Screen, area: R, hot: &mut Option<&'a Control>) {
    let mut feet: alloc::vec::Vec<&'a Control> = s.model.controls.iter().filter(|c| is_footer(c) && c.visible).collect();
    feet.sort_by_key(|c| c.rect[0]);
    let (w, h, gap) = (ui.u(220.0), ui.u(44.0), ui.u(12.0));
    let mut x = area.x;
    for c in feet {
        stock_button(ui, s, c, R::new(x, area.bottom() - h, w, h), hot);
        x += w + gap;
    }
}
