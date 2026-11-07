//! One window for the whole game (`[Window] OneWindow`, PLAN-FLEETOPS.md decisions 314 to
//! 319): a game that makes its Direct3D device on a new top-level window at each switch
//! (Fleet Operations: Roots destroys its device window and makes another between its
//! menus, the loading screen and a mission) is given one top-level window of Display8's
//! for the rest of its life, the **host**, and each of its device windows is put inside
//! the host as a child covering it. The swap chain stays on the host, so:
//!
//! - the window manager sees no window close or open at a switch (it only ever sees the
//!   host), and the keyboard focus and the pointer's clip stay with it;
//! - the last picture stays on the screen across the switch (no new surface, no new swap
//!   chain), instead of the black of a window just mapped.
//!
//! The device window keeps its own procedure and receives the mouse (it covers the host)
//! and the keyboard (the host hands its focus on); the host answers activation by giving
//! the focus to the device window and forwards `WM_ACTIVATEAPP` and `WM_CLOSE` to it. It
//! stays enabled: a modal form disables the thread's top-level windows (the host now),
//! which would take the mouse from the device window inside it, so a host disabled is
//! enabled again (the device window's own enabled state is what a game sets).
//!
//! The host is made at the first `CreateDevice` where the build's profile says the game
//! makes a device window at each switch (`from_start`), else at the first `CreateDevice` on
//! another window (`stub::follow_window`); a game that makes its device on one window, as
//! Armada II does, never meets it.
use crate::log;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassA(wc: *const WndClassA) -> u16;
    fn CreateWindowExA(ex: u32, class: *const u8, name: *const u8, style: u32, x: i32, y: i32, w: i32, h: i32, parent: *mut c_void, menu: *mut c_void, inst: *mut c_void, param: *mut c_void) -> *mut c_void;
    fn DefWindowProcA(h: *mut c_void, m: u32, wp: usize, lp: isize) -> isize;
    fn SetParent(h: *mut c_void, parent: *mut c_void) -> *mut c_void;
    fn SetWindowLongA(hwnd: *mut c_void, index: i32, v: i32) -> i32;
    fn SetWindowPos(hwnd: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn SetFocus(h: *mut c_void) -> *mut c_void;
    fn GetFocus() -> *mut c_void;
    fn SetForegroundWindow(h: *mut c_void) -> i32;
    fn EnableWindow(h: *mut c_void, on: i32) -> i32;
    fn PostMessageA(h: *mut c_void, m: u32, wp: usize, lp: isize) -> i32;
    fn SendMessageA(h: *mut c_void, m: u32, wp: usize, lp: isize) -> isize;
    fn IsWindow(h: *mut c_void) -> i32;
    fn GetWindowTextA(h: *mut c_void, s: *mut u8, n: i32) -> i32;
    fn ValidateRect(h: *mut c_void, r: *const c_void) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetTickCount() -> u32;
}

#[repr(C)]
struct WndClassA {
    style: u32,
    proc_: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    inst: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    brush: *mut c_void,
    menu: *const u8,
    name: *const u8,
}

/// The host's window class (other plugins tell the host by it: the UI walks from a form's
/// owner, the host, to the device window inside it).
pub const CLASS: &core::ffi::CStr = c"Display8Host";

const WM_ACTIVATE: u32 = 0x06;
const WM_SETFOCUS: u32 = 0x07;
const WM_ENABLE: u32 = 0x0a;
const WM_CLOSE: u32 = 0x10;
const WM_PAINT: u32 = 0x0f;
const WM_ERASEBKGND: u32 = 0x14;
const WM_ACTIVATEAPP: u32 = 0x1c;
const WM_WINDOWPOSCHANGING: u32 = 0x46;
const SWP_HIDEWINDOW: u32 = 0x80;
/// Ours: enable the host again (posted from `WM_ENABLE`, not done inside it).
const WM_REENABLE: u32 = 0x8000 + 0x3d8;

const WS_POPUP: u32 = 0x8000_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
const SWP_NOZORDER: u32 = 0x4;
const SWP_FRAMECHANGED: u32 = 0x20;
const SWP_SHOWWINDOW: u32 = 0x40;

static ON: AtomicBool = AtomicBool::new(true);
static HOST: AtomicUsize = AtomicUsize::new(0);
static CHILD: AtomicUsize = AtomicUsize::new(0);
static SWITCHES: AtomicUsize = AtomicUsize::new(0);
static HIDE_REFUSED: AtomicBool = AtomicBool::new(false);

