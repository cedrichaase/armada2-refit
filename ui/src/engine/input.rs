//! The engine side of input (`crate::input`; README.md, "Input and capture"): the four
//! points of the engine's message path the UI sits at (`WindowProc`'s calls of
//! `ProcessKeyboardMessages` and `ProcessMouseMessages`, and the `PeekMessageA` each
//! drains the queue with), the stock functions called on from there, and the game
//! window as the engine names it. What the UI records and keeps from the engine is
//! `crate::input`'s; this file only patches the sites and calls the engine.
//!
//! A build whose window procedure is not the engine's `WindowProc` (Fleet Operations:
//! Roots, where the game window is FleetOpsHook's own form and its window procedure calls
//! the two functions itself) takes the messages at the two functions' entries instead of
//! at `WindowProc`'s calls (`core.input_callee`, the profile's `core_input_at_callee`;
//! PLAN-FLEETOPS.md, FO-6, decision 71). The drains are the same either way.
use super::{eng, hook as patch};
use crate::rt::win::*;
use crate::syms_core::{field, hook, sym, value};
use alloc::string::String;

/// Whether this build takes the messages at the two functions' entries (the profile's value).
pub fn at_callee() -> bool {
    value::CORE_INPUT_AT_CALLEE() != 0
}

/// The trampolines the entry hooks run the two functions through (`at_callee`): the moved
/// instructions, then a jump back. 0 until installed.
static mut KEYS_TRAMP: usize = 0;
static mut MOUSE_TRAMP: usize = 0;
#[repr(align(16))]
struct Tramp([u8; 32]);
static mut KEYS_CODE: Tramp = Tramp([0xcc; 32]);
static mut MOUSE_CODE: Tramp = Tramp([0xcc; 32]);
/// `crate::input`'s handlers, which the entry hooks hand each message to.
static mut KEYS_HANDLER: usize = 0;
static mut MOUSE_HANDLER: usize = 0;
/// The return address of the call that reached an entry hook, noted by its thunk.
static mut KEYS_CALLER: usize = 0;
static mut MOUSE_CALLER: usize = 0;

/// Patches the four points to call `keys_message`, `keys_peek`, `mouse_messages` and
/// `mouse_peek` (the addresses of `crate::input`'s handlers): at `WindowProc`'s calls, or
/// at the two functions' entries on a build that asks for it (`at_callee`). False, with
/// nothing written, when any site is not stock.
pub fn install(keys_message: usize, keys_peek: usize, mouse_messages: usize, mouse_peek: usize) -> bool {
    let callee = at_callee();
    let h = if callee { hook::CORE_INPUT_CALLEE } else { hook::CORE_INPUT };
    if eng().ready(h).is_err() {
        crate::log!("input: standing down");
        return false;
    }
    let (peek_keys, peek_mouse) = (
        eng().addr(sym::CORE_SITE_KEYS_PEEK),
        eng().addr(sym::CORE_SITE_MOUSE_PEEK),
    );
    // The six bytes at each drain, which the profile's check has just read.
    let six = |at: usize| -> [u8; 6] { unsafe { core::ptr::read_unaligned(at as *const [u8; 6]) } };
    let (keys_stock, mouse_stock) = (six(peek_keys), six(peek_mouse));
    // `call [imp]` -> `call rel32; nop`, and `mov ebx, [imp]` -> `mov ebx, imm32; nop`.
    let peek = keys_peek;
    let rel = (peek as i64 - (peek_keys as i64 + 5)) as i32;
    let mut call = [0xe8u8, 0, 0, 0, 0, 0x90];
    call[1..5].copy_from_slice(&rel.to_le_bytes());
    let mut mov = [0xbbu8, 0, 0, 0, 0, 0x90];
    mov[1..5].copy_from_slice(&(mouse_peek as u32).to_le_bytes());
    if callee {
        return install_entries(keys_message, mouse_messages)
            && patch::patch_bytes(peek_keys, &keys_stock, &call)
            && patch::patch_bytes(peek_mouse, &mouse_stock, &mov);
    }
    let (site_keys, site_mouse) = (eng().addr(sym::CORE_SITE_KEYS), eng().addr(sym::CORE_SITE_MOUSE));
    patch::patch_bytes(peek_keys, &keys_stock, &call)
        && patch::patch_bytes(peek_mouse, &mouse_stock, &mov)
        && patch::patch_call(site_keys, eng().addr(sym::CORE_PROCESS_KEYS), keys_message)
        && patch::patch_call(site_mouse, eng().addr(sym::CORE_PROCESS_MOUSE), mouse_messages)
}

