//! What the UI claims to draw (gpu/src/display.rs, "Claims"; README.md, "Claims"),
//! registered with Display8 at load and asked by it every frame. A part claims only
//! while it draws: `[Loading] Enable=0`, `[UI] Cursor=0`, `[Menus] Credits=0` or a HUD
//! with no panel of ours claim nothing, and neither does a UI that cannot draw
//! (`gfx::drawing`), so the engine's own draws of that part are drawn by passthrough.
use gpu::display::{CLAIM_CREDITS, CLAIM_CURSOR, CLAIM_HUD, CLAIM_LOADING, CLAIM_SELECTION};

static mut HUD: bool = false;
static mut REGISTERED: bool = false;

/// The HUD's panels of ours are in (`hud::install`).
pub fn set_hud(on: bool) {
    unsafe { HUD = on };
}

/// Registers with Display8 (contract 1.1). Called once at load, before the parts are
/// patched.
pub fn register() {
    let ok = crate::gfx::gpu().register(claims);
    unsafe { REGISTERED = ok };
}

/// Whether the UI is registered with Display8: its claims are asked.
pub fn registered() -> bool {
    unsafe { REGISTERED }
}

/// The callback: what the UI draws now.
pub extern "C" fn claims() -> u32 {
    if !crate::gfx::drawing() {
        return 0;
    }
    let cfg = crate::config::get();
    let mut c = 0;
    if unsafe { HUD } {
        c |= CLAIM_HUD;
    }
    if crate::hud::selection_ours() && !cfg.hud.selection_bubbles {
        c |= CLAIM_SELECTION;
    }
    if crate::engine::loading::own() {
        c |= CLAIM_LOADING;
    }
    if crate::engine::shell::credits_ours() {
        c |= CLAIM_CREDITS;
    }
    if crate::engine::cursor::hiding() {
        c |= CLAIM_CURSOR;
    }
    c
}
