//! The display's side of a frame: what Display8 (`display/`, the game's `d3d8.dll` of
//! ours) owns and hands the plugins through plain C exports, so that render3d and the
//! UI, each linking this crate on its own, record into **one** device and **one**
//! frame (the display plan, "A frame"). Rust types have no stable layout across DLLs,
//! so everything here is `#[repr(C)]` and plain handles; this file is the contract, and
//! both sides build against it.
//!
//! **Vulkan** (Display8 `Mode=stub`, `Backend=vulkan`). Display8 makes the instance and
//! the device (Vulkan 1.1, one graphics+present queue, `VK_KHR_swapchain`), owns the
//! swap chain, an offscreen colour target and depth target that stand for the engine's
//! back buffer and depth surface, two frames in flight with a command buffer, a fence,
//! a descriptor pool and an upload ring each, and the pipeline cache. A plugin's `gpu`
//! records into the frame's command buffer, on the game's thread, in the order the
//! calls come (the engine's `Clear`, render3d's frame, the UI's), and Display8 ends,
//! submits and presents it at the engine's `Present`, after its last pass (the gamma
//! ramp, `display/shaders/present.hlsl`).
//!
//! Layouts between recorders: outside a `Cmd`, the colour target is in
//! `COLOR_ATTACHMENT_OPTIMAL` and the depth target in
//! `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`; a mirror is in `SHADER_READ_ONLY_OPTIMAL`. A
//! recorder that changes one puts it back before it returns control. No render pass is
//! open between recorders.
//!
//! The exports (`display8_*`, `extern "C"`), found with
//! `GetProcAddress(GetModuleHandleA("d3d8.dll"), name)`:
//!
//! | Export | Signature | What |
//! |---|---|---|
//! | `display8_backend` | `fn() -> u32` | `BACKEND_*`: what the plugins draw on |
//! | `display8_vk` | `fn() -> *const VkDevice8` | the device, or null |
//! | `display8_vk_frame` | `fn() -> *const VkFrame8` | the frame now, begun at the first ask after a `Present` |
//! | `display8_vk_alloc` | `fn(bytes: u32, align: u32, out: *mut VkAlloc8) -> bool` | space in the frame's upload ring (host-visible, mapped, coherent; vertex, index, uniform and transfer-source usage) |
//! | `display8_vk_defer` | `fn(f: extern "C" fn(*mut c_void), arg: *mut c_void)` | `f(arg)` once the frame now is done on the GPU: destroying what it may still read |
//! | `display8_vk_target` | `fn(which: u32, out: *mut VkImage8) -> bool` | `TARGET_COLOR`, `TARGET_DEPTH`: the engine's back buffer and depth |
//! | `display8_vk_mirror` | `fn(tex8: *mut c_void, out: *mut VkImage8) -> bool` | an engine texture (an `IDirect3DTexture8` the engine made) as a sampled image, kept up to date by Display8 |
//! | `display8_vk_flush` | `fn() -> bool` | submits what the frame now has recorded and waits for it; the frame goes on in the same command buffer (begun again), its ring, pool and deferred calls untouched: for a read-back. Called with no render pass open |
//! | `display8_engine_viewport` | `fn(out: *mut [u32; 6]) -> bool` | the engine's viewport (`D3DVIEWPORT8`: x, y, w, h, min z, max z as f32 bits) |
//! | `display8_engine_transform` | `fn(state: u32, out: *mut [f32; 16]) -> bool` | the engine's transform (`D3DTRANSFORMSTATETYPE`), row-major as Direct3D sets it |
//!
//! **Direct3D 9** (`Backend=d3d9`, and forward mode): none of the above; the plugins
//! reach the device as before (`QueryInterface(IID_IDirect3DDevice9)` on the engine's
//! device) and keep their own `Frame`s, which on Direct3D 9 are trivial.
//!
//! **Direct3D 12** (Display8 `Mode=stub`, `Backend=d3d12`), shaped the same way. Display8
//! makes the device (feature level 11_0, the default adapter) and one direct queue, owns
//! a flip-model swap chain on the game's window, the colour and depth targets that
//! stand for the engine's back buffer and depth surface, two frames in flight with a
//! command allocator, a command list, a fence value and an upload ring each, and two
//! **shader-visible descriptor heaps** (CBV/SRV/UAV and sampler), which it sets on the
//! frame's command list when the frame begins. Only one heap of each type can be set at
//! a time, so a recorder never sets heaps of its own: it takes a frame's descriptors
//! from Display8's (`display8_dx12_descriptors`), writes or copies its views into them
//! (a texture's view lives in a heap of the recorder's own that is not shader-visible,
//! and is copied in with `CopyDescriptorsSimple`), and binds them with its own root
//! signature. Root signatures and pipeline states are each recorder's, set by it
//! before it draws; nothing of them is assumed between recorders.
//!
//! States between recorders: the colour target in `RENDER_TARGET`, the depth target in
//! `DEPTH_WRITE`, a mirror in `PIXEL_SHADER_RESOURCE | NON_PIXEL_SHADER_RESOURCE`; a
//! recorder that changes one puts it back before it returns control. The upload ring is
//! in an upload heap (`GENERIC_READ`), so a span of it can be a vertex, index or
//! constant buffer (at `uniform_align`, 256) or a copy source as it is.
//!
//! Shaders are DXIL from the same sources (`render3d/hlsl-rs.sh`, `*_dxil.rs`): a
//! stage's cbuffers at b0 (vertex) and b1 (pixel), the k-th cbuffer of a source in
//! space k (each module's `CB_SPACES` says how many), all of them views of the stage's
//! one register file; textures at tN and samplers at sN of space 0
//! (`gpu/shaders/compat.hlsli`, `render3d/dxil-spaces.py`).
//!
//! | Export | Signature | What |
//! |---|---|---|
//! | `display8_dx12` | `fn() -> *const Dx12Device8` | the device, or null |
//! | `display8_dx12_frame` | `fn() -> *const Dx12Frame8` | the frame now, begun at the first ask after a `Present` |
//! | `display8_dx12_alloc` | `fn(bytes: u32, align: u32, out: *mut Dx12Alloc8) -> bool` | space in the frame's upload ring |
//! | `display8_dx12_descriptors` | `fn(heap: u32, count: u32, out: *mut Dx12Descriptors8) -> bool` | `count` contiguous descriptors of the frame in a shader-visible heap (`HEAP_CBV_SRV_UAV`, `HEAP_SAMPLER`), valid until the frame is done |
//! | `display8_dx12_defer` | `fn(f: extern "C" fn(*mut c_void), arg: *mut c_void)` | `f(arg)` once the frame now is done on the GPU |
//! | `display8_dx12_target` | `fn(which: u32, out: *mut Dx12Image8) -> bool` | `TARGET_COLOR`, `TARGET_DEPTH` |
//! | `display8_dx12_mirror` | `fn(tex8: *mut c_void, out: *mut Dx12Image8) -> bool` | an engine texture as a sampled texture, kept up to date by Display8 |
//! | `display8_dx12_flush` | `fn() -> bool` | submits what the frame now has recorded and waits for it; the frame goes on in the same command list (reset, its heaps set again), its ring, descriptors and deferred calls untouched: for a read-back (display 0.6.0) |
//!
//! `Dx12Device8::features`, `sampler_anisotropy` and `sampler_lod_bias` (display 0.6.0,
//! after the fields before them) are as `VkDevice8`'s: `PixelCentres=d3d9`, and the
//! samplers as the game's `dxvk.conf` has DXVK make them. A frame's descriptors come
//! back to a recorder only through `display8_dx12_descriptors`, so a set of views a
//! recorder binds again in the same frame may keep the range it was given.
//!
//! `display8_backend`, `display8_engine_viewport` and `display8_engine_transform` are
//! the same exports on every backend.
use core::ffi::c_void;

