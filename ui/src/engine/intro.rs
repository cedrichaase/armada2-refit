//! The launch reels and the intro: their frames drawn by the UI, so neither needs the cutscene proxy (README.md,
//! "The intro without the cutscene proxy").
//!
//! The mode: Display8 raises the intro's 640x480 lookups to the desktop's size
//! (`[Window] RaiseIntro`, display/README.md, "The engine's hard-coded modes"); the UI
//! no longer patches them.
//!
//! The frames: `NextBinkFrame` (the back-buffer overload, 0x4637f0) locks the back
//! buffer and copies each frame into it with `BinkCopyToBuffer`, the import called at
//! 0x4638af, at (0,0) and unscaled. That call is re-pointed here: the frame is copied
//! into a buffer of ours and handed to `movie::ui_present_movie`, which draws it fitted
//! between black bars at the present that follows; the back buffer is left as it is.
//! The proxy does this itself, and a call through here would hide the return address
//! it tells the intro by, so with the proxy loaded the site stays stock.
use super::{eng, hook as patch};
use crate::rt::win::*;
use crate::syms_core::{hook, sym};
use alloc::vec::Vec;

// `call [__imp__BinkCopyToBuffer@28]` in `NextBinkFrame` is the profile's
// (`ui.core.toml`, hook `core.intro_frames`).
/// `BinkCopyToBuffer`'s surface type for 32-bit BGRX, as the proxy asks for.
const BINK_32: u32 = 3;

type CopyFn = unsafe extern "system" fn(*mut u32, *mut u8, u32, u32, u32, u32, u32) -> i32;

static mut REAL_COPY: usize = 0;
static mut SCRATCH: Vec<u8> = Vec::new();

/// Whether the loaded `binkw32.dll` is the cutscene proxy: it exports only the ten
/// functions the game imports, and stock's `BinkGetError` is not one of them.
fn proxy_loaded() -> bool {
    unsafe {
        let m = GetModuleHandleA(c"binkw32.dll".as_ptr() as *const u8);
        !m.is_null() && GetProcAddress(m, c"_BinkGetError@0".as_ptr() as *const u8) == 0
    }
}

/// `NextBinkFrame`'s copy re-pointed at `copy`, unless the proxy is loaded.
pub fn route_frames() -> bool {
    if proxy_loaded() {
        crate::log!("intro: frames left to the cutscene proxy");
        return false;
    }
    if eng().ready(hook::CORE_INTRO_FRAMES).is_err() {
        return false;
    }
    let (site, import) = (eng().addr(sym::CORE_BINK_COPY_SITE), eng().addr(sym::CORE_BINK_COPY_IMPORT));
    let real = unsafe { super::rd::<usize>(import) };
    if real == 0 {
        crate::log!("intro: the frame copy is not stock: frames copied as the engine does");
        return false;
    }
    unsafe { REAL_COPY = real };
    // call rel32 and a nop, in place of call [import]; the six bytes are the ones the
    // profile's check has just read.
    let stock: [u8; 6] = unsafe { core::ptr::read_unaligned(site as *const [u8; 6]) };
    let rel = (copy as *const () as i64 - (site as i64 + 5)) as i32;
    let mut new = [0xe8, 0, 0, 0, 0, 0x90];
    new[1..5].copy_from_slice(&rel.to_le_bytes());
    let ok = patch::patch_bytes(site, &stock, &new);
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
