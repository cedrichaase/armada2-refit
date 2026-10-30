//! The Vulkan backend: the interface on the device Display8 (`display/`, the game's
//! `d3d8.dll` of ours) makes in stub mode with `Backend=vulkan`, reached through the
//! exports of `display.rs` (the contract). Display8 owns the instance, the device, the
//! queue, the swap chain, the engine's back buffer and depth (images of its own), and
//! the frame: its command buffer, descriptor pool and upload ring. This backend records
//! into that command buffer, on the game's thread, in the order the calls come, and
//! Display8 submits it at the engine's `Present`. README.md beside this crate, "The
//! Vulkan backend", says how each part of the interface maps and why.
//!
//! - **Recorded, not immediate**, but in the order Direct3D 9 would have applied it:
//!   a pass begun stays the pass until another is begun (a `Cmd` drawing with no pass
//!   of its own draws into the last one, as the Direct3D 9 backend's targets stay
//!   bound), and its render pass is opened at the first draw and closed whenever
//!   something must happen outside one (an upload, a layout change, a copy) and at the
//!   `Cmd`'s end, so no render pass is ever open between recorders.
//! - **Layouts** are tracked per image, the whole image in one: what it was last used
//!   as. An image Display8 owns is put back in its layout (`VkImage8::layout`) when the
//!   `Cmd` that changed it ends.
//! - **Uniform blocks** are a shadow of each stage's Direct3D 9 register file per
//!   `Cmd`, uploaded when changed into a ring of this backend's own and bound as dynamic
//!   uniform buffers (set 0); textures with their samplers are combined image samplers
//!   (set 1, binding = the Direct3D 9 stage), a set from the frame's pool when they
//!   change.
//! - **Nothing is destroyed while a frame may still use it**: what is let go of waits
//!   until Display8 has begun the frame two after (its fence has passed).
pub mod spirv;
#[rustfmt::skip]
#[allow(clippy::all)]
mod shaders;
mod cmd;
mod mem;
mod pipe;
mod tex;

pub use cmd::Cmd;
pub(crate) use cmd::gpu_engine_viewport as engine_viewport;
pub use pipe::{ComputePipeline, Pipeline};
pub use tex::{Buffer, Storage, Texture, Timer};

use crate::display::{Display8, VkFrame8, VkImage8};
use crate::types::*;
use alloc::rc::{Rc, Weak};
use alloc::vec::Vec;
use ash::vk;
use core::cell::{Cell, RefCell};
use mem::{Alloc, Pools};

pub type Log = fn(core::fmt::Arguments);

macro_rules! vlog {
    ($g:expr, $($t:tt)*) => { ($g.log)(format_args!($($t)*)) };
}
pub(crate) use vlog;

/// The shader stages' register files, as Direct3D 9 has them: 256 vertex shader float4
/// registers and 224 pixel shader ones. Each stage's uniform buffer is its file from
/// c0, so these are the most bytes one is bound with.
pub(crate) const VS_REGS: usize = 256;
pub(crate) const PS_REGS: usize = 224;
const VS_RANGE: u64 = VS_REGS as u64 * 16;
const PS_RANGE: u64 = PS_REGS as u64 * 16;
const UBO_CHUNK: u64 = 1 << 20;
pub(crate) const SLOTS: usize = 16;

use crate::display::{FEATURE_ANISOTROPY, FEATURE_D3D9_CENTRES};

/// What waits until the GPU is done with the frame it was last used in.
pub(crate) enum Garbage {
    Image(vk::Image),
    View(vk::ImageView),
    Buffer(vk::Buffer),
    Memory(Alloc),
    Framebuffer(vk::Framebuffer),
    Pipeline(vk::Pipeline),
    Module(vk::ShaderModule),
    Query(vk::QueryPool),
}

/// The device's functions and what every resource needs to let go of itself.
pub(crate) struct Shared {
    pub disp: Display8,
    pub dev: ash::Device,
    pub inst: ash::Instance,
    pub pdev: vk::PhysicalDevice,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
    pub limits: vk::PhysicalDeviceLimits,
    pub fif: u64,
    pub features: u32,
    /// What DXVK's `dxvk.conf` makes of every sampler (`VkDevice8::sampler_anisotropy`,
    /// `sampler_lod_bias`): applied to ours, so the pictures agree with Direct3D 9's.
    pub force_aniso: i32,
    pub lod_bias: f32,
    pub cache: vk::PipelineCache,
    pub pools: RefCell<Pools>,
    garbage: RefCell<Vec<(u64, Garbage)>>,
    /// The frame recorded last (Display8's index).
    pub now: Cell<u64>,
}

impl Shared {
    pub fn retire(&self, g: Garbage) {
        self.garbage.borrow_mut().push((self.now.get(), g));
    }

    fn destroy(&self, g: Garbage) {
        unsafe {
            match g {
                Garbage::Image(i) => self.dev.destroy_image(i, None),
                Garbage::View(v) => self.dev.destroy_image_view(v, None),
                Garbage::Buffer(b) => self.dev.destroy_buffer(b, None),
                Garbage::Memory(a) => self.pools.borrow_mut().free(&self.dev, a),
                Garbage::Framebuffer(f) => self.dev.destroy_framebuffer(f, None),
                Garbage::Pipeline(p) => self.dev.destroy_pipeline(p, None),
                Garbage::Module(m) => self.dev.destroy_shader_module(m, None),
                Garbage::Query(q) => self.dev.destroy_query_pool(q, None),
            }
        }
    }

