//! The immediate-mode UI: widgets are functions called every frame, which lay
//! themselves out, draw and answer for their input in one go. What must outlive a
//! frame (which widget the pointer is over, which one a button press started on) is
//! kept here by widget id.
//!
//! Sizes are in units of a 1080-pixel-tall screen, times `[UI] Scale`: `Ui::u` turns
//! one into back-buffer pixels, so a layout looks the same at any resolution.
pub mod anim;
pub mod layout;
pub mod theme;

mod checkbox;
mod dropdown;
mod focus;
mod image_button;
mod list;
mod modal;
mod scroll;
mod spinner;
mod state;
mod tabs;
mod text_field;
mod tooltip;

pub use list::ListEvent;
pub use text_field::FieldEvent;

use crate::gfx::{Color, Draw, Weight, R};
use crate::input::Input;
pub use layout::{Anchor, Stack};

pub use theme::{Race, State, Theme};

/// A widget's identity from one frame to the next: a hash of its label and the ids
/// it is nested in.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Id(u32);

fn fnv(seed: u32, s: &str) -> u32 {
    let mut h = seed ^ 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Kept between frames.
#[derive(Default)]
struct Memory {
    /// The widget a button press started on, held until release.
    active: Option<Id>,
    /// Seconds, for animation: advances by the time between frames.
    time: f64,
    last: f64,
}

static mut MEMORY: Memory = Memory { active: None, time: 0.0, last: 0.0 };

pub struct Ui<'a> {
    pub draw: &'a mut Draw,
    pub w: f32,
    pub h: f32,
    pub input: Input,
    pub theme: Theme,
    /// Back-buffer pixels per unit.
    pub scale: f32,
    ids: alloc::vec::Vec<u32>,
    /// The pointer is over something of ours that keeps it from the engine.
    over_ui: bool,
    keyboard: bool,
    hot: Option<Id>,
    /// Deferred drawing, run last by `end` (state.rs).
    overlays: alloc::vec::Vec<alloc::boxed::Box<dyn FnOnce(&mut Ui<'a>)>>,
    /// What the new widgets keep per frame (state.rs).
    x: state::Frame,
}

/// How text sits in a rectangle, horizontally.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Centre,
    Right,
}

