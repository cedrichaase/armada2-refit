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
//! **Direct3D 12** (phase 6) gets its own block here, shaped the same way.
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
