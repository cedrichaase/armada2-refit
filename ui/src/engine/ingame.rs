//! A mission's frame: where the UI draws over the engine's HUD.
//!
//! Contract (see `crate::surface::frame`): `install()` patches a hook that calls
//! `crate::surface::frame(Context::Mission)` once per presented frame, on the game's
//! thread, inside the engine's scene (between its BeginScene and EndScene) with the
//! back buffer bound, after the engine's own HUD has drawn.
//!
//! The site (README.md, "The mission frame"): `Armada_RenderFrame` draws a mission's
//! frame in one scene: the 3D view (`Armada_RenderAllOurStuff`, render3d's seam), the
//! HUD (`DisplayInterface::RenderAll`, whose action camera draws the scene a second
//! time inside it), the frame rate, the text-entry line and the chat lines
//! (`ChatDisplay::Render`), then ends the scene and presents (`RefreshDisplay`). Its
//! call of `ChatDisplay::Render` is the last thing drawn there and is called once per
//! frame, so the UI goes after it. The chat display exists for the whole of every
//! mission (`Program::GameOpen` makes it) and nowhere else, and `Armada_RenderFrame`
//! runs only from the mission loop, so the front end and the loading screen never
//! reach this hook.
use super::{eng, field, storm3d};
use crate::engine::hook as patch;
use crate::surface::{self, Context};
use crate::syms_core::{field, hook, sym};

// The profile (`ui.core.toml`) names, for the hook `core.mission`: in
// `Armada_RenderFrame`, `call ChatDisplay::Render` (thiscall on `g_chat_display`);
// `ST3D_TrifanBuffer::Flush`, thiscall, draws the 2D quads the device has batched; and
// `ST3D_DeviceDirectX8`'s trifan buffer, which `EndScene` flushes.

/// Frames drawn since the last log line, and when it was written.
static mut FRAMES: u32 = 0;
static mut SINCE: f64 = 0.0;
static mut LAST: f64 = 0.0;
static mut REPORTS: u32 = 0;
/// The windows were logged at a mission frame (`input::log_windows`).
static mut WINDOWS_LOGGED: bool = false;

/// Patches the hook; false (and nothing patched) when the site is not stock.
pub fn install() -> bool {
    if eng().ready(hook::CORE_MISSION).is_err() {
        return false;
    }
    let (site, render) = (eng().addr(sym::CORE_SITE_CHAT), eng().addr(sym::CORE_CHAT_RENDER));
    if !patch::patch_call(site, render, chat_render as *const () as usize) {
        return false;
    }
    crate::log!("cursor: {}", if super::cursor::install() { "can be hidden" } else { "stays the engine's" });
    super::cursor::install_probe();
    super::cursor::install_square();
    true
}

unsafe extern "thiscall" fn chat_render(chat: usize) {
    let stock: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(eng().addr(sym::CORE_CHAT_RENDER)) };
    super::chat::render_stock(chat, stock);
    let dev = field(storm3d(), field::CORE_STORM3D_DEVICES());
    let flush = eng().addr(sym::CORE_TRIFAN_FLUSH);
    if dev == 0 || flush == 0 {
        return;
    }
    // The HUD's sprites and text are batched in the device's trifan buffer until the
    // scene ends; draw them now, or they would land on top of the UI.
    let flush: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(flush) };
    unsafe { flush(dev + field::CORE_DX8_TRIFAN()) };
    count();
    // The render lists, once per mission (`hud::log_render_lists`), and the windows at
    // the first mission frame of a game.
    let first = !unsafe { WINDOWS_LOGGED };
    super::hud::log_render_lists();
    super::palette::log_measure();
    if first {
        unsafe { WINDOWS_LOGGED = true };
        super::input::log_engine_window("the first mission frame");
    }
    // The info panel's close-up: a view of the engine's, drawn before the UI's frame
    // and flushed with it (`engine::closeup`).
    if crate::config::get().hud.info_camera {
        super::closeup::render();
        unsafe { flush(dev + field::CORE_DX8_TRIFAN()) };
    }
    crate::widgets::theme::set_player_race(super::race::mission_race());
    surface::frame(Context::Mission);
    super::closeup::end_frame();
    // The frame as it stands, kept for the in-mission menus to draw over.
    if crate::config::get().menus {
        crate::menus::backdrop::keep_mission_frame();
    }
}

/// Logs the frame rate the hook runs at, a few times after each start, so UI.log
/// shows it fires once per presented frame and when it stops.
fn count() {
    let now = crate::rt::now();
    unsafe {
        if now - LAST > 1.0 {
            crate::log!("mission: frames {} after a gap of {:.1} s", if LAST == 0.0 { "start" } else { "resume" }, now - LAST);
            FRAMES = 0;
            SINCE = now;
            REPORTS = 0;
        }
        LAST = now;
        FRAMES += 1;
        if now - SINCE >= 5.0 {
            if REPORTS < 3 {
                crate::log!("mission: {} UI frames in {:.1} s ({:.1}/s), cursor {:?}",
                    FRAMES, now - SINCE, FRAMES as f64 / (now - SINCE), super::cursor::shape());
                REPORTS += 1;
            }
            FRAMES = 0;
            SINCE = now;
        }
    }
}

/// Whether the engine has hidden its HUD (a cinematic, or the player hiding it): the info
/// panel's own hidden flag, which `DisplayInterface::SetInterfaceState` sets in every mode
/// that hides the HUD. The panel is the one the render lists name (`hud::ship_display`),
/// never the static `shipDisplay`: on Roots, FleetOpsHook constructs a panel of its own and
/// the flag is set in that one (PLAN-FLEETOPS.md, FO-6, decision 64). False while the
/// lists cannot be read or no panel is found.
pub fn hud_hidden() -> bool {
    super::hud::flag_hidden(super::hud::ship_display())
}

/// Whether a mission frame was drawn in the last two seconds: the in-mission menus open
/// from a running mission.
pub fn recent() -> bool {
    let last = unsafe { LAST };
    last != 0.0 && crate::rt::now() - last < 2.0
}
