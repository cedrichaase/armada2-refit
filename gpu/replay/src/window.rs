//! The window the device is made on, at the recording's size (as vkprobe's,
//! display/vkprobe/src/user32.rs).
#![allow(non_snake_case, clippy::upper_case_acronyms)]
use crate::win::HANDLE;
use core::ffi::c_void;

#[repr(C)]
struct WndClass {
    style: u32,
    proc_: unsafe extern "system" fn(HANDLE, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: HANDLE,
    icon: HANDLE,
    cursor: HANDLE,
    background: HANDLE,
    menu: *const u8,
    name: *const u8,
}

#[repr(C)]
struct Msg {
    hwnd: HANDLE,
    message: u32,
    w: usize,
    l: isize,
    time: u32,
    pt: [i32; 2],
    private: [u32; 2],
}

#[repr(C)]
struct Rect {
    l: i32,
    t: i32,
    r: i32,
    b: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassA(c: *const WndClass) -> u16;
    fn CreateWindowExA(ex: u32, class: *const u8, title: *const u8, style: u32, x: i32, y: i32, w: i32, h: i32, parent: HANDLE, menu: HANDLE, inst: HANDLE, param: *mut c_void) -> HANDLE;
    fn DefWindowProcA(h: HANDLE, m: u32, w: usize, l: isize) -> isize;
    fn PeekMessageA(m: *mut Msg, h: HANDLE, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(m: *const Msg) -> i32;
    fn DispatchMessageA(m: *const Msg) -> isize;
    fn AdjustWindowRect(r: *mut Rect, style: u32, menu: i32) -> i32;
    fn DestroyWindow(h: HANDLE) -> i32;
}

const WS_POPUP: u32 = 0x8000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;

unsafe extern "system" fn wndproc(h: HANDLE, m: u32, w: usize, l: isize) -> isize {
    unsafe { DefWindowProcA(h, m, w, l) }
}

/// A visible window with a `w` x `h` client area at the origin, or null.
pub fn open(w: i32, h: i32) -> HANDLE {
    unsafe {
        let inst = crate::win::GetModuleHandleA(core::ptr::null());
        let class = WndClass {
            style: 0,
            proc_: wndproc,
            cls_extra: 0,
            wnd_extra: 0,
            instance: inst,
            icon: core::ptr::null_mut(),
            cursor: core::ptr::null_mut(),
            background: core::ptr::null_mut(),
            menu: core::ptr::null(),
            name: c"a2replay".as_ptr() as *const u8,
        };
        if RegisterClassA(&class) == 0 {
            return core::ptr::null_mut();
        }
        let mut r = Rect { l: 0, t: 0, r: w, b: h };
        AdjustWindowRect(&mut r, WS_POPUP, 0);
        CreateWindowExA(
            0,
            c"a2replay".as_ptr() as *const u8,
            c"a2replay".as_ptr() as *const u8,
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            r.r - r.l,
            r.b - r.t,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            inst,
            core::ptr::null_mut(),
        )
    }
}

/// Handles what is queued.
pub fn pump() {
    let mut m: Msg = unsafe { core::mem::zeroed() };
    unsafe {
        while PeekMessageA(&mut m, core::ptr::null_mut(), 0, 0, 1) != 0 {
            TranslateMessage(&m);
            DispatchMessageA(&m);
        }
    }
}

pub fn close(h: HANDLE) {
    unsafe { DestroyWindow(h) };
}
