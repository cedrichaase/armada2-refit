//! Where a frame of UI happens. Each drawing context's hook (`engine::ingame`,
//! `engine::shell`, `engine::loading`) calls `frame` once per presented frame; this
//! reads input, runs the UI for that context and draws it.

/// Which part of the game is on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Context {
    /// A mission (or Instant Action), over the engine's HUD.
    Mission,
    /// The front end, in a screen of our own.
    Shell,
    /// The loading screen.
    Loading,
    /// A stock shell dialog hosted by `engine::dialogs` (the front end's menus, and the
    /// in-mission menus over the mission).
    Menus,
}

/// Draws a frame of UI for `ctx` into the bound back buffer.
///
/// Called on the game's thread, inside a scene (between BeginScene and EndScene),
/// with the back buffer as render target 0. Saves and restores every device state it
/// touches, so the engine's next call finds the device as it left it. Does nothing
/// without a Direct3D 9 device behind the game's (DXVK's own d3d8, GOG's d3d8to9).
pub fn frame(ctx: Context) {
    crate::gfx::frame(|w, h, draw| {
        crate::input::poll(w, h);
        if ctx == Context::Mission {
            crate::input::keep_clip();
        }
        let mut ui = crate::widgets::Ui::begin(draw, w, h, crate::input::state());
        ui.theme = crate::widgets::Theme::for_context(ctx);
        crate::app::run(ctx, &mut ui);
        // The overlay layer (boxes, dropdowns, tooltips) first: the cursor goes on top.
        ui.flush_overlays();
        crate::cursor::draw(ctx, &mut ui);
        let (mouse, keys) = ui.end();
        crate::input::set_capture(mouse, keys);
        if ctx == Context::Mission {
            // No stock panel takes the mouse under one of ours.
            crate::engine::hud::set_pointer_over_ui(mouse);
        }
    });
}
