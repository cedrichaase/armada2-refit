//! The game's display mode and pointer clip (`[Window]` in `Display8.ini`; contract C3 of
//! ROADMAP phase 9): Display8 is the only plugin that changes a display-mode request of
//! the engine or the pointer's clip. The top-level window itself (style, position) is
//! set at the first `CreateDevice` (`stub/mod.rs`); `MoveWindow` and `SetWindowPos` of
//! the exe stay with the UI, which moves its hosted child dialogs with them.
//!
//! - **`Native`**: the call of `GameConfiguration::InitializeGraphicsSystem` in `WinMain`
//!   is ours. `GameConfiguration::LoadProfile` has read the mode from `ARMADA.PRF` into
//!   the profile (`UserProfile`) at +0x50 (width), +0x54 (height) and +0x58 (depth), and
//!   `GraphicsSettings::CommandLineOverride` writes a mode given on the command line
//!   into the same fields. Without a profile they hold `GraphicsSettings`' defaults,
//!   800x600x16; GOG's `ARMADA.PRF` ships 1024x768x32. When the profile still holds one
//!   of those two stock modes the desktop's mode goes in first, as a command-line
//!   resolution would. Any other mode, chosen in Graphics Settings, is kept; the game
//!   saves the mode in use into `ARMADA.PRF` on exit, as always.
//! - **The shell and the intro** are answered in `stub/modes.rs`, in the engine's own
//!   mode lookups (`raise`): the shell's 800x600x16 while a plugin claims `CLAIM_MENUS`,
//!   the intro's 640x480 under `RaiseIntro`.
//! - **`KeepClip`**: under Wine, at each `Present`, the pointer's clip is put back as
//!   Wine itself does when the keyboard comes back (`reapply_cursor_clipping`): the clip
//!   rectangle it has, handed to `ClipCursor` again. For a full-screen window that
//!   rectangle is the whole screen, and Wine answers it by clipping the pointer to the
//!   window again if the window covers its monitor, is the foreground one and has the X
//!   input focus, and by nothing otherwise. A screenshot's overlay takes the keyboard
//!   and Wine lets go of its clip; when the keyboard comes back it does not always clip
//!   again, and an unclipped pointer can move onto a desktop bar at the top of the
//!   screen, which takes it from the game. Asked at most every `KEEP_CLIP_MS`, on a
//!   frame where the pointer has moved, while the game's window is the foreground and
//!   active one and the engine neither holds the pointer for a pan nor has a button down.
use crate::engine_syms::{field, hook, sym, value};
use crate::log;
use crate::rt::win::*;
use crate::rt::{hook as patch, Ini};
use crate::settings_table::{WINDOW_KEEPCLIP, WINDOW_NATIVE, WINDOW_RAISEINTRO};
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetCursorPos(p: *mut [i32; 2]) -> i32;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetActiveWindow() -> *mut c_void;
    fn GetClipCursor(r: *mut [i32; 4]) -> i32;
    fn ClipCursor(r: *const [i32; 4]) -> i32;
}

/// How often, at most, the clip is asked for again, in milliseconds.
const KEEP_CLIP_MS: u32 = 500;

static NATIVE: AtomicBool = AtomicBool::new(false);
static KEEP_CLIP: AtomicBool = AtomicBool::new(false);
static RAISE_INTRO: AtomicBool = AtomicBool::new(false);
/// The mode the device was opened at (`display8v1_mode`), 0 until it is.
static MODE_W: AtomicU32 = AtomicU32::new(0);
static MODE_H: AtomicU32 = AtomicU32::new(0);
static HWND_NOW: AtomicU32 = AtomicU32::new(0);

/// The desktop's size, 0 when it is not known.
pub fn desktop() -> (u32, u32) {
    let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
    if w > 0 && h > 0 { (w as u32, h as u32) } else { (0, 0) }
}

/// Reads `[Window]` (the overlay first, `settings_table.rs`) and installs `Native`.
pub fn configure(ini: &Ini) {
    use crate::rt::settings::on;
    NATIVE.store(on(ini, WINDOW_NATIVE), Ordering::Relaxed);
    KEEP_CLIP.store(on(ini, WINDOW_KEEPCLIP), Ordering::Relaxed);
    RAISE_INTRO.store(on(ini, WINDOW_RAISEINTRO), Ordering::Relaxed);
    if KEEP_CLIP.load(Ordering::Relaxed) {
        if on_wine() {
            log!("window: KeepClip on (Wine)");
        } else {
            log!("window: KeepClip has nothing to do without Wine");
            KEEP_CLIP.store(false, Ordering::Relaxed);
        }
    }
    if !NATIVE.load(Ordering::Relaxed) {
        log!("window: Native=0: the game's mode is as set");
        return;
    }
    let e = crate::engine();
    if e.ready(hook::NATIVE).is_err() {
        return;
    }
    let ok = patch::patch_call(e.addr(sym::SITE_INIT_GRAPHICS), e.addr(sym::INIT_GRAPHICS), init as *const () as usize);
    log!("window: {}", if ok { "a stock mode becomes the desktop's" } else { "the call is not stock: the game's mode is as set" });
}

/// Whether the intro's 640x480 is raised (`RaiseIntro`).
pub fn raise_intro() -> bool {
    RAISE_INTRO.load(Ordering::Relaxed)
}

