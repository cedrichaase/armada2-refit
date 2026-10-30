//! `vkprobe.txt`: the verdict on its first line, then every step with PASS or FAIL and
//! the VkResult, then what was found along the way. It is rewritten whole after every
//! step, so a probe that crashes or hangs still leaves a file naming the step it was on.
use crate::win::*;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

struct Report {
    path: Vec<u8>,
    steps: String,
    body: String,
    /// The first step that failed, and how.
    fail: Option<String>,
    /// The step under way: what the verdict names if the probe never comes back.
    current: String,
    done: bool,
}

static mut R: Option<Report> = None;

fn r() -> &'static mut Report {
    unsafe { R.as_mut().expect("report") }
}

/// `format!` without `alloc::format!`, which does not link (crates/a2rt).
pub fn fmt(args: core::fmt::Arguments) -> String {
    let mut s = String::new();
    let _ = s.write_fmt(args);
    s
}

/// One line into the body of the report.
#[macro_export]
macro_rules! out {
    ($($t:tt)*) => { $crate::report::line(&$crate::report::fmt(format_args!($($t)*))) };
}

/// `vkprobe.txt` beside the exe.
pub fn open() {
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(core::ptr::null_mut(), buf.as_mut_ptr(), 280) } as usize;
    let dir = buf[..n].iter().rposition(|&c| c == b'\\' || c == b'/').map_or(0, |i| i + 1);
    let mut path = buf[..dir].to_vec();
    path.extend_from_slice(b"vkprobe.txt\0");
    unsafe {
        R = Some(Report { path, steps: String::new(), body: String::new(), fail: None, current: String::from("start-up"), done: false });
    }
    save();
}

pub fn line(s: &str) {
    let b = &mut r().body;
    b.push_str(s);
    b.push('\n');
}

pub fn section(title: &str) {
    let b = &mut r().body;
    b.push_str("\n== ");
    b.push_str(title);
    b.push_str(" ==\n");
}

/// A step begins: saved, so that a crash or a hang inside it is reported as its own.
pub fn begin(name: &str) {
    r().current = String::from(name);
    save();
}

pub fn pass(name: &str, detail: &str) {
    step("PASS", name, detail);
}

pub fn fail(name: &str, detail: &str) {
    step("FAIL", name, detail);
    if r().fail.is_none() {
        r().fail = Some(if detail.is_empty() { String::from(name) } else { fmt(format_args!("{} ({})", name, detail)) });
    }
    save();
}

/// A note in the step list that neither passes nor fails (an optional part skipped).
pub fn note(name: &str, detail: &str) {
    step("----", name, detail);
}

fn step(tag: &str, name: &str, detail: &str) {
    let s = &mut r().steps;
    let _ = write!(s, "{}  {}", tag, name);
    if !detail.is_empty() {
        let _ = write!(s, ": {}", detail);
    }
    s.push('\n');
    save();
}

/// The step under way failed in a way that ends the process (a crash, a panic).
pub fn fail_now(what: &str) {
    if unsafe { R.is_none() } {
        return;
    }
    let cur = r().current.clone();
    fail(&cur, what);
}

pub fn finish(ok: bool) {
    r().done = ok;
    save();
}

pub fn save() {
    let rep = r();
    let mut s = String::new();
    match (&rep.fail, rep.done) {
        (Some(f), _) => {
            let _ = writeln!(s, "VERDICT: no: {}", f);
        }
        (None, true) => s.push_str("VERDICT: Vulkan works for Display8\n"),
        (None, false) => {
            let _ = writeln!(s, "VERDICT: no: stopped during {} (crashed or hung; nothing more was written)", rep.current);
        }
    }
    let _ = writeln!(s, "vkprobe {} (32-bit), Armada II Refit -- display/vkprobe/README.md", env!("CARGO_PKG_VERSION"));
    s.push_str("\n== steps ==\n");
    s.push_str(&rep.steps);
    s.push_str(&rep.body);
    unsafe {
        let h = CreateFileA(rep.path.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ, core::ptr::null_mut(), CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, core::ptr::null_mut());
        if h == INVALID_HANDLE {
            return;
        }
        let mut w = 0u32;
        WriteFile(h, s.as_ptr(), s.len() as u32, &mut w, core::ptr::null_mut());
        CloseHandle(h);
    }
}

/// A NUL-terminated string at `p`, at most `max` bytes, lossily.
pub fn cstr_at(p: *const u8, max: usize) -> String {
    let mut s = String::new();
    if p.is_null() {
        return s;
    }
    for i in 0..max {
        let c = unsafe { *p.add(i) };
        if c == 0 {
            break;
        }
        s.push(if c.is_ascii() { c as char } else { '?' });
    }
    s
}

/// What the probe runs on: Wine's version and host (the macOS question is a Wine
/// question), the machine WoW64 reports, and whether this exe is large-address-aware.
pub fn system() {
    section("system");
    unsafe {
        let mut buf = [0u8; 300];
        let n = GetModuleFileNameA(core::ptr::null_mut(), buf.as_mut_ptr(), 280) as usize;
        out!("exe: {}", core::str::from_utf8(&buf[..n]).unwrap_or("?"));
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr() as *const u8);
        let ver = GetProcAddress(ntdll, c"wine_get_version".as_ptr() as *const u8);
        if ver != 0 {
            let f: extern "C" fn() -> *const u8 = core::mem::transmute(ver);
            out!("wine: {}", cstr_at(f(), 100));
            let build = GetProcAddress(ntdll, c"wine_get_build_id".as_ptr() as *const u8);
            if build != 0 {
                let f: extern "C" fn() -> *const u8 = core::mem::transmute(build);
                out!("wine build: {}", cstr_at(f(), 200));
            }
            let host = GetProcAddress(ntdll, c"wine_get_host_version".as_ptr() as *const u8);
            if host != 0 {
                let f: extern "C" fn(*mut *const u8, *mut *const u8) = core::mem::transmute(host);
                let (mut sys, mut rel) = (core::ptr::null(), core::ptr::null());
                f(&mut sys, &mut rel);
                out!("host: {} {}", cstr_at(sys, 100), cstr_at(rel, 100));
            }
        } else {
            out!("wine: no (Windows, or a Wine that hides itself)");
        }
        let k32 = GetModuleHandleA(c"kernel32.dll".as_ptr() as *const u8);
        let w2 = GetProcAddress(k32, c"IsWow64Process2".as_ptr() as *const u8);
        if w2 != 0 {
            let f: extern "system" fn(HANDLE, *mut u16, *mut u16) -> i32 = core::mem::transmute(w2);
            let (mut process, mut native) = (0u16, 0u16);
            if f(GetCurrentProcess(), &mut process, &mut native) != 0 {
                out!("IsWow64Process2: process machine {:#06x}, native machine {:#06x} (0x014c i386, 0x8664 x86-64, 0xaa64 arm64)", process, native);
            }
        }
        // The PE header's Characteristics: 0x20 is IMAGE_FILE_LARGE_ADDRESS_AWARE.
        let base = GetModuleHandleA(core::ptr::null()) as *const u8;
        let pe = base.add(*(base.add(0x3c) as *const u32) as usize);
        let chars = *(pe.add(0x16) as *const u16);
        out!("large-address-aware: {} (Armada2.exe is not; neither is the probe, so it has the game's 2 GB)", if chars & 0x20 != 0 { "yes" } else { "no" });
    }
}