    /// Frame `now` is begun: whatever was let go of two frames before is done with.
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
            self.destroy(x);
        }
    }

    /// Memory of `req` with `want`, from a pool (`image`: images and buffers are kept
    /// apart, for `bufferImageGranularity`).
    pub fn alloc(&self, req: vk::MemoryRequirements, want: vk::MemoryPropertyFlags, image: bool) -> Option<Alloc> {
        let ty = (0..self.mem_props.memory_type_count)
            .find(|&i| req.memory_type_bits & (1 << i) != 0 && self.mem_props.memory_types[i as usize].property_flags.contains(want))?;
        self.pools.borrow_mut().alloc(&self.dev, ty, req.size, req.alignment, image, want.contains(vk::MemoryPropertyFlags::HOST_VISIBLE))
    }

    pub fn format_features(&self, f: vk::Format) -> vk::FormatFeatureFlags {
        unsafe { self.inst.get_physical_device_format_properties(self.pdev, f) }.optimal_tiling_features
    }
}

/// A uniform chunk: a host-visible buffer, mapped, with its set (both stages' dynamic
/// uniform buffers in it).
struct UboChunk {
    _buf: vk::Buffer,
    _mem: Alloc,
    cpu: *mut u8,
    set: vk::DescriptorSet,
}

/// One frame slot's uniform chunks, and how far they are used in the frame now.
#[derive(Default)]
struct UboRing {
    chunks: Vec<UboChunk>,
    at: (usize, u64),
    frame: u64,
}

/// A colour or depth attachment of the pass now.
#[derive(Clone)]
pub(crate) struct Att {
    pub tex: Texture,
    pub layer: u32,
}

/// The pass now: begun by `Cmd::begin_pass`, kept until another is begun (the targets
/// "stay bound").
#[derive(Clone)]
pub(crate) struct PassState {
    pub colors: Vec<Att>,
    pub depth: Option<Att>,
    pub size: (u32, u32),
    pub key: PassKey,
}

/// What a render pass is compatible by: the attachments' formats and samples.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PassKey {
    pub colors: [vk::Format; MAX_COLORS],
    pub ncolors: u32,
    pub depth: vk::Format,
    pub samples: u32,
}

/// A render pass open in the command buffer now.
pub(crate) struct Open {
    pub key: PassKey,
}

pub(crate) struct State {
    pub sh: Rc<Shared>,
    pub set0: vk::DescriptorSetLayout,
    pub set1: vk::DescriptorSetLayout,
    /// Set 2: the read-only storage buffers (every stage); set 3: the read-write ones
    /// and the storage images (compute).
    pub set2: vk::DescriptorSetLayout,
    pub set3: vk::DescriptorSetLayout,
    /// Graphics: sets 0..2. Compute: sets 0..3.
    pub layout: vk::PipelineLayout,
    pub clayout: vk::PipelineLayout,
    ubo_pool: vk::DescriptorPool,
    /// Pools of the backend's own for the storage sets (Display8's frame pool has no
    /// storage descriptors), one per frame slot, reset when its slot comes round: (the
    /// frame it was last reset for, the pool).
    spools: Vec<(u64, vk::DescriptorPool)>,
    /// A small storage buffer bound where a slot a shader reads has nothing.
    pub dummy_storage: Option<Storage>,
    samplers: Vec<(Sampler, vk::Sampler)>,
    /// Render passes by (compatibility, loads: 2 bits each, colours then depth).
    passes: Vec<((PassKey, u32), vk::RenderPass)>,
    /// Placeholders bound where a pipeline reads a texture nothing is bound to: 2D,
    /// volume, cube; (0, 0, 0, 0) as DXVK's empty slots read.
    pub dummies: [Option<Texture>; 3],
    pub dummy_vb: Option<Buffer>,
    ubo: Vec<UboRing>,
    pub frame: VkFrame8,
    pub has_frame: bool,
    pub open: Option<Open>,
    pub pass: Option<PassState>,
    /// The images of Display8's wrapped, by image (one wrapper each while it lives).
    externals: Vec<(u64, Weak<tex::TexInner>)>,
    /// Display8's images whose layout a `Cmd` changed: put back at its end.
    pub touched: Vec<Texture>,
    /// `resolve_depth`'s pipelines, by (multisampled, depth format).
    pub depth_pipes: Vec<((bool, vk::Format), vk::Pipeline)>,
    pub depth_mods: Option<(vk::ShaderModule, vk::ShaderModule, vk::ShaderModule)>,
    pub told: u32,
}

/// The backend's state for one plugin (as `d3d9::Gpu`): made at the first use once
/// Display8 has said the plugins draw on Vulkan.
pub struct Gpu {
    log: Log,
    st: RefCell<Option<State>>,
    /// Moves on at each `Cmd`'s start and end: a `Cmd` that sees another value binds
    /// everything again (another recorder may have bound its own since).
    serial: Cell<u32>,
    depth: Cell<u32>,
    /// The pass now is published to Display8 at each outermost `Cmd`'s end, for its
    /// passthrough (`display8v1_vk_pass`; render3d's, whose scene pass the engine's draws
    /// it hands back belong in).
    publish: Cell<bool>,
}

