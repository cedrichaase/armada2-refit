//! A tooltip: after the pointer rests on a rectangle for a moment, a small panel with
//! text follows it, kept on the screen. Drawn in the overlay layer.
use super::state::persist;
use super::Ui;
use crate::gfx::R;
use alloc::string::String;

/// Seconds the pointer must rest before the tooltip shows.
const DELAY: f64 = 0.5;

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
        let shown = now - p.tip_since - DELAY;
        self.overlay(move |ui| {
            let (w, h) = super::theme::tooltip_size(ui, &text, ui.u(360.0));
            let mut x = mx + ui.u(14.0);
            let mut y = my + ui.u(24.0);
            if x + w > ui.w - ui.u(4.0) {
                x = (ui.w - w - ui.u(4.0)).max(0.0);
            }
            if y + h > ui.h - ui.u(4.0) {
                y = (my - h - ui.u(8.0)).max(0.0);
            }
            // Fades in over a tenth of a second (STYLE.md, "Motion").
            let a = ((shown / 0.1) as f32).clamp(0.0, 1.0);
            super::theme::tooltip(ui, R::new(libm::roundf(x), libm::roundf(y), w, h), &text, a);
        });
    }
}
