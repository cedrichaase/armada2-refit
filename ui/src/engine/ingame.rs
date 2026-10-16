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
use super::{field, rd, storm3d};
use crate::engine::hook;
use crate::surface::{self, Context};

/// In `Armada_RenderFrame`: `call ChatDisplay::Render` (thiscall on `g_chat_display`).
const SITE_CHAT: usize = 0x598a16;
/// `ChatDisplay::Render`.
const CHAT_RENDER: usize = 0x51d3c0;
/// `ST3D_TrifanBuffer::Flush`, thiscall: draws the 2D quads the device has batched.
const TRIFAN_FLUSH: usize = 0x621a90;
/// `ST3D_DeviceDirectX8+0xe4`: its trifan buffer, which `EndScene` flushes.
const DEV_TRIFAN: usize = 0xe4;
/// `shipDisplay+0x16` (the HUD's ship panel, `ShipDisplay`): its hidden flag, which
/// `DisplayInterface::SetInterfaceState` sets in every mode that hides the HUD.
const SHIP_DISPLAY_HIDDEN: usize = 0x76397e;

/// Frames drawn since the last log line, and when it was written.
static mut FRAMES: u32 = 0;
static mut SINCE: f64 = 0.0;
static mut LAST: f64 = 0.0;
static mut REPORTS: u32 = 0;

/// Patches the hook; false (and nothing patched) when the site is not stock.
pub fn install() -> bool {
    if hook::call_target(SITE_CHAT) != Some(CHAT_RENDER) {
        crate::log!("mission: the call at {:#x} is not to ChatDisplay::Render", SITE_CHAT);
        return false;
    }
    if !hook::patch_call(SITE_CHAT, CHAT_RENDER, chat_render as *const () as usize) {
        return false;
    }
    crate::log!("cursor: {}", if super::cursor::install() { "can be hidden" } else { "stays the engine's" });
    true
}

unsafe extern "thiscall" fn chat_render(chat: usize) {
    let stock: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(CHAT_RENDER) };
    super::chat::render_stock(chat, stock);
    let dev = field(storm3d(), 0xcc);
    if dev == 0 {
        return;
    }
    // The HUD's sprites and text are batched in the device's trifan buffer until the
    // scene ends; draw them now, or they would land on top of the UI.
    let flush: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(TRIFAN_FLUSH) };
    unsafe { flush(dev + DEV_TRIFAN) };
    count();
    surface::frame(Context::Mission);
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

/// Whether the engine has hidden its HUD (a cinematic, or the player hiding it).
pub fn hud_hidden() -> bool {
    unsafe { rd::<u8>(SHIP_DISPLAY_HIDDEN) != 0 }
}

/// Whether a mission frame was drawn in the last two seconds: the in-mission menus open
/// from a running mission.
pub fn recent() -> bool {
    let last = unsafe { LAST };
    last != 0.0 && crate::rt::now() - last < 2.0
}