/// Whether a format of this interface is a depth one, and its Vulkan format on this
/// device (`D24` where the device has none: `D32_SFLOAT`, as DXVK does).
pub(crate) fn vk_format(sh: &Shared, f: Format) -> vk::Format {
    use vk::Format as F;
    match f {
        Format::Unknown => F::UNDEFINED,
        Format::Bgra8 | Format::Bgrx8 => F::B8G8R8A8_UNORM,
        Format::B5G6R5 => F::R5G6B5_UNORM_PACK16,
        Format::Bgr10A2 => F::A2R10G10B10_UNORM_PACK32,
        Format::Rgb10A2 => F::A2B10G10R10_UNORM_PACK32,
        Format::R8 | Format::A8 => F::R8_UNORM,
        Format::R16F => F::R16_SFLOAT,
        Format::Rg16F => F::R16G16_SFLOAT,
        Format::Rgba16F => F::R16G16B16A16_SFLOAT,
        Format::R32F => F::R32_SFLOAT,
        Format::Rg32F => F::R32G32_SFLOAT,
        Format::Rgba32F => F::R32G32B32A32_SFLOAT,
        Format::D16 | Format::D15S1 => F::D16_UNORM,
        Format::D32 => F::D32_SFLOAT,
        Format::D24S8 | Format::D24X4S4 | Format::D24X8 => {
            let want = if f == Format::D24X8 { F::X8_D24_UNORM_PACK32 } else { F::D24_UNORM_S8_UINT };
            let ok = |v| sh.format_features(v).contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT | vk::FormatFeatureFlags::SAMPLED_IMAGE);
            if ok(want) {
                want
            } else if f != Format::D24X8 && ok(F::D32_SFLOAT_S8_UINT) {
                F::D32_SFLOAT_S8_UINT
            } else {
                F::D32_SFLOAT
            }
        }
    }
}

/// How a format of this interface is sampled where Vulkan's differs: Direct3D 9's `L8`
/// reads (r, r, r, 1), `A8` (0, 0, 0, a), `X8R8G8B8` an alpha of 1, and a float format's
/// missing channels 1.
pub(crate) fn swizzle(f: Format) -> vk::ComponentMapping {
    use vk::ComponentSwizzle as S;
    let m = |r, g, b, a| vk::ComponentMapping { r, g, b, a };
    match f {
        Format::R8 => m(S::R, S::R, S::R, S::ONE),
        Format::A8 => m(S::ZERO, S::ZERO, S::ZERO, S::R),
        Format::Bgrx8 | Format::B5G6R5 => m(S::IDENTITY, S::IDENTITY, S::IDENTITY, S::ONE),
        Format::R16F | Format::R32F => m(S::IDENTITY, S::ONE, S::ONE, S::ONE),
        Format::Rg16F | Format::Rg32F => m(S::IDENTITY, S::IDENTITY, S::ONE, S::ONE),
        _ => m(S::IDENTITY, S::IDENTITY, S::IDENTITY, S::IDENTITY),
    }
}

pub(crate) fn is_depth_vk(f: vk::Format) -> bool {
    matches!(f, vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 | vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D16_UNORM_S8_UINT)
}

pub(crate) fn has_stencil(f: vk::Format) -> bool {
    matches!(f, vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D16_UNORM_S8_UINT)
}

pub(crate) fn aspect_of(f: vk::Format) -> vk::ImageAspectFlags {
    if is_depth_vk(f) {
        if has_stencil(f) { vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL } else { vk::ImageAspectFlags::DEPTH }
    } else {
        vk::ImageAspectFlags::COLOR
    }
}

fn samples_flag(n: u32) -> vk::SampleCountFlags {
    match n {
        0 | 1 => vk::SampleCountFlags::TYPE_1,
        2 => vk::SampleCountFlags::TYPE_2,
        4 => vk::SampleCountFlags::TYPE_4,
        8 => vk::SampleCountFlags::TYPE_8,
        _ => vk::SampleCountFlags::TYPE_16,
    }
}

impl Gpu {
    pub const fn new(log: Log) -> Gpu {
        Gpu { log, st: RefCell::new(None), serial: Cell::new(0), depth: Cell::new(0), publish: Cell::new(false) }
    }

    /// As `new`, publishing its pass now at each `Cmd`'s end (`display8v1_vk_pass`).
    pub const fn publishing(log: Log) -> Gpu {
        Gpu { log, st: RefCell::new(None), serial: Cell::new(0), depth: Cell::new(0), publish: Cell::new(true) }
    }