/// `WINDOWPOS` (user32's).
#[repr(C)]
struct WindowPos {
    hwnd: *mut c_void,
    after: *mut c_void,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: u32,
}
static SWITCH_AT: AtomicU32 = AtomicU32::new(0);
/// Holding: from a switch until the first frame that drew something.
static HOLDING: AtomicBool = AtomicBool::new(false);
static HELD: AtomicU32 = AtomicU32::new(0);
/// Whether anything was drawn since the last `Present` (an engine draw through the
/// passthrough, or a plugin's pass begun).
static DREW: AtomicBool = AtomicBool::new(false);
/// Whether a plugin's pass began since the last `Present`, and when a frame last had one.
static PASSED: AtomicBool = AtomicBool::new(false);
static LAST_PASS: AtomicU32 = AtomicU32::new(0);
/// Whether this switch's hold ends only at a frame a plugin drew on (plugins drew just
/// before the switch), not at the engine's first draw.
static NEEDS_PASS: AtomicBool = AtomicBool::new(false);
/// How long a switch may hold the picture at most.
const HOLD_MS: u32 = 5000;

/// Whether the engine's scene now is `RefreshDisplay`'s of the cursor alone.
static CURSOR_SCENE: AtomicBool = AtomicBool::new(false);

/// A plugin's pass began on the frame.
pub fn drew() {
    DREW.store(true, Ordering::Relaxed);
    PASSED.store(true, Ordering::Relaxed);
}

/// The passthrough drew one of the engine's draws: something on the frame, unless it is
/// the cursor's scene (over a black frame, the cursor alone is still a black frame).
pub fn engine_drew() {
    if !CURSOR_SCENE.load(Ordering::Relaxed) {
        DREW.store(true, Ordering::Relaxed);
    }
}

/// The engine's `BeginScene`, and whether it is the cursor's scene (`drop::Scene::Cursor`).
pub fn scene_begun(cursor: bool) {
    CURSOR_SCENE.store(cursor, Ordering::Relaxed);
}

/// Frames held between two screens, outside a switch (`between_screens`), and since when.
static BETWEEN: AtomicU32 = AtomicU32::new(0);
static BETWEEN_AT: AtomicU32 = AtomicU32::new(0);
static BETWEEN_RUNS: AtomicU32 = AtomicU32::new(0);
/// How long a picture of the plugins' is held at most when the frames after it are none
/// of theirs.
const BETWEEN_MS: u32 = 5000;

/// Outside a switch, with the host made (the game is one that makes a window per switch):
/// while the plugins are drawing the game's frames, a frame none of them drew on is not
/// shown, for at most `BETWEEN_MS` after the last one they drew; the picture stays.
/// Measured on Roots (PLAN-FLEETOPS.md, decision 403): End Mission keeps its window (the menus' window of
/// the Esc before it), and between the in-mission menu's end and the campaign
/// selection's first frame the engine presents frames with nothing of ours on them, and
/// the campaign selection's first frame takes about a second (the menu scene's first
/// draw after a mission), so a black frame stood on the screen for that second.
fn between_screens(now: u32, passed: bool) -> bool {
    if passed || host().is_null() {
        let n = BETWEEN.swap(0, Ordering::Relaxed);
        if n > 0 {
            let runs = BETWEEN_RUNS.fetch_add(1, Ordering::Relaxed) + 1;
            if runs <= 20 || runs % 50 == 0 {
                log!("window: OneWindow: {} frames between two screens held, the picture kept {} ms", n,
                     now.wrapping_sub(BETWEEN_AT.load(Ordering::Relaxed)));
            }
        }
        return false;
    }
    let last = LAST_PASS.load(Ordering::Relaxed);
    if last == 0 || now.wrapping_sub(last) > BETWEEN_MS {
        return false;
    }
    if BETWEEN.fetch_add(1, Ordering::Relaxed) == 0 {
        BETWEEN_AT.store(now, Ordering::Relaxed);
    }
    true
}

