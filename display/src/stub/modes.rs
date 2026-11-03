//! The engine's own requests for a display mode, answered with the stub's one mode
//! (README.md, "Stub mode", "The engine's hard-coded modes").
//!
//! Storm3D keeps the modes the adapter enumerated in a list on its display device and
//! looks a mode up by width, height and depth (`ST3D_DisplayDevice::FindDisplayMode`),
//! which answers nothing when there is no exact match. Three callers ask for
//! a mode the stub never offers, since it offers the desktop's alone:
//!
//! - `SetActiveDisplay_Internal` (the calls at 0x62c4e0 and 0x62c643): the shell's
//!   hard-coded 800x600x16, and the same mode again as its fallback when opening a
//!   display fails;
//! - `GameConfiguration::InitializeGraphicsSystem` (0x53e7c9, 0x53e7e7): the profile's
//!   mode (`ARMADA.PRF`, 1024x768x32 as GOG ships it, else 800x600x16);
//! - `Program::PlayIntroMovie` (0x4861da, 0x4861f2): the intro's 640x480.
//!
//! With no mode found the device is opened on none, that fails, and the engine's
//! retries end in a stack overflow before the first `CreateDevice` (seen 2026-10-30
//! with no `UI.asi`). Each of those six calls is re-pointed here: the engine's own
//! lookup first, and when it finds nothing, the listed mode of the stub's size. The
//! other callers (the adapter's enumeration, which uses the lookup to skip a mode it
//! has already listed, and Graphics Settings, which offers what is listed) keep the
//! exact lookup. Only in stub mode: the real chain lists the stock modes.
//!
//! The UI raises the shell's mode and the profile's to the desktop's by patching the
//! pushes before two of these calls and the call of `InitializeGraphicsSystem`, so with
//! it the lookup matches and this answers what the engine's would; the two never write
//! the same bytes.
use crate::engine_syms::{hook, sym, value};
use crate::log;
use crate::rt::hook as patch;

/// The six calls, with what each asks for (`display.modes` in `profiles/<build>/display.toml`).
const SITES: [(a2engine::Sym, &str); 6] = [
    (sym::SHELL_DISPLAY_FIND, "the shell's display"),
    (sym::FAILED_DISPLAY_FIND, "a display that failed to open"),
    (sym::GAME_MODE_FIND, "the game's mode (the profile's)"),
    (sym::GAME_MODE_FIND_AGAIN, "the game's mode (the profile's)"),
    (sym::INTRO_MODE_FIND, "the intro movie"),
    (sym::INTRO_MODE_FIND_AGAIN, "the intro movie"),
];

type Find = unsafe extern "thiscall" fn(usize, i32, i32, i32) -> usize;

static mut DONE: bool = false;
static mut LOGGED: u32 = 0;
/// `FindDisplayMode`, as the engine's own lookup is called from ours.
static mut REAL: usize = 0;

/// Re-points the six calls (once; at the stub's `Direct3DCreate8`, before the engine
/// enumerates or asks for a mode), when `display.modes` is ready: `FindDisplayMode`'s
/// first bytes are stock and each call calls it.
pub fn install() {
    unsafe {
        if DONE {
            return;
        }
        DONE = true;
    }
    let e = crate::engine();
    if e.ready(hook::MODES).is_err() {
        log!("stub: the engine's own mode requests are left alone");
        return;
    }
    let real = e.addr(sym::FIND_DISPLAY_MODE);
    unsafe { REAL = real };
    let mut n = 0;
    for &(s, what) in &SITES {
        let site = e.addr(s);
        if patch::patch_call(site, real, find as *const () as usize) {
            n += 1;
        } else {
            log!("stub: the call at {:#x} ({}) could not be written: its mode request is left alone", site, what);
        }
    }
    log!("stub: the engine's own mode requests ({} of 6 calls) get the one mode offered when theirs is not listed", n);
}

/// The front end's own request (`SetActiveDisplay_Internal`'s hard-coded mode), named
/// in the log when it is answered.
fn front_end(w: i32, h: i32, bpp: i32) -> bool {
    (w, h, bpp) == (value::FRONT_END_WIDTH, value::FRONT_END_HEIGHT, value::FRONT_END_DEPTH)
}

unsafe extern "thiscall" fn find(dev: usize, w: i32, h: i32, bpp: i32) -> usize {
    let real: Find = unsafe { core::mem::transmute(REAL) };
    let m = unsafe { real(dev, w, h, bpp) };
    if m != 0 || dev == 0 {
        return m;
    }
    let want = super::stub().mode;
    let mut got = 0usize;
    unsafe {
        let head = *((dev + 0x14) as *const usize);
        if head != 0 {
            let mut node = *(head as *const usize);
            while node != head && node != 0 {
                let mode = *((node + 8) as *const usize);
                if mode != 0 {
                    let (mw, mh, mb) = (*(mode as *const i32), *((mode + 4) as *const i32), *((mode + 8) as *const i32));
                    if mw as u32 == want.width && mh as u32 == want.height && (got == 0 || mb == 32) {
                        got = mode;
                    }
                }
                node = *(node as *const usize);
            }
        }
    }
    unsafe {
        if LOGGED < 16 {
            LOGGED += 1;
            if got != 0 {
                let mb = *((got + 8) as *const i32);
                let whose = if front_end(w, h, bpp) { " (the front end's)" } else { "" };
                log!("stub: the engine asks for {}x{}x{}{}, not listed: given the one mode, {}x{}x{}", w, h, bpp, whose, want.width, want.height, mb);
            } else {
                log!("stub: the engine asks for {}x{}x{}, not listed, and the one mode is not listed either", w, h, bpp);
            }
        }
    }
    got
}
