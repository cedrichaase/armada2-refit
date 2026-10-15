//! The front end: the shell's menus, and screens of our own run in their place.
//!
//! Contract (see `crate::surface::frame`): a screen of ours runs a loop of its own on
//! the game's thread that pumps messages and, each frame, opens a scene, calls
//! `crate::surface::frame(Context::Shell)` and presents.

/// Patches the hooks; false (and nothing patched) when a site is not stock.
pub fn install() -> bool {
    false
}
