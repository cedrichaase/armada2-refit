//! The Direct3D 12 backend: the interface on the device Display8 (`display/`, the
//! game's `d3d8.dll` of ours) makes in stub mode with `Backend=d3d12`, reached through
//! the exports of `display.rs` (the contract). Display8 owns the device, the queue,
//! the swap chain, the engine's back buffer and depth, the two shader-visible
//! descriptor heaps and the frame: its command list, upload ring and descriptor
//! ranges. This backend records into that command list, on the game's thread, in the
//! order the calls come, and Display8 submits it at the engine's `Present`. README.md
//! beside this crate, "The Direct3D 12 backend", says how each part maps and why; it is
//! the Vulkan backend's plan (`vulkan/`) in Direct3D 12's terms:
//!
//! - **One root signature** for every pipeline: each stage's register file as a root
//!   constant buffer at b0 (vertex) or b1 (pixel) in every space a cbuffer may take
//!   (`tools/shaderc/dxil-spaces.py`), one table of the sixteen stages' textures (t0..t15),
//!   one table of one sampler per stage (s0..s15), and root constants at b2 (the
//!   alpha test, the blit's region).
//! - **The pass now** stays the pass across `Cmd`s, as on Direct3D 9 (its targets stay
//!   bound); its targets are set on the list at the first draw of each `Cmd`, as is
//!   everything else (another recorder may have set its own since).
//! - **States** are tracked per texture, the whole texture in one, and changed by a
//!   barrier when the next use needs another. Display8's (its targets, mirrors) are put
//!   back in theirs when the `Cmd` that changed them ends.
//! - **Descriptors**: every view of ours lives in heaps of the backend's own that
//!   shaders do not see, and is copied into the frame's ranges of Display8's heaps when a
//!   draw binds it (a table per set of textures, kept for the frame; a sampler once a
//!   frame).
//! - **Nothing is released while a frame may still use it**: what is let go of waits
//!   until Display8 has begun the frame two after (its fence has passed).
pub mod raw;
#[rustfmt::skip]
#[allow(clippy::all)]
mod shaders;
mod cmd;
mod pipe;
mod tex;

pub use cmd::Cmd;
pub use pipe::{ComputePipeline, Pipeline};
pub use tex::{Buffer, Storage, Texture, Timer};

use crate::d3d9::raw::Guid;
use crate::display::{Display8, Display8Dx12, Dx12Frame8, Dx12Image8, FEATURE_D3D9_CENTRES, HEAP_CBV_SRV_UAV, HEAP_SAMPLER};
use crate::types::*;
use alloc::collections::BTreeMap;
use alloc::rc::{Rc, Weak};
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use raw::*;

pub type Log = fn(core::fmt::Arguments);

macro_rules! dlog {
    ($g:expr, $($t:tt)*) => { ($g.log)(format_args!($($t)*)) };
}
pub(crate) use dlog;

/// Direct3D 9's sampler stages.
pub(crate) const SLOTS: usize = 16;
/// The register spaces a stage's cbuffers may take (`tools/shaderc/dxil-spaces.py`).
pub(crate) const SPACES: u32 = 8;
/// The root signature's parameters.
pub(crate) const RP_VS: u32 = 0;
pub(crate) const RP_PS: u32 = SPACES;
pub(crate) const RP_SRV: u32 = 2 * SPACES;
pub(crate) const RP_SAMPLER: u32 = RP_SRV + 1;
pub(crate) const RP_K: u32 = RP_SAMPLER + SLOTS as u32;
/// The table of the read-only storage buffers (t16..t23).
pub(crate) const RP_STORAGE: u32 = RP_K + 1;
/// The compute root signature's parameters: the register file at b0 in each space, the
/// textures, a sampler per stage, the read-only storage, the read-write storage and
/// storage images (u0..u15).
pub(crate) const RPC_CB: u32 = 0;
pub(crate) const RPC_SRV: u32 = SPACES;
pub(crate) const RPC_SAMPLER: u32 = RPC_SRV + 1;
pub(crate) const RPC_STORAGE: u32 = RPC_SAMPLER + SLOTS as u32;
pub(crate) const RPC_UAV: u32 = RPC_STORAGE + 1;
/// Root constants at b2: words.
pub(crate) const K_WORDS: u32 = 8;
/// The stages' register files, as Direct3D 9 has them (float4 registers).
pub(crate) const VS_REGS: usize = 256;
pub(crate) const PS_REGS: usize = 224;

/// Kinds of the backend's own descriptor heaps.
pub(crate) const H_SRV: usize = 0;
pub(crate) const H_SAMPLER: usize = 1;
pub(crate) const H_RTV: usize = 2;
pub(crate) const H_DSV: usize = 3;
const HEAP_TYPES: [u32; 4] = [DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, DESCRIPTOR_HEAP_TYPE_SAMPLER, DESCRIPTOR_HEAP_TYPE_RTV, DESCRIPTOR_HEAP_TYPE_DSV];
const BLOCK: u32 = 1024;

/// The null views a stage the shader reads and nothing is bound to gets, by what the
/// shader declares: (0, 0, 0, 0), as DXVK's empty slots read.
pub(crate) const NULL_2D: usize = 0;
pub(crate) const NULL_3D: usize = 1;
pub(crate) const NULL_CUBE: usize = 2;
pub(crate) const NULL_2DMS: usize = 3;

