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
//! **Versions** (display 0.12.0; ROADMAP Phase 4, "ABI handshake"). The contract has a
//! major and a minor version, `ABI_MAJOR` and `ABI_MINOR` below, which both sides build
//! against. `display8_abi(caller_major, caller_minor, caller, &major, &minor)` is how a
//! plugin learns Display8's: its name and signature never change, whatever the major.
//! The rule:
//!
//! - **A minor only adds.** New exports, new constants, and new fields at the end of a
//!   struct. Every `#[repr(C)]` struct here starts with `size`, its size in bytes, so
//!   that each side can tell how much of it the other knows: a struct one side hands out
//!   (`*const VkDevice8`) says its own size, and the reader takes the fields within it
//!   (`read`); a struct the caller hands in to be filled (`*mut VkImage8`) says how much
//!   space it has, and the writer fills no more than that and sets `size` to what it
//!   wrote (`write`). Whatever is past the other side's size is the field's default. A
//!   plugin uses a field or an export of minor N only when Display8's minor is N or more.
//!   A struct inside another (`VkPass8`'s images) never grows in a minor: that would
//!   move what comes after it.
//! - **Anything else is a new major**: a field removed, moved, retyped or given a new
//!   meaning; an export's signature or meaning changed; a rule between the recorders
//!   (layouts, states, heaps, who closes a pass) changed. The exports are named with
//!   their major (`display8v1_*` for major 1), so a plugin of another major finds none
//!   of them rather than misreading them, and `display8_abi` says why.
//!
//! A plugin whose major is not Display8's stands down (Direct3D 9 only, as with no
//! Display8; on Display8's own Vulkan or Direct3D 12 it draws nothing and the engine's
//! draws pass through), and both sides log both versions. The contract before 1.0 had
//! no handshake and unversioned names (`display8v1_backend`, `display8v1_vk`, ...): a plugin
//! of that time finds nothing here, and a plugin of now that finds those names and no
//! `display8_abi` is on a Display8 too old for it.
//!
//! The exports (`display8v1_*`, `extern "C"`), found with
//! `GetProcAddress(GetModuleHandleA("d3d8.dll"), name)`:
//!
//! | Export | Signature | What |
//! |---|---|---|
//! | `display8v1_backend` | `fn() -> u32` | `BACKEND_*`: what the plugins draw on; `BACKEND_NONE` until the stub's device is made (since display 0.7.0: `Backend=`'s order is tried at the engine's first `CreateDevice`, so nothing is said before), asked again then |
//! | `display8v1_vk` | `fn() -> *const VkDevice8` | the device, or null |
//! | `display8v1_vk_frame` | `fn() -> *const VkFrame8` | the frame now, begun at the first ask after a `Present` |
//! | `display8v1_vk_alloc` | `fn(bytes: u32, align: u32, out: *mut VkAlloc8) -> bool` | space in the frame's upload ring (host-visible, mapped, coherent; vertex, index, uniform and transfer-source usage) |
//! | `display8v1_vk_defer` | `fn(f: extern "C" fn(*mut c_void), arg: *mut c_void)` | `f(arg)` once the frame now is done on the GPU: destroying what it may still read |
//! | `display8v1_vk_target` | `fn(which: u32, out: *mut VkImage8) -> bool` | `TARGET_COLOR`, `TARGET_DEPTH`: the engine's back buffer and depth |
//! | `display8v1_vk_mirror` | `fn(tex8: *mut c_void, out: *mut VkImage8) -> bool` | an engine texture (an `IDirect3DTexture8` the engine made) as a sampled image, kept up to date by Display8 |
//! | `display8v1_vk_flush` | `fn() -> bool` | submits what the frame now has recorded and waits for it; the frame goes on in the same command buffer (begun again), its ring, pool and deferred calls untouched: for a read-back. Called with no render pass open |
//! | `display8v1_engine_viewport` | `fn(out: *mut [u32; 6]) -> bool` | the engine's viewport (`D3DVIEWPORT8`: x, y, w, h, min z, max z as f32 bits) |
//! | `display8v1_engine_transform` | `fn(state: u32, out: *mut [f32; 16]) -> bool` | the engine's transform (`D3DTRANSFORMSTATETYPE`), row-major as Direct3D sets it |
//! | `display8v1_vk_pass` | `fn(event: u32, pass: *const VkPass8)` | optional (display 0.10.0): a plugin's `Cmd` begins (`PASS_ENTER`) or ends (`PASS_LEAVE`), or ends with `pass` its pass now (`PASS_PUBLISH`, a plugin that publishes: render3d), every attachment in its attachment layout. Display8's passthrough draws the engine's draws into the pass published last (in this frame), and records none while a `Cmd` is alive |
//!
//! **Claims** (contract 1.1, display 0.13.0; ROADMAP Phase 4, "Claims"). Every backend,
//! forward mode included:
//!
//! | Export | Signature | What |
//! |---|---|---|
//! | `display8v1_register` | `fn(plugin: *const Plugin8) -> u32` | a plugin says who it is and hands Display8 its `claims` callback, at load; an id (1 and up) for `display8v1_claims`, 0 when refused. A module that registers again replaces its first registration |
//! | `display8v1_claims` | `fn(except: u32) -> u32` | the `CLAIM_*` every registered plugin but `except` (an id, or 0 for none) draws now, each callback asked as it is called |
//!
//! A claim says "this plugin draws this category now". Display8 asks every callback at
//! each of the engine's `BeginScene`s in stub mode and logs what changed; a category
//! nobody claims is the engine's, drawn by passthrough. A plugin claims only what it
//! draws now: a render3d module at 0, a UI part at 0, or a plugin that cannot draw (no
//! device, another contract: it never registers) claims nothing. Display8 drops the
//! engine's draws of a claimed category only where the engine draws it anyway under the
//! plugin's (`CLAIM_LOADING`, `CLAIM_CREDITS`, `CLAIM_CURSOR`: told by the scene they
//! are drawn in); the 3D categories, `CLAIM_HUD` and `CLAIM_SELECTION` are never dropped
//! by Display8, since the engine issues those draws only when no plugin draws the part
//! (render3d skips the engine's step; the UI skips a stock component), so what the
//! engine issues there is always drawn. Claims also carry what one plugin must know of
//! another's: render3d draws no selection bubbles while another plugin claims
//! `CLAIM_SELECTION` (the UI's `[HUD] SelectionBubbles=0`), in place of reading an
//! engine byte the UI patched.
//!
//! **Passthrough** (display/README.md, "Passthrough"): in stub mode Display8 draws the
//! engine's own Direct3D 8 draws on Vulkan, between the plugins' `Cmd`s, into the pass a
//! plugin published at its last `Cmd`'s end (render3d's scene pass while its body runs),
//! else into the engine's targets. Its render pass may be left open after a draw, and
//! is closed at the next call into Display8 that records or hands the frame over
//! (`display8v1_vk_frame`, `display8v1_vk_mirror`, `display8v1_vk_flush`, `display8v1_vk_pass`),
//! so a recorder still never finds one open.
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
//! from Display8's (`display8v1_dx12_descriptors`), writes or copies its views into them
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
//! Shaders are DXIL from the same sources (`tools/shaderc/hlsl-rs.sh`, `*_dxil.rs`): a
//! stage's cbuffers at b0 (vertex) and b1 (pixel), the k-th cbuffer of a source in
//! space k (each module's `CB_SPACES` says how many), all of them views of the stage's
//! one register file; textures at tN and samplers at sN of space 0
//! (`gpu/shaders/compat.hlsli`, `tools/shaderc/dxil-spaces.py`).
//!
//! | Export | Signature | What |
//! |---|---|---|
//! | `display8v1_dx12` | `fn() -> *const Dx12Device8` | the device, or null |
//! | `display8v1_dx12_frame` | `fn() -> *const Dx12Frame8` | the frame now, begun at the first ask after a `Present` |
//! | `display8v1_dx12_alloc` | `fn(bytes: u32, align: u32, out: *mut Dx12Alloc8) -> bool` | space in the frame's upload ring |
//! | `display8v1_dx12_descriptors` | `fn(heap: u32, count: u32, out: *mut Dx12Descriptors8) -> bool` | `count` contiguous descriptors of the frame in a shader-visible heap (`HEAP_CBV_SRV_UAV`, `HEAP_SAMPLER`), valid until the frame is done |
//! | `display8v1_dx12_defer` | `fn(f: extern "C" fn(*mut c_void), arg: *mut c_void)` | `f(arg)` once the frame now is done on the GPU |
//! | `display8v1_dx12_target` | `fn(which: u32, out: *mut Dx12Image8) -> bool` | `TARGET_COLOR`, `TARGET_DEPTH` |
//! | `display8v1_dx12_mirror` | `fn(tex8: *mut c_void, out: *mut Dx12Image8) -> bool` | an engine texture as a sampled texture, kept up to date by Display8 |
//! | `display8v1_dx12_flush` | `fn() -> bool` | submits what the frame now has recorded and waits for it; the frame goes on in the same command list (reset, its heaps set again), its ring, descriptors and deferred calls untouched: for a read-back (display 0.6.0) |
//!
//! `Dx12Device8::features`, `sampler_anisotropy` and `sampler_lod_bias` (display 0.6.0,
//! after the fields before them) are as `VkDevice8`'s: `PixelCentres=d3d9`, and the
//! samplers as the game's `dxvk.conf` has DXVK make them. A frame's descriptors come
//! back to a recorder only through `display8v1_dx12_descriptors`, so a set of views a
//! recorder binds again in the same frame may keep the range it was given.
//!
//! `display8v1_backend`, `display8v1_engine_viewport` and `display8v1_engine_transform` are
//! the same exports on every backend.
use core::ffi::c_void;