/// At the engine's `Present`: whether to leave the picture on the screen as it is instead
/// of presenting this frame. After a switch (a new device window inside the host) the game
/// presents a few frames with nothing drawn on them, cleared to black, before the next
/// screen draws (measured: two to four within a few milliseconds of the `CreateDevice`,
/// then a second or more with no `Present` while it loads), and the black then stood on
/// the screen. Such frames are not shown until the first frame that drew something, for
/// at most `HOLD_MS`; the last picture stays. Any other frame, and every frame outside a
/// switch, is presented as before. Where plugins drew the frames before the switch, the
/// first frame that counts is one a plugin drew on.
pub fn hold() -> bool {
    let now = unsafe { GetTickCount() };
    let passed = PASSED.swap(false, Ordering::Relaxed);
    if passed {
        LAST_PASS.store(now, Ordering::Relaxed);
    }
    let drew = if NEEDS_PASS.load(Ordering::Relaxed) { DREW.swap(false, Ordering::Relaxed) && passed } else { DREW.swap(false, Ordering::Relaxed) };
    if !HOLDING.load(Ordering::Relaxed) {
        return between_screens(now, passed);
    }
    let ms = now.wrapping_sub(SWITCH_AT.load(Ordering::Relaxed));
    if drew || ms > HOLD_MS {
        HOLDING.store(false, Ordering::Relaxed);
        let n = HELD.swap(0, Ordering::Relaxed);
        if n > 0 {
            log!("window: OneWindow: {} empty frames held after the switch, the picture kept {} ms{}", n, ms,
                 if drew { "" } else { " (the hold's limit)" });
        }
        return false;
    }
    HELD.fetch_add(1, Ordering::Relaxed);
    true
}

pub fn configure(on: bool) {
    ON.store(on, Ordering::Relaxed);
    if !on {
        log!("window: OneWindow=0: the swap chain follows each new device window");
    }
}

/// Whether the host is made at the first `CreateDevice`: the build's profile says the game
/// makes its device on a new window at each switch (`device_windows`, Roots' 1), so the
/// first switch (the menus to the loading screen) is seamless too. Elsewhere the host waits
/// for a second window, which Armada II never makes.
pub fn from_start() -> bool {
    // Decided once, at the first `CreateDevice`, and kept: a host made stays the game's.
    match DECIDED.load(Ordering::Relaxed) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    let yes = decide();
    DECIDED.store(if yes { 2 } else { 1 }, Ordering::Relaxed);
    yes
}

static DECIDED: AtomicU32 = AtomicU32::new(0);

fn decide() -> bool {
    use crate::engine_syms::{hook, value};
    if !ON.load(Ordering::Relaxed) || crate::engine().ready(hook::ONEWINDOW).is_err() || value::DEVICE_WINDOWS() != 1 {
        return false;
    }
    // Only for a plugin that draws the game's screens across its switches (the UI's Roots
    // forms, `CLAIM_WINDOW`, contract 1.5): with the game's own screens (its forms and
    // dialogs, windows of their own over the game's) the host took the pointer from them
    // (PLAN-FLEETOPS.md, decision 405).
    let claimed = crate::claims::ask() & gpu::display::CLAIM_WINDOW != 0;
    if !claimed && !SAID_UNCLAIMED.swap(true, Ordering::Relaxed) {
        log!("window: OneWindow: no plugin draws the game's screens across its windows (no window claim): the swap chain follows each new window");
    }
    claimed
}

static SAID_UNCLAIMED: AtomicBool = AtomicBool::new(false);

/// The host, once made.
pub fn host() -> *mut c_void {
    HOST.load(Ordering::Relaxed) as *mut c_void
}

fn child() -> *mut c_void {
    let c = CHILD.load(Ordering::Relaxed) as *mut c_void;
    if !c.is_null() && unsafe { IsWindow(c) } != 0 { c } else { core::ptr::null_mut() }
}

