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
//! exact lookup.
//!
//! **The raise** (`[Window]`, `window.rs`; ROADMAP phase 9): the same six calls are where
//! the shell's and the intro's modes are raised to the desktop's, in forward mode too (the
//! calls are then re-pointed from `Direct3DCreate8`, and the lookup is the real chain's
//! alone). The front end's 800x600x16 (the first two calls) is raised while a plugin
//! claims `CLAIM_MENUS` (the UI's menu host, which draws the shell's dialogs at the
//! screen's size), the intro's 640x480 (the last two) under `RaiseIntro=1`: the request
//! is replaced by the desktop's size at 32 bits before the lookup, and the engine's own
//! request is the fallback when that mode is not listed. The UI and the cutscene proxy
//! used to patch the pushes before these calls instead; with Display8 the engine's
//! pushes stay stock, and a patched push (the proxy's, without Display8's raise, or
//! `Menus.asi`'s) simply asks for the desktop's mode itself and matches.
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
static mut RAISED: u32 = 0;
/// `FindDisplayMode`, as the engine's own lookup is called from ours.
static mut REAL: usize = 0;

/// Re-points the six calls (once; at `Direct3DCreate8`, before the engine enumerates or
/// asks for a mode), when `display.modes` is ready: `FindDisplayMode`'s
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
    if super::active() {
        log!("stub: the engine's own mode requests ({} of 6 calls) get the one mode offered when theirs is not listed", n);
    } else {
        log!("window: the engine's own mode requests ({} of 6 calls) are raised to the desktop's where asked", n);
    }
}

/// The front end's own request (`SetActiveDisplay_Internal`'s hard-coded mode), named
/// in the log when it is answered.
fn front_end(w: i32, h: i32, bpp: i32) -> bool {
    (w, h, bpp) == (value::FRONT_END_WIDTH, value::FRONT_END_HEIGHT, value::FRONT_END_DEPTH)
}

unsafe extern "thiscall" fn find(dev: usize, w: i32, h: i32, bpp: i32) -> usize {
    let real: Find = unsafe { core::mem::transmute(REAL) };
    if dev != 0 {
        // After a bail-out no plugin has a device, so the UI's menu host refuses every
        // dialog and the shell draws stock, 800x600: its mode is not raised, whatever the
        // host claims (it claims the menus from load, before a device is known).
        let menus = || !crate::bail::taken() && crate::claims::menus();
        if let Some((rw, rh, rb)) = crate::window::raise(w, h, bpp, menus) {
            let m = unsafe { real(dev, rw, rh, rb) };
            unsafe {
                if m != 0 && RAISED < 8 {
                    RAISED += 1;
                    if front_end(w, h, bpp) {
                        log!("window: shell mode raised to {}x{} (MENUS claimed); the engine asked for {}x{}x{}", rw, rh, w, h, bpp);
                    } else {
                        log!("window: intro mode raised to {}x{}; the engine asked for {}x{}x{}", rw, rh, w, h, bpp);
                    }
                }
            }
            if m != 0 {
                return m;
            }
        }
    }
    let m = unsafe { real(dev, w, h, bpp) };
    if m != 0 || dev == 0 || !super::active() {
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