    /// Opens the backend on Display8's device. False (and logged) when it cannot be.
    pub fn init(&self, disp: Display8) -> bool {
        if self.st.borrow().is_some() {
            return true;
        }
        let d = (disp.vk)();
        if d.is_null() {
            vlog!(self, "gpu: Display8 says Vulkan but has no device");
            return false;
        }
        // Only the fields within the size Display8 says (gpu/src/display.rs, "Versions").
        let d = unsafe { crate::display::read(d) };
        let sfn = ash::StaticFn {
            get_instance_proc_addr: unsafe { core::mem::transmute::<usize, vk::PFN_vkGetInstanceProcAddr>(d.get_instance_proc_addr) },
        };
        let inst = unsafe { ash::Instance::load(&sfn, vk::Handle::from_raw(d.instance as u64)) };
        let dev = unsafe { ash::Device::load(inst.fp_v1_0(), vk::Handle::from_raw(d.device as u64)) };
        let pdev: vk::PhysicalDevice = vk::Handle::from_raw(d.physical_device as u64);
        let props = unsafe { inst.get_physical_device_properties(pdev) };
        let mem_props = unsafe { inst.get_physical_device_memory_properties(pdev) };
        let sh = Rc::new(Shared {
            disp,
            dev,
            inst,
            pdev,
            mem_props,
            limits: props.limits,
            fif: d.frames_in_flight.max(1) as u64,
            features: d.features,
            force_aniso: d.sampler_anisotropy,
            lod_bias: d.sampler_lod_bias,
            cache: vk::Handle::from_raw(d.pipeline_cache),
            pools: RefCell::new(Pools::default()),
            garbage: RefCell::new(Vec::new()),
            now: Cell::new(0),
        });
        let dv = &sh.dev;
        // Set 0: each stage's register file, a dynamic uniform buffer. Set 1: the
        // Direct3D 9 stages' textures, combined image samplers.
        // A compute shader's register file is binding 0's (its cbuffers at b0), and it
        // reads textures as the other stages do.
        let (vsf, psf, csf) = (vk::ShaderStageFlags::VERTEX, vk::ShaderStageFlags::FRAGMENT, vk::ShaderStageFlags::COMPUTE);
        let b0 = [
            vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).descriptor_count(1).stage_flags(vsf | csf),
            vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).descriptor_count(1).stage_flags(psf),
        ];
        let mut b1 = Vec::new();
        for i in 0..SLOTS as u32 {
            b1.push(
                vk::DescriptorSetLayoutBinding::default()
                    .binding(i)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vsf | psf | csf),
            );
        }
        // Set 2: read-only storage buffers; set 3: read-write ones, then storage images.
        let mut b2 = Vec::new();
        let mut b3 = Vec::new();
        for i in 0..STORAGE_SLOTS as u32 {
            b2.push(vk::DescriptorSetLayoutBinding::default().binding(i).descriptor_type(vk::DescriptorType::STORAGE_BUFFER).descriptor_count(1).stage_flags(vsf | psf | csf));
            b3.push(vk::DescriptorSetLayoutBinding::default().binding(i).descriptor_type(vk::DescriptorType::STORAGE_BUFFER).descriptor_count(1).stage_flags(csf));
        }
        for i in 0..STORAGE_SLOTS as u32 {
            b3.push(vk::DescriptorSetLayoutBinding::default().binding(STORAGE_SLOTS as u32 + i).descriptor_type(vk::DescriptorType::STORAGE_IMAGE).descriptor_count(1).stage_flags(csf));
        }
        let made = (|| unsafe {
            let set0 = dv.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b0), None).ok()?;
            let set1 = dv.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b1), None).ok()?;
            let set2 = dv.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b2), None).ok()?;
            let set3 = dv.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b3), None).ok()?;
            let sets = [set0, set1, set2];
            let layout = dv.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().set_layouts(&sets), None).ok()?;
            let csets = [set0, set1, set2, set3];
            let clayout = dv.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().set_layouts(&csets), None).ok()?;
            let sizes = [vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, descriptor_count: 512 }];
            let ubo_pool = dv.create_descriptor_pool(&vk::DescriptorPoolCreateInfo::default().max_sets(256).pool_sizes(&sizes), None).ok()?;
            Some((set0, set1, set2, set3, layout, clayout, ubo_pool))
        })();
        let Some((set0, set1, set2, set3, layout, clayout, ubo_pool)) = made else {
            vlog!(self, "gpu: Vulkan: the pipeline layout could not be made");
            return false;
        };
        let name = unsafe { core::ffi::CStr::from_ptr(props.device_name.as_ptr()) };
        vlog!(
            self,
            "gpu: on Vulkan: {} ({}.{}), {} frames in flight, uniform alignment {}, {}",
            name.to_str().unwrap_or("?"),
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version),
            sh.fif,
            props.limits.min_uniform_buffer_offset_alignment,
            if d.features & FEATURE_D3D9_CENTRES != 0 { "Direct3D 9's pixel centres" } else { "Vulkan's pixel centres" }
        );
        let mut ubo = Vec::new();
        for _ in 0..sh.fif {
            ubo.push(UboRing::default());
        }
        *self.st.borrow_mut() = Some(State {
            sh,
            set0,
            set1,
            set2,
            set3,
            layout,
            clayout,
            ubo_pool,
            spools: Vec::new(),
            dummy_storage: None,
            samplers: Vec::new(),
            passes: Vec::new(),
            dummies: [None, None, None],
            dummy_vb: None,
            ubo,
            frame: VkFrame8::default(),
            has_frame: false,
            open: None,
            pass: None,
            externals: Vec::new(),
            touched: Vec::new(),
            depth_pipes: Vec::new(),
            depth_mods: None,
            told: 0,
        });
        true
    }

    pub fn ready(&self) -> bool {
        self.st.borrow().is_some()
    }

    /// Compute and storage: every Vulkan device's graphics queue has them.
    pub fn compute(&self) -> bool {
        self.ready()
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

    /// Whether Direct3D 9's pixel centres are kept (the comparison mode): the viewport
    /// moved by DXVK's offset, and `half_pixel` 0.5.
    pub fn d3d9_centres(&self) -> bool {
        self.sh().is_some_and(|s| s.features & FEATURE_D3D9_CENTRES != 0)
    }

    // -- the frame --

    /// The command buffer of the frame now, with no render pass of ours open in it:
    /// for whatever is recorded outside a pass (uploads, layout changes, copies).
    pub(crate) fn outside(&self) -> Option<vk::CommandBuffer> {
        self.suspend();
        self.cmdbuf()
    }

    /// The command buffer of the frame now (asked of Display8 when none is known: it
    /// begins one after a `Present`). Display8 may record into it at that ask (its
    /// mirrors), so a render pass of ours is closed first.
    pub(crate) fn cmdbuf(&self) -> Option<vk::CommandBuffer> {
        if let Some(Some(c)) = self.with(|s| s.has_frame.then_some(vk::Handle::from_raw(s.frame.command_buffer as u64))) {
            return Some(c);
        }
        self.new_frame()
    }

    /// Asks Display8 for the frame now; when it is another than the last, what the
    /// frames before let go of may be destroyed, and the uniform ring of its slot starts
    /// again.
    pub(crate) fn new_frame(&self) -> Option<vk::CommandBuffer> {
        self.suspend();
        let sh = self.sh()?;
        let f = (sh.disp.vk_frame)();
        if f.is_null() {
            return None;
        }
        let f = unsafe { crate::display::read(f) };
        if f.command_buffer == 0 {
            return None;
        }
        sh.collect(f.index);
        self.with(|s| {
            s.frame = f;
            s.has_frame = true;
        });
        Some(vk::Handle::from_raw(f.command_buffer as u64))
    }

    /// Closes the render pass now, if one is open (the pass stays the pass).
    pub(crate) fn suspend(&self) {
        let r = self.with(|s| {
            if s.open.take().is_some() {
                Some((s.sh.clone(), s.frame.command_buffer))
            } else {
                None
            }
        });
        if let Some(Some((sh, cb))) = r {
            unsafe { sh.dev.cmd_end_render_pass(vk::Handle::from_raw(cb as u64)) };
        }
    }

    /// A `Cmd` begins: the frame now is asked for. False without one.
    pub(crate) fn enter(&self) -> bool {
        self.serial.set(self.serial.get().wrapping_add(1));
        self.depth.set(self.depth.get() + 1);
        if self.depth.get() == 1 {
            // Display8 records none of the engine's draws while a Cmd is alive, and
            // closes a render pass of its own first (gpu/src/display.rs, "Passthrough").
            self.tell_display(crate::display::PASS_ENTER, None);
        }
        self.with(|s| s.has_frame = false);
        self.new_frame().is_some()
    }

    /// A `Cmd` ends: the render pass is closed, Display8's images are put back in their
    /// layouts, and the next recorder may be another.
    pub(crate) fn leave(&self) {
        self.suspend();
        self.restore_externals();
        self.serial.set(self.serial.get().wrapping_add(1));
        let d = self.depth.get().saturating_sub(1);
        self.depth.set(d);
        if d == 0 {
            let p = if self.publish.get() { self.published() } else { None };
            self.with(|s| s.has_frame = false);
            match p {
                Some(p) => self.tell_display(crate::display::PASS_PUBLISH, Some(&p)),
                None => self.tell_display(crate::display::PASS_LEAVE, None),
            }
        }
    }

    /// `display8v1_vk_pass`, when this Display8 has it.
    fn tell_display(&self, event: u32, pass: Option<&crate::display::VkPass8>) {
        let Some(f) = self.sh().and_then(|sh| sh.disp.vk_pass) else { return };
        f(event, pass.map_or(core::ptr::null(), |p| p as *const _));
    }

    /// The pass now as Display8's passthrough draws into it, every attachment put in its
    /// attachment layout (outside a render pass, which `leave` has closed): one colour
    /// target, layer 0. None without a pass, or with several colour targets.
    fn published(&self) -> Option<crate::display::VkPass8> {
        let ps = self.with(|s| s.pass.clone())??;
        if ps.colors.len() != 1 {
            return None;
        }
        let cb = self.outside()?;
        let c = &ps.colors[0];
        self.transition(cb, &c.tex, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        if let Some(d) = &ps.depth {
            self.transition(cb, &d.tex, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        }
        // An image of Display8's goes back to its own layout (the same ones).
        self.restore_externals();
        let img = |a: &Att, layout: vk::ImageLayout| VkImage8 {
            size: core::mem::size_of::<VkImage8>() as u32,
            image: vk::Handle::as_raw(a.tex.image()),
            view: vk::Handle::as_raw(a.tex.att_view(a.layer)),
            format: a.tex.vk_format().as_raw() as u32,
            width: ps.size.0,
            height: ps.size.1,
            samples: ps.key.samples.max(1),
            layout: layout.as_raw() as u32,
        };
        let color = img(c, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        if color.view == 0 {
            return None;
        }
        let depth = ps.depth.as_ref().map_or(VkImage8::default(), |d| img(d, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL));
        Some(crate::display::VkPass8 { size: core::mem::size_of::<crate::display::VkPass8>() as u32, color, depth })
    }

    pub(crate) fn serial(&self) -> u32 {
        self.serial.get()
    }

    /// Display8's images whose layout changed, back in theirs (outside a pass).
    pub(crate) fn restore_externals(&self) {
        let list = self.with(|s| core::mem::take(&mut s.touched)).unwrap_or_default();
        if list.is_empty() {
            return;
        }
        let Some(cb) = self.outside() else { return };
        for t in list {
            if let Some(home) = t.0.home {
                self.transition(cb, &t, home);
            }
        }
    }

    /// Puts `t` (made) in `to`, the whole image; a barrier with broad masks (a frame
    /// recorded once: correct, if not the tightest). Must be outside a render pass.
    pub(crate) fn transition(&self, cb: vk::CommandBuffer, t: &Texture, to: vk::ImageLayout) {
        let from = t.0.layout.get();
        // The same layout needs no barrier, but for a copy after a copy (render passes
        // carry their own dependencies).
        if from == to && to != vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            return;
        }
        let Some(sh) = self.sh() else { return };
        let o = t.0.obj.borrow();
        let Some(img) = o.as_ref() else { return };
        barrier(&sh, cb, img.image, img.aspect, 0, vk::REMAINING_MIP_LEVELS, from, to);
        drop(o);
        t.0.layout.set(to);
        if t.0.home.is_some() {
            self.with(|s| {
                if !s.touched.iter().any(|x| x.same(t)) {
                    s.touched.push(t.clone());
                }
            });
        }
    }

    // -- objects shared by every pipeline --

    pub(crate) fn sampler(&self, d: &Sampler) -> vk::Sampler {
        if let Some(Some(s)) = self.with(|s| s.samplers.iter().find(|x| x.0 == *d).map(|x| x.1)) {
            return s;
        }
        let Some(sh) = self.sh() else { return vk::Sampler::null() };
        let f = |f: Filter| if f == Filter::Point { vk::Filter::NEAREST } else { vk::Filter::LINEAR };
        let a = |a: Address| match a {
            Address::Wrap => vk::SamplerAddressMode::REPEAT,
            Address::Mirror => vk::SamplerAddressMode::MIRRORED_REPEAT,
            Address::Clamp => vk::SamplerAddressMode::CLAMP_TO_EDGE,
        };
        // As DXVK makes a sampler under the game's dxvk.conf (renderer/README.md): with
        // `d3d9.samplerAnisotropy`, a linear filter is an anisotropic one of that much;
        // `d3d9.samplerLodBias` biases every sampler with mips (one without has no level
        // to pick; DXVK zeroes its bias too).
        let forced = sh.force_aniso >= 0;
        let wants = |x: Filter| x == Filter::Anisotropic || (forced && x == Filter::Linear);
        let aniso = (wants(d.min) || wants(d.mag)) && sh.features & FEATURE_ANISOTROPY != 0;
        let most = if forced { sh.force_aniso as u32 } else { d.anisotropy };
        let bias = if d.mip == MipFilter::None { 0.0 } else { sh.lod_bias.clamp(-sh.limits.max_sampler_lod_bias, sh.limits.max_sampler_lod_bias) };
        let ci = vk::SamplerCreateInfo::default()
            .mag_filter(f(d.mag))
            .min_filter(f(d.min))
            .mipmap_mode(if d.mip == MipFilter::Linear { vk::SamplerMipmapMode::LINEAR } else { vk::SamplerMipmapMode::NEAREST })
            .address_mode_u(a(d.address_u))
            .address_mode_v(a(d.address_v))
            .address_mode_w(a(d.address_w))
            .anisotropy_enable(aniso)
            .max_anisotropy(if aniso { (most.clamp(1, 16) as f32).min(sh.limits.max_sampler_anisotropy) } else { 1.0 })
            .mip_lod_bias(bias)
            .min_lod(0.0)
            .max_lod(if d.mip == MipFilter::None { 0.0 } else { vk::LOD_CLAMP_NONE });
        if d.srgb {
            self.once(1 << 20, format_args!("gpu: Vulkan: an sRGB sampler is sampled as linear (no sRGB views yet)"));
        }
        let s = unsafe { sh.dev.create_sampler(&ci, None) }.unwrap_or_default();
        self.with(|st| st.samplers.push((*d, s)));
        s
    }

    /// The render pass of `key` with `loads` (2 bits per attachment: 0 load, 1 clear,
    /// 2 don't care; colours first, then depth), every attachment in its attachment
    /// layout before and after.
    pub(crate) fn render_pass(&self, key: &PassKey, loads: u32) -> vk::RenderPass {
        if let Some(Some(rp)) = self.with(|s| s.passes.iter().find(|p| p.0 == (*key, loads)).map(|p| p.1)) {
            return rp;
        }
        let Some(sh) = self.sh() else { return vk::RenderPass::null() };
        let op = |i: u32| match (loads >> (2 * i)) & 3 {
            1 => vk::AttachmentLoadOp::CLEAR,
            2 => vk::AttachmentLoadOp::DONT_CARE,
            _ => vk::AttachmentLoadOp::LOAD,
        };
        let mut atts = Vec::new();
        let mut crefs = Vec::new();
        for i in 0..key.ncolors {
            atts.push(
                vk::AttachmentDescription::default()
                    .format(key.colors[i as usize])
                    .samples(samples_flag(key.samples))
                    .load_op(op(i))
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            );
            crefs.push(vk::AttachmentReference { attachment: i, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL });
        }
        let dref = vk::AttachmentReference { attachment: key.ncolors, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };
        if key.depth != vk::Format::UNDEFINED {
            let st = has_stencil(key.depth);
            atts.push(
                vk::AttachmentDescription::default()
                    .format(key.depth)
                    .samples(samples_flag(key.samples))
                    .load_op(op(MAX_COLORS as u32))
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .stencil_load_op(if st { vk::AttachmentLoadOp::LOAD } else { vk::AttachmentLoadOp::DONT_CARE })
                    .stencil_store_op(if st { vk::AttachmentStoreOp::STORE } else { vk::AttachmentStoreOp::DONT_CARE })
                    .initial_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
            );
        }
        let mut sub = vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&crefs);
        if key.depth != vk::Format::UNDEFINED {
            sub = sub.depth_stencil_attachment(&dref);
        }
        let subs = [sub];
        // Whatever came before (another pass's writes, a copy, an upload) is done before
        // this one reads or writes its attachments, and it is done before what follows.
        let all = vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE;
        let deps = [
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .dst_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(all),
            vk::SubpassDependency::default()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .dst_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(all),
        ];
        let ci = vk::RenderPassCreateInfo::default().attachments(&atts).subpasses(&subs).dependencies(&deps);
        let rp = unsafe { sh.dev.create_render_pass(&ci, None) }.unwrap_or_default();
        self.with(|s| s.passes.push(((*key, loads), rp)));
        rp
    }

    /// The placeholder texture for a slot of `dim`.
    pub(crate) fn dummy(&self, dim: spirv::Dim) -> Option<Texture> {
        let i = match dim {
            spirv::Dim::D3 => 1,
            spirv::Dim::Cube => 2,
            _ => 0,
        };
        if let Some(Some(t)) = self.with(|s| s.dummies[i].clone()) {
            return Some(t);
        }
        let px = alloc::vec![0u8; 4 * 6];
        let desc = match i {
            1 => TextureDesc::d3(1, 1, 1, 1, Format::Bgra8, Usage::SAMPLED),
            2 => TextureDesc::cube(1, 1, Format::Bgra8, Usage::SAMPLED),
            _ => TextureDesc::d2(1, 1, Format::Bgra8, Usage::SAMPLED),
        };
        let t = self.texture(desc, Contents::Owned(px));
        self.with(|s| s.dummies[i] = Some(t.clone()));
        Some(t)
    }

    /// A vertex buffer of one (0, 0, 0, 1): what a shader input reads that the layout
    /// does not give (Direct3D 9's default for a missing element).
    pub(crate) fn dummy_vb(&self) -> Option<Buffer> {
        if let Some(Some(b)) = self.with(|s| s.dummy_vb.clone()) {
            return Some(b);
        }
        let data: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, 16) };
        let b = self.buffer(BufferDesc { size: 16, kind: BufferKind::Vertex, dynamic: false }, bytes);
        self.with(|s| s.dummy_vb = Some(b.clone()));
        Some(b)
    }

    /// Space for `vs` and `ps` bytes of uniforms in the frame slot's ring: the set and
    /// both offsets. A chunk keeps room for a whole range past every offset, as the
    /// set's descriptors are bound with the stages' whole register files.
    pub(crate) fn ubo_alloc(&self, vs: &[u8], ps: &[u8]) -> Option<(vk::DescriptorSet, u32, u32)> {
        let sh = self.sh()?;
        let align = sh.limits.min_uniform_buffer_offset_alignment.max(16);
        let up = |x: u64| x.div_ceil(align) * align;
        let need = up(vs.len() as u64) + up(ps.len() as u64) + VS_RANGE.max(PS_RANGE);
        let (slot, index) = self.with(|s| (s.frame.slot as usize % s.ubo.len(), s.frame.index))?;
        let at = self.with(|s| {
            let r = &mut s.ubo[slot];
            if r.frame != index {
                r.frame = index;
                r.at = (0, 0);
            }
            let (mut ci, mut off) = r.at;
            while ci < r.chunks.len() {
                if off + need <= UBO_CHUNK {
                    return Some((ci, off));
                }
                ci += 1;
                off = 0;
            }
            None
        })?;
        let (ci, off) = match at {
            Some(x) => x,
            None => {
                let c = self.ubo_chunk(&sh)?;
                self.with(|s| {
                    let r = &mut s.ubo[slot];
                    r.chunks.push(c);
                    (r.chunks.len() - 1, 0)
                })?
            }
        };
        self.with(|s| {
            let r = &mut s.ubo[slot];
            let c = &r.chunks[ci];
            let vo = off;
            let po = vo + up(vs.len() as u64);
            unsafe {
                core::ptr::copy_nonoverlapping(vs.as_ptr(), c.cpu.add(vo as usize), vs.len());
                core::ptr::copy_nonoverlapping(ps.as_ptr(), c.cpu.add(po as usize), ps.len());
            }
            r.at = (ci, po + up(ps.len() as u64));
            (c.set, vo as u32, po as u32)
        })
    }

    fn ubo_chunk(&self, sh: &Shared) -> Option<UboChunk> {
        let dv = &sh.dev;
        let bci = vk::BufferCreateInfo::default().size(UBO_CHUNK).usage(vk::BufferUsageFlags::UNIFORM_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { dv.create_buffer(&bci, None) }.ok()?;
        let req = unsafe { dv.get_buffer_memory_requirements(buf) };
        let mem = sh.alloc(req, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, false)?;
        unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.ok()?;
        let cpu = sh.pools.borrow_mut().map(dv, &mem)?;
        let (pool, layout) = self.with(|s| (s.ubo_pool, s.set0))?;
        let layouts = [layout];
        let set = unsafe { dv.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&layouts)) }.ok()?[0];
        let bi0 = [vk::DescriptorBufferInfo { buffer: buf, offset: 0, range: VS_RANGE }];
        let bi1 = [vk::DescriptorBufferInfo { buffer: buf, offset: 0, range: PS_RANGE }];
        let w = [
            vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).buffer_info(&bi0),
            vk::WriteDescriptorSet::default().dst_set(set).dst_binding(1).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).buffer_info(&bi1),
        ];
        unsafe { dv.update_descriptor_sets(&w, &[]) };
        vlog!(self, "gpu: Vulkan: a uniform chunk of {} KB", UBO_CHUNK >> 10);
        Some(UboChunk { _buf: buf, _mem: mem, cpu, set })
    }

    /// A set of the frame's pool, of set 1's layout.
    pub(crate) fn texture_set(&self) -> Option<vk::DescriptorSet> {
        let sh = self.sh()?;
        let (pool, layout) = self.with(|s| (vk::Handle::from_raw(s.frame.descriptor_pool), s.set1))?;
        let layouts = [layout];
        match unsafe { sh.dev.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&layouts)) } {
            Ok(v) => Some(v[0]),
            Err(e) => {
                self.once(1 << 21, format_args!("gpu: Vulkan: the frame's descriptor pool is spent ({:?}); draws are dropped", e));
                None
            }
        }
    }

    /// A storage set (set 2 or 3's layout) from the backend's pool of the frame slot
    /// now; the pool is reset the first time its slot is asked for in a new frame
    /// (whatever it held is two frames old by then).
    pub(crate) fn storage_set(&self, which: u32) -> Option<vk::DescriptorSet> {
        let sh = self.sh()?;
        let (slot, index, layout) = self.with(|s| (s.frame.slot as usize % sh.fif.max(1) as usize, s.frame.index, if which == 3 { s.set3 } else { s.set2 }))?;
        let pool = self.with(|s| {
            while s.spools.len() <= slot {
                let sizes = [
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_BUFFER, descriptor_count: 4096 },
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_IMAGE, descriptor_count: 1024 },
                ];
                let p = unsafe { sh.dev.create_descriptor_pool(&vk::DescriptorPoolCreateInfo::default().max_sets(512).pool_sizes(&sizes), None) }.unwrap_or_default();
                s.spools.push((u64::MAX, p));
            }
            let e = &mut s.spools[slot];
            if e.0 != index {
                e.0 = index;
                unsafe { sh.dev.reset_descriptor_pool(e.1, vk::DescriptorPoolResetFlags::empty()) }.ok();
            }
            e.1
        })?;
        if pool == vk::DescriptorPool::null() {
            return None;
        }
        let layouts = [layout];
        match unsafe { sh.dev.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&layouts)) } {
            Ok(v) => Some(v[0]),
            Err(e) => {
                self.once(1 << 26, format_args!("gpu: Vulkan: the frame's storage descriptors are spent ({:?}); draws and dispatches are dropped", e));
                None
            }
        }
    }

    /// The storage buffer bound where a slot a shader uses has nothing (16 bytes).
    pub(crate) fn dummy_storage(&self) -> Option<Storage> {
        if let Some(Some(b)) = self.with(|s| s.dummy_storage.clone()) {
            return Some(b);
        }
        let b = Storage::new(StorageDesc { size: 64, stride: 4 });
        self.with(|s| s.dummy_storage = Some(b.clone()));
        Some(b)
    }

    // -- Display8's objects --

    /// One of Display8's images, wrapped (one wrapper per image while it lives).
    pub(crate) fn external(&self, i: &VkImage8, format: Format) -> Option<Texture> {
        if i.image == 0 {
            return None;
        }
        let found = self.with(|s| {
            s.externals.retain(|e| e.1.strong_count() > 0);
            s.externals.iter().find(|e| e.0 == i.image).and_then(|e| e.1.upgrade())
        })?;
        if let Some(t) = found {
            if t.desc.width == i.width && t.desc.height == i.height {
                return Some(Texture(t));
            }
        }
        let vf = vk::Format::from_raw(i.format as i32);
        let depth = is_depth_vk(vf);
        let desc = TextureDesc {
            width: i.width,
            height: i.height,
            levels: 1,
            format,
            samples: i.samples.max(1),
            usage: if depth { Usage::DEPTH } else { Usage::RENDER_TARGET | Usage::SAMPLED | Usage::COPY_SRC },
            kind: TextureKind::D2,
            depth: 1,
        };
        let t = tex::Texture::external(desc, i, vf);
        self.with(|s| s.externals.push((i.image, Rc::downgrade(&t.0))));
        Some(t)
    }

    pub fn engine_target(&self) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = VkImage8::default();
        if !(sh.disp.vk_target)(crate::display::TARGET_COLOR, &mut i) {
            return None;
        }
        // The engine's back buffer: X8R8G8B8, as it asked for.
        self.external(&i, Format::Bgrx8)
    }

    pub fn engine_depth(&self) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = VkImage8::default();
        if !(sh.disp.vk_target)(crate::display::TARGET_DEPTH, &mut i) {
            return None;
        }
        let f = match vk::Format::from_raw(i.format as i32) {
            vk::Format::D32_SFLOAT => Format::D32,
            vk::Format::D24_UNORM_S8_UINT => Format::D24S8,
            vk::Format::X8_D24_UNORM_PACK32 => Format::D24X8,
            _ => Format::D16,
        };
        self.external(&i, f)
    }

    /// An engine texture (an `IDirect3DTexture8` of Display8's) as Display8's mirror.
    pub fn engine_texture(&self, tex8: *mut core::ffi::c_void) -> Option<Texture> {
        let sh = self.sh()?;
        let mut i = VkImage8::default();
        if tex8.is_null() || !(sh.disp.vk_mirror)(tex8, &mut i) {
            return None;
        }
        self.external(&i, Format::Bgra8)
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
        Storage::new(desc)
    }

    pub fn compute_pipeline(&self, desc: ComputeDesc) -> ComputePipeline {
        pipe::ComputePipeline::new(desc)
    }

    pub fn begin<'a>(&'a self, frame: &'a mut crate::Frame) -> Option<Cmd<'a>> {
        Cmd::new(self, frame)
    }

    /// Submits what is recorded and waits for it (Display8's `display8v1_vk_flush`): for
    /// a read-back. The frame goes on in the same command buffer.
    pub(crate) fn flush(&self) -> bool {
        self.suspend();
        let Some(sh) = self.sh() else { return false };
        // The command buffer is begun again: nothing bound in it is bound any more.
        self.serial.set(self.serial.get().wrapping_add(1));
        (sh.disp.vk_flush)()
    }
}