/// Descriptors in heaps of the backend's own, which shaders do not see: a texture's
/// views, the samplers. Blocks of 1024 added as needed. What a command takes of a CPU
/// descriptor it takes when it is recorded, but a freed one waits with its resource
/// (`Garbage::View`).
pub(crate) struct CpuHeap {
    ty: u32,
    inc: u32,
    blocks: Vec<(P, usize)>,
    free: Vec<u32>,
    next: u32,
}

impl CpuHeap {
    fn alloc(&mut self, dev: P) -> Option<(usize, u32)> {
        let i = match self.free.pop() {
            Some(i) => i,
            None => {
                if self.next as usize >= self.blocks.len() * BLOCK as usize {
                    let d = DescriptorHeapDesc { ty: self.ty, count: BLOCK, flags: 0, node_mask: 0 };
                    let mut h: P = core::ptr::null_mut();
                    let hr = vcall!(dev, slot::DEV_CREATE_DESCRIPTOR_HEAP, fn(*const DescriptorHeapDesc, *const Guid, *mut P) -> i32, &d, &IID_DESCRIPTOR_HEAP, &mut h);
                    if hr < 0 || h.is_null() {
                        return None;
                    }
                    let mut start = 0usize;
                    vcall!(h, slot::HEAP_GET_CPU_START, fn(*mut usize) -> *mut usize, &mut start);
                    self.blocks.push((h, start));
                }
                self.next += 1;
                self.next - 1
            }
        };
        Some((self.handle(i), i))
    }

    fn handle(&self, i: u32) -> usize {
        let b = &self.blocks[(i / BLOCK) as usize];
        b.1 + ((i % BLOCK) * self.inc) as usize
    }
}

/// What waits until the GPU is done with the frame it was last used in.
pub(crate) enum Garbage {
    Com(P),
    /// A descriptor of the backend's heaps (kind, index): freed with its resource, so
    /// that no handle is used for two textures in one frame (the frame's tables are kept
    /// by handle).
    View(usize, u32),
}

/// What every object needs to reach the device and let go of itself.
pub(crate) struct Shared {
    pub disp: Display8,
    pub dx: Display8Dx12,
    pub dev: P,
    pub queue: P,
    pub root: P,
    /// The compute root signature (null: no compute).
    pub croot: P,
    /// Null views for empty storage slots: a buffer's shader view, a buffer's unordered
    /// access view, a 2D texture's.
    pub null_sto: usize,
    pub null_uav_buf: usize,
    pub null_uav_img: usize,
    pub fif: u64,
    pub features: u32,
    /// What DXVK's `dxvk.conf` makes of every sampler (`Dx12Device8::sampler_*`).
    pub force_aniso: i32,
    pub lod_bias: f32,
    /// Display8's pipeline library (`Dx12Device8::pipeline_library`, contract 1.2), or
    /// null: pipelines are made plainly.
    pub lib: P,
    pub log: Log,
    /// Pipelines loaded from the library, and stored into it (made, then kept).
    pub lib_loaded: Cell<u32>,
    pub lib_stored: Cell<u32>,
    pub heaps: RefCell<[CpuHeap; 4]>,
    pub null_srv: [usize; 4],
    garbage: RefCell<Vec<(u64, Garbage)>>,
    /// The frame recorded last (Display8's index).
    pub now: Cell<u64>,
}

impl Shared {
    /// Counts a pipeline loaded from (or stored into) the library, and logs the counts
    /// when they pass a power of two.
    pub fn lib_count(&self, loaded: bool) {
        let c = if loaded { &self.lib_loaded } else { &self.lib_stored };
        c.set(c.get() + 1);
        if c.get().is_power_of_two() {
            dlog!(self, "gpu: Direct3D 12: pipeline library: {} loaded, {} stored", self.lib_loaded.get(), self.lib_stored.get());
        }
    }

    pub fn retire(&self, g: Garbage) {
        self.garbage.borrow_mut().push((self.now.get(), g));
    }

    fn collect(&self, now: u64) {
        self.now.set(now);
        let done: Vec<Garbage> = {
            let mut g = self.garbage.borrow_mut();
            let mut keep = Vec::with_capacity(g.len());
            let mut done = Vec::new();
            for (f, x) in g.drain(..) {
                if f + self.fif <= now {
                    done.push(x);
                } else {
                    keep.push((f, x));
                }
            }
            *g = keep;
            done
        };
        for x in done {
            match x {
                Garbage::Com(p) => release(p),
                Garbage::View(h, i) => self.heaps.borrow_mut()[h].free.push(i),
            }
        }
    }

    /// A descriptor in the backend's heap of kind `h` (`H_*`): its handle and index.
    pub fn view(&self, h: usize) -> Option<(usize, u32)> {
        self.heaps.borrow_mut()[h].alloc(self.dev)
    }

    /// A descriptor freed once the frames that may have copied it are done.
    pub fn free_view(&self, h: usize, i: u32) {
        self.retire(Garbage::View(h, i));
    }

