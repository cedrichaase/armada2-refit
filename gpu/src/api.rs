//! The interface's objects, on the backend the plugins draw on: Direct3D 9 (`d3d9`,
//! the device behind the game's d3d8 one, or Display8's own in stub mode) or Vulkan
//! (`vulkan`, Display8's device in stub mode with `Backend=vulkan`). Which one is chosen
//! at run time, once, by what Display8 says (`display8_backend`): with no Display8, an
//! older one, or one that says Direct3D 9, it is Direct3D 9 as before. Each object is
//! an enum of the two backends' own, and every call goes to the one it was made on.
use crate::d3d9::raw::Device as Dev;
use crate::display::{Display8, BACKEND_D3D9, BACKEND_VULKAN};
use crate::frame::{Frame, Span};
use crate::pixels::Mapped;
use crate::types::*;
use crate::{d3d9, vulkan};
use core::cell::Cell;
use core::ffi::c_void;

pub type Log = fn(core::fmt::Arguments);

const UNDECIDED: u8 = 0;
const D3D9: u8 = 1;
const VULKAN: u8 = 2;

/// Which backend the objects are made on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    D3d9,
    Vulkan,
}

/// The engine's device as `Gpu::attach_engine` found it: an identity to tell a new one
/// by, and whether it is new since the last call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Attached {
    pub id: usize,
    pub new: bool,
}

pub struct Gpu {
    log: Log,
    d9: d3d9::Gpu,
    vk: vulkan::Gpu,
    lookup: Option<fn(&str) -> usize>,
    mode: Cell<u8>,
    /// The Vulkan device last handed to `attach_engine` (it never changes).
    vk_seen: Cell<usize>,
    /// An object was made before Display8 had said (logged once).
    early: Cell<bool>,
    /// Display8's exports, looked up once.
    disp: Cell<Option<Display8>>,
}

impl Gpu {
    /// Direct3D 9 only (no Display8 is asked).
    pub const fn new(log: Log) -> Gpu {
        Gpu { log, d9: d3d9::Gpu::new(log), vk: vulkan::Gpu::new(log), lookup: None, mode: Cell::new(D3D9), vk_seen: Cell::new(0), early: Cell::new(false), disp: Cell::new(None) }
    }

    /// On the backend Display8 says, through `lookup` (the plugin's `GetProcAddress` on
    /// `d3d8.dll`; 0 for a name it lacks).
    pub const fn with_display(log: Log, lookup: fn(&str) -> usize) -> Gpu {
        Gpu { log, d9: d3d9::Gpu::new(log), vk: vulkan::Gpu::new(log), lookup: Some(lookup), mode: Cell::new(UNDECIDED), vk_seen: Cell::new(0), early: Cell::new(false), disp: Cell::new(None) }
    }

    fn mode(&self) -> u8 {
        let m = self.mode.get();
        if m != UNDECIDED {
            return m;
        }
        let Some(lookup) = self.lookup else {
            self.mode.set(D3D9);
            return D3D9;
        };
        let d = match self.disp.get() {
            Some(d) => d,
            None => match Display8::load(lookup) {
                Some(d) => {
                    self.disp.set(Some(d));
                    d
                }
                None => {
                    // Not Display8, or one without the Vulkan exports: Direct3D 9 for good.
                    self.mode.set(D3D9);
                    return D3D9;
                }
            },
        };
        match (d.backend)() {
            BACKEND_VULKAN => {
                if self.vk.init(d) {
                    self.mode.set(VULKAN);
                    VULKAN
                } else {
                    (self.log)(format_args!("gpu: Display8 is on Vulkan and this plugin could not open it: nothing is drawn"));
                    self.mode.set(D3D9);
                    D3D9
                }
            }
            BACKEND_D3D9 => {
                self.mode.set(D3D9);
                D3D9
            }
            // Not decided yet (before the engine's Direct3DCreate8): Direct3D 9's answer
            // for now, asked again next time.
            _ => UNDECIDED,
        }
    }

    /// The engine has a device: Display8 has said by now if it ever will (forward mode
    /// never does), so an undecided backend is Direct3D 9 from here on.
    fn settle(&self) -> u8 {
        let m = self.mode();
        if m == UNDECIDED {
            self.mode.set(D3D9);
            return D3D9;
        }
        m
    }