impl<'a> Ui<'a> {
    pub fn begin(draw: &'a mut Draw, w: u32, h: u32, input: Input) -> Ui<'a> {
        let cfg_scale = crate::config::get().scale;
        let m = unsafe { &mut *core::ptr::addr_of_mut!(MEMORY) };
        let now = crate::rt::now();
        let dt = if m.last == 0.0 { 0.0 } else { (now - m.last).clamp(0.0, 0.25) };
        m.last = now;
        m.time += dt;
        Ui {
            draw,
            w: w as f32,
            h: h as f32,
            input,
            theme: Theme::default(),
            scale: h as f32 / 1080.0 * cfg_scale,
            ids: alloc::vec::Vec::new(),
            over_ui: false,
            keyboard: false,
            hot: None,
            overlays: alloc::vec::Vec::new(),
            x: state::Frame::default(),
        }
    }

    /// Ends the frame: whether the UI wants the mouse and the keyboard kept from the
    /// engine.
    pub fn end(mut self) -> (bool, bool) {
        state::end_frame(&mut self);
        let m = unsafe { &mut *core::ptr::addr_of_mut!(MEMORY) };
        if !self.input.down[0] {
            m.active = None;
        }
        (self.over_ui || m.active.is_some(), self.keyboard)
    }

    /// Units to back-buffer pixels.
    pub fn u(&self, v: f32) -> f32 {
        v * self.scale
    }

    /// The whole screen.
    pub fn screen(&self) -> R {
        R::new(0.0, 0.0, self.w, self.h)
    }

    /// Seconds since the UI first drew, for animation.
    pub fn time(&self) -> f64 {
        unsafe { (*core::ptr::addr_of!(MEMORY)).time }
    }

    /// An id for a widget labelled `s`, inside the ids pushed so far.
    pub fn id(&self, s: &str) -> Id {
        Id(fnv(self.ids.last().copied().unwrap_or(0), s))
    }

    /// Nests the ids made inside `f` under `s`, so two panels can each have an "OK".
    pub fn scope<T>(&mut self, s: &str, f: impl FnOnce(&mut Self) -> T) -> T {
        let id = self.id(s).0;
        self.ids.push(id);
        let r = f(self);
        self.ids.pop();
        r
    }

    fn mouse_in(&self, r: R) -> bool {
        self.input.mouse.is_some_and(|p| r.contains(p) && self.mouse_live(p))
    }

    /// Marks `r` as ours: while the pointer is over it, the engine does not see the
    /// mouse.
    pub fn claim(&mut self, r: R) {
        if self.mouse_in(r) {
            self.over_ui = true;
        }
    }

    /// Keeps the keyboard from the engine this frame (a text field has focus).
    pub fn claim_keyboard(&mut self) {
        self.keyboard = true;
    }

    /// Hover and click for an interactive rectangle: (hot, held, clicked). A click is
    /// a release over the widget the press started on.
    pub fn interact(&mut self, id: Id, r: R) -> (bool, bool, bool) {
        let m = unsafe { &mut *core::ptr::addr_of_mut!(MEMORY) };
        let over = self.mouse_in(r);
        if over {
            self.over_ui = true;
            self.hot = Some(id);
        }
        if over && self.input.pressed[0] && m.active.is_none() {
            m.active = Some(id);
        }
        let held = m.active == Some(id);
        let clicked = held && over && self.input.released[0];
        (over && (m.active.is_none() || held), held && self.input.down[0], clicked)
    }

    // ---- drawing helpers -----------------------------------------------------------

    /// Text placed in `r`: centred vertically, aligned horizontally; returns its width.
    pub fn text_in(&mut self, r: R, s: &str, size: f32, align: Align, c: Color, w: Weight) -> f32 {
        let px = self.u(size);
        let font = match w {
            Weight::Regular => crate::text::font(),
            Weight::Bold => crate::text::font_bold(),
        };
        let (tw, th) = crate::text::measure_with(font, s, px);
        let x = match align {
            Align::Left => r.x,
            Align::Centre => r.x + (r.w - tw) * 0.5,
            Align::Right => r.right() - tw,
        };
        let y = r.y + (r.h - th) * 0.5;
        self.draw.text_w(s, px, libm::roundf(x), libm::roundf(y), c, w)
    }

    // ---- widgets ---------------------------------------------------------------------

    /// A panel in the theme's form (theme.rs) that keeps the mouse from the engine.
    pub fn panel(&mut self, r: R) {
        theme::panel(self, r);
        self.claim(r);
        crate::hud::layout::note(r);
    }

    /// A panel with a title; returns the room left inside it, padded. Its title
    /// underline sweeps in when it first shows (anim.rs).
    pub fn window(&mut self, r: R, title: &str) -> R {
        let opened = anim::appear(self.id(&crate::sfmt!("window:{}", title)), 0.22, self.time());
        let inner = theme::window(self, r, title, opened);
        self.claim(r);
        inner
    }

    /// A line of text.
    pub fn label(&mut self, r: R, s: &str) {
        let t = self.theme;
        self.text_in(r, s, t.text_size, Align::Left, t.text, Weight::Regular);
    }

    /// A dimmer line of text, smaller.
    pub fn note(&mut self, r: R, s: &str) {
        let t = self.theme;
        self.text_in(r, s, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    }

    /// A push button; true on the frame it is clicked (or activated with Enter or
    /// Space while it has the focus).
    pub fn button(&mut self, r: R, label: &str) -> bool {
        let id = self.id(label);
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let mut st = State::of(self, id, hot, held);
        st.focus = focused;
        theme::button(self, r, label, st);
        clicked || (focused && self.activate_pressed())
    }

    /// A check box with its label; true on the frame it changes.
    pub fn toggle(&mut self, r: R, label: &str, on: &mut bool) -> bool {
        let id = self.id(label);
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let flip = clicked || (focused && self.activate_pressed());
        if flip {
            *on = !*on;
        }
        let mut st = State::of(self, id, hot, held);
        st.focus = focused;
        let bw = theme::check(self, r, *on, st);
        let t = self.theme;
        let gap = self.u(t.pad * 0.7);
        let col = theme::row_text(self, State { on: false, ..st });
        self.text_in(R::new(r.x + bw + gap, r.y, r.w - bw - gap, r.h), label, t.text_size, Align::Left, col, Weight::Regular);
        flip
    }

    /// A horizontal slider over `lo..hi`; true while it changes `v`. Left and Right
    /// step it by a twentieth while it has the focus.
    pub fn slider(&mut self, r: R, label: &str, v: &mut f32, lo: f32, hi: f32) -> bool {
        let id = self.id(label);
        let focused = self.focus_register(id);
        let (hot, held, _) = self.interact(id, r);
        let mut changed = false;
        if held {
            if let Some((mx, _)) = self.input.mouse {
                let f = ((mx - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
                let nv = lo + (hi - lo) * f;
                changed = nv != *v;
                *v = nv;
            }
        }
        if focused {
            let step = (hi - lo) / 20.0;
            let input = self.input;
            for &k in input.keys() {
                let nv = match k {
                    focus::VK_LEFT => (*v - step).max(lo.min(hi)),
                    focus::VK_RIGHT => (*v + step).min(hi.max(lo)),
                    _ => *v,
                };
                changed |= nv != *v;
                *v = nv;
            }
        }
        let f = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0);
        let mut st = State::of(self, id, hot, held);
        st.focus = focused;
        theme::slider(self, r, f, st);
        changed
    }

    /// A bar filled to `f` (0..1).
    pub fn progress(&mut self, r: R, f: f32) {
        theme::bar(self, r, f, None);
    }
}
