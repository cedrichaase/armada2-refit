//! A tooltip: after the pointer rests on a rectangle for a moment, a small panel with
//! text follows it, kept on the screen. Drawn in the overlay layer.
use super::state::persist;
use super::Ui;
use crate::gfx::R;
use alloc::string::String;
use alloc::vec::Vec;

/// Seconds the pointer must rest before the tooltip shows.
const DELAY: f64 = 0.5;

/// Breaks `text` into lines no wider than `max_w` pixels at `px`, at '\n' and spaces.
fn wrap(text: &str, px: f32, max_w: f32) -> Vec<String> {
    let font = crate::text::font();
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let mut try_ = line.clone();
            if !try_.is_empty() {
                try_.push(' ');
            }
            try_.push_str(word);
            if !line.is_empty() && crate::text::measure_with(font, &try_, px).0 > max_w {
                out.push(core::mem::take(&mut line));
                line.push_str(word);
            } else {
                line = try_;
            }
        }
        out.push(line);
    }
    out
}

impl<'a> Ui<'a> {
    /// Call after a widget with its rectangle: shows `text` once the pointer has rested
    /// on `r` for half a second (not while a button is held).
    pub fn tooltip(&mut self, r: R, text: &str) {
        let id = self.id(&crate::sfmt!("tip{:.0},{:.0}:{}", r.x, r.y, text)).0;
        let p = persist();
        let over = self.mouse_in(r) && !self.input.down[0];
        if !over {
            if p.tip_id == id {
                p.tip_id = 0;
            }
            return;
        }
        let now = self.time();
        if p.tip_id != id {
            p.tip_id = id;
            p.tip_since = now;
            return;
        }
        if now - p.tip_since < DELAY {
            return;
        }
        let Some((mx, my)) = self.input.mouse else { return };
        let text = String::from(text);
        self.overlay(move |ui| {
            let t = ui.theme;
            let px = ui.u(t.text_size * 0.8);
            let pad = ui.u(t.pad * 0.7);
            let lines = wrap(&text, px, ui.u(360.0));
            let font = crate::text::font();
            let lh = crate::text::measure_with(font, "Ag", px).1;
            let w = lines.iter().map(|l| crate::text::measure_with(font, l, px).0).fold(0.0f32, f32::max) + 2.0 * pad;
            let h = lines.len() as f32 * lh + 2.0 * pad;
            let mut x = mx + ui.u(14.0);
            let mut y = my + ui.u(24.0);
            if x + w > ui.w - ui.u(4.0) {
                x = (ui.w - w - ui.u(4.0)).max(0.0);
            }
            if y + h > ui.h - ui.u(4.0) {
                y = (my - h - ui.u(8.0)).max(0.0);
            }
            let b = R::new(x, y, w, h);
            let e = ui.u(t.edge).max(1.0);
            let rad = ui.u(t.radius * 0.7);
            ui.draw.rect_round(b, rad, t.panel_edge);
            ui.draw.rect_round(b.inset(e), (rad - e).max(0.0), crate::gfx::mix(t.track, 0xff000000, 0.2));
            for (i, l) in lines.iter().enumerate() {
                ui.draw.text(l, px, libm::roundf(x + pad), libm::roundf(y + pad + i as f32 * lh), t.text);
            }
        });
    }
}
