//! The plugin's own runtime: allocator, memory functions, Win32, log, ini, files,
//! clock. Everything above it is ordinary `core` + `alloc` Rust.
pub mod mem;
pub mod win;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use win::*;

static GAME_DIR: AtomicPtr<String> = AtomicPtr::new(core::ptr::null_mut());
static LOG_ON: AtomicBool = AtomicBool::new(true);
static LOG_FILE: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

/// NUL-terminated copy of `s`, for the A-suffixed Win32 calls.
pub fn cstr(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

/// The game directory, with a trailing backslash (the directory of Armada2.exe).
pub fn game_dir() -> &'static str {
    let p = GAME_DIR.load(Ordering::Relaxed);
    if p.is_null() { "" } else { unsafe { (*p).as_str() } }
}

/// A path under the game directory.
pub fn game_path(rel: &str) -> String {
    let mut s = String::from(game_dir());
    s.push_str(rel);
    s
}

pub fn init() {
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(core::ptr::null_mut(), buf.as_mut_ptr(), 300) } as usize;
    let mut cut = 0;
    for (i, &c) in buf[..n].iter().enumerate() {
        if c == b'\\' || c == b'/' {
            cut = i + 1;
        }
    }
    let dir = String::from(core::str::from_utf8(&buf[..cut]).unwrap_or(""));
    GAME_DIR.store(alloc::boxed::Box::into_raw(alloc::boxed::Box::new(dir)), Ordering::Relaxed);
}

/// Opens the log, truncating it. `on` false: nothing is written.
pub fn log_open(name: &str, on: bool) {
    LOG_ON.store(on, Ordering::Relaxed);
    if !on {
        return;
    }
    let p = cstr(&game_path(name));
    let h = unsafe {
        CreateFileA(p.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, core::ptr::null_mut(),
            CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut())
    };
    if h != INVALID_HANDLE {
        LOG_FILE.store(h, Ordering::Relaxed);
    }
}

pub fn log_str(s: &str) {
    if !LOG_ON.load(Ordering::Relaxed) {
        return;
    }
    let h = LOG_FILE.load(Ordering::Relaxed);
    if h.is_null() {
        return;
    }
    let mut w = 0u32;
    unsafe {
        WriteFile(h, s.as_ptr(), s.len() as u32, &mut w, core::ptr::null_mut());
        WriteFile(h, b"\r\n".as_ptr(), 2, &mut w, core::ptr::null_mut());
    }
}

pub fn log_fmt(args: core::fmt::Arguments) {
    if !LOG_ON.load(Ordering::Relaxed) {
        return;
    }
    let mut s = String::new();
    let _ = s.write_fmt(args);
    log_str(&s);
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::rt::log_fmt(format_args!($($t)*)) };
}

/// Reads a whole file; None if it cannot be opened.
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    let p = cstr(path);
    unsafe {
        let h = CreateFileA(p.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, core::ptr::null_mut(),
            OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut());
        if h == INVALID_HANDLE {
            return None;
        }
        let n = GetFileSize(h, core::ptr::null_mut()) as usize;
        let mut v: Vec<u8> = Vec::with_capacity(n);
        let mut got = 0u32;
        let ok = ReadFile(h, v.as_mut_ptr(), n as u32, &mut got, core::ptr::null_mut());
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        v.set_len(got as usize);
        Some(v)
    }
}

pub fn write_file(path: &str, data: &[u8]) -> bool {
    let p = cstr(path);
    unsafe {
        let h = CreateFileA(p.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ, core::ptr::null_mut(),
            CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut());
        if h == INVALID_HANDLE {
            return false;
        }
        let mut w = 0u32;
        let ok = WriteFile(h, data.as_ptr(), data.len() as u32, &mut w, core::ptr::null_mut());
        CloseHandle(h);
        ok != 0
    }
}

pub fn file_exists(path: &str) -> bool {
    let p = cstr(path);
    let a = unsafe { GetFileAttributesA(p.as_ptr()) };
    a != INVALID_FILE_ATTRIBUTES && a & FILE_ATTRIBUTE_DIRECTORY == 0
}

/// File names (not paths) matching a pattern such as `C:\dir\*.sod`.
pub fn list_files(pattern: &str) -> Vec<String> {
    let p = cstr(pattern);
    let mut out = Vec::new();
    unsafe {
        let mut fd: FindData = core::mem::zeroed();
        let h = FindFirstFileA(p.as_ptr(), &mut fd);
        if h == INVALID_HANDLE {
            return out;
        }
        loop {
            let n = fd.name.iter().position(|&c| c == 0).unwrap_or(260);
            if fd.attrs & FILE_ATTRIBUTE_DIRECTORY == 0 {
                out.push(String::from_utf8_lossy(&fd.name[..n]).into_owned());
            }
            if FindNextFileA(h, &mut fd) == 0 {
                break;
            }
        }
        FindClose(h);
    }
    out
}

