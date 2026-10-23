//! The engine's mouse state, read for the focus diagnostics in `input.rs` (README.md,
//! "Focus and the pointer"). Read only: nothing here is written.
//!
//! In a mission the input thread's `MouseDriver::ReadChannels` turns the mouse messages
//! posted to it into button bits, and while a right-drag or a both-buttons drag pans
//! the camera (`cOverViewImp::mMouseRightDrag`, `mMouseBothDown`, through
//! `MouseLockPosition` and `MouseUnlockPosition`) it holds the pointer: each read moves
//! it back to the middle of the window with `SetCursorPos` and takes the motion from
//! there, and the release puts it back where the drag began. `MouseInit`, from
//! `cOverViewImp::PostLoad`, clears both at every mission's start; the window's
//! `WM_ACTIVATE` clears only the keyboard's state.
use super::rd;

/// `MouseDriver`'s button bits, set and cleared from the messages: 1 left, 2 right,
/// 4 middle (higher bits are its click latches).
const BUTTONS: usize = 0x763308;
/// Non-zero while `ReadChannels` holds the pointer in the middle of the window.
const HELD: usize = 0x763350;

/// The buttons the engine believes are down: bit 0 left, 1 right, 2 middle.
pub fn buttons() -> u32 {
    unsafe { rd::<u32>(BUTTONS) & 7 }
}

/// Whether the engine is holding the pointer in the middle of the window (a pan).
pub fn held() -> bool {
    unsafe { rd::<u32>(HELD) != 0 }
}
