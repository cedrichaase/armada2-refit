//! The engine's own requests for a display mode, answered with the stub's one mode
//! (README.md, "Stub mode", "The engine's hard-coded modes").
//!
//! Storm3D keeps the modes the adapter enumerated in a list on its display device and
//! looks a mode up by width, height and depth (`ST3D_DisplayDevice::FindDisplayMode`,
//! 0x626bf0), which answers nothing when there is no exact match. Three callers ask for
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
use crate::log;
use crate::rt::hook;

const FIND_DISPLAY_MODE: usize = 0x626bf0;
const SITES: [(usize, &str); 6] = [
    (0x62c4e0, "the shell's display"),
    (0x62c643, "a display that failed to open"),
    (0x53e7c9, "the game's mode (the profile's)"),
    (0x53e7e7, "the game's mode (the profile's)"),
    (0x4861da, "the intro movie"),
    (0x4861f2, "the intro movie"),
];
/// `FindDisplayMode`'s first bytes: the walk over the list at `this+0x14` (a list
/// node's next at +0, its mode at +8; a mode's width, height and depth at +0, +4, +8).
const FIND_STOCK: [u8; 13] = [0x55, 0x8b, 0xec, 0x8b, 0x51, 0x14, 0x53, 0x56, 0x57, 0x8b, 0x0a, 0x3b, 0xca];

type Find = unsafe extern "thiscall" fn(usize, i32, i32, i32) -> usize;

static mut DONE: bool = false;
static mut LOGGED: u32 = 0;

/// Re-points the six calls (once; at the stub's `Direct3DCreate8`, before the engine
/// enumerates or asks for a mode).
pub fn install() {
    unsafe {
        if DONE {
            return;
        }
        DONE = true;
    }
    if unsafe { core::slice::from_raw_parts(FIND_DISPLAY_MODE as *const u8, FIND_STOCK.len()) } != FIND_STOCK {
        log!("stub: FindDisplayMode at {:#x} is not stock: the engine's own mode requests are left alone", FIND_DISPLAY_MODE);
        return;
    }
    let mut n = 0;
    for &(site, what) in &SITES {
        if hook::patch_call(site, FIND_DISPLAY_MODE, find as *const () as usize) {
            n += 1;
        } else {
            log!("stub: the call at {:#x} ({}) is not stock: its mode request is left alone", site, what);
        }
    }
    log!("stub: the engine's own mode requests ({} of 6 calls) get the one mode offered when theirs is not listed", n);
}

unsafe extern "thiscall" fn find(dev: usize, w: i32, h: i32, bpp: i32) -> usize {
    let real: Find = unsafe { core::mem::transmute(FIND_DISPLAY_MODE) };
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
                log!("stub: the engine asks for {}x{}x{}, not listed: given the one mode, {}x{}x{}", w, h, bpp, want.width, want.height, mb);
            } else {
                log!("stub: the engine asks for {}x{}x{}, not listed, and the one mode is not listed either", w, h, bpp);
            }
        }
    }
    got
}
