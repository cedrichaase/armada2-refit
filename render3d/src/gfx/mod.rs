//! Drawing helpers every module shares: render3d's `gpu` context and frame (the
//! interface shared with UI.asi, gpu/README.md), and, for the modules not yet ported
//! to it, the transitional pieces they draw with on the raw Direct3D 9 device: `Guard`
//! around their draws, shaders and declarations made on first use, a dynamic vertex
//! buffer for things rebuilt every frame (sprites, particles, beams). A module ported
//! to `gpu` uses none of these (gpu/README.md, "Porting a module").
//!
//! d3d8to9 keeps its own record of what the game bound (platform/D3D9.md), so a
//! module that changes device state between the engine's steps must leave it as it
//! found it: `Guard` does that with one of `gpu`'s state blocks.
use gpu::d3d9::raw::*;
use crate::frame::steps::View;
use crate::math::{M34, M4};
use alloc::vec::Vec;
use core::ffi::c_void;

pub mod chain;

pub use gpu::d3d9::{slot_caps, slots};

fn log_line(a: core::fmt::Arguments) {
    crate::rt::log_fmt(a);
}

/// render3d's `gpu` context (each plugin keeps its own) and the frame it records into,
/// moved on after each main view.
static mut GPU: gpu::Gpu = gpu::Gpu::new(log_line);
static mut FRAME: gpu::Frame = gpu::Frame::new();

#[allow(static_mut_refs)]
pub fn gpu() -> &'static gpu::Gpu {
    unsafe { &GPU }
}

#[allow(static_mut_refs)]
pub fn frame() -> &'static mut gpu::Frame {
    unsafe { &mut FRAME }
}

/// **Transitional**: the device's state is captured now and put back when the guard
/// is dropped (`gpu::Gpu::section`), around a module's raw draws.
pub struct Guard {
    s: Option<gpu::Section<'static>>,
}

impl Guard {
    pub fn new(dev: Device) -> Guard {
        let g = gpu();
        if g.device() != Some(dev) {
            g.attach(dev);
        }
        let _z = crate::timing::zone("gfx: state capture");
        Guard { s: Some(g.section()) }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _z = crate::timing::zone("gfx: state restore");
        self.s = None;
    }
}

/// Before a `Reset`: `gpu` lets go of its state blocks and what it holds in the
/// default pool.
pub fn reset() {
    gpu().before_reset();
}

/// The objects `Shader` and `Decl` made. They hold them here rather than in
/// themselves, so that `forget` can let go of all of them at once, wherever they are.
static mut MADE: Vec<Com> = Vec::new();
/// Moves on at each `forget`: a `Shader` or `Decl` made before it makes its object again.
static mut EPOCH: u32 = 0;

#[allow(static_mut_refs)]
fn keep(c: Com) -> (u32, *mut c_void) {
    let p = c.ptr();
    unsafe {
        MADE.push(c);
        (EPOCH, p)
    }
}

fn current(obj: &Option<(Device, u32, *mut c_void)>, dev: Device) -> bool {
    obj.as_ref().is_some_and(|(d, e, _)| *d == dev && *e == unsafe { EPOCH })
}

/// Lets go of everything made here, when the engine lets go of its device (every
/// object holds its device, and native Direct3D 9 cannot make another while it lives),
/// and has `gpu` let go of all it holds.
#[allow(static_mut_refs)]
pub fn forget() {
    unsafe {
        MADE = Vec::new();
        EPOCH = EPOCH.wrapping_add(1);
    }
    gpu().release_device();
}

/// **Transitional**: a vertex or pixel shader from embedded bytecode, made on first
/// use on a device. A module on `gpu` has a `gpu::Pipeline` instead.
pub struct Shader {
    code: &'static [u32],
    pixel: bool,
    obj: Option<(Device, u32, *mut c_void)>,
    told: bool,
}

impl Shader {
    pub const fn vertex(code: &'static [u32]) -> Shader {
        Shader { code, pixel: false, obj: None, told: false }
    }
    pub const fn pixel(code: &'static [u32]) -> Shader {
        Shader { code, pixel: true, obj: None, told: false }
    }
    /// The shader on `dev`, or null if it cannot be made.
    pub fn get(&mut self, dev: Device) -> *mut c_void {
        if !current(&self.obj, dev) {
            let c = if self.pixel { dev.create_ps(self.code) } else { dev.create_vs(self.code) };
            if c.is_none() && !self.told {
                self.told = true;
                let (vs, ps) = slot_caps(dev);
                crate::log!(
                    "gfx: Direct3D refused a {} shader of {} instruction slots (the card takes {}); what draws with it stands down",
                    if self.pixel { "pixel" } else { "vertex" }, slots(self.code), if self.pixel { ps } else { vs }
                );
            }
            self.obj = c.map(|c| {
                let (e, p) = keep(c);
                (dev, e, p)
            });
        }
        self.obj.as_ref().map_or(core::ptr::null_mut(), |(_, _, p)| *p)
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

/// **Transitional**: `scene::VERTEX` as a Direct3D 9 declaration, for the modules not
/// yet ported.
pub static SOD_DECL: [VertexElement; 4] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT3, DECLUSAGE_NORMAL, 0),
    ve(0, 24, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    VE_END,
];

/// **Transitional**: a vertex declaration made on first use on a device (a
/// `gpu::PipelineDesc`'s layout, on `gpu`).
pub struct Decl {
    els: &'static [VertexElement],
    obj: Option<(Device, u32, *mut c_void)>,
}

impl Decl {
    pub const fn new(els: &'static [VertexElement]) -> Decl {
        Decl { els, obj: None }
    }
    pub fn bind(&mut self, dev: Device) -> bool {
        if !current(&self.obj, dev) {
            self.obj = dev.create_vertex_decl(self.els).map(|c| {
                let (e, p) = keep(c);
                (dev, e, p)
            });
        }
        match &self.obj {
            Some((_, _, p)) => {
                dev.set_vertex_decl(*p);
                true
            }
            None => false,
        }
    }
}

/// **Transitional**: a dynamic vertex buffer filled with `DISCARD` / `NOOVERWRITE` as
/// a ring. Dropped before a `Reset` (it is in the default pool). On `gpu` the frame's
/// ring takes its place (`gpu::Cmd::vertices`).
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

/// render3d's command recorder for a module's draws: `gpu().begin(frame())`. None
/// without a device.
pub fn cmd() -> Option<gpu::Cmd<'static>> {
    let _z = crate::timing::zone("gfx: state capture");
    gpu().begin(frame())
}

