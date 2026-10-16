//! Check boxes and radio buttons, focusable (Space or Enter flips or picks the focused
//! one).
use super::Ui;
use crate::gfx::{Weight, R};

impl<'a> Ui<'a> {
    /// A check box with its label; true on the frame it changes.
    pub fn checkbox(&mut self, r: R, label: &str, on: &mut bool) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let mut flip = clicked;
        if clicked {
            self.focus(id);
        }
        if focused && self.activate_pressed() {
            flip = true;
        }
        if flip {
            *on = !*on;
        }
        let bs = r.h.min(self.u(t.text_size + 2.0));
        let b = R::new(r.x, r.y + (r.h - bs) * 0.5, bs, bs);
        let rad = self.u(3.0);
        let e = self.u(t.edge).max(1.0);
        if focused {
            self.focus_ring(b, rad);
        }
        self.draw.rect_round(b, rad, if hot || held { t.accent } else { t.panel_edge });
        self.draw.rect_round(b.inset(e), (rad - e).max(0.0), t.track);
        if *on {
            let w = self.u(2.5).max(1.5);
            let p = |fx: f32, fy: f32| (b.x + b.w * fx, b.y + b.h * fy);
            self.draw.line(p(0.24, 0.52), p(0.43, 0.72), w, t.accent);
            self.draw.line(p(0.43, 0.72), p(0.78, 0.30), w, t.accent);
        }
        let gap = self.u(t.pad * 0.6);
        self.text_in(R::new(b.right() + gap, r.y, r.w - bs - gap, r.h), label, t.text_size, super::Align::Left, t.text, Weight::Regular);
        flip
    }

    /// A radio button choosing `value` in `sel`; true on the frame it is picked.
    pub fn radio(&mut self, r: R, label: &str, sel: &mut usize, value: usize) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let mut pick = clicked;
        if clicked {
            self.focus(id);
        }
        if focused && self.activate_pressed() {
            pick = true;
        }
        let picked = pick && *sel != value;
        if pick {
            *sel = value;
        }
        let bs = r.h.min(self.u(t.text_size + 2.0));
        let b = R::new(r.x, r.y + (r.h - bs) * 0.5, bs, bs);
        let e = self.u(t.edge).max(1.0);
        if focused {
            self.focus_ring(b, bs * 0.5);
        }
        self.draw.rect_round(b, bs * 0.5, if hot || held { t.accent } else { t.panel_edge });
        self.draw.rect_round(b.inset(e), bs * 0.5, t.track);
        if *sel == value {
            self.draw.rect_round(b.inset(bs * 0.28), bs * 0.5, t.accent);
        }
        let gap = self.u(t.pad * 0.6);
        self.text_in(R::new(b.right() + gap, r.y, r.w - bs - gap, r.h), label, t.text_size, super::Align::Left, t.text, Weight::Regular);
        picked
    }

    /// A group of radio buttons down `r`, one per item, choosing `sel`; true on the
    /// frame the choice changes.
    pub fn radio_group(&mut self, r: R, label: &str, items: &[&str], sel: &mut usize) -> bool {
        if items.is_empty() {
            return false;
        }
        let mut changed = false;
        let h = r.h / items.len() as f32;
        self.scope(label, |ui| {
            for (i, name) in items.iter().enumerate() {
                changed |= ui.radio(R::new(r.x, r.y + i as f32 * h, r.w, h), name, sel, i);
            }
        });
        changed
    }
}
