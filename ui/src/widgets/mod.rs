//! The immediate-mode UI: widgets are functions called every frame, which lay
//! themselves out, draw and answer for their input in one go. What must outlive a
//! frame (which widget the pointer is over, which one a button press started on) is
//! kept here by widget id.
//!
//! Sizes are in units of a 1080-pixel-tall screen, times `[UI] Scale`: `Ui::u` turns
//! one into back-buffer pixels, so a layout looks the same at any resolution.
pub mod layout;

use crate::gfx::{fade, mix, Color, Draw, Weight, R};
use crate::input::Input;
pub use layout::{Anchor, Stack};

/// Colours and sizes every widget draws with.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub panel: Color,
    pub panel_edge: Color,
    pub text: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub button: Color,
    pub button_hot: Color,
    pub button_down: Color,
    pub track: Color,
    /// In units.
    pub radius: f32,
    pub edge: f32,
    pub text_size: f32,
    pub pad: f32,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            panel: 0xd80a1220,
            panel_edge: 0xff2c4a6e,
            text: 0xffdce8f5,
            text_dim: 0xff8aa0b8,
            accent: 0xffe8a33a,
            button: 0xff1b2d45,
            button_hot: 0xff28456a,
            button_down: 0xff3a6496,
            track: 0xff101c2c,
            radius: 6.0,
            edge: 1.5,
            text_size: 22.0,
            pad: 12.0,
        }
    }
}

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
        }
    }

    /// Ends the frame: whether the UI wants the mouse and the keyboard kept from the
    /// engine.
    pub fn end(self) -> (bool, bool) {
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
        self.input.mouse.is_some_and(|p| r.contains(p))
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

    /// A panel: a rounded, edged background that keeps the mouse from the engine.
    pub fn panel(&mut self, r: R) {
        let t = self.theme;
        let (rad, e) = (self.u(t.radius), self.u(t.edge).max(1.0));
        self.draw.rect_round(r, rad, t.panel_edge);
        self.draw.rect_round(r.inset(e), (rad - e).max(0.0), t.panel);
        self.claim(r);
    }

    /// A panel with a title bar; returns the room left inside it, padded.
    pub fn window(&mut self, r: R, title: &str) -> R {
        self.panel(r);
        let t = self.theme;
        let bar_h = self.u(t.text_size + 2.0 * t.pad * 0.6);
        let (bar, rest) = r.take_top(bar_h);
        let pad = self.u(t.pad);
        self.draw.rect(R::new(bar.x + pad, bar.bottom() - 1.0, bar.w - 2.0 * pad, self.u(1.0).max(1.0)), fade(t.accent, 0.6));
        self.text_in(R::new(bar.x + pad, bar.y, bar.w - 2.0 * pad, bar.h), title, t.text_size, Align::Left, t.accent, Weight::Bold);
        R::new(rest.x + pad, rest.y + pad, rest.w - 2.0 * pad, rest.h - 2.0 * pad)
    }

    /// A line of text.
    pub fn label(&mut self, r: R, s: &str) {
        let t = self.theme;
        self.text_in(r, s, t.text_size, Align::Left, t.text, Weight::Regular);
    }

    /// A dimmer line of text, smaller.
    pub fn note(&mut self, r: R, s: &str) {
        let t = self.theme;
        self.text_in(r, s, t.text_size * 0.8, Align::Left, t.text_dim, Weight::Regular);
    }

    /// A push button; true on the frame it is clicked.
    pub fn button(&mut self, r: R, label: &str) -> bool {
        let id = self.id(label);
        let (hot, held, clicked) = self.interact(id, r);
        let t = self.theme;
        let bg = if held { t.button_down } else if hot { t.button_hot } else { t.button };
        let rad = self.u(t.radius * 0.7);
        let edge = if hot { t.accent } else { t.panel_edge };
        self.draw.rect_round(r, rad, edge);
        let e = self.u(t.edge).max(1.0);
        self.draw.rect_round(r.inset(e), (rad - e).max(0.0), bg);
        self.text_in(r, label, t.text_size, Align::Centre, if hot { 0xffffffff } else { t.text }, Weight::Bold);
        clicked
    }

    /// A check box with its label; true on the frame it changes.
    pub fn toggle(&mut self, r: R, label: &str, on: &mut bool) -> bool {
        let id = self.id(label);
        let (hot, _, clicked) = self.interact(id, r);
        if clicked {
            *on = !*on;
        }
        let t = self.theme;
        let bs = r.h.min(self.u(t.text_size));
        let b = R::new(r.x, r.y + (r.h - bs) * 0.5, bs, bs);
        let rad = self.u(3.0);
        self.draw.rect_round(b, rad, if hot { t.accent } else { t.panel_edge });
        self.draw.rect_round(b.inset(self.u(t.edge).max(1.0)), rad, t.track);
        if *on {
            self.draw.rect_round(b.inset(bs * 0.25), rad * 0.5, t.accent);
        }
        let gap = self.u(t.pad * 0.6);
        self.text_in(R::new(b.right() + gap, r.y, r.w - bs - gap, r.h), label, t.text_size, Align::Left, t.text, Weight::Regular);
        clicked
    }

    /// A horizontal slider over `lo..hi`; true while it changes `v`.
    pub fn slider(&mut self, r: R, label: &str, v: &mut f32, lo: f32, hi: f32) -> bool {
        let id = self.id(label);
        let (hot, held, _) = self.interact(id, r);
        let t = self.theme;
        let mut changed = false;
        if held {
            if let Some((mx, _)) = self.input.mouse {
                let f = ((mx - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
                let nv = lo + (hi - lo) * f;
                changed = nv != *v;
                *v = nv;
            }
        }
        let f = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0);
        let th = self.u(6.0);
        let track = R::new(r.x, r.y + (r.h - th) * 0.5, r.w, th);
        self.draw.rect_round(track, th * 0.5, t.track);
        self.draw.rect_round(R::new(track.x, track.y, track.w * f, th), th * 0.5, t.accent);
        let k = self.u(if hot || held { 18.0 } else { 16.0 });
        let kx = r.x + r.w * f;
        self.draw.rect_round(R::new(kx - k * 0.5, r.y + (r.h - k) * 0.5, k, k), k * 0.5, if held { 0xffffffff } else { t.text });
        changed
    }

    /// A bar filled to `f` (0..1).
    pub fn progress(&mut self, r: R, f: f32) {
        let t = self.theme;
        let rad = r.h * 0.5;
        self.draw.rect_round(r, rad, t.track);
        let f = f.clamp(0.0, 1.0);
        if f > 0.0 {
            let fill = R::new(r.x, r.y, (r.w * f).max(r.h), r.h);
            self.draw.rect_round(fill, rad, mix(t.accent, 0xffffffff, 0.1));
        }
    }
}
