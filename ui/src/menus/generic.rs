//! The generic screen: any hosted dialog, drawn without a spec of its own.
//!
//! Two ways, `[Menus] Generic`:
//!
//! - **mirrored** (the default): the dialog as the game paints it, every control
//!   included, scaled into the design area; the pointer and the keyboard go to the
//!   hidden dialog as they would to the stock one. Every screen works this way, the
//!   art, the hover pictures and the Bink animations included.
//! - **widgets**: the dialog's controls (`model`) as our widgets in their design places,
//!   over a backdrop of ours; what the model cannot read (a picture button, an
//!   owner-drawn control) is cropped from the mirror. Labels the screen paints into its
//!   background art are not controls, so a screen that relies on them needs a spec.
use super::model::{Class, Control, What};
use super::{actions, local, Screen};
use crate::engine::dialogs;
use crate::engine::shellui::Kind;
use crate::gfx::{fade, Weight, R};
use crate::rt::win::*;
use crate::widgets::{Align, Ui};
use alloc::vec::Vec;

/// What the pointer did last on a dialog: where it was (design coordinates) and which
/// window took a press, which then gets the moves and the release as if it had
/// captured the mouse.
struct Track {
    dlg: HWND,
    last: Option<(i32, i32)>,
    press: [Option<HWND>; 2],
    /// The stock edit the player clicked into, for typing when the keyboard focus is
    /// not in the dialog.
    edit: HWND,
}

static mut TRACKS: Vec<Track> = Vec::new();

fn track(dlg: HWND) -> &'static mut Track {
    let v = unsafe { &mut *core::ptr::addr_of_mut!(TRACKS) };
    if let Some(i) = v.iter().position(|t| t.dlg == dlg) {
        return &mut v[i];
    }
    v.push(Track { dlg, last: None, press: [None, None], edit: core::ptr::null_mut() });
    v.last_mut().unwrap()
}

pub fn forget(dlg: HWND) {
    unsafe { (*core::ptr::addr_of_mut!(TRACKS)).retain(|t| t.dlg != dlg) };
}

/// A window's place in the design space.
fn origin_of(h: HWND) -> (i32, i32) {
    dialogs::design_origin(h)
}

fn is_edit(h: HWND) -> bool {
    let mut b = [0u8; 8];
    let n = unsafe { GetClassNameA(h, b.as_mut_ptr(), 8) };
    n == 4 && b[..4].eq_ignore_ascii_case(b"edit")
}

/// The pointer and the keyboard, given to the hidden dialog as user32 would give them
/// to the stock one: moves to the window under the pointer (or to the one a button went
/// down on, until it comes up), presses and releases, the wheel, and typing into the
/// edit the player clicked into.
pub fn forward_input(ui: &mut Ui, s: &Screen) {
    let dlg = s.slot.hwnd;
    let t = track(dlg);
    let inp = ui.input;
    let Some(p) = inp.mouse else { return };
    let (dx, dy) = s.fit.design(p);
    let (ox, oy) = (s.model.rect[0], s.model.rect[1]);
    let (lx, ly) = (dx - ox, dy - oy);
    let keys = (if inp.down[0] { MK_LBUTTON } else { 0 }) | (if inp.down[1] { MK_RBUTTON } else { 0 });
    let at = |h: HWND| {
        let (hx, hy) = origin_of(h);
        (dx - hx, dy - hy)
    };
    let held = t.press[0].or(t.press[1]);
    if t.last != Some((dx, dy)) {
        t.last = Some((dx, dy));
        let (h, x, y) = match held {
            Some(h) => {
                let (x, y) = at(h);
                (h, x, y)
            }
            None => dialogs::hit(dlg, lx, ly),
        };
        actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys, x, y });
    }
    for b in 0..2 {
        let (down, up, dbl) = [(WM_LBUTTONDOWN, WM_LBUTTONUP, WM_LBUTTONDBLCLK), (WM_RBUTTONDOWN, WM_RBUTTONUP, 0x206)][b];
        if inp.pressed[b] {
            let (h, x, y) = dialogs::hit(dlg, lx, ly);
            let msg = if inp.double[b] { dbl } else { down };
            actions::push(actions::Action::Mouse { h, msg, keys, x, y });
            t.press[b] = Some(h);
            if b == 0 {
                t.edit = if is_edit(h) { h } else { core::ptr::null_mut() };
            }
        }
        if inp.released[b] {
            if let Some(h) = t.press[b].take() {
                let (x, y) = at(h);
                actions::push(actions::Action::Mouse { h, msg: up, keys, x, y });
            }
        }
    }
    if inp.wheel != 0.0 {
        let (h, _, _) = dialogs::hit(dlg, lx, ly);
        let d = (inp.wheel * 120.0) as i32 as u32;
        let mut sp = Point { x: p.0 as i32, y: p.1 as i32 };
        unsafe { ClientToScreen(crate::input::window(), &mut sp) };
        let lp = ((sp.y as u32 & 0xffff) << 16 | (sp.x as u32 & 0xffff)) as i32;
        actions::push(actions::Action::Mouse { h, msg: WM_MOUSEWHEEL, keys: ((d & 0xffff) << 16) as usize | keys, x: lp & 0xffff, y: lp >> 16 });
    }
    // Typing: with the keyboard focus in the dialog, user32 gives the keys to the edit
    // itself (the host's pump); otherwise they reached the 3D window and are sent on.
    let focus = unsafe { GetFocus() };
    if !t.edit.is_null() && unsafe { IsWindow(t.edit) } != 0 && !dialogs::in_hosted(focus) {
        for &vk in inp.keys() {
            // Left, right, home, end, delete: the edit takes them as keys.
            if matches!(vk, 0x25 | 0x27 | 0x24 | 0x23 | 0x2e) {
                actions::push(actions::Action::Key { h: t.edit, msg: WM_KEYDOWN, wp: vk as usize });
            }
        }
        for &c in inp.chars() {
            if c != '\r' && c != '\u{1b}' && (c as u32) < 256 {
                actions::push(actions::Action::Key { h: t.edit, msg: WM_CHAR, wp: c as usize });
            }
        }
    }
}