pub const BACKEND_NONE: u32 = 0;
pub const BACKEND_D3D9: u32 = 1;
pub const BACKEND_VULKAN: u32 = 2;
pub const BACKEND_D3D12: u32 = 3;

/// `VkDevice8::features`: `VK_KHR_dynamic_rendering` (or Vulkan 1.3) is enabled; else
/// render passes (gpu uses render passes either way).
pub const FEATURE_DYNAMIC_RENDERING: u32 = 1;
/// Direct3D 9's pixel centres: the plugins draw as on Direct3D 9 (gpu's viewport moved
/// by DXVK's offset, `Gpu::half_pixel` 0.5), for comparing with it pixel for pixel
/// (Display8.ini `PixelCentres=d3d9`).
pub const FEATURE_D3D9_CENTRES: u32 = 2;
/// `samplerAnisotropy` is enabled.
pub const FEATURE_ANISOTROPY: u32 = 4;

pub const TARGET_COLOR: u32 = 0;
pub const TARGET_DEPTH: u32 = 1;

/// The Vulkan device Display8 made. Dispatchable handles are pointers, the others
/// 64-bit, as Vulkan defines them on a 32-bit process.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDevice8 {
    /// `PFN_vkGetInstanceProcAddr` of the loader Display8 opened (`vulkan-1.dll`).
    pub get_instance_proc_addr: usize,
    pub instance: usize,
    pub physical_device: usize,
    pub device: usize,
    pub queue: usize,
    pub queue_family: u32,
    /// `VK_MAKE_API_VERSION` of the device.
    pub api_version: u32,
    pub pipeline_cache: u64,
    pub frames_in_flight: u32,
    /// `FEATURE_*`.
    pub features: u32,
    /// `VkPhysicalDeviceLimits.minUniformBufferOffsetAlignment`.
    pub uniform_align: u32,
    /// The anisotropy every linear sampler is given, as DXVK's `d3d9.samplerAnisotropy`
    /// in the game's `dxvk.conf` gives it on Direct3D 9 (renderer/README.md); -1: each
    /// sampler as it asks.
    pub sampler_anisotropy: i32,
    /// The LOD bias of every sampler with mips (`d3d9.samplerLodBias`, held at zero or
    /// above when `d3d9.clampNegativeLodBias` says so).
    pub sampler_lod_bias: f32,
}

