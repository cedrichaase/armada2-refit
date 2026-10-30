//! The Win32 functions only the UI uses, declared by hand (no Windows crate: the DLL
//! links each through the import library build.rs makes from its `.def`), over the
//! kernel32 ones every plugin shares (`a2rt::win`).
#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub use a2rt::win::*;

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
    pub fn MapVirtualKeyA(code: u32, map: u32) -> u32;
    pub fn GetWindowThreadProcessId(w: HWND, pid: *mut u32) -> u32;
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
    pub fn GetActiveWindow() -> HWND;
    pub fn SetFocus(w: HWND) -> HWND;
    pub fn ClipCursor(r: *const WinRect) -> i32;
    pub fn GetClipCursor(r: *mut WinRect) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn Sleep(ms: u32);
}

// ---- the menu host (engine/dialogs.rs): dialogs, painting, GDI ----------------------

pub type HDC = *mut u8;
pub type HGDIOBJ = *mut u8;
pub type DlgProc = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PaintStruct {
    pub hdc: HDC,
    pub erase: i32,
    pub rc: WinRect,
    pub restore: i32,
    pub inc_update: i32,
    pub reserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BitmapInfoHeader {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bit_count: u16,
    pub compression: u32,
    pub size_image: u32,
    pub xppm: i32,
    pub yppm: i32,
    pub clr_used: u32,
    pub clr_important: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MemoryBasicInformation {
    pub base: usize,
    pub alloc_base: usize,
    pub alloc_protect: u32,
    pub region_size: usize,
    pub state: u32,
    pub protect: u32,
    pub kind: u32,
}

pub const WM_DESTROY: u32 = 0x02;
pub const WM_SETFOCUS: u32 = 0x07;
pub const WM_KILLFOCUS: u32 = 0x08;
pub const WM_SETTEXT: u32 = 0x0c;
pub const WM_GETTEXT: u32 = 0x0d;
pub const WM_GETTEXTLENGTH: u32 = 0x0e;
pub const WM_PAINT: u32 = 0x0f;
pub const WM_ERASEBKGND: u32 = 0x14;
pub const WM_NCDESTROY: u32 = 0x82;
pub const WM_KEYUP: u32 = 0x101;
pub const WM_CHAR: u32 = 0x102;
pub const WM_SYSKEYUP: u32 = 0x105;
pub const WM_SYSCHAR: u32 = 0x106;
pub const WM_KEYLAST: u32 = 0x109;
pub const WM_INITDIALOG: u32 = 0x110;
pub const WM_COMMAND: u32 = 0x111;
pub const WM_TIMER: u32 = 0x113;
pub const WM_MOUSEMOVE: u32 = 0x200;
pub const WM_LBUTTONUP: u32 = 0x202;
pub const WM_LBUTTONDBLCLK: u32 = 0x203;
pub const WM_RBUTTONUP: u32 = 0x205;
pub const WM_MOUSEWHEEL: u32 = 0x20a;
pub const WM_MOUSELAST: u32 = 0x20e;
pub const WM_PRINT: u32 = 0x317;
pub const WM_NULL: u32 = 0;
pub const PRF_NONCLIENT: usize = 2;
pub const PRF_CLIENT: usize = 4;
pub const PRF_CHILDREN: usize = 0x10;
pub const MK_LBUTTON: usize = 1;
pub const MK_RBUTTON: usize = 2;

pub const GW_HWNDNEXT: u32 = 2;
pub const GW_OWNER: u32 = 4;
pub const GW_CHILD: u32 = 5;
pub const SWP_NOSIZE: u32 = 1;
pub const SWP_NOMOVE: u32 = 2;
pub const SWP_NOZORDER: u32 = 4;
pub const SWP_NOACTIVATE: u32 = 0x10;
pub const SWP_SHOWWINDOW: u32 = 0x40;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_DISABLED: u32 = 0x0800_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CAPTION: u32 = 0x00c0_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_THICKFRAME: u32 = 0x0004_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_EX_TOPMOST: u32 = 0x8;
pub const WS_EX_APPWINDOW: u32 = 0x40000;
pub const DS_MODALFRAME: u32 = 0x80;
pub const DS_SYSMODAL: u32 = 0x2;
pub const DS_SETFOREGROUND: u32 = 0x200;
pub const SRCCOPY: u32 = 0x00cc_0020;
pub const BLACKNESS: u32 = 0x0000_0042;
pub const RT_DIALOG: usize = 5;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const MEM_COMMIT: u32 = 0x1000;
pub const VK_RETURN: u8 = 0x0d;
pub const VK_ESCAPE: u8 = 0x1b;

#[link(name = "user32")]
unsafe extern "system" {
    pub fn CreateDialogIndirectParamA(inst: HANDLE, tpl: *const u8, parent: HWND, proc_: DlgProc, lp: isize) -> HWND;
    pub fn DestroyWindow(w: HWND) -> i32;
    pub fn SendMessageA(w: HWND, m: u32, wp: usize, lp: isize) -> isize;
    pub fn PostMessageA(w: HWND, m: u32, wp: usize, lp: isize) -> i32;
    pub fn GetWindow(w: HWND, cmd: u32) -> HWND;
    pub fn GetParent(w: HWND) -> HWND;
    pub fn GetWindowRect(w: HWND, r: *mut WinRect) -> i32;
    pub fn MoveWindow(w: HWND, x: i32, y: i32, cx: i32, cy: i32, repaint: i32) -> i32;
    pub fn SetWindowPos(w: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    pub fn ShowWindow(w: HWND, cmd: i32) -> i32;
    pub fn GetClassNameA(w: HWND, buf: *mut u8, n: i32) -> i32;
    pub fn GetSystemMetrics(i: i32) -> i32;
    pub fn ClientToScreen(w: HWND, p: *mut Point) -> i32;
    pub fn InvalidateRect(w: HWND, r: *const WinRect, erase: i32) -> i32;
    pub fn ValidateRect(w: HWND, r: *const WinRect) -> i32;
    pub fn GetCapture() -> HWND;
    pub fn ReleaseCapture() -> i32;
    pub fn IsChild(parent: HWND, w: HWND) -> i32;
    pub fn GetDC(w: HWND) -> HDC;
    pub fn ReleaseDC(w: HWND, dc: HDC) -> i32;
    pub fn BeginPaint(w: HWND, ps: *mut PaintStruct) -> HDC;
    pub fn EndPaint(w: HWND, ps: *const PaintStruct) -> i32;
    pub fn IsDialogMessageA(dlg: HWND, m: *mut Msg) -> i32;
    pub fn IsWindowVisible(w: HWND) -> i32;
    pub fn IsWindowEnabled(w: HWND) -> i32;
    pub fn GetDlgCtrlID(w: HWND) -> i32;
    pub fn GetWindowTextA(w: HWND, buf: *mut u8, n: i32) -> i32;
    pub fn RedrawWindow(w: HWND, r: *const WinRect, rgn: HANDLE, flags: u32) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn CreateDIBSection(dc: HDC, bi: *const BitmapInfoHeader, usage: u32, bits: *mut *mut u8, sec: HANDLE, off: u32) -> HGDIOBJ;
    pub fn SelectObject(dc: HDC, o: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(o: HGDIOBJ) -> i32;
    pub fn DeleteDC(dc: HDC) -> i32;
    pub fn BitBlt(dst: HDC, x: i32, y: i32, w: i32, h: i32, src: HDC, sx: i32, sy: i32, rop: u32) -> i32;
    pub fn PatBlt(dc: HDC, x: i32, y: i32, w: i32, h: i32, rop: u32) -> i32;
    pub fn SetViewportOrgEx(dc: HDC, x: i32, y: i32, old: *mut Point) -> i32;
    pub fn GdiFlush() -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn FindResourceA(module: HANDLE, name: *const u8, kind: *const u8) -> HANDLE;
    pub fn LoadResource(module: HANDLE, res: HANDLE) -> HANDLE;
    pub fn LockResource(res: HANDLE) -> *const u8;
    pub fn SizeofResource(module: HANDLE, res: HANDLE) -> u32;
    pub fn VirtualQuery(p: *const u8, info: *mut MemoryBasicInformation, n: usize) -> usize;
    pub fn GetProcAddress(module: HANDLE, name: *const u8) -> usize;
}