/// The dialog as the game paints it.
pub fn mirrored(ui: &mut Ui, s: &Screen) {
    let r = s.fit.rect(s.model.rect);
    if let Some(t) = s.mirror {
        let (w, h) = (s.model.rect[2] as f32 / dialogs::DESIGN_W as f32, s.model.rect[3] as f32 / dialogs::DESIGN_H as f32);
        ui.draw.image_uv(t, r, [0.0, 0.0, w, h], 0xffffffff);
    }
    if s.top {
        forward_input(ui, s);
    }
}

fn argb(c: u32) -> u32 {
    c | 0xff00_0000
}

/// The dialog's controls as our widgets.
pub fn widgets(ui: &mut Ui, s: &Screen) {
    let area = s.fit.rect(s.model.rect);
    if s.model.rect[2] >= dialogs::DESIGN_W && !super::backdrop::over_mission() {
        ui.draw.rect_vgrad(ui.screen(), 0xff04070d, 0xff0b1524);
    } else {
        ui.draw.rect_round(area, ui.u(8.0), 0xf0101a2a);
    }
    let fs = s.fit.s;
    for c in s.model.controls.iter() {
        if !c.visible || c.rect[2] <= 0 || c.rect[3] <= 0 {
            continue;
        }
        let r = s.fit.rect(c.rect);
        let id = crate::sfmt!("c{:x}", c.key);
        match &c.what {
            What::Shell(Kind::Label) => {
                let size = (c.rect[3] as f32 * fs * 0.8).min(ui.u(26.0)) / ui.scale;
                let al = match c.align { 0 => Align::Centre, 1 => Align::Right, _ => Align::Left };
                ui.text_in(r, &c.text, size, al, argb(c.colour), Weight::Regular);
            }
            What::Shell(_) if c.text.is_empty() => {
                // A picture button: its own art, as the game draws it now.
                s.mirror_part(ui, c.rect, r, if c.enabled { 0xffffffff } else { 0x80ffffff });
                if s.top && c.enabled && shell_button(ui, &id, r, None) {
                    s.click_control(c);
                }
            }
            What::Shell(k) => {
                let label = if *k == Kind::Toggle && c.checked { crate::sfmt!("{}  [on]", c.text) } else { c.text.clone() };
                if s.top && c.enabled {
                    if shell_button(ui, &id, r, Some(&label)) {
                        s.click_control(c);
                    }
                } else {
                    shell_button_look(ui, r, &label, false, false, c.enabled);
                }
            }
            What::Window(Class::Edit) => {
                if local::text_field(ui, &id, r, &c.text, s.top) {
                    // Focused: what is typed goes to the stock edit.
                    for &vk in ui.input.keys() {
                        if matches!(vk, 0x25 | 0x27 | 0x24 | 0x23 | 0x2e) {
                            actions::push(actions::Action::Key { h: c.hwnd, msg: WM_KEYDOWN, wp: vk as usize });
                        }
                    }
                    for &ch in ui.input.chars() {
                        if ch != '\r' && ch != '\u{1b}' && (ch as u32) < 256 {
                            actions::push(actions::Action::Key { h: c.hwnd, msg: WM_CHAR, wp: ch as usize });
                        }
                    }
                }
            }
            What::Window(Class::List) if !c.items.is_empty() || c.selected < 0 => {
                if let Some((i, dbl)) = local::list(ui, &id, r, &c.items, c.selected, c.top, s.top) {
                    actions::push(actions::Action::Select { h: c.hwnd, combo: false, index: i as i32, double: dbl });
                }
            }
            What::Window(Class::Combo) if !c.items.is_empty() => {
                if let Some(i) = local::dropdown(ui, &id, r, &c.items, c.selected, s.top) {
                    actions::push(actions::Action::Select { h: c.hwnd, combo: true, index: i as i32, double: false });
                }
            }
            What::Window(Class::Button) => {
                let t = if c.checked { crate::sfmt!("{}  [x]", c.text) } else { c.text.clone() };
                if s.top && c.enabled && shell_button(ui, &id, r, Some(&t)) {
                    s.click_control(c);
                }
            }
            What::Window(Class::Pane) | What::Window(Class::Static) => {}
            What::Window(_) => {
                // Owner-drawn and anything else: as the game draws it, clicks passed on.
                s.mirror_part(ui, c.rect, r, 0xffffffff);
                if s.top && c.enabled && shell_button(ui, &id, r, None) {
                    s.click_control(c);
                }
            }
        }
    }
    // The dialog's own hover pictures and sounds follow the pointer.
    if s.top {
        if let Some(p) = ui.input.mouse {
            let t = track(s.slot.hwnd);
            let (dx, dy) = s.fit.design(p);
            if t.last != Some((dx, dy)) {
                t.last = Some((dx, dy));
                let (ox, oy) = (s.model.rect[0], s.model.rect[1]);
                let (h, x, y) = dialogs::hit(s.slot.hwnd, dx - ox, dy - oy);
                actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x, y });
            }
        }
    }
}

