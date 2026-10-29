//! The interface's objects, on the backend the plugins draw on: Direct3D 9 (`d3d9`,
//! the device behind the game's d3d8 one, or Display8's own in stub mode), Vulkan
//! (`vulkan`, Display8's device in stub mode with `Backend=vulkan`) or Direct3D 12
//! (`dx12`, with `Backend=d3d12`). Which one is chosen at run time, once, by what
//! Display8 says (`display8_backend`): with no Display8, an older one, or one that says
//! Direct3D 9, it is Direct3D 9 as before. Each object is an enum of the backends' own,
//! and every call goes to the one it was made on.
use crate::d3d9::raw::Device as Dev;
use crate::display::{Display8, Display8Dx12, BACKEND_D3D12, BACKEND_D3D9, BACKEND_VULKAN};
use crate::frame::{Frame, Span};
use crate::pixels::Mapped;
use crate::types::*;
use crate::{d3d9, dx12, vulkan};
use core::cell::Cell;
use core::ffi::c_void;

pub type Log = fn(core::fmt::Arguments);

const UNDECIDED: u8 = 0;
const D3D9: u8 = 1;
const VULKAN: u8 = 2;
const D3D12: u8 = 3;

/// Which backend the objects are made on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    D3d9,
    Vulkan,
    D3d12,
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
    dx: dx12::Gpu,
    lookup: Option<fn(&str) -> usize>,
    mode: Cell<u8>,
    /// Display8's device last handed to `attach_engine` (it never changes).
    seen: Cell<usize>,
    /// An object was made before Display8 had said (logged once).
    early: Cell<bool>,
    /// Display8's exports, looked up once.
    disp: Cell<Option<Display8>>,
}

impl Gpu {
    /// Direct3D 9 only (no Display8 is asked).
    pub const fn new(log: Log) -> Gpu {
        Gpu { log, d9: d3d9::Gpu::new(log), vk: vulkan::Gpu::new(log), dx: dx12::Gpu::new(log), lookup: None, mode: Cell::new(D3D9), seen: Cell::new(0), early: Cell::new(false), disp: Cell::new(None) }
    }

    /// On the backend Display8 says, through `lookup` (the plugin's `GetProcAddress` on
    /// `d3d8.dll`; 0 for a name it lacks).
    pub const fn with_display(log: Log, lookup: fn(&str) -> usize) -> Gpu {
        Self::on_display_of(log, lookup, false)
    }

    /// As `with_display`, publishing the pass now to Display8 at each `Cmd`'s end on
    /// Vulkan, so that the engine's draws Display8 passes through land in it
    /// (gpu/src/display.rs, "Passthrough"): render3d's, whose scene pass the draws it
    /// hands back to the engine belong in.
    pub const fn with_display_publishing(log: Log, lookup: fn(&str) -> usize) -> Gpu {
        Self::on_display_of(log, lookup, true)
    }