/// The contract's major version: a plugin and a Display8 of different majors do not
/// talk (above, "Versions"). 1: display 0.12.0, the first with the handshake.
pub const ABI_MAJOR: u32 = 1;
/// The contract's minor version: what of the major's additions this side knows. 1:
/// display 0.13.0, the claims (`display8v1_register`, `display8v1_claims`).
pub const ABI_MINOR: u32 = 1;

/// `display8v1_register` (contract 1.1).
pub type RegisterFn = extern "C" fn(plugin: *const Plugin8) -> u32;
/// `display8v1_claims` (contract 1.1).
pub type ClaimsFn = extern "C" fn(except: u32) -> u32;
/// A plugin's claims callback: the `CLAIM_*` it draws now.
pub type ClaimsCallback = extern "C" fn() -> u32;

/// What a plugin draws now (`Plugin8::claims`, above, "Claims"). The 3D categories are
/// render3d's modules; the 2D ones the UI's parts.
pub const CLAIM_SKY: u32 = 1 << 0;
pub const CLAIM_HULLS: u32 = 1 << 1;
/// Planets, moons, asteroids and nebulae (render3d's `world`).
pub const CLAIM_WORLD: u32 = 1 << 2;
pub const CLAIM_PARTICLES: u32 = 1 << 3;
/// Shields, explosions and the game features (render3d's `effects`).
pub const CLAIM_EFFECTS: u32 = 1 << 4;
pub const CLAIM_ORDNANCE: u32 = 1 << 5;
pub const CLAIM_SPECIALS: u32 = 1 << 6;
/// The fog of war, the shroud and the tactical grid (render3d's `[Overlays] Fog`).
pub const CLAIM_FOG: u32 = 1 << 7;
/// The 3D selection bubbles (`RenderSelectionEffects`): render3d's `[Overlays]
/// Selection`, or the UI's `[HUD] SelectionBubbles=0`, which marks the selection itself.
pub const CLAIM_SELECTION: u32 = 1 << 8;
/// The HUD's panels in a mission (the UI's `[HUD]`).
pub const CLAIM_HUD: u32 = 1 << 16;
/// The loading screen (the UI's `[Loading] Enable`).
pub const CLAIM_LOADING: u32 = 1 << 17;
/// The Credits (the UI's `[Menus] Credits`).
pub const CLAIM_CREDITS: u32 = 1 << 18;
/// The mouse cursor (the UI's `[UI] Cursor`).
pub const CLAIM_CURSOR: u32 = 1 << 19;

