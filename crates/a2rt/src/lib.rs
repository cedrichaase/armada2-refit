//! The plugins' own runtime, one copy for Display8 (`d3d8.dll`), `Render3D.asi` and
//! `UI.asi`: allocator, memory functions, Win32, log, ini, files, the animation clock's
//! switch, the patching of `Armada2.exe` in memory (`hook`), frame dumps (`dump`,
//! feature `dump`), the settings descriptors (`settings`) and the player's overlay in
//! front of every ini of the game directory (`Ini`, `overlay`). Everything above it is ordinary `core` + `alloc` Rust. Each plugin
//! reaches it as its own `rt` (`pub mod rt { pub use a2rt::*; }`), with `log!` and
//! `sfmt!` re-exported at its root, and keeps its own statics: an rlib is linked into
//! each DLL separately, so each has its own log, game directory and ini cache.
//!
//! The allocator (`mem::Heap`) and the panic handler are declared by each DLL, not
//! here: the handler's last log line names the plugin and how to play without it, and
//! a `#[global_allocator]` or `#[panic_handler]` in this crate would be forced on
//! everything that links it (the replayer and vkprobe bring their own, and the host
//! cannot test a crate that has one). `alloc::format!` does not link: use `sfmt!`.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

pub mod clock;
#[cfg(feature = "dump")]
pub mod dump;
pub mod hook;
pub mod mem;
pub mod overlay;
pub mod settings;
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

/// The folder the logs go in: `%LOCALAPPDATA%\armada2-refit\logs` (beside Display8's
/// pipeline cache), made if missing; None when `LOCALAPPDATA` is unset or the folder
/// cannot be made.
pub fn log_dir() -> Option<String> {
    let mut d = env("LOCALAPPDATA")?;
    d.push_str("\\armada2-refit");
    if !create_dir(&d) {
        return None;
    }
    d.push_str("\\logs");
    create_dir(&d).then_some(d)
}

/// Whether `log!` has a file to write to (an overlay value read before it is open is
/// noted again by the next read, not marked as logged).
pub(crate) fn log_is_open() -> bool {
    LOG_ON.load(Ordering::Relaxed) && !LOG_FILE.load(Ordering::Relaxed).is_null()
}

fn open_log(path: &str) -> HANDLE {
    let p = cstr(path);
    unsafe {
        CreateFileA(p.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, core::ptr::null_mut(),
            CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut())
    }
}

