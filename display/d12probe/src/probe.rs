//! The probe itself, step by step, in the order Display8 brings Direct3D 12 up
//! (`display/src/dx12/mod.rs`): the optional Agility SDK and debug layer, a factory and
//! the adapter asked for, a device at 11_0, a queue and fence, a flip-model swap chain,
//! the two shader-visible heaps, an 8 MB upload ring mapped once and kept mapped, and
//! the targets. Then Display8's root signatures and pipelines, built by the very code it
//! uses (`dx12/build.rs`). Then 300 frames: each writes its filler, ramp, vertices and
//! constant blocks into the ring (two frames in flight share it, half each), draws the
//! passthrough's pipeline into an offscreen target, and that target through the present
//! pass into the swap chain. The last frame's back buffer is copied into a read-back
//! buffer and seven of its pixels checked against what the clears must make. With the
//! debug layer on, its messages are listed and an error among them fails the probe.
use crate::dx12::build::{self, Key};
use crate::dx12::raw::*;
use crate::passthrough::state::{self, Layout, PipeState, PS_AT};
use crate::report::{begin, cstr_at, fail, fmt, note, pass, section};
use crate::shaders::ff_dxil::CB_SPACES;
use crate::user32;
use crate::win::*;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;

const W: u32 = 640;
const H: u32 = 480;
const FRAMES: u32 = 300;
const SLOTS: usize = 2;
/// The ring: as big as a frame of the game's would want; each frame in flight has half.
const RING: u64 = 8 << 20;
/// Written into the ring each frame besides its draws, standing in for the engine's
/// dynamic vertices.
const FILLER: usize = 64 << 10;
/// Five seconds, for every wait: a probe that hangs says so.
const TIMEOUT_MS: u32 = 5000;

/// The left half of the offscreen target is cleared to A and the right half to B; the
/// present pass, with an identity ramp, must put them in the back buffer as they are.
const A: [f32; 4] = [0.8, 0.2, 0.1, 1.0];
const B: [f32; 4] = [0.1, 0.3, 0.9, 1.0];

const FORMAT_R16G16B16A16_FLOAT: u32 = 10;
const FORMAT_D32_FLOAT: u32 = 40;
const HEAP_TYPE_READBACK: u32 = 3;
const STATE_COPY_SOURCE: u32 = 0x800;

const IID_FACTORY4: crate::com::Guid = crate::com::Guid { d1: 0x1bc6ea02, d2: 0xef36, d3: 0x464f, d4: [0xbf, 0x0c, 0x21, 0xca, 0x39, 0xe5, 0x16, 0x8a] };
const IID_ADAPTER: crate::com::Guid = crate::com::Guid { d1: 0x2411e7e1, d2: 0x12ac, d3: 0x4ccf, d4: [0xbd, 0x14, 0x97, 0x98, 0xe8, 0x53, 0x4d, 0xc0] };
const IID_INFO_QUEUE: crate::com::Guid = crate::com::Guid { d1: 0x0742a90b, d2: 0xc387, d3: 0x483f, d4: [0xb9, 0x46, 0x30, 0xa7, 0xe4, 0xe6, 0x14, 0x58] };
const CLSID_SDK_CONFIGURATION: crate::com::Guid = crate::com::Guid { d1: 0x7cda6aca, d2: 0xa03e, d3: 0x49c8, d4: [0x94, 0x58, 0x03, 0x34, 0xd2, 0x0e, 0x07, 0xce] };
const IID_SDK_CONFIGURATION: crate::com::Guid = crate::com::Guid { d1: 0xe9eb5314, d2: 0x33aa, d3: 0x42b2, d4: [0xa7, 0x18, 0xd7, 0x7f, 0x58, 0xb1, 0xf1, 0xc7] };

mod extra_slot {
    pub const FACTORY_ENUM_WARP_ADAPTER: usize = 27;
    pub const SDK_SET_VERSION: usize = 3;
    pub const INFO_GET_MESSAGE: usize = 5;
    pub const INFO_GET_NUM_STORED: usize = 8;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCommandLineA() -> *const u8;
}

type R<T> = Result<T, String>;

fn made(hr: i32, p: P, what: &str) -> R<P> {
    if hr < 0 || p.is_null() {
        Err(fmt(format_args!("{}: {:#010x}", what, hr as u32)))
    } else {
        Ok(p)
    }
}

/// What the command line asks for (README.md).
#[derive(Default)]
struct Args {
    warp: bool,
    debug: bool,
    /// `--agility <version> <folder>`: Agility SDK's D3D12Core.dll and, with it, the
    /// debug layer's D3D12SDKLayers.dll, from this folder beside the exe.
    agility: Option<(u32, Vec<u8>)>,
}

fn args() -> Args {
    let line = unsafe { cstr_at(GetCommandLineA(), 1000) };
    let mut words = line.split_ascii_whitespace().skip(1);
    let mut a = Args::default();
    while let Some(w) = words.next() {
        match w {
            "--warp" => a.warp = true,
            "--debug" => a.debug = true,
            "--agility" => {
                let v = words.next().and_then(|v| v.parse::<u32>().ok());
                let p = words.next();
                if let (Some(v), Some(p)) = (v, p) {
                    let mut path = p.trim_matches('"').as_bytes().to_vec();
                    path.push(0);
                    a.agility = Some((v, path));
                }
            }
            _ => {}
        }
    }
    a
}

struct Dev {
    dev: P,
    queue: P,
    fence: P,
    event: HANDLE,
    value: u64,
    #[allow(dead_code)]
    info: P,
}

