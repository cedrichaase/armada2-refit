//! The exports of `gpu/src/display.rs` (the contract): the device, the frame, the
//! upload ring, deferred frees, the engine's targets and mirrors, and the engine's
//! state the plugins read (its viewport and transforms), on whatever backend runs.
use super::get;
use crate::stub::obj::SObj;
use crate::stub::Backend;
use core::ffi::c_void;
use gpu::display::{VkAlloc8, VkDevice8, VkFrame8, VkImage8, BACKEND_D3D12, BACKEND_D3D9, BACKEND_NONE, BACKEND_VULKAN};

/// The backend the stub's device was made on, and none before it exists (the engine's
/// first `CreateDevice`, where `Backend=`'s order is tried): a plugin is never told a
/// backend that a fallback then leaves (README.md, "Choosing a backend"). `gpu` asks again
/// while the answer is none.
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_backend() -> u32 {
    match crate::stub::made() {
        Some(Backend::Vulkan) => BACKEND_VULKAN,
        Some(Backend::D3d12) => BACKEND_D3D12,
        Some(Backend::D3d9) => BACKEND_D3D9,
        None => BACKEND_NONE,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk() -> *const VkDevice8 {
    get().map_or(core::ptr::null(), |v| &v.exported as *const VkDevice8)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_frame() -> *const VkFrame8 {
    match get() {
        Some(v) => {
            super::pass::close(v);
            crate::stub::flush_mirrors();
            v.frame() as *const VkFrame8
        }
        None => core::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_alloc(bytes: u32, align: u32, out: *mut VkAlloc8) -> bool {
    let (Some(v), false) = (get(), out.is_null()) else { return false };
    match v.alloc(bytes, align) {
        Some(a) => {
            unsafe { gpu::display::write(out, &a) };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_defer(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    match get() {
        Some(v) => v.defer(f, arg),
        None => f(arg),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_flush() -> bool {
    match get() {
        Some(v) => v.flush(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_target(which: u32, out: *mut VkImage8) -> bool {
    let (Some(v), false) = (get(), out.is_null()) else { return false };
    unsafe { gpu::display::write(out, &v.target(which)) };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_mirror(tex8: *mut c_void, out: *mut VkImage8) -> bool {
    if out.is_null() || !crate::stub::obj::is_stub(tex8) {
        return false;
    }
    match super::mirror::image(tex8 as *mut SObj) {
        Some(i) => {
            unsafe { gpu::display::write(out, &i) };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_engine_viewport(out: *mut [u32; 6]) -> bool {
    let d = crate::api::display8_device();
    if out.is_null() || !crate::stub::obj::is_stub(d) {
        return false;
    }
    unsafe { *out = crate::stub::device::state(d as *mut SObj).viewport };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_engine_transform(state: u32, out: *mut [f32; 16]) -> bool {
    let d = crate::api::display8_device();
    if out.is_null() || !crate::stub::obj::is_stub(d) {
        return false;
    }
    let ident = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    unsafe { *out = crate::stub::device::state(d as *mut SObj).transforms.iter().find(|t| t.0 == state).map_or(ident, |t| t.1) };
    true
}

/// A plugin's `Cmd` begins or ends, or ends with its pass now published (gpu/src/display.rs;
/// the passthrough, `pass.rs`).
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_vk_pass(event: u32, pass: *const gpu::display::VkPass8) {
    super::pass::event(event, pass);
}