    /// `src` (a descriptor of heap type `ty`) copied to `dst`.
    pub fn copy_view(&self, dst: usize, src: usize, ty: u32) {
        vcall!(self.dev, slot::DEV_COPY_DESCRIPTORS_SIMPLE, fn(u32, usize, usize, u32) -> (), 1, dst, src, ty);
    }
}

/// A colour or depth attachment of the pass now.
#[derive(Clone)]
pub(crate) struct Att {
    pub tex: Texture,
    pub layer: u32,
}

/// What a pipeline is made for: the targets' formats and samples.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) struct PassKey {
    pub colors: [u32; MAX_COLORS],
    pub ncolors: u32,
    pub depth: u32,
    pub samples: u32,
}

/// The pass now: begun by `Cmd::begin_pass`, kept until another is begun.
#[derive(Clone)]
pub(crate) struct PassState {
    pub colors: Vec<Att>,
    pub depth: Option<Att>,
    pub size: (u32, u32),
    pub key: PassKey,
}

pub(crate) struct State {
    pub sh: Rc<Shared>,
    samplers: Vec<(Sampler, usize)>,
    pub frame: Dx12Frame8,
    pub has_frame: bool,
    pub pass: Option<PassState>,
    /// Moves on with every pass begun: a `Cmd` sets the targets again when it changed.
    pub pass_gen: u32,
    externals: Vec<(usize, Weak<tex::TexInner>)>,
    pub touched: Vec<Texture>,
    /// The frame the tables below belong to.
    tables_frame: u64,
    srv_tables: BTreeMap<[usize; SLOTS], u64>,
    sampler_tables: BTreeMap<usize, u64>,
    sto_tables: BTreeMap<[usize; STORAGE_SLOTS], u64>,
    uav_tables: BTreeMap<[usize; 2 * STORAGE_SLOTS], u64>,
    /// The frame's list and its `ID3D12GraphicsCommandList1` (0: none).
    list1: (usize, usize),
    pub depth_psos: Vec<((bool, u32), P)>,
    pub blit_psos: Vec<(u32, P)>,
    pub dummy_vb: Option<Buffer>,
    pub told: u32,
}

/// The backend's state for one plugin (as `vulkan::Gpu`): made at the first use once
/// Display8 has said the plugins draw on Direct3D 12 and has its device.
pub struct Gpu {
    log: Log,
    st: RefCell<Option<State>>,
    /// Moves on at each `Cmd`'s start and end: a `Cmd` that sees another value sets
    /// everything again (another recorder may have set its own since).
    serial: Cell<u32>,
    depth: Cell<u32>,
    /// Moves on with every submission Display8 makes of the list (a new frame, a
    /// flush): buffers decay to `COMMON` then, so a storage buffer's state recorded in
    /// an earlier one is `COMMON` now.
    pub(crate) epoch: Cell<u64>,
}

/// The root signature every pipeline of the backend shares (the module doc).
fn root_signature(dev: P, serialize: usize, log: Log, compute: bool) -> Option<P> {
    let srv = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SRV, count: SLOTS as u32, base_register: 0, space: 0, offset_in_table: 0 }];
    let sto = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SRV, count: STORAGE_SLOTS as u32, base_register: 16, space: 0, offset_in_table: 0 }];
    let uav = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_UAV, count: 2 * STORAGE_SLOTS as u32, base_register: 0, space: 0, offset_in_table: 0 }];
    let mut samp = Vec::new();
    for i in 0..SLOTS as u32 {
        samp.push(DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SAMPLER, count: 1, base_register: i, space: 0, offset_in_table: 0 });
    }
    let table = |r: *const DescriptorRange| RootParameter { ty: ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, a: 1, b: r as usize, c: 0, visibility: SHADER_VISIBILITY_ALL };
    let mut params = Vec::new();
    if compute {
        for s in 0..SPACES {
            params.push(RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 0, b: s as usize, c: 0, visibility: SHADER_VISIBILITY_ALL });
        }
        params.push(table(srv.as_ptr()));
        for r in &samp {
            params.push(table(r));
        }
        params.push(table(sto.as_ptr()));
        params.push(table(uav.as_ptr()));
        debug_assert!(params.len() as u32 == RPC_UAV + 1);
    } else {
        for s in 0..SPACES {
            params.push(RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 0, b: s as usize, c: 0, visibility: SHADER_VISIBILITY_VERTEX });
        }
        for s in 0..SPACES {
            params.push(RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 1, b: s as usize, c: 0, visibility: SHADER_VISIBILITY_PIXEL });
        }
        params.push(table(srv.as_ptr()));
        for r in &samp {
            params.push(table(r));
        }
        params.push(RootParameter { ty: ROOT_PARAMETER_TYPE_32BIT_CONSTANTS, a: 2, b: 0, c: K_WORDS, visibility: SHADER_VISIBILITY_ALL });
        params.push(table(sto.as_ptr()));
        debug_assert!(params.len() as u32 == RP_STORAGE + 1);
    }
    let desc = RootSignatureDesc {
        count: params.len() as u32,
        params: params.as_ptr(),
        samplers_count: 0,
        samplers: core::ptr::null(),
        flags: if compute { 0 } else { ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT },
    };
    let serialize: unsafe extern "system" fn(*const RootSignatureDesc, u32, *mut P, *mut P) -> i32 = unsafe { core::mem::transmute(serialize) };
    let (mut blob, mut err): (P, P) = (core::ptr::null_mut(), core::ptr::null_mut());
    let hr = unsafe { serialize(&desc, ROOT_SIGNATURE_VERSION_1_0, &mut blob, &mut err) };
    if hr < 0 || blob.is_null() {
        if !err.is_null() {
            let msg = vcall!(err, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const u8,);
            let n = vcall!(err, slot::BLOB_GET_BUFFER_SIZE, fn() -> usize,);
            let s = unsafe { core::slice::from_raw_parts(msg, n) };
            log(format_args!("gpu: Direct3D 12: the root signature: {}", core::str::from_utf8(s).unwrap_or("?")));
        }
        release(err);
        return None;
    }
    release(err);
    let (ptr, len) = (vcall!(blob, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const c_void,), vcall!(blob, slot::BLOB_GET_BUFFER_SIZE, fn() -> usize,));
    let mut root: P = core::ptr::null_mut();
    let hr = vcall!(dev, slot::DEV_CREATE_ROOT_SIGNATURE, fn(u32, *const c_void, usize, *const Guid, *mut P) -> i32, 0, ptr, len, &IID_ROOT_SIGNATURE, &mut root);
    release(blob);
    (hr >= 0 && !root.is_null()).then_some(root)
}

