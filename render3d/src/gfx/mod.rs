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

/// Every render state Direct3D 9 defines.
const RENDER_STATES: &[u32] = &[
    7, 8, 9, 14, 15, 16, 19, 20, 22, 23, 24, 25, 26, 27, 28, 29, 34, 35, 36, 37, 38, 48, 52, 53, 54, 55, 56, 57,
    58, 59, 60, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 145, 146, 147, 148,
    151, 152, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 165, 166, 167, 168, 170, 171, 172, 173, 174, 175,
    176, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 198, 199, 200,
    201, 202, 203, 204, 205, 206, 207, 208, 209,
];
const TSS_TYPES: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 23, 24, 26, 27, 28, 32];
/// The pixel samplers and the vertex texture samplers (`D3DVERTEXTEXTURESAMPLER0..3`).
const SAMPLERS: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 257, 258, 259, 260];

/// A state block holding only what render3d's draws change: every render state,
/// texture-stage and sampler state, the textures, shaders, vertex declaration and FVF,
/// the streams, the indices, the shader constants, the viewport and the scissor
/// rectangle. Not the transforms, lights, materials or clip planes, which no draw of
/// ours sets. `SBT_ALL` captured and applied all of those around every draw: three
/// quarters of render3d's CPU time in a battle (README.md, "Timing"). The constants
/// stay in: leaving them out saved almost nothing and lost most of the starfield on the
/// bench (some draw relies on constants being put back; not traced). Recorded with
/// Begin/EndStateBlock, during which the calls only name the states and change nothing
/// on the device.
fn recorded_block(dev: Device) -> Option<Com> {
    let p = dev.p();
    if vcall!(p, 60, fn() -> HRESULT,) < 0 {
        return None;
    }
    for &rs in RENDER_STATES {
        dev.set_rs(rs, 0);
    }
    for stage in 0..8 {
        for &t in TSS_TYPES {
            dev.set_tss(stage, t, 0);
        }
    }
    for &s in SAMPLERS {
        for t in 1..=13 {
            dev.set_ss(s, t, 0);
        }
        dev.set_texture(s, core::ptr::null_mut());
    }
    dev.set_vs(core::ptr::null_mut());
    dev.set_ps(core::ptr::null_mut());
    dev.set_vertex_decl(core::ptr::null_mut());
    dev.set_fvf(2);
    for n in 0..4 {
        dev.set_stream(n, core::ptr::null_mut(), 0, 0);
        dev.set_stream_freq(n, 1);
    }
    dev.set_indices(core::ptr::null_mut());
    let zero = [0.0f32; 4 * 224];
    dev.vs_const(0, &zero);
    dev.vs_const(224, &zero[..4 * 32]);
    dev.ps_const(0, &zero);
    dev.set_viewport(&Viewport { x: 0, y: 0, w: 1, h: 1, min_z: 0.0, max_z: 1.0 });
    dev.set_scissor(&Rect { left: 0, top: 0, right: 1, bottom: 1 });
    let mut sb: *mut c_void = core::ptr::null_mut();
    let hr = vcall!(p, 61, fn(*mut *mut c_void) -> HRESULT, &mut sb);
    if hr < 0 || sb.is_null() { None } else { Some(Com(sb)) }
}

impl Guard {
    #[allow(static_mut_refs)]
    pub fn new(dev: Device) -> Guard {
        unsafe {
            if SB.as_ref().is_none_or(|(d, _)| *d != dev) {
                SB = match recorded_block(dev) {
                    Some(c) => {
                        crate::log!("gfx: draws save and restore a recorded state block");
                        Some((dev, c))
                    }
                    None => {
                        crate::log!("gfx: no recorded state block: the whole state is saved around each draw");
                        dev.create_state_block(SBT_ALL).map(|c| (dev, c))
                    }
                };
            }
            match SB.as_ref() {
                Some((_, sb)) => {
                    let _z = crate::timing::zone("gfx: state capture");
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
            let _z = crate::timing::zone("gfx: state restore");
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