/// The claims' names, for the logs: (bit, name).
pub const CLAIM_NAMES: [(u32, &str); 13] = [
    (CLAIM_SKY, "sky"),
    (CLAIM_HULLS, "hulls"),
    (CLAIM_WORLD, "world"),
    (CLAIM_PARTICLES, "particles"),
    (CLAIM_EFFECTS, "effects"),
    (CLAIM_ORDNANCE, "ordnance"),
    (CLAIM_SPECIALS, "specials"),
    (CLAIM_FOG, "fog"),
    (CLAIM_SELECTION, "selection"),
    (CLAIM_HUD, "hud"),
    (CLAIM_LOADING, "loading"),
    (CLAIM_CREDITS, "credits"),
    (CLAIM_CURSOR, "cursor"),
];

/// A plugin, as it registers (`display8v1_register`, contract 1.1): handed in, read
/// by Display8 within its `size`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Plugin8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
    /// An address in the plugin's module (any function of it), by which Display8 names
    /// it in its log.
    pub caller: usize,
    /// The `CLAIM_*` it draws now, asked every frame; None: it claims nothing.
    pub claims: Option<ClaimsCallback>,
}

/// `display8_abi`: Display8's major and minor through `major` and `minor`. The caller
/// says its own (`caller_major`, `caller_minor`) and an address in its own module
/// (`caller`, any function of it), so that Display8 logs which plugin asked and with
/// what. Never changes.
pub type AbiFn = extern "C" fn(caller_major: u32, caller_minor: u32, caller: usize, major: *mut u32, minor: *mut u32);

