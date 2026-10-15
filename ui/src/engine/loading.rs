//! The loading screen: where the UI draws over it, and how far loading has got.
//!
//! Contract (see `crate::surface::frame`): `install()` patches a hook that calls
//! `crate::surface::frame(Context::Loading)` on every frame the loading screen
//! presents, inside its scene with the back buffer bound.

/// Patches the hook; false (and nothing patched) when the site is not stock.
pub fn install() -> bool {
    false
}

/// How far the current load has got, 0..1; None when no load is running.
pub fn progress() -> Option<f32> {
    None
}