/// How a format of this interface is a resource on Direct3D 12: the resource's format,
/// the shader view's, the target view's (render target or depth), the component
/// mapping the shader view reads through (Direct3D 9's `L8` is (r, r, r, 1), `X8R8G8B8`
/// an alpha of one, a float format's missing channels one), and whether it is depth.
/// A depth format's resource is typeless, so that it can be sampled too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Fmt {
    pub res: u32,
    pub srv: u32,
    pub view: u32,
    pub map: u32,
    pub depth: bool,
}

pub(crate) fn dxgi(f: Format) -> Option<Fmt> {
    let c = |res: u32, map: u32| Fmt { res, srv: res, view: res, map, depth: false };
    let d = |res: u32, srv: u32, view: u32| Fmt { res, srv, view, map: DEFAULT_COMPONENT_MAPPING, depth: true };
    Some(match f {
        Format::Unknown => return None,
        Format::Bgra8 => c(FORMAT_B8G8R8A8_UNORM, DEFAULT_COMPONENT_MAPPING),
        Format::Bgrx8 => c(FORMAT_B8G8R8A8_UNORM, mapping(0, 1, 2, 5)),
        Format::B5G6R5 => c(FORMAT_B5G6R5_UNORM, DEFAULT_COMPONENT_MAPPING),
        Format::Bgr10A2 | Format::Rgb10A2 => c(FORMAT_R10G10B10A2_UNORM, DEFAULT_COMPONENT_MAPPING),
        Format::R8 => c(FORMAT_R8_UNORM, mapping(0, 0, 0, 5)),
        Format::A8 => c(FORMAT_A8_UNORM, DEFAULT_COMPONENT_MAPPING),
        Format::R16F => c(FORMAT_R16_FLOAT, mapping(0, 5, 5, 5)),
        Format::Rg16F => c(FORMAT_R16G16_FLOAT, mapping(0, 1, 5, 5)),
        Format::Rgba16F => c(FORMAT_R16G16B16A16_FLOAT, DEFAULT_COMPONENT_MAPPING),
        Format::R32F => c(FORMAT_R32_FLOAT, mapping(0, 5, 5, 5)),
        Format::Rg32F => c(FORMAT_R32G32_FLOAT, mapping(0, 1, 5, 5)),
        Format::Rgba32F => c(FORMAT_R32G32B32A32_FLOAT, DEFAULT_COMPONENT_MAPPING),
        Format::D16 | Format::D15S1 => d(FORMAT_R16_TYPELESS, FORMAT_R16_UNORM, FORMAT_D16_UNORM),
        Format::D32 => d(FORMAT_R32_TYPELESS, FORMAT_R32_FLOAT, FORMAT_D32_FLOAT),
        Format::D24S8 | Format::D24X8 | Format::D24X4S4 => d(FORMAT_R24G8_TYPELESS, FORMAT_R24_UNORM_X8_TYPELESS, FORMAT_D24_UNORM_S8_UINT),
    })
}

impl Gpu {
    pub const fn new(log: Log) -> Gpu {
        Gpu { log, st: RefCell::new(None), serial: Cell::new(0), depth: Cell::new(0), epoch: Cell::new(0) }
    }

