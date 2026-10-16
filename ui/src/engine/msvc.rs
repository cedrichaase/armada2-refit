//! The game's C++ runtime (MSVC 6's `msvcp60.dll`), for the engine functions that
//! write their answer into a `std::stringstream` (a button's tooltip, HUD.md).
//!
//! The game imports what it uses from `msvcp60.dll`; we call the same imports through
//! the game's own import table, so the stream is made and freed by the library that
//! the engine's code expects. The object is the most-derived `basic_stringstream`:
//! the engine's functions take it as `basic_stringstream &` and write into its
//! `ostream` part themselves.
use super::rd;
use alloc::string::String;
use alloc::vec::Vec;

/// Import slots in `Armada2.exe`'s import table (each holds the function's address).
/// `basic_stringstream::basic_stringstream(int mode)`: thiscall, the mode and then the
/// hidden "construct the virtual base" flag (1), as the game calls it (`push 1; push 3`).
const IMP_SS_CTOR: usize = 0x7b7c58;
/// `basic_stringstream::str() const`: thiscall, returns a `std::string` through a
/// pointer it is given.
const IMP_SS_STR: usize = 0x7b7db0;
/// `basic_stringstream`'s whole-object destructor (`??_D`, the virtual base included).
const IMP_SS_DTOR: usize = 0x7b7ea8;
/// `basic_string::~basic_string()`.
const IMP_STR_DTOR: usize = 0x7b7dc0;
/// `ios_base::in | ios_base::out`.
const MODE_IN_OUT: i32 = 3;
/// Room for the stream: the engine allocates 0x8c bytes for one; more is harmless.
const STREAM_BYTES: usize = 0x100;

/// Runs `f` with a fresh `std::stringstream` and returns what it wrote, or None when
/// it wrote nothing.
pub fn with_stringstream(f: impl FnOnce(usize)) -> Option<String> {
    unsafe {
        let ctor: unsafe extern "thiscall" fn(usize, i32, i32) -> usize = core::mem::transmute(rd::<usize>(IMP_SS_CTOR));
        let str_: unsafe extern "thiscall" fn(usize, usize) -> usize = core::mem::transmute(rd::<usize>(IMP_SS_STR));
        let dtor: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(IMP_SS_DTOR));
        let sdtor: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(IMP_STR_DTOR));
        // u32s, so the object is 4-aligned.
        let mut obj: Vec<u32> = alloc::vec![0; STREAM_BYTES / 4];
        let ss = obj.as_mut_ptr() as usize;
        ctor(ss, MODE_IN_OUT, 1);
        f(ss);
        // std::string: an allocator byte, then the pointer (+4), the length (+8) and
        // the capacity (+0xc).
        let mut s = [0u32; 4];
        let sp = s.as_mut_ptr() as usize;
        str_(ss, sp);
        let (p, n) = (s[1] as usize, s[2] as usize);
        let out = if p != 0 && n > 0 && n < 4096 {
            let bytes = core::slice::from_raw_parts(p as *const u8, n);
            // Latin-1, as the game's localised strings are.
            Some(bytes.iter().map(|&b| b as char).collect::<String>())
        } else {
            None
        };
        sdtor(sp);
        dtor(ss);
        out
    }
}

/// What an existing `std::stringstream` holds (one the engine made), or None when it
/// is empty.
///
/// # Safety
/// `ss` is a live `basic_stringstream` made by the game's `msvcp60.dll`.
pub unsafe fn stream_text(ss: usize) -> Option<String> {
    if ss == 0 {
        return None;
    }
    let str_: unsafe extern "thiscall" fn(usize, usize) -> usize = unsafe { core::mem::transmute(rd::<usize>(IMP_SS_STR)) };
    with_string(|sp| unsafe {
        str_(ss, sp);
    })
}

/// Runs `f` with an empty `std::string` (all zero, as its default constructor leaves
/// it), returns what `f` left in it, and frees it.
pub fn with_string(f: impl FnOnce(usize)) -> Option<String> {
    unsafe {
        let sdtor: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(IMP_STR_DTOR));
        let mut s = [0u32; 4];
        let sp = s.as_mut_ptr() as usize;
        f(sp);
        let (p, n) = (s[1] as usize, s[2] as usize);
        let out = if p != 0 && n > 0 && n < 4096 {
            Some(core::slice::from_raw_parts(p as *const u8, n).iter().map(|&b| b as char).collect::<String>())
        } else {
            None
        };
        sdtor(sp);
        out
    }
}

/// The imports are filled in by the loader; each must point into `msvcp60.dll`, not at
/// nothing.
pub fn available() -> bool {
    [IMP_SS_CTOR, IMP_SS_STR, IMP_SS_DTOR, IMP_STR_DTOR].iter().all(|&a| unsafe { rd::<usize>(a) } > 0x10000)
}
