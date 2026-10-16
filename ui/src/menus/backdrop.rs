//! What the menus draw over. In the front end the stock dialogs cover the screen
//! themselves (black around the 4:3 design area, or the screen's own art); a menu opened
//! from a mission (the in-mission menu, Save and Load, the Technology Tree) draws over
//! the mission it interrupts: the last frame the mission presented, read back from the
//! front buffer when the first dialog opens, and dimmed.
use crate::d3d::{Device, FMT_A8R8G8B8, POOL_SYSTEMMEM};
use crate::gfx::{self, Tex};
use crate::widgets::Ui;
use core::ffi::c_void;

static mut SHOT: Option<(Tex, u32, u32)> = None;
static mut ACTIVE: bool = false;
/// The texture is kept for the next time: a mission's menus open many times.
static mut POOL: Option<(Tex, u32, u32)> = None;
/// The last picture, and when the dialogs over it ended: an in-mission menu that hands
/// over to another (Options to Load Game goes through the shell's loop between them)
/// keeps the same picture, as no mission frame comes in between.
static mut LAST: Option<(Tex, u32, u32)> = None;
static mut ENDED: f64 = 0.0;

/// `IDirect3DDevice9::GetFrontBufferData` (d3d8to9 is behind the game's device).
const DEV9_GET_FRONT_BUFFER_DATA: usize = 33;

/// The first hosted dialog is opening: keep a picture of the mission, if one runs.
pub fn begin() {
    unsafe {
        ACTIVE = true;
        SHOT = None;
    }
    if !crate::engine::ingame::recent() {
        unsafe {
            if crate::rt::now() - ENDED < 1.5 {
                SHOT = LAST;
            }
        }
        return;
    }
    let Some(dev) = Device::from_d3d8(crate::engine::d3d8_device()) else { return };
    let Some(m) = crate::engine::dev8::display_mode(crate::engine::d3d8_device()) else { return };
    let (w, h) = (m.width, m.height);
    let Some(surf) = dev.create_offscreen_surface(w, h, FMT_A8R8G8B8, POOL_SYSTEMMEM) else { return };
    let hr = vcall!(dev.p(), DEV9_GET_FRONT_BUFFER_DATA, fn(u32, *mut c_void) -> i32, 0, surf.ptr());
    if hr < 0 {
        crate::log!("menus: the mission's last frame could not be read ({:#x}); the menus draw over black", hr as u32);
        return;
    }
    let Some(lr) = surf.surf_lock(0) else { return };
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
    gfx::tex_update(tex, lr.bits as *const u8, lr.pitch as usize, w, h);
    surf.surf_unlock();
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
