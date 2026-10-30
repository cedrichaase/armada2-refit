//! Armada2.exe (GOG patch 1.1): the addresses, offsets and calls the renderer reads.
//! Where each was established: `$A2_DATA/notes/renderer-inventory/` and the plugin
//! READMEs it cites. Names come from the game's own `armada2.map`.
pub use crate::rt::hook;
pub mod obj;
pub mod sound;

use crate::math::{M34, V3};
use core::ffi::c_void;

/// Reads a value at an absolute address.
///
/// # Safety
/// `a` must be a valid, aligned-enough address of a `T`.
#[inline(always)]
pub unsafe fn rd<T: Copy>(a: usize) -> T {
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

/// Reads a pointer-sized field at `base + off`; 0 when `base` is 0.
#[inline(always)]
pub fn field(base: usize, off: usize) -> usize {
    if base == 0 { 0 } else { unsafe { rd::<usize>(base + off) } }
}

#[inline(always)]
pub fn field_f32(base: usize, off: usize) -> f32 {
    if base == 0 { 0.0 } else { unsafe { rd::<f32>(base + off) } }
}

#[inline(always)]
pub fn field_u8(base: usize, off: usize) -> u8 {
    if base == 0 { 0 } else { unsafe { rd::<u8>(base + off) } }
}

pub fn field_m34(base: usize, off: usize) -> M34 {
    if base == 0 { M34::IDENTITY } else { unsafe { rd::<M34>(base + off) } }
}

pub fn field_v3(base: usize, off: usize) -> V3 {
    if base == 0 { V3::ZERO } else { unsafe { rd::<V3>(base + off) } }
}

/// The vtable of an object.
pub fn vtable(obj: usize) -> usize {
    field(obj, 0)
}

// ---- the engine root ------------------------------------------------------------

/// `ST3D_GraphicsEngine*` (Storm3D).
pub const STORM3D: usize = 0x7ad508;
pub fn storm3d() -> usize {
    unsafe { rd::<usize>(STORM3D) }
}

/// The game's `IDirect3DDevice8`: Storm3D `+0xc0` index into the device table at
/// `+0xcc`, the `ST3D_DeviceDirectX8` holding it at `+0x90`.
pub fn d3d8_device() -> *mut c_void {
    let s = storm3d();
    if s == 0 {
        return core::ptr::null_mut();
    }
    let idx = field(s, 0xc0);
    let dev = field(s + 0xcc, idx * 4);
    field(dev, 0x90) as *mut c_void
}

/// The ST3D device object (`ST3D_DeviceDirectX8`).
pub fn st3d_device() -> usize {
    let s = storm3d();
    if s == 0 {
        return 0;
    }
    let idx = field(s, 0xc0);
    field(s + 0xcc, idx * 4)
}

// ---- clock ----------------------------------------------------------------------

/// Sim time in seconds (`Get_Time` reads the float at 0x7b0fc8).
pub fn sim_time() -> f32 {
    unsafe { rd::<f32>(0x7b0fc8) }
}

// ---- lists ----------------------------------------------------------------------

/// `GameObject::objectList` 0x761084: `std::list*`, head node `+4`, each node `+8`
/// the object (`plugins.md` §3).
pub const OBJECT_LIST: usize = 0x761084;

/// Walks a VC6 `std::list` whose pointer sits at `list_ptr_addr`, calling `f` with
/// each element value (the pointer stored at node `+8`).
pub fn for_each_in_list(list_ptr_addr: usize, mut f: impl FnMut(usize)) {
    let list = unsafe { rd::<usize>(list_ptr_addr) };
    if list == 0 {
        return;
    }
    let head = field(list, 4);
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 1_000_000 {
        f(field(n, 8));
        n = field(n, 0);
        guard += 1;
    }
}

/// `GameFeature` list (a `std::vector<GameFeature*>*` at 0x736c84: begin `+4`, end
/// `+8`), what `GameFeature_RenderAll` walks.
pub const FEATURE_LIST: usize = 0x736c84;

/// Calls `f` with each game feature, in the vector's order.
pub fn for_each_feature(mut f: impl FnMut(usize)) {
    let v = unsafe { rd::<usize>(FEATURE_LIST) };
    if v == 0 {
        return;
    }
    let (mut p, end) = (field(v, 4), field(v, 8));
    let mut guard = 0;
    while p != 0 && p < end && guard < 100_000 {
        f(field(p, 0));
        p += 4;
        guard += 1;
    }
}
