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
use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

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
