//! The exports of `gpu/src/display.rs` (the contract): the device, the frame, the
//! upload ring, deferred frees, the engine's targets and mirrors, and the engine's
//! state the plugins read (its viewport and transforms), on whatever backend runs.
use super::get;
use crate::stub::obj::SObj;
use core::ffi::c_void;
use gpu::display::{VkAlloc8, VkDevice8, VkFrame8, VkImage8, BACKEND_D3D12, BACKEND_D3D9, BACKEND_NONE, BACKEND_VULKAN};

/// Vulkan from the moment the stub starts on it (the engine's `Direct3DCreate8`), before
/// the device: a plugin that asks early makes its objects for the right backend. A stub
/// on Vulkan has no Direct3D 9 to fall back to.
#[unsafe(no_mangle)]
pub extern "C" fn display8_backend() -> u32 {
    if get().is_some() || crate::stub::vulkan() {
        BACKEND_VULKAN
    } else if crate::dx12::get().is_some() {
        BACKEND_D3D12
    } else if !crate::stub::dev9().is_null() {
        BACKEND_D3D9
    } else {
        BACKEND_NONE
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk() -> *const VkDevice8 {
    get().map_or(core::ptr::null(), |v| &v.exported as *const VkDevice8)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_frame() -> *const VkFrame8 {
    match get() {
        Some(v) => {
            crate::stub::flush_mirrors();
            v.frame() as *const VkFrame8
        }
        None => core::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_alloc(bytes: u32, align: u32, out: *mut VkAlloc8) -> bool {
    let (Some(v), false) = (get(), out.is_null()) else { return false };
    match v.alloc(bytes, align) {
        Some(a) => {
            unsafe { *out = a };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_defer(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    match get() {
        Some(v) => v.defer(f, arg),
        None => f(arg),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_flush() -> bool {
    match get() {
        Some(v) => v.flush(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_target(which: u32, out: *mut VkImage8) -> bool {
    let (Some(v), false) = (get(), out.is_null()) else { return false };
    unsafe { *out = v.target(which) };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_vk_mirror(tex8: *mut c_void, out: *mut VkImage8) -> bool {
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

#[unsafe(no_mangle)]
pub extern "C" fn display8_engine_viewport(out: *mut [u32; 6]) -> bool {
    let d = crate::api::display8_device();
    if out.is_null() || !crate::stub::obj::is_stub(d) {
        return false;
    }
    unsafe { *out = crate::stub::device::state(d as *mut SObj).viewport };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8_engine_transform(state: u32, out: *mut [f32; 16]) -> bool {
    let d = crate::api::display8_device();
    if out.is_null() || !crate::stub::obj::is_stub(d) {
        return false;
    }
    let ident = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    unsafe { *out = crate::stub::device::state(d as *mut SObj).transforms.iter().find(|t| t.0 == state).map_or(ident, |t| t.1) };
    true
}
