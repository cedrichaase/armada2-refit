//! The Direct3D 12 exports of `gpu/src/display.rs` (the contract), shaped as the Vulkan
//! ones (`vk/exports.rs`): the device, the frame, the upload ring, shader-visible
//! descriptors, deferred frees, the engine's targets and mirrors, and the pass a plugin
//! publishes (contract 1.4, `display8v1_dx12_pass`). `display8v1_backend` and the
//! engine's state (`display8_engine_*`) are shared, in `vk/exports.rs`.
use super::get;
use crate::log;
use crate::stub::obj::SObj;
use core::ffi::c_void;
use gpu::display::{Dx12Alloc8, Dx12Descriptors8, Dx12Device8, Dx12Frame8, Dx12Image8, Dx12Pass8, PASS_ENTER, PASS_PUBLISH};

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12() -> *const Dx12Device8 {
    get().map_or(core::ptr::null(), |d| &d.exported as *const Dx12Device8)
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_frame() -> *const Dx12Frame8 {
    match get() {
        Some(d) => {
            crate::stub::flush_mirrors();
            d.frame().map_or(core::ptr::null(), |f| f as *const Dx12Frame8)
        }
        None => core::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_alloc(bytes: u32, align: u32, out: *mut Dx12Alloc8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    match d.alloc(bytes, align) {
        Some(a) => {
            unsafe { gpu::display::write(out, &a) };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_descriptors(heap: u32, count: u32, out: *mut Dx12Descriptors8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    match d.descriptors(heap, count) {
        Some(r) => {
            unsafe { gpu::display::write(out, &r) };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_defer(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    match get() {
        Some(d) => d.defer(f, arg),
        None => f(arg),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_target(which: u32, out: *mut Dx12Image8) -> bool {
    let (Some(d), false) = (get(), out.is_null()) else { return false };
    unsafe { gpu::display::write(out, &d.target(which)) };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_flush() -> bool {
    get().is_some_and(|d| d.flush())
}

#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_mirror(tex8: *mut c_void, out: *mut Dx12Image8) -> bool {
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

/// What the plugins said of their `Cmd`s (`display8v1_dx12_pass`): the pass published
/// last and the frame (Display8's index) it is for, how many `Cmd`s are alive, and the
/// kinds of pass logged so far (width, height, samples, depth), each once.
struct Passes {
    published: Option<(Dx12Pass8, u64)>,
    alive: u32,
    logged: [(u32, u32, u32, bool); 8],
    nlogged: usize,
}

static mut PASSES: Passes = Passes { published: None, alive: 0, logged: [(0, 0, 0, false); 8], nlogged: 0 };

#[allow(static_mut_refs)]
fn passes() -> &'static mut Passes {
    unsafe { &mut PASSES }
}

/// A plugin's outermost `Cmd` begins or ends, or ends with its pass now published
/// (gpu/src/display.rs, "Pass publishing on Direct3D 12"). Kept for the passthrough,
/// which draws the engine's draws into the pass published in this frame while no `Cmd`
/// is alive; nothing is recorded here.
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_dx12_pass(event: u32, pass: *const Dx12Pass8) {
    let Some(d) = get() else { return };
    // The plugin records on the frame's list around this: what the passthrough set on
    // it is not there to be assumed (`pass::forget`).
    super::pass::forget();
    let p = passes();
    match event {
        PASS_ENTER => {
            p.alive += 1;
            crate::host::drew();
        }
        _ => {
            p.alive = p.alive.saturating_sub(1);
            if event == PASS_PUBLISH && !pass.is_null() {
                // Only what the plugin's size says (gpu/src/display.rs, "Versions").
                let v = unsafe { gpu::display::read(pass) };
                if v.colour.resource == 0 || v.rtv == 0 {
                    return;
                }
                let k = (v.width, v.height, v.samples, v.depth.resource != 0);
                if p.nlogged < p.logged.len() && !p.logged[..p.nlogged].contains(&k) {
                    p.logged[p.nlogged] = k;
                    p.nlogged += 1;
                    log!("dx12: pass published {}x{} x{}{} (frame {})", v.width, v.height, v.samples, if v.depth.resource != 0 { " with depth" } else { "" }, d.index);
                }
                p.published = Some((v, d.index));
            }
        }
    }
}

/// The pass a plugin published in the frame now, if any: forgotten at the engine's
/// `Present` (the frame's index moves on there).
pub fn published() -> Option<Dx12Pass8> {
    let d = get()?;
    match passes().published {
        Some((v, f)) if f == d.index => Some(v),
        _ => None,
    }
}

/// Whether a plugin's `Cmd` is alive now (Display8 records nothing into the list then).
pub fn cmd_alive() -> bool {
    passes().alive != 0
}
