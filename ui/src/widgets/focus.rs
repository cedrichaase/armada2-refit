//! Keyboard focus: one widget at a time has it, Tab and Shift+Tab move it between the
//! focusable widgets in the order they were called, Enter and Space activate the
//! focused one, Esc takes the focus away. A click gives the focus only to a widget
//! that takes typing or arrow keys (text field, list, dropdown, spinner); a button,
//! toggle, slider, check box or tab gets it from Tab alone, so a click on one never
//! keeps the keyboard from the engine or Esc from the screen. A focused widget keeps the keyboard from the
//! engine (`claim_keyboard`), so a screen in a mission must never leave focus behind:
//! a click that lands on nothing focusable drops it, and so does a frame in which the
//! focused widget was not called.
//!
//! Tab only starts navigation when something is focused or the screen asked for it
//! with `keyboard_nav` (a mission's Tab is the game's).
use super::state::persist;
use super::{Id, Ui};
use crate::gfx::R;

pub const VK_BACK: u8 = 0x08;
pub const VK_TAB: u8 = 0x09;
pub const VK_RETURN: u8 = 0x0d;
pub const VK_ESCAPE: u8 = 0x1b;
pub const VK_SPACE: u8 = 0x20;
pub const VK_PRIOR: u8 = 0x21;
pub const VK_NEXT: u8 = 0x22;
pub const VK_END: u8 = 0x23;
pub const VK_HOME: u8 = 0x24;
pub const VK_LEFT: u8 = 0x25;
pub const VK_UP: u8 = 0x26;
pub const VK_RIGHT: u8 = 0x27;
pub const VK_DOWN: u8 = 0x28;
pub const VK_DELETE: u8 = 0x2e;

impl<'a> Ui<'a> {
    /// A focusable widget calls this every frame with its id; true when it has the
    /// focus (and then the engine is kept from the keyboard).
    pub fn focus_register(&mut self, id: Id) -> bool {
        self.x.order.push(id.0);
        let f = persist().focus == Some(id) && self.keys_free();
        if f {
            self.claim_keyboard();
        }
        f
    }

    /// Gives `id` the focus (a click on it does).
    pub fn focus(&mut self, id: Id) {
        persist().focus = Some(id);
        self.x.focus_taken = true;
    }

    /// Takes the focus away from whatever has it.
    pub fn blur(&mut self) {
        persist().focus = None;
    }

    pub fn has_focus(&self, id: Id) -> bool {
        persist().focus == Some(id)
    }

    /// This screen has no game keys to protect: Tab focuses its first widget.
    pub fn keyboard_nav(&mut self) {
        self.x.nav = true;
    }

    /// Enter or Space went down this frame (for a focused widget to activate on).
    pub fn activate_pressed(&self) -> bool {
        self.input.key_pressed(VK_RETURN) || self.input.key_pressed(VK_SPACE)
    }

    /// The mark of the focused widget: an accent outline just outside `r`. Drawn
    /// before the widget's own body, which covers its inside.
    pub fn focus_ring(&mut self, r: R, radius: f32) {
        super::theme::focus_ring(self, r, crate::gfx::Corners::round(radius));
    }

    pub(super) fn focus_end(&mut self) {
        let p = persist();
        let mut order = core::mem::take(&mut self.x.order);
        if p.focus.is_some_and(|f| !order.contains(&f.0)) {
            p.focus = None;
        }
        if self.input.pressed[0] && !self.x.focus_taken {
            p.focus = None;
        }
        if !order.is_empty() {
            if self.input.key_pressed(VK_TAB) && (p.focus.is_some() || self.x.nav) {
                let n = order.len();
                let at = p.focus.and_then(|f| order.iter().position(|&o| o == f.0));
                let to = match (at, self.input.shift) {
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                    (None, false) => 0,
                    (None, true) => n - 1,
                };
                p.focus = Some(Id(order[to]));
            } else if self.input.key_pressed(VK_ESCAPE) && p.focus.is_some() {
                p.focus = None;
            }
        }
        order.clear();
        self.x.order = order;
    }
}