/// The frame now.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkFrame8 {
    pub index: u64,
    /// Which of the frames in flight (0 .. frames_in_flight): what per-frame state a
    /// recorder keeps of its own is kept per slot.
    pub slot: u32,
    pub command_buffer: usize,
    /// Reset at the start of this frame; sets from it live until the frame is done.
    pub descriptor_pool: u64,
}

/// Space in the frame's upload ring.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkAlloc8 {
    pub buffer: u64,
    pub offset: u32,
    pub cpu: *mut u8,
}

impl Default for VkFrame8 {
    fn default() -> Self {
        VkFrame8 { index: 0, slot: 0, command_buffer: 0, descriptor_pool: 0 }
    }
}

/// An image Display8 owns: the engine's targets, a mirror.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkImage8 {
    pub image: u64,
    pub view: u64,
    /// `VkFormat`.
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    /// The layout it is in between recorders (above).
    pub layout: u32,
}

/// The exports, looked up once.
#[derive(Clone, Copy)]
pub struct Display8 {
    pub backend: extern "C" fn() -> u32,
    pub vk: extern "C" fn() -> *const VkDevice8,
    pub vk_frame: extern "C" fn() -> *const VkFrame8,
    pub vk_alloc: extern "C" fn(u32, u32, *mut VkAlloc8) -> bool,
    pub vk_defer: extern "C" fn(extern "C" fn(*mut c_void), *mut c_void),
    pub vk_target: extern "C" fn(u32, *mut VkImage8) -> bool,
    pub vk_mirror: extern "C" fn(*mut c_void, *mut VkImage8) -> bool,
    pub vk_flush: extern "C" fn() -> bool,
    pub engine_viewport: extern "C" fn(*mut [u32; 6]) -> bool,
    pub engine_transform: extern "C" fn(u32, *mut [f32; 16]) -> bool,
}

