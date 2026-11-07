//! What Roots' screen specs share: finding a form's controls by name and caption, and
//! drawing each kind of control Roots has as the widget of ours that Armada II's settings
//! screens use, acting on it as a player would (`engine::api::forms`): a combo by its
//! window, a check box by a press at its centre, a slider by a drag on its track, a text
//! field by giving its window the keyboard, a button by a press. Nothing here writes a
//! setting: the form's own handlers do.
//!
//! The layout is `screens::form`'s (the window, the heading, the caption over a field, the
//! footer), so Roots' screens stand where Armada II's settings screens stand.
use crate::engine::api::forms::{self, Action, Control, Form};
use crate::gfx::{Weight, R};
use crate::menus::screens::form as sform;
use crate::widgets::theme::{self, State};
use crate::widgets::{Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// A control by name when it is on the form now (visible, with a size).
pub fn shown<'a>(f: &'a Form, name: &str) -> Option<&'a Control> {
    f.control(name).filter(|c| c.visible && c.rect[2] > 0 && c.rect[3] > 0)
}

/// The caption of a label by its name: what the player reads, "" when it is not shown.
pub fn text<'a>(f: &'a Form, name: &str) -> &'a str {
    shown(f, name).map_or("", |c| c.text.as_str())
}

/// A button's caption, by what Roots set on it (its name when that is empty).
pub fn caption(c: &Control) -> &str {
    if c.text.is_empty() || c.text == "*" { c.name.as_str() } else { c.text.as_str() }
}

/// A press of ours on the control, when the form is live and it is enabled.
pub fn press(f: &Form, c: &Control) {
    if f.top && c.enabled {
        forms::press(f, c);
    }
}

/// The foot's button, "Previous Menu" or the caption Roots gave it: where Armada II's
/// settings screens have theirs (the foot's left end).
pub fn back(ui: &mut Ui, f: &Form, foot: R, name: &str) {
    let Some(c) = shown(f, name) else { return };
    let r = R::new(foot.x, foot.bottom() - ui.u(44.0), ui.u(220.0), ui.u(44.0));
    if ui.scope("back", |ui| ui.button(r, caption(c))) {
        press(f, c);
    }
}

/// A button as a field of the form, in `r`.
pub fn button(ui: &mut Ui, f: &Form, c: &Control, r: R) -> bool {
    let clicked = ui.scope(&crate::sfmt!("btn{}", c.name), |ui| ui.button(r, caption(c)));
    if clicked {
        press(f, c);
    }
    clicked
}

/// A combo box as a dropdown of ours: the choice is `CB_SETCURSEL` and the parent's
/// notification, the way a player's choice reaches the form's change handler.
pub fn combo(ui: &mut Ui, f: &Form, c: &Control, r: R) {
    let items: Vec<&str> = c.items.iter().map(|s| s.as_str()).collect();
    let mut sel = c.selected.max(0) as usize;
    let changed = ui.scope(&crate::sfmt!("combo{}", c.name), |ui| ui.dropdown(r, "combo", &items, &mut sel));
    if changed && f.top && c.enabled && sel as i32 != c.selected {
        crate::log!("forms: {} of {} set to {:?}", c.name, f.class, items.get(sel));
        forms::act(Action::Select(c.hwnd, sel as i32));
    }
}

/// A check box as ours, showing the state Roots' control holds; a click is a press of the
/// control (it flips itself). A control whose state could not be read shows unchecked and
/// still presses through.
pub fn check(ui: &mut Ui, f: &Form, c: &Control, r: R) {
    let mut on = c.checked.unwrap_or(false);
    let label = if c.text.is_empty() { c.name.as_str() } else { c.text.as_str() };
    let flipped = ui.scope(&crate::sfmt!("check{}", c.name), |ui| ui.checkbox(r, label, &mut on));
    if flipped && f.top && c.enabled {
        click(f, c);
    }
}

/// A click at a windowed control's centre, delivered to the window there (a check box has
/// a window of its own, which a press posted to the form does not reach).
pub fn click(f: &Form, c: &Control) {
    let (x, y) = (c.rect[0] + 8, c.rect[1] + c.rect[3] / 2);
    crate::log!("forms: click on {} ({:?}) of {}", c.name, c.text, f.class);
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_MOVE, x, y));
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, x, y));
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, x, y));
}

/// What a slider of ours is being dragged to: the control's name and the value.
static mut DRAG: (u32, f32) = (0, 0.0);
/// A value being set on Roots' slider: the control's name, the target, the frame count
/// since the last attempt and the attempts made.
static mut SETTING: (u32, i32, u32, u32) = (0, 0, 0, 0);

fn key_of(name: &str) -> u32 {
    name.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193)) | 1
}

/// The slider's thumb centre on its track, in the form's client coordinates, for a value.
fn thumb_x(c: &Control, v: i32, lo: i32, hi: i32) -> i32 {
    let edge = 6;
    let span = (c.rect[2] - 2 * edge).max(1);
    c.rect[0] + edge + ((v - lo) as f32 / (hi - lo).max(1) as f32 * span as f32) as i32
}

