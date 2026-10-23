//! The Direct3D 12 exports of `gpu/src/display.rs` (the contract), shaped as the Vulkan
//! ones (`vk/exports.rs`): the device, the frame, the upload ring, shader-visible
//! descriptors, deferred frees, the engine's targets and mirrors. `display8_backend`
//! and the engine's state (`display8_engine_*`) are shared, in `vk/exports.rs`.
use super::get;
use crate::stub::obj::SObj;
use core::ffi::c_void;
use gpu::display::{Dx12Alloc8, Dx12Descriptors8, Dx12Device8, Dx12Frame8, Dx12Image8};

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12() -> *const Dx12Device8 {
    get().map_or(core::ptr::null(), |d| &d.exported as *const Dx12Device8)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_frame() -> *const Dx12Frame8 {
    match get() {
        Some(d) => {
            crate::stub::flush_mirrors();
            d.frame() as *const Dx12Frame8
        }
        None => core::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_alloc(bytes: u32, align: u32, out: *mut Dx12Alloc8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    match d.alloc(bytes, align) {
        Some(a) => {
            unsafe { *out = a };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_descriptors(heap: u32, count: u32, out: *mut Dx12Descriptors8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    match d.descriptors(heap, count) {
        Some(r) => {
            unsafe { *out = r };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_defer(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    match get() {
        Some(d) => d.defer(f, arg),
        None => f(arg),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_target(which: u32, out: *mut Dx12Image8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    unsafe { *out = d.target(which) };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_flush() -> bool {
    get().is_some_and(|d| d.flush())
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_dx12_mirror(tex8: *mut c_void, out: *mut Dx12Image8) -> bool {
    if out.is_null() || !crate::stub::obj::is_stub(tex8) {
        return false;
    }
    match super::mirror::image(tex8 as *mut SObj) {
        Some(i) => {
            unsafe { *out = i };
            true
        }
        None => false,
    }
}
