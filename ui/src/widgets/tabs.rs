//! A strip of tabs; the caller draws the page of the chosen one.
use super::focus::*;
use super::Ui;
use crate::gfx::R;

impl<'a> Ui<'a> {
    /// Tabs along `r` (its height is the strip's), choosing `sel`; true on the frame it
    /// changes. Left and Right change it while it has the focus.
    pub fn tabs(&mut self, r: R, label: &str, names: &[&str], sel: &mut usize) -> bool {
        if names.is_empty() {
            return false;
        }
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let mut changed = false;
        let n = names.len();
        *sel = (*sel).min(n - 1);
        if focused && self.keys_free() {
            let keys = self.input.keys;
            for &k in &keys[..self.input.nkeys as usize] {
                let to = match k {
                    VK_LEFT => sel.saturating_sub(1),
                    VK_RIGHT => (*sel + 1).min(n - 1),
                    _ => *sel,
                };
                if to != *sel {
                    *sel = to;
                    changed = true;
                }
            }
        }
        let gap = self.u(4.0);
        let tw = ((r.w - gap * (n as f32 - 1.0)) / n as f32).min(self.u(220.0));
        let line = self.u(t.edge).max(1.0);
        if t.form != super::theme::Form::Lcars {
            self.draw.rect(R::new(r.x, r.bottom() - line, r.w, line), t.panel_edge);
        }
        self.draw.push_clip(r.inset(-self.u(8.0)));
        for (i, name) in names.iter().enumerate() {
            let tab = R::new(r.x + i as f32 * (tw + gap), r.y, tw, r.h);
            let tid = self.sub_id(id, name);
            let (hot, held, clicked) = self.interact(tid, tab);
            if clicked {
                self.focus(id);
                if *sel != i {
                    *sel = i;
                    changed = true;
                }
            }
            let mut st = super::theme::State::of(self, tid, hot, held);
            st.on = *sel == i;
            st.focus = st.on && focused;
            super::theme::tab(self, tab, name, st);
        }
        self.draw.pop_clip();
        changed
    }
}
