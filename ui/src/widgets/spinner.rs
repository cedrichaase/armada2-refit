//! A number spinner: minus and plus buttons around a value; hold a button to repeat,
//! the wheel steps it (Shift: ten at a time), Up/Down/PageUp/PageDown/Home/End when
//! focused.
use super::focus::*;
use super::{Align, Ui};
use crate::gfx::{Weight, R};

/// Seconds a button is held before it repeats, and between repeats.
const REPEAT_AFTER: f32 = 0.4;
const REPEAT_EVERY: f32 = 0.07;

impl<'a> Ui<'a> {
    /// A spinner over `lo..=hi` in steps of `step`, shown with `decimals` places; true
    /// on the frame the value changes.
    pub fn spinner(&mut self, r: R, label: &str, v: &mut f32, lo: f32, hi: f32, step: f32, decimals: usize) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let st = self.slot(id);
        let old = *v;
        let now = self.time() as f32;
        let bw = r.h;
        let minus = R::new(r.x, r.y, bw, r.h);
        let plus = R::new(r.right() - bw, r.y, bw, r.h);
        let mid = R::new(r.x + bw, r.y, r.w - 2.0 * bw, r.h);
        let mut delta = 0.0f32;
        let rad = self.u(t.radius * 0.7);
        let e = self.u(t.edge).max(1.0);
        let big = if self.input.shift { 10.0 } else { 1.0 };
        if focused {
            self.focus_ring(r, rad);
        }
        for (i, (b, sign, glyph)) in [(minus, -1.0f32, "-"), (plus, 1.0, "+")].into_iter().enumerate() {
            let bid = self.sub_id(id, glyph);
            let (hot, held, _) = self.interact(bid, b);
            let pressed = self.input.pressed[0] && held;
            if pressed {
                self.focus(id);
                st[0] = now;
                st[1] = now;
                delta += sign * step * big;
            } else if held && now - st[0] > REPEAT_AFTER && now - st[1] > REPEAT_EVERY {
                st[1] = now;
                delta += sign * step * big;
            }
            let _ = i;
            self.draw.rect_round(b, rad, if hot { t.accent } else { t.panel_edge });
            self.draw.rect_round(b.inset(e), (rad - e).max(0.0), if held { t.button_down } else if hot { t.button_hot } else { t.button });
            self.text_in(b, glyph, t.text_size * 1.2, Align::Centre, t.text, Weight::Bold);
        }
        if self.mouse_in(r) {
            delta += self.input.wheel * step * big;
        }
        if focused && self.keys_free() {
            let mut to = None;
            for &k in self.input.keys() {
                match k {
                    VK_UP => delta += step,
                    VK_DOWN => delta -= step,
                    VK_PRIOR => delta += step * 10.0,
                    VK_NEXT => delta -= step * 10.0,
                    VK_HOME => to = Some(lo),
                    VK_END => to = Some(hi),
                    _ => {}
                }
            }
            if let Some(x) = to {
                *v = x;
            }
        }
        if delta != 0.0 {
            *v += delta;
        }
        *v = v.clamp(lo, hi);
        // Rounded to the places shown, so repeated steps do not drift.
        let k = libm::powf(10.0, decimals as f32);
        *v = libm::roundf(*v * k) / k;
        self.draw.rect_round(mid, 0.0, t.panel_edge);
        self.draw.rect(mid.inset(e), t.track);
        let s = crate::sfmt!("{:.*}", decimals, *v);
        self.text_in(mid, &s, t.text_size, Align::Centre, t.text, Weight::Regular);
        self.claim(r);
        *v != old
    }
}