/// A button of ours over a shell control: a label, or (None) a frame over a picture.
pub fn shell_button(ui: &mut Ui, id: &str, r: R, label: Option<&str>) -> bool {
    let wid = ui.id(id);
    let (hot, held, clicked) = ui.interact(wid, r);
    match label {
        Some(l) => shell_button_look(ui, r, l, hot, held, true),
        None => {
            if hot {
                let t = ui.theme;
                ui.draw.frame(r, ui.u(2.0).max(1.0), if held { t.accent } else { fade(t.accent, 0.7) });
            }
        }
    }
    clicked
}

pub fn shell_button_look(ui: &mut Ui, r: R, label: &str, hot: bool, held: bool, enabled: bool) {
    let t = ui.theme;
    let bg = if held { t.button_down } else if hot { t.button_hot } else { t.button };
    let rad = ui.u(t.radius * 0.6).min(r.h * 0.3);
    ui.draw.rect_round(r, rad, if hot { t.accent } else { t.panel_edge });
    let e = ui.u(t.edge).max(1.0);
    ui.draw.rect_round(r.inset(e), (rad - e).max(0.0), bg);
    let size = (r.h * 0.6 / ui.scale).clamp(10.0, t.text_size);
    let c = if !enabled { t.text_dim } else if hot { 0xffffffff } else { t.text };
    ui.text_in(r.inset(ui.u(4.0)), label, size, Align::Centre, c, Weight::Bold);
}

/// The model's controls of note, for a spec: the visible shell buttons, in the order
/// the screen made them.
pub fn buttons<'a>(s: &'a Screen) -> impl Iterator<Item = &'a Control> {
    s.model.controls.iter().filter(|c| matches!(c.what, What::Shell(k) if k != Kind::Label) && c.visible)
}
