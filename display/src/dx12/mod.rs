//! Stub mode on Direct3D 12 (`Backend=d3d12`): Display8's own device, swap chain and
//! frame, the same as `vk/` on Vulkan, which render3d's and the UI's `gpu` record into
//! through the exports of `gpu/src/display.rs` (the contract; README.md, "Direct3D 12").
//!
//! - **`d3d12.dll` and `dxgi.dll`** are opened with `LoadLibraryA` and reached through
//!   `GetProcAddress` (`D3D12CreateDevice`, `D3D12SerializeRootSignature`,
//!   `D3D12GetDebugInterface`, `CreateDXGIFactory1`); every interface is called by slot
//!   (`raw.rs`). The device is made at feature level 11_0 on the default adapter, with
//!   one direct queue.
//! - **A flip-model swap chain** (`FLIP_DISCARD`, two B8G8R8A8 buffers) on the game's
//!   window. DXGI's own Alt+Enter and window changes are switched off: the window is
//!   the stub's, borderless at the desktop's size.
//! - **The engine's back buffer and depth** are resources of ours (B8G8R8A8_UNORM and
//!   D16_UNORM), in `RENDER_TARGET` and `DEPTH_WRITE` between recorders. The frame's
//!   last pass draws the colour one into the swap chain's buffer through the engine's
//!   gamma ramp (`shaders/present.hlsl`, DXIL), so the swap chain is no one else's
//!   target.
//! - **Descriptors**: two shader-visible heaps (CBV/SRV/UAV and samplers), set on the
//!   frame's command list when it begins, each split into one range per frame slot that
//!   the recorders take descriptors from (`display8v1_dx12_descriptors`); the views of
//!   Display8's own resources (targets, mirrors, the ramp) live in heaps that are not
//!   shader-visible and are copied into a frame's range to be bound.
//! - **Two frames in flight**, as on Vulkan: each slot has a command allocator and list,
//!   the fence value its submission signals, an upload ring (upload-heap buffers,
//!   mapped for good; chunks added when a frame needs more) and the callbacks deferred
//!   until its fence value has passed. A frame is begun at the first call that needs it
//!   after a `Present`, and ended, submitted and presented at the next.
pub mod cache;
pub mod exports;
pub mod mirror;
pub mod raw;

use crate::log;
use crate::rt::win::{CloseHandle, CreateEventA, GetProcAddress, LoadLibraryA, WaitForSingleObject, HANDLE};
use crate::shaders::present_dxil::{PS_GAMMA_DXIL, VS_PRESENT_DXIL};
use alloc::vec::Vec;
use core::ffi::c_void;
use gpu::display::{Dx12Alloc8, Dx12Descriptors8, Dx12Device8, Dx12Frame8, Dx12Image8};
use raw::*;

const SLOTS: usize = 2;
const CHUNK: u64 = 4 << 20;
/// Shader-visible descriptors per frame slot: CBV/SRV/UAV, and samplers (2048 in all
/// is Direct3D 12's limit for a sampler heap).
const FRAME_VIEWS: u32 = 32768;
const FRAME_SAMPLERS: u32 = 1024;
/// Views of Display8's own resources (targets, ramp, mirrors), not shader-visible.
const OWN_VIEWS: u32 = 1024;
const SWAP_BUFFERS: u32 = 2;

pub struct Img {
    pub res: P,
    pub format: u32,
    pub w: u32,
    pub h: u32,
    pub levels: u32,
    /// The state it is in between recorders.
    pub state: u32,
    pub rtv: usize,
    pub dsv: usize,
    /// Its view in the heap of Display8's own (not shader-visible), or 0.
    pub srv: usize,
    srv_index: u32,
}

struct Chunk {
    res: P,
    gpu: u64,
    ptr: *mut u8,
    size: u64,
}

struct Slot {
    alloc: P,
    list: P,
    /// The fence value this slot's last submission signals.
    done: u64,
    chunks: Vec<Chunk>,
    at: (usize, u64),
    views_at: u32,
    samplers_at: u32,
    deferred: Vec<(extern "C" fn(*mut c_void), *mut c_void)>,
}

pub struct Dx {
    pub dev: P,
    queue: P,
    swap: P,
    sc_buffers: Vec<P>,
    sc_w: u32,
    sc_h: u32,
    sync_interval: u32,
    rtv_heap: P,
    rtv_start: usize,
    dsv_heap: P,
    own_heap: P,
    own_start: usize,
    own_free: Vec<u32>,
    own_next: u32,
    views_heap: P,
    views_cpu: usize,
    views_gpu: u64,
    samplers_heap: P,
    samplers_cpu: usize,
    samplers_gpu: u64,
    pub inc: [u32; 4],
    fence: P,
    fence_event: HANDLE,
    fence_value: u64,
    pub color: Img,
    pub depth: Img,
    ramp: Img,
    ramp_values: Option<[u16; 768]>,
    stale: bool,
    slots: Vec<Slot>,
    slot: usize,
    pub index: u64,
    recording: bool,
    first: bool,
    root: P,
    pso: P,
    /// The pipeline library (null: none), its file (kept: the library may point into
    /// it), the adapter's header words and the size last written.
    lib: P,
    lib_file: Vec<u8>,
    lib_id: [u32; 8],
    lib_saved: usize,
    pub exported: Dx12Device8,
    pub frame_out: Dx12Frame8,
    lost: bool,
}

static mut DX: Option<Dx> = None;

#[allow(static_mut_refs)]
pub fn get() -> Option<&'static mut Dx> {
    unsafe { DX.as_mut() }
}

fn proc(m: HANDLE, name: &core::ffi::CStr) -> usize {
    unsafe { GetProcAddress(m, name.as_ptr() as *const u8) }
}

