//! The front-end menus, drawn by the UI: every stock shell screen runs hidden in the
//! dialog host (`engine::dialogs`) and is drawn here, each frame, from the hidden
//! dialog (MENUS.md).
//!
//! - `model`: what a dialog shows, read every frame.
//! - `generic`: any dialog, from its mirror (the picture the game paints) or from its
//!   model as our widgets.
//! - `screens`: a layout of our own for a screen, by name (`screens::SPECS`).
//! - `actions`: what the UI does to a dialog, run after the frame.
//! - `backdrop`: what the menus draw over (the mission, for the in-mission menus).
pub mod actions;
pub mod backdrop;
pub mod generic;
pub mod model;
pub mod screens;

use crate::engine::dialogs::{self, DESIGN_H, DESIGN_W};
use crate::gfx::{self, Tex, R};
use crate::rt::win::HWND;
use crate::widgets::Ui;
use alloc::vec::Vec;

/// Where the 800x600 design space lands on the back buffer: filling its height, centred,
/// at 4:3 (wider screens get bars, or what a screen draws there), as Menus.asi fits it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub x: f32,
    pub y: f32,
    /// Back-buffer pixels per design pixel.
    pub s: f32,
}

impl Fit {
    pub fn of(w: f32, h: f32) -> Fit {
        let s = if w * DESIGN_H as f32 > h * DESIGN_W as f32 { h / DESIGN_H as f32 } else { w / DESIGN_W as f32 };
        Fit { x: libm::floorf((w - DESIGN_W as f32 * s) * 0.5), y: libm::floorf((h - DESIGN_H as f32 * s) * 0.5), s }
    }
    /// A design rectangle (x, y, w, h) on the screen.
    pub fn rect(&self, r: [i32; 4]) -> R {
        R::new(self.x + r[0] as f32 * self.s, self.y + r[1] as f32 * self.s, r[2] as f32 * self.s, r[3] as f32 * self.s)
    }
    /// The design area.
    pub fn area(&self) -> R {
        self.rect([0, 0, DESIGN_W, DESIGN_H])
    }
    /// A screen point in design coordinates.
    pub fn design(&self, p: (f32, f32)) -> (i32, i32) {
        (libm::floorf((p.0 - self.x) / self.s) as i32, libm::floorf((p.1 - self.y) / self.s) as i32)
    }
}

/// Each hosted dialog's mirror texture, by window.
static mut MIRRORS: Vec<(HWND, Tex, u32)> = Vec::new();
static mut LAST_FIT: Option<Fit> = None;
static mut LOGGED: Vec<HWND> = Vec::new();

/// The mirror of `h` (its frame, as `engine::dialogs` composed it), uploaded for this
/// frame. Textures are reused as dialogs come and go.
pub fn mirror(s: &dialogs::Slot) -> Option<Tex> {
    if !s.frame.ok() {
        return None;
    }
    let v = unsafe { &mut *core::ptr::addr_of_mut!(MIRRORS) };
    let i = match v.iter().position(|m| m.0 == s.hwnd) {
        Some(i) => i,
        None => match v.iter().position(|m| m.0.is_null()) {
            Some(i) => {
                v[i].0 = s.hwnd;
                i
            }
            None => {
                v.push((s.hwnd, gfx::texture_dynamic(DESIGN_W as u32, DESIGN_H as u32), 0));
                v.len() - 1
            }
        },
    };
    gfx::tex_update(v[i].1, s.frame.bits, DESIGN_W as usize * 4, DESIGN_W as u32, DESIGN_H as u32);
    Some(v[i].1)
}

/// A hosted dialog ended: its mirror is free for the next.
pub fn closed(h: HWND) {
    let v = unsafe { &mut *core::ptr::addr_of_mut!(MIRRORS) };
    for m in v.iter_mut() {
        if m.0 == h {
            m.0 = core::ptr::null_mut();
        }
    }
    unsafe { (*core::ptr::addr_of_mut!(LOGGED)).retain(|&x| x != h) };
    generic::forget(h);
}

/// Whether a text field of ours has the keyboard (Enter and Esc are then its, not the
/// dialog's).
pub fn wants_keys() -> bool {
    crate::input::keys_captured()
}

/// A frame of the menus: every hosted dialog, outermost first, each by its spec or the
/// generic screen; only the innermost takes input.
pub fn frame(ui: &mut Ui) {
    let fit = Fit::of(ui.w, ui.h);
    unsafe {
        if LAST_FIT != Some(fit) {
            LAST_FIT = Some(fit);
            // The bench maps design coordinates to the screen with this line, as it
            // reads Menus.asi's (testbench/bench/scenario.py, design_to_screen).
            let a = fit.area();
            crate::log!("menus: MoveWindow 800x600 @0,0 -> {}x{} @{},{} (the design space on the screen)", a.w as i32, a.h as i32, a.x as i32, a.y as i32);
        }
    }
    backdrop::draw(ui);
    let all = dialogs::stack();
    // A full-screen dialog hides everything under it.
    let first = all.iter().rposition(|s| s.w() >= DESIGN_W && s.h() >= DESIGN_H).unwrap_or(0);
    let stack = &all[first..];
    let n = stack.len();
    let cfg = crate::config::get();
    for (i, s) in stack.iter().enumerate() {
        let top = i + 1 == n;
        let tex = mirror(s);
        let m = model::read(s);
        let logged = unsafe { &mut *core::ptr::addr_of_mut!(LOGGED) };
        if !logged.contains(&s.hwnd) {
            logged.push(s.hwnd);
            model::log(&m);
        }
        let scr = Screen { slot: s, model: &m, fit, mirror: tex, top };
        ui.scope(&crate::sfmt!("dlg{:p}", s.hwnd), |ui| {
            if cfg.menus_specs {
                if let Some(spec) = screens::find(m.screen) {
                    if (spec.draw)(ui, &scr) {
                        return;
                    }
                }
            }
            if cfg.menus_widgets { generic::widgets(ui, &scr) } else { generic::mirrored(ui, &scr) }
        });
    }
    // The whole screen is the menus': nothing of the engine's is under them.
    ui.claim(ui.screen());
}

/// What a screen is drawn from.
pub struct Screen<'a> {
    pub slot: &'a dialogs::Slot,
    pub model: &'a model::Model,
    pub fit: Fit,
    /// The dialog as the game paints it.
    pub mirror: Option<Tex>,
    /// It takes input (the innermost dialog).
    pub top: bool,
}

impl Screen<'_> {
    /// Draws the part `r` (design coordinates) of the dialog's mirror into `dst`.
    pub fn mirror_part(&self, ui: &mut Ui, r: [i32; 4], dst: R, tint: u32) {
        let Some(t) = self.mirror else { return };
        // The mirror holds the dialog at its own origin.
        let (ox, oy) = (self.model.rect[0], self.model.rect[1]);
        let (u0, v0) = ((r[0] - ox) as f32 / DESIGN_W as f32, (r[1] - oy) as f32 / DESIGN_H as f32);
        let (u1, v1) = (u0 + r[2] as f32 / DESIGN_W as f32, v0 + r[3] as f32 / DESIGN_H as f32);
        ui.draw.image_uv(t, dst, [u0, v0, u1, v1], tint);
    }
    /// A click on a design point of this dialog, sent after the frame.
    pub fn click(&self, x: i32, y: i32) {
        actions::click_design(self.slot.hwnd, x, y);
    }
    /// A click on the middle of a control.
    pub fn click_control(&self, c: &model::Control) {
        self.click(c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2);
    }
}
