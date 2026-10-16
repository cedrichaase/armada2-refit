//! Check boxes and radio buttons, focusable (Space or Enter flips or picks the focused
//! one).
use super::theme::{self, State};
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
        if focused && self.activate_pressed() {
            flip = true;
        }
        if flip {
            *on = !*on;
        }
        let mut st = State::of(self, id, hot, held);
        st.focus = focused;
        let bs = theme::check(self, r, *on, st);
        let gap = self.u(t.pad * 0.7);
        let col = theme::row_text(self, st);
        self.text_in(R::new(r.x + bs + gap, r.y, r.w - bs - gap, r.h), label, t.text_size, super::Align::Left, col, Weight::Regular);
        flip
    }

    /// A radio button choosing `value` in `sel`; true on the frame it is picked.
    pub fn radio(&mut self, r: R, label: &str, sel: &mut usize, value: usize) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let mut pick = clicked;
        if focused && self.activate_pressed() {
            pick = true;
        }
        let picked = pick && *sel != value;
        if pick {
            *sel = value;
        }
        let mut st = State::of(self, id, hot, held);
        st.focus = focused;
        let bs = theme::radio(self, r, *sel == value, st);
        let gap = self.u(t.pad * 0.7);
        let col = theme::row_text(self, st);
        self.text_in(R::new(r.x + bs + gap, r.y, r.w - bs - gap, r.h), label, t.text_size, super::Align::Left, col, Weight::Regular);
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
