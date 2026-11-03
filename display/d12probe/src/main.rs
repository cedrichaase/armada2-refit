//! d12probe: can this 32-bit process draw with Direct3D 12 the way Display8 does? It
//! opens a window, makes a device at feature level 11_0 on an adapter it is told to use
//! (`--warp`: Windows' software one, which a CI runner has), a flip-model swap chain, one
//! 8 MB persistently mapped upload ring and two shader-visible heaps, and then builds
//! Display8's own root signatures and pipeline states -- the present pass's, and the
//! passthrough's (`vs_ff`, `vs_bump`, `ps_ff`) -- from the same source file
//! (`display/src/dx12/build.rs`, `#[path]`) and the committed DXIL, so that Microsoft's
//! runtime and, with `--debug`, its debug layer judge exactly what Display8 makes. It
//! draws ~300 frames (the passthrough's draw into an offscreen target, then the present
//! pass into the swap chain), reads the last back and checks seven pixels, and writes
//! everything into `d12probe.txt` beside the exe. Unattended: no message boxes, exit
//! code 0 when it works, 1 when not. README.md has the options.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The no-C-runtime support the plugins share (crates/a2rt: the allocator, mem*, 64-bit
// division, the stack probe) and its kernel32 declarations, shared rather than copied.
#[path = "../../../crates/a2rt/src/win.rs"]
#[allow(dead_code)]
mod win;
#[path = "../../../crates/a2rt/src/mem.rs"]
mod mem;

// Display8's side, as its crate has it, for `raw.rs` and `build.rs`, which name
// `crate::com::Guid`, `crate::vcall!`, `crate::passthrough::state` and `crate::shaders`.
#[macro_use]
#[path = "../../src/macros.rs"]
mod macros;
mod com {
    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Guid {
        pub d1: u32,
        pub d2: u16,
        pub d3: u16,
        pub d4: [u8; 8],
    }
}
#[path = "../../src/passthrough/state.rs"]
pub mod state_rs;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../src/shaders/ff_dxil.rs"]
pub mod ff_dxil_rs;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../src/shaders/present_dxil.rs"]
pub mod present_dxil_rs;
#[allow(dead_code)]
#[path = "../../src/dx12/raw.rs"]
pub mod raw_rs;
#[allow(dead_code)]
#[path = "../../src/dx12/build.rs"]
pub mod build_rs;
// ... under the names those files use for each other.
mod passthrough {
    pub use crate::state_rs as state;
}
mod shaders {
    pub use crate::ff_dxil_rs as ff_dxil;
    pub use crate::present_dxil_rs as present_dxil;
}
mod dx12 {
    pub use crate::build_rs as build;
    pub use crate::raw_rs as raw;
}

#[macro_use]
mod report;
mod probe;
mod user32;

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

/// A crash anywhere still leaves a report that says which step it was, and how.
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