    /// The backend to make an object on: Direct3D 9 while Display8 has not said (an
    /// object made then draws only if it says Direct3D 9; logged once).
    fn make_on(&self) -> u8 {
        match self.mode() {
            UNDECIDED => {
                if !self.early.replace(true) {
                    (self.log)(format_args!("gpu: an object is made before Display8 has said which backend draws: made for Direct3D 9"));
                }
                D3D9
            }
            m => m,
        }
    }

    pub fn backend(&self) -> Backend {
        if self.mode() == VULKAN { Backend::Vulkan } else { Backend::D3d9 }
    }

    pub fn is_vulkan(&self) -> bool {
        self.mode() == VULKAN
    }

    /// The Direct3D 9 backend (the plugins' engine glue that is Direct3D 9's alone).
    pub fn d3d9(&self) -> &d3d9::Gpu {
        &self.d9
    }

    /// Half a pixel where the backend puts pixel centres on whole coordinates
    /// (Direct3D 9, and Vulkan in the comparison mode), 0 where they are at the half (a
    /// caller that maps pixels to clip space adds it, as the UI's `VP` does).
    pub fn half_pixel(&self) -> f32 {
        if self.mode() == VULKAN && !self.vk.d3d9_centres() { 0.0 } else { 0.5 }
    }

    // -- the engine's device --

    /// The engine's device, every frame: Direct3D 9's behind `dev8` (attached, as
    /// `attach_d3d8`), or Display8's Vulkan device. None: nothing to draw on.
    pub fn attach_engine(&self, dev8: *mut c_void) -> Option<Attached> {
        if dev8.is_null() && self.mode() != VULKAN {
            return None;
        }
        if self.settle() == VULKAN {
            let id = self.vk.ready().then_some(self as *const Gpu as usize)?;
            let new = self.vk_seen.get() != id;
            self.vk_seen.set(id);
            return Some(Attached { id, new });
        }
        let d = Dev::from_d3d8(dev8)?;
        let new = self.d9.attach(d);
        Some(Attached { id: d.p() as usize, new })
    }

    /// The engine's device's identity, without attaching it: what `attach_engine`'s
    /// `id` will be (to let go of what was made for another first).
    pub fn engine_id(&self, dev8: *mut c_void) -> Option<usize> {
        if dev8.is_null() && self.mode() != VULKAN {
            return None;
        }
        if self.settle() == VULKAN {
            return self.vk.ready().then_some(self as *const Gpu as usize);
        }
        Dev::from_d3d8(dev8).map(|d| d.p() as usize)
    }

    /// Direct3D 9: the device to draw on (`d3d9::Gpu::attach`).
    pub fn attach(&self, dev: Dev) -> bool {
        self.mode() != VULKAN && self.d9.attach(dev)
    }

    pub fn attach_d3d8(&self, dev8: *mut c_void) -> Option<Dev> {
        if self.mode() == VULKAN {
            return None;
        }
        self.d9.attach_d3d8(dev8)
    }

    /// Before the device's `Reset` (Direct3D 9; Display8's Vulkan device is never lost).
    pub fn before_reset(&self) {
        if self.mode() != VULKAN {
            self.d9.before_reset();
        }
    }

    /// Before the engine releases its device (Direct3D 9; nothing on Vulkan).
    pub fn release_device(&self) {
        if self.mode() != VULKAN {
            self.d9.release_device();
        }
    }

    // -- resources --

    pub fn texture(&self, desc: TextureDesc, contents: Contents) -> Texture {
        if self.make_on() == VULKAN { Texture::Vk(self.vk.texture(desc, contents)) } else { Texture::D9(self.d9.texture(desc, contents)) }
    }

    pub fn wrap_d3d9_texture(&self, tex: d3d9::raw::Com) -> Texture {
        Texture::D9(self.d9.wrap_d3d9_texture(tex))
    }

    pub fn wrap_d3d9_surface(&self, surf: d3d9::raw::Com) -> Texture {
        Texture::D9(self.d9.wrap_d3d9_surface(surf))
    }

