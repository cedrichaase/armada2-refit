//! What the single-player screen specs share: the shell's backdrop, a centred window, a
//! stock button drawn as ours, and the small statics a spec keeps between frames.
//! Everything is drawn with `crate::widgets` in the shell theme (`STYLE.md`); nothing
//! here writes a setting, a click is the stock control's own.
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Control, What};
use crate::menus::Screen;
use crate::rt::win::HWND;
use crate::widgets::{theme, Align, Anchor, State, Ui};
use alloc::vec::Vec;

/// What a full-screen front-end screen of ours stands on: the theme's night gradient
/// (the menu plate has the main menu's words baked into it, so it is left to the main
/// menu). In a mission the dimmed mission is already under it.
pub fn backdrop(ui: &mut Ui) {
    if crate::menus::scene::shown() {
        let r = ui.screen();
        theme::backdrop(ui, r);
    } else if !crate::menus::backdrop::over_mission() {
        let (r, t) = (ui.screen(), ui.theme);
        ui.draw.rect_vgrad(r, t.backdrop_top, t.backdrop_bottom);
    }
}

/// A window of `w` x `h` units, centred and never bigger than the screen.
pub fn centred(ui: &Ui, w: f32, h: f32) -> R {
    let sc = ui.screen();
    let (w, h) = (ui.u(w).min(sc.w - ui.u(32.0)), ui.u(h).min(sc.h - ui.u(32.0)));
    Anchor::Centre.place(sc, w, h, 0.0)
}

/// Dims what is under a modal screen.
pub fn scrim(ui: &mut Ui) {
    let (r, c) = (ui.screen(), ui.theme.scrim);
    ui.draw.rect(r, c);
}

/// The shell buttons of the dialog that show, in the order the screen made them.
pub fn buttons<'a>(s: &'a Screen) -> Vec<&'a Control> {
    s.model.controls.iter().filter(|c| matches!(c.what, What::Shell(k) if k != Kind::Label) && c.visible).collect()
}

/// The dialog's labels, hidden ones too, in the order the screen made them.
pub fn labels<'a>(s: &'a Screen) -> Vec<&'a Control> {
    s.model.controls.iter().filter(|c| matches!(c.what, What::Shell(Kind::Label))).collect()
}

/// A stock control drawn as our button: a click on it is a click on the control. Only
/// the innermost dialog acts; a disabled control is drawn so. True on the click.
pub fn button(ui: &mut Ui, s: &Screen, r: R, c: &Control, label: &str) -> bool {
    button_if(ui, s, r, c, label, c.enabled)
}

/// `button`, drawn and answering only when `on` (a screen that knows better than the
/// control's own flag, which the stock rows leave set).
pub fn button_if(ui: &mut Ui, s: &Screen, r: R, c: &Control, label: &str, on: bool) -> bool {
    if s.top && on {
        if ui.button(r, label) {
            s.click_control(c);
            return true;
        }
    } else {
        theme::button(ui, r, label, State { off: !on, ..State::default() });
    }
    false
}

/// A small arrow button (up or down), for a stock scroll button.
pub fn arrow(ui: &mut Ui, s: &Screen, r: R, c: &Control, up: bool) -> bool {
    let id = ui.id(if up { "arrow-up" } else { "arrow-down" });
    let (hot, held, clicked) = if s.top && c.enabled { ui.interact(id, r) } else { (false, false, false) };
    let st = State::of(ui, id, hot, held);
    theme::button_frame(ui, r, st);
    let t = ui.theme;
    let (cx, cy) = r.centre();
    let (hw, hh) = (r.w * 0.22, r.h * 0.12);
    let d = if up { -1.0 } else { 1.0 };
    let w = ui.u(2.5).max(1.5);
    let col = if c.enabled { t.text } else { t.text_off };
    ui.draw.line((cx - hw, cy - d * hh), (cx, cy + d * hh), w, col);
    ui.draw.line((cx, cy + d * hh), (cx + hw, cy - d * hh), w, col);
    if clicked {
        s.click_control(c);
    }
    clicked
}

/// The fit of a source rectangle (design coordinates) into `dst` keeping its shape,
/// centred: where the part is drawn.
pub fn contain(dst: R, src: [i32; 4]) -> R {
    let k = (dst.w / src[2] as f32).min(dst.h / src[3] as f32);
    let (w, h) = (src[2] as f32 * k, src[3] as f32 * k);
    R::new(libm::roundf(dst.x + (dst.w - w) * 0.5), libm::roundf(dst.y + (dst.h - h) * 0.5), libm::roundf(w), libm::roundf(h))
}

/// Hands the pointer (and the wheel and typing) to the hidden dialog as the stock
/// screen would see them, for a part `src` of the dialog that is drawn in `dst` (the
/// same shape, from `contain`). Used where a stock pane is mirrored and its own
/// controls must keep working.
pub fn forward_part(ui: &mut Ui, s: &Screen, dst: R, src: [i32; 4]) {
    if !s.top {
        return;
    }
    let inside = ui.input.mouse.is_some_and(|p| dst.contains(p));
    let busy = ui.input.down[0] || ui.input.released[0] || ui.input.down[1] || ui.input.released[1];
    if !inside && !busy {
        return;
    }
    let k = dst.w / src[2] as f32;
    let fit = crate::menus::Fit { x: dst.x - src[0] as f32 * k, y: dst.y - src[1] as f32 * k, s: k };
    let part = Screen { slot: s.slot, model: s.model, fit, mirror: s.mirror, top: s.top };
    crate::menus::generic::forward_input(ui, &part);
}

/// A line of text in `r`, in the theme's body size.
pub fn text(ui: &mut Ui, r: R, s: &str, align: Align, c: u32) {
    let size = ui.theme.text_size;
    ui.text_in(r, s, size, align, c, Weight::Regular);
}

/// What a spec keeps for one dialog between frames: which window it was for.
pub struct Memory<T> {
    pub dlg: HWND,
    pub v: T,
}

impl<T: Default> Memory<T> {
    pub const fn new(v: T) -> Memory<T> {
        Memory { dlg: core::ptr::null_mut(), v }
    }
    /// The value for `dlg`, reset when it is a new dialog.
    pub fn of(&mut self, dlg: HWND) -> &mut T {
        if self.dlg != dlg {
            self.dlg = dlg;
            self.v = T::default();
        }
        &mut self.v
    }
}