/// The two entry hooks (`core.input_callee`): each function's first instructions move to a
/// trampoline, and the entry jumps to a thunk that notes the caller and hands the message
/// to `crate::input`, which calls the function through the trampoline for what it lets
/// through. The moved bytes lie within the entries' checks, which `ready` compared.
#[allow(static_mut_refs)]
fn install_entries(keys_message: usize, mouse_messages: usize) -> bool {
    let len = value::CORE_INPUT_MOVED().max(0) as usize;
    let (k, m) = (eng().addr(sym::CORE_KEYS_ENTRY), eng().addr(sym::CORE_MOUSE_ENTRY));
    let (kc, mc) = (eng().check_bytes(sym::CORE_KEYS_ENTRY), eng().check_bytes(sym::CORE_MOUSE_ENTRY));
    if len < 5 || len > kc.len() || len > mc.len() {
        crate::log!("input: the entries cannot be moved ({} of {}/{} checked bytes)", len, kc.len(), mc.len());
        return false;
    }
    unsafe {
        KEYS_HANDLER = keys_message;
        MOUSE_HANDLER = mouse_messages;
        if !patch::patch_entry(k, kc, len, keys_thunk as *const () as usize, &mut KEYS_CODE.0) {
            crate::log!("input: ProcessKeyboardMessages's entry is not stock");
            return false;
        }
        KEYS_TRAMP = KEYS_CODE.0.as_ptr() as usize;
        if !patch::patch_entry(m, mc, len, mouse_thunk as *const () as usize, &mut MOUSE_CODE.0) {
            crate::log!("input: ProcessMouseMessages's entry is not stock");
            return false;
        }
        MOUSE_TRAMP = MOUSE_CODE.0.as_ptr() as usize;
    }
    crate::log!("input: taken at the entries of ProcessKeyboardMessages ({:#x}) and ProcessMouseMessages ({:#x}), {} bytes moved", k, m, len);
    true
}

/// The keyboard entry's thunk: the caller's return address noted, then `keys_entry`.
#[unsafe(naked)]
unsafe extern "C" fn keys_thunk() {
    core::arch::naked_asm!(
        "mov eax, dword ptr [esp]",
        "mov dword ptr [{c}], eax",
        "jmp {f}",
        c = sym KEYS_CALLER,
        f = sym keys_entry,
    )
}

/// The mouse entry's thunk, likewise.
#[unsafe(naked)]
unsafe extern "C" fn mouse_thunk() {
    core::arch::naked_asm!(
        "mov eax, dword ptr [esp]",
        "mov dword ptr [{c}], eax",
        "jmp {f}",
        c = sym MOUSE_CALLER,
        f = sym mouse_entry,
    )
}

/// `ProcessKeyboardMessages(msg, wParam, lParam)`, cdecl, at its entry.
unsafe extern "C" fn keys_entry(msg: u32, wp: usize, lp: isize) {
    note_caller(0, unsafe { KEYS_CALLER });
    let f: unsafe extern "C" fn(u32, usize, isize) = unsafe { core::mem::transmute(KEYS_HANDLER) };
    unsafe { f(msg, wp, lp) };
}

/// `ProcessMouseMessages(hwnd, msg, wParam, lParam)`, cdecl, at its entry.
unsafe extern "C" fn mouse_entry(w: HWND, msg: u32, wp: usize, lp: isize) {
    note_caller(1, unsafe { MOUSE_CALLER });
    let f: unsafe extern "C" fn(HWND, u32, usize, isize) = unsafe { core::mem::transmute(MOUSE_HANDLER) };
    unsafe { f(w, msg, wp, lp) };
}

/// The callers seen at each entry (keyboard, mouse), each logged once: which code hands
/// the engine its messages on this build (README.md, "Input and capture").
static mut CALLERS: [[usize; 4]; 2] = [[0; 4]; 2];

fn note_caller(which: usize, ret: usize) {
    let seen = unsafe { &mut (*core::ptr::addr_of_mut!(CALLERS))[which] };
    if ret == 0 || seen.contains(&ret) {
        return;
    }
    let Some(slot) = seen.iter_mut().find(|s| **s == 0) else { return };
    *slot = ret;
    crate::log!("input: {} called from {}", if which == 0 { "ProcessKeyboardMessages" } else { "ProcessMouseMessages" }, where_is(ret));
}