/// Opens the log, truncating it: `%LOCALAPPDATA%\armada2-refit\logs\<name>`, or the
/// game directory when that is unset or not writable. The first line says where.
/// `on` false: nothing is written.
pub fn log_open(name: &str, on: bool) {
    LOG_ON.store(on, Ordering::Relaxed);
    if !on {
        return;
    }
    let mut path = String::new();
    let mut h = INVALID_HANDLE;
    if let Some(mut d) = log_dir() {
        d.push('\\');
        d.push_str(name);
        h = open_log(&d);
        path = d;
    }
    let mut fell_back = false;
    if h == INVALID_HANDLE {
        path = game_path(name);
        h = open_log(&path);
        fell_back = true;
    }
    if h != INVALID_HANDLE {
        LOG_FILE.store(h, Ordering::Relaxed);
        if fell_back {
            log_fmt(format_args!("log: {} (LOCALAPPDATA unset or not writable)", path));
        } else {
            log_fmt(format_args!("log: {}", path));
        }
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

/// `format!` without `alloc::format!`, which does not link: the prebuilt `alloc` is
/// compiled to unwind, and its `format_inner` needs the C++ runtime's frame handler.
/// Writing into a `String` does not.
pub fn fmt_string(args: core::fmt::Arguments) -> String {
    let mut s = String::new();
    let _ = s.write_fmt(args);
    s
}

#[macro_export]
macro_rules! sfmt {
    ($($t:tt)*) => { $crate::fmt_string(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::log_fmt(format_args!($($t)*)) };
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

/// Renames a file over another (`MOVEFILE_REPLACE_EXISTING`): a reader sees the old
/// file or the new one, never half of one.
pub fn move_file(from: &str, to: &str) -> bool {
    let (f, t) = (cstr(from), cstr(to));
    unsafe { MoveFileExA(f.as_ptr(), t.as_ptr(), 1) != 0 }
}

/// Deletes a file; false if it could not be.
pub fn delete_file(path: &str) -> bool {
    let p = cstr(path);
    unsafe { DeleteFileA(p.as_ptr()) != 0 }
}

/// An environment variable of the process; None when it is not set or empty.
pub fn env(name: &str) -> Option<String> {
    let n = cstr(name);
    let mut buf = [0u8; 520];
    let got = unsafe { GetEnvironmentVariableA(n.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if got == 0 || got >= buf.len() {
        return None;
    }
    core::str::from_utf8(&buf[..got]).ok().map(String::from)
}

/// Makes a directory (its parent must exist); true if it exists afterwards.
pub fn create_dir(path: &str) -> bool {
    let p = cstr(path);
    unsafe {
        CreateDirectoryA(p.as_ptr(), core::ptr::null_mut());
        let a = GetFileAttributesA(p.as_ptr());
        a != INVALID_FILE_ATTRIBUTES && a & FILE_ATTRIBUTE_DIRECTORY != 0
    }
}

/// A file written a piece at a time (frame dumps, too big to build in memory first).
/// Closed on drop.
pub struct Writer {
    h: HANDLE,
    pub ok: bool,
}

impl Writer {
    pub fn create(path: &str) -> Option<Writer> {
        let p = cstr(path);
        let h = unsafe {
            CreateFileA(p.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ, core::ptr::null_mut(),
                CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut())
        };
        if h == INVALID_HANDLE { None } else { Some(Writer { h, ok: true }) }
    }
    pub fn write(&mut self, data: &[u8]) {
        let mut w = 0u32;
        if unsafe { WriteFile(self.h, data.as_ptr(), data.len() as u32, &mut w, core::ptr::null_mut()) } == 0
            || w as usize != data.len()
        {
            self.ok = false;
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.h) };
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

/// An ini file (path absolute), read as `GetPrivateProfile*` reads one (`ini_cache`).
/// Every getter goes through `lookup`, the one place a value is found.
///
/// An ini directly in the game directory has the player's overlay in front of it
/// (`RefitPlayer.ini`, tools/settings/README.md): a key is looked up in the overlay's
/// `[<Stem>.<Section>]` first (`<Stem>`: the file's name without `.ini`), then in the
/// file. `open_plain` reads the file alone.
pub struct Ini {
    path: Vec<u8>,
    /// The overlay's section prefix ("Render3D"), or None: no overlay.
    stem: Option<String>,
    /// The file's name as given ("Render3D.ini"), for the log line.
    name: String,
}

impl Ini {
    pub fn open(path: &str) -> Ini {
        let (dir, name) = match path.rfind(['\\', '/']) {
            Some(i) => (&path[..i + 1], &path[i + 1..]),
            None => ("", path),
        };
        let stem = (!game_dir().is_empty()
            && same_path(dir, game_dir())
            && name.len() > 4
            && name[name.len() - 4..].eq_ignore_ascii_case(".ini")
            && !name.eq_ignore_ascii_case(overlay::FILE)
            && !name.eq_ignore_ascii_case(overlay::OLD_FILE))
        .then(|| String::from(&name[..name.len() - 4]));
        Ini { path: cstr(path), stem, name: String::from(name) }
    }
    /// The file alone, without the overlay (the overlay's writer; recipe files).
    pub fn open_plain(path: &str) -> Ini {
        let mut i = Ini::open(path);
        i.stem = None;
        i
    }
    /// The overlay's section prefix of this file ("Render3D"), if it has an overlay.
    pub fn stem(&self) -> Option<&str> {
        self.stem.as_deref()
    }
    pub fn exists(&self) -> bool {
        file_exists(core::str::from_utf8(&self.path[..self.path.len() - 1]).unwrap_or(""))
    }
    /// The value of `key` in `[sec]` as written, or None when there is none: the
    /// overlay's when it holds one that is not empty, else the file's.
    fn lookup(&self, sec: &str, key: &str) -> Option<String> {
        if let Some(stem) = &self.stem {
            let osec = sfmt!("{}.{}", stem, sec);
            if let Some(v) = ini_cache::value(&overlay::path_c(), &osec, key) {
                let shown = strip_value(&v);
                if !shown.is_empty() {
                    overlay::note(&osec, key, shown, &self.name, || {
                        ini_cache::value(&self.path, sec, key).map(|m| String::from(strip_value(&m)))
                    });
                    return Some(v);
                }
            }
        }
        ini_cache::value(&self.path, sec, key)
    }
    pub fn string(&self, sec: &str, key: &str) -> Option<String> {
        let v = self.lookup(sec, key)?;
        Some(String::from(strip_value(&v)))
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
    /// The file's own sections, never the overlay's.
    pub fn sections(&self) -> Vec<String> {
        ini_cache::sections(&self.path)
    }
}

/// Two directories, the same without regard to case or slash direction.
fn same_path(a: &str, b: &str) -> bool {
    let n = |c: u8| if c == b'/' { b'\\' } else { c.to_ascii_lowercase() };
    a.len() == b.len() && a.bytes().zip(b.bytes()).all(|(x, y)| n(x) == n(y))
}

/// A value as `Ini::string` gives it: a trailing `;` comment and whitespace dropped.
fn strip_value(v: &str) -> &str {
    match v.find(';') {
        Some(c) => v[..c].trim(),
        None => v.trim(),
    }
}

/// Marks every cached ini stale, so that the next lookup in each looks at its file
/// again (`<Plugin>_SettingsChanged`, after the overlay was written).
pub fn ini_refresh_all() {
    ini_cache::refresh_all();
}

/// The write time and size of a file, None when it is missing (what `ini_cache` reads
/// a file again on; the UI's settings hub watches the overlay by it).
pub fn file_stamp(path: &str) -> Option<([u32; 2], u32)> {
    ini_cache::stamp(&cstr(path))
}

/// Ini files read once and kept parsed, read again only when the file's write time or
/// size changes (looked at no more than twice a second). Wine's `GetPrivateProfile*`
/// opens and parses the whole file on every call, which made the modules' periodic
/// re-reads of their settings cost tens of milliseconds a time (README.md, "Timing").
/// The parser is `a2ini::parse_into`, with `GetPrivateProfileString`'s rules.
mod ini_cache {
    use super::win::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    struct File {
        path: Vec<u8>,
        stamp: Option<([u32; 2], u32)>,
        checked: f64,
        parsed: a2ini::Parsed,
    }

    static mut FILES: Vec<File> = Vec::new();

    pub fn stamp(path: &[u8]) -> Option<([u32; 2], u32)> {
        let mut d: FindData = unsafe { core::mem::zeroed() };
        let h = unsafe { FindFirstFileA(path.as_ptr(), &mut d) };
        if h == INVALID_HANDLE {
            return None;
        }
        unsafe { FindClose(h) };
        Some((d.written, d.size_low))
    }

    fn parse(f: &mut File) {
        let name = core::str::from_utf8(&f.path[..f.path.len().saturating_sub(1)]).unwrap_or("");
        match super::read_file(name) {
            Some(bytes) => a2ini::parse_into(&String::from_utf8_lossy(&bytes), &mut f.parsed),
            None => f.parsed.clear(),
        }
    }

    #[allow(static_mut_refs)]
    fn file(path: &[u8]) -> &'static File {
        let files = unsafe { &mut FILES };
        let now = super::now();
        let i = match files.iter().position(|f| f.path == path) {
            Some(i) => i,
            None => {
                files.push(File { path: path.to_vec(), stamp: None, checked: f64::NEG_INFINITY, parsed: a2ini::Parsed::default() });
                files.len() - 1
            }
        };
        let f = &mut files[i];
        if now - f.checked >= 0.5 {
            f.checked = now;
            let s = stamp(path);
            if s != f.stamp || s.is_none() && !f.parsed.entries.is_empty() {
                f.stamp = s;
                parse(f);
            }
        }
        &files[i]
    }

    pub fn value(path: &[u8], sec: &str, key: &str) -> Option<String> {
        file(path).parsed.value(sec, key).map(String::from)
    }

    pub fn sections(path: &[u8]) -> Vec<String> {
        file(path).parsed.sections.clone()
    }

    #[allow(static_mut_refs)]
    pub fn refresh_all() {
        for f in unsafe { FILES.iter_mut() } {
            f.checked = f64::NEG_INFINITY;
        }
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