impl Display8 {
    /// The exports, through `lookup` (the plugin's `GetProcAddress` on `d3d8.dll`;
    /// this crate imports nothing of its own). None: not Display8, or an older one.
    pub fn load(lookup: impl Fn(&str) -> usize) -> Option<Display8> {
        let f = |n: &str| match lookup(n) {
            0 => None,
            p => Some(p),
        };
        unsafe {
            Some(Display8 {
                backend: core::mem::transmute::<usize, extern "C" fn() -> u32>(f("display8_backend")?),
                vk: core::mem::transmute::<usize, extern "C" fn() -> *const VkDevice8>(f("display8_vk")?),
                vk_frame: core::mem::transmute::<usize, extern "C" fn() -> *const VkFrame8>(f("display8_vk_frame")?),
                vk_alloc: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut VkAlloc8) -> bool>(f("display8_vk_alloc")?),
                vk_defer: core::mem::transmute::<usize, extern "C" fn(extern "C" fn(*mut c_void), *mut c_void)>(f("display8_vk_defer")?),
                vk_target: core::mem::transmute::<usize, extern "C" fn(u32, *mut VkImage8) -> bool>(f("display8_vk_target")?),
                vk_mirror: core::mem::transmute::<usize, extern "C" fn(*mut c_void, *mut VkImage8) -> bool>(f("display8_vk_mirror")?),
                vk_flush: core::mem::transmute::<usize, extern "C" fn() -> bool>(f("display8_vk_flush")?),
                engine_viewport: core::mem::transmute::<usize, extern "C" fn(*mut [u32; 6]) -> bool>(f("display8_engine_viewport")?),
                engine_transform: core::mem::transmute::<usize, extern "C" fn(u32, *mut [f32; 16]) -> bool>(f("display8_engine_transform")?),
            })
        }
    }
}

/// Shader-visible heaps a frame's descriptors come from (`display8_dx12_descriptors`).
pub const HEAP_CBV_SRV_UAV: u32 = 0;
pub const HEAP_SAMPLER: u32 = 1;

/// The Direct3D 12 device Display8 made. COM objects are pointers, not referenced:
/// valid as long as Display8 is loaded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dx12Device8 {
    /// `ID3D12Device*`.
    pub device: usize,
    /// `ID3D12CommandQueue*`: the direct queue Display8 submits the frame on.
    pub queue: usize,
    /// `D3D12SerializeRootSignature` of the `d3d12.dll` Display8 loaded.
    pub serialize_root_signature: usize,
    /// `D3D_FEATURE_LEVEL` the device was made at.
    pub feature_level: u32,
    pub frames_in_flight: u32,
    /// `GetDescriptorHandleIncrementSize` of CBV/SRV/UAV, sampler, RTV and DSV.
    pub increments: [u32; 4],
    /// The shader-visible heaps the frame's command list has set
    /// (`ID3D12DescriptorHeap*`): CBV/SRV/UAV, sampler.
    pub heaps: [usize; 2],
    /// Constant-buffer placement alignment (256).
    pub uniform_align: u32,
    /// `FEATURE_*` (as `VkDevice8::features`; added in display 0.6.0).
    pub features: u32,
    /// The anisotropy every linear sampler is given, as `VkDevice8::sampler_anisotropy`.
    pub sampler_anisotropy: i32,
    /// The LOD bias of every sampler with mips, as `VkDevice8::sampler_lod_bias`.
    pub sampler_lod_bias: f32,
}

/// The frame now.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Dx12Frame8 {
    pub index: u64,
    /// Which of the frames in flight (0 .. frames_in_flight).
    pub slot: u32,
    /// `ID3D12GraphicsCommandList*`, open, with Display8's heaps set.
    pub command_list: usize,
}