/// A `#[repr(C)]` struct of the contract: it starts with `size: u32`, and its default
/// is all zero but for `size`, which is its own size.
pub unsafe trait Sized8: Copy + Default {}

macro_rules! sized8 {
    ($($t:ident),*) => {$(
        impl Default for $t {
            fn default() -> Self {
                // Every field is an integer, a float or a raw pointer: zero is valid.
                let mut v: $t = unsafe { core::mem::zeroed() };
                v.size = core::mem::size_of::<$t>() as u32;
                v
            }
        }
        unsafe impl Sized8 for $t {}
    )*};
}

/// A struct the other side handed out (`*const T`): the fields within the size it says,
/// the rest at their defaults, and `size` the part that is the other side's.
pub unsafe fn read<T: Sized8>(p: *const T) -> T {
    let mut v = T::default();
    let ours = core::mem::size_of::<T>();
    let theirs = unsafe { *(p as *const u32) } as usize;
    let n = theirs.min(ours);
    unsafe { core::ptr::copy_nonoverlapping(p as *const u8, &mut v as *mut T as *mut u8, n) };
    unsafe { *(&mut v as *mut T as *mut u32) = n as u32 };
    v
}

/// Fills a struct the caller handed in (`*mut T`, its `size` the space it has): no more
/// than that, and `size` set to what was written.
pub unsafe fn write<T: Sized8>(out: *mut T, v: &T) {
    let room = unsafe { *(out as *const u32) } as usize;
    let n = room.min(core::mem::size_of::<T>());
    if n < 4 {
        return;
    }
    unsafe { core::ptr::copy_nonoverlapping(v as *const T as *const u8, out as *mut u8, n) };
    unsafe { *(out as *mut u32) = n as u32 };
}

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

/// `display8v1_vk_pass`'s events: a plugin's `Cmd` begins; ends; ends with its pass now
/// published.
pub const PASS_ENTER: u32 = 0;
pub const PASS_LEAVE: u32 = 1;
pub const PASS_PUBLISH: u32 = 2;

/// The Vulkan device Display8 made. Dispatchable handles are pointers, the others
/// 64-bit, as Vulkan defines them on a 32-bit process.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDevice8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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
#[derive(Clone, Copy)]
pub struct VkAlloc8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
    pub buffer: u64,
    pub offset: u32,
    pub cpu: *mut u8,
}


/// An image Display8 owns: the engine's targets, a mirror.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VkImage8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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

/// A plugin's pass now, published at its `Cmd`'s end (`display8v1_vk_pass`): one colour
/// attachment and the depth, if any (`depth.image` 0), each a view of one layer, in its
/// attachment layout. Valid until the frame is presented: a plugin publishes again in
/// each frame.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VkPass8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
    pub color: VkImage8,
    pub depth: VkImage8,
}