    /// An engine texture (an `IDirect3DTexture8` the engine made) to sample: d3d8to9's
    /// (or Display8's) Direct3D 9 texture behind it, or Display8's Vulkan mirror of it.
    pub fn engine_texture(&self, tex8: *mut c_void) -> Option<Texture> {
        if tex8.is_null() {
            return None;
        }
        if self.mode() == VULKAN {
            return self.vk.engine_texture(tex8).map(Texture::Vk);
        }
        let t9 = d3d9::raw::query(tex8, &d3d9::raw::IID_TEXTURE9)?;
        Some(self.wrap_d3d9_texture(t9))
    }

    pub fn engine_target(&self) -> Option<Texture> {
        if self.mode() == VULKAN { self.vk.engine_target().map(Texture::Vk) } else { self.d9.engine_target().map(Texture::D9) }
    }

    pub fn engine_depth(&self) -> Option<Texture> {
        if self.mode() == VULKAN { self.vk.engine_depth().map(Texture::Vk) } else { self.d9.engine_depth().map(Texture::D9) }
    }

    /// The viewport the engine has set.
    pub fn engine_viewport(&self) -> Option<Viewport> {
        if self.mode() == VULKAN { vulkan::engine_viewport(&self.vk) } else { self.d9.engine_viewport() }
    }

    /// A transform the engine has set (`D3DTRANSFORMSTATETYPE`), as Direct3D sets it.
    pub fn engine_transform(&self, state: u32) -> Option<[[f32; 4]; 4]> {
        if self.mode() == VULKAN {
            return vulkan::engine_transform(&self.vk, state);
        }
        Some(self.d9.device()?.get_transform(state))
    }

    pub fn buffer(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        if self.make_on() == VULKAN { Buffer::Vk(self.vk.buffer(desc, data)) } else { Buffer::D9(self.d9.buffer(desc, data)) }
    }

    pub fn buffer_transient(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        if self.make_on() == VULKAN { Buffer::Vk(self.vk.buffer_transient(desc, data)) } else { Buffer::D9(self.d9.buffer_transient(desc, data)) }
    }

    pub fn timer(&self, n: usize) -> Timer {
        if self.make_on() == VULKAN { Timer::Vk(self.vk.timer(n)) } else { Timer::D9(self.d9.timer(n)) }
    }

    pub fn pipeline(&self, desc: PipelineDesc) -> Pipeline {
        if self.make_on() == VULKAN { Pipeline::Vk(self.vk.pipeline(desc)) } else { Pipeline::D9(self.d9.pipeline(desc)) }
    }

    // -- commands --

    pub fn begin<'a>(&'a self, frame: &'a mut Frame) -> Option<Cmd<'a>> {
        let inner = if self.mode() == VULKAN { CmdInner::Vk(self.vk.begin(frame)?) } else { CmdInner::D9(self.d9.begin(frame)?) };
        Some(Cmd { gpu: self, inner })
    }

    pub fn read_back<R>(&self, tex: &Texture, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        self.read_back_layer(tex, 0, f)
    }

    pub fn read_back_layer<R>(&self, tex: &Texture, layer: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        match tex {
            Texture::D9(t) => self.d9.read_back_layer(t, layer, f),
            Texture::Vk(t) => self.vk.read_back_layer(t, layer, f),
        }
    }

