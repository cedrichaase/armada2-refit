//! The few kernel32 calls the recorder makes (files beside the exe, the page the
//! plugins share, the module it is in). Each plugin's `kernel32.def` lists them; they
//! are reached only while a recording is asked for.
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;

type Handle = *mut c_void;
const INVALID: Handle = usize::MAX as Handle;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileA(name: *const u8, access: u32, share: u32, sec: *mut u8, disp: u32, flags: u32, tmpl: Handle) -> Handle;
    fn ReadFile(h: Handle, buf: *mut u8, n: u32, read: *mut u32, ov: *mut u8) -> i32;
    fn WriteFile(h: Handle, buf: *const u8, n: u32, written: *mut u32, ov: *mut u8) -> i32;
    fn GetFileSize(h: Handle, hi: *mut u32) -> u32;
    fn CloseHandle(h: Handle) -> i32;
    fn GetFileAttributesA(name: *const u8) -> u32;
    fn CreateDirectoryA(name: *const u8, sec: *mut u8) -> i32;
    fn GetModuleFileNameA(module: Handle, buf: *mut u8, n: u32) -> u32;
    fn GetModuleHandleExA(flags: u32, name: *const u8, out: *mut Handle) -> i32;
    fn GetTickCount() -> u32;
    fn GetCurrentProcessId() -> u32;
    fn CreateFileMappingA(file: Handle, sec: *mut u8, prot: u32, hi: u32, lo: u32, name: *const u8) -> Handle;
    fn MapViewOfFile(map: Handle, access: u32, hi: u32, lo: u32, n: usize) -> *mut c_void;
}

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const SHARE_ALL: u32 = 7;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;

fn cstr(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

pub fn ticks() -> u32 {
    unsafe { GetTickCount() }
}

/// The directory of the module `addr` is in (the plugin's own; the exe's for 0), with
/// a trailing backslash, and the module's file name.
pub fn module_of(addr: usize) -> (String, String) {
    let mut h: Handle = core::ptr::null_mut();
    if addr != 0 {
        // FROM_ADDRESS | UNCHANGED_REFCOUNT
        unsafe { GetModuleHandleExA(6, addr as *const u8, &mut h) };
    }
    let mut buf = [0u8; 520];
    let n = unsafe { GetModuleFileNameA(h, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let full = String::from_utf8_lossy(&buf[..n.min(buf.len())]).into_owned();
    match full.rfind(['\\', '/']) {
        Some(i) => (String::from(&full[..=i]), String::from(&full[i + 1..])),
        None => (String::new(), full),
    }
}

/// The exe's directory, with a trailing backslash.
pub fn exe_dir() -> String {
    module_of(0).0
}

pub fn exists(path: &str) -> bool {
    unsafe { GetFileAttributesA(cstr(path).as_ptr()) != u32::MAX }
}

pub fn mkdir(path: &str) -> bool {
    unsafe { CreateDirectoryA(cstr(path).as_ptr(), core::ptr::null_mut()) };
    exists(path)
}

pub fn read(path: &str) -> Option<Vec<u8>> {
    let h = unsafe { CreateFileA(cstr(path).as_ptr(), GENERIC_READ, SHARE_ALL, core::ptr::null_mut(), OPEN_EXISTING, 0, core::ptr::null_mut()) };
    if h == INVALID {
        return None;
    }
    let n = unsafe { GetFileSize(h, core::ptr::null_mut()) };
    let mut v: Vec<u8> = Vec::new();
    let ok = n != u32::MAX && v.try_reserve_exact(n as usize).is_ok();
    if ok {
        v.resize(n as usize, 0);
        let mut at = 0usize;
        while at < v.len() {
            let mut got = 0u32;
            let chunk = (v.len() - at).min(1 << 24) as u32;
            if unsafe { ReadFile(h, v.as_mut_ptr().add(at), chunk, &mut got, core::ptr::null_mut()) } == 0 || got == 0 {
                break;
            }
            at += got as usize;
        }
        v.truncate(at);
    }
    unsafe { CloseHandle(h) };
    ok.then_some(v)
}

pub fn write(path: &str, data: &[u8]) -> bool {
    let h = unsafe { CreateFileA(cstr(path).as_ptr(), GENERIC_WRITE, 0, core::ptr::null_mut(), CREATE_ALWAYS, 0x80, core::ptr::null_mut()) };
    if h == INVALID {
        return false;
    }
    let mut at = 0usize;
    let mut ok = true;
    while at < data.len() {
        let mut put = 0u32;
        let chunk = (data.len() - at).min(1 << 24) as u32;
        if unsafe { WriteFile(h, data.as_ptr().add(at), chunk, &mut put, core::ptr::null_mut()) } == 0 || put == 0 {
            ok = false;
            break;
        }
        at += put as usize;
    }
    unsafe { CloseHandle(h) };
    ok
}

/// The page every plugin's recorder shares (`Local\a2-gpu-record-<pid>`), made by the
/// first to ask; zeros when new.
pub fn shared_page(bytes: usize) -> *mut u8 {
    let mut name = String::from("Local\\a2-gpu-record-");
    let mut pid = unsafe { GetCurrentProcessId() };
    let mut digits = [0u8; 10];
    let mut n = 0;
    loop {
        digits[n] = b'0' + (pid % 10) as u8;
        n += 1;
        pid /= 10;
        if pid == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        name.push(char::from(digits[i]));
    }
    // PAGE_READWRITE; FILE_MAP_ALL_ACCESS
    let m = unsafe { CreateFileMappingA(INVALID, core::ptr::null_mut(), 4, 0, bytes as u32, cstr(&name).as_ptr()) };
    if m.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { MapViewOfFile(m, 0xf001f, 0, 0, bytes) as *mut u8 }
}
