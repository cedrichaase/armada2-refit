//! The kernel32 functions the plugin uses, declared by hand (no Windows crate: the
//! DLL links nothing but kernel32, through the import library build.rs makes).
#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub type HANDLE = *mut u8;
pub const INVALID_HANDLE: HANDLE = usize::MAX as HANDLE;

pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_SHARE_READ: u32 = 1;
pub const FILE_SHARE_WRITE: u32 = 2;
pub const CREATE_ALWAYS: u32 = 2;
pub const OPEN_EXISTING: u32 = 3;
pub const OPEN_ALWAYS: u32 = 4;
pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
pub const FILE_END: u32 = 2;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;
pub const INVALID_FILE_ATTRIBUTES: u32 = 0xffff_ffff;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

#[repr(C)]
pub struct FindData {
    pub attrs: u32,
    pub created: [u32; 2],
    pub accessed: [u32; 2],
    pub written: [u32; 2],
    pub size_high: u32,
    pub size_low: u32,
    pub reserved: [u32; 2],
    pub name: [u8; 260],
    pub alt_name: [u8; 14],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn HeapAlloc(heap: HANDLE, flags: u32, n: usize) -> *mut u8;
    pub fn HeapFree(heap: HANDLE, flags: u32, p: *mut u8) -> i32;
    pub fn HeapReAlloc(heap: HANDLE, flags: u32, p: *mut u8, n: usize) -> *mut u8;
    pub fn GetProcessHeap() -> HANDLE;
    pub fn GetModuleFileNameA(module: HANDLE, buf: *mut u8, n: u32) -> u32;
    pub fn GetModuleHandleA(name: *const u8) -> HANDLE;
    pub fn VirtualProtect(p: *mut u8, n: usize, prot: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(proc_: HANDLE, p: *const u8, n: usize) -> i32;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn CreateFileA(name: *const u8, access: u32, share: u32, sec: *mut u8, disp: u32, flags: u32, tmpl: HANDLE) -> HANDLE;
    pub fn ReadFile(h: HANDLE, buf: *mut u8, n: u32, read: *mut u32, ov: *mut u8) -> i32;
    pub fn WriteFile(h: HANDLE, buf: *const u8, n: u32, written: *mut u32, ov: *mut u8) -> i32;
    pub fn SetFilePointer(h: HANDLE, lo: i32, hi: *mut i32, method: u32) -> u32;
    pub fn GetFileSize(h: HANDLE, hi: *mut u32) -> u32;
    pub fn CloseHandle(h: HANDLE) -> i32;
    pub fn GetFileAttributesA(name: *const u8) -> u32;
    pub fn FindFirstFileA(pattern: *const u8, data: *mut FindData) -> HANDLE;
    pub fn FindNextFileA(h: HANDLE, data: *mut FindData) -> i32;
    pub fn FindClose(h: HANDLE) -> i32;
    pub fn GetPrivateProfileIntA(sec: *const u8, key: *const u8, def: i32, file: *const u8) -> u32;
    pub fn GetPrivateProfileStringA(sec: *const u8, key: *const u8, def: *const u8, out: *mut u8, n: u32, file: *const u8) -> u32;
    pub fn GetPrivateProfileSectionNamesA(out: *mut u8, n: u32, file: *const u8) -> u32;
    pub fn QueryPerformanceCounter(v: *mut i64) -> i32;
    pub fn QueryPerformanceFrequency(v: *mut i64) -> i32;
    pub fn GetTickCount() -> u32;
    pub fn OutputDebugStringA(s: *const u8);
    pub fn ExitProcess(code: u32) -> !;
    pub fn GetCurrentThreadId() -> u32;
}

// ---- user32 -----------------------------------------------------------------------

pub type HWND = *mut u8;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WinRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Msg {
    pub hwnd: HWND,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub pt: Point,
}

pub const GWL_WNDPROC: i32 = -4;
pub const PM_REMOVE: u32 = 1;

#[link(name = "user32")]
unsafe extern "system" {
    pub fn GetCursorPos(p: *mut Point) -> i32;
    pub fn ScreenToClient(w: HWND, p: *mut Point) -> i32;
    pub fn GetClientRect(w: HWND, r: *mut WinRect) -> i32;
    pub fn GetForegroundWindow() -> HWND;
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn SetWindowLongA(w: HWND, idx: i32, v: isize) -> isize;
    pub fn GetWindowLongA(w: HWND, idx: i32) -> isize;
    pub fn CallWindowProcA(prev: isize, w: HWND, m: u32, wp: usize, lp: isize) -> isize;
    pub fn PeekMessageA(m: *mut Msg, w: HWND, min: u32, max: u32, remove: u32) -> i32;
    pub fn TranslateMessage(m: *const Msg) -> i32;
    pub fn DispatchMessageA(m: *const Msg) -> isize;
    pub fn IsWindow(w: HWND) -> i32;
    pub fn ShowCursor(show: i32) -> i32;
    pub fn GetKeyState(vk: i32) -> i16;
}

// ---- the front end's own loop (engine/shell.rs) --------------------------------------

pub const WM_QUIT: u32 = 0x12;
pub const WM_SETCURSOR: u32 = 0x20;
pub const WM_KEYDOWN: u32 = 0x100;
pub const WM_SYSKEYDOWN: u32 = 0x104;
pub const WM_LBUTTONDOWN: u32 = 0x201;
pub const WM_RBUTTONDOWN: u32 = 0x204;
pub const WM_MBUTTONDOWN: u32 = 0x207;
pub const HTCLIENT: u16 = 1;
pub const IDC_ARROW: usize = 32512;
pub const GWL_STYLE: i32 = -16;

#[link(name = "user32")]
unsafe extern "system" {
    pub fn LoadCursorA(inst: HANDLE, name: usize) -> HANDLE;
    pub fn SetCursor(c: HANDLE) -> HANDLE;
    pub fn PostQuitMessage(code: i32);
    pub fn GetFocus() -> HWND;
    pub fn SetFocus(w: HWND) -> HWND;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn Sleep(ms: u32);
}