/// What a mode lookup of the engine's is raised to: the desktop's mode at 32 bits, or
/// None to leave the request as it is. `menus` says whether a plugin claims
/// `CLAIM_MENUS`; asked only for the front end's request.
pub fn raise(w: i32, h: i32, bpp: i32, menus: impl FnOnce() -> bool) -> Option<(i32, i32, i32)> {
    let front_end = (w, h, bpp) == (value::FRONT_END_WIDTH(), value::FRONT_END_HEIGHT(), value::FRONT_END_DEPTH());
    let intro = (w, h) == (value::INTRO_WIDTH(), value::INTRO_HEIGHT()) && (bpp == 32 || bpp == 16);
    if !(front_end || intro) {
        return None;
    }
    if intro && !raise_intro() {
        return None;
    }
    if front_end && !menus() {
        return None;
    }
    let (dw, dh) = desktop();
    // The desktop must hold what is asked for: a smaller one keeps the stock mode.
    if dw < w as u32 || dh < h as u32 || (dw, dh) == (w as u32, h as u32) {
        return None;
    }
    Some((dw as i32, dh as i32, 32))
}

/// `InitializeGraphicsSystem()` is thiscall; `cfg` is the profile.
unsafe extern "thiscall" fn init(cfg: usize) -> u8 {
    let up = crate::engine().addr(sym::USER_PROFILE);
    if cfg != 0 && up != 0 && cfg == unsafe { *(up as *const usize) } {
        let rd = |off: usize| unsafe { *((cfg + off) as *const u32) };
        let (w, h) = (rd(field::PROFILE_MODE_W()), rd(field::PROFILE_MODE_H()));
        let (dw, dh) = desktop();
        let stock = [(value::DEFAULT_MODE_W() as u32, value::DEFAULT_MODE_H() as u32), (value::GOG_MODE_W() as u32, value::GOG_MODE_H() as u32)];
        if dw == 0 {
            log!("window: the desktop's size is unknown: the game's mode stays {}x{}", w, h);
        } else if !stock.contains(&(w, h)) {
            log!("window: the game's mode is {}x{}, chosen in Graphics Settings: kept", w, h);
        } else if (w, h) != (dw, dh) {
            unsafe {
                *((cfg + field::PROFILE_MODE_W()) as *mut u32) = dw;
                *((cfg + field::PROFILE_MODE_H()) as *mut u32) = dh;
                *((cfg + field::PROFILE_MODE_BPP()) as *mut u32) = 32;
            }
            log!("window: the game's mode {}x{} (the stock default) -> the desktop's, {}x{}x32", w, h, dw, dh);
        }
    }
    let f: unsafe extern "thiscall" fn(usize) -> u8 = unsafe { core::mem::transmute(crate::engine().addr(sym::INIT_GRAPHICS)) };
    unsafe { f(cfg) }
}

/// The engine's device was opened on `hwnd` at `w` x `h`: `display8v1_mode`'s answer,
/// and the window `KeepClip` watches.
pub fn device_opened(hwnd: *mut c_void, w: u32, h: u32) {
    if w > 0 && h > 0 {
        MODE_W.store(w, Ordering::Relaxed);
        MODE_H.store(h, Ordering::Relaxed);
    }
    HWND_NOW.store(hwnd as usize as u32, Ordering::Relaxed);
}

/// `display8v1_mode` (contract 1.3): the mode the shell and the game run at.
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_mode(w: *mut u32, h: *mut u32) -> bool {
    let (mw, mh) = (MODE_W.load(Ordering::Relaxed), MODE_H.load(Ordering::Relaxed));
    if mw == 0 || mh == 0 || w.is_null() || h.is_null() {
        return false;
    }
    unsafe {
        *w = mw;
        *h = mh;
    }
    true
}

struct Clip {
    wine: Option<bool>,
    last: Option<(i32, i32)>,
    tick: u32,
    count: u32,
}

static mut CLIP: Clip = Clip { wine: None, last: None, tick: 0, count: 0 };

fn on_wine() -> bool {
    let ntdll = unsafe { GetModuleHandleA(c"ntdll.dll".as_ptr() as *const u8) };
    !ntdll.is_null() && unsafe { GetProcAddress(ntdll, c"wine_get_version".as_ptr() as *const u8) } != 0
}

fn engine_u32(s: a2engine::Sym) -> u32 {
    match crate::engine().addr(s) {
        0 => 0,
        a => unsafe { *(a as *const u32) },
    }
}

/// At each `Present` (`api::presented`): `KeepClip`.
#[allow(static_mut_refs)]
pub fn present() {
    if !KEEP_CLIP.load(Ordering::Relaxed) {
        return;
    }
    let c = unsafe { &mut CLIP };
    if !*c.wine.get_or_insert_with(on_wine) {
        // Nothing to do without Wine, ever.
        KEEP_CLIP.store(false, Ordering::Relaxed);
        return;
    }
    let hwnd = HWND_NOW.load(Ordering::Relaxed) as usize as *mut c_void;
    if hwnd.is_null() {
        return;
    }
    let mut p = [0i32; 2];
    if unsafe { GetCursorPos(&mut p) } == 0 {
        return;
    }
    let moved = c.last.is_some_and(|l| l != (p[0], p[1]));
    c.last = Some((p[0], p[1]));
    let now = unsafe { GetTickCount() };
    if !moved || now.wrapping_sub(c.tick) < KEEP_CLIP_MS {
        return;
    }
    if unsafe { GetForegroundWindow() != hwnd || GetActiveWindow() != hwnd } {
        return;
    }
    let e = crate::engine();
    if e.ready(hook::KEEPCLIP).is_err() || engine_u32(sym::MOUSE_BUTTONS) & 7 != 0 || engine_u32(sym::MOUSE_HELD) != 0 {
        return;
    }
    let mut r = [0i32; 4];
    if unsafe { GetClipCursor(&mut r) } == 0 {
        return;
    }
    unsafe { ClipCursor(&r) };
    c.tick = now;
    c.count += 1;
    if c.count == 1 || c.count % 100 == 0 {
        log!("window: the pointer's clip put back ({} times so far)", c.count);
    }
}
