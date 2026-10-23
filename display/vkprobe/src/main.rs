//! vkprobe: can this 32-bit process draw with Vulkan the way Display8 will? It opens a
//! window, brings up Vulkan on it as `display/src/vk` does, maps a host-visible ring
//! into its own address space, draws ~300 frames out of that ring (vertices, indices,
//! uniforms at dynamic offsets, a texture upload each frame), reads the last frame back
//! and checks its pixels, and writes everything into `vkprobe.txt` beside the exe. The
//! question it exists for is macOS: Wine there is new-style WoW64 only, so the 32-bit
//! game's Vulkan memory has to be mapped below 4 GB by winevulkan on top of MoltenVK
//! (README.md). Unattended: no message boxes, exit code 0 when it works, 1 when not.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The no-C-runtime support Display8 has already written (the allocator, mem*, 64-bit
// division, the stack probe) and its kernel32 declarations, shared rather than copied.
#[path = "../../src/rt/win.rs"]
#[allow(dead_code)]
mod win;
#[path = "../../src/rt/mem.rs"]
mod mem;

#[macro_use]
mod report;
mod probe;
mod user32;
#[rustfmt::skip]
#[allow(dead_code)]
mod shaders {
    pub mod probe;
    pub mod probe_spv;
}

use core::ffi::c_void;
use win::*;

#[global_allocator]
static HEAP: mem::Heap = mem::Heap;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(f: unsafe extern "system" fn(*const ExceptionPointers) -> i32) -> usize;
}

#[repr(C)]
struct ExceptionPointers {
    record: *const ExceptionRecord,
    context: *const c_void,
}

#[repr(C)]
struct ExceptionRecord {
    code: u32,
    flags: u32,
    next: *const ExceptionRecord,
    address: usize,
    nparams: u32,
    info: [usize; 15],
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    report::fail_now(&report::fmt(format_args!("panic: {}", info)));
    unsafe { ExitProcess(1) }
}

/// A crash anywhere (a write into a mapping that is not really there is the one this
/// probe expects) still leaves a report that says which step it was, and how.
unsafe extern "system" fn crashed(p: *const ExceptionPointers) -> i32 {
    let (code, at, info) = unsafe {
        let r = &*(*p).record;
        (r.code, r.address, if r.nparams >= 2 { r.info[1] } else { 0 })
    };
    let what = if code == 0xC000_0005 { "access violation" } else { "exception" };
    report::fail_now(&report::fmt(format_args!("{} {:#010x} at {:#010x} (address {:#010x})", what, code, at, info)));
    unsafe { ExitProcess(1) }
}

#[unsafe(no_mangle)]
pub extern "system" fn start() -> ! {
    report::open();
    unsafe { SetUnhandledExceptionFilter(crashed) };
    report::system();
    let ok = probe::run();
    report::finish(ok);
    unsafe { ExitProcess(if ok { 0 } else { 1 }) }
}
