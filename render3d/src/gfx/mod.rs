//! Drawing helpers every module shares: putting the engine's device state back after
//! our draws, shaders made on first use, a dynamic vertex buffer for things rebuilt
//! every frame (sprites, particles, beams), and the camera's matrices.
//!
//! d3d8to9 keeps its own record of what the game bound (platform/D3D9.md), so a
//! module that changes device state between the engine's steps must leave it as it
//! found it: `Guard` does that with one state block.
use crate::d3d::*;
use crate::frame::steps::View;
use crate::math::{M34, M4};
use alloc::vec::Vec;
use core::ffi::c_void;

/// Captures the device's whole state; `drop` puts it back. One state block per
/// device, reused (made on first use).
pub struct Guard {
    sb: *mut c_void,
}

static mut SB: Option<(Device, Com)> = None;

impl Guard {
    #[allow(static_mut_refs)]
    pub fn new(dev: Device) -> Guard {
        unsafe {
            if SB.as_ref().is_none_or(|(d, _)| *d != dev) {
                SB = dev.create_state_block(SBT_ALL).map(|c| (dev, c));
            }
            match SB.as_ref() {
                Some((_, sb)) => {
                    sb.sb_capture();
                    Guard { sb: sb.ptr() }
                }
                None => Guard { sb: core::ptr::null_mut() },
            }
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !self.sb.is_null() {
            vcall!(self.sb, 5, fn() -> HRESULT,);
        }
    }
}

/// Drops the shared state block (before a `Reset`).
pub fn reset() {
    unsafe { SB = None };
}

/// A vertex or pixel shader from embedded bytecode, made on first use on a device.
pub struct Shader {
    code: &'static [u32],
    pixel: bool,
    obj: Option<(Device, Com)>,
}

impl Shader {
    pub const fn vertex(code: &'static [u32]) -> Shader {
        Shader { code, pixel: false, obj: None }
    }
    pub const fn pixel(code: &'static [u32]) -> Shader {
        Shader { code, pixel: true, obj: None }
    }
    /// The shader on `dev`, or null if it cannot be made.
    pub fn get(&mut self, dev: Device) -> *mut c_void {
        if self.obj.as_ref().is_none_or(|(d, _)| *d != dev) {
            let c = if self.pixel { dev.create_ps(self.code) } else { dev.create_vs(self.code) };
            self.obj = c.map(|c| (dev, c));
        }
        self.obj.as_ref().map_or(core::ptr::null_mut(), |(_, c)| c.ptr())
    }
    pub fn bind(&mut self, dev: Device) -> bool {
        let p = self.get(dev);
        if p.is_null() {
            return false;
        }
        if self.pixel { dev.set_ps(p) } else { dev.set_vs(p) };
        true
    }
}

/// A vertex declaration made on first use on a device.
pub struct Decl {
    els: &'static [VertexElement],
    obj: Option<(Device, Com)>,
}

impl Decl {
    pub const fn new(els: &'static [VertexElement]) -> Decl {
        Decl { els, obj: None }
    }
    pub fn bind(&mut self, dev: Device) -> bool {
        if self.obj.as_ref().is_none_or(|(d, _)| *d != dev) {
            self.obj = dev.create_vertex_decl(self.els).map(|c| (dev, c));
        }
        match &self.obj {
            Some((_, c)) => {
                dev.set_vertex_decl(c.ptr());
                true
            }
            None => false,
        }
    }
}

/// A dynamic vertex buffer filled with `DISCARD` / `NOOVERWRITE` as a ring. Dropped
/// before a `Reset` (it is in the default pool).
pub struct DynVb {
    bytes: u32,
    pos: u32,
    vb: Option<(Device, Com)>,
}

impl DynVb {
    pub const fn new(bytes: u32) -> DynVb {
        DynVb { bytes, pos: 0, vb: None }
    }
    pub fn reset(&mut self) {
        self.vb = None;
    }
    /// Copies `data` into the buffer and binds it as stream 0 with `stride`; returns
    /// the first vertex's index, or None if it does not fit or cannot be made.
    pub fn push<T: Copy>(&mut self, dev: Device, data: &[T], stride: u32) -> Option<u32> {
        let n = core::mem::size_of_val(data) as u32;
        if n == 0 || n > self.bytes {
            return None;
        }
        if self.vb.as_ref().is_none_or(|(d, _)| *d != dev) {
            self.vb = dev.create_vertex_buffer(self.bytes, USAGE_DYNAMIC | USAGE_WRITEONLY, 0, POOL_DEFAULT).map(|c| (dev, c));
            self.pos = self.bytes;
        }
        let (_, vb) = self.vb.as_ref()?;
        // Start each push on a whole vertex of this stride.
        let mut at = self.pos.div_ceil(stride) * stride;
        let flags = if at + n > self.bytes {
            at = 0;
            LOCK_DISCARD
        } else {
            LOCK_NOOVERWRITE
        };
        let p = vb.buf_lock(at, n, flags)?;
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, p, n as usize) };
        vb.buf_unlock();
        self.pos = at + n;
        dev.set_stream(0, vb.ptr(), 0, stride);
        Some(at / stride)
    }
}

/// The engine's 3x4 (rows: x, y, z axes, then position) as a Direct3D world matrix.
pub fn world(m: &M34) -> M4 {
    m.to_m4()
}

/// World x view x projection, transposed for `SetVertexShaderConstantF` (HLSL reads
/// constant registers as rows of a column-major matrix by default).
pub fn wvp_constants(w: &M4, v: &View) -> [f32; 16] {
    let m = w.mul(&v.view).mul(&v.proj).transpose();
    *m.as_floats()
}

/// View x projection, transposed for shader constants.
pub fn vp_constants(v: &View) -> [f32; 16] {
    *v.view.mul(&v.proj).transpose().as_floats()
}

/// Several things a module keeps per view (the main view and the inset), by
/// `View::key`.
pub struct PerView<T> {
    pub v: Vec<T>,
}

impl<T: Default> PerView<T> {
    pub const fn new() -> PerView<T> {
        PerView { v: Vec::new() }
    }
    pub fn get(&mut self, key: usize) -> &mut T {
        while self.v.len() <= key {
            self.v.push(T::default());
        }
        &mut self.v[key]
    }
}
