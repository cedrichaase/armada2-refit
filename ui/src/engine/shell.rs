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
use super::{field, hook as patcher, rd, storm3d};
use crate::rt::win::*;
use crate::log;
use crate::surface::Context;

use crate::syms_shell::{field, hook, sym, value, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// `ST3D_GraphicsEngine::SetActiveDisplay(ST3D_DisplayType)`, thiscall: 2 is the 3D
/// display (the game's resolution), 3 the shell's (800x600 stock, or what Display8
/// raises it to for the menu host).
#[inline(always)]
fn display_3d() -> u32 { value::SHELL_DISPLAY_3D() as u32 }
#[inline(always)]
fn display_shell() -> u32 { value::SHELL_DISPLAY_SHELL() as u32 }
#[inline(always)]
fn shell_mode_main_menu() -> i32 { value::SHELL_MODE_MAIN_MENU() }

/// `ExitCode`'s address, where the engine's pump keeps a `WM_QUIT`'s code; 0 when the
/// shell's device reads did not become ready.
pub(crate) fn exit_code_addr() -> usize {
    if eng().ready(hook::SHELL_READS_DEVICE).is_err() {
        return 0;
    }
    eng().addr(sym::SHELL_EXIT_CODE)
}

/// `star_trek` (the `Program`)'s address: its first byte lets Alt+Tab and Alt+Esc
/// through to Windows, as `HandleMessageQueue` reads it; 0 when not ready.
pub(crate) fn program_addr() -> usize {
    if eng().ready(hook::SHELL_READS_DEVICE).is_err() {
        return 0;
    }
    eng().addr(sym::SHELL_PROGRAM)
}

/// Whether the Alt+Tab / Alt+Esc keys are kept from Windows (the program does not allow
/// them), as `HandleMessageQueue` decides.
fn keep_alt_keys() -> bool {
    let p = program_addr();
    p == 0 || unsafe { rd::<u8>(p) } == 0
}

/// What `run_screen` returns when the game was asked to quit while it ran (the
/// `WM_QUIT` is posted again, so the shell sees it too).
pub const QUIT: i32 = i32::MIN;

static mut READY: bool = false;
/// The Credits are ours (`[Menus] Credits=1`, the call patched): the claim `CLAIM_CREDITS`.
static mut CREDITS: bool = false;

/// Whether the Credits are ours: the engine's never run.
pub fn credits_ours() -> bool {
    unsafe { CREDITS }
}

/// Patches the hooks; false (and nothing patched) when the engine stands the hook down.
pub fn install() -> bool {
    if eng().ready(hook::SHELL_SCREENS).is_err() || eng().ready(hook::SHELL_READS_DEVICE).is_err() {
        return false;
    }
    let site = eng().addr(sym::SHELL_CREDITS_SITE);
    let show = eng().addr(sym::SHELL_SHOW_CREDITS);
    unsafe { READY = true };
    if crate::config::get().demo_shell || crate::config::get().demo_style {
        // The demo takes the Credits' place (below).
    } else if crate::config::get().credits {
        if !patcher::patch_call(site, show, crate::credits::show as *const () as usize) {
            log!("shell: could not patch {:#x}", site);
            return false;
        }
        unsafe { CREDITS = true };
        log!("shell: [Menus] Credits=1 -- the Credits are ours");
        return true;
    }
    if crate::config::get().demo_shell || crate::config::get().demo_style {
        if !patcher::patch_call(site, show, demo_credits as *const () as usize) {
            log!("shell: could not patch {:#x}", site);
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
    unsafe { this1(eng().addr(sym::SHELL_SET_ACTIVE_DISPLAY), s, t as usize) as u32 }
}

/// The current `ST3D_DeviceDirectX8` (Storm3D `+0xc0` into the table at `+0xcc`), or 0
/// if it is not one.
pub(crate) fn st3d_device() -> usize {
    if eng().ready(hook::SHELL_READS_DEVICE).is_err() {
        return 0;
    }
    let s = storm3d();
    let d = field(s + field::SHELL_STORM3D_DEVICES(), field(s, field::SHELL_STORM3D_DEVICE_INDEX()) * 4);
    if d != 0 && field(d, 0) == eng().addr(sym::SHELL_DX8_VTABLE) { d } else { 0 }
}

fn window() -> HWND {
    let s = storm3d();
    if s == 0 || field(s, field::SHELL_STORM3D_DEVICES()) == 0 {
        return core::ptr::null_mut();
    }
    unsafe { this0(eng().addr(sym::SHELL_GET_WINDOW_HANDLE), s) as HWND }
}

fn flip_to_gdi() {
    let d = st3d_device();
    let slot = eng().addr(sym::SHELL_DX8_FLIP_TO_GDI);
    if d != 0 && slot != 0 {
        unsafe { this0(field(slot, 0), d) };
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
            if m.message == WM_SYSKEYDOWN && (m.wparam == 0x09 || m.wparam == 0x1b) && keep_alt_keys() {
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
        if eng().ready(hook::SHELL_CURSOR).is_err() {
            return false;
        }
        // In `WindowProc`, the `WM_SETCURSOR` case: `push 0` then a `call [SetCursor]`,
        // replaced by a call of ours (`ready` checked what is there).
        let at = eng().addr(sym::SHELL_WM_SETCURSOR);
        let rel = (wm_setcursor as *const () as usize as i64 - (at as i64 + 5)) as i32;
        let mut new = [0xe8u8, 0, 0, 0, 0, 0x90];
        new[1..5].copy_from_slice(&rel.to_le_bytes());
        let mut stock = [0u8; 6];
        core::ptr::copy_nonoverlapping(at as *const u8, stock.as_mut_ptr(), 6);
        if !patcher::patch_bytes(at, &stock, &new) {
            log!("shell: could not patch WindowProc's WM_SETCURSOR at {:#x}: the 3D window keeps no cursor", at);
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
        unsafe { this0(eng().addr(sym::SHELL_ST3D_TEST_COOP), st) };
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
        "shell: {}: Storm3D {:#x} display {} device[{}] {:#x} (dx8 class: {}) d3d8 {:p} window {:p} client {}x{} style {:#x}; Display8 says the mode is {:?}",
        when,
        s,
        field(s, field::SHELL_STORM3D_DISPLAY()),
        field(s, field::SHELL_STORM3D_DEVICE_INDEX()),
        field(s + field::SHELL_STORM3D_DEVICES(), field(s, field::SHELL_STORM3D_DEVICE_INDEX()) * 4),
        st3d_device() != 0,
        dev,
        w,
        cr.right - cr.left,
        cr.bottom - cr.top,
        style as u32,
        crate::gfx::gpu().display_mode()
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
    let err = set_active_display(display_3d());
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
                let ec = exit_code_addr();
                if ec != 0 {
                    core::ptr::write_unaligned(ec as *mut i32, code);
                }
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
            unsafe { this0(eng().addr(sym::SHELL_ST3D_TEST_COOP), st) };
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
    unsafe { this1(eng().addr(sym::SHELL_CLEAR_DEVICE_DISPLAYS), s, eng().addr(sym::SHELL_COLOUR_BLACK)) };
    set_shell_display_and(result)
}

/// The shell's display back, as the Credits leave it: `SetActiveDisplay(shell)`,
/// `FlipToGDISurface` and a drain of the queue through the engine's own pump.
fn set_shell_display_and(r: i32) -> i32 {
    set_active_display(display_shell());
    flip_to_gdi();
    unsafe { cdecl0(eng().addr(sym::SHELL_HANDLE_MESSAGE_QUEUE)) };
    log_facts("back in the shell");
    r
}

/// The black shell background the Credits leave behind them (`SetCurrentBackground`
/// with `ALL_BLACK_8_BY_6_BACKGROUND`), so the next menu opens over black as it does
/// after the stock Credits.
fn black_background() {
    if !unsafe { READY } {
        return;
    }
    let w = window();
    unsafe {
        let bmp = cdecl1(eng().addr(sym::SHELL_NEW), value::SHELL_BITMAP_SIZE() as usize);
        if bmp == 0 {
            return;
        }
        this3(eng().addr(sym::SHELL_SHELL_BITMAP), bmp, rd::<usize>(eng().addr(sym::SHELL_ALL_BLACK_BACKGROUND)), 0, 0);
        cdecl2(eng().addr(sym::SHELL_SET_CURRENT_BACKGROUND), w as usize, bmp);
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
    log!("shell: demo screen left ({}); g_shellMode = {}", r, shell_mode_main_menu());
}

/// After a screen of ours in the Credits' place: the black background the Credits
/// leave and the main menu next, as the stock Credits do.
pub fn leave_to_main_menu() {
    black_background();
    let mode = eng().addr(sym::SHELL_MODE);
    if mode != 0 {
        unsafe { core::ptr::write_unaligned(mode as *mut i32, shell_mode_main_menu()) };
    }
}