/// Space in the frame's upload ring.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dx12Alloc8 {
    /// `ID3D12Resource*`: an upload-heap buffer.
    pub resource: usize,
    pub offset: u32,
    /// The space's GPU virtual address (the buffer's plus `offset`).
    pub gpu: u64,
    pub cpu: *mut u8,
}

impl Default for Dx12Alloc8 {
    fn default() -> Self {
        Dx12Alloc8 { resource: 0, offset: 0, gpu: 0, cpu: core::ptr::null_mut() }
    }
}

/// A frame's run of descriptors in a shader-visible heap.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Dx12Descriptors8 {
    /// `D3D12_CPU_DESCRIPTOR_HANDLE` of the first: where to write or copy them.
    pub cpu: usize,
    /// `D3D12_GPU_DESCRIPTOR_HANDLE` of the first: what a root table binds.
    pub gpu: u64,
    pub count: u32,
}

/// A texture Display8 owns: the engine's targets, a mirror.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Dx12Image8 {
    /// `ID3D12Resource*`.
    pub resource: usize,
    /// `DXGI_FORMAT`.
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    pub samples: u32,
    /// `D3D12_RESOURCE_STATES` it is in between recorders (above).
    pub state: u32,
    /// Its render-target view (the colour target), or 0.
    pub rtv: usize,
    /// Its depth-stencil view (the depth target), or 0.
    pub dsv: usize,
    /// A shader-resource view in a heap of Display8's that is not shader-visible (the
    /// colour target, a mirror; 0 for the depth target), to copy into a frame's
    /// descriptors.
    pub srv: usize,
}

/// The Direct3D 12 exports, looked up once; beside `Display8`, which holds the ones
/// every backend has.
#[derive(Clone, Copy)]
pub struct Display8Dx12 {
    pub dx12: extern "C" fn() -> *const Dx12Device8,
    pub frame: extern "C" fn() -> *const Dx12Frame8,
    pub alloc: extern "C" fn(u32, u32, *mut Dx12Alloc8) -> bool,
    pub descriptors: extern "C" fn(u32, u32, *mut Dx12Descriptors8) -> bool,
    pub defer: extern "C" fn(extern "C" fn(*mut c_void), *mut c_void),
    pub target: extern "C" fn(u32, *mut Dx12Image8) -> bool,
    pub mirror: extern "C" fn(*mut c_void, *mut Dx12Image8) -> bool,
    /// None: a Display8 before 0.6.0, which reads nothing back.
    pub flush: Option<extern "C" fn() -> bool>,
}

impl Display8Dx12 {
    /// The exports, through `lookup` (as `Display8::load`). None: not Display8, or one
    /// without Direct3D 12.
    pub fn load(lookup: impl Fn(&str) -> usize) -> Option<Display8Dx12> {
        let f = |n: &str| match lookup(n) {
            0 => None,
            p => Some(p),
        };
        unsafe {
            Some(Display8Dx12 {
                dx12: core::mem::transmute::<usize, extern "C" fn() -> *const Dx12Device8>(f("display8_dx12")?),
                frame: core::mem::transmute::<usize, extern "C" fn() -> *const Dx12Frame8>(f("display8_dx12_frame")?),
                alloc: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut Dx12Alloc8) -> bool>(f("display8_dx12_alloc")?),
                descriptors: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut Dx12Descriptors8) -> bool>(f("display8_dx12_descriptors")?),
                defer: core::mem::transmute::<usize, extern "C" fn(extern "C" fn(*mut c_void), *mut c_void)>(f("display8_dx12_defer")?),
                target: core::mem::transmute::<usize, extern "C" fn(u32, *mut Dx12Image8) -> bool>(f("display8_dx12_target")?),
                mirror: core::mem::transmute::<usize, extern "C" fn(*mut c_void, *mut Dx12Image8) -> bool>(f("display8_dx12_mirror")?),
                flush: f("display8_dx12_flush").map(|p| core::mem::transmute::<usize, extern "C" fn() -> bool>(p)),
            })
        }
    }
}
