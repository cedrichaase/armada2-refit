//! The front end: the shell's menus, and screens of our own run in their place.
//!
//! Contract (see `crate::surface::frame`): a screen of ours runs a loop of its own on
//! the game's thread that pumps messages and, each frame, opens a scene, calls
//! `crate::surface::frame(Context::Shell)` and presents.
//!
//! The model is the game's own Credits (`Show_Credits`), the one shell screen that is
//! not a GDI dialog: it switches Storm3D to its 3D display, runs a message loop that
//! draws through Direct3D until it is done, then puts the shell's display back and
//! names the next screen in `g_shellMode`. `run_screen` does the same around a
//! closure, and `[Demo] Shell=1` puts the demo screen in the Credits' place.
use super::dev8::{self, Dev8};
use super::{field, hook, rd, storm3d};
use crate::rt::win::*;
use crate::log;
use crate::surface::Context;

/// `DispatchDialog`'s call of `Show_Credits` (the shell mode 30 case).
const SITE_CREDITS: usize = 0x606bd4;
/// `Show_Credits()`, cdecl.
const FN_SHOW_CREDITS: usize = 0x5913a0;

/// `ST3D_GraphicsEngine::SetActiveDisplay(ST3D_DisplayType)`, thiscall: 2 is the 3D
/// display (the game's resolution), 3 the shell's (800x600 stock, or what Menus.asi
/// raises it to).
const FN_SET_ACTIVE_DISPLAY: usize = 0x62c6d0;
const DISPLAY_3D: u32 = 2;
const DISPLAY_SHELL: u32 = 3;
/// `ST3D_GraphicsEngine::ClearDeviceDisplays(const ST3D_Colour&)`, thiscall.
const FN_CLEAR_DEVICE_DISPLAYS: usize = 0x62d880;
/// `ST3D_Colour_Black`.
const COLOUR_BLACK: usize = 0x7a8f98;
/// `ST3D_GraphicsEngine::GetWindowHandle()`, thiscall: the 3D window.
const FN_GET_WINDOW_HANDLE: usize = 0x62bc70;
/// `HandleMessageQueue()`, cdecl: the engine's own pump.
const FN_HANDLE_MESSAGE_QUEUE: usize = 0x488730;
/// `ST3D_DeviceDirectX8::TestCooperativeLevel()`, thiscall: resets a lost device.
const FN_ST3D_TEST_COOP: usize = 0x623c70;
/// `ST3D_DeviceDirectX8`'s vtable, and its `FlipToGDISurface` slot.
const VT_ST3D_DX8: usize = 0x6bc6ac;
const SLOT_FLIP_TO_GDI: usize = 0x6c / 4;
/// `SetCurrentBackground(HWND, ShellBitmap*)`, cdecl.
const FN_SET_CURRENT_BACKGROUND: usize = 0x6083f0;
/// `ShellBitmap::ShellBitmap(const char*, int, int)`, thiscall; the object is 0x30 bytes.
const FN_SHELL_BITMAP: usize = 0x5a3960;
const SHELL_BITMAP_SIZE: usize = 0x30;
/// `operator new(size_t)`, cdecl: the game's heap, which `ShellBitmap`'s deleting
/// destructor frees into.
const FN_NEW: usize = 0x652710;
/// `ALL_BLACK_8_BY_6_BACKGROUND`: the name of the plain black shell background.
const ALL_BLACK_BACKGROUND: usize = 0x727fd0;
/// `g_shellMode`: the screen `do_smain`'s loop dispatches next (1: main menu).
const SHELL_MODE: usize = 0x7a8980;
const SHELL_MODE_MAIN_MENU: i32 = 1;
/// `ExitCode`: where the engine's pump keeps a `WM_QUIT`'s code.
pub(crate) const EXIT_CODE: usize = 0x737bb0;
/// `star_trek` (the `Program`): its first byte lets Alt+Tab and Alt+Esc through to
/// Windows, as `HandleMessageQueue` reads it.
pub(crate) const PROGRAM: usize = 0x737b30;

/// Each function is checked by its first bytes before anything calls it.
const SIGS: &[(usize, &[u8])] = &[
    (FN_SHOW_CREDITS, &[0x55, 0x8b, 0xec, 0x6a, 0xff]),
    (FN_SET_ACTIVE_DISPLAY, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x1c]),
    (FN_CLEAR_DEVICE_DISPLAYS, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x18]),
    (FN_GET_WINDOW_HANDLE, &[0x8b, 0x89, 0xcc, 0x00, 0x00, 0x00]),
    (FN_HANDLE_MESSAGE_QUEUE, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x1c]),
    (FN_ST3D_TEST_COOP, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08]),
    (FN_SET_CURRENT_BACKGROUND, &[0x55, 0x8b, 0xec, 0x51]),
    (FN_SHELL_BITMAP, &[0x55, 0x8b, 0xec, 0x6a, 0xff]),
    (FN_NEW, &[0x55, 0x8b, 0xec]),
];

