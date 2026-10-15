//! Input for the UI: the mouse and keyboard as the UI sees them this frame, and
//! capture, which keeps what the UI takes from reaching the engine.
//!
//! Positions are in back-buffer pixels. `poll()` is called once per frame by
//! `crate::surface::frame` before anything is laid out.

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    /// The pointer in back-buffer pixels; None when it is outside the game window.
    pub mouse: Option<(f32, f32)>,
    /// Buttons held now: left, right, middle.
    pub down: [bool; 3],
    /// Went down / came up since the last poll.
    pub pressed: [bool; 3],
    pub released: [bool; 3],
    /// Wheel notches since the last poll, positive away from the player.
    pub wheel: f32,
}

static mut STATE: Input = Input { mouse: None, down: [false; 3], pressed: [false; 3], released: [false; 3], wheel: 0.0 };

/// Installs whatever input capture needs (a window subclass, an engine hook).
pub fn install() -> bool {
    false
}

/// Reads this frame's input. `w`, `h`: the back buffer's size.
pub fn poll(_w: u32, _h: u32) {}

#[allow(static_mut_refs)]
pub fn state() -> Input {
    unsafe { STATE }
}

/// Whether the engine should be kept from the mouse (the pointer is over UI of ours)
/// and from the keyboard (a text field of ours has focus). Set each frame by the UI
/// after layout; read by the capture hooks.
pub fn set_capture(_mouse: bool, _keyboard: bool) {}
