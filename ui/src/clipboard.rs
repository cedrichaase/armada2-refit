//! The Windows clipboard, as plain text in the ANSI code page (read as Latin-1, like the
//! characters the keyboard gives the UI).
use crate::rt::win::HANDLE;
use alloc::string::String;

const CF_TEXT: u32 = 1;
const GMEM_MOVEABLE: u32 = 2;

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner: *mut u8) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(format: u32) -> HANDLE;
    fn SetClipboardData(format: u32, mem: HANDLE) -> HANDLE;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> HANDLE;
    fn GlobalLock(mem: HANDLE) -> *mut u8;
    fn GlobalUnlock(mem: HANDLE) -> i32;
    fn GlobalSize(mem: HANDLE) -> usize;
}

/// The clipboard's text, or None when it holds none. Line breaks and other control
/// characters are left in; the caller decides what a field does with them.
pub fn get() -> Option<String> {
    unsafe {
        if OpenClipboard(core::ptr::null_mut()) == 0 {
            return None;
        }
        let mut out = None;
        let h = GetClipboardData(CF_TEXT);
        if !h.is_null() {
            let p = GlobalLock(h);
            if !p.is_null() {
                let max = GlobalSize(h).min(1 << 20);
                let mut s = String::new();
                for i in 0..max {
                    let b = *p.add(i);
                    if b == 0 {
                        break;
                    }
                    s.push(char::from(b));
                }
                out = Some(s);
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        out
    }
}

/// Puts `s` on the clipboard (characters beyond Latin-1 become '?'). False if the
/// clipboard could not be opened.
pub fn set(s: &str) -> bool {
    unsafe {
        let n = s.chars().count();
        let mem = GlobalAlloc(GMEM_MOVEABLE, n + 1);
        if mem.is_null() {
            return false;
        }
        let p = GlobalLock(mem);
        if p.is_null() {
            return false;
        }
        for (i, c) in s.chars().enumerate() {
            *p.add(i) = if (c as u32) < 256 { c as u32 as u8 } else { b'?' };
        }
        *p.add(n) = 0;
        GlobalUnlock(mem);
        if OpenClipboard(core::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();
        // The clipboard owns the memory once it accepts it.
        let ok = !SetClipboardData(CF_TEXT, mem).is_null();
        CloseClipboard();
        ok
    }
}
