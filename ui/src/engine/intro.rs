//! The launch reels and the intro: their display mode raised to the desktop's, and
//! their frames drawn by the UI, so neither needs the cutscene proxy (README.md,
//! "The intro without the cutscene proxy").
//!
//! `Program::PlayIntroMovie` looks up a 640x480 mode in the engine's mode list, at 32
//! bits (the pushes at 0x4861bf), else at 16 (0x4861e6), and re-creates the device at
//! whatever it found. Display8's stub offers the desktop's mode alone, so the lookup
//! finds nothing and `ST3D_DeviceDirectX8::CreateDevice` reads the missing mode: the
//! game crashed before the first movie wherever the proxy was not installed (the
//! release package). Both lookups ask for the desktop's size instead, at the depth
//! stock asks for, as the proxy does from its DllMain. The proxy is loaded before any
//! ASI (Armada2.exe imports it), so where it is installed it has patched both sites
//! first, and they are left alone.
//!
//! The frames: `NextBinkFrame` (the back-buffer overload, 0x4637f0) locks the back
//! buffer and copies each frame into it with `BinkCopyToBuffer`, the import called at
//! 0x4638af, at (0,0) and unscaled. That call is re-pointed here: the frame is copied
//! into a buffer of ours and handed to `movie::ui_present_movie`, which draws it fitted
//! between black bars at the present that follows; the back buffer is left as it is.
//! The proxy does this itself, and a call through here would hide the return address
//! it tells the intro by, so with the proxy loaded the site stays stock.
use super::hook;
use crate::rt::win::*;
use alloc::vec::Vec;

/// The pushes of `PlayIntroMovie`'s two mode lookups: depth, height 480, width 640.
const MODE_SITES: [(usize, u8); 2] = [(0x4861bf, 0x20), (0x4861e6, 0x10)];
/// `call [__imp__BinkCopyToBuffer@28]` in `NextBinkFrame`.
const COPY_SITE: usize = 0x4638af;
const COPY_STOCK: [u8; 6] = [0xff, 0x15, 0xa0, 0x82, 0x7b, 0x00];
const COPY_IMPORT: usize = 0x7b82a0;
/// `BinkCopyToBuffer`'s surface type for 32-bit BGRX, as the proxy asks for.
const BINK_32: u32 = 3;

type CopyFn = unsafe extern "system" fn(*mut u32, *mut u8, u32, u32, u32, u32, u32) -> i32;

static mut REAL_COPY: usize = 0;
static mut SCRATCH: Vec<u8> = Vec::new();

fn mode_stock(depth: u8) -> [u8; 12] {
    [0x6a, depth, 0x68, 0xe0, 0x01, 0x00, 0x00, 0x68, 0x80, 0x02, 0x00, 0x00]
}

/// Whether the loaded `binkw32.dll` is the cutscene proxy: it exports only the ten
/// functions the game imports, and stock's `BinkGetError` is not one of them.
fn proxy_loaded() -> bool {
    unsafe {
        let m = GetModuleHandleA(c"binkw32.dll".as_ptr() as *const u8);
        !m.is_null() && GetProcAddress(m, c"_BinkGetError@0".as_ptr() as *const u8) == 0
    }
}

/// The intro's mode raised to the desktop's. Logs what it did; false if neither site
/// was patched.
pub fn raise_mode() -> bool {
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    if w < 640 || h < 480 {
        crate::log!("intro: desktop {}x{} is smaller than the intro: its mode is left at 640x480", w, h);
        return false;
    }
    let mut n = 0;
    for &(at, depth) in &MODE_SITES {
        let stock = mode_stock(depth);
        let mut new = stock;
        new[3..7].copy_from_slice(&(h as u32).to_le_bytes());
        new[8..12].copy_from_slice(&(w as u32).to_le_bytes());
        if hook::patch_bytes(at, &stock, &new) {
            n += 1;
        }
    }
    if n > 0 {
        crate::log!("intro: display mode -> {}x{} ({} of 2 sites)", w, h, n);
    } else if proxy_loaded() {
        crate::log!("intro: display mode left to the cutscene proxy");
    } else {
        crate::log!("intro: the mode lookups are not stock: left as they are");
    }
    n > 0
}

/// `NextBinkFrame`'s copy re-pointed at `copy`, unless the proxy is loaded.
pub fn route_frames() -> bool {
    if proxy_loaded() {
        crate::log!("intro: frames left to the cutscene proxy");
        return false;
    }
    let real = unsafe { super::rd::<usize>(COPY_IMPORT) };
    if real == 0 || unsafe { core::slice::from_raw_parts(COPY_SITE as *const u8, 6) } != COPY_STOCK {
        crate::log!("intro: the frame copy is not stock: frames copied as the engine does");
        return false;
    }
    unsafe { REAL_COPY = real };
    // call rel32 and a nop, in place of call [import].
    let rel = (copy as *const () as i64 - (COPY_SITE as i64 + 5)) as i32;
    let mut new = [0xe8, 0, 0, 0, 0, 0x90];
    new[1..5].copy_from_slice(&rel.to_le_bytes());
    let ok = hook::patch_bytes(COPY_SITE, &COPY_STOCK, &new);
    crate::log!("intro: {}", if ok { "frames drawn by the UI, fitted to the screen" } else { "could not patch the frame copy" });
    ok
}

/// `BinkCopyToBuffer` at `NextBinkFrame`: the frame to the UI, else the stock copy.
#[allow(static_mut_refs)]
unsafe extern "system" fn copy(bink: *mut u32, dest: *mut u8, pitch: u32, dest_h: u32, x: u32, y: u32, flags: u32) -> i32 {
    let real: CopyFn = unsafe { core::mem::transmute(REAL_COPY) };
    // A BINK starts with its width and height.
    let (w, h) = if bink.is_null() { (0, 0) } else { unsafe { (*bink, *bink.add(1)) } };
    if (2..=8192).contains(&w) && (2..=8192).contains(&h) {
        let row = w as usize * 4;
        let s = unsafe { &mut SCRATCH };
        s.resize(row * h as usize, 0);
        if unsafe { real(bink, s.as_mut_ptr(), row as u32, h, 0, 0, (flags & !15) | BINK_32) } != 0 {
            return 1;
        }
        if crate::movie::ui_present_movie(s.as_ptr(), w as i32, h as i32, row as i32) != 0 {
            return 0;
        }
    }
    unsafe { real(bink, dest, pitch, dest_h, x, y, flags) }
}
