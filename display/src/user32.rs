//! Who reads the display's size and who moves the mouse (phase 0, item 6): the
//! `user32` and `gdi32` functions `Armada2.exe` imports for the cursor, the window's
//! rectangles and the screen's metrics, traced like a method of ours. Hooked in the
//! exe's import table (found by walking its import directory, not by address), only
//! while tracing. `MoveWindow` and `SetWindowPos` are left alone: the UI hooks those
//! imports itself for the hosted menus, and stands down if it finds them hooked.
use crate::com::Kind;
use crate::rt::win::*;
use crate::trace::{self, once};
use crate::{log, sfmt};
use core::ffi::c_void;

type P = *mut c_void;

static mut ORIG: [usize; 18] = [0; 18];

macro_rules! hook {
    ($idx:literal $name:ident($($a:ident: $t:ty),*) -> $r:ty) => {
        unsafe extern "system" fn $name($($a: $t),*) -> $r {
            let fp = frame!();
            trace::call(fp, Kind::User32, $idx);
            let f: unsafe extern "system" fn($($t),*) -> $r = unsafe { core::mem::transmute(ORIG[$idx]) };
            unsafe { f($($a),*) }
        }
    };
}

hook!(0 get_cursor_pos(p: P) -> i32);
hook!(3 get_clip_cursor(r: P) -> i32);
hook!(4 get_client_rect(w: P, r: P) -> i32);
hook!(5 get_window_rect(w: P, r: P) -> i32);
hook!(6 client_to_screen(w: P, p: P) -> i32);
hook!(7 screen_to_client(w: P, p: P) -> i32);
hook!(10 enum_display_settings_a(dev: P, n: u32, mode: P) -> i32);
hook!(13 adjust_window_rect(r: P, style: u32, menu: i32) -> i32);
hook!(14 adjust_window_rect_ex(r: P, style: u32, menu: i32, ex: u32) -> i32);
hook!(16 show_cursor(show: i32) -> i32);
hook!(17 set_cursor(c: P) -> P);

unsafe extern "system" fn set_cursor_pos(x: i32, y: i32) -> i32 {
    let fp = frame!();
    trace::call(fp, Kind::User32, 1);
    once(&sfmt!("MOUSE SetCursorPos by {}", trace::chain(fp)));
    let f: unsafe extern "system" fn(i32, i32) -> i32 = unsafe { core::mem::transmute(ORIG[1]) };
    unsafe { f(x, y) }
}

unsafe extern "system" fn clip_cursor(r: *const [i32; 4]) -> i32 {
    let fp = frame!();
    trace::call(fp, Kind::User32, 2);
    let rect = if r.is_null() { sfmt!("none") } else { let v = unsafe { *r }; sfmt!("{},{} {},{}", v[0], v[1], v[2], v[3]) };
    once(&sfmt!("MOUSE ClipCursor {} by {}", rect, trace::chain(fp)));
    let f: unsafe extern "system" fn(*const [i32; 4]) -> i32 = unsafe { core::mem::transmute(ORIG[2]) };
    unsafe { f(r) }
}

unsafe extern "system" fn get_system_metrics(n: i32) -> i32 {
    let fp = frame!();
    trace::call(fp, Kind::User32, 8);
    let f: unsafe extern "system" fn(i32) -> i32 = unsafe { core::mem::transmute(ORIG[8]) };
    let v = unsafe { f(n) };
    once(&sfmt!("MODE GetSystemMetrics({}) -> {} by {}", n, v, trace::caller(fp)));
    v
}

unsafe extern "system" fn change_display_settings_a(mode: *const u8, flags: u32) -> i32 {
    let fp = frame!();
    trace::call(fp, Kind::User32, 9);
    let f: unsafe extern "system" fn(*const u8, u32) -> i32 = unsafe { core::mem::transmute(ORIG[9]) };
    let r = unsafe { f(mode, flags) };
    // DEVMODEA: dmPelsWidth and dmPelsHeight at 108 and 112.
    let (w, h) = if mode.is_null() { (0, 0) } else { unsafe { (*(mode.add(108) as *const u32), *(mode.add(112) as *const u32)) } };
    log!("MODE ChangeDisplaySettingsA {}x{} flags {:#x} -> {} by {}", w, h, flags, r, trace::chain(fp));
    r
}

