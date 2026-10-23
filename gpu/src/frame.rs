//! The frame a `Cmd` records into: its number, the rings the frame's dynamic vertices
//! and indices are written to, and what is to be let go of once the GPU is done with
//! it. The caller owns it (a static of the plugin's today; the display's later, handed
//! out through the C interface), never this crate, and moves it on with `next` once
//! per presented frame.
//!
//! On Direct3D 9 the frame is trivial: the driver renames a buffer locked with
//! `DISCARD`, and a resource can be released at once. A later backend waits on the
//! frame's fence before reusing its ring space or freeing what was retired in it.
use crate::{Buffer, Texture};
use crate::d3d9::Buffer as D9Buffer;
use alloc::vec::Vec;
use core::ffi::c_void;

/// A ring of dynamic data in one buffer, grown when a frame needs more of it.
pub(crate) struct Ring {
    pub(crate) buf: Option<D9Buffer>,
    pub(crate) bytes: u32,
    pub(crate) pos: u32,
    /// Bytes written into the ring in this frame (none of them may be overwritten
    /// before the frame is over: a span made earlier may not have been drawn yet).
    pub(crate) used: u32,
}

impl Ring {
    const fn new() -> Ring {
        Ring { buf: None, bytes: 0, pos: 0, used: 0 }
    }
}

#[allow(dead_code)]
pub(crate) enum Retired {
    Texture(Texture),
    Buffer(Buffer),
    /// A ring of the Direct3D 9 backend's own, outgrown in this frame.
    D9Buffer(D9Buffer),
}

pub struct Frame {
    index: u64,
    pub(crate) vertices: Ring,
    pub(crate) indices: Ring,
    pub(crate) retired: Vec<Retired>,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame::new()
    }
}

impl Frame {
    pub const fn new() -> Frame {
        Frame { index: 0, vertices: Ring::new(), indices: Ring::new(), retired: Vec::new() }
    }

    /// The frame's number, from 0.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// The next frame: what was retired in this one is let go of, and its ring space
    /// may be written again.
    pub fn next(&mut self) {
        self.index = self.index.wrapping_add(1);
        self.retired.clear();
        self.vertices.used = 0;
        self.indices.used = 0;
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
    /// The ring's buffer when the span was written (kept alive by the frame if the
    /// ring has grown since): Direct3D 9's.
    pub(crate) buf: *mut c_void,
    /// The `VkBuffer` on Vulkan (the display's upload ring, or a static buffer's).
    pub(crate) vk: u64,
    /// The first element (vertex or index) in the buffer.
    pub first: u32,
    pub count: u32,
    /// Bytes per element.
    pub stride: u32,
}