/// `riid`, `out`: makes something and checks it came back.
fn made(hr: i32, out: P, what: &'static str) -> Result<P, &'static str> {
    if hr < 0 || out.is_null() {
        log!("d3d12: {} failed: {:#x}", what, hr as u32);
        return Err(what);
    }
    Ok(out)
}

impl Dx {
    /// Opens Direct3D 12 on `hwnd`, `w` x `h`; `interval` is the engine's presentation
    /// interval (0 and 1: wait for the vertical blank; IMMEDIATE: don't).
    pub fn start(hwnd: *mut c_void, w: u32, h: u32, interval: u32, validate: bool) -> Result<(), &'static str> {
        let d3d12 = unsafe { LoadLibraryA(c"d3d12.dll".as_ptr() as *const u8) };
        if d3d12.is_null() {
            return Err("no d3d12.dll");
        }
        let dxgi = unsafe { LoadLibraryA(c"dxgi.dll".as_ptr() as *const u8) };
        if dxgi.is_null() {
            return Err("no dxgi.dll");
        }
        let create_device = proc(d3d12, c"D3D12CreateDevice");
        let serialize = proc(d3d12, c"D3D12SerializeRootSignature");
        let create_factory = proc(dxgi, c"CreateDXGIFactory1");
        if create_device == 0 || serialize == 0 {
            return Err("d3d12.dll has no D3D12CreateDevice or D3D12SerializeRootSignature");
        }
        if create_factory == 0 {
            return Err("dxgi.dll has no CreateDXGIFactory1");
        }
        if validate {
            match proc(d3d12, c"D3D12GetDebugInterface") {
                0 => log!("d3d12: Validate=1, but d3d12.dll has no D3D12GetDebugInterface"),
                f => {
                    let get: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(f) };
                    let mut dbg: P = core::ptr::null_mut();
                    if unsafe { get(&IID_DEBUG, &mut dbg) } >= 0 && !dbg.is_null() {
                        vcall!(dbg, slot::DEBUG_ENABLE_DEBUG_LAYER, fn() -> (),);
                        release(dbg);
                        log!("d3d12: the debug layer is on (Validate=1)");
                    } else {
                        log!("d3d12: Validate=1, but no debug layer");
                    }
                }
            }
        }
        let create_device: unsafe extern "system" fn(P, u32, *const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(create_device) };
        let mut dev: P = core::ptr::null_mut();
        let hr = unsafe { create_device(core::ptr::null_mut(), FEATURE_LEVEL_11_0, &IID_DEVICE, &mut dev) };
        let dev = made(hr, dev, "D3D12CreateDevice at feature level 11_0")?;

        // From here what is made is released again if the start fails, so that another
        // backend can have the window (Backend=auto; README.md, "Choosing a backend").
        let qd = CommandQueueDesc { ty: COMMAND_LIST_TYPE_DIRECT, ..Default::default() };
        let mut queue: P = core::ptr::null_mut();
        let hr = vcall!(dev, slot::DEV_CREATE_COMMAND_QUEUE, fn(*const CommandQueueDesc, *const crate::com::Guid, *mut P) -> i32, &qd, &IID_COMMAND_QUEUE, &mut queue);
        let queue = made(hr, queue, "CreateCommandQueue").inspect_err(|_| release(dev))?;

        let inc = [
            DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            DESCRIPTOR_HEAP_TYPE_SAMPLER,
            DESCRIPTOR_HEAP_TYPE_RTV,
            DESCRIPTOR_HEAP_TYPE_DSV,
        ]
        .map(|t| vcall!(dev, slot::DEV_GET_DESCRIPTOR_HANDLE_INCREMENT_SIZE, fn(u32) -> u32, t));

        let mut fence: P = core::ptr::null_mut();
        let hr = vcall!(dev, slot::DEV_CREATE_FENCE, fn(u64, u32, *const crate::com::Guid, *mut P) -> i32, 0, 0, &IID_FENCE, &mut fence);
        let fence = made(hr, fence, "CreateFence").inspect_err(|_| {
            release(queue);
            release(dev);
        })?;
        let fence_event = unsafe { CreateEventA(core::ptr::null_mut(), 0, 0, core::ptr::null()) };
        if fence_event.is_null() {
            release(fence);
            release(queue);
            release(dev);
            return Err("CreateEventA failed");
        }

        let samplers = crate::dxvk_samplers();
        log!("d3d12: the plugins' samplers as dxvk.conf has DXVK make them: anisotropy {}, LOD bias {}", samplers.0, samplers.1);
        if crate::d3d9_centres() {
            log!("d3d12: PixelCentres=d3d9: the plugins draw with Direct3D 9's pixel centres (for comparing)");
        }
        let mut me = Dx {
            dev,
            queue,
            swap: core::ptr::null_mut(),
            sc_buffers: Vec::new(),
            sc_w: w,
            sc_h: h,
            sync_interval: if interval & 0x8000_0000 != 0 { 0 } else { 1 },
            rtv_heap: core::ptr::null_mut(),
            rtv_start: 0,
            dsv_heap: core::ptr::null_mut(),
            own_heap: core::ptr::null_mut(),
            own_start: 0,
            own_free: Vec::new(),
            own_next: 0,
            views_heap: core::ptr::null_mut(),
            views_cpu: 0,
            views_gpu: 0,
            samplers_heap: core::ptr::null_mut(),
            samplers_cpu: 0,
            samplers_gpu: 0,
            inc,
            fence,
            fence_event,
            fence_value: 0,
            color: Img::null(),
            depth: Img::null(),
            ramp: Img::null(),
            ramp_values: None,
            stale: true,
            slots: Vec::new(),
            slot: 0,
            index: 0,
            recording: false,
            first: true,
            root: core::ptr::null_mut(),
            pso: core::ptr::null_mut(),
            lib: core::ptr::null_mut(),
            lib_file: Vec::new(),
            lib_id: [0; 8],
            lib_saved: 0,
            exported: Dx12Device8 {
                size: core::mem::size_of::<Dx12Device8>() as u32,
                device: dev as usize,
                queue: queue as usize,
                serialize_root_signature: serialize,
                feature_level: FEATURE_LEVEL_11_0,
                frames_in_flight: SLOTS as u32,
                increments: inc,
                heaps: [0, 0],
                uniform_align: CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT,
                features: gpu::display::FEATURE_ANISOTROPY | if crate::d3d9_centres() { gpu::display::FEATURE_D3D9_CENTRES } else { 0 },
                sampler_anisotropy: samplers.0,
                sampler_lod_bias: samplers.1,
                pipeline_library: 0,
            },
            frame_out: Dx12Frame8::default(),
            lost: false,
        };
        me.make_library(create_factory);
        if let Err(e) = me.build(create_factory, hwnd, serialize) {
            me.destroy();
            return Err(e);
        }
        log!(
            "d3d12: device {:#x} at feature level 11_0; swap chain {}x{} flip-discard B8G8R8A8, sync interval {}; targets {}x{} B8G8R8A8 + D16",
            dev as usize, me.sc_w, me.sc_h, me.sync_interval, w, h
        );
        unsafe { DX = Some(me) };
        Ok(())
    }

    /// The pipeline library, from the file if it is this device's.
    fn make_library(&mut self, create_factory: usize) {
        self.lib_id = cache::identity(create_factory);
        let file = cache::load(&self.lib_id);
        let mut lib = cache::create(self.dev, file.get(64..).unwrap_or(&[]));
        if lib.is_null() && file.len() > 64 {
            lib = cache::create(self.dev, &[]);
            self.lib_file = Vec::new();
        } else {
            self.lib_file = file;
        }
        self.lib = lib;
        self.exported.pipeline_library = lib as usize;
        if !lib.is_null() {
            self.lib_saved = cache::size(lib);
        }
    }

    /// The heaps, swap chain, targets, pipeline and frame slots of a device just made.
    fn build(&mut self, create_factory: usize, hwnd: *mut c_void, serialize: usize) -> Result<(), &'static str> {
        let me = self;
        let (w, h, dev, inc) = (me.sc_w, me.sc_h, me.dev, me.inc);
        me.make_heaps()?;
        me.make_swapchain(create_factory, hwnd)?;
        me.color = me.texture(w, h, 1, FORMAT_B8G8R8A8_UNORM, RESOURCE_FLAG_ALLOW_RENDER_TARGET, STATE_RENDER_TARGET, None)?;
        me.depth = me.texture(w, h, 1, FORMAT_D16_UNORM, RESOURCE_FLAG_ALLOW_DEPTH_STENCIL, STATE_DEPTH_WRITE, None)?;
        me.ramp = me.texture(256, 1, 1, FORMAT_B8G8R8A8_UNORM, 0, STATE_COPY_DEST, Some(DEFAULT_COMPONENT_MAPPING))?;
        let (rtv, dsv) = (me.rtv_start + (SWAP_BUFFERS * inc[2]) as usize, me.dsv_cpu());
        vcall!(dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const c_void, usize) -> (), me.color.res, core::ptr::null(), rtv);
        vcall!(dev, slot::DEV_CREATE_DEPTH_STENCIL_VIEW, fn(P, *const c_void, usize) -> (), me.depth.res, core::ptr::null(), dsv);
        me.color.rtv = rtv;
        me.depth.dsv = dsv;
        let (srv, index) = me.own_view().ok_or("no view for the colour target")?;
        me.view(me.color.res, FORMAT_B8G8R8A8_UNORM, 1, DEFAULT_COMPONENT_MAPPING, srv);
        me.color.srv = srv;
        me.color.srv_index = index;
        me.make_pipeline(serialize)?;
        for _ in 0..SLOTS {
            let s = me.make_slot()?;
            me.slots.push(s);
        }
        // The bench's deliberate failure (FailBackend=d3d12), after everything is made,
        // so that the teardown is what the next backend starts after.
        if crate::fail_backend(crate::stub::Backend::D3d12) {
            return Err("FailBackend=d3d12 (the test bench's deliberate failure)");
        }
        Ok(())
    }

    /// Releases everything `start` made, when it fails: the swap chain first, so that the
    /// window is free again for the next backend. Nothing has been submitted yet.
    fn destroy(mut self) {
        for s in self.slots.drain(..) {
            for c in s.chunks {
                release(c.res);
            }
            release(s.list);
            release(s.alloc);
        }
        for b in self.sc_buffers.drain(..) {
            release(b);
        }
        release(self.swap);
        release(self.pso);
        release(self.lib);
        release(self.root);
        for img in [&self.color, &self.depth, &self.ramp] {
            release(img.res);
        }
        for heap in [self.rtv_heap, self.dsv_heap, self.own_heap, self.views_heap, self.samplers_heap] {
            release(heap);
        }
        release(self.fence);
        unsafe { CloseHandle(self.fence_event) };
        release(self.queue);
        release(self.dev);
        log!("d3d12: what was made is released again");
    }

    fn heap(&self, ty: u32, count: u32, flags: u32) -> Result<P, &'static str> {
        let d = DescriptorHeapDesc { ty, count, flags, node_mask: 0 };
        let mut h: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_DESCRIPTOR_HEAP, fn(*const DescriptorHeapDesc, *const crate::com::Guid, *mut P) -> i32, &d, &IID_DESCRIPTOR_HEAP, &mut h);
        made(hr, h, "CreateDescriptorHeap")
    }

    fn make_heaps(&mut self) -> Result<(), &'static str> {
        self.rtv_heap = self.heap(DESCRIPTOR_HEAP_TYPE_RTV, SWAP_BUFFERS + 1, 0)?;
        self.dsv_heap = self.heap(DESCRIPTOR_HEAP_TYPE_DSV, 1, 0)?;
        self.own_heap = self.heap(DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, OWN_VIEWS, 0)?;
        self.views_heap = self.heap(DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, FRAME_VIEWS * SLOTS as u32, DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)?;
        self.samplers_heap = self.heap(DESCRIPTOR_HEAP_TYPE_SAMPLER, FRAME_SAMPLERS * SLOTS as u32, DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)?;
        self.rtv_start = cpu_start(self.rtv_heap);
        self.own_start = cpu_start(self.own_heap);
        (self.views_cpu, self.views_gpu) = (cpu_start(self.views_heap), gpu_start(self.views_heap));
        (self.samplers_cpu, self.samplers_gpu) = (cpu_start(self.samplers_heap), gpu_start(self.samplers_heap));
        self.exported.heaps = [self.views_heap as usize, self.samplers_heap as usize];
        Ok(())
    }

    fn dsv_cpu(&self) -> usize {
        cpu_start(self.dsv_heap)
    }

    fn make_swapchain(&mut self, create_factory: usize, hwnd: *mut c_void) -> Result<(), &'static str> {
        let create: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(create_factory) };
        let mut factory: P = core::ptr::null_mut();
        let hr = unsafe { create(&IID_DXGI_FACTORY2, &mut factory) };
        let factory = made(hr, factory, "CreateDXGIFactory1 (IDXGIFactory2)")?;
        let desc = SwapChainDesc1 {
            width: self.sc_w,
            height: self.sc_h,
            format: FORMAT_B8G8R8A8_UNORM,
            sample: SampleDesc { count: 1, quality: 0 },
            usage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            buffers: SWAP_BUFFERS,
            scaling: DXGI_SCALING_STRETCH,
            swap_effect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            ..Default::default()
        };
        let mut sc1: P = core::ptr::null_mut();
        let hr = vcall!(factory, slot::FACTORY_CREATE_SWAP_CHAIN_FOR_HWND, fn(P, *mut c_void, *const SwapChainDesc1, *const c_void, P, *mut P) -> i32,
            self.queue, hwnd, &desc, core::ptr::null(), core::ptr::null_mut(), &mut sc1);
        let sc1 = match made(hr, sc1, "CreateSwapChainForHwnd") {
            Ok(s) => s,
            Err(e) => {
                release(factory);
                return Err(e);
            }
        };
        vcall!(factory, slot::FACTORY_MAKE_WINDOW_ASSOCIATION, fn(*mut c_void, u32) -> i32, hwnd, DXGI_MWA_NO_WINDOW_CHANGES | DXGI_MWA_NO_ALT_ENTER);
        release(factory);
        let mut sc3: P = core::ptr::null_mut();
        let hr = vcall!(sc1, slot::QUERY_INTERFACE, fn(*const crate::com::Guid, *mut P) -> i32, &IID_DXGI_SWAP_CHAIN3, &mut sc3);
        release(sc1);
        self.swap = made(hr, sc3, "IDXGISwapChain3")?;
        self.swap_buffers()
    }

    /// The swap chain's buffers and their render-target views.
    fn swap_buffers(&mut self) -> Result<(), &'static str> {
        for b in self.sc_buffers.drain(..) {
            release(b);
        }
        for i in 0..SWAP_BUFFERS {
            let mut b: P = core::ptr::null_mut();
            let hr = vcall!(self.swap, slot::SWAP_GET_BUFFER, fn(u32, *const crate::com::Guid, *mut P) -> i32, i, &IID_RESOURCE, &mut b);
            let b = made(hr, b, "IDXGISwapChain::GetBuffer")?;
            let rtv = self.rtv_start + (i * self.inc[2]) as usize;
            vcall!(self.dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const c_void, usize) -> (), b, core::ptr::null(), rtv);
            self.sc_buffers.push(b);
        }
        Ok(())
    }

    /// A committed resource in the default heap: a 2D texture, colour or depth target.
    /// With `srv`, also a view in the heap of Display8's own with that component mapping.
    pub fn texture(&mut self, w: u32, h: u32, levels: u32, format: u32, flags: u32, state: u32, srv: Option<u32>) -> Result<Img, &'static str> {
        let hp = HeapProperties { ty: HEAP_TYPE_DEFAULT, ..Default::default() };
        let rd = ResourceDesc {
            dimension: RESOURCE_DIMENSION_TEXTURE2D,
            alignment: 0,
            width: w as u64,
            height: h,
            depth_or_array_size: 1,
            mip_levels: levels as u16,
            format,
            sample: SampleDesc { count: 1, quality: 0 },
            layout: TEXTURE_LAYOUT_UNKNOWN,
            flags,
        };
        let clear = ClearValue { format, color: if format == FORMAT_D16_UNORM { [1.0, 0.0, 0.0, 0.0] } else { [0.0; 4] } };
        let cv: *const ClearValue = if flags & (RESOURCE_FLAG_ALLOW_RENDER_TARGET | RESOURCE_FLAG_ALLOW_DEPTH_STENCIL) != 0 { &clear } else { core::ptr::null() };
        let mut res: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_COMMITTED_RESOURCE, fn(*const HeapProperties, u32, *const ResourceDesc, u32, *const ClearValue, *const crate::com::Guid, *mut P) -> i32,
            &hp, 0, &rd, state, cv, &IID_RESOURCE, &mut res);
        let res = made(hr, res, "CreateCommittedResource (texture)")?;
        let mut img = Img { res, format, w, h, levels, state, rtv: 0, dsv: 0, srv: 0, srv_index: u32::MAX };
        if let Some(mapping) = srv {
            let Some((cpu, index)) = self.own_view() else {
                release(res);
                return Err("no view left in Display8's heap");
            };
            self.view(res, format, levels, mapping, cpu);
            img.srv = cpu;
            img.srv_index = index;
        }
        Ok(img)
    }

    fn view(&self, res: P, format: u32, levels: u32, mapping: u32, cpu: usize) {
        let d = SrvDesc { format, dimension: SRV_DIMENSION_TEXTURE2D, mapping, most_detailed_mip: 0, mip_levels: levels, ..Default::default() };
        vcall!(self.dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), res, &d, cpu);
    }

    fn own_view(&mut self) -> Option<(usize, u32)> {
        let i = match self.own_free.pop() {
            Some(i) => i,
            None if self.own_next < OWN_VIEWS => {
                self.own_next += 1;
                self.own_next - 1
            }
            None => return None,
        };
        Some((self.own_start + (i * self.inc[0]) as usize, i))
    }

    /// An image of ours is gone, now that no frame can read it any more.
    pub fn free_image(&mut self, img: &Img) {
        release(img.res);
        if img.srv_index != u32::MAX {
            self.own_free.push(img.srv_index);
        }
    }

    fn make_pipeline(&mut self, serialize: usize) -> Result<(), &'static str> {
        // b0: the vertex shader's register file, four root constants (the half-pixel
        // shift, nought here); t0, t1: the frame and the ramp; s0, s1: point, clamped.
        let range = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SRV, count: 2, base_register: 0, space: 0, offset_in_table: 0 }];
        let params = [
            RootParameter { ty: ROOT_PARAMETER_TYPE_32BIT_CONSTANTS, a: 0, b: 0, c: 4, visibility: SHADER_VISIBILITY_VERTEX },
            RootParameter { ty: ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, a: 1, b: range.as_ptr() as usize, c: 0, visibility: SHADER_VISIBILITY_PIXEL },
        ];
        let sampler = |register| StaticSampler {
            filter: FILTER_MIN_MAG_MIP_POINT,
            address_u: TEXTURE_ADDRESS_MODE_CLAMP,
            address_v: TEXTURE_ADDRESS_MODE_CLAMP,
            address_w: TEXTURE_ADDRESS_MODE_CLAMP,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            comparison: COMPARISON_FUNC_ALWAYS,
            border: STATIC_BORDER_COLOR_OPAQUE_BLACK,
            min_lod: 0.0,
            max_lod: 0.0,
            register,
            space: 0,
            visibility: SHADER_VISIBILITY_PIXEL,
        };
        let samplers = [sampler(0), sampler(1)];
        let desc = RootSignatureDesc {
            count: params.len() as u32,
            params: params.as_ptr(),
            samplers_count: samplers.len() as u32,
            samplers: samplers.as_ptr(),
            flags: ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        };
        let serialize: unsafe extern "system" fn(*const RootSignatureDesc, u32, *mut P, *mut P) -> i32 = unsafe { core::mem::transmute(serialize) };
        let (mut blob, mut err): (P, P) = (core::ptr::null_mut(), core::ptr::null_mut());
        let hr = unsafe { serialize(&desc, ROOT_SIGNATURE_VERSION_1_0, &mut blob, &mut err) };
        if hr < 0 || blob.is_null() {
            if !err.is_null() {
                let msg = vcall!(err, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const u8,);
                log!("d3d12: D3D12SerializeRootSignature: {}", unsafe { crate::rt::cstr_at(msg, 1000) });
            }
            release(err);
            return Err("D3D12SerializeRootSignature failed");
        }
        release(err);
        let (ptr, len) = (vcall!(blob, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const c_void,), vcall!(blob, slot::BLOB_GET_BUFFER_SIZE, fn() -> usize,));
        let mut root: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_ROOT_SIGNATURE, fn(u32, *const c_void, usize, *const crate::com::Guid, *mut P) -> i32, 0, ptr, len, &IID_ROOT_SIGNATURE, &mut root);
        release(blob);
        self.root = made(hr, root, "CreateRootSignature")?;

        let inputs = [
            InputElement { semantic: c"POSITION".as_ptr() as *const u8, index: 0, format: FORMAT_R32G32_FLOAT, slot: 0, offset: 0, class: INPUT_CLASSIFICATION_PER_VERTEX_DATA, step_rate: 0 },
            InputElement { semantic: c"TEXCOORD".as_ptr() as *const u8, index: 0, format: FORMAT_R32G32_FLOAT, slot: 0, offset: 8, class: INPUT_CLASSIFICATION_PER_VERTEX_DATA, step_rate: 0 },
        ];
        let mut blend = BlendDesc::default();
        blend.targets[0] = RenderTargetBlend {
            src: BLEND_ONE,
            dst: BLEND_ZERO,
            op: BLEND_OP_ADD,
            src_alpha: BLEND_ONE,
            dst_alpha: BLEND_ZERO,
            op_alpha: BLEND_OP_ADD,
            logic_op: LOGIC_OP_NOOP,
            write_mask: COLOR_WRITE_ENABLE_ALL,
            ..Default::default()
        };
        let keep = StencilOpDesc { fail: STENCIL_OP_KEEP, depth_fail: STENCIL_OP_KEEP, pass: STENCIL_OP_KEEP, func: COMPARISON_FUNC_ALWAYS };
        let mut formats = [FORMAT_UNKNOWN; 8];
        formats[0] = FORMAT_B8G8R8A8_UNORM;
        let pd = GraphicsPipelineDesc {
            root: self.root,
            vs: ShaderBytecode::of(&VS_PRESENT_DXIL),
            ps: ShaderBytecode::of(&PS_GAMMA_DXIL),
            ds: ShaderBytecode::NONE,
            hs: ShaderBytecode::NONE,
            gs: ShaderBytecode::NONE,
            stream_output: StreamOutputDesc { decl: core::ptr::null(), entries: 0, strides: core::ptr::null(), nstrides: 0, rasterized_stream: 0 },
            blend,
            sample_mask: u32::MAX,
            rasterizer: RasterizerDesc { fill: FILL_MODE_SOLID, cull: CULL_MODE_NONE, depth_clip: 1, ..Default::default() },
            depth_stencil: DepthStencilDesc { depth: 0, write_mask: 0, func: COMPARISON_FUNC_ALWAYS, front: keep, back: keep, ..Default::default() },
            input_layout: InputLayoutDesc { elements: inputs.as_ptr(), count: inputs.len() as u32 },
            strip_cut: 0,
            topology: PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            num_targets: 1,
            formats,
            dsv_format: FORMAT_UNKNOWN,
            sample: SampleDesc { count: 1, quality: 0 },
            node_mask: 0,
            cached: CachedPso { blob: core::ptr::null(), len: 0 },
            flags: 0,
        };
        let mut pso: P = core::ptr::null_mut();
        // Through the library when it has the present pass (stored under this name, the
        // pass being fixed for a given Display8; the library checks the description).
        let name: Vec<u16> = "Display8.present\0".encode_utf16().collect();
        let mut hr = -1;
        if !self.lib.is_null() {
            hr = vcall!(self.lib, slot::LIB_LOAD_GRAPHICS_PIPELINE, fn(*const u16, *const GraphicsPipelineDesc, *const crate::com::Guid, *mut P) -> i32, name.as_ptr(), &pd, &IID_PIPELINE_STATE, &mut pso);
            if hr < 0 {
                pso = core::ptr::null_mut();
            }
        }
        if hr < 0 {
            hr = vcall!(self.dev, slot::DEV_CREATE_GRAPHICS_PIPELINE_STATE, fn(*const GraphicsPipelineDesc, *const crate::com::Guid, *mut P) -> i32, &pd, &IID_PIPELINE_STATE, &mut pso);
            if hr >= 0 && !pso.is_null() && !self.lib.is_null() {
                cache::store(self.lib, &name, pso);
            }
        }
        self.pso = made(hr, pso, "CreateGraphicsPipelineState (the present pass)")?;
        Ok(())
    }

    fn make_slot(&self) -> Result<Slot, &'static str> {
        let mut alloc: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_COMMAND_ALLOCATOR, fn(u32, *const crate::com::Guid, *mut P) -> i32, COMMAND_LIST_TYPE_DIRECT, &IID_COMMAND_ALLOCATOR, &mut alloc);
        let alloc = made(hr, alloc, "CreateCommandAllocator")?;
        let mut list: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_COMMAND_LIST, fn(u32, u32, P, P, *const crate::com::Guid, *mut P) -> i32,
            0, COMMAND_LIST_TYPE_DIRECT, alloc, core::ptr::null_mut(), &IID_GRAPHICS_COMMAND_LIST, &mut list);
        let list = made(hr, list, "CreateCommandList")?;
        // A list is made open: closed until its frame begins.
        vcall!(list, slot::LIST_CLOSE, fn() -> i32,);
        let mut s = Slot { alloc, list, done: 0, chunks: Vec::new(), at: (0, 0), views_at: 0, samplers_at: 0, deferred: Vec::new() };
        s.chunks.push(self.chunk(CHUNK)?);
        Ok(s)
    }

    /// An upload-heap buffer, mapped for good.
    fn chunk(&self, size: u64) -> Result<Chunk, &'static str> {
        let hp = HeapProperties { ty: HEAP_TYPE_UPLOAD, ..Default::default() };
        let rd = ResourceDesc {
            dimension: RESOURCE_DIMENSION_BUFFER,
            alignment: 0,
            width: size,
            height: 1,
            depth_or_array_size: 1,
            mip_levels: 1,
            format: FORMAT_UNKNOWN,
            sample: SampleDesc { count: 1, quality: 0 },
            layout: TEXTURE_LAYOUT_ROW_MAJOR,
            flags: 0,
        };
        let mut res: P = core::ptr::null_mut();
        let hr = vcall!(self.dev, slot::DEV_CREATE_COMMITTED_RESOURCE, fn(*const HeapProperties, u32, *const ResourceDesc, u32, *const ClearValue, *const crate::com::Guid, *mut P) -> i32,
            &hp, 0, &rd, STATE_GENERIC_READ, core::ptr::null(), &IID_RESOURCE, &mut res);
        let res = made(hr, res, "CreateCommittedResource (upload ring)")?;
        let mut ptr: P = core::ptr::null_mut();
        let none = [0usize; 2]; // an empty read range: the CPU only writes
        let hr = vcall!(res, slot::RES_MAP, fn(u32, *const [usize; 2], *mut P) -> i32, 0, &none, &mut ptr);
        if hr < 0 || ptr.is_null() {
            release(res);
            log!("d3d12: Map of the upload ring failed: {:#x}", hr as u32);
            return Err("Map (upload ring)");
        }
        let gpu = vcall!(res, slot::RES_GET_GPU_VIRTUAL_ADDRESS, fn() -> u64,);
        Ok(Chunk { res, gpu, ptr: ptr as *mut u8, size })
    }

    pub fn list(&self) -> P {
        self.slots[self.slot].list
    }

    /// The frame now, begun if no frame is being recorded.
    pub fn frame(&mut self) -> &Dx12Frame8 {
        if !self.recording {
            self.begin();
        }
        &self.frame_out
    }

    fn wait(&self, value: u64) {
        if value == 0 || vcall!(self.fence, slot::FENCE_GET_COMPLETED_VALUE, fn() -> u64,) >= value {
            return;
        }
        if vcall!(self.fence, slot::FENCE_SET_EVENT_ON_COMPLETION, fn(u64, HANDLE) -> i32, value, self.fence_event) >= 0 {
            // A device that stops answering does not hang the game for good.
            unsafe { WaitForSingleObject(self.fence_event, 5000) };
        }
    }

    fn begin(&mut self) {
        let done = self.slots[self.slot].done;
        self.wait(done);
        let s = &mut self.slots[self.slot];
        for (f, arg) in s.deferred.drain(..) {
            f(arg);
        }
        vcall!(s.alloc, slot::ALLOC_RESET, fn() -> i32,);
        vcall!(s.list, slot::LIST_RESET, fn(P, P) -> i32, s.alloc, core::ptr::null_mut());
        s.at = (0, 0);
        s.views_at = 0;
        s.samplers_at = 0;
        let heaps = [self.views_heap, self.samplers_heap];
        vcall!(s.list, slot::LIST_SET_DESCRIPTOR_HEAPS, fn(u32, *const P) -> (), 2, heaps.as_ptr());
        self.recording = true;
        self.frame_out = Dx12Frame8 { size: core::mem::size_of::<Dx12Frame8>() as u32, index: self.index, slot: self.slot as u32, command_list: s.list as usize };
        if self.first {
            self.first = false;
            if self.ramp_values.is_none() {
                // An identity ramp until the engine sets one.
                let mut id = [0u16; 768];
                for i in 0..256 {
                    let v = (i as u16) * 257;
                    (id[i], id[256 + i], id[512 + i]) = (v, v, v);
                }
                self.ramp_values = Some(id);
            }
        }
    }

    /// A transition of one subresource (or all: `ALL_SUBRESOURCES`).
    pub fn barrier(&self, res: P, sub: u32, before: u32, after: u32) {
        if before == after {
            return;
        }
        let b = [Barrier { ty: BARRIER_TYPE_TRANSITION, flags: 0, resource: res, subresource: sub, before, after }];
        vcall!(self.list(), slot::LIST_RESOURCE_BARRIER, fn(u32, *const Barrier) -> (), 1, b.as_ptr());
    }

    /// Space in the frame's upload ring.
    pub fn alloc(&mut self, bytes: u32, align: u32) -> Option<Dx12Alloc8> {
        if !self.recording {
            self.begin();
        }
        let align = align.max(4) as u64;
        let need = bytes as u64;
        let (ci, off) = self.chunk_for(need, align)?;
        let s = &mut self.slots[self.slot];
        s.at = (ci, off + need);
        let c = &s.chunks[ci];
        Some(Dx12Alloc8 { size: core::mem::size_of::<Dx12Alloc8>() as u32, resource: c.res as usize, offset: off as u32, gpu: c.gpu + off, cpu: unsafe { c.ptr.add(off as usize) } })
    }

    fn chunk_for(&mut self, need: u64, align: u64) -> Option<(usize, u64)> {
        let (mut ci, at) = self.slots[self.slot].at;
        let mut off = at.div_ceil(align) * align;
        loop {
            let len = self.slots[self.slot].chunks.len();
            if ci < len {
                if off + need <= self.slots[self.slot].chunks[ci].size {
                    return Some((ci, off));
                }
                ci += 1;
                off = 0;
                continue;
            }
            let c = match self.chunk(need.max(CHUNK)) {
                Ok(c) => c,
                Err(e) => {
                    log!("d3d12: the upload ring could not grow: {}", e);
                    return None;
                }
            };
            log!("d3d12: the upload ring of frame slot {} grew to {} chunks", self.slot, len + 1);
            self.slots[self.slot].chunks.push(c);
        }
    }

    /// `count` descriptors in a shader-visible heap (`HEAP_*`), for the frame now.
    pub fn descriptors(&mut self, heap: u32, count: u32) -> Option<Dx12Descriptors8> {
        if !self.recording {
            self.begin();
        }
        let s = &mut self.slots[self.slot];
        let (per, at, cpu, gpu, inc) = if heap == gpu::display::HEAP_SAMPLER {
            (FRAME_SAMPLERS, &mut s.samplers_at, self.samplers_cpu, self.samplers_gpu, self.inc[1])
        } else {
            (FRAME_VIEWS, &mut s.views_at, self.views_cpu, self.views_gpu, self.inc[0])
        };
        if *at + count > per {
            log!("d3d12: frame slot {} is out of shader-visible descriptors ({} of {} taken, {} asked for)", self.slot, *at, per, count);
            return None;
        }
        let i = self.slot as u32 * per + *at;
        *at += count;
        Some(Dx12Descriptors8 { size: core::mem::size_of::<Dx12Descriptors8>() as u32, cpu: cpu + (i * inc) as usize, gpu: gpu + (i * inc) as u64, count })
    }

    pub fn defer(&mut self, f: extern "C" fn(*mut c_void), arg: *mut c_void) {
        let s = if self.recording { self.slot } else { (self.slot + SLOTS - 1) % SLOTS };
        self.slots[s].deferred.push((f, arg));
    }

    /// The engine's `Clear` of its targets: rectangles (none: the whole viewport).
    pub fn clear(&mut self, rects: &[[i32; 4]], viewport: [u32; 4], flags: u32, color: u32, z: f32) {
        if !self.recording {
            self.begin();
        }
        let (w, h) = (self.color.w as i32, self.color.h as i32);
        let vr = [viewport[0] as i32, viewport[1] as i32, (viewport[0] + viewport[2]) as i32, (viewport[1] + viewport[3]) as i32];
        let list: Vec<[i32; 4]> = if rects.is_empty() { alloc::vec![vr] } else { rects.to_vec() };
        let mut rs: Vec<Rect> = Vec::new();
        for r in list {
            let (x0, y0, x1, y1) = (r[0].max(0).max(vr[0]), r[1].max(0).max(vr[1]), r[2].min(w).min(vr[2]), r[3].min(h).min(vr[3]));
            if x1 > x0 && y1 > y0 {
                rs.push(Rect { left: x0, top: y0, right: x1, bottom: y1 });
            }
        }
        if rs.is_empty() {
            return;
        }
        let l = self.list();
        if flags & 1 != 0 {
            let c = [((color >> 16) & 255) as f32 / 255.0, ((color >> 8) & 255) as f32 / 255.0, (color & 255) as f32 / 255.0, (color >> 24) as f32 / 255.0];
            vcall!(l, slot::LIST_CLEAR_RENDER_TARGET_VIEW, fn(usize, *const [f32; 4], u32, *const Rect) -> (), self.color.rtv, &c, rs.len() as u32, rs.as_ptr());
        }
        if flags & 2 != 0 {
            vcall!(l, slot::LIST_CLEAR_DEPTH_STENCIL_VIEW, fn(usize, u32, f32, u32, u32, *const Rect) -> (), self.depth.dsv, CLEAR_FLAG_DEPTH, z, 0, rs.len() as u32, rs.as_ptr());
        }
    }

    /// Pixels into a texture's level (`bpp`-byte texels, `pitch` bytes a row), through
    /// the ring: a mirror's level, the ramp, the engine's back buffer written by lock.
    /// The level goes from state `from` to `to`.
    pub fn upload(&mut self, res: P, format: u32, level: u32, w: u32, h: u32, src: &[u8], pitch: u32, bpp: u32, from: u32, to: u32) -> bool {
        if !self.recording {
            self.begin();
        }
        let row = w * bpp;
        let rp = row.div_ceil(TEXTURE_DATA_PITCH_ALIGNMENT) * TEXTURE_DATA_PITCH_ALIGNMENT;
        let Some(a) = self.alloc(rp * h, TEXTURE_DATA_PLACEMENT_ALIGNMENT) else { return false };
        for y in 0..h as usize {
            unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add(y * pitch as usize), a.cpu.add(y * rp as usize), row as usize) };
        }
        self.barrier(res, level, from, STATE_COPY_DEST);
        let dst = CopyLocation { resource: res, ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: level as u64, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
        let srcl = CopyLocation {
            resource: a.resource as P,
            ty: TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            offset: a.offset as u64,
            format,
            width: w,
            height: h,
            depth: 1,
            row_pitch: rp,
        };
        vcall!(self.list(), slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const c_void) -> (),
            &dst, 0, 0, 0, &srcl, core::ptr::null());
        self.barrier(res, level, STATE_COPY_DEST, to);
        true
    }

    pub fn set_ramp(&mut self, r: &[u16; 768]) {
        self.ramp_values = Some(*r);
        self.stale = true;
    }

    /// The engine's `Present`: the last pass into the swap chain's buffer, then submit
    /// and present. False: the device is gone.
    pub fn present(&mut self) -> bool {
        if self.lost {
            return false;
        }
        if !self.recording {
            self.begin();
        }
        if self.stale {
            self.stale = false;
            if let Some(r) = self.ramp_values {
                let mut px = [0u8; 1024];
                for i in 0..256 {
                    let hi = |v: u16| (v >> 8) as u8; // as Direct3D 9's stub draws it (stub/gamma.rs)
                    px[i * 4..i * 4 + 4].copy_from_slice(&[hi(r[512 + i]), hi(r[256 + i]), hi(r[i]), 255]);
                }
                let (res, from) = (self.ramp.res, self.ramp.state);
                self.upload(res, FORMAT_B8G8R8A8_UNORM, 0, 256, 1, &px, 1024, 4, from, STATE_SHADER_RESOURCE);
                self.ramp.state = STATE_SHADER_RESOURCE;
            }
        }
        let l = self.list();
        let bb = vcall!(self.swap, slot::SWAP_GET_CURRENT_BACK_BUFFER_INDEX, fn() -> u32,) as usize % self.sc_buffers.len();
        let target = self.sc_buffers[bb];
        let rtv = self.rtv_start + bb * self.inc[2] as usize;
        let color = self.color.res;
        self.barrier(color, ALL_SUBRESOURCES, STATE_RENDER_TARGET, STATE_PIXEL_SHADER_RESOURCE);
        self.barrier(target, ALL_SUBRESOURCES, STATE_PRESENT, STATE_RENDER_TARGET);
        let quad: [f32; 16] = [-1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0];
        let (Some(va), Some(table)) = (self.alloc(64, 16), self.descriptors(gpu::display::HEAP_CBV_SRV_UAV, 2)) else {
            return false;
        };
        unsafe { core::ptr::copy_nonoverlapping(quad.as_ptr() as *const u8, va.cpu, 64) };
        vcall!(self.dev, slot::DEV_COPY_DESCRIPTORS_SIMPLE, fn(u32, usize, usize, u32) -> (), 1, table.cpu, self.color.srv, DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
        vcall!(self.dev, slot::DEV_COPY_DESCRIPTORS_SIMPLE, fn(u32, usize, usize, u32) -> (), 1, table.cpu + self.inc[0] as usize, self.ramp.srv, DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
        let vp = Viewport { x: 0.0, y: 0.0, w: self.sc_w as f32, h: self.sc_h as f32, min_z: 0.0, max_z: 1.0 };
        let sc = Rect { left: 0, top: 0, right: self.sc_w as i32, bottom: self.sc_h as i32 };
        let vbv = VertexBufferView { location: va.gpu, size: 64, stride: 16 };
        let shift = [0f32; 4];
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 1, &rtv, 0, core::ptr::null());
        vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport) -> (), 1, &vp);
        vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const Rect) -> (), 1, &sc);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), self.root);
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), self.pso);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS, fn(u32, u32, *const f32, u32) -> (), 0, 4, shift.as_ptr(), 0);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 1, table.gpu);
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 0, 1, &vbv);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), 4, 1, 0, 0);
        self.barrier(target, ALL_SUBRESOURCES, STATE_RENDER_TARGET, STATE_PRESENT);
        self.barrier(color, ALL_SUBRESOURCES, STATE_PIXEL_SHADER_RESOURCE, STATE_RENDER_TARGET);
        let hr = vcall!(l, slot::LIST_CLOSE, fn() -> i32,);
        if hr < 0 {
            log!("d3d12: the frame's command list did not close: {:#x}", hr as u32);
        }
        let lists = [l];
        vcall!(self.queue, slot::QUEUE_EXECUTE_COMMAND_LISTS, fn(u32, *const P) -> (), 1, lists.as_ptr());
        let hr = vcall!(self.swap, slot::SWAP_PRESENT, fn(u32, u32) -> i32, self.sync_interval, 0);
        self.fence_value += 1;
        vcall!(self.queue, slot::QUEUE_SIGNAL, fn(P, u64) -> i32, self.fence, self.fence_value);
        self.slots[self.slot].done = self.fence_value;
        self.recording = false;
        self.slot = (self.slot + 1) % SLOTS;
        self.index += 1;
        if self.index % cache::EVERY == 0 {
            self.lib_saved = cache::save(self.lib, &self.lib_id, self.lib_saved);
        }
        if hr == DXGI_ERROR_DEVICE_REMOVED || hr == DXGI_ERROR_DEVICE_RESET {
            let why = vcall!(self.dev, slot::DEV_GET_DEVICE_REMOVED_REASON, fn() -> i32,);
            log!("d3d12: the device is gone ({:#x}, reason {:#x}); nothing is drawn from here", hr as u32, why as u32);
            self.lost = true;
            return false;
        }
        if hr < 0 {
            log!("d3d12: Present: {:#x}", hr as u32);
        } else if self.index == 1 || self.index == 1000 {
            log!("d3d12: frame {} presented", self.index);
        }
        true
    }

    /// Submits what the frame now has recorded, waits for it, and opens the same list
    /// again with its heaps set (`display8v1_dx12_flush`: a plugin's read-back). The
    /// frame's ring, descriptors and deferred calls stay as they are.
    pub fn flush(&mut self) -> bool {
        if self.lost {
            return false;
        }
        if !self.recording {
            return true;
        }
        let s = &self.slots[self.slot];
        let (alloc, l) = (s.alloc, s.list);
        let hr = vcall!(l, slot::LIST_CLOSE, fn() -> i32,);
        if hr < 0 {
            log!("d3d12: flush: the frame's command list did not close: {:#x}", hr as u32);
            return false;
        }
        let lists = [l];
        vcall!(self.queue, slot::QUEUE_EXECUTE_COMMAND_LISTS, fn(u32, *const P) -> (), 1, lists.as_ptr());
        self.fence_value += 1;
        vcall!(self.queue, slot::QUEUE_SIGNAL, fn(P, u64) -> i32, self.fence, self.fence_value);
        self.wait(self.fence_value);
        vcall!(alloc, slot::ALLOC_RESET, fn() -> i32,);
        vcall!(l, slot::LIST_RESET, fn(P, P) -> i32, alloc, core::ptr::null_mut());
        let heaps = [self.views_heap, self.samplers_heap];
        vcall!(l, slot::LIST_SET_DESCRIPTOR_HEAPS, fn(u32, *const P) -> (), 2, heaps.as_ptr());
        true
    }

    pub fn target(&self, which: u32) -> Dx12Image8 {
        let i = if which == gpu::display::TARGET_DEPTH { &self.depth } else { &self.color };
        i.info()
    }
}

impl Img {
    pub const fn null() -> Img {
        Img { res: core::ptr::null_mut(), format: 0, w: 0, h: 0, levels: 0, state: 0, rtv: 0, dsv: 0, srv: 0, srv_index: u32::MAX }
    }

    pub fn info(&self) -> Dx12Image8 {
        Dx12Image8 {
            size: core::mem::size_of::<Dx12Image8>() as u32,
            resource: self.res as usize,
            format: self.format,
            width: self.w,
            height: self.h,
            levels: self.levels,
            samples: 1,
            state: self.state,
            rtv: self.rtv,
            dsv: self.dsv,
            srv: self.srv,
        }
    }
}

fn cpu_start(heap: P) -> usize {
    let mut h = 0usize;
    vcall!(heap, slot::HEAP_GET_CPU_START, fn(*mut usize) -> *mut usize, &mut h);
    h
}

fn gpu_start(heap: P) -> u64 {
    let mut h = 0u64;
    vcall!(heap, slot::HEAP_GET_GPU_START, fn(*mut u64) -> *mut u64, &mut h);
    h
}