impl Dev {
    fn wait(&self, value: u64) -> R<()> {
        if vcall!(self.fence, slot::FENCE_GET_COMPLETED_VALUE, fn() -> u64,) >= value {
            return Ok(());
        }
        vcall!(self.fence, slot::FENCE_SET_EVENT_ON_COMPLETION, fn(u64, HANDLE) -> i32, value, self.event);
        if unsafe { WaitForSingleObject(self.event, TIMEOUT_MS) } != 0 {
            return Err(fmt(format_args!("the GPU did not reach fence {} in {} ms", value, TIMEOUT_MS)));
        }
        Ok(())
    }

    fn signal(&mut self) -> u64 {
        self.value += 1;
        vcall!(self.queue, slot::QUEUE_SIGNAL, fn(P, u64) -> i32, self.fence, self.value);
        self.value
    }
}

fn heap(d: P, ty: u32, count: u32, flags: u32) -> R<P> {
    let hd = DescriptorHeapDesc { ty, count, flags, node_mask: 0 };
    let mut h: P = core::ptr::null_mut();
    let hr = vcall!(d, slot::DEV_CREATE_DESCRIPTOR_HEAP, fn(*const DescriptorHeapDesc, *const crate::com::Guid, *mut P) -> i32, &hd, &IID_DESCRIPTOR_HEAP, &mut h);
    made(hr, h, "CreateDescriptorHeap")
}

fn cpu_start(h: P) -> usize {
    let mut v = 0usize;
    vcall!(h, slot::HEAP_GET_CPU_START, fn(*mut usize) -> *mut usize, &mut v);
    v
}

fn gpu_start(h: P) -> u64 {
    let mut v = 0u64;
    vcall!(h, slot::HEAP_GET_GPU_START, fn(*mut u64) -> *mut u64, &mut v);
    v
}

fn buffer(d: P, kind: u32, size: u64, state: u32) -> R<P> {
    let hp = HeapProperties { ty: kind, ..Default::default() };
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
    let hr = vcall!(d, slot::DEV_CREATE_COMMITTED_RESOURCE, fn(*const HeapProperties, u32, *const ResourceDesc, u32, *const ClearValue, *const crate::com::Guid, *mut P) -> i32,
        &hp, 0, &rd, state, core::ptr::null(), &IID_RESOURCE, &mut res);
    made(hr, res, "CreateCommittedResource (buffer)")
}

fn texture(d: P, w: u32, h: u32, format: u32, flags: u32, state: u32) -> R<P> {
    let hp = HeapProperties { ty: HEAP_TYPE_DEFAULT, ..Default::default() };
    let rd = ResourceDesc {
        dimension: RESOURCE_DIMENSION_TEXTURE2D,
        alignment: 0,
        width: w as u64,
        height: h,
        depth_or_array_size: 1,
        mip_levels: 1,
        format,
        sample: SampleDesc { count: 1, quality: 0 },
        layout: TEXTURE_LAYOUT_UNKNOWN,
        flags,
    };
    let clear = ClearValue { format, color: if format == FORMAT_D16_UNORM { [1.0, 0.0, 0.0, 0.0] } else { [0.0; 4] } };
    let cv: *const ClearValue = if flags != 0 { &clear } else { core::ptr::null() };
    let mut res: P = core::ptr::null_mut();
    let hr = vcall!(d, slot::DEV_CREATE_COMMITTED_RESOURCE, fn(*const HeapProperties, u32, *const ResourceDesc, u32, *const ClearValue, *const crate::com::Guid, *mut P) -> i32,
        &hp, 0, &rd, state, cv, &IID_RESOURCE, &mut res);
    made(hr, res, "CreateCommittedResource (texture)")
}

fn srv(d: P, res: P, format: u32, cpu: usize) {
    let sd = SrvDesc { format, dimension: SRV_DIMENSION_TEXTURE2D, mapping: DEFAULT_COMPONENT_MAPPING, most_detailed_mip: 0, mip_levels: 1, ..Default::default() };
    vcall!(d, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), res, &sd, cpu);
}

fn barrier(l: P, res: P, before: u32, after: u32) {
    let b = [Barrier { ty: BARRIER_TYPE_TRANSITION, flags: 0, resource: res, subresource: ALL_SUBRESOURCES, before, after }];
    vcall!(l, slot::LIST_RESOURCE_BARRIER, fn(u32, *const Barrier) -> (), 1, b.as_ptr());
}

/// The debug layer's messages: counted by severity, the first twenty listed. Returns
/// (corruption + errors, warnings).
fn messages(info: P) -> (u32, u32) {
    if info.is_null() {
        return (0, 0);
    }
    let n = vcall!(info, extra_slot::INFO_GET_NUM_STORED, fn() -> u64,);
    let (mut bad, mut warn) = (0, 0);
    for i in 0..n {
        let mut len = 0usize;
        vcall!(info, extra_slot::INFO_GET_MESSAGE, fn(u64, *mut u8, *mut usize) -> i32, i, core::ptr::null_mut(), &mut len);
        if len == 0 || len > 1 << 20 {
            continue;
        }
        // The message is a header followed by its text: 4-aligned room for both.
        let mut buf: Vec<u32> = alloc::vec![0u32; len.div_ceil(4) + 8];
        if vcall!(info, extra_slot::INFO_GET_MESSAGE, fn(u64, *mut u8, *mut usize) -> i32, i, buf.as_mut_ptr() as *mut u8, &mut len) < 0 {
            continue;
        }
        // D3D12_MESSAGE: category, severity, id, description, its length.
        let sev = buf[1];
        let id = buf[2];
        let text = buf[3] as usize as *const u8;
        match sev {
            0 | 1 => bad += 1,
            2 => warn += 1,
            _ => {}
        }
        if i < 20 && sev <= 2 {
            let tag = ["CORRUPTION", "ERROR", "WARNING"][sev as usize];
            out!("debug layer {} #{}: {}", tag, id, cstr_at(text, 600));
        }
    }
    (bad, warn)
}