    /// The frame presented last at `w` x `h`, as `Bgra8` on Direct3D 9; on Vulkan the
    /// engine's back buffer as it is now (before the gamma ramp), as `Bgrx8`.
    pub fn read_front_buffer<R>(&self, w: u32, h: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, i32> {
        if self.mode() == VULKAN {
            let t = self.vk.engine_target().ok_or(-1)?;
            return self.vk.read_back_layer(&t, 0, f).map_err(|_| -1);
        }
        self.d9.read_front_buffer(w, h, f)
    }
}

// ---- objects --------------------------------------------------------------------------

/// A texture: sampled, a target, or both. Clones share it. Valid until the last clone
/// is dropped.
#[derive(Clone)]
pub enum Texture {
    D9(d3d9::Texture),
    Vk(vulkan::Texture),
}

impl Texture {
    pub fn desc(&self) -> TextureDesc {
        match self {
            Texture::D9(t) => t.desc(),
            Texture::Vk(t) => t.desc(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        match self {
            Texture::D9(t) => t.size(),
            Texture::Vk(t) => t.size(),
        }
    }

    /// Makes it on the device now: false if the device cannot.
    pub fn ready(&self, gpu: &Gpu) -> bool {
        match self {
            Texture::D9(t) => t.ready(&gpu.d9),
            Texture::Vk(t) => t.ready(&gpu.vk),
        }
    }

    pub fn write(&self, src: *const u8, pitch: usize, w: u32, rows: u32) {
        match self {
            Texture::D9(t) => t.write(src, pitch, w, rows),
            Texture::Vk(t) => t.write(src, pitch, w, rows),
        }
    }

    pub fn same(&self, o: &Texture) -> bool {
        match (self, o) {
            (Texture::D9(a), Texture::D9(b)) => a.same(b),
            (Texture::Vk(a), Texture::Vk(b)) => a.same(b),
            _ => false,
        }
    }

    pub(crate) fn d9(&self) -> Option<&d3d9::Texture> {
        match self {
            Texture::D9(t) => Some(t),
            _ => None,
        }
    }

    pub(crate) fn vk(&self) -> Option<&vulkan::Texture> {
        match self {
            Texture::Vk(t) => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum Buffer {
    D9(d3d9::Buffer),
    Vk(vulkan::Buffer),
}

impl Buffer {
    pub fn desc(&self) -> BufferDesc {
        match self {
            Buffer::D9(b) => b.desc(),
            Buffer::Vk(b) => b.desc(),
        }
    }
}

#[derive(Clone)]
pub enum Pipeline {
    D9(d3d9::Pipeline),
    Vk(vulkan::Pipeline),
}

impl Pipeline {
    pub fn desc(&self) -> &PipelineDesc {
        match self {
            Pipeline::D9(p) => p.desc(),
            Pipeline::Vk(p) => p.desc(),
        }
    }

    pub fn ready(&self, gpu: &Gpu) -> bool {
        match self {
            Pipeline::D9(p) => p.ready(&gpu.d9),
            Pipeline::Vk(p) => p.ready(&gpu.vk),
        }
    }
}

#[derive(Clone)]
pub enum Timer {
    D9(d3d9::Timer),
    Vk(vulkan::Timer),
}

impl Timer {
    pub fn begin(&self, gpu: &Gpu) -> bool {
        match self {
            Timer::D9(t) => t.begin(&gpu.d9),
            Timer::Vk(t) => t.begin(&gpu.vk),
        }
    }

    pub fn stamp(&self, gpu: &Gpu, i: usize) {
        match self {
            Timer::D9(t) => t.stamp(&gpu.d9, i),
            Timer::Vk(t) => t.stamp(&gpu.vk, i),
        }
    }

    pub fn end(&self, gpu: &Gpu) {
        match self {
            Timer::D9(t) => t.end(&gpu.d9),
            Timer::Vk(t) => t.end(&gpu.vk),
        }
    }

    pub fn read(&self, gpu: &Gpu, out: &mut [u64]) -> Option<TimerResult> {
        match self {
            Timer::D9(t) => t.read(&gpu.d9, out),
            Timer::Vk(t) => t.read(&gpu.vk, out),
        }
    }
}

// ---- recording ------------------------------------------------------------------------

enum CmdInner<'a> {
    D9(d3d9::Cmd<'a>),
    Vk(vulkan::Cmd<'a>),
}

/// Commands recorded into a frame (`Gpu::begin`): applied at once on Direct3D 9, into
/// Display8's command buffer on Vulkan. Either way the engine finds its state as it left
/// it when the `Cmd` is dropped.
pub struct Cmd<'a> {
    gpu: &'a Gpu,
    inner: CmdInner<'a>,
}

macro_rules! both {
    ($s:expr, $c:ident => $e:expr) => {
        match &mut $s.inner {
            CmdInner::D9($c) => $e,
            CmdInner::Vk($c) => $e,
        }
    };
}

impl<'a> Cmd<'a> {
    pub fn gpu(&self) -> &'a Gpu {
        self.gpu
    }

    pub fn frame(&mut self) -> &mut Frame {
        both!(self, c => c.frame())
    }

    pub fn begin_pass(&mut self, pass: &Pass) -> bool {
        both!(self, c => c.begin_pass(pass))
    }

    pub fn end_pass(&mut self) {
        both!(self, c => c.end_pass())
    }

    pub fn set_viewport(&mut self, vp: &Viewport) {
        both!(self, c => c.set_viewport(vp))
    }

    pub fn set_scissor(&mut self, r: Rect) {
        both!(self, c => c.set_scissor(r))
    }

    pub fn clear_rects(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        both!(self, c => c.clear_rects(rects, color, depth))
    }

    pub fn set_pipeline(&mut self, p: &Pipeline) -> bool {
        match (&mut self.inner, p) {
            (CmdInner::D9(c), Pipeline::D9(p)) => c.set_pipeline(p),
            (CmdInner::Vk(c), Pipeline::Vk(p)) => c.set_pipeline(p),
            _ => false,
        }
    }

    pub fn set_sampler(&mut self, slot: u32, s: &Sampler) {
        both!(self, c => c.set_sampler(slot, s))
    }

    pub fn bind_texture(&mut self, slot: u32, t: Option<&Texture>, s: &Sampler) {
        match &mut self.inner {
            CmdInner::D9(c) => c.bind_texture(slot, t.and_then(|t| t.d9()), s),
            CmdInner::Vk(c) => c.bind_texture(slot, t.and_then(|t| t.vk()), s),
        }
    }

    pub fn vs_block(&mut self, b: Block) {
        both!(self, c => c.vs_block(b))
    }

    pub fn ps_block(&mut self, b: Block) {
        both!(self, c => c.ps_block(b))
    }

    /// Binds a binding group: its textures with their samplers, its uniform blocks.
    pub fn bind(&mut self, g: &Bindings) {
        for t in g.textures {
            self.bind_texture(t.slot, t.texture, &t.sampler);
        }
        for b in g.vs {
            self.vs_block(*b);
        }
        for b in g.ps {
            self.ps_block(*b);
        }
    }

    pub fn vertices<T: Copy>(&mut self, data: &[T]) -> Option<Span> {
        both!(self, c => c.vertices(data))
    }

    pub fn indices(&mut self, data: &[u16]) -> Option<Span> {
        both!(self, c => c.indices(data))
    }

    pub fn draw(&mut self, v: &Span) {
        both!(self, c => c.draw(v))
    }

    pub fn draw_indexed(&mut self, v: &Span, base_vertex: u32, vertices: u32, i: &Span, first: u32, count: u32) {
        both!(self, c => c.draw_indexed(v, base_vertex, vertices, i, first, count))
    }

    pub fn draw_buffer(&mut self, b: &Buffer, stride: u32, first: u32, count: u32) {
        match (&mut self.inner, b) {
            (CmdInner::D9(c), Buffer::D9(b)) => c.draw_buffer(b, stride, first, count),
            (CmdInner::Vk(c), Buffer::Vk(b)) => c.draw_buffer(b, stride, first, count),
            _ => {}
        }
    }

    pub fn span_of(&mut self, b: &Buffer, stride: u32) -> Option<Span> {
        match (&mut self.inner, b) {
            (CmdInner::D9(c), Buffer::D9(b)) => c.span_of(b, stride),
            (CmdInner::Vk(c), Buffer::Vk(b)) => c.span_of(b, stride),
            _ => None,
        }
    }

    pub fn generate_mips(&mut self, t: &Texture) {
        match (&mut self.inner, t) {
            (CmdInner::D9(c), Texture::D9(t)) => c.generate_mips(t),
            (CmdInner::Vk(c), Texture::Vk(t)) => c.generate_mips(t),
            _ => {}
        }
    }

    pub fn resolve_depth(&mut self, src: &Texture, dst: &Texture) -> bool {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.resolve_depth(s, d),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.resolve_depth(s, d),
            _ => false,
        }
    }

    pub fn resolve(&mut self, src: &Texture, dst: &Texture, r: Rect) {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.resolve(s, d, r),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.resolve(s, d, r),
            _ => {}
        }
    }

    pub fn copy(&mut self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, f: Filter) -> bool {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.copy(s, sr, d, dr, f),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.copy(s, sr, d, dr, f),
            _ => false,
        }
    }

    pub fn read_back<R>(&mut self, tex: &Texture, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        self.gpu.read_back(tex, f)
    }

    pub fn read_back_layer<R>(&mut self, tex: &Texture, layer: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        self.gpu.read_back_layer(tex, layer, f)
    }
}