/// Entries matching a pattern, files and directories, as (name, is_directory).
pub fn list_entries(pattern: &str) -> Vec<(String, bool)> {
    let p = cstr(pattern);
    let mut out = Vec::new();
    unsafe {
        let mut fd: FindData = core::mem::zeroed();
        let h = FindFirstFileA(p.as_ptr(), &mut fd);
        if h == INVALID_HANDLE {
            return out;
        }
        loop {
            let n = fd.name.iter().position(|&c| c == 0).unwrap_or(260);
            out.push((String::from_utf8_lossy(&fd.name[..n]).into_owned(), fd.attrs & FILE_ATTRIBUTE_DIRECTORY != 0));
            if FindNextFileA(h, &mut fd) == 0 {
                break;
            }
        }
        FindClose(h);
    }
    out
}

/// Seconds from a monotonic counter.
pub fn now() -> f64 {
    let mut f = 0i64;
    let mut c = 0i64;
    unsafe {
        QueryPerformanceFrequency(&mut f);
        QueryPerformanceCounter(&mut c);
    }
    if f == 0 { 0.0 } else { c as f64 / f as f64 }
}

/// An ini file read through GetPrivateProfile* (path absolute).
pub struct Ini {
    path: Vec<u8>,
}

impl Ini {
    pub fn open(path: &str) -> Ini {
        Ini { path: cstr(path) }
    }
    pub fn exists(&self) -> bool {
        file_exists(core::str::from_utf8(&self.path[..self.path.len() - 1]).unwrap_or(""))
    }
    pub fn string(&self, sec: &str, key: &str) -> Option<String> {
        let (s, k) = (cstr(sec), cstr(key));
        let mut buf = alloc::vec![0u8; 2048];
        let n = unsafe {
            GetPrivateProfileStringA(s.as_ptr(), k.as_ptr(), b"\x01\0".as_ptr(), buf.as_mut_ptr(), 2048, self.path.as_ptr())
        } as usize;
        if n == 1 && buf[0] == 1 {
            return None;
        }
        // Strip a trailing ; comment and whitespace.
        let mut t = &buf[..n];
        if let Some(c) = t.iter().position(|&c| c == b';') {
            t = &t[..c];
        }
        Some(String::from(String::from_utf8_lossy(t).trim()))
    }
    pub fn int(&self, sec: &str, key: &str, def: i32) -> i32 {
        self.string(sec, key).and_then(|s| parse_i32(&s)).unwrap_or(def)
    }
    pub fn float(&self, sec: &str, key: &str, def: f32) -> f32 {
        self.string(sec, key).and_then(|s| parse_f32(&s)).unwrap_or(def)
    }
    pub fn floats<const N: usize>(&self, sec: &str, key: &str, def: [f32; N]) -> [f32; N] {
        match self.string(sec, key) {
            None => def,
            Some(s) => {
                let mut out = def;
                for (i, part) in s.split([',', ' ', '\t']).filter(|p| !p.is_empty()).take(N).enumerate() {
                    if let Some(v) = parse_f32(part) {
                        out[i] = v;
                    }
                }
                out
            }
        }
    }
    pub fn sections(&self) -> Vec<String> {
        let mut buf = alloc::vec![0u8; 32768];
        let n = unsafe { GetPrivateProfileSectionNamesA(buf.as_mut_ptr(), 32768, self.path.as_ptr()) } as usize;
        buf[..n].split(|&c| c == 0).filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned()).collect()
    }
}

pub fn parse_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(h, 16).ok().map(|v| v as i32);
    }
    s.parse::<i32>().ok()
}

pub fn parse_f32(s: &str) -> Option<f32> {
    s.trim().trim_end_matches(['f', 'F']).parse::<f32>().ok()
}

/// ASCII lower-case copy.
pub fn lower(s: &str) -> String {
    s.chars().map(|c| c.to_ascii_lowercase()).collect()
}

/// A NUL-terminated C string at `p`, as &str (lossy ASCII), at most `max` bytes.
///
/// # Safety
/// `p` must be readable up to its terminator or `max` bytes.
pub unsafe fn cstr_at<'a>(p: *const u8, max: usize) -> &'a str {
    if p.is_null() {
        return "";
    }
    let mut n = 0;
    unsafe {
        while n < max && *p.add(n) != 0 {
            n += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(p, n)).unwrap_or("")
    }
}