/// What `run_screen` returns when the game was asked to quit while it ran (the
/// `WM_QUIT` is posted again, so the shell sees it too).
pub const QUIT: i32 = i32::MIN;

static mut READY: bool = false;

/// Patches the hooks; false (and nothing patched) when a site is not stock.
pub fn install() -> bool {
    for &(at, sig) in SIGS {
        if unsafe { core::slice::from_raw_parts(at as *const u8, sig.len()) } != sig {
            log!("shell: {:#x} is not stock; standing down", at);
            return false;
        }
    }
    if hook::call_target(SITE_CREDITS) != Some(FN_SHOW_CREDITS) {
        log!("shell: the Credits call at {:#x} is not stock; standing down", SITE_CREDITS);
        return false;
    }
    unsafe { READY = true };
    if crate::config::get().demo_shell || crate::config::get().demo_style {
        // The demo takes the Credits' place (below).
    } else if crate::config::get().credits {
        if !hook::patch_call(SITE_CREDITS, FN_SHOW_CREDITS, crate::credits::show as *const () as usize) {
            log!("shell: could not patch {:#x}", SITE_CREDITS);
            return false;
        }
        log!("shell: [Menus] Credits=1 -- the Credits are ours");
        return true;
    }
    if crate::config::get().demo_shell || crate::config::get().demo_style {
        if !hook::patch_call(SITE_CREDITS, FN_SHOW_CREDITS, demo_credits as *const () as usize) {
            log!("shell: could not patch {:#x}", SITE_CREDITS);
            return false;
        }
        log!("shell: [Demo] Shell=1 -- Credits shows the demo screen");
    }
    true
}

// ---- calling the engine ----------------------------------------------------------------

unsafe fn this0(f: usize, this: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> usize>(f)(this) }
}
unsafe fn this1(f: usize, this: usize, a: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize) -> usize>(f)(this, a) }
}
unsafe fn this3(f: usize, this: usize, a: usize, b: usize, c: usize) -> usize {
    unsafe {
        core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize, usize, usize) -> usize>(f)(this, a, b, c)
    }
}
unsafe fn cdecl0(f: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> usize>(f)() }
}
unsafe fn cdecl1(f: usize, a: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(f)(a) }
}
unsafe fn cdecl2(f: usize, a: usize, b: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize, usize) -> usize>(f)(a, b) }
}

fn set_active_display(t: u32) -> u32 {
    let s = storm3d();
    if s == 0 {
        return 1;
    }
    unsafe { this1(FN_SET_ACTIVE_DISPLAY, s, t as usize) as u32 }
}

/// The current `ST3D_DeviceDirectX8` (Storm3D `+0xc0` into the table at `+0xcc`), or 0
/// if it is not one.
pub(crate) fn st3d_device() -> usize {
    let s = storm3d();
    let d = field(s + 0xcc, field(s, 0xc0) * 4);
    if d != 0 && field(d, 0) == VT_ST3D_DX8 { d } else { 0 }
}

fn window() -> HWND {
    let s = storm3d();
    if s == 0 || field(s, 0xcc) == 0 {
        return core::ptr::null_mut();
    }
    unsafe { this0(FN_GET_WINDOW_HANDLE, s) as HWND }
}

fn flip_to_gdi() {
    let d = st3d_device();
    if d != 0 {
        unsafe { this0(field(VT_ST3D_DX8, SLOT_FLIP_TO_GDI * 4), d) };
    }
}

// ---- the screen loop -------------------------------------------------------------------

/// What the message pump saw since the last frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Pumped {
    /// Keys that went down (fresh presses, not auto-repeats), as virtual-key codes.
    pub keys: [u8; 16],
    pub nkeys: usize,
    /// Buttons that went down: left, right, middle.
    pub clicked: [bool; 3],
}

impl Pumped {
    pub fn key(&self, vk: u8) -> bool {
        self.keys[..self.nkeys].contains(&vk)
    }
    pub fn escape(&self) -> bool {
        self.key(0x1b)
    }
    pub fn any_click(&self) -> bool {
        self.clicked.iter().any(|&c| c)
    }
}

