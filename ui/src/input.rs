//! Input for the UI: the mouse and keyboard as the UI sees them this frame, and
//! capture, which keeps what the UI takes from reaching the engine.
//!
//! Positions are in back-buffer pixels. `poll()` is called once per frame by
//! `crate::surface::frame` before anything is laid out.
//!
//! How it works (README.md, "Input and capture"): the engine takes the mouse and
//! keyboard from its 3D window's messages. Its `WindowProc` hands keyboard messages to
//! `ProcessKeyboardMessages` and mouse messages to `ProcessMouseMessages` (which, in a
//! mission, forwards each to the input thread). Both drain the rest of the queue's
//! messages of their kind with `PeekMessageA` as they go. We sit at four points of that
//! path, all on the game's thread: `WindowProc`'s two calls and the two drains. Each
//! message passes exactly one of them, which records it for the UI and drops what
//! capture keeps from the engine. This works wherever the 3D window gets messages: in
//! a mission, and in a screen of ours in the front end (`engine::shell`). Pointer
//! position is read with `GetCursorPos`, as the engine itself does.
use crate::engine::{self, hook};
use crate::rt::win::*;

/// How many keys and characters one frame records; more in one frame are dropped.
pub const MAX_KEYS: usize = 16;

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    /// The pointer in back-buffer pixels; None when it is outside the game window.
    pub mouse: Option<(f32, f32)>,
    /// Buttons held now: left, right, middle.
    pub down: [bool; 3],
    /// Went down / came up since the last poll.
    pub pressed: [bool; 3],
    pub released: [bool; 3],
    /// A second press in quick succession (the window's double-click), since the last poll.
    pub double: [bool; 3],
    /// Wheel notches since the last poll, positive away from the player.
    pub wheel: f32,
    /// Virtual-key codes that went down since the last poll, auto-repeats included,
    /// in order; `nkeys` of them are valid.
    pub keys: [u8; MAX_KEYS],
    pub nkeys: u8,
    /// Characters typed since the last poll (WM_CHAR, the ANSI code page read as
    /// Latin-1), control characters included (8 backspace, 13 enter, 27 escape);
    /// `nchars` of them are valid.
    pub chars: [char; MAX_KEYS],
    pub nchars: u8,
    /// Modifiers held now.
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Input {
    /// The keys that went down this frame.
    pub fn keys(&self) -> &[u8] {
        &self.keys[..self.nkeys as usize]
    }
    /// The characters typed this frame.
    pub fn chars(&self) -> &[char] {
        &self.chars[..self.nchars as usize]
    }
    /// Whether virtual key `vk` went down this frame.
    pub fn key_pressed(&self, vk: u8) -> bool {
        self.keys().contains(&vk)
    }
}

// ---- Armada2.exe ------------------------------------------------------------------

/// In `WindowProc`: `call ProcessKeyboardMessages(msg, wParam, lParam)`, cdecl.
const SITE_KEYS: usize = 0x488869;
/// `ProcessKeyboardMessages`: stores key downs for the game and updates the key state,
/// then drains the queue's other keyboard messages.
const PROCESS_KEYS: usize = 0x4dd670;
/// In `ProcessKeyboardMessages`: `call [__imp__PeekMessageA]`, the drain.
const SITE_KEYS_PEEK: usize = 0x4dd709;
const SITE_KEYS_PEEK_STOCK: [u8; 6] = [0xff, 0x15, 0xd4, 0x81, 0x7b, 0x00];
/// In `WindowProc`: `call ProcessMouseMessages(hwnd, msg, wParam, lParam)`, cdecl.
const SITE_MOUSE: usize = 0x488952;
/// `ProcessMouseMessages`: posts the message to the input thread (in a mission), drains
/// the queue's other mouse messages the same way, and captures the mouse for the window
/// while a button is down.
const PROCESS_MOUSE: usize = 0x4de250;
/// In `ProcessMouseMessages`: `mov ebx, [__imp__PeekMessageA]`, the function it drains
/// with (`call ebx`).
const SITE_MOUSE_PEEK: usize = 0x4de260;
const SITE_MOUSE_PEEK_STOCK: [u8; 6] = [0x8b, 0x1d, 0xd4, 0x81, 0x7b, 0x00];
/// `ST3D_GraphicsEngine::GetWindowHandle`, thiscall on `Storm3D`.
const GET_WINDOW_HANDLE: usize = 0x62bc70;

