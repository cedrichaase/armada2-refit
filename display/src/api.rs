//! What Display8 exports to the plugins, beyond Direct3D 8 itself: plain C functions,
//! found with `GetProcAddress(GetModuleHandleA("d3d8.dll"), name)`. A plugin that does
//! not find them is on another `d3d8.dll` and works as before.
//!
//! - `display8_version()`: this DLL's version, a NUL-terminated string.
//! - `display8_device_epoch()`: moves on at every `CreateDevice` and every `Reset`
//!   that succeeds, and when the device goes. Whatever a plugin made for the device it
//!   saw at another epoch is gone or must be made again; no plugin need watch the
//!   engine's device pointer for that any more.
//! - `display8_device()`: the engine's device now (an `IDirect3DDevice8` of ours, not
//!   referenced), or null.
//! - `display8_frame()`: frames presented so far.
//! - `display8_abi(caller_major, caller_minor, caller, &major, &minor)`: the contract's
//!   handshake (`gpu/src/display.rs`, "Versions"): Display8's contract version, and the
//!   plugin's logged beside it. Its name and signature never change.
use crate::rt::win::*;
use crate::{log, sfmt};
use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};
use gpu::display::{ABI_MAJOR, ABI_MINOR};

static EPOCH: AtomicU32 = AtomicU32::new(0);
static FRAME: AtomicU32 = AtomicU32::new(0);
static DEVICE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

pub fn device_made(dev: *mut c_void) {
    DEVICE.store(dev, Ordering::Release);
    EPOCH.fetch_add(1, Ordering::AcqRel);
}

pub fn device_reset() {
    EPOCH.fetch_add(1, Ordering::AcqRel);
}

/// The device of ours `dev` is gone (its last reference released).
pub fn device_gone(dev: *mut c_void) {
    if DEVICE.compare_exchange(dev, core::ptr::null_mut(), Ordering::AcqRel, Ordering::Relaxed).is_ok() {
        EPOCH.fetch_add(1, Ordering::AcqRel);
    }
}

pub fn presented() {
    FRAME.fetch_add(1, Ordering::Relaxed);
}

/// The modules that shook hands at our major (`display8_abi`), by module handle.
static SHOOK: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];

/// The first mission's frames (`scene_begun`): 0 before the engine's first loading
/// loop, `LOADING` while it runs, then the frame the check is due at; `DONE` after it.
static MISSION: AtomicU32 = AtomicU32::new(0);
const LOADING: u32 = 1;
const DONE: u32 = u32::MAX;
/// Frames into the first mission at which every plugin of the contract has asked: by then
/// render3d has drawn the mission and the UI its HUD.
const MISSION_FRAMES: u32 = 300;
/// The plugins of the contract.
const CONTRACT_PLUGINS: [&str; 2] = ["Render3D.asi", "UI.asi"];

/// The module `a` is in: its handle and file name.
fn module_of(a: usize) -> (usize, alloc::string::String) {
    let mut mi = MemInfo::default();
    if a == 0 || unsafe { VirtualQuery(a as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 || mi.alloc_base == 0 {
        return (0, sfmt!("a module at {:#x}", a));
    }
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(mi.alloc_base as HANDLE, buf.as_mut_ptr(), 300) } as usize;
    let full = core::str::from_utf8(&buf[..n.min(300)]).unwrap_or("?");
    (mi.alloc_base, alloc::string::String::from(full.rsplit(['\\', '/']).next().unwrap_or(full)))
}

/// The engine's `BeginScene` in stub mode (`fp` its frame pointer), until the first
/// mission has run `MISSION_FRAMES` frames: then a plugin of the contract that is loaded
/// and has not shaken hands is one built before the handshake (display 0.12.0). It
/// finds none of the `display8v1_*` exports and draws through Direct3D 9 only, so
/// nothing of it on Vulkan or Direct3D 12. (Such a plugin calls nothing of Display8's:
/// with no device to draw on it stops before it asks for the epoch.) Forward mode needs
/// no check: there every plugin draws on Direct3D 9.
pub fn scene_begun(fp: usize) {
    let m = MISSION.load(Ordering::Relaxed);
    if m == DONE {
        return;
    }
    let loading = crate::drop::in_loading(fp);
    let now = FRAME.load(Ordering::Relaxed);
    let next = match (m, loading) {
        (_, true) => LOADING,
        (LOADING, false) => now.saturating_add(MISSION_FRAMES).max(2),
        (0, false) => 0,
        (due, false) if now >= due => {
            unshaken();
            DONE
        }
        (due, false) => due,
    };
    MISSION.store(next, Ordering::Relaxed);
}

fn unshaken() {
    for name in CONTRACT_PLUGINS {
        let h = unsafe { GetModuleHandleA(crate::rt::cstr(name).as_ptr()) } as usize;
        if h == 0 || SHOOK.iter().any(|s| s.load(Ordering::Acquire) == h) {
            continue;
        }
        log!(
            "contract: {} is loaded and has not asked for the contract (display8_abi) {} frames into the first mission: built for a Display8 before 0.12.0 (or stood down on its own; its log says), it finds none of the contract's exports and draws through Direct3D 9 only, so nothing of it on Vulkan or Direct3D 12, where the engine's own draws stand in (passthrough). Install d3d8.dll, Render3D.asi and UI.asi from one release",
            name, MISSION_FRAMES
        );
    }
}

/// The contract's handshake (`gpu/src/display.rs`, "Versions"): tells our version,
/// and logs the caller's (`caller`: an address in its module).
#[unsafe(no_mangle)]
pub extern "C" fn display8_abi(caller_major: u32, caller_minor: u32, caller: usize, major: *mut u32, minor: *mut u32) {
    if !major.is_null() {
        unsafe { *major = ABI_MAJOR };
    }
    if !minor.is_null() {
        unsafe { *minor = ABI_MINOR };
    }
    let (h, name) = module_of(caller);
    if caller_major == ABI_MAJOR {
        if h != 0 && !SHOOK.iter().any(|s| s.load(Ordering::Acquire) == h) {
            let _ = SHOOK.iter().find(|s| s.compare_exchange(0, h, Ordering::AcqRel, Ordering::Relaxed).is_ok());
        }
        log!("contract: {} asks for {}.{}; Display8's is {}.{}: drawn through", name, caller_major, caller_minor, ABI_MAJOR, ABI_MINOR);
    } else {
        log!(
            "contract: {} asks for {}.{}; Display8's is {}.{}, another major: it finds none of this Display8's exports and stands down. Install d3d8.dll, Render3D.asi and UI.asi from one release",
            name, caller_major, caller_minor, ABI_MAJOR, ABI_MINOR
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_version() -> *const u8 {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_device_epoch() -> u32 {
    EPOCH.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_device() -> *mut c_void {
    DEVICE.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_frame() -> u32 {
    FRAME.load(Ordering::Relaxed)
}