/// The exports, looked up once.
#[derive(Clone, Copy)]
pub struct Display8 {
    /// Display8's contract version (`display8_abi`): the same major as ours, and the
    /// minor that says which of the major's additions it has.
    pub major: u32,
    pub minor: u32,
    /// `display8_version`, Display8's own version, for the logs.
    pub version: &'static str,
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
    /// Always there from contract 1.0 (an Option from when it was new, display 0.10.0).
    pub vk_pass: Option<extern "C" fn(u32, *const VkPass8)>,
    /// Contract 1.1 (display 0.13.0): None on a Display8 of minor 0.
    pub register: Option<RegisterFn>,
    pub claims: Option<ClaimsFn>,
}

/// Why `Display8::load` found nothing to draw through. Each is logged apart: no
/// Display8 at all is the ordinary forward chain, the others a mismatch of files.
#[derive(Clone, Copy, Debug)]
pub enum Refused {
    /// No Display8: the game's `d3d8.dll` is another chain's (crosire's d3d8to9, DXVK's,
    /// Wine's).
    NoDisplay8,
    /// A Display8 from before the handshake (display 0.11 and older, its contract
    /// unversioned): too old for this plugin. Its version, when it says one.
    TooOld(Option<&'static str>),
    /// A Display8 of another major: its major and minor, and its version.
    Major(u32, u32, Option<&'static str>),
    /// The same major, and an export the major has is missing (a broken build).
    Missing(&'static str),
}

/// A NUL-terminated string of Display8's (`display8_version`), which lives as long as
/// the DLL.
fn static_cstr(p: *const u8) -> Option<&'static str> {
    if p.is_null() {
        return None;
    }
    let mut n = 0;
    while n < 64 && unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, n) }).ok()
}

/// `display8_version` through `lookup`, if there is one.
pub fn display8_version(lookup: &impl Fn(&str) -> usize) -> Option<&'static str> {
    match lookup("display8_version") {
        0 => None,
        p => static_cstr(unsafe { core::mem::transmute::<usize, extern "C" fn() -> *const u8>(p) }()),
    }
}

impl Display8 {
    /// The exports, through `lookup` (the plugin's `GetProcAddress` on `d3d8.dll`;
    /// this crate imports nothing of its own), after the handshake: `display8_abi`
    /// told our version and `caller` (an address in the plugin's module, for Display8's
    /// log), and Display8's major the same as ours.
    pub fn load(lookup: impl Fn(&str) -> usize, caller: usize) -> Result<Display8, Refused> {
        let version = display8_version(&lookup);
        let abi = lookup("display8_abi");
        if abi == 0 {
            // Every Display8 before the handshake has `display8_version` (and, in stub
            // mode's time, `display8v1_backend`); another chain's d3d8.dll has neither.
            return Err(if version.is_some() || lookup("display8v1_backend") != 0 { Refused::TooOld(version) } else { Refused::NoDisplay8 });
        }
        let (mut major, mut minor) = (0u32, 0u32);
        let abi = unsafe { core::mem::transmute::<usize, AbiFn>(abi) };
        abi(ABI_MAJOR, ABI_MINOR, caller, &mut major, &mut minor);
        if major != ABI_MAJOR {
            return Err(Refused::Major(major, minor, version));
        }
        let f = |n: &'static str| match lookup(n) {
            0 => Err(Refused::Missing(n)),
            p => Ok(p),
        };
        unsafe {
            Ok(Display8 {
                major,
                minor,
                version: version.unwrap_or("?"),
                backend: core::mem::transmute::<usize, extern "C" fn() -> u32>(f("display8v1_backend")?),
                vk: core::mem::transmute::<usize, extern "C" fn() -> *const VkDevice8>(f("display8v1_vk")?),
                vk_frame: core::mem::transmute::<usize, extern "C" fn() -> *const VkFrame8>(f("display8v1_vk_frame")?),
                vk_alloc: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut VkAlloc8) -> bool>(f("display8v1_vk_alloc")?),
                vk_defer: core::mem::transmute::<usize, extern "C" fn(extern "C" fn(*mut c_void), *mut c_void)>(f("display8v1_vk_defer")?),
                vk_target: core::mem::transmute::<usize, extern "C" fn(u32, *mut VkImage8) -> bool>(f("display8v1_vk_target")?),
                vk_mirror: core::mem::transmute::<usize, extern "C" fn(*mut c_void, *mut VkImage8) -> bool>(f("display8v1_vk_mirror")?),
                vk_flush: core::mem::transmute::<usize, extern "C" fn() -> bool>(f("display8v1_vk_flush")?),
                engine_viewport: core::mem::transmute::<usize, extern "C" fn(*mut [u32; 6]) -> bool>(f("display8v1_engine_viewport")?),
                engine_transform: core::mem::transmute::<usize, extern "C" fn(u32, *mut [f32; 16]) -> bool>(f("display8v1_engine_transform")?),
                vk_pass: Some(core::mem::transmute::<usize, extern "C" fn(u32, *const VkPass8)>(f("display8v1_vk_pass")?)),
                register: if minor >= 1 { Some(core::mem::transmute::<usize, RegisterFn>(f("display8v1_register")?)) } else { None },
                claims: if minor >= 1 { Some(core::mem::transmute::<usize, ClaimsFn>(f("display8v1_claims")?)) } else { None },
            })
        }
    }
}

