//! A mission's frame: where the UI draws over the engine's HUD.
//!
//! Contract (see `crate::surface::frame`): `install()` patches a hook that calls
//! `crate::surface::frame(Context::Mission)` once per presented frame, on the game's
//! thread, inside the engine's scene (between its BeginScene and EndScene) with the
//! back buffer bound, after the engine's own HUD has drawn.

/// Patches the hook; false (and nothing patched) when the site is not stock.
pub fn install() -> bool {
    false
}
