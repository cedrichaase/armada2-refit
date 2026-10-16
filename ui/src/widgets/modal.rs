//! A modal: dims the screen and takes all input until it is closed. The body is the
//! caller's closure; its widgets see the pointer and keys, and nothing else does.
//!
//! Call it last in the frame: it draws where it is called, so what is drawn after it
//! would land on the dimming. A modal covers the screen for the widgets called before
//! it in the same frame only from the next frame (they were called first), which is
//! the frame after the click that opened it.
use super::focus::VK_ESCAPE;
use super::{Anchor, Ui};
use crate::gfx::R;

impl<'a> Ui<'a> {
    /// A `w` x `h` unit window titled `title` over a dimmed screen; `body` gets the room
    /// inside it. True when Esc was pressed (the caller closes it, or not).
    pub fn modal(&mut self, title: &str, w: f32, h: f32, body: impl FnOnce(&mut Ui<'a>, R)) -> bool {
        self.set_modal();
        let screen = self.screen();
        self.occlude(screen);
        self.claim_overlay(screen);
        self.claim_keyboard();
        // Only the modal's own widgets take part in Tab navigation.
        self.x.order.clear();
        self.x.nav = true;
        self.draw.rect(screen, 0xb0000000);
        let r = Anchor::Centre.place(screen, self.u(w), self.u(h), 0.0);
        let inner = self.window(r, title);
        let was = core::mem::replace(&mut self.x.in_overlay, true);
        let prev_clip = self.x.input_clip.take();
        body(self, inner);
        self.x.input_clip = prev_clip;
        self.x.in_overlay = was;
        self.input.key_pressed(VK_ESCAPE)
    }
}