/// One drain of the thread's queue, as the engine's `HandleMessageQueue` does it, also
/// noting presses for the screen. Some(code) on `WM_QUIT`.
fn pump(p: &mut Pumped) -> Option<i32> {
    let mut m = Msg { hwnd: core::ptr::null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, pt: Point::default() };
    unsafe {
        while PeekMessageA(&mut m, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            match m.message {
                WM_QUIT => return Some(m.wparam as i32),
                WM_KEYDOWN | WM_SYSKEYDOWN if m.lparam & (1 << 30) == 0 => {
                    if p.nkeys < p.keys.len() {
                        p.keys[p.nkeys] = m.wparam as u8;
                        p.nkeys += 1;
                    }
                }
                WM_LBUTTONDOWN => p.clicked[0] = true,
                WM_RBUTTONDOWN => p.clicked[1] = true,
                WM_MBUTTONDOWN => p.clicked[2] = true,
                _ => {}
            }
            // HandleMessageQueue keeps Alt+Tab and Alt+Esc from Windows unless the
            // program allows it.
            if m.message == WM_SYSKEYDOWN && (m.wparam == 0x09 || m.wparam == 0x1b) && rd::<u8>(PROGRAM) == 0 {
                continue;
            }
            TranslateMessage(&m);
            DispatchMessageA(&m);
        }
    }
    None
}

// The 3D window answers WM_SETCURSOR with no cursor, for the engine's sprite cursor in
// a mission. A screen of ours (and a hosted dialog) has the arrow, as the GDI menus do,
// or none while the UI draws its own.
//
// Not by subclassing the window (README.md, "The front end: screens of our own"): DXVK
// hooks the procedure of a full-screen device's window and gives it back only if its
// own is still on top when the device goes. A subclass made while a device was up kept
// DXVK's procedure as the one before it; a new device then took the subclass as its
// own "before", and the two called each other until the stack ran out. So the engine's
// own answer is changed instead, at the one `call [SetCursor]` in its `WM_SETCURSOR`
// case.
/// In `WindowProc`, the `WM_SETCURSOR` case: `push 0` then this `call [SetCursor]`.
const SITE_WM_SETCURSOR: usize = 0x488883;
/// `call dword ptr [__imp__SetCursor@4]`.
const STOCK_WM_SETCURSOR: [u8; 6] = [0xff, 0x15, 0xa8, 0x81, 0x7b, 0x00];
static mut ARROW: HANDLE = core::ptr::null_mut();
static mut ARROW_ON: bool = false;
static mut OS_CURSOR_HIDDEN: bool = false;
static mut SETCURSOR_PATCHED: bool = false;

/// Points the engine's `WM_SETCURSOR` answer at ours (once); false, and the engine's
/// "no cursor" kept, when the site is not stock.
fn patch_wm_setcursor() -> bool {
    unsafe {
        if SETCURSOR_PATCHED {
            return true;
        }
        let at = SITE_WM_SETCURSOR;
        let rel = (wm_setcursor as *const () as usize as i64 - (at as i64 + 5)) as i32;
        let mut new = [0xe8u8, 0, 0, 0, 0, 0x90];
        new[1..5].copy_from_slice(&rel.to_le_bytes());
        if !hook::patch_bytes(at, &STOCK_WM_SETCURSOR, &new) {
            log!("shell: WindowProc's WM_SETCURSOR at {:#x} is not stock: the 3D window keeps no cursor", at);
            return false;
        }
        SETCURSOR_PATCHED = true;
        log!("shell: WindowProc's WM_SETCURSOR answered by us");
        true
    }
}

/// `SetCursor` as the engine's `WM_SETCURSOR` calls it (stdcall, the cursor it wants:
/// none).
unsafe extern "system" fn wm_setcursor(c: HANDLE) -> HANDLE {
    unsafe {
        if ARROW_ON {
            return SetCursor(if OS_CURSOR_HIDDEN { core::ptr::null_mut() } else { ARROW });
        }
        SetCursor(c)
    }
}

/// Hides the arrow Windows draws over a screen of ours, for the UI to draw its own
/// (cursor.rs). Lasts until the screen ends.
pub fn hide_os_cursor() {
    unsafe {
        if ARROW_ON {
            OS_CURSOR_HIDDEN = true;
            SetCursor(core::ptr::null_mut());
        }
    }
}

fn subclass(_w: HWND) {
    unsafe {
        if ARROW.is_null() {
            ARROW = LoadCursorA(core::ptr::null_mut(), IDC_ARROW);
        }
        if !patch_wm_setcursor() {
            return;
        }
        ARROW_ON = true;
        OS_CURSOR_HIDDEN = false;
        SetCursor(ARROW);
    }
}

