//! Movies through the UI (README.md, "Movies"): the cutscene proxy
//! (`cutscenes/binkproxy`) hands each frame of a full-screen movie to
//! `ui_present_movie`, and the UI draws it fitted to the screen, centred between black
//! bars, at the next `Present` of the engine's device (the one `PlayMovie` makes after
//! the copy), in place of the proxy's own copy into the locked back buffer.
//!
//! The frame is copied when it is handed over (the proxy's buffer is its own), and is
//! drawn once: a frame the engine never presents is replaced by the next.
use crate::gfx::{Tex, R};
use alloc::vec::Vec;

struct Pending {
    px: Vec<u8>,
    w: u32,
    h: u32,
}

static mut PENDING: Option<Pending> = None;
/// The texture frames are drawn from, and its size (a new one when the size changes).
static mut TEX: Option<(Tex, u32, u32)> = None;
static mut FRAMES: u32 = 0;

/// Whether the UI draws movies (`[UI] Movies`; the device is checked by the caller).
fn on() -> bool {
    crate::config::try_get().is_some_and(|c| c.movies)
}

/// A frame of a full-screen movie: `w` x `h` pixels of BGRX (the fourth byte
/// ignored), rows `pitch` bytes apart, top-down. Returns 1 when the UI will draw it at
/// the next present, 0 when the caller must draw it itself (no UI device, or
/// `Movies=0`).
#[unsafe(no_mangle)]
pub extern "C" fn ui_present_movie(bgra: *const u8, w: i32, h: i32, pitch: i32) -> i32 {
    if !on() || bgra.is_null() || w < 2 || h < 2 || w > 8192 || h > 8192 || pitch < w * 4 {
        return 0;
    }
    // The device, attached, so its Present is chained (the intro plays before any
    // frame of UI).
    if crate::gfx::attach().is_none() {
        return 0;
    }
    let (w, h, pitch) = (w as u32, h as u32, pitch as usize);
    let row = w as usize * 4;
    let mut px = Vec::with_capacity(row * h as usize);
    for y in 0..h as usize {
        let src = unsafe { core::slice::from_raw_parts(bgra.add(y * pitch), row) };
        px.extend_from_slice(src);
    }
    unsafe {
        if FRAMES == 0 {
            crate::log!("movie: frames drawn by the UI, {}x{}", w, h);
        }
        FRAMES = FRAMES.wrapping_add(1);
        PENDING = Some(Pending { px, w, h });
    }
    1
}

/// At a present of the engine's device: the pending frame, if any, drawn over the
/// whole back buffer.
#[allow(static_mut_refs)]
pub fn present(dev8: crate::engine::dev8::Dev8) {
    let Some(p) = (unsafe { PENDING.take() }) else { return };
    let tex = unsafe {
        match TEX {
            Some((t, w, h)) if w == p.w && h == p.h => t,
            _ => {
                let t = crate::gfx::texture_dynamic(p.w, p.h);
                TEX = Some((t, p.w, p.h));
                t
            }
        }
    };
    crate::gfx::tex_update(tex, p.px.as_ptr(), p.w as usize * 4, p.w, p.h);
    // PlayMovie presents outside a scene; one is opened for the draw, and closed only if
    // it was ours.
    let opened = crate::engine::dev8::begin_scene(dev8) >= 0;
    crate::gfx::frame(|bw, bh, draw| {
        let (bw, bh) = (bw as f32, bh as f32);
        draw.rect(R::new(0.0, 0.0, bw, bh), 0xff000000);
        let k = (bw / p.w as f32).min(bh / p.h as f32);
        let (w, h) = (libm::floorf(p.w as f32 * k), libm::floorf(p.h as f32 * k));
        let (x, y) = (libm::floorf((bw - w) * 0.5), libm::floorf((bh - h) * 0.5));
        draw.image(tex, R::new(x, y, w, h), 0xffffffff);
    });
    if opened {
        crate::engine::dev8::end_scene(dev8);
    }
}