pub fn run() -> bool {
    match probe() {
        Ok(()) => true,
        Err((step, why)) => {
            fail(&step, &why);
            false
        }
    }
}

/// The step names are passed up with the reason, so the verdict names the step.
fn probe() -> Result<(), (String, String)> {
    macro_rules! step {
        ($name:expr, $body:expr) => {{
            begin($name);
            match (|| -> R<_> { $body })() {
                Ok(v) => v,
                Err(e) => return Err((String::from($name), e)),
            }
        }};
    }

    section("options");
    let a = step!("command line", {
        let a = args();
        out!("--warp: {}, --debug: {}, --agility: {}", a.warp, a.debug, match &a.agility {
            Some((v, p)) => fmt(format_args!("{} in {}", v, core::str::from_utf8(&p[..p.len() - 1]).unwrap_or("?"))),
            None => String::from("no"),
        });
        pass("command line", "");
        Ok(a)
    });

    section("device");
    let d3d12 = step!("d3d12.dll", {
        let m = unsafe { LoadLibraryA(c"d3d12.dll".as_ptr() as *const u8) };
        if m.is_null() {
            return Err(String::from("no d3d12.dll"));
        }
        pass("d3d12.dll", "");
        Ok(m)
    });
    let proc = |m: HANDLE, name: &core::ffi::CStr| unsafe { GetProcAddress(m, name.as_ptr() as *const u8) };

    // The Agility SDK, if asked: its D3D12Core.dll replaces the system's (and brings the
    // debug layer along), chosen before anything else is made.
    if let Some((version, path)) = &a.agility {
        begin("Agility SDK");
        let get = proc(d3d12, c"D3D12GetInterface");
        if get == 0 {
            return Err((String::from("Agility SDK"), String::from("d3d12.dll has no D3D12GetInterface (Windows older than 10 2004)")));
        }
        let get: unsafe extern "system" fn(*const crate::com::Guid, *const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(get) };
        let mut cfg: P = core::ptr::null_mut();
        let hr = unsafe { get(&CLSID_SDK_CONFIGURATION, &IID_SDK_CONFIGURATION, &mut cfg) };
        if hr < 0 || cfg.is_null() {
            return Err((String::from("Agility SDK"), fmt(format_args!("D3D12GetInterface(ID3D12SDKConfiguration): {:#010x}", hr as u32))));
        }
        let hr = vcall!(cfg, extra_slot::SDK_SET_VERSION, fn(u32, *const u8) -> i32, *version, path.as_ptr());
        release(cfg);
        if hr < 0 {
            return Err((String::from("Agility SDK"), fmt(format_args!("SetSDKVersion({}): {:#010x}", version, hr as u32))));
        }
        pass("Agility SDK", &fmt(format_args!("SDK version {}", version)));
    } else {
        note("Agility SDK", "not asked for: the system's Direct3D 12");
    }

    let debug_on = if a.debug {
        begin("debug layer");
        let get = proc(d3d12, c"D3D12GetDebugInterface");
        if get == 0 {
            return Err((String::from("debug layer"), String::from("d3d12.dll has no D3D12GetDebugInterface")));
        }
        let get: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(get) };
        let mut dbg: P = core::ptr::null_mut();
        let hr = unsafe { get(&IID_DEBUG, &mut dbg) };
        if hr < 0 || dbg.is_null() {
            return Err((String::from("debug layer"), fmt(format_args!("no debug layer ({:#010x}): Graphics Tools or --agility is missing", hr as u32))));
        }
        vcall!(dbg, slot::DEBUG_ENABLE_DEBUG_LAYER, fn() -> (),);
        release(dbg);
        pass("debug layer", "enabled");
        true
    } else {
        note("debug layer", "off (--debug)");
        false
    };

    let (mut dev, adapter_name) = step!("device", {
        let dxgi = unsafe { LoadLibraryA(c"dxgi.dll".as_ptr() as *const u8) };
        let create = proc(dxgi, c"CreateDXGIFactory1");
        if dxgi.is_null() || create == 0 {
            return Err(String::from("no dxgi.dll / CreateDXGIFactory1"));
        }
        let create: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(create) };
        let mut factory: P = core::ptr::null_mut();
        let hr = unsafe { create(&IID_FACTORY4, &mut factory) };
        let factory = made(hr, factory, "CreateDXGIFactory1 (IDXGIFactory4)")?;
        let mut adapter: P = core::ptr::null_mut();
        let mut name = String::from("the default adapter");
        if a.warp {
            let hr = vcall!(factory, extra_slot::FACTORY_ENUM_WARP_ADAPTER, fn(*const crate::com::Guid, *mut P) -> i32, &IID_ADAPTER, &mut adapter);
            made(hr, adapter, "EnumWarpAdapter")?;
        } else {
            let hr = vcall!(factory, slot::FACTORY_ENUM_ADAPTERS, fn(u32, *mut P) -> i32, 0, &mut adapter);
            if hr >= 0 && !adapter.is_null() {
                // Leave the choice to Direct3D 12 when asked for nothing: the first
                // adapter is only named in the report.
            } else {
                adapter = core::ptr::null_mut();
            }
        }
        if !adapter.is_null() {
            // DXGI_ADAPTER_DESC: Description[128] (UTF-16), then ids and memory sizes.
            let mut desc = [0u8; 400];
            if vcall!(adapter, slot::ADAPTER_GET_DESC, fn(*mut u8) -> i32, desc.as_mut_ptr()) >= 0 {
                name = String::new();
                for c in desc[..256].chunks(2) {
                    let ch = u16::from_le_bytes([c[0], c[1]]);
                    if ch == 0 {
                        break;
                    }
                    name.push(if ch < 128 { ch as u8 as char } else { '?' });
                }
            }
        }
        let create_device = proc(d3d12, c"D3D12CreateDevice");
        if create_device == 0 {
            return Err(String::from("d3d12.dll has no D3D12CreateDevice"));
        }
        let create_device: unsafe extern "system" fn(P, u32, *const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(create_device) };
        let mut dev: P = core::ptr::null_mut();
        let hr = unsafe { create_device(if a.warp { adapter } else { core::ptr::null_mut() }, FEATURE_LEVEL_11_0, &IID_DEVICE, &mut dev) };
        let dev = made(hr, dev, "D3D12CreateDevice at feature level 11_0")?;
        release(adapter);
        release(factory);
        pass("device", &fmt(format_args!("feature level 11_0 on {}", name)));
        Ok((dev, name))
    });
    out!("adapter: {}", adapter_name);

    let mut info: P = core::ptr::null_mut();
    if debug_on {
        let hr = vcall!(dev, slot::QUERY_INTERFACE, fn(*const crate::com::Guid, *mut P) -> i32, &IID_INFO_QUEUE, &mut info);
        if hr < 0 {
            info = core::ptr::null_mut();
            note("info queue", "the device has none: the debug layer's messages are not listed");
        }
    }

    let mut d = step!("queue and fence", {
        let qd = CommandQueueDesc { ty: COMMAND_LIST_TYPE_DIRECT, ..Default::default() };
        let mut queue: P = core::ptr::null_mut();
        let hr = vcall!(dev, slot::DEV_CREATE_COMMAND_QUEUE, fn(*const CommandQueueDesc, *const crate::com::Guid, *mut P) -> i32, &qd, &IID_COMMAND_QUEUE, &mut queue);
        let queue = made(hr, queue, "CreateCommandQueue")?;
        let mut fence: P = core::ptr::null_mut();
        let hr = vcall!(dev, slot::DEV_CREATE_FENCE, fn(u64, u32, *const crate::com::Guid, *mut P) -> i32, 0, 0, &IID_FENCE, &mut fence);
        let fence = made(hr, fence, "CreateFence")?;
        let event = unsafe { CreateEventA(core::ptr::null_mut(), 0, 0, core::ptr::null()) };
        if event.is_null() {
            return Err(String::from("CreateEventA failed"));
        }
        pass("queue and fence", "");
        Ok(Dev { dev, queue, fence, event, value: 0, info })
    });
    dev = d.dev;

    // The swap chain: flip-model, discard, two B8G8R8A8 buffers, as Display8 makes it.
    let (window, swap, backs) = step!("swap chain", {
        let window = user32::open(W as i32, H as i32);
        if window.is_null() {
            return Err(String::from("no window"));
        }
        let dxgi = unsafe { LoadLibraryA(c"dxgi.dll".as_ptr() as *const u8) };
        let create: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(proc(dxgi, c"CreateDXGIFactory1")) };
        let mut factory: P = core::ptr::null_mut();
        let hr = unsafe { create(&IID_DXGI_FACTORY2, &mut factory) };
        let factory = made(hr, factory, "CreateDXGIFactory1 (IDXGIFactory2)")?;
        let desc = SwapChainDesc1 {
            width: W,
            height: H,
            format: FORMAT_B8G8R8A8_UNORM,
            sample: SampleDesc { count: 1, quality: 0 },
            usage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            buffers: 2,
            scaling: DXGI_SCALING_STRETCH,
            swap_effect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            ..Default::default()
        };
        let mut sc1: P = core::ptr::null_mut();
        let hr = vcall!(factory, slot::FACTORY_CREATE_SWAP_CHAIN_FOR_HWND, fn(P, *mut c_void, *const SwapChainDesc1, *const c_void, P, *mut P) -> i32,
            d.queue, window as *mut c_void, &desc, core::ptr::null(), core::ptr::null_mut(), &mut sc1);
        let sc1 = made(hr, sc1, "CreateSwapChainForHwnd")?;
        vcall!(factory, slot::FACTORY_MAKE_WINDOW_ASSOCIATION, fn(*mut c_void, u32) -> i32, window as *mut c_void, DXGI_MWA_NO_WINDOW_CHANGES | DXGI_MWA_NO_ALT_ENTER);
        release(factory);
        let mut sc3: P = core::ptr::null_mut();
        let hr = vcall!(sc1, slot::QUERY_INTERFACE, fn(*const crate::com::Guid, *mut P) -> i32, &IID_DXGI_SWAP_CHAIN3, &mut sc3);
        release(sc1);
        let sc3 = made(hr, sc3, "IDXGISwapChain3")?;
        let mut backs = [core::ptr::null_mut(); 2];
        for (i, b) in backs.iter_mut().enumerate() {
            let hr = vcall!(sc3, slot::SWAP_GET_BUFFER, fn(u32, *const crate::com::Guid, *mut P) -> i32, i as u32, &IID_RESOURCE, b);
            made(hr, *b, "IDXGISwapChain::GetBuffer")?;
        }
        pass("swap chain", "flip-discard, 2 x B8G8R8A8, 640x480");
        Ok((window, sc3, backs))
    });

    section("resources");
    let inc = [DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, DESCRIPTOR_HEAP_TYPE_SAMPLER, DESCRIPTOR_HEAP_TYPE_RTV, DESCRIPTOR_HEAP_TYPE_DSV]
        .map(|t| vcall!(dev, slot::DEV_GET_DESCRIPTOR_HANDLE_INCREMENT_SIZE, fn(u32) -> u32, t));
    struct Res {
        views: P,
        samplers: P,
        ring: P,
        ring_gpu: u64,
        ring_cpu: *mut u8,
        readback: P,
        color: P,
        #[allow(dead_code)]
        depth: P,
        ramp: P,
        white: P,
        rtv_heap: P,
        dsv_heap: P,
    }
    let r = step!("heaps, ring and targets", {
        let rtv_heap = heap(dev, DESCRIPTOR_HEAP_TYPE_RTV, 3, 0)?;
        let dsv_heap = heap(dev, DESCRIPTOR_HEAP_TYPE_DSV, 1, 0)?;
        // The two shader-visible heaps, as Display8's frame has them.
        let views = heap(dev, DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, 64, DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)?;
        let samplers = heap(dev, DESCRIPTOR_HEAP_TYPE_SAMPLER, 8, DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)?;
        let ring = buffer(dev, HEAP_TYPE_UPLOAD, RING, STATE_GENERIC_READ)?;
        let mut ptr: P = core::ptr::null_mut();
        let none = [0usize; 2];
        let hr = vcall!(ring, slot::RES_MAP, fn(u32, *const [usize; 2], *mut P) -> i32, 0, &none, &mut ptr);
        made(hr, ptr, "Map (upload ring)")?;
        let ring_gpu = vcall!(ring, slot::RES_GET_GPU_VIRTUAL_ADDRESS, fn() -> u64,);
        let readback = buffer(dev, HEAP_TYPE_READBACK, (W * H * 4) as u64, STATE_COPY_DEST)?;
        let color = texture(dev, W, H, FORMAT_B8G8R8A8_UNORM, RESOURCE_FLAG_ALLOW_RENDER_TARGET, STATE_RENDER_TARGET)?;
        let depth = texture(dev, W, H, FORMAT_D16_UNORM, RESOURCE_FLAG_ALLOW_DEPTH_STENCIL, STATE_DEPTH_WRITE)?;
        let ramp = texture(dev, 256, 1, FORMAT_B8G8R8A8_UNORM, 0, STATE_COPY_DEST)?;
        let white = texture(dev, 1, 1, FORMAT_B8G8R8A8_UNORM, 0, STATE_COPY_DEST)?;
        // Views: swap buffers 0 and 1, then the offscreen colour, in the RTV heap.
        let rtv0 = cpu_start(rtv_heap);
        for (i, b) in backs.iter().enumerate() {
            vcall!(dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const c_void, usize) -> (), *b, core::ptr::null(), rtv0 + i * inc[2] as usize);
        }
        vcall!(dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const c_void, usize) -> (), color, core::ptr::null(), rtv0 + 2 * inc[2] as usize);
        vcall!(dev, slot::DEV_CREATE_DEPTH_STENCIL_VIEW, fn(P, *const c_void, usize) -> (), depth, core::ptr::null(), cpu_start(dsv_heap));
        // The shader-visible heap: [0] colour and [1] ramp for the present pass, [2] and
        // [3] the white texel twice for the passthrough.
        let v0 = cpu_start(views);
        srv(dev, color, FORMAT_B8G8R8A8_UNORM, v0);
        srv(dev, ramp, FORMAT_B8G8R8A8_UNORM, v0 + inc[0] as usize);
        srv(dev, white, FORMAT_B8G8R8A8_UNORM, v0 + 2 * inc[0] as usize);
        srv(dev, white, FORMAT_B8G8R8A8_UNORM, v0 + 3 * inc[0] as usize);
        let s0 = cpu_start(samplers);
        let sd = SamplerDesc {
            filter: FILTER_MIN_MAG_MIP_POINT,
            address_u: TEXTURE_ADDRESS_MODE_CLAMP,
            address_v: TEXTURE_ADDRESS_MODE_CLAMP,
            address_w: TEXTURE_ADDRESS_MODE_CLAMP,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            comparison: COMPARISON_FUNC_ALWAYS,
            border: [0.0; 4],
            min_lod: 0.0,
            max_lod: 1000.0,
        };
        for i in 0..2 {
            vcall!(dev, slot::DEV_CREATE_SAMPLER, fn(*const SamplerDesc, usize) -> (), &sd, s0 + i * inc[1] as usize);
        }
        pass("heaps, ring and targets", "2 shader-visible heaps, an 8 MB upload ring mapped once, 640x480 B8G8R8A8 + D16");
        Ok(Res { views, samplers, ring, ring_gpu, ring_cpu: ptr as *mut u8, readback, color, depth, ramp, white, rtv_heap, dsv_heap })
    });

    section("Display8's root signatures and pipelines (dx12/build.rs)");
    let serialize = {
        let f = proc(d3d12, c"D3D12SerializeRootSignature");
        if f == 0 {
            return Err((String::from("root signatures"), String::from("d3d12.dll has no D3D12SerializeRootSignature")));
        }
        f
    };
    let say = |m: &str| out!("serializer: {}", m);
    let present_root = step!("present root signature", {
        let root = build::present_root(dev, serialize, &say).map_err(|e| fmt(format_args!("{}: {:#010x}", e.0, e.1 as u32)))?;
        pass("present root signature", "");
        Ok(root)
    });
    let present_pso = step!("present pipeline", {
        let (hr, pso) = build::present_desc(present_root, |pd| {
            let mut pso: P = core::ptr::null_mut();
            let hr = vcall!(dev, slot::DEV_CREATE_GRAPHICS_PIPELINE_STATE, fn(*const GraphicsPipelineDesc, *const crate::com::Guid, *mut P) -> i32, pd, &IID_PIPELINE_STATE, &mut pso);
            (hr, pso)
        });
        made(hr, pso, "CreateGraphicsPipelineState (present)")?;
        pass("present pipeline", "vs_present, ps_gamma");
        Ok(pso)
    });
    let pass_root = step!("passthrough root signature", {
        let root = build::pass_root(dev, serialize, &say).map_err(|e| fmt(format_args!("{}: {:#010x}", e.0, e.1 as u32)))?;
        pass("passthrough root signature", "5 parameters: 3 root CBVs, a table of t0-t1, a table of s0-s1");
        Ok(root)
    });
    // The keys the game makes most: an XYZ|DIFFUSE|TEX1 vertex (FVF 0x142), opaque and
    // blended, with and without depth, a line list, the Borg bump layout, and the
    // scene pass render3d publishes (half-float, D32, four samples).
    let fvf = state::layout(0x142).expect("0x142 is a fixed-function FVF");
    let bump = Layout { size: 56, bump: Some([0, 12, 20, 32, 44]), ..Default::default() };
    let tris = PipeState { topology: 4, sets: [0, 0], blend: [0, 5, 6, 1, 15], depth: [1, 1, 4], cull: 3 };
    let blended = PipeState { blend: [1, 5, 6, 1, 15], depth: [1, 0, 4], ..tris };
    let lines = PipeState { topology: 2, depth: [0, 0, 8], cull: 1, ..tris };
    let key = |layout: Layout, st: PipeState, color: u32, depth: u32, samples: u32| Key { color, depth, samples, stride: layout.size, layout, st, no_dst_alpha: false };
    let keys = [
        ("vs_ff opaque, B8G8R8A8 + D16", key(fvf, tris, FORMAT_B8G8R8A8_UNORM, FORMAT_D16_UNORM, 1)),
        ("vs_ff alpha-blended, B8G8R8A8 + D16", key(fvf, blended, FORMAT_B8G8R8A8_UNORM, FORMAT_D16_UNORM, 1)),
        ("vs_ff opaque, B8G8R8A8, no depth", key(fvf, PipeState { depth: [0, 0, 8], ..tris }, FORMAT_B8G8R8A8_UNORM, FORMAT_UNKNOWN, 1)),
        ("vs_ff line list", key(fvf, lines, FORMAT_B8G8R8A8_UNORM, FORMAT_D16_UNORM, 1)),
        ("vs_ff opaque, R16G16B16A16_FLOAT + D32 x4 (render3d's scene pass)", key(fvf, tris, FORMAT_R16G16B16A16_FLOAT, FORMAT_D32_FLOAT, 4)),
        ("vs_bump (Borg), B8G8R8A8 + D16", key(bump, tris, FORMAT_B8G8R8A8_UNORM, FORMAT_D16_UNORM, 1)),
        ("vs_bump (Borg), R16G16B16A16_FLOAT + D32 x4", key(bump, tris, FORMAT_R16G16B16A16_FLOAT, FORMAT_D32_FLOAT, 4)),
    ];
    let mut draw_pso: P = core::ptr::null_mut();
    for (name, k) in keys.iter() {
        begin(name);
        match build::pass_pipeline(dev, pass_root, k) {
            Ok(p) => {
                pass(name, "");
                if draw_pso.is_null() {
                    draw_pso = p;
                }
            }
            Err(hr) => return Err((String::from(*name), fmt(format_args!("CreateGraphicsPipelineState: {:#010x}", hr as u32)))),
        }
    }
    let _ = CB_SPACES;

    section("frames");
    // Per-frame objects: a command allocator and list for each frame in flight.
    let mut allocs = [core::ptr::null_mut(); SLOTS];
    let mut lists = [core::ptr::null_mut(); SLOTS];
    step!("command lists", {
        for s in 0..SLOTS {
            let mut al: P = core::ptr::null_mut();
            let hr = vcall!(dev, slot::DEV_CREATE_COMMAND_ALLOCATOR, fn(u32, *const crate::com::Guid, *mut P) -> i32, COMMAND_LIST_TYPE_DIRECT, &IID_COMMAND_ALLOCATOR, &mut al);
            allocs[s] = made(hr, al, "CreateCommandAllocator")?;
            let mut l: P = core::ptr::null_mut();
            let hr = vcall!(dev, slot::DEV_CREATE_COMMAND_LIST, fn(u32, u32, P, P, *const crate::com::Guid, *mut P) -> i32,
                0, COMMAND_LIST_TYPE_DIRECT, allocs[s], core::ptr::null_mut(), &IID_GRAPHICS_COMMAND_LIST, &mut l);
            lists[s] = made(hr, l, "CreateCommandList")?;
            vcall!(lists[s], slot::LIST_CLOSE, fn() -> i32,);
        }
        pass("command lists", "");
        Ok(())
    });

    begin("frames");
    let (views_gpu, samplers_gpu) = (gpu_start(r.views), gpu_start(r.samplers));
    let rtv0 = cpu_start(r.rtv_heap);
    let dsv = cpu_start(r.dsv_heap);
    let mut done = [0u64; SLOTS];
    let vp = Viewport { x: 0.0, y: 0.0, w: W as f32, h: H as f32, min_z: 0.0, max_z: 1.0 };
    let sc = Rect { left: 0, top: 0, right: W as i32, bottom: H as i32 };
    // The colour is cleared in two halves, so the check sees both and the split.
    let halves = [Rect { left: 0, top: 0, right: (W / 2) as i32, bottom: H as i32 }, Rect { left: (W / 2) as i32, top: 0, right: W as i32, bottom: H as i32 }];
    let mut ramp_state = STATE_COPY_DEST;
    let mut white_ready = false;
    for f in 0..FRAMES {
        if !user32::pump() {
            return Err((String::from("frames"), String::from("the window was closed")));
        }
        let s = (f as usize) % SLOTS;
        d.wait(done[s]).map_err(|e| (String::from("frames"), e))?;
        let (al, l) = (allocs[s], lists[s]);
        vcall!(al, slot::ALLOC_RESET, fn() -> i32,);
        vcall!(l, slot::LIST_RESET, fn(P, P) -> i32, al, core::ptr::null_mut());
        let heaps = [r.views, r.samplers];
        vcall!(l, slot::LIST_SET_DESCRIPTOR_HEAPS, fn(u32, *const P) -> (), 2, heaps.as_ptr());

        // The ring: this frame's half, from the start.
        let base = s as u64 * (RING / SLOTS as u64);
        let mut at = 0u64;
        let mut alloc = |bytes: u64, align: u64| -> (u64, *mut u8) {
            at = at.div_ceil(align) * align;
            let o = at;
            at += bytes;
            (r.ring_gpu + base + o, unsafe { r.ring_cpu.add((base + o) as usize) })
        };
        // The filler, written so that the whole ring gets touched as the frames go round.
        let (_, filler) = alloc(FILLER as u64, 256);
        unsafe { core::ptr::write_bytes(filler, f as u8, FILLER) };

        // Texture uploads: the identity ramp every frame, the white texel once. Each is a
        // footprint in the ring, 512-aligned, a row pitch of 256.
        let mut upload = |res: P, w: u32, bytes: &[u8], from: u32| {
            let (gpu, cpu) = alloc(1024, 512);
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), cpu, bytes.len()) };
            let off = gpu - r.ring_gpu;
            barrier(l, res, from, STATE_COPY_DEST);
            let dst = CopyLocation { resource: res, ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: 0, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
            let src = CopyLocation { resource: r.ring, ty: TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, offset: off, format: FORMAT_B8G8R8A8_UNORM, width: w, height: 1, depth: 1, row_pitch: 1024 };
            vcall!(l, slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const c_void) -> (), &dst, 0, 0, 0, &src, core::ptr::null());
            barrier(l, res, STATE_COPY_DEST, STATE_PIXEL_SHADER_RESOURCE);
        };
        let mut ramp = [0u8; 1024];
        for i in 0..256 {
            ramp[i * 4..i * 4 + 4].copy_from_slice(&[i as u8, i as u8, i as u8, 255]);
        }
        upload(r.ramp, 256, &ramp, ramp_state);
        ramp_state = STATE_PIXEL_SHADER_RESOURCE;
        if !white_ready {
            upload(r.white, 1, &[255, 255, 255, 255], STATE_COPY_DEST);
            white_ready = true;
        }

        // The passthrough's draw into the offscreen target: two halves cleared, then a
        // triangle through vs_ff with the engine's constant blocks (zeros here: its
        // vertices are all clipped; the root signature, tables, vertex streams, topology
        // and pipeline are what is judged).
        let (vgpu, vcpu) = alloc(3 * 24, 16);
        unsafe { core::ptr::write_bytes(vcpu, 0, 3 * 24) };
        let (dgpu, dcpu) = alloc(state::DEFAULTS_BYTES as u64, 16);
        unsafe { core::ptr::copy_nonoverlapping(state::defaults().as_ptr(), dcpu, state::DEFAULTS_BYTES) };
        let (ugpu, ucpu) = alloc(PS_AT as u64 + 256, 256);
        unsafe { core::ptr::write_bytes(ucpu, 0, PS_AT as usize + 256) };
        let rtv_color = rtv0 + 2 * inc[2] as usize;
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 1, &rtv_color, 0, &dsv);
        vcall!(l, slot::LIST_CLEAR_RENDER_TARGET_VIEW, fn(usize, *const [f32; 4], u32, *const Rect) -> (), rtv_color, &A, 1, &halves[0]);
        vcall!(l, slot::LIST_CLEAR_RENDER_TARGET_VIEW, fn(usize, *const [f32; 4], u32, *const Rect) -> (), rtv_color, &B, 1, &halves[1]);
        vcall!(l, slot::LIST_CLEAR_DEPTH_STENCIL_VIEW, fn(usize, u32, f32, u32, u32, *const Rect) -> (), dsv, CLEAR_FLAG_DEPTH, 1.0, 0, 1, &sc);
        vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport) -> (), 1, &vp);
        vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const Rect) -> (), 1, &sc);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), pass_root);
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), draw_pso);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 0, ugpu);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 1, ugpu);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CBV, fn(u32, u64) -> (), 2, ugpu + PS_AT as u64);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 3, views_gpu + 2 * inc[0] as u64);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 4, samplers_gpu);
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), 4);
        let vbs = [VertexBufferView { location: vgpu, size: 3 * 24, stride: 24 }, VertexBufferView { location: dgpu, size: state::DEFAULTS_BYTES as u32, stride: state::DEFAULTS_BYTES as u32 }];
        vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 0, 2, vbs.as_ptr());
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), 3, 1, 0, 0);

        // The present pass: the offscreen colour and the ramp through ps_gamma into the
        // back buffer, as Display8's present does.
        let bb = vcall!(swap, slot::SWAP_GET_CURRENT_BACK_BUFFER_INDEX, fn() -> u32,) as usize % 2;
        let target = backs[bb];
        let rtv = rtv0 + bb * inc[2] as usize;
        barrier(l, r.color, STATE_RENDER_TARGET, STATE_PIXEL_SHADER_RESOURCE);
        barrier(l, target, STATE_PRESENT, STATE_RENDER_TARGET);
        let quad: [f32; 16] = [-1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0];
        let (qgpu, qcpu) = alloc(64, 16);
        unsafe { core::ptr::copy_nonoverlapping(quad.as_ptr() as *const u8, qcpu, 64) };
        let qv = VertexBufferView { location: qgpu, size: 64, stride: 16 };
        let shift = [0f32; 4];
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 1, &rtv, 0, core::ptr::null());
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), present_root);
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), present_pso);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS, fn(u32, u32, *const f32, u32) -> (), 0, 4, shift.as_ptr(), 0);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), 1, views_gpu);
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 0, 1, &qv);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), 4, 1, 0, 0);
        if f == FRAMES - 1 {
            // The last frame's back buffer into the read-back buffer.
            barrier(l, target, STATE_RENDER_TARGET, STATE_COPY_SOURCE);
            let dst = CopyLocation { resource: r.readback, ty: TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, offset: 0, format: FORMAT_B8G8R8A8_UNORM, width: W, height: H, depth: 1, row_pitch: W * 4 };
            let src = CopyLocation { resource: target, ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: 0, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
            vcall!(l, slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const c_void) -> (), &dst, 0, 0, 0, &src, core::ptr::null());
            barrier(l, target, STATE_COPY_SOURCE, STATE_PRESENT);
        } else {
            barrier(l, target, STATE_RENDER_TARGET, STATE_PRESENT);
        }
        barrier(l, r.color, STATE_PIXEL_SHADER_RESOURCE, STATE_RENDER_TARGET);
        let hr = vcall!(l, slot::LIST_CLOSE, fn() -> i32,);
        if hr < 0 {
            return Err((String::from("frames"), fmt(format_args!("frame {}: the command list did not close: {:#010x}", f, hr as u32))));
        }
        let ls = [l];
        vcall!(d.queue, slot::QUEUE_EXECUTE_COMMAND_LISTS, fn(u32, *const P) -> (), 1, ls.as_ptr());
        let hr = vcall!(swap, slot::SWAP_PRESENT, fn(u32, u32) -> i32, 0, 0);
        if hr < 0 {
            let why = vcall!(dev, slot::DEV_GET_DEVICE_REMOVED_REASON, fn() -> i32,);
            return Err((String::from("frames"), fmt(format_args!("frame {}: Present {:#010x} (device removed reason {:#010x})", f, hr as u32, why as u32))));
        }
        done[s] = d.signal();
    }
    d.wait(d.value).map_err(|e| (String::from("frames"), e))?;
    let why = vcall!(dev, slot::DEV_GET_DEVICE_REMOVED_REASON, fn() -> i32,);
    if why < 0 {
        return Err((String::from("frames"), fmt(format_args!("the device was removed: {:#010x}", why as u32))));
    }
    pass("frames", &fmt(format_args!("{} frames, the ring reused by each slot", FRAMES)));

    section("pixels");
    begin("read-back");
    let mut ptr: P = core::ptr::null_mut();
    let range = [0usize, (W * H * 4) as usize];
    let hr = vcall!(r.readback, slot::RES_MAP, fn(u32, *const [usize; 2], *mut P) -> i32, 0, &range, &mut ptr);
    made(hr, ptr, "Map (read-back)").map_err(|e| (String::from("read-back"), e))?;
    let px = |x: u32, y: u32| -> [u8; 3] {
        let p = unsafe { core::slice::from_raw_parts((ptr as *const u8).add(((y * W + x) * 4) as usize), 4) };
        [p[2], p[1], p[0]]
    };
    let want = |c: &[f32; 4]| [(c[0] * 255.0 + 0.5) as i32, (c[1] * 255.0 + 0.5) as i32, (c[2] * 255.0 + 0.5) as i32];
    let points = [(10, 10, &A), (630, 10, &B), (10, 470, &A), (630, 470, &B), (319, 240, &A), (320, 240, &B), (160, 240, &A)];
    let mut wrong = 0;
    for (x, y, c) in points {
        let (got, w) = (px(x, y), want(c));
        let ok = (0..3).all(|i| (got[i] as i32 - w[i]).abs() <= 3);
        out!("pixel ({:3},{:3}): {:3} {:3} {:3}, wanted {:3} {:3} {:3}: {}", x, y, got[0], got[1], got[2], w[0], w[1], w[2], if ok { "ok" } else { "WRONG" });
        wrong += !ok as u32;
    }
    if wrong != 0 {
        return Err((String::from("read-back"), fmt(format_args!("{} of 7 pixels are not what the clears and the present pass must make", wrong))));
    }
    pass("read-back", "7 of 7 pixels");

    if debug_on {
        begin("debug layer messages");
        let (bad, warn) = messages(info);
        if bad > 0 {
            return Err((String::from("debug layer messages"), fmt(format_args!("{} error(s) from the debug layer (listed above), {} warning(s)", bad, warn))));
        }
        pass("debug layer messages", &fmt(format_args!("no errors, {} warning(s)", warn)));
    }
    user32::close(window);
    Ok(())
}