// ---- Win32 ------------------------------------------------------------------------

const WM_KEYDOWN: u32 = 0x100;
const WM_KEYUP: u32 = 0x101;
const WM_CHAR: u32 = 0x102;
const WM_LBUTTONDOWN: u32 = 0x201;
const WM_MBUTTONDBLCLK: u32 = 0x209;
const WM_MOUSEWHEEL: u32 = 0x20a;
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
/// GetAsyncKeyState's codes for left, right, middle.
const VK_BUTTON: [i32; 3] = [0x01, 0x02, 0x04];

/// Capture set by a frame older than this no longer applies: no UI frame is running
/// (the in-mission menu's modal dialog, the end of a mission, the loading screen).
const CAPTURE_STALE_MS: u32 = 250;

/// What the message hooks gathered since the last poll, and what capture ate.
struct Raw {
    down: [bool; 3],
    pressed: [bool; 3],
    released: [bool; 3],
    double: [bool; 3],
    wheel: i32,
    keys: [u8; MAX_KEYS],
    nkeys: u8,
    chars: [char; MAX_KEYS],
    nchars: u8,
    /// A button whose down the engine was not given, so its up is not given either.
    eaten_button: [bool; 3],
    /// Likewise for keys, by virtual-key code.
    eaten_key: [u32; 8],
    capture_mouse: bool,
    capture_keys: bool,
    capture_tick: u32,
}

static mut RAW: Raw = Raw {
    down: [false; 3],
    pressed: [false; 3],
    released: [false; 3],
    double: [false; 3],
    wheel: 0,
    keys: [0; MAX_KEYS],
    nkeys: 0,
    chars: ['\0'; MAX_KEYS],
    nchars: 0,
    eaten_button: [false; 3],
    eaten_key: [0; 8],
    capture_mouse: false,
    capture_keys: false,
    capture_tick: 0,
};

static mut STATE: Input = Input {
    mouse: None,
    down: [false; 3],
    pressed: [false; 3],
    released: [false; 3],
    double: [false; 3],
    wheel: 0.0,
    keys: [0; MAX_KEYS],
    nkeys: 0,
    chars: ['\0'; MAX_KEYS],
    nchars: 0,
    shift: false,
    ctrl: false,
    alt: false,
};

static mut INSTALLED: bool = false;

/// A stock dialog is hosted (`engine::dialogs`): the engine gets no button, wheel or key
/// down at all, whatever the widgets claim, as a modal dialog keeps them from its owner.
static mut HOSTING: bool = false;

fn raw() -> &'static mut Raw {
    unsafe { &mut *core::ptr::addr_of_mut!(RAW) }
}

/// Installs the four message hooks. False, with nothing written, when any site is not
/// stock: then the UI still reads the pointer, but cannot keep input from the engine
/// and sees no buttons or keys.
pub fn install() -> bool {
    if hook::call_target(SITE_KEYS) != Some(PROCESS_KEYS) || hook::call_target(SITE_MOUSE) != Some(PROCESS_MOUSE) {
        crate::log!("input: WindowProc's calls of ProcessKeyboardMessages and ProcessMouseMessages are not stock: standing down");
        return false;
    }
    let stock = |at: usize, b: &[u8]| unsafe { core::slice::from_raw_parts(at as *const u8, b.len()) == b };
    if !stock(SITE_KEYS_PEEK, &SITE_KEYS_PEEK_STOCK) || !stock(SITE_MOUSE_PEEK, &SITE_MOUSE_PEEK_STOCK) {
        crate::log!("input: ProcessKeyboardMessages or ProcessMouseMessages is not stock: standing down");
        return false;
    }
    // `call [imp]` -> `call rel32; nop`, and `mov ebx, [imp]` -> `mov ebx, imm32; nop`.
    let peek = keys_peek as *const () as usize;
    let rel = (peek as i64 - (SITE_KEYS_PEEK as i64 + 5)) as i32;
    let mut call = [0xe8u8, 0, 0, 0, 0, 0x90];
    call[1..5].copy_from_slice(&rel.to_le_bytes());
    let mut mov = [0xbbu8, 0, 0, 0, 0, 0x90];
    mov[1..5].copy_from_slice(&(mouse_peek as *const () as usize as u32).to_le_bytes());
    let ok = hook::patch_bytes(SITE_KEYS_PEEK, &SITE_KEYS_PEEK_STOCK, &call)
        && hook::patch_bytes(SITE_MOUSE_PEEK, &SITE_MOUSE_PEEK_STOCK, &mov)
        && hook::patch_call(SITE_KEYS, PROCESS_KEYS, keys_message as *const () as usize)
        && hook::patch_call(SITE_MOUSE, PROCESS_MOUSE, mouse_messages as *const () as usize);
    unsafe { INSTALLED = ok };
    ok
}

