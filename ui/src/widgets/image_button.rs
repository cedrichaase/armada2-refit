//! A button that shows a picture: a sprite of the game's art, or any texture.
use super::Ui;
use crate::gfx::{Tex, R};

impl<'a> Ui<'a> {
    /// A button in `r` showing the part `uv` (u0 v0 u1 v1) of `tex`, inset by a little
    /// and kept square-ish by the caller's `r`. `label` names it (its id); true on the
    /// frame it is clicked. Focusable like a button would be: Enter or Space click it.
    pub fn image_button(&mut self, r: R, label: &str, tex: Tex, uv: [f32; 4]) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let focused = self.focus_register(id);
        let (hot, held, clicked) = self.interact(id, r);
        let click = clicked || (focused && self.activate_pressed());
        let e = self.u(t.edge).max(1.0);
        let mut st = super::theme::State::of(self, id, hot, held);
        st.focus = focused;
        super::theme::button_frame(self, r, st);
        let pad = self.u(t.pad * 0.5);
        let mut img = r.inset(e + pad);
        if held {
            img.y += self.u(1.0);
        }
        self.draw.image_uv(tex, img, uv, if hot { 0xffffffff } else { 0xffd8d8d8 });
        click
    }
}
