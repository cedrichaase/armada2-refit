//! Host tests of Display8's pure parts, which build without the game's runtime:
//!
//!   cargo test --release --target x86_64-unknown-linux-gnu --manifest-path display/hosttest/Cargo.toml
//!
//! `passthrough::state` (phase 10, X1): the API-neutral half of the passthrough that
//! `vk/pass.rs` and `dx12/pass.rs` both read: the FVF layouts, the constant blocks, the
//! pipeline keys, the blend, vertex attribute and sampler translation.
extern crate alloc;

#[path = "../../src/passthrough/state.rs"]
pub mod state;

/// `stub::blocks` (the Roots readiness work): the level sizes of block-compressed textures.
#[path = "../../src/stub/blocks.rs"]
pub mod blocks;