fn unsubclass() {
    unsafe {
        ARROW_ON = false;
        OS_CURSOR_HIDDEN = false;
    }
}

/// The arrow over the 3D window while a screen of ours (or a hosted dialog) is up, in
/// place of the engine's "no cursor".
pub(crate) fn arrow_cursor(w: HWND, on: bool) {
    if on {
        if !w.is_null() {
            subclass(w);
        }
    } else {
        unsubclass();
    }
}

/// The engine's own `TestCooperativeLevel` on its device, which resets a device that
/// can be reset.
pub(crate) fn test_cooperative(st: usize) {
    if unsafe { READY } && st != 0 {
        unsafe { this0(FN_ST3D_TEST_COOP, st) };
    }
}

static mut FACTS_LOGGED: u32 = 0;

/// The device facts the shell's hooks rely on, logged on the first screens.
fn log_facts(when: &str) {
    unsafe {
        if FACTS_LOGGED >= 4 {
            return;
        }
        FACTS_LOGGED += 1;
    }
    let s = storm3d();
    let dev = super::d3d8_device();
    let w = window();
    let mut cr = WinRect::default();
    let style = if w.is_null() { 0 } else { unsafe { GetClientRect(w, &mut cr); GetWindowLongA(w, GWL_STYLE) } };
    log!(
        "shell: {}: Storm3D {:#x} display {} device[{}] {:#x} (dx8 class: {}) d3d8 {:p} window {:p} client {}x{} style {:#x}",
        when,
        s,
        field(s, 0x10c),
        field(s, 0xc0),
        field(s + 0xcc, field(s, 0xc0) * 4),
        st3d_device() != 0,
        dev,
        w,
        cr.right - cr.left,
        cr.bottom - cr.top,
        style as u32
    );
    if dev.is_null() {
        return;
    }
    if let Some(b) = dev8::back_buffer_desc(dev) {
        log!("shell: {}: back buffer {}x{} format {} ms {}", when, b.width, b.height, b.format, b.ms_type);
    }
    if let Some(m) = dev8::display_mode(dev) {
        log!("shell: {}: display mode {}x{} @{} format {}", when, m.width, m.height, m.refresh, m.format);
    }
    match dev8::present_params9(dev) {
        Some(p) => log!(
            "shell: {}: d3d9 swap chain {}x{} x{} windowed {} swap effect {} interval {:#x} window {:#x}",
            when, p.bb_width, p.bb_height, p.bb_count, p.windowed, p.swap_effect, p.interval, p.window
        ),
        None => log!("shell: {}: no d3d9 device behind the game's (not d3d8to9)", when),
    }
}

/// Runs a screen of our own in the shell until `f` returns Some, and returns that
/// value (or `QUIT`). On the game's thread, between two shell screens, as the game's
/// Credits run: the 3D display is put up, every frame the queue is drained, a scene
/// opened and cleared, `surface::frame(Context::Shell)` drawn, `f` asked whether the
/// screen is done, and the frame presented; afterwards the shell's display is put back
/// and the screen cleared to black, ready for the next GDI menu.
///
/// A frame whose device is lost is skipped (the engine's own test resets it); `f` is
/// still asked, so the screen can be left. Frames are held to about 60 a second.
pub fn run_screen(f: &mut dyn FnMut(&Pumped) -> Option<i32>) -> i32 {
    run_screen_under(&mut |_, _| {}, f)
}