/// A slider as ours, at the value Roots' slider holds (`Value` of `MinValue..MaxValue`).
/// Dragging it is a drag of the control's thumb at the matching points; the value is read
/// back and, where the drag fell short, stepped by presses at the track's ends, so what it
/// shows is always the control's. Without a readable value it draws nothing (the caller
/// keeps the form's picture for it).
pub fn slider(ui: &mut Ui, f: &Form, c: &Control, r: R) -> bool {
    let Some((value, lo, hi)) = c.value else { return false };
    if hi <= lo {
        return false;
    }
    let k = key_of(&c.name);
    let drag = unsafe { DRAG.0 == k };
    let mut v = if drag { unsafe { DRAG.1 } } else { value as f32 };
    let changed = ui.scope(&crate::sfmt!("slider{}", c.name), |ui| ui.slider_step(r, "bar", &mut v, lo as f32, hi as f32, 1.0));
    let to = libm::roundf(v) as i32;
    if f.top && c.enabled {
        if changed {
            if ui.input.down[0] {
                unsafe { DRAG = (k, v) };
            } else if to != value {
                set(f, c, to);
            }
        }
        if drag && !ui.input.down[0] {
            let t = libm::roundf(unsafe { DRAG.1 }) as i32;
            unsafe { DRAG = (0, 0.0) };
            if t != value {
                set(f, c, t);
            }
        }
        follow(f, c, value, lo, hi);
    }
    true
}

/// Starts setting a slider: a drag from the thumb's place to the target's.
fn set(f: &Form, c: &Control, target: i32) {
    let Some((value, lo, hi)) = c.value else { return };
    crate::log!("forms: {} set to {} (of {}..{})", c.name, target, lo, hi);
    unsafe { SETTING = (key_of(&c.name), target, 0, 0) };
    let (y, a, b) = (c.rect[1] + c.rect[3] / 2, thumb_x(c, value, lo, hi), thumb_x(c, target, lo, hi));
    let h = f.hwnd;
    forms::act(Action::Mouse(h, forms::MOUSE_DOWN, a, y));
    forms::act(Action::Mouse(h, forms::MOUSE_MOVE, (a + b) / 2, y));
    forms::act(Action::Mouse(h, forms::MOUSE_MOVE, b, y));
    forms::act(Action::Mouse(h, forms::MOUSE_UP, b, y));
}

/// A slider being set: read back, and where the drag left it short, a press at the end of
/// the track on the target's side (the control steps towards a press on its track).
fn follow(f: &Form, c: &Control, value: i32, lo: i32, hi: i32) {
    let (k, target, since, tries) = unsafe { SETTING };
    if k != key_of(&c.name) {
        return;
    }
    if value == target || tries >= 40 {
        unsafe { SETTING = (0, 0, 0, 0) };
        return;
    }
    if since < 6 {
        unsafe { SETTING.2 += 1 };
        return;
    }
    let x = if target > value { c.rect[0] + c.rect[2] - 2 } else { c.rect[0] + 2 };
    let y = c.rect[1] + c.rect[3] / 2;
    let _ = (lo, hi);
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, x, y));
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, x, y));
    unsafe { SETTING = (k, target, 0, tries + 1) };
}

/// A text field of Roots', drawn as a field: what it holds, with a caret while it has
/// the keyboard (a click gives the real field the keyboard and the typing goes to it).
pub fn edit(ui: &mut Ui, f: &Form, c: &Control, r: R) {
    let id = ui.id(&crate::sfmt!("edit{}", c.name));
    let t = ui.theme;
    let (hot, held, clicked) = ui.interact(id, r);
    if clicked && f.top {
        forms::act(Action::Focus(c.hwnd));
    }
    let st = State::of(ui, id, hot, held);
    theme::field(ui, r, st);
    let pad = ui.u(t.pad * 0.7);
    let inner = R::new(r.x + pad, r.y, (r.w - 2.0 * pad).max(1.0), r.h);
    let shown: String = c.text.clone();
    ui.text_in(inner, &shown, t.text_size, Align::Left, t.text, Weight::Regular);
}

/// A small dim note under or beside a field.
pub fn note(ui: &mut Ui, r: R, text: &str) {
    let t = ui.theme;
    let px = ui.u(t.small_size);
    ui.draw.text_wrapped(text, px, r.w, r.x, r.y, t.text_dim, Weight::Regular);
}

/// The caption over a field, in the settings screens' place and size.
pub fn caption_over(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) {
    sform::caption(ui, R::new(x, y, w, ui.u(sform::LABEL_H)), text);
}

/// One column of a settings form: items stacked from the top, the way
/// `screens::form::draw_with` lays a column out (a caption over each field, 16 units
/// between).
pub struct Column {
    pub x: f32,
    pub y: f32,
    pub w: f32,
}

impl Column {
    pub fn new(x: f32, y: f32, w: f32) -> Column {
        Column { x, y, w }
    }

    pub fn heading(&mut self, ui: &mut Ui, text: &str) {
        sform::heading(ui, self.x, self.y, self.w, text);
        self.y += ui.u(sform::HEADING_H);
    }

    /// A caption over a field of `h` units; the field's rectangle.
    pub fn field(&mut self, ui: &mut Ui, label: &str, h: f32) -> R {
        if !label.is_empty() {
            caption_over(ui, self.x, self.y, self.w, label);
            self.y += ui.u(sform::LABEL_H);
        }
        let r = R::new(self.x, self.y, self.w, ui.u(h));
        self.y += ui.u(h + sform::GAP);
        r
    }

    /// A row of `h` units with `gap` after it.
    pub fn row(&mut self, ui: &mut Ui, h: f32, gap: f32) -> R {
        let r = R::new(self.x, self.y, self.w, ui.u(h));
        self.y += ui.u(h + gap);
        r
    }

    pub fn skip(&mut self, ui: &mut Ui, h: f32) {
        self.y += ui.u(h);
    }
}

/// The height, in units, of a column of the given parts: for the window's body.
pub fn height(parts: &[f32]) -> f32 {
    parts.iter().sum()
}

pub const FIELD: f32 = sform::FIELD_H;
pub const LABEL: f32 = sform::LABEL_H;
pub const GAP: f32 = sform::GAP;
pub const SLIDER: f32 = sform::SLIDER_H;
pub const HEADING: f32 = sform::HEADING_H;
pub const CHECK: f32 = 30.0;
pub const CHECK_GAP: f32 = 10.0;