/// A `CreateDevice` on `hwnd`, another window than the last one's: `hwnd` is put inside
/// the host (made now, the first time, with the old window's title) at `w` x `h`. The
/// window to present to: `Some((host, made))`, or None with `OneWindow=0` or when the
/// host cannot be made (the swap chain then follows `hwnd` as before).
pub fn adopt(hwnd: *mut c_void, w: u32, h: u32) -> Option<(*mut c_void, bool)> {
    if !ON.load(Ordering::Relaxed) || hwnd.is_null() {
        return None;
    }
    let (w, h) = (w as i32, h as i32);
    let mut made = false;
    let mut host = host();
    if host.is_null() {
        let inst = unsafe { GetModuleHandleA(core::ptr::null()) };
        let wc = WndClassA {
            style: 0,
            proc_: host_proc,
            cls_extra: 0,
            wnd_extra: 0,
            inst,
            icon: core::ptr::null_mut(),
            cursor: core::ptr::null_mut(),
            brush: core::ptr::null_mut(),
            menu: core::ptr::null(),
            name: CLASS.as_ptr() as *const u8,
        };
        unsafe { RegisterClassA(&wc) };
        let mut title = [0u8; 128];
        let n = unsafe { GetWindowTextA(hwnd, title.as_mut_ptr(), 127) }.max(0) as usize;
        title[n.min(127)] = 0;
        host = unsafe {
            CreateWindowExA(0, CLASS.as_ptr() as *const u8, title.as_ptr(), WS_POPUP | WS_CLIPSIBLINGS, 0, 0, w, h,
                            core::ptr::null_mut(), core::ptr::null_mut(), inst, core::ptr::null_mut())
        };
        if host.is_null() {
            log!("window: OneWindow: the host window could not be made: the swap chain follows each new window");
            ON.store(false, Ordering::Relaxed);
            return None;
        }
        HOST.store(host as usize, Ordering::Relaxed);
        made = true;
    }
    unsafe {
        // A child: no frame, not a window of its own to the window manager, covering the
        // host. WS_CHILD before SetParent, as user32 asks, and WS_VISIBLE only once it is
        // inside: a style with WS_VISIBLE on a top-level window maps it at once, and the
        // window manager saw it come and go, full screen and black, at every switch.
        SetWindowLongA(hwnd, -16, (WS_CHILD | WS_CLIPSIBLINGS) as i32);
        SetParent(hwnd, host);
        SetWindowPos(hwnd, core::ptr::null_mut(), 0, 0, w, h, SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        if made {
            SetWindowPos(host, core::ptr::null_mut(), 0, 0, w, h, SWP_NOZORDER | SWP_SHOWWINDOW);
        }
        CHILD.store(hwnd as usize, Ordering::Relaxed);
        SetForegroundWindow(host);
        SetFocus(hwnd);
    }
    let n = SWITCHES.fetch_add(1, Ordering::Relaxed) + 1;
    let now = unsafe { GetTickCount() };
    SWITCH_AT.store(now, Ordering::Relaxed);
    // Plugins drew the frames before the switch (the UI, render3d): the next picture is
    // theirs, and a frame of the engine's alone in between (a single draw over black when
    // a mission resumes, while its textures load again) is held as well.
    NEEDS_PASS.store(now.wrapping_sub(LAST_PASS.load(Ordering::Relaxed)) < 1000, Ordering::Relaxed);
    DREW.store(false, Ordering::Relaxed);
    HELD.store(0, Ordering::Relaxed);
    HOLDING.store(true, Ordering::Relaxed);
    if made {
        log!("window: OneWindow: the device window {:p} is inside Display8's window {:p} ({}x{}), which the game keeps from now on", hwnd, host, w, h);
    } else if n <= 20 || n % 50 == 0 {
        log!("window: OneWindow: the new device window {:p} is put inside {:p} ({} so far); the picture stays", hwnd, host, n);
    }
    Some((host, made))
}

unsafe extern "system" fn host_proc(h: *mut c_void, m: u32, wp: usize, lp: isize) -> isize {
    let c = child();
    match m {
        WM_ACTIVATE => {
            // DefWindowProc would put the focus on the host itself.
            if wp & 0xffff != 0 && !c.is_null() && unsafe { GetFocus() } != c {
                unsafe { SetFocus(c) };
            }
            if !c.is_null() {
                unsafe { SendMessageA(c, m, wp, lp) };
            }
            0
        }
        WM_SETFOCUS => {
            if !c.is_null() {
                unsafe { SetFocus(c) };
            }
            0
        }
        WM_ACTIVATEAPP => {
            if !c.is_null() {
                unsafe { SendMessageA(c, m, wp, lp) };
            }
            0
        }
        WM_ENABLE => {
            if wp == 0 {
                unsafe { PostMessageA(h, WM_REENABLE, 0, 0) };
            }
            0
        }
        WM_REENABLE => {
            unsafe { EnableWindow(h, 1) };
            0
        }
        WM_CLOSE => {
            if !c.is_null() {
                unsafe { PostMessageA(c, WM_CLOSE, 0, 0) };
            }
            0
        }
        WM_WINDOWPOSCHANGING if lp != 0 => {
            // Kept shown: Roots hides the thread's top-level windows as it goes from its
            // main menu to a screen of the shell's (its own forms, `[Menus] Forms=0`), and
            // the host went with them, the device window inside it too, so the shell's
            // screen had nothing under it and took no click (decision 404).
            let pos = unsafe { &mut *(lp as *mut WindowPos) };
            if pos.flags & SWP_HIDEWINDOW != 0 {
                pos.flags &= !SWP_HIDEWINDOW;
                if !HIDE_REFUSED.swap(true, Ordering::Relaxed) {
                    log!("window: OneWindow: a hide of the host refused: the game's window stays shown");
                }
            }
            unsafe { DefWindowProcA(h, m, wp, lp) }
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            unsafe { ValidateRect(h, core::ptr::null()) };
            0
        }
        _ => unsafe { DefWindowProcA(h, m, wp, lp) },
    }
}