/// Shader-visible heaps a frame's descriptors come from (`display8v1_dx12_descriptors`).
pub const HEAP_CBV_SRV_UAV: u32 = 0;
pub const HEAP_SAMPLER: u32 = 1;

/// The Direct3D 12 device Display8 made. COM objects are pointers, not referenced:
/// valid as long as Display8 is loaded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dx12Device8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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
#[derive(Clone, Copy)]
pub struct Dx12Frame8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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
    /// Its size in bytes (above, "Versions").
    pub size: u32,
    /// `ID3D12Resource*`: an upload-heap buffer.
    pub resource: usize,
    pub offset: u32,
    /// The space's GPU virtual address (the buffer's plus `offset`).
    pub gpu: u64,
    pub cpu: *mut u8,
}


/// A frame's run of descriptors in a shader-visible heap.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dx12Descriptors8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
    /// `D3D12_CPU_DESCRIPTOR_HANDLE` of the first: where to write or copy them.
    pub cpu: usize,
    /// `D3D12_GPU_DESCRIPTOR_HANDLE` of the first: what a root table binds.
    pub gpu: u64,
    pub count: u32,
}

/// A texture Display8 owns: the engine's targets, a mirror.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dx12Image8 {
    /// Its size in bytes (above, "Versions").
    pub size: u32,
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
    /// The exports, through `lookup`, once `Display8::load` has shaken hands. None: one
    /// without Direct3D 12.
    pub fn load(lookup: impl Fn(&str) -> usize) -> Option<Display8Dx12> {
        let f = |n: &str| match lookup(n) {
            0 => None,
            p => Some(p),
        };
        unsafe {
            Some(Display8Dx12 {
                dx12: core::mem::transmute::<usize, extern "C" fn() -> *const Dx12Device8>(f("display8v1_dx12")?),
                frame: core::mem::transmute::<usize, extern "C" fn() -> *const Dx12Frame8>(f("display8v1_dx12_frame")?),
                alloc: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut Dx12Alloc8) -> bool>(f("display8v1_dx12_alloc")?),
                descriptors: core::mem::transmute::<usize, extern "C" fn(u32, u32, *mut Dx12Descriptors8) -> bool>(f("display8v1_dx12_descriptors")?),
                defer: core::mem::transmute::<usize, extern "C" fn(extern "C" fn(*mut c_void), *mut c_void)>(f("display8v1_dx12_defer")?),
                target: core::mem::transmute::<usize, extern "C" fn(u32, *mut Dx12Image8) -> bool>(f("display8v1_dx12_target")?),
                mirror: core::mem::transmute::<usize, extern "C" fn(*mut c_void, *mut Dx12Image8) -> bool>(f("display8v1_dx12_mirror")?),
                flush: f("display8v1_dx12_flush").map(|p| core::mem::transmute::<usize, extern "C" fn() -> bool>(p)),
            })
        }
    }
}

sized8!(Plugin8, VkDevice8, VkFrame8, VkAlloc8, VkImage8, Dx12Device8, Dx12Frame8, Dx12Alloc8, Dx12Descriptors8, Dx12Image8);

impl Default for VkPass8 {
    fn default() -> Self {
        VkPass8 { size: core::mem::size_of::<VkPass8>() as u32, color: VkImage8::default(), depth: VkImage8::default() }
    }
}
unsafe impl Sized8 for VkPass8 {}
