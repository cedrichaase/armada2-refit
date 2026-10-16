//! A strip of tabs; the caller draws the page of the chosen one.
use super::focus::*;
use super::{Align, Ui};
use crate::gfx::{Weight, R};

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
        let rad = self.u(t.radius * 0.7);
        self.draw.rect(R::new(r.x, r.bottom() - line, r.w, line), t.panel_edge);
        self.draw.push_clip(r);
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
            let on = *sel == i;
            // Drawn taller than the strip, so the corners below are clipped away.
            let body = R::new(tab.x, tab.y, tab.w, tab.h + rad * 2.0);
            let (edge, fill) = if on {
                (t.panel_edge, t.button_hot)
            } else {
                (t.panel_edge, if held { t.button_down } else if hot { t.button_hot } else { t.button })
            };
            if on && focused {
                self.focus_ring(R::new(body.x, body.y, body.w, tab.h), rad);
            }
            self.draw.rect_round(body, rad, edge);
            self.draw.rect_round(body.inset(line), (rad - line).max(0.0), fill);
            if on {
                self.draw.rect(R::new(tab.x + line, tab.bottom() - line, tab.w - 2.0 * line, line), t.button_hot);
                self.draw.rect(R::new(tab.x + rad, tab.y + line, tab.w - 2.0 * rad, line * 1.5), t.accent);
            }
            let c = if on { 0xffffffff } else if hot { t.text } else { t.text_dim };
            self.text_in(tab, name, t.text_size, Align::Centre, c, if on { Weight::Bold } else { Weight::Regular });
        }
        self.draw.pop_clip();
        changed
    }
}