    const fn on_display_of(log: Log, lookup: fn(&str) -> usize, publish: bool) -> Gpu {
        Gpu {
            log,
            d9: d3d9::Gpu::new(log),
            vk: if publish { vulkan::Gpu::publishing(log) } else { vulkan::Gpu::new(log) },
            dx: dx12::Gpu::new(log),
            lookup: Some(lookup),
            mode: Cell::new(UNDECIDED),
            seen: Cell::new(0),
            early: Cell::new(false),
            disp: Cell::new(None),
        }
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
            BACKEND_D3D12 => {
                let Some(x) = Display8Dx12::load(lookup) else {
                    (self.log)(format_args!("gpu: Display8 is on Direct3D 12 but has no Direct3D 12 exports: nothing is drawn"));
                    self.mode.set(D3D9);
                    return D3D9;
                };
                match self.dx.init(d, x) {
                    Some(true) => {
                        self.mode.set(D3D12);
                        D3D12
                    }
                    Some(false) => {
                        (self.log)(format_args!("gpu: Display8 is on Direct3D 12 and this plugin could not open it: nothing is drawn"));
                        self.mode.set(D3D9);
                        D3D9
                    }
                    // Its device is not made yet: asked again next time.
                    None => UNDECIDED,
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
        match self.mode() {
            VULKAN => Backend::Vulkan,
            D3D12 => Backend::D3d12,
            _ => Backend::D3d9,
        }
    }

    pub fn is_vulkan(&self) -> bool {
        self.mode() == VULKAN
    }

    /// Whether the plugins draw on Display8's own device (Vulkan or Direct3D 12), not a
    /// Direct3D 9 one: the engine's targets, viewport and transforms are Display8's.
    pub fn on_display(&self) -> bool {
        matches!(self.mode(), VULKAN | D3D12)
    }

    /// The backend's name, for the logs.
    pub fn backend_name(&self) -> &'static str {
        match self.mode() {
            VULKAN => "Vulkan",
            D3D12 => "Direct3D 12",
            _ => "d3d9",
        }
    }

    /// The Direct3D 9 backend (the plugins' engine glue that is Direct3D 9's alone).
    pub fn d3d9(&self) -> &d3d9::Gpu {
        &self.d9
    }

    /// Half a pixel where the backend puts pixel centres on whole coordinates
    /// (Direct3D 9, and Vulkan or Direct3D 12 in the comparison mode), 0 where they are
    /// at the half (a caller that maps pixels to clip space adds it, as the UI's `VP`
    /// does).
    pub fn half_pixel(&self) -> f32 {
        match self.mode() {
            VULKAN if !self.vk.d3d9_centres() => 0.0,
            D3D12 if !self.dx.d3d9_centres() => 0.0,
            _ => 0.5,
        }
    }

    // -- the engine's device --

    /// The engine's device, every frame: Direct3D 9's behind `dev8` (attached, as
    /// `attach_d3d8`), or Display8's Vulkan or Direct3D 12 device. None: nothing to draw
    /// on.
    pub fn attach_engine(&self, dev8: *mut c_void) -> Option<Attached> {
        if dev8.is_null() && !self.on_display() {
            return None;
        }
        if let Some(id) = self.display_id() {
            let id = id?;
            let new = self.seen.get() != id;
            self.seen.set(id);
            return Some(Attached { id, new });
        }
        let d = Dev::from_d3d8(dev8)?;
        let new = self.d9.attach(d);
        Some(Attached { id: d.p() as usize, new })
    }

    /// Some on Display8's device (its identity, None while it cannot be drawn on); None
    /// on Direct3D 9.
    fn display_id(&self) -> Option<Option<usize>> {
        match self.settle() {
            VULKAN => Some(self.vk.ready().then_some(self as *const Gpu as usize)),
            D3D12 => Some(self.dx.ready().then_some(self as *const Gpu as usize)),
            _ => None,
        }
    }

    /// The engine's device's identity, without attaching it: what `attach_engine`'s
    /// `id` will be (to let go of what was made for another first).
    pub fn engine_id(&self, dev8: *mut c_void) -> Option<usize> {
        if dev8.is_null() && !self.on_display() {
            return None;
        }
        if let Some(id) = self.display_id() {
            return id;
        }
        Dev::from_d3d8(dev8).map(|d| d.p() as usize)
    }

    /// Direct3D 9: the device to draw on (`d3d9::Gpu::attach`).
    pub fn attach(&self, dev: Dev) -> bool {
        !self.on_display() && self.d9.attach(dev)
    }

    pub fn attach_d3d8(&self, dev8: *mut c_void) -> Option<Dev> {
        if self.on_display() {
            return None;
        }
        self.d9.attach_d3d8(dev8)
    }

    /// Before the device's `Reset` (Direct3D 9; Display8's devices are never lost).
    pub fn before_reset(&self) {
        if !self.on_display() {
            self.d9.before_reset();
        }
    }

    /// Before the engine releases its device (Direct3D 9; nothing on Display8's).
    pub fn release_device(&self) {
        if !self.on_display() {
            self.d9.release_device();
        }
    }

    // -- resources --

    pub fn texture(&self, desc: TextureDesc, contents: Contents) -> Texture {
        match self.make_on() {
            VULKAN => Texture::Vk(self.vk.texture(desc, contents)),
            D3D12 => Texture::Dx(self.dx.texture(desc, contents)),
            _ => Texture::D9(self.d9.texture(desc, contents)),
        }
    }

    pub fn wrap_d3d9_texture(&self, tex: d3d9::raw::Com) -> Texture {
        Texture::D9(self.d9.wrap_d3d9_texture(tex))
    }

    pub fn wrap_d3d9_surface(&self, surf: d3d9::raw::Com) -> Texture {
        Texture::D9(self.d9.wrap_d3d9_surface(surf))
    }

    /// An engine texture (an `IDirect3DTexture8` the engine made) to sample: d3d8to9's
    /// (or Display8's) Direct3D 9 texture behind it, or Display8's mirror of it.
    pub fn engine_texture(&self, tex8: *mut c_void) -> Option<Texture> {
        if tex8.is_null() {
            return None;
        }
        match self.mode() {
            VULKAN => self.vk.engine_texture(tex8).map(Texture::Vk),
            D3D12 => self.dx.engine_texture(tex8).map(Texture::Dx),
            _ => {
                let t9 = d3d9::raw::query(tex8, &d3d9::raw::IID_TEXTURE9)?;
                Some(self.wrap_d3d9_texture(t9))
            }
        }
    }

    pub fn engine_target(&self) -> Option<Texture> {
        match self.mode() {
            VULKAN => self.vk.engine_target().map(Texture::Vk),
            D3D12 => self.dx.engine_target().map(Texture::Dx),
            _ => self.d9.engine_target().map(Texture::D9),
        }
    }

    pub fn engine_depth(&self) -> Option<Texture> {
        match self.mode() {
            VULKAN => self.vk.engine_depth().map(Texture::Vk),
            D3D12 => self.dx.engine_depth().map(Texture::Dx),
            _ => self.d9.engine_depth().map(Texture::D9),
        }
    }

    /// The viewport the engine has set.
    pub fn engine_viewport(&self) -> Option<Viewport> {
        match self.mode() {
            VULKAN => vulkan::engine_viewport(&self.vk),
            D3D12 => dx12::engine_viewport(&self.dx),
            _ => self.d9.engine_viewport(),
        }
    }

    /// A transform the engine has set (`D3DTRANSFORMSTATETYPE`), as Direct3D sets it.
    pub fn engine_transform(&self, state: u32) -> Option<[[f32; 4]; 4]> {
        match self.mode() {
            VULKAN => vulkan::engine_transform(&self.vk, state),
            D3D12 => dx12::engine_transform(&self.dx, state),
            _ => Some(self.d9.device()?.get_transform(state)),
        }
    }

    pub fn buffer(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        match self.make_on() {
            VULKAN => Buffer::Vk(self.vk.buffer(desc, data)),
            D3D12 => Buffer::Dx(self.dx.buffer(desc, data)),
            _ => Buffer::D9(self.d9.buffer(desc, data)),
        }
    }

    pub fn buffer_transient(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        match self.make_on() {
            VULKAN => Buffer::Vk(self.vk.buffer_transient(desc, data)),
            D3D12 => Buffer::Dx(self.dx.buffer_transient(desc, data)),
            _ => Buffer::D9(self.d9.buffer_transient(desc, data)),
        }
    }

    /// Whether compute and storage are there (`Cmd::dispatch`, `Gpu::storage`, storage
    /// images): on Vulkan and Direct3D 12; never on Direct3D 9, where every storage
    /// object is an empty one and every dispatch does nothing.
    pub fn compute(&self) -> bool {
        match self.mode() {
            VULKAN => self.vk.compute(),
            D3D12 => self.dx.compute(),
            _ => false,
        }
    }

    /// A storage buffer (`StorageDesc`); an empty one on Direct3D 9.
    pub fn storage(&self, desc: StorageDesc) -> Storage {
        match self.make_on() {
            VULKAN => Storage::Vk(self.vk.storage(desc)),
            D3D12 => Storage::Dx(self.dx.storage(desc)),
            _ => Storage::None,
        }
    }

    /// A compute pipeline; an empty one on Direct3D 9 (never ready).
    pub fn compute_pipeline(&self, desc: ComputeDesc) -> ComputePipeline {
        match self.make_on() {
            VULKAN => ComputePipeline::Vk(self.vk.compute_pipeline(desc)),
            D3D12 => ComputePipeline::Dx(self.dx.compute_pipeline(desc)),
            _ => ComputePipeline::None,
        }
    }

    pub fn timer(&self, n: usize) -> Timer {
        match self.make_on() {
            VULKAN => Timer::Vk(self.vk.timer(n)),
            D3D12 => Timer::Dx(self.dx.timer(n)),
            _ => Timer::D9(self.d9.timer(n)),
        }
    }

    pub fn pipeline(&self, desc: PipelineDesc) -> Pipeline {
        match self.make_on() {
            VULKAN => Pipeline::Vk(self.vk.pipeline(desc)),
            D3D12 => Pipeline::Dx(self.dx.pipeline(desc)),
            _ => Pipeline::D9(self.d9.pipeline(desc)),
        }
    }

    // -- commands --

    pub fn begin<'a>(&'a self, frame: &'a mut Frame) -> Option<Cmd<'a>> {
        let inner = match self.mode() {
            VULKAN => CmdInner::Vk(self.vk.begin(frame)?),
            D3D12 => CmdInner::Dx(self.dx.begin(frame)?),
            _ => CmdInner::D9(self.d9.begin(frame)?),
        };
        Some(Cmd { gpu: self, inner })
    }

    pub fn read_back<R>(&self, tex: &Texture, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        self.read_back_layer(tex, 0, f)
    }

    pub fn read_back_layer<R>(&self, tex: &Texture, layer: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        match tex {
            Texture::D9(t) => self.d9.read_back_layer(t, layer, f),
            Texture::Vk(t) => self.vk.read_back_layer(t, layer, f),
            Texture::Dx(t) => self.dx.read_back_layer(t, layer, f),
        }
    }

    /// The frame presented last at `w` x `h`, as `Bgra8` on Direct3D 9; on Display8's
    /// devices the engine's back buffer as it is now (before the gamma ramp), as `Bgrx8`.
    pub fn read_front_buffer<R>(&self, w: u32, h: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, i32> {
        match self.mode() {
            VULKAN => {
                let t = self.vk.engine_target().ok_or(-1)?;
                self.vk.read_back_layer(&t, 0, f).map_err(|_| -1)
            }
            D3D12 => {
                let t = self.dx.engine_target().ok_or(-1)?;
                self.dx.read_back_layer(&t, 0, f).map_err(|_| -1)
            }
            _ => self.d9.read_front_buffer(w, h, f),
        }
    }
}

// ---- objects --------------------------------------------------------------------------

/// A texture: sampled, a target, or both. Clones share it. Valid until the last clone
/// is dropped.
#[derive(Clone)]
pub enum Texture {
    D9(d3d9::Texture),
    Vk(vulkan::Texture),
    Dx(dx12::Texture),
}

macro_rules! each {
    ($s:expr, $t:ident => $e:expr) => {
        match $s {
            Texture::D9($t) => $e,
            Texture::Vk($t) => $e,
            Texture::Dx($t) => $e,
        }
    };
}

impl Texture {
    pub fn desc(&self) -> TextureDesc {
        each!(self, t => t.desc())
    }

    pub fn size(&self) -> (u32, u32) {
        each!(self, t => t.size())
    }

    /// Makes it on the device now: false if the device cannot.
    pub fn ready(&self, gpu: &Gpu) -> bool {
        match self {
            Texture::D9(t) => t.ready(&gpu.d9),
            Texture::Vk(t) => t.ready(&gpu.vk),
            Texture::Dx(t) => t.ready(&gpu.dx),
        }
    }

    pub fn write(&self, src: *const u8, pitch: usize, w: u32, rows: u32) {
        each!(self, t => t.write(src, pitch, w, rows))
    }

    pub fn same(&self, o: &Texture) -> bool {
        match (self, o) {
            (Texture::D9(a), Texture::D9(b)) => a.same(b),
            (Texture::Vk(a), Texture::Vk(b)) => a.same(b),
            (Texture::Dx(a), Texture::Dx(b)) => a.same(b),
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

    pub(crate) fn dx(&self) -> Option<&dx12::Texture> {
        match self {
            Texture::Dx(t) => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum Buffer {
    D9(d3d9::Buffer),
    Vk(vulkan::Buffer),
    Dx(dx12::Buffer),
}

impl Buffer {
    pub fn desc(&self) -> BufferDesc {
        match self {
            Buffer::D9(b) => b.desc(),
            Buffer::Vk(b) => b.desc(),
            Buffer::Dx(b) => b.desc(),
        }
    }
}

#[derive(Clone)]
pub enum Pipeline {
    D9(d3d9::Pipeline),
    Vk(vulkan::Pipeline),
    Dx(dx12::Pipeline),
}

impl Pipeline {
    pub fn desc(&self) -> &PipelineDesc {
        match self {
            Pipeline::D9(p) => p.desc(),
            Pipeline::Vk(p) => p.desc(),
            Pipeline::Dx(p) => p.desc(),
        }
    }

    pub fn ready(&self, gpu: &Gpu) -> bool {
        match self {
            Pipeline::D9(p) => p.ready(&gpu.d9),
            Pipeline::Vk(p) => p.ready(&gpu.vk),
            Pipeline::Dx(p) => p.ready(&gpu.dx),
        }
    }
}

/// A storage buffer (`Gpu::storage`). Clones share it. `None` on Direct3D 9.
#[derive(Clone)]
pub enum Storage {
    None,
    Vk(vulkan::Storage),
    Dx(dx12::Storage),
}

impl Storage {
    pub fn desc(&self) -> Option<StorageDesc> {
        match self {
            Storage::None => None,
            Storage::Vk(s) => Some(s.desc()),
            Storage::Dx(s) => Some(s.desc()),
        }
    }

    fn vk(&self) -> Option<&vulkan::Storage> {
        match self {
            Storage::Vk(s) => Some(s),
            _ => None,
        }
    }

    fn dx(&self) -> Option<&dx12::Storage> {
        match self {
            Storage::Dx(s) => Some(s),
            _ => None,
        }
    }
}

/// A compute pipeline (`Gpu::compute_pipeline`). `None` on Direct3D 9.
#[derive(Clone)]
pub enum ComputePipeline {
    None,
    Vk(vulkan::ComputePipeline),
    Dx(dx12::ComputePipeline),
}

impl ComputePipeline {
    /// Made on the device now: false if it cannot be (or on Direct3D 9).
    pub fn ready(&self, gpu: &Gpu) -> bool {
        match self {
            ComputePipeline::None => false,
            ComputePipeline::Vk(p) => p.ready(&gpu.vk),
            ComputePipeline::Dx(p) => p.ready(&gpu.dx),
        }
    }
}

#[derive(Clone)]
pub enum Timer {
    D9(d3d9::Timer),
    Vk(vulkan::Timer),
    Dx(dx12::Timer),
}

impl Timer {
    pub fn begin(&self, gpu: &Gpu) -> bool {
        match self {
            Timer::D9(t) => t.begin(&gpu.d9),
            Timer::Vk(t) => t.begin(&gpu.vk),
            Timer::Dx(t) => t.begin(&gpu.dx),
        }
    }

    pub fn stamp(&self, gpu: &Gpu, i: usize) {
        match self {
            Timer::D9(t) => t.stamp(&gpu.d9, i),
            Timer::Vk(t) => t.stamp(&gpu.vk, i),
            Timer::Dx(t) => t.stamp(&gpu.dx, i),
        }
    }

    pub fn end(&self, gpu: &Gpu) {
        match self {
            Timer::D9(t) => t.end(&gpu.d9),
            Timer::Vk(t) => t.end(&gpu.vk),
            Timer::Dx(t) => t.end(&gpu.dx),
        }
    }

    pub fn read(&self, gpu: &Gpu, out: &mut [u64]) -> Option<TimerResult> {
        match self {
            Timer::D9(t) => t.read(&gpu.d9, out),
            Timer::Vk(t) => t.read(&gpu.vk, out),
            Timer::Dx(t) => t.read(&gpu.dx, out),
        }
    }
}

// ---- recording ------------------------------------------------------------------------

enum CmdInner<'a> {
    D9(d3d9::Cmd<'a>),
    Vk(vulkan::Cmd<'a>),
    Dx(dx12::Cmd<'a>),
}

/// Commands recorded into a frame (`Gpu::begin`): applied at once on Direct3D 9, into
/// Display8's command buffer or list on Vulkan or Direct3D 12. Either way the engine
/// finds its state as it left it when the `Cmd` is dropped.
pub struct Cmd<'a> {
    gpu: &'a Gpu,
    inner: CmdInner<'a>,
}

macro_rules! all {
    ($s:expr, $c:ident => $e:expr) => {
        match &mut $s.inner {
            CmdInner::D9($c) => $e,
            CmdInner::Vk($c) => $e,
            CmdInner::Dx($c) => $e,
        }
    };
}

impl<'a> Cmd<'a> {
    pub fn gpu(&self) -> &'a Gpu {
        self.gpu
    }

    pub fn frame(&mut self) -> &mut Frame {
        all!(self, c => c.frame())
    }

    pub fn begin_pass(&mut self, pass: &Pass) -> bool {
        all!(self, c => c.begin_pass(pass))
    }

    pub fn end_pass(&mut self) {
        all!(self, c => c.end_pass())
    }

    pub fn set_viewport(&mut self, vp: &Viewport) {
        all!(self, c => c.set_viewport(vp))
    }

    pub fn set_scissor(&mut self, r: Rect) {
        all!(self, c => c.set_scissor(r))
    }

    pub fn clear_rects(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        all!(self, c => c.clear_rects(rects, color, depth))
    }

    pub fn set_pipeline(&mut self, p: &Pipeline) -> bool {
        match (&mut self.inner, p) {
            (CmdInner::D9(c), Pipeline::D9(p)) => c.set_pipeline(p),
            (CmdInner::Vk(c), Pipeline::Vk(p)) => c.set_pipeline(p),
            (CmdInner::Dx(c), Pipeline::Dx(p)) => c.set_pipeline(p),
            _ => false,
        }
    }

    pub fn set_sampler(&mut self, slot: u32, s: &Sampler) {
        all!(self, c => c.set_sampler(slot, s))
    }

    pub fn bind_texture(&mut self, slot: u32, t: Option<&Texture>, s: &Sampler) {
        match &mut self.inner {
            CmdInner::D9(c) => c.bind_texture(slot, t.and_then(|t| t.d9()), s),
            CmdInner::Vk(c) => c.bind_texture(slot, t.and_then(|t| t.vk()), s),
            CmdInner::Dx(c) => c.bind_texture(slot, t.and_then(|t| t.dx()), s),
        }
    }

    pub fn vs_block(&mut self, b: Block) {
        all!(self, c => c.vs_block(b))
    }

    pub fn ps_block(&mut self, b: Block) {
        all!(self, c => c.ps_block(b))
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

    // -- storage and compute (nothing on Direct3D 9) --

    /// A read-only storage buffer at `slot` (`STORAGE(n)`: t16 + n), for draws and
    /// dispatches.
    pub fn bind_storage(&mut self, slot: u32, s: Option<&Storage>) {
        match &mut self.inner {
            CmdInner::D9(_) => {}
            CmdInner::Vk(c) => c.bind_storage(slot, s.and_then(|s| s.vk())),
            CmdInner::Dx(c) => c.bind_storage(slot, s.and_then(|s| s.dx())),
        }
    }

    /// A read-write storage buffer at `slot` (`STORAGE_RW(n)`: u n), for dispatches.
    pub fn bind_storage_rw(&mut self, slot: u32, s: Option<&Storage>) {
        match &mut self.inner {
            CmdInner::D9(_) => {}
            CmdInner::Vk(c) => c.bind_storage_rw(slot, s.and_then(|s| s.vk())),
            CmdInner::Dx(c) => c.bind_storage_rw(slot, s.and_then(|s| s.dx())),
        }
    }

    /// A storage image (a texture made with `Usage::STORAGE`) at `slot`
    /// (`STORAGE_IMAGE_*(n)`: u 8 + n), for dispatches; its first level.
    pub fn bind_storage_image(&mut self, slot: u32, t: Option<&Texture>) {
        match &mut self.inner {
            CmdInner::D9(_) => {}
            CmdInner::Vk(c) => c.bind_storage_image(slot, t.and_then(|t| t.vk())),
            CmdInner::Dx(c) => c.bind_storage_image(slot, t.and_then(|t| t.dx())),
        }
    }

    /// Registers of the compute shaders' register file (their cbuffers at b0).
    pub fn cs_block(&mut self, b: Block) {
        match &mut self.inner {
            CmdInner::D9(_) => {}
            CmdInner::Vk(c) => c.cs_block(b),
            CmdInner::Dx(c) => c.cs_block(b),
        }
    }

    /// `data` into a storage buffer at byte `offset`, through the frame's upload ring,
    /// ordered before whatever reads it after (whole words).
    pub fn write_storage(&mut self, s: &Storage, offset: u32, data: &[u8]) {
        match (&mut self.inner, s) {
            (CmdInner::Vk(c), Storage::Vk(s)) => c.write_storage(s, offset, data),
            (CmdInner::Dx(c), Storage::Dx(s)) => c.write_storage(s, offset, data),
            _ => {}
        }
    }

    /// Runs a compute pipeline over `x` by `y` by `z` groups with what is bound now.
    /// Ordered after what came before and before what comes after: what it writes is
    /// read by the next draw or dispatch as it left it. False: nothing was run.
    pub fn dispatch(&mut self, p: &ComputePipeline, x: u32, y: u32, z: u32) -> bool {
        match (&mut self.inner, p) {
            (CmdInner::Vk(c), ComputePipeline::Vk(p)) => c.dispatch(p, x, y, z),
            (CmdInner::Dx(c), ComputePipeline::Dx(p)) => c.dispatch(p, x, y, z),
            _ => false,
        }
    }

    /// A barrier between everything recorded before and after (a dispatch and a copy
    /// already place their own).
    pub fn barrier(&mut self) {
        match &mut self.inner {
            CmdInner::D9(_) => {}
            CmdInner::Vk(c) => c.barrier(),
            CmdInner::Dx(c) => c.barrier(),
        }
    }

    pub fn vertices<T: Copy>(&mut self, data: &[T]) -> Option<Span> {
        all!(self, c => c.vertices(data))
    }

    pub fn indices(&mut self, data: &[u16]) -> Option<Span> {
        all!(self, c => c.indices(data))
    }

    pub fn draw(&mut self, v: &Span) {
        all!(self, c => c.draw(v))
    }

    pub fn draw_indexed(&mut self, v: &Span, base_vertex: u32, vertices: u32, i: &Span, first: u32, count: u32) {
        all!(self, c => c.draw_indexed(v, base_vertex, vertices, i, first, count))
    }

    pub fn draw_buffer(&mut self, b: &Buffer, stride: u32, first: u32, count: u32) {
        match (&mut self.inner, b) {
            (CmdInner::D9(c), Buffer::D9(b)) => c.draw_buffer(b, stride, first, count),
            (CmdInner::Vk(c), Buffer::Vk(b)) => c.draw_buffer(b, stride, first, count),
            (CmdInner::Dx(c), Buffer::Dx(b)) => c.draw_buffer(b, stride, first, count),
            _ => {}
        }
    }

    pub fn span_of(&mut self, b: &Buffer, stride: u32) -> Option<Span> {
        match (&mut self.inner, b) {
            (CmdInner::D9(c), Buffer::D9(b)) => c.span_of(b, stride),
            (CmdInner::Vk(c), Buffer::Vk(b)) => c.span_of(b, stride),
            (CmdInner::Dx(c), Buffer::Dx(b)) => c.span_of(b, stride),
            _ => None,
        }
    }

    pub fn generate_mips(&mut self, t: &Texture) {
        match (&mut self.inner, t) {
            (CmdInner::D9(c), Texture::D9(t)) => c.generate_mips(t),
            (CmdInner::Vk(c), Texture::Vk(t)) => c.generate_mips(t),
            (CmdInner::Dx(c), Texture::Dx(t)) => c.generate_mips(t),
            _ => {}
        }
    }

    pub fn resolve_depth(&mut self, src: &Texture, dst: &Texture) -> bool {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.resolve_depth(s, d),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.resolve_depth(s, d),
            (CmdInner::Dx(c), Texture::Dx(s), Texture::Dx(d)) => c.resolve_depth(s, d),
            _ => false,
        }
    }

    pub fn resolve(&mut self, src: &Texture, dst: &Texture, r: Rect) {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.resolve(s, d, r),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.resolve(s, d, r),
            (CmdInner::Dx(c), Texture::Dx(s), Texture::Dx(d)) => c.resolve(s, d, r),
            _ => {}
        }
    }

    pub fn copy(&mut self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, f: Filter) -> bool {
        match (&mut self.inner, src, dst) {
            (CmdInner::D9(c), Texture::D9(s), Texture::D9(d)) => c.copy(s, sr, d, dr, f),
            (CmdInner::Vk(c), Texture::Vk(s), Texture::Vk(d)) => c.copy(s, sr, d, dr, f),
            (CmdInner::Dx(c), Texture::Dx(s), Texture::Dx(d)) => c.copy(s, sr, d, dr, f),
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