    /// Opens the backend on Display8's device. None: Display8 has no device yet (asked
    /// again later); Some(false): it cannot be opened (logged).
    pub fn init(&self, disp: Display8, dx: Display8Dx12) -> Option<bool> {
        if self.st.borrow().is_some() {
            return Some(true);
        }
        let d = (dx.dx12)();
        if d.is_null() {
            return None;
        }
        // Only the fields within the size Display8 says (gpu/src/display.rs, "Versions").
        let d = unsafe { crate::display::read(d) };
        let dev = d.device as P;
        let Some(root) = root_signature(dev, d.serialize_root_signature, self.log, false) else {
            dlog!(self, "gpu: Direct3D 12: the root signature could not be made");
            return Some(false);
        };
        let croot = root_signature(dev, d.serialize_root_signature, self.log, true).unwrap_or_else(|| {
            dlog!(self, "gpu: Direct3D 12: the compute root signature could not be made: no compute");
            core::ptr::null_mut()
        });
        let heaps = HEAP_TYPES.map(|ty| CpuHeap { ty, inc: d.increments[ty as usize], blocks: Vec::new(), free: Vec::new(), next: 0 });
        let mut sh = Shared {
            disp,
            dx,
            dev,
            queue: d.queue as P,
            root,
            croot,
            null_sto: 0,
            null_uav_buf: 0,
            null_uav_img: 0,
            fif: d.frames_in_flight.max(1) as u64,
            features: d.features,
            force_aniso: d.sampler_anisotropy,
            lod_bias: d.sampler_lod_bias,
            lib: if disp.minor >= 2 { d.pipeline_library as P } else { core::ptr::null_mut() },
            log: self.log,
            lib_loaded: Cell::new(0),
            lib_stored: Cell::new(0),
            heaps: RefCell::new(heaps),
            null_srv: [0; 4],
            garbage: RefCell::new(Vec::new()),
            now: Cell::new(0),
        };
        // The null views: (0, 0, 0, 0) of the dimension the shader reads.
        for (i, (dim, u)) in [
            (SRV_DIMENSION_TEXTURE2D, [0, 1, 0, 0, 0, 0]),
            (SRV_DIMENSION_TEXTURE3D, [0, 1, 0, 0, 0, 0]),
            (SRV_DIMENSION_TEXTURECUBE, [0, 1, 0, 0, 0, 0]),
            (SRV_DIMENSION_TEXTURE2DMS, [0; 6]),
        ]
        .into_iter()
        .enumerate()
        {
            let Some((cpu, _)) = sh.view(H_SRV) else {
                dlog!(self, "gpu: Direct3D 12: no descriptor heap of its own");
                return Some(false);
            };
            let v = SrvDesc { format: FORMAT_B8G8R8A8_UNORM, dimension: dim, mapping: DEFAULT_COMPONENT_MAPPING, _pad: 0, u };
            vcall!(dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), core::ptr::null_mut(), &v, cpu);
            sh.null_srv[i] = cpu;
        }
        // The null storage views: a buffer of words, read and written; a 2D float texture.
        let Some((cpu, _)) = sh.view(H_SRV) else { return Some(false) };
        let v = SrvDesc { format: FORMAT_R32_FLOAT, dimension: SRV_DIMENSION_BUFFER, mapping: DEFAULT_COMPONENT_MAPPING, _pad: 0, u: [0, 0, 1, 0, 0, 0] };
        vcall!(dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), core::ptr::null_mut(), &v, cpu);
        sh.null_sto = cpu;
        let Some((cpu, _)) = sh.view(H_SRV) else { return Some(false) };
        let v = UavDesc { format: FORMAT_R32_FLOAT, dimension: UAV_DIMENSION_BUFFER, u: [0, 0, 1, 0, 0, 0, 0, 0] };
        vcall!(dev, slot::DEV_CREATE_UNORDERED_ACCESS_VIEW, fn(P, P, *const UavDesc, usize) -> (), core::ptr::null_mut(), core::ptr::null_mut(), &v, cpu);
        sh.null_uav_buf = cpu;
        let Some((cpu, _)) = sh.view(H_SRV) else { return Some(false) };
        let v = UavDesc { format: FORMAT_R32_FLOAT, dimension: UAV_DIMENSION_TEXTURE2D, u: [0; 8] };
        vcall!(dev, slot::DEV_CREATE_UNORDERED_ACCESS_VIEW, fn(P, P, *const UavDesc, usize) -> (), core::ptr::null_mut(), core::ptr::null_mut(), &v, cpu);
        sh.null_uav_img = cpu;
        dlog!(
            self,
            "gpu: on Direct3D 12: feature level {:#x}, {} frames in flight, {}",
            d.feature_level,
            sh.fif,
            if d.features & FEATURE_D3D9_CENTRES != 0 { "Direct3D 9's pixel centres" } else { "Direct3D 12's pixel centres" }
        );
        *self.st.borrow_mut() = Some(State {
            sh: Rc::new(sh),
            samplers: Vec::new(),
            frame: Dx12Frame8::default(),
            has_frame: false,
            pass: None,
            pass_gen: 0,
            externals: Vec::new(),
            touched: Vec::new(),
            tables_frame: u64::MAX,
            srv_tables: BTreeMap::new(),
            sampler_tables: BTreeMap::new(),
            sto_tables: BTreeMap::new(),
            uav_tables: BTreeMap::new(),
            list1: (0, 0),
            depth_psos: Vec::new(),
            blit_psos: Vec::new(),
            dummy_vb: None,
            told: 0,
        });
        Some(true)
    }

    pub fn ready(&self) -> bool {
        self.st.borrow().is_some()
    }

    pub(crate) fn sh(&self) -> Option<Rc<Shared>> {
        self.st.borrow().as_ref().map(|s| s.sh.clone())
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> Option<R> {
        let mut st = self.st.borrow_mut();
        Some(f(st.as_mut()?))
    }

    /// Logs `what` once per bit of `bit`.
    pub(crate) fn once(&self, bit: u32, what: core::fmt::Arguments) {
        let first = self.with(|s| {
            let f = s.told & bit == 0;
            s.told |= bit;
            f
        });
        if first == Some(true) {
            (self.log)(what);
        }
    }

    /// Whether Direct3D 9's pixel centres are kept (the comparison mode).
    pub fn d3d9_centres(&self) -> bool {
        self.sh().is_some_and(|s| s.features & FEATURE_D3D9_CENTRES != 0)
    }

    // -- the frame --

    /// The command list of the frame now (asked of Display8 when none is known: it
    /// begins one after a `Present`).
    pub(crate) fn list(&self) -> Option<P> {
        if let Some(Some(l)) = self.with(|s| s.has_frame.then_some(s.frame.command_list as P)) {
            return Some(l);
        }
        self.new_frame()
    }

    /// Asks Display8 for the frame now; when it is another than the last, what the frames
    /// before let go of may be released, and the frame's tables start again.
    pub(crate) fn new_frame(&self) -> Option<P> {
        let sh = self.sh()?;
        let f = (sh.dx.frame)();
        if f.is_null() {
            return None;
        }
        let f = unsafe { crate::display::read(f) };
        if f.command_list == 0 {
            return None;
        }
        sh.collect(f.index);
        self.with(|s| {
            s.frame = f;
            s.has_frame = true;
            if s.tables_frame != f.index {
                s.tables_frame = f.index;
                s.srv_tables.clear();
                s.sampler_tables.clear();
                s.sto_tables.clear();
                s.uav_tables.clear();
                self.epoch.set(self.epoch.get() + 1);
            }
        });
        Some(f.command_list as P)
    }

    /// A `Cmd` begins: the frame now is asked for. False without one.
    pub(crate) fn enter(&self) -> bool {
        self.serial.set(self.serial.get().wrapping_add(1));
        self.depth.set(self.depth.get() + 1);
        self.with(|s| s.has_frame = false);
        self.new_frame().is_some()
    }

    /// A `Cmd` ends: Display8's textures go back in their states, and the next recorder
    /// may be another.
    pub(crate) fn leave(&self) {
        self.restore_externals();
        self.serial.set(self.serial.get().wrapping_add(1));
        let d = self.depth.get().saturating_sub(1);
        self.depth.set(d);
        if d == 0 {
            self.with(|s| s.has_frame = false);
        }
    }

    pub(crate) fn serial(&self) -> u32 {
        self.serial.get()
    }

    pub(crate) fn restore_externals(&self) {
        let list = self.with(|s| core::mem::take(&mut s.touched)).unwrap_or_default();
        if list.is_empty() {
            return;
        }
        let Some(l) = self.list() else { return };
        for t in list {
            if let Some(home) = t.0.home {
                self.transition_on(l, &t, home);
            }
        }
    }

    /// Puts `t` (made) in state `to`, the whole of it.
    pub(crate) fn transition(&self, t: &Texture, to: u32) {
        if t.0.state.get() == to {
            return;
        }
        if let Some(l) = self.list() {
            self.transition_on(l, t, to);
        }
    }

    fn transition_on(&self, l: P, t: &Texture, to: u32) {
        let from = t.0.state.get();
        if from == to {
            return;
        }
        let res = t.res();
        if res.is_null() {
            return;
        }
        barrier(l, res, ALL_SUBRESOURCES, from, to);
        t.0.state.set(to);
        if t.0.home.is_some() {
            self.with(|s| {
                if !s.touched.iter().any(|x| x.same(t)) {
                    s.touched.push(t.clone());
                }
            });
        }
    }

    /// The frame's list as an `ID3D12GraphicsCommandList1` (for a resolve of a region),
    /// or None.
    pub(crate) fn list1(&self) -> Option<P> {
        let l = self.list()? as usize;
        if let Some((k, v)) = self.with(|s| s.list1) {
            if k == l {
                return (v != 0).then_some(v as P);
            }
        }
        let mut p: P = core::ptr::null_mut();
        let hr = vcall!(l as P, slot::QUERY_INTERFACE, fn(*const Guid, *mut P) -> i32, &IID_GRAPHICS_COMMAND_LIST1, &mut p);
        let v = if hr >= 0 && !p.is_null() {
            // Display8 holds the list; this is the same object by another name.
            release(p);
            p as usize
        } else {
            0
        };
        self.with(|s| s.list1 = (l, v));
        (v != 0).then_some(v as P)
    }

    // -- descriptors --

    /// The sampler's descriptor in the backend's heap, made once per description as
    /// DXVK would make it under the game's `dxvk.conf` (`vulkan::Gpu::sampler`).
    pub(crate) fn sampler(&self, d: &Sampler) -> usize {
        if let Some(Some(s)) = self.with(|s| s.samplers.iter().find(|x| x.0 == *d).map(|x| x.1)) {
            return s;
        }
        let Some(sh) = self.sh() else { return 0 };
        let Some((cpu, _)) = sh.view(H_SAMPLER) else { return 0 };
        let forced = sh.force_aniso >= 0;
        let wants = |x: Filter| x == Filter::Anisotropic || (forced && x == Filter::Linear);
        let aniso = wants(d.min) || wants(d.mag);
        let most = if forced { sh.force_aniso as u32 } else { d.anisotropy };
        let lin = |f: Filter| (f != Filter::Point) as u32;
        let filter = if aniso { FILTER_ANISOTROPIC } else { (lin(d.min) << 4) | (lin(d.mag) << 2) | (d.mip == MipFilter::Linear) as u32 };
        let a = |a: Address| match a {
            Address::Wrap => TEXTURE_ADDRESS_MODE_WRAP,
            Address::Mirror => TEXTURE_ADDRESS_MODE_MIRROR,
            Address::Clamp => TEXTURE_ADDRESS_MODE_CLAMP,
        };
        let bias = if d.mip == MipFilter::None { 0.0 } else { sh.lod_bias.clamp(-16.0, 15.99) };
        let sd = SamplerDesc {
            filter,
            address_u: a(d.address_u),
            address_v: a(d.address_v),
            address_w: a(d.address_w),
            mip_lod_bias: bias,
            max_anisotropy: if aniso { most.clamp(1, 16) } else { 1 },
            comparison: COMPARISON_FUNC_NEVER,
            border: [0.0; 4],
            min_lod: 0.0,
            max_lod: if d.mip == MipFilter::None { 0.0 } else { FLOAT32_MAX },
        };
        if d.srgb {
            self.once(1 << 20, format_args!("gpu: Direct3D 12: an sRGB sampler is sampled as linear (no sRGB views yet)"));
        }
        vcall!(sh.dev, slot::DEV_CREATE_SAMPLER, fn(*const SamplerDesc, usize) -> (), &sd, cpu);
        self.with(|st| st.samplers.push((*d, cpu)));
        cpu
    }

    /// The frame's table of these sixteen views (made once a frame per set).
    pub(crate) fn srv_table(&self, views: &[usize; SLOTS]) -> Option<u64> {
        if let Some(Some(g)) = self.with(|s| s.srv_tables.get(views).copied()) {
            return Some(g);
        }
        let sh = self.sh()?;
        let mut r = crate::display::Dx12Descriptors8::default();
        if !(sh.dx.descriptors)(HEAP_CBV_SRV_UAV, SLOTS as u32, &mut r) {
            self.once(1 << 21, format_args!("gpu: Direct3D 12: the frame's descriptors are spent; draws are dropped"));
            return None;
        }
        let one = [1u32; SLOTS];
        let dst = [r.cpu];
        let n = [SLOTS as u32];
        vcall!(sh.dev, 23, fn(u32, *const usize, *const u32, u32, *const usize, *const u32, u32) -> (),
            1, dst.as_ptr(), n.as_ptr(), SLOTS as u32, views.as_ptr(), one.as_ptr(), DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
        self.with(|s| s.srv_tables.insert(*views, r.gpu));
        Some(r.gpu)
    }

    /// A range of the frame's shader-visible heap holding copies of `views`: its GPU
    /// handle.
    fn copy_table(&self, views: &[usize]) -> Option<u64> {
        let sh = self.sh()?;
        let n = views.len() as u32;
        let mut r = crate::display::Dx12Descriptors8::default();
        if !(sh.dx.descriptors)(HEAP_CBV_SRV_UAV, n, &mut r) {
            self.once(1 << 21, format_args!("gpu: Direct3D 12: the frame's descriptors are spent; draws are dropped"));
            return None;
        }
        let one = alloc::vec![1u32; views.len()];
        let dst = [r.cpu];
        let nn = [n];
        vcall!(sh.dev, 23, fn(u32, *const usize, *const u32, u32, *const usize, *const u32, u32) -> (),
            1, dst.as_ptr(), nn.as_ptr(), n, views.as_ptr(), one.as_ptr(), DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
        Some(r.gpu)
    }

    /// The frame's table of the eight read-only storage views (made once a frame per set).
    pub(crate) fn sto_table(&self, views: &[usize; STORAGE_SLOTS]) -> Option<u64> {
        if let Some(Some(g)) = self.with(|s| s.sto_tables.get(views).copied()) {
            return Some(g);
        }
        let g = self.copy_table(views)?;
        self.with(|s| s.sto_tables.insert(*views, g));
        Some(g)
    }

    /// The frame's table of the sixteen unordered access views (u0..u15).
    pub(crate) fn uav_table(&self, views: &[usize; 2 * STORAGE_SLOTS]) -> Option<u64> {
        if let Some(Some(g)) = self.with(|s| s.uav_tables.get(views).copied()) {
            return Some(g);
        }
        let g = self.copy_table(views)?;
        self.with(|s| s.uav_tables.insert(*views, g));
        Some(g)
    }

    /// The frame's copy of a sampler (made once a frame).
    pub(crate) fn sampler_table(&self, cpu: usize) -> Option<u64> {
        if let Some(Some(g)) = self.with(|s| s.sampler_tables.get(&cpu).copied()) {
            return Some(g);
        }
        let sh = self.sh()?;
        let mut r = crate::display::Dx12Descriptors8::default();
        if !(sh.dx.descriptors)(HEAP_SAMPLER, 1, &mut r) {
            self.once(1 << 22, format_args!("gpu: Direct3D 12: the frame's samplers are spent; draws are dropped"));
            return None;
        }
        sh.copy_view(r.cpu, cpu, DESCRIPTOR_HEAP_TYPE_SAMPLER);
        self.with(|s| s.sampler_tables.insert(cpu, r.gpu));
        Some(r.gpu)
    }

    // -- Display8's objects --

    /// One of Display8's textures, wrapped (one wrapper per resource while it lives).
    pub(crate) fn external(&self, i: &Dx12Image8, format: Format, depth: bool) -> Option<Texture> {
        if i.resource == 0 {
            return None;
        }
        let found = self.with(|s| {
            s.externals.retain(|e| e.1.strong_count() > 0);
            s.externals.iter().find(|e| e.0 == i.resource).and_then(|e| e.1.upgrade())
        })?;
        if let Some(t) = found {
            if t.desc.width == i.width && t.desc.height == i.height {
                return Some(Texture(t));
            }
        }
        let desc = TextureDesc {
            width: i.width,
            height: i.height,
            levels: i.levels.max(1),
            format,
            samples: i.samples.max(1),
            usage: if depth { Usage::DEPTH } else { Usage::RENDER_TARGET | Usage::SAMPLED | Usage::COPY_SRC },
            kind: TextureKind::D2,
            depth: 1,
        };
        let t = tex::Texture::external(desc, i, depth);
        self.with(|s| s.externals.push((i.resource, Rc::downgrade(&t.0))));
        Some(t)
    }

    pub fn engine_target(&self) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = Dx12Image8::default();
        if !(sh.dx.target)(crate::display::TARGET_COLOR, &mut i) {
            return None;
        }
        // The engine's back buffer: X8R8G8B8, as it asked for.
        self.external(&i, Format::Bgrx8, false)
    }

    pub fn engine_depth(&self) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = Dx12Image8::default();
        if !(sh.dx.target)(crate::display::TARGET_DEPTH, &mut i) {
            return None;
        }
        let f = match i.format {
            FORMAT_D32_FLOAT => Format::D32,
            FORMAT_D24_UNORM_S8_UINT => Format::D24S8,
            _ => Format::D16,
        };
        self.external(&i, f, true)
    }

    /// An engine texture (an `IDirect3DTexture8` of Display8's) as Display8's mirror.
    pub fn engine_texture(&self, tex8: *mut c_void) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = Dx12Image8::default();
        if tex8.is_null() || !(sh.dx.mirror)(tex8, &mut i) {
            return None;
        }
        self.external(&i, Format::Bgra8, false)
    }

    // -- making things --

    pub fn texture(&self, desc: TextureDesc, contents: Contents) -> Texture {
        tex::Texture::new(desc, contents)
    }

    pub fn buffer(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        tex::Buffer::new(desc, data.into(), false)
    }

    pub fn buffer_transient(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        let b = tex::Buffer::new(desc, data.into(), true);
        b.realize(self);
        b
    }

    pub fn timer(&self, n: usize) -> Timer {
        tex::Timer::new(n)
    }

    pub fn pipeline(&self, desc: PipelineDesc) -> Pipeline {
        pipe::Pipeline::new(desc)
    }

    pub fn storage(&self, desc: StorageDesc) -> Storage {
        tex::Storage::new(desc)
    }

    pub fn compute_pipeline(&self, desc: ComputeDesc) -> ComputePipeline {
        pipe::ComputePipeline::new(desc)
    }

    /// Compute and storage: with the compute root signature made.
    pub fn compute(&self) -> bool {
        self.sh().is_some_and(|s| !s.croot.is_null())
    }

    pub fn begin<'a>(&'a self, frame: &'a mut crate::Frame) -> Option<Cmd<'a>> {
        Cmd::new(self, frame)
    }

    /// Submits what is recorded and waits for it (`display8v1_dx12_flush`): for a
    /// read-back. The frame goes on in the same list, which binds nothing any more.
    pub(crate) fn flush(&self) -> bool {
        let Some(sh) = self.sh() else { return false };
        self.serial.set(self.serial.get().wrapping_add(1));
        self.epoch.set(self.epoch.get() + 1);
        match sh.dx.flush {
            Some(f) => f(),
            None => {
                self.once(1 << 23, format_args!("gpu: Direct3D 12: this Display8 cannot flush a frame: nothing is read back"));
                false
            }
        }
    }
}

/// A transform the engine set (Display8's record), as Direct3D sets it (row-major).
pub(crate) fn engine_transform(g: &Gpu, state: u32) -> Option<[[f32; 4]; 4]> {
    let sh = g.sh()?;
    let mut m = [0f32; 16];
    if !(sh.disp.engine_transform)(state, &mut m) {
        return None;
    }
    let mut out = [[0f32; 4]; 4];
    for (i, r) in out.iter_mut().enumerate() {
        r.copy_from_slice(&m[i * 4..i * 4 + 4]);
    }
    Some(out)
}

/// The engine's viewport (Display8's record).
pub(crate) fn engine_viewport(g: &Gpu) -> Option<Viewport> {
    let sh = g.sh()?;
    let mut v = [0u32; 6];
    if !(sh.disp.engine_viewport)(&mut v) {
        return None;
    }
    Some(Viewport { x: v[0], y: v[1], w: v[2], h: v[3], min_z: f32::from_bits(v[4]), max_z: f32::from_bits(v[5]) })
}