/// Whether capture set by the UI still applies (a UI frame ran recently).
fn live(r: &Raw) -> bool {
    unsafe { GetTickCount() }.wrapping_sub(r.capture_tick) < CAPTURE_STALE_MS
}

fn button_of(msg: u32) -> Option<(usize, u32)> {
    // 0x201..0x209: down, up, double-click for left, right, middle in turn.
    if (WM_LBUTTONDOWN..=WM_MBUTTONDBLCLK).contains(&msg) {
        let i = (msg - WM_LBUTTONDOWN) as usize;
        Some((i / 3, (i % 3) as u32))
    } else {
        None
    }
}

/// Records a mouse message; true when the engine must not see it.
fn mouse_message(msg: u32, wp: usize) -> bool {
    let r = raw();
    let capture = unsafe { HOSTING } || (r.capture_mouse && live(r));
    if msg == WM_MOUSEWHEEL {
        r.wheel += (wp >> 16) as u16 as i16 as i32;
        return capture;
    }
    let Some((b, kind)) = button_of(msg) else { return false };
    match kind {
        0 | 2 => {
            // down, or the double-click Windows sends in place of a second down
            r.down[b] = true;
            r.pressed[b] = true;
            if kind == 2 {
                r.double[b] = true;
            }
            r.eaten_button[b] = capture;
            capture
        }
        _ => {
            r.down[b] = false;
            r.released[b] = true;
            core::mem::take(&mut r.eaten_button[b])
        }
    }
}

fn key_bit(vk: usize) -> (usize, u32) {
    (vk / 32 & 7, 1 << (vk % 32))
}

/// Records a keyboard message; true when the engine must not see it. Only plain key
/// downs and characters are kept from it: the system keys (Alt combinations, F10)
/// always reach it, so Alt+F4 and Alt+Tab work whatever the UI has focus on, and a
/// key's up is kept only when its down was.
fn key_message(msg: u32, wp: usize) -> bool {
    let r = raw();
    let capture = unsafe { HOSTING } || (r.capture_keys && live(r));
    match msg {
        WM_KEYDOWN => {
            if (r.nkeys as usize) < MAX_KEYS {
                r.keys[r.nkeys as usize] = wp as u8;
                r.nkeys += 1;
            }
            let (w, bit) = key_bit(wp & 0xff);
            if capture {
                r.eaten_key[w] |= bit;
            } else {
                r.eaten_key[w] &= !bit;
            }
            capture
        }
        WM_KEYUP => {
            let (w, bit) = key_bit(wp & 0xff);
            let eaten = r.eaten_key[w] & bit != 0;
            r.eaten_key[w] &= !bit;
            eaten
        }
        WM_CHAR => {
            if (r.nchars as usize) < MAX_KEYS {
                r.chars[r.nchars as usize] = char::from(wp as u8);
                r.nchars += 1;
            }
            // The engine ignores WM_CHAR; dropping it changes nothing but is tidy.
            capture
        }
        _ => false,
    }
}

/// In place of `WindowProc`'s call of `ProcessKeyboardMessages`.
unsafe extern "C" fn keys_message(msg: u32, wp: usize, lp: isize) {
    if key_message(msg, wp) {
        return;
    }
    let stock: unsafe extern "C" fn(u32, usize, isize) = unsafe { core::mem::transmute(PROCESS_KEYS) };
    unsafe { stock(msg, wp, lp) };
}

/// In place of the `PeekMessageA` that `ProcessKeyboardMessages` drains the queue with:
/// what capture keeps from the engine is translated (so its characters still come)
/// and skipped, and the next message is peeked in its place.
unsafe extern "system" fn keys_peek(m: *mut Msg, w: HWND, min: u32, max: u32, remove: u32) -> i32 {
    loop {
        let got = unsafe { PeekMessageA(m, w, min, max, remove) };
        if got == 0 {
            return 0;
        }
        let (msg, wp) = unsafe { ((*m).message, (*m).wparam) };
        if !key_message(msg, wp) {
            return got;
        }
        if msg == WM_KEYDOWN {
            unsafe { TranslateMessage(m) };
        }
    }
}

