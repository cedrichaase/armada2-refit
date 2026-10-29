//! Drawing helpers every module shares: render3d's `gpu` context and frame (the
//! interface shared with UI.asi, gpu/README.md), a module's pipelines kept per
//! combination of states (`Pipes`), the view's pass begun again after a pass of a
//! module's own (`view_pass`), and the engine's textures wrapped (`engine_texture`).
//! Every module draws through a `gpu::Cmd` (`cmd`); none calls Direct3D 9 itself.
//!
//! d3d8to9 keeps its own record of what the game bound (platform/D3D9.md), so a
//! module that changes device state between the engine's steps must leave it as it
//! found it: a `Cmd` does that with one of `gpu`'s state blocks, put back when dropped.
use crate::frame::steps::View;
use crate::math::{M34, M4};
use alloc::vec::Vec;
use core::ffi::c_void;

fn log_line(a: core::fmt::Arguments) {
    crate::rt::log_fmt(a);
}

/// An export of Display8 (`d3d8.dll`), for `gpu` to ask which backend it draws on
/// (gpu/src/display.rs); 0 when there is none.
fn display_export(name: &str) -> usize {
    use crate::rt::win::{GetModuleHandleA, GetProcAddress};
    let m = unsafe { GetModuleHandleA(c"d3d8.dll".as_ptr() as *const u8) };
    if m.is_null() {
        return 0;
    }
    unsafe { GetProcAddress(m, crate::rt::cstr(name).as_ptr()) }
}

/// render3d's `gpu` context (each plugin keeps its own) and the frame it records into,
/// moved on after each main view. Its backend is Direct3D 9, or Vulkan when Display8
/// draws on it (gpu/README.md). On Vulkan it publishes its pass now to Display8, whose
/// passthrough draws what render3d hands back to the engine into the scene pass
/// (display/README.md, "Passthrough").
static mut GPU: gpu::Gpu = gpu::Gpu::with_display_publishing(log_line, display_export);
static mut FRAME: gpu::Frame = gpu::Frame::new();

#[allow(static_mut_refs)]
pub fn gpu() -> &'static gpu::Gpu {
    unsafe { &GPU }
}

#[allow(static_mut_refs)]
pub fn frame() -> &'static mut gpu::Frame {
    unsafe { &mut FRAME }
}

/// Before a `Reset`: `gpu` lets go of its state blocks and what it holds in the
/// default pool.
pub fn reset() {
    gpu().before_reset();
}

/// When the engine lets go of its device: `gpu` lets go of everything it holds (every
/// object holds its device, and native Direct3D 9 cannot make another while it lives).
pub fn forget() {
    gpu().release_device();
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

/// An engine texture (`ST3D_Texture`) to sample: its per-device texture (`+0x40 +
/// device index * 4`, the index at `Storm3D+0xc0`), whose `+4` is the
/// `IDirect3DTexture8`, which `gpu` makes a texture of (`gpu::Gpu::engine_texture`:
/// d3d8to9's `IDirect3DTexture9` behind it, or Display8's Vulkan mirror of it).
pub fn engine_texture(st3d_tex: usize) -> Option<gpu::Texture> {
    if st3d_tex == 0 {
        return None;
    }
    let idx = crate::engine::field(crate::engine::storm3d(), 0xc0);
    if idx > 8 {
        return None;
    }
    let devtex = crate::engine::field(st3d_tex, 0x40 + idx * 4);
    let t8 = crate::engine::field(devtex, 4);
    gpu().engine_texture(t8 as *mut c_void)
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