/// `module+0xoffset` for an address, or the bare address outside every module.
pub fn where_is(a: usize) -> String {
    let mut m: HANDLE = core::ptr::null_mut();
    if a == 0 || unsafe { GetModuleHandleExA(GET_MODULE_HANDLE_EX_FROM_ADDRESS, a as *const u8, &mut m) } == 0 || m.is_null() {
        return crate::sfmt!("{:#x} (in no module)", a);
    }
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(m, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let p = &buf[..n.min(buf.len())];
    let start = p.iter().rposition(|&c| c == b'\\' || c == b'/').map_or(0, |i| i + 1);
    crate::sfmt!("{}+{:#x}", String::from_utf8_lossy(&p[start..]), a - m as usize)
}

/// The windows of the game's thread, logged (class, shown or not, client size, the window
/// procedure and where it is), with which one the engine names as its window: what draws
/// where on a build whose windows are not stock's (PLAN-FLEETOPS.md, FO-6 step 3). `when`
/// names the moment ("the first loading frame"). Nothing here sends a window a message
/// (no title is read: `GetWindowTextA` is a message, and one sent from inside the mission's
/// frame stopped Roots there).
pub fn log_windows(when: &str) {
    let named = window();
    crate::log!("windows at {}: the engine's window {:#x} (thread {})", when, named as usize, unsafe { GetCurrentThreadId() });
    unsafe extern "system" fn each(w: HWND, named: isize) -> i32 {
        log_window(w, w as isize == named);
        1
    }
    unsafe { EnumThreadWindows(GetCurrentThreadId(), each, named as isize) };
}

/// The engine's window alone, as `log_windows` logs each.
pub fn log_engine_window(when: &str) {
    let w = window();
    crate::log!("windows at {}: the engine's window {:#x} (thread {})", when, w as usize, unsafe { GetCurrentThreadId() });
    if !w.is_null() {
        log_window(w, true);
    }
}

fn log_window(w: HWND, engines: bool) {
    let mut cls = [0u8; 64];
    let n = unsafe { GetClassNameA(w, cls.as_mut_ptr(), cls.len() as i32) }.max(0) as usize;
    let proc_ = unsafe { GetWindowLongA(w, GWL_WNDPROC) } as usize;
    let mut c = WinRect::default();
    unsafe { GetClientRect(w, &mut c) };
    let thread = unsafe { GetWindowThreadProcessId(w, core::ptr::null_mut()) };
    crate::log!(
        "windows:   {:#x}{} thread {} class \"{}\" {} client {}x{}, window procedure {}",
        w as usize,
        if engines { " (the engine's)" } else { "" },
        thread,
        String::from_utf8_lossy(&cls[..n.min(cls.len())]),
        if unsafe { IsWindowVisible(w) } != 0 { "shown" } else { "hidden" },
        c.right - c.left,
        c.bottom - c.top,
        where_is(proc_)
    );
}

/// The stock `ProcessKeyboardMessages`, for a message the UI lets through.
///
/// # Safety
/// Called from the hooked site or entry only (`install` has succeeded), on the game's thread.
pub unsafe fn process_keys(msg: u32, wp: usize, lp: isize) {
    let tramp = unsafe { KEYS_TRAMP };
    let at = if tramp != 0 { tramp } else { eng().addr(sym::CORE_PROCESS_KEYS) };
    let stock: unsafe extern "C" fn(u32, usize, isize) = unsafe { core::mem::transmute(at) };
    unsafe { stock(msg, wp, lp) };
}

/// The stock `ProcessMouseMessages`, for a message the UI lets through.
///
/// # Safety
/// Called from the hooked site or entry only (`install` has succeeded), on the game's thread.
pub unsafe fn process_mouse(w: HWND, msg: u32, wp: usize, lp: isize) {
    let tramp = unsafe { MOUSE_TRAMP };
    let at = if tramp != 0 { tramp } else { eng().addr(sym::CORE_PROCESS_MOUSE) };
    let stock: unsafe extern "C" fn(HWND, u32, usize, isize) = unsafe { core::mem::transmute(at) };
    unsafe { stock(w, msg, wp, lp) };
}

/// The game window, as the engine names it.
pub fn window() -> HWND {
    let s = super::storm3d();
    if s == 0 || super::field(s, field::CORE_STORM3D_DEVICES()) == 0 || eng().ready(hook::CORE_WINDOW).is_err() {
        return core::ptr::null_mut();
    }
    let f: unsafe extern "thiscall" fn(usize) -> HWND =
        unsafe { core::mem::transmute(eng().addr(sym::CORE_GET_WINDOW_HANDLE)) };
    unsafe { f(s) }
}
