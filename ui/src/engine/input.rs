//! The engine side of input (`crate::input`; README.md, "Input and capture"): the four
//! points of the engine's message path the UI sits at (`WindowProc`'s calls of
//! `ProcessKeyboardMessages` and `ProcessMouseMessages`, and the `PeekMessageA` each
//! drains the queue with), the stock functions called on from there, and the game
//! window as the engine names it. What the UI records and keeps from the engine is
//! `crate::input`'s; this file only patches the sites and calls the engine.
use super::{eng, hook as patch};
use crate::rt::win::HWND;
use crate::syms_core::{field, hook, sym};

/// Patches the four sites to call `keys_message`, `keys_peek`, `mouse_messages` and
/// `mouse_peek` (the addresses of `crate::input`'s handlers). False, with nothing
/// written, when any site is not stock.
pub fn install(keys_message: usize, keys_peek: usize, mouse_messages: usize, mouse_peek: usize) -> bool {
    if eng().ready(hook::CORE_INPUT).is_err() {
        crate::log!("input: standing down");
        return false;
    }
    let (site_keys, site_mouse) = (eng().addr(sym::CORE_SITE_KEYS), eng().addr(sym::CORE_SITE_MOUSE));
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
    patch::patch_bytes(peek_keys, &keys_stock, &call)
        && patch::patch_bytes(peek_mouse, &mouse_stock, &mov)
        && patch::patch_call(site_keys, eng().addr(sym::CORE_PROCESS_KEYS), keys_message)
        && patch::patch_call(site_mouse, eng().addr(sym::CORE_PROCESS_MOUSE), mouse_messages)
}

/// The stock `ProcessKeyboardMessages`, for a message the UI lets through.
///
/// # Safety
/// Called from the hooked site only (`install` has succeeded), on the game's thread.
pub unsafe fn process_keys(msg: u32, wp: usize, lp: isize) {
    let stock: unsafe extern "C" fn(u32, usize, isize) =
        unsafe { core::mem::transmute(eng().addr(sym::CORE_PROCESS_KEYS)) };
    unsafe { stock(msg, wp, lp) };
}

/// The stock `ProcessMouseMessages`, for a message the UI lets through.
///
/// # Safety
/// Called from the hooked site only (`install` has succeeded), on the game's thread.
pub unsafe fn process_mouse(w: HWND, msg: u32, wp: usize, lp: isize) {
    let stock: unsafe extern "C" fn(HWND, u32, usize, isize) =
        unsafe { core::mem::transmute(eng().addr(sym::CORE_PROCESS_MOUSE)) };
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
