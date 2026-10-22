//! What the menus draw over. In the front end the stock dialogs cover the screen
//! themselves (black around the 4:3 design area, or the screen's own art); a menu opened
//! from a mission (the in-mission menu, Save and Load, the Technology Tree) draws over
//! the mission it interrupts: the last frame the mission presented, read back from the
//! front buffer when the first dialog opens, and dimmed.
use crate::gfx::{self, Tex};
use crate::widgets::Ui;

static mut SHOT: Option<(Tex, u32, u32)> = None;
static mut ACTIVE: bool = false;
/// The texture is kept for the next time: a mission's menus open many times.
static mut POOL: Option<(Tex, u32, u32)> = None;
/// The last picture, and when the dialogs over it ended: an in-mission menu that hands
/// over to another (Options to Load Game goes through the shell's loop between them)
/// keeps the same picture, as no mission frame comes in between.
static mut LAST: Option<(Tex, u32, u32)> = None;
static mut ENDED: f64 = 0.0;

/// Screens no mission opens: over them the picture of a mission just left is dropped
/// (aborting one goes Options, the admiral's log, the campaign selection, each within
/// the handover's 1.5 s of the last, and the menu scene stayed off behind them).
const FRONT_END: [&str; 2] = ["main menu", "single player"];

/// The first hosted dialog, `screen` (its name in `engine::dialogs`), is opening: keep
/// a picture of the mission, if one runs.
pub fn begin(screen: &str) {
    unsafe {
        ACTIVE = true;
        SHOT = None;
    }
    if FRONT_END.contains(&screen) {
        unsafe { LAST = None };
        return;
    }
    if !crate::engine::ingame::recent() {
        unsafe {
            if crate::rt::now() - ENDED < 1.5 {
                SHOT = LAST;
            }
        }
        return;
    }
    if gfx::attach().is_none() {
        return;
    }
    let Some(m) = crate::engine::dev8::display_mode(crate::engine::d3d8_device()) else { return };
    let (w, h) = (m.width, m.height);
    let tex = unsafe {
        match POOL {
            Some((t, pw, ph)) if pw == w && ph == h => t,
            _ => {
                let t = gfx::texture_dynamic(w, h);
                POOL = Some((t, w, h));
                t
            }
        }
    };
    let read = gfx::gpu().read_front_buffer(w, h, |px| gfx::tex_update(tex, px.ptr(), px.pitch(), w, h));
    if let Err(hr) = read {
        crate::log!("menus: the mission's last frame could not be read ({:#x}); the menus draw over black", hr as u32);
        return;
    }
    unsafe { SHOT = Some((tex, w, h)) };
    crate::log!("menus: over the mission's last frame ({}x{})", w, h);
}

pub fn end() {
    unsafe {
        ACTIVE = false;
        LAST = SHOT;
        ENDED = crate::rt::now();
        SHOT = None;
    }
}

/// Draws the backdrop: the mission dimmed, when there is one. True when it drew.
pub fn draw(ui: &mut Ui) -> bool {
    let Some((t, _, _)) = (unsafe { SHOT }) else { return false };
    let s = ui.screen();
    ui.draw.image(t, s, 0xff_ff_ff_ff);
    ui.draw.rect(s, 0xa0_00_00_00);
    true
}

pub fn over_mission() -> bool {
    unsafe { SHOT.is_some() && ACTIVE }
}
