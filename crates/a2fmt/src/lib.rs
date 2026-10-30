//! Readers for Star Trek: Armada II's asset formats, for the renderer in `render3d/`.
//!
//! - [`sod`]: the binary model format (`SOD/*.sod`), every version the engine accepts.
//! - [`tga`]: Targa images as stock ships them, decoded to BGRA8 top-down, and the
//!   named mip chains beside them.
//! - [`spr`]: the sprite tables (`Sprites/*.spr`).
//! - [`odf`]: the object definition files (`odf/**/*.odf`).
//!
//! `no_std` with `alloc`: nothing here touches a file system. Callers hand in bytes, and,
//! where a format includes other files (`.spr`, ODF), a closure that fetches them by name.
//! No parser panics on bad input; every one returns an error or skips what it cannot read.
//!
//! The formats are described in `README.md` beside this crate.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod bytes;
pub mod math;
pub mod odf;
pub mod sod;
pub mod spr;
mod text;
pub mod tga;

pub use math::Matrix34;
