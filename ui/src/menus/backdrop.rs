//! What the menus draw over. In the front end the stock dialogs cover the screen
//! themselves (black around the 4:3 design area, or the screen's own art); a menu opened
//! from a mission (the in-mission menu, Save and Load, the Technology Tree) draws over
//! the mission it interrupts: the last frame the mission presented, dimmed. That frame
//! is ours: at the end of every mission frame the UI copies the back buffer (3D view,
//! HUD and UI) into a target of its own (`keep_mission_frame`), a copy on the GPU. Only
//! when there is none (no mission frame was kept at that size, or the copy failed) is
//! it read back from the front buffer as before (`GetFrontBufferData`).
use crate::gfx::{self, Tex};
use crate::widgets::Ui;

/// The mission's last frame, a target of ours, its size, and the `Tex` that draws it.
static mut KEPT: Option<(gpu::Texture, u32, u32, Tex)> = None;
static mut KEPT_OK: bool = false;
static mut KEPT_SAID: bool = false;

/// At the end of a mission frame: the back buffer copied into our target, for the
/// menus to draw over should one open now.
#[allow(static_mut_refs)]
pub fn keep_mission_frame() {
    let g = gfx::gpu();
    let Some(bb) = g.engine_target() else { return };
    let (w, h) = bb.size();
    if w == 0 || h == 0 {
        return;
    }
    let fmt = bb.desc().format;
    unsafe {
        let fits = matches!(&KEPT, Some((t, kw, kh, _)) if *kw == w && *kh == h && t.desc().format == fmt);
        if !fits {
            let t = g.texture(gpu::TextureDesc::d2(w, h, fmt, gpu::Usage::SAMPLED | gpu::Usage::RENDER_TARGET), gpu::Contents::None);
            let tex = match &KEPT {
                Some((_, _, _, tex)) => *tex,
                None => gfx::texture_external(),
            };
            gfx::tex_set_external(tex, Some(t.clone()), w, h);
            KEPT = Some((t, w, h, tex));
        }
        let Some((t, _, _, _)) = &KEPT else { return };
        let ok = gfx::with_cmd(|cmd| cmd.copy(&bb, None, t, None, gpu::Filter::Point)).unwrap_or(false);
        KEPT_OK = ok;
        if !ok && !KEPT_SAID {
            KEPT_SAID = true;
            crate::log!("menus: the mission's frame could not be copied; the menus read the front buffer");
        }
    }
}

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
    unsafe {
        if let (true, Some((_, kw, kh, tex))) = (KEPT_OK, &*core::ptr::addr_of!(KEPT)) {
            SHOT = Some((*tex, *kw, *kh));
            crate::log!("menus: over the mission's last frame, ours ({}x{})", kw, kh);
            return;
        }
    }
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