/// `run_screen`, with `under` drawn each frame after the clear and before the UI's
/// frame, with the back buffer's size (the Credits draw render3d's map there, as the
/// dialog host draws the menu scene).
pub fn run_screen_under(under: &mut dyn FnMut(u32, u32), f: &mut dyn FnMut(&Pumped) -> Option<i32>) -> i32 {
    if !unsafe { READY } {
        return QUIT;
    }
    log_facts("shell display");
    let err = set_active_display(DISPLAY_3D);
    if err != 0 {
        log!("shell: SetActiveDisplay(3D) failed ({}); screen not shown", err);
        return set_shell_display_and(QUIT);
    }
    log_facts("3D display");
    let w = window();
    if !w.is_null() {
        subclass(w);
        unsafe {
            if GetFocus().is_null() {
                SetFocus(w);
            }
        }
    }

    let mut freq = 0i64;
    unsafe { QueryPerformanceFrequency(&mut freq) };
    let period = (freq / 60).max(1);
    let mut frames = 0u32;
    let mut lost = 0u32;
    let result;
    loop {
        let mut t0 = 0i64;
        unsafe { QueryPerformanceCounter(&mut t0) };
        let mut p = Pumped::default();
        if let Some(code) = pump(&mut p) {
            unsafe {
                core::ptr::write_unaligned(EXIT_CODE as *mut i32, code);
                PostQuitMessage(code);
            }
            log!("shell: WM_QUIT ({}) during a screen of ours", code);
            result = QUIT;
            break;
        }
        // The device can change under us (a display switch re-creates it when the
        // mode differs), so it is read every frame.
        let dev: Dev8 = super::d3d8_device();
        let st = st3d_device();
        let mut done = None;
        if !dev.is_null() && st != 0 {
            unsafe { this0(FN_ST3D_TEST_COOP, st) };
            if dev8::test_cooperative_level(dev) < 0 {
                lost += 1;
                done = f(&p);
                unsafe { Sleep(50) };
            } else if let Some(bb) = dev8::back_buffer_desc(dev) {
                dev8::begin_scene(dev);
                dev8::set_viewport(dev, bb.width, bb.height);
                dev8::clear(dev, dev8::CLEAR_TARGET, 0xff000000);
                under(bb.width, bb.height);
                crate::surface::frame(Context::Shell);
                done = f(&p);
                dev8::end_scene(dev);
                dev8::present(dev);
                frames += 1;
                if frames == 1 {
                    log!("shell: first frame of ours presented, {}x{}", bb.width, bb.height);
                }
            }
        } else {
            done = f(&p);
            unsafe { Sleep(50) };
        }
        if let Some(r) = done {
            result = r;
            break;
        }
        let mut t1 = 0i64;
        unsafe { QueryPerformanceCounter(&mut t1) };
        let left = period - (t1 - t0);
        if left > 0 {
            unsafe { Sleep((left * 1000 / freq) as u32) };
        }
    }
    log!("shell: screen of ours done ({}) after {} frames, {} lost", result, frames, lost);
    unsubclass();
    let s = storm3d();
    unsafe { this1(FN_CLEAR_DEVICE_DISPLAYS, s, COLOUR_BLACK) };
    set_shell_display_and(result)
}

/// The shell's display back, as the Credits leave it: `SetActiveDisplay(shell)`,
/// `FlipToGDISurface` and a drain of the queue through the engine's own pump.
fn set_shell_display_and(r: i32) -> i32 {
    set_active_display(DISPLAY_SHELL);
    flip_to_gdi();
    unsafe { cdecl0(FN_HANDLE_MESSAGE_QUEUE) };
    log_facts("back in the shell");
    r
}

/// The black shell background the Credits leave behind them (`SetCurrentBackground`
/// with `ALL_BLACK_8_BY_6_BACKGROUND`), so the next menu opens over black as it does
/// after the stock Credits.
fn black_background() {
    let w = window();
    unsafe {
        let bmp = cdecl1(FN_NEW, SHELL_BITMAP_SIZE);
        if bmp == 0 {
            return;
        }
        this3(FN_SHELL_BITMAP, bmp, rd::<usize>(ALL_BLACK_BACKGROUND), 0, 0);
        cdecl2(FN_SET_CURRENT_BACKGROUND, w as usize, bmp);
        // Its deleting destructor, vtable slot 0, with the delete flag.
        this1(field(field(bmp, 0), 0), bmp, 1);
    }
}

/// `[Demo] Shell=1`: the Credits replaced by the demo screen, left by Esc, Enter or a
/// right click (a left click is the demo's widgets'), returning to the main menu as
/// the stock Credits do.
extern "C" fn demo_credits() {
    log!("shell: Credits -> the demo screen");
    let started = crate::rt::now();
    let r = run_screen(&mut |p: &Pumped| {
        // A click in the first moment is still the one that opened the screen.
        let settled = crate::rt::now() - started > 0.3;
        if ((p.escape() || p.key(0x0d)) && !crate::input::keys_captured()) || (settled && p.clicked[1]) {
            Some(0)
        } else {
            None
        }
    });
    leave_to_main_menu();
    log!("shell: demo screen left ({}); g_shellMode = {}", r, SHELL_MODE_MAIN_MENU);
}

/// After a screen of ours in the Credits' place: the black background the Credits
/// leave and the main menu next, as the stock Credits do.
pub fn leave_to_main_menu() {
    black_background();
    unsafe { core::ptr::write_unaligned(SHELL_MODE as *mut i32, SHELL_MODE_MAIN_MENU) };
}