/// In place of `WindowProc`'s call of `ProcessMouseMessages`.
unsafe extern "C" fn mouse_messages(w: HWND, msg: u32, wp: usize, lp: isize) {
    if mouse_message(msg, wp) {
        return;
    }
    let stock: unsafe extern "C" fn(HWND, u32, usize, isize) = unsafe { core::mem::transmute(PROCESS_MOUSE) };
    unsafe { stock(w, msg, wp, lp) };
}

/// In place of the `PeekMessageA` that `ProcessMouseMessages` drains the queue with:
/// what capture keeps from the engine is skipped, and the next message is peeked in
/// its place.
unsafe extern "system" fn mouse_peek(m: *mut Msg, w: HWND, min: u32, max: u32, remove: u32) -> i32 {
    loop {
        let got = unsafe { PeekMessageA(m, w, min, max, remove) };
        if got == 0 {
            return 0;
        }
        let (msg, wp) = unsafe { ((*m).message, (*m).wparam) };
        if !mouse_message(msg, wp) {
            return got;
        }
    }
}

/// The game window, as the engine names it.
pub fn window() -> HWND {
    let s = engine::storm3d();
    if s == 0 || engine::field(s, 0xcc) == 0 {
        return core::ptr::null_mut();
    }
    let f: unsafe extern "thiscall" fn(usize) -> HWND = unsafe { core::mem::transmute(GET_WINDOW_HANDLE) };
    unsafe { f(s) }
}

/// A move of the pointer between two frames longer than this, in pixels, is logged as a
/// jump (README.md, "Focus and the pointer").
const JUMP: i32 = 320;
/// At most this many focus and jump lines a game, so a pointer gone wrong for good
/// cannot fill the log.
const DIAG_LINES: u32 = 60;

struct Diag {
    /// Whether the game's window was the foreground, the active and the focus window
    /// (bits 0, 1, 2) at the last poll; None before the first.
    has: Option<u32>,
    /// The pointer on the screen at the last poll, and whether the engine held it then.
    last: Option<(i32, i32)>,
    held: bool,
    lines: u32,
}

static mut DIAG: Diag = Diag { has: None, last: None, held: false, lines: 0 };

/// `[UI] PointerLog=1` (README.md, "Focus and the pointer"): logs what the window's
/// focus and the pointer do, for the cursor that jumps after the game loses the focus
/// and gets it back. Each change of the foreground, active or focus window, with the
/// pointer and the engine's mouse state at that moment, and each jump of the pointer
/// between two frames that the engine's own pan did not make.
fn diagnose(hwnd: HWND, screen: Option<(i32, i32)>) {
    if !crate::config::try_get().is_some_and(|c| c.pointer_log) {
        return;
    }
    let d = unsafe { &mut *core::ptr::addr_of_mut!(DIAG) };
    let has = unsafe {
        (GetForegroundWindow() == hwnd) as u32
            | ((GetActiveWindow() == hwnd) as u32) << 1
            | ((GetFocus() == hwnd) as u32) << 2
    };
    let held = engine::mouse::held();
    if d.lines < DIAG_LINES && d.has != Some(has) {
        d.lines += 1;
        let (x, y) = screen.unwrap_or((-1, -1));
        let yn = |bit: u32| if has & bit != 0 { "yes" } else { "no" };
        crate::log!(
            "focus: foreground {}, active {}, focus {}; pointer {},{}; engine buttons {:#x}{}",
            yn(1),
            yn(2),
            yn(4),
            x,
            y,
            engine::mouse::buttons(),
            if held { ", pointer held for a pan" } else { "" }
        );
    }
    d.has = Some(has);
    if let (Some((x0, y0)), Some((x, y))) = (d.last, screen) {
        // The engine's pan moves the pointer to the middle at every read and back at the
        // release: not a jump.
        let pan = held || d.held;
        if !pan && d.lines < DIAG_LINES && ((x - x0).abs() > JUMP || (y - y0).abs() > JUMP) {
            d.lines += 1;
            crate::log!("pointer: jumped from {},{} to {},{} between two frames; engine buttons {:#x}", x0, y0, x, y,
                engine::mouse::buttons());
        }
    }
    d.last = screen;
    d.held = held;
}

