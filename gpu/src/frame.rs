//! The frame a `Cmd` records into: its number, the rings the frame's dynamic vertices
//! and indices are written to, and what is to be let go of once the GPU is done with
//! it. The caller owns it (a static of the plugin's today; the display's later, handed
//! out through the C interface), never this crate, and moves it on with `next` once
//! per presented frame.
//!
//! The backends wait on the frame's fence before reusing its upload-ring space (which
//! Display8 owns) or freeing what was retired in it.
use crate::{Buffer, Texture};
use alloc::vec::Vec;

#[allow(dead_code)]
pub(crate) enum Retired {
    Texture(Texture),
    Buffer(Buffer),
}

pub struct Frame {
    index: u64,
    pub(crate) retired: Vec<Retired>,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame::new()
    }
}

impl Frame {
    pub const fn new() -> Frame {
        Frame { index: 0, retired: Vec::new() }
    }

    /// The frame's number, from 0.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// The next frame: what was retired in this one is let go of, and its ring space
    /// may be written again.
    pub fn next(&mut self) {
        crate::api::frame_next(self);
        self.index = self.index.wrapping_add(1);
        self.retired.clear();
    }

    /// Keeps `t` until the frame is over (a draw recorded in it may still read it).
    pub fn retire_texture(&mut self, t: Texture) {
        self.retired.push(Retired::Texture(t));
    }

    pub fn retire_buffer(&mut self, b: Buffer) {
        self.retired.push(Retired::Buffer(b));
    }
}

/// Data written into one of the frame's rings: drawn from in the same frame only.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    /// The `VkBuffer` on Vulkan (the display's upload ring, or a static buffer's); on
    /// Direct3D 12 that buffer's GPU address (`first` counts from it).
    pub(crate) vk: u64,
    /// The first element (vertex or index) in the buffer.
    pub first: u32,
    pub count: u32,
    /// Bytes per element.
    pub stride: u32,
}