/// A layout change of `levels` from `level` of every layer, with broad masks.
#[allow(clippy::too_many_arguments)]
pub(crate) fn barrier(sh: &Shared, cb: vk::CommandBuffer, image: vk::Image, aspect: vk::ImageAspectFlags, level: u32, levels: u32, from: vk::ImageLayout, to: vk::ImageLayout) {
    let b = [vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
        .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
        .old_layout(from)
        .new_layout(to)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: level, level_count: levels, base_array_layer: 0, layer_count: vk::REMAINING_ARRAY_LAYERS })];
    unsafe { sh.dev.cmd_pipeline_barrier(cb, vk::PipelineStageFlags::ALL_COMMANDS, vk::PipelineStageFlags::ALL_COMMANDS, vk::DependencyFlags::empty(), &[], &[], &b) };
}

/// A memory barrier with broad masks: a buffer written by a copy, read after.
pub(crate) fn memory_barrier(sh: &Shared, cb: vk::CommandBuffer) {
    let b = [vk::MemoryBarrier::default().src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)];
    unsafe { sh.dev.cmd_pipeline_barrier(cb, vk::PipelineStageFlags::ALL_COMMANDS, vk::PipelineStageFlags::ALL_COMMANDS, vk::DependencyFlags::empty(), &b, &[], &[]) };
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
