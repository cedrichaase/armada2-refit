//! Armada2.exe (GOG patch 1.1): the addresses, offsets and calls the UI runtime reads.
//! Every finding here belongs to the hook that uses it, and is explained in README.md
//! beside the hook (hard rule 9). Names come from the game's own `armada2.map`.
//!
//! One file per drawing context: `ingame` (a mission's HUD), `shell` (the front-end
//! menus), `loading` (the loading screen). Each has an `install()` that patches its
//! hooks, checked against what stock has there, and stands down on a mismatch.
pub mod hook;
pub mod ingame;
pub mod loading;
pub mod shell;

use core::ffi::c_void;

/// Reads a value at an absolute address.
///
/// # Safety
/// `a` must be a valid address of a `T`.
#[inline(always)]
pub unsafe fn rd<T: Copy>(a: usize) -> T {
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

/// Reads a pointer-sized field at `base + off`; 0 when `base` is 0.
#[inline(always)]
pub fn field(base: usize, off: usize) -> usize {
    if base == 0 { 0 } else { unsafe { rd::<usize>(base + off) } }
}

/// `ST3D_GraphicsEngine*` (Storm3D).
pub const STORM3D: usize = 0x7ad508;
pub fn storm3d() -> usize {
    unsafe { rd::<usize>(STORM3D) }
}

/// The game's `IDirect3DDevice8`: Storm3D `+0xc0` index into the device table at
/// `+0xcc`, the `ST3D_DeviceDirectX8` holding it at `+0x90` (render3d/README.md).
pub fn d3d8_device() -> *mut c_void {
    let s = storm3d();
    if s == 0 {
        return core::ptr::null_mut();
    }
    let idx = field(s, 0xc0);
    let dev = field(s + 0xcc, idx * 4);
    field(dev, 0x90) as *mut c_void
}