/// Reads this frame's input. `w`, `h`: the back buffer's size.
pub fn poll(w: u32, h: u32) {
    let r = raw();
    let hwnd = window();
    let mut mouse = None;
    if !hwnd.is_null() {
        let mut p = Point::default();
        let mut c = WinRect::default();
        unsafe {
            let got = GetCursorPos(&mut p) != 0;
            diagnose(hwnd, got.then_some((p.x, p.y)));
            if got && ScreenToClient(hwnd, &mut p) != 0 && GetClientRect(hwnd, &mut c) != 0 {
                let (cw, ch) = (c.right - c.left, c.bottom - c.top);
                if cw > 0 && ch > 0 && p.x >= 0 && p.y >= 0 && p.x < cw && p.y < ch {
                    // The back buffer can differ from the client area (a window
                    // stretched by the compositor, a mode the window has not caught up
                    // with): scale, sampling at pixel centres.
                    let x = (p.x as f32 + 0.5) * w as f32 / cw as f32;
                    let y = (p.y as f32 + 0.5) * h as f32 / ch as f32;
                    mouse = Some((x, y));
                }
            }
        }
    }
    // A button released outside the window when the engine did not capture the mouse
    // (because it never saw the down) sends us no up: ask the hardware.
    for b in 0..3 {
        if r.down[b] && unsafe { GetAsyncKeyState(VK_BUTTON[b]) } as u16 & 0x8000 == 0 {
            r.down[b] = false;
            r.released[b] = true;
            r.eaten_button[b] = false;
        }
    }
    let held = |vk| unsafe { GetKeyState(vk) } as u16 & 0x8000 != 0;
    let s = Input {
        mouse,
        down: r.down,
        pressed: r.pressed,
        released: r.released,
        double: r.double,
        wheel: r.wheel as f32 / 120.0,
        keys: r.keys,
        nkeys: r.nkeys,
        chars: r.chars,
        nchars: r.nchars,
        shift: held(VK_SHIFT),
        ctrl: held(VK_CONTROL),
        alt: held(VK_MENU),
    };
    r.pressed = [false; 3];
    r.released = [false; 3];
    r.double = [false; 3];
    r.wheel = 0;
    r.nkeys = 0;
    r.nchars = 0;
    unsafe { STATE = s };
}

#[allow(static_mut_refs)]
pub fn state() -> Input {
    unsafe { STATE }
}

/// Whether a widget of ours holds the keyboard (a focused text field): the screen loop
/// of a front-end screen leaves its own Esc and Enter to it then.
pub fn keys_captured() -> bool {
    let r = raw();
    r.capture_keys && live(r)
}

/// While a stock dialog is hosted (`engine::dialogs`), every button, wheel and key down
/// is kept from the engine (and its up with it), as the stock modal dialog does by
/// disabling its owner. Moves still pass. Nested hostings each set it again.
pub fn set_hosting(on: bool) {
    unsafe { HOSTING = on };
}

/// Records a mouse or keyboard message for the UI that never reaches the window
/// procedure hooks (the dialog host's pump takes it off the queue itself).
pub fn note_message(msg: u32, wp: usize) {
    if (WM_LBUTTONDOWN..=WM_MOUSEWHEEL).contains(&msg) {
        mouse_message(msg, wp);
    } else {
        key_message(msg, wp);
    }
}

/// Whether the message hooks are in (input capture works).
pub fn installed() -> bool {
    unsafe { INSTALLED }
}

/// Whether the engine should be kept from the mouse (the pointer is over UI of ours)
/// and from the keyboard (a text field of ours has focus). Set each frame by the UI
/// after layout; read by the capture hooks for the input that arrives after it. It
/// lapses on its own when frames stop coming (`CAPTURE_STALE_MS`).
///
/// The mouse is kept from the engine as buttons and wheel only: it still sees the
/// pointer move (its hover, tooltips and edge scrolling go on), and a press it saw
/// always gets its release.
pub fn set_capture(mouse: bool, keyboard: bool) {
    let r = raw();
    r.capture_mouse = mouse;
    r.capture_keys = keyboard;
    r.capture_tick = unsafe { GetTickCount() };
}
