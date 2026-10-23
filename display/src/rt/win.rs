//! The kernel32 functions d3d8.dll uses, declared by hand (no Windows crate: the
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

/// `MEMORY_BASIC_INFORMATION`, for telling which module an address is in.
#[repr(C)]
#[derive(Default)]
pub struct MemInfo {
    pub base: usize,
    pub alloc_base: usize,
    pub alloc_protect: u32,
    pub size: usize,
    pub state: u32,
    pub protect: u32,
    pub mtype: u32,
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
    pub fn CreateDirectoryA(name: *const u8, sec: *mut u8) -> i32;
    pub fn DeleteFileA(name: *const u8) -> i32;
    pub fn GetEnvironmentVariableA(name: *const u8, buf: *mut u8, n: u32) -> u32;
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
    pub fn LoadLibraryA(name: *const u8) -> HANDLE;
    pub fn GetProcAddress(module: HANDLE, name: *const u8) -> usize;
    pub fn VirtualQuery(p: *const u8, info: *mut MemInfo, n: usize) -> usize;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetSystemDirectoryA(buf: *mut u8, n: u32) -> u32;
    pub fn MoveFileExA(from: *const u8, to: *const u8, flags: u32) -> i32;
    pub fn CreateEventA(sec: *mut u8, manual: i32, initial: i32, name: *const u8) -> HANDLE;
    pub fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
}