unsafe extern "system" fn gdi_get_device_caps(dc: P, n: i32) -> i32 {
    let fp = frame!();
    trace::call(fp, Kind::User32, 15);
    let f: unsafe extern "system" fn(P, i32) -> i32 = unsafe { core::mem::transmute(ORIG[15]) };
    let v = unsafe { f(dc, n) };
    once(&sfmt!("MODE GetDeviceCaps({}) -> {} by {}", n, v, trace::caller(fp)));
    v
}

/// The import-table entry of `dll!name` in the exe, or None.
fn iat_slot(base: usize, dll: &str, name: &str) -> Option<*mut usize> {
    unsafe {
        let nt = base + *((base + 0x3c) as *const u32) as usize;
        let dir = nt + 0x18 + 0x60 + 8; // OptionalHeader.DataDirectory[IMPORT]
        let rva = *(dir as *const u32) as usize;
        if rva == 0 {
            return None;
        }
        let mut d = base + rva;
        loop {
            let (oft, nm, ft) = (*(d as *const u32), *((d + 12) as *const u32), *((d + 16) as *const u32));
            if nm == 0 && ft == 0 {
                return None;
            }
            let dname = crate::rt::cstr_at((base + nm as usize) as *const u8, 64);
            if dname.eq_ignore_ascii_case(dll) {
                let names = base + if oft != 0 { oft } else { ft } as usize;
                let iat = base + ft as usize;
                let mut i = 0;
                loop {
                    let t = *((names + 4 * i) as *const u32);
                    if t == 0 {
                        break;
                    }
                    if t & 0x8000_0000 == 0 && crate::rt::cstr_at((base + t as usize + 2) as *const u8, 64) == name {
                        return Some((iat + 4 * i) as *mut usize);
                    }
                    i += 1;
                }
            }
            d += 20;
        }
    }
}

/// Points the exe's imports at the hooks above. Returns how many were hooked.
pub fn install() -> usize {
    let base = unsafe { GetModuleHandleA(core::ptr::null()) } as usize;
    let table: [(&str, &str, usize, usize); 16] = [
        ("user32.dll", "GetCursorPos", 0, get_cursor_pos as *const () as usize),
        ("user32.dll", "SetCursorPos", 1, set_cursor_pos as *const () as usize),
        ("user32.dll", "ClipCursor", 2, clip_cursor as *const () as usize),
        ("user32.dll", "GetClipCursor", 3, get_clip_cursor as *const () as usize),
        ("user32.dll", "GetClientRect", 4, get_client_rect as *const () as usize),
        ("user32.dll", "GetWindowRect", 5, get_window_rect as *const () as usize),
        ("user32.dll", "ClientToScreen", 6, client_to_screen as *const () as usize),
        ("user32.dll", "ScreenToClient", 7, screen_to_client as *const () as usize),
        ("user32.dll", "GetSystemMetrics", 8, get_system_metrics as *const () as usize),
        ("user32.dll", "ChangeDisplaySettingsA", 9, change_display_settings_a as *const () as usize),
        ("user32.dll", "EnumDisplaySettingsA", 10, enum_display_settings_a as *const () as usize),
        ("user32.dll", "AdjustWindowRect", 13, adjust_window_rect as *const () as usize),
        ("user32.dll", "AdjustWindowRectEx", 14, adjust_window_rect_ex as *const () as usize),
        ("gdi32.dll", "GetDeviceCaps", 15, gdi_get_device_caps as *const () as usize),
        ("user32.dll", "ShowCursor", 16, show_cursor as *const () as usize),
        ("user32.dll", "SetCursor", 17, set_cursor as *const () as usize),
    ];
    let mut n = 0;
    for (dll, name, idx, f) in table {
        let Some(slot) = iat_slot(base, dll, name) else { continue };
        let cur = unsafe { *slot };
        // Bound: an address outside the exe (an unbound entry is an RVA to the name).
        let image = unsafe { *((base + *((base + 0x3c) as *const u32) as usize + 0x50) as *const u32) } as usize;
        if cur < 0x10000 || (cur >= base && cur < base + image) {
            log!("user32: {}!{} not bound yet, left alone", dll, name);
            continue;
        }
        unsafe { ORIG[idx] = cur };
        let mut old = 0u32;
        unsafe {
            if VirtualProtect(slot as *mut u8, 4, PAGE_EXECUTE_READWRITE, &mut old) != 0 {
                *slot = f;
                VirtualProtect(slot as *mut u8, 4, old, &mut old);
                n += 1;
            }
        }
    }
    log!("user32: {} imports of Armada2.exe traced", n);
    n
}
