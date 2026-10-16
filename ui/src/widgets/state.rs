//! What the widgets beyond the first few keep: per-widget state by id, keyboard focus,
//! the deferred overlay layer, and the rectangles that cover what is below them.
//!
//! The overlay layer is how a dropdown's list or a tooltip is drawn over everything:
//! `Ui::overlay` stores a closure that `Ui::end` runs last, after all the screen's own
//! drawing. Input cannot be deferred the same way (a click is answered where the
//! widget is called), so a widget that opens something over others handles that
//! thing's input in its own call and registers the rectangle with `Ui::occlude`; from
//! the next frame on, `mouse_live` keeps widgets underneath from seeing the pointer
//! there. A modal occludes the whole screen and also keeps the keyboard from them.
use super::{Id, Ui};
use crate::gfx::R;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Kept between frames.
pub(super) struct Persist {
    /// Four numbers per widget id, for whatever the widget needs (a caret, a scroll
    /// offset, whether a dropdown is open).
    slots: BTreeMap<u32, [f32; 4]>,
    pub focus: Option<Id>,
    /// Rectangles that cover what is below them: last frame's (read), this frame's
    /// (built).
    pub occ_prev: Vec<R>,
    occ_now: Vec<R>,
    pub modal_prev: bool,
    modal_now: bool,
    /// The widget the tooltip timer runs for, and since when.
    pub tip_id: u32,
    pub tip_since: f64,
}

static mut PERSIST: Persist = Persist {
    slots: BTreeMap::new(),
    focus: None,
    occ_prev: Vec::new(),
    occ_now: Vec::new(),
    modal_prev: false,
    modal_now: false,
    tip_id: 0,
    tip_since: 0.0,
};

pub(super) fn persist() -> &'static mut Persist {
    unsafe { &mut *core::ptr::addr_of_mut!(PERSIST) }
}

/// What the new widgets keep for one frame.
#[derive(Default)]
pub(super) struct Frame {
    /// Running the overlay layer, or a modal's body: the pointer is seen through
    /// whatever is occluded, and by widgets outside the input clip.
    pub in_overlay: bool,
    /// Input is only seen inside this rectangle (a scroll area's view).
    pub input_clip: Option<R>,
    /// The focusable widgets of this frame, in call order.
    pub order: Vec<u32>,
    /// Tab starts keyboard navigation even with nothing focused (a screen with no
    /// game keys to protect).
    pub nav: bool,
    /// A widget took focus this frame.
    pub focus_taken: bool,
    /// Drawing something that takes no input (`Ui::inert_unless`).
    pub inert: bool,
}

impl<'a> Ui<'a> {
    /// Four numbers kept for the widget `id` from frame to frame (zero at first).
    pub fn slot(&self, id: Id) -> &'static mut [f32; 4] {
        persist().slots.entry(id.0).or_insert([0.0; 4])
    }

    /// Draws `f` after everything else this frame, over it: a popup list, a tooltip.
    /// `f` runs with the pointer visible through what is occluded, and may itself call
    /// `overlay`. It owns what it draws with (it cannot borrow the caller's frame).
    pub fn overlay(&mut self, f: impl FnOnce(&mut Ui<'a>) + 'static) {
        self.overlays.push(alloc::boxed::Box::new(f));
    }

    /// Covers `r` for the widgets below it from the next frame on, while this is called
    /// every frame.
    pub fn occlude(&mut self, r: R) {
        persist().occ_now.push(r);
    }

    /// Marks `r` as ours for the engine even though widgets below do not see it
    /// (`claim` honours occlusion, this does not).
    pub(super) fn claim_overlay(&mut self, r: R) {
        if self.input.mouse.is_some_and(|p| r.contains(p)) {
            self.over_ui = true;
        }
    }

    /// Whether the pointer at `p` reaches a widget now: not under an occluder and
    /// inside the input clip, unless this is the overlay layer's own input.
    pub(super) fn mouse_live(&self, p: (f32, f32)) -> bool {
        if self.x.in_overlay {
            return true;
        }
        if self.x.input_clip.is_some_and(|c| !c.contains(p)) {
            return false;
        }
        !persist().occ_prev.iter().any(|o| o.contains(p))
    }

    /// Whether keys reach widgets now (not while a modal is open, except to its own,
    /// and never inside `inert_unless`).
    pub fn keys_free(&self) -> bool {
        !self.x.inert && (self.x.in_overlay || !persist().modal_prev)
    }

    /// Runs `f` with its widgets drawn as usual but seeing neither the pointer nor the
    /// keys, unless `live`: a screen under another one.
    pub fn inert_unless<T>(&mut self, live: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        if live {
            return f(self);
        }
        let (clip, inert) = (self.x.input_clip, self.x.inert);
        self.x.input_clip = Some(R::new(0.0, 0.0, 0.0, 0.0));
        self.x.inert = true;
        let out = f(self);
        self.x.input_clip = clip;
        self.x.inert = inert;
        out
    }

    /// Says a modal is open this frame (modal.rs).
    pub(super) fn set_modal(&mut self) {
        persist().modal_now = true;
    }
}

/// `Ui::end`'s part: the overlay layer, focus navigation, and rolling the occluders.
pub(super) fn end_frame(ui: &mut Ui) {
    ui.x.in_overlay = true;
    ui.x.input_clip = None;
    loop {
        let v = core::mem::take(&mut ui.overlays);
        if v.is_empty() {
            break;
        }
        for f in v {
            f(ui);
        }
    }
    ui.x.in_overlay = false;
    ui.focus_end();
    let p = persist();
    p.occ_prev = core::mem::take(&mut p.occ_now);
    p.modal_prev = core::mem::take(&mut p.modal_now);
}