/// A module's pipelines, one per combination of the states it draws with, kept by a
/// key of the module's own (a few bits per state). Those that draw into the view take
/// the scene pass's formats and samples, and are made again when they change.
pub struct Pipes {
    fmt: Option<(gpu::Format, u32, gpu::Format)>,
    v: Vec<(u64, gpu::Pipeline)>,
}

impl Pipes {
    pub const fn new() -> Pipes {
        Pipes { fmt: None, v: Vec::new() }
    }

    /// The pipeline `key` that draws into the view: `desc` (its target formats and
    /// samples are the scene pass's) made on first use. None without the scene's targets.
    pub fn view(&mut self, key: u64, desc: impl FnOnce() -> gpu::PipelineDesc) -> Option<gpu::Pipeline> {
        let f = crate::frame::scene_formats()?;
        if self.fmt != Some(f) {
            self.fmt = Some(f);
            self.v.clear();
        }
        if let Some((_, p)) = self.v.iter().find(|(k, _)| *k == key) {
            return Some(p.clone());
        }
        let mut d = desc();
        d.colors = [f.0, gpu::Format::Unknown, gpu::Format::Unknown, gpu::Format::Unknown];
        d.samples = f.1;
        d.depth_format = Some(f.2);
        let p = gpu().pipeline(d);
        self.v.push((key, p.clone()));
        Some(p)
    }

    /// The pipeline `key` that draws into a target of the module's own, as `desc` says.
    pub fn own(&mut self, key: u64, desc: impl FnOnce() -> gpu::PipelineDesc) -> gpu::Pipeline {
        let key = key | 1 << 63;
        if let Some((_, p)) = self.v.iter().find(|(k, _)| *k == key) {
            return p.clone();
        }
        let p = gpu().pipeline(desc());
        self.v.push((key, p.clone()));
        p
    }
}

/// Begins the view's pass again (the scene target and depth, over the view), after a
/// module drew into a target of its own, with the viewport `vp` the engine had set
/// (`gpu::Gpu::engine_viewport`, read before the module's first pass).
pub fn view_pass(cmd: &mut gpu::Cmd, v: &View, vp: Option<gpu::Viewport>) -> bool {
    if !crate::frame::scene_pass(cmd, v.rect) {
        return false;
    }
    if let Some(vp) = vp {
        cmd.set_viewport(&vp);
    }
    true
}

/// A static vertex or index buffer filled with `data`.
pub fn buffer<T: Copy>(data: &[T], kind: gpu::BufferKind) -> gpu::Buffer {
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, core::mem::size_of_val(data)) };
    gpu().buffer(gpu::BufferDesc { size: bytes.len() as u32, kind, dynamic: false }, bytes)
}

/// `buffer`, keeping no copy of `data` (`gpu::Gpu::buffer_transient`): gone with the
/// device, and made again by its owner (the scene's models, after a `Reset`).
pub fn buffer_once<T: Copy>(data: &[T], kind: gpu::BufferKind) -> gpu::Buffer {
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, core::mem::size_of_val(data)) };
    gpu().buffer_transient(gpu::BufferDesc { size: bytes.len() as u32, kind, dynamic: false }, bytes)
}

/// A 2D BGRA8 texture of `w` x `h` from its levels (tightly packed, the first first),
/// sampled only.
pub fn texture_bgra8(w: u32, h: u32, levels: &[&[u8]]) -> gpu::Texture {
    let mut px = Vec::with_capacity(levels.iter().map(|l| l.len()).sum());
    for l in levels {
        px.extend_from_slice(l);
    }
    let d = gpu::TextureDesc::d2(w, h, gpu::Format::Bgra8, gpu::Usage::SAMPLED).with_levels(levels.len() as u32);
    gpu().texture(d, gpu::Contents::Owned(px))
}

/// A sampler: `filter` for magnification, minification and between levels (None:
/// one level), `u` and `v` (and `w`) addressing.
pub const fn sampler(filter: gpu::Filter, mip: gpu::MipFilter, u: gpu::Address, v: gpu::Address) -> gpu::Sampler {
    gpu::Sampler { min: filter, mag: filter, mip, address_u: u, address_v: v, address_w: gpu::Address::Clamp, anisotropy: 1, srgb: false }
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

/// Several things a module keeps per view (the main view, the inset, the UI's close-up), by
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
