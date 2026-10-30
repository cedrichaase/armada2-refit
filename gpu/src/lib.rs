//! The graphics interface `Render3D.asi` and `UI.asi` draw through (README.md beside
//! this crate). It is shaped for Vulkan and Direct3D 12 and implemented on Direct3D 9
//! (`d3d9`: the device behind the game's d3d8 one) and on Vulkan (`vulkan`: Display8's
//! device in its stub mode), the backend chosen at run time by what Display8 says
//! (`api`):
//!
//! - **Resources** (`Texture`, `Buffer`, `Pipeline`) are made once from a description
//!   and stay valid until dropped. A pipeline is immutable: shaders, vertex layout,
//!   blend, depth, rasteriser state, topology, target formats and sample count.
//! - **Commands** are recorded into a `Frame` the caller owns, through a `Cmd`: render
//!   passes begun with load operations, pipelines bound, binding groups (textures with
//!   their samplers, uniform blocks), draws, resolves, copies.
//! - **Nothing per frame is kept in a static** here. Each plugin links this crate on its
//!   own (two copies in one process), and later the display will own the frame and hand
//!   it out through plain handles; so the frame is a value the caller passes in.
//!
//! The device going away (a `Reset`, or the engine releasing it) is the backend's
//! business: it lets go of what must go and makes it again from its description at the
//! next use. Nothing in the interface hears of it.
#![no_std]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

/// A call through slot `$slot` of a COM object's vtable, `__stdcall`, with the object
/// as the first argument: `vcall!(obj, 12, fn(u32) -> HRESULT, x)`.
#[macro_export]
macro_rules! vcall {
    ($obj:expr, $slot:expr, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {{
        let o: *mut ::core::ffi::c_void = $obj;
        let f: unsafe extern "system" fn(*mut ::core::ffi::c_void $(, $t)*) -> $r =
            unsafe { ::core::mem::transmute(*(*(o as *const *const usize)).add($slot)) };
        unsafe { f(o $(, $a)*) }
    }};
}

/// This crate's version, for the logs beside the contract's (`display::ABI_MAJOR`,
/// `ABI_MINOR`): the plugins' changelogs say when it moves (README.md, "Versions").
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod api;
pub mod chain;
pub mod d3d9;
pub mod display;
pub mod dx12;
mod frame;
pub mod pixels;
pub mod record;
pub mod replay;
pub mod types;
pub mod vulkan;

pub use api::{Attached, Backend, Buffer, Cmd, ComputePipeline, Gpu, Pipeline, Storage, Texture, Timer};
pub use frame::{Frame, Span};
pub use pixels::Mapped;
pub use types::*;
