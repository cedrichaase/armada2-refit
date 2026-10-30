//! Stub mode on Vulkan (`Backend=vulkan`): Display8's own device, swap chain and frame,
//! which render3d's and the UI's `gpu` record into through the exports of
//! `gpu/src/display.rs` (the contract; README.md, "Vulkan").
//!
//! - **The loader** is `vulkan-1.dll`, opened with `LoadLibraryA`; everything else is
//!   reached through `vkGetInstanceProcAddr` (`ash`, without its loader). Vulkan 1.1,
//!   `VK_KHR_swapchain` and `VK_KHR_win32_surface`, one queue family that draws and
//!   presents: what MoltenVK has as well.
//! - **The engine's back buffer and depth** are offscreen images of ours
//!   (B8G8R8A8_UNORM and D16_UNORM, as the engine asks for X8R8G8B8 and D16), kept in
//!   the attachment layouts between recorders. The frame's last pass draws the colour
//!   one into the swap chain's image through the engine's gamma ramp
//!   (`shaders/present.hlsl`, SPIR-V), so the swap chain is never a target of anyone
//!   else's and can be any size or format the surface offers.
//! - **Two frames in flight.** Each slot has a command buffer, a fence, a descriptor
//!   pool, an upload ring (host-visible and mapped; chunks added when a frame needs
//!   more, so the address space it takes stays small in a 32-bit process) and the
//!   callbacks deferred until its fence has passed. A frame is begun at the first call
//!   that needs it after a `Present`, and ended, submitted and presented at the next.
mod cache;
pub mod exports;
pub mod mirror;
pub mod pass;

use crate::log;
use crate::shaders::present_spv::{PS_GAMMA_SPV, VS_PRESENT_SPV};
use alloc::vec::Vec;
use ash::vk;
use core::ffi::c_void;
use gpu::display::{VkAlloc8, VkDevice8, VkFrame8, VkImage8};

const SLOTS: usize = 2;
const CHUNK: u64 = 4 << 20;
/// A mirror's upload past this many bytes goes through a staging buffer of its own
/// (`upload_levels`), not the ring.
pub const STAGE_ABOVE: usize = 256 << 10;

pub struct Img {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub mem: vk::DeviceMemory,
    pub format: vk::Format,
    pub w: u32,
    pub h: u32,
    pub levels: u32,
    pub layout: vk::ImageLayout,
}

struct Chunk {
    buf: vk::Buffer,
    /// Kept with its buffer: a chunk lives as long as the device.
    #[allow(dead_code)]
    mem: vk::DeviceMemory,
    ptr: *mut u8,
    size: u64,
}

struct Slot {
    pool: vk::CommandPool,
    cmd: vk::CommandBuffer,
    fence: vk::Fence,
    dpool: vk::DescriptorPool,
    acquired: vk::Semaphore,
    chunks: Vec<Chunk>,
    /// (chunk, offset) of the next free byte.
    at: (usize, u64),
    deferred: Vec<(extern "C" fn(*mut c_void), *mut c_void)>,
}

pub struct Vk {
    _entry: ash::Entry,
    instance: ash::Instance,
    surface_fn: ash::khr::surface::Instance,
    swap_fn: ash::khr::swapchain::Device,
    pdev: vk::PhysicalDevice,
    pub dev: ash::Device,
    queue: vk::Queue,
    qfam: u32,
    mem: vk::PhysicalDeviceMemoryProperties,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    sc_images: Vec<vk::Image>,
    sc_views: Vec<vk::ImageView>,
    sc_fbs: Vec<vk::Framebuffer>,
    sc_done: Vec<vk::Semaphore>,
    sc_format: vk::Format,
    sc_extent: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
    pub color: Img,
    pub depth: Img,
    slots: Vec<Slot>,
    slot: usize,
    pub index: u64,
    recording: bool,
    first: bool,
    present_pass: vk::RenderPass,
    clear_pass: vk::RenderPass,
    clear_fb: vk::Framebuffer,
    dsl: [vk::DescriptorSetLayout; 2],
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    sampler: vk::Sampler,
    ramp: Img,
    ramp_values: Option<[u16; 768]>,
    cache: vk::PipelineCache,
    /// The size the cache on disk was last written at (cache.rs).
    cache_saved: usize,
    pub exported: VkDevice8,
    pub frame_out: VkFrame8,
    pub uniform_align: u32,
    stale: bool,
    /// The validation layer's messenger (`Validate=1`), destroyed with the instance.
    messenger: (Option<ash::ext::debug_utils::Instance>, vk::DebugUtilsMessengerEXT),
}

/// What `Vk::start` has made before there is a `Vk` to own it: destroyed if the start
/// fails there (dropped), forgotten once the `Vk` owns it.
struct Partial {
    instance: ash::Instance,
    surface_fn: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    du: Option<ash::ext::debug_utils::Instance>,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Drop for Partial {
    fn drop(&mut self) {
        unsafe {
            self.surface_fn.destroy_surface(self.surface, None);
            if let Some(du) = self.du.take() {
                du.destroy_debug_utils_messenger(self.messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

static mut VK: Option<Vk> = None;

#[allow(static_mut_refs)]
pub fn get() -> Option<&'static mut Vk> {
    unsafe { VK.as_mut() }
}

use crate::rt::win::{GetModuleHandleA, GetProcAddress, LoadLibraryA};

fn find_memory(props: &vk::PhysicalDeviceMemoryProperties, bits: u32, want: vk::MemoryPropertyFlags) -> Option<u32> {
    (0..props.memory_type_count).find(|&i| bits & (1 << i) != 0 && props.memory_types[i as usize].property_flags.contains(want))
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _ty: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user: *mut c_void,
) -> vk::Bool32 {
    if !data.is_null() {
        let msg = unsafe { crate::rt::cstr_at((*data).p_message as *const u8, 2000) };
        log!("vulkan {:#x}: {}", severity.as_raw(), msg);
    }
    vk::FALSE
}

impl Vk {
    /// Opens Vulkan on `hwnd`, `w` x `h`; `interval` is the engine's presentation
    /// interval (0 and 1: wait for the vertical blank).
    pub fn start(hwnd: *mut c_void, w: u32, h: u32, interval: u32, validate: bool) -> Result<(), &'static str> {
        let m = unsafe { LoadLibraryA(c"vulkan-1.dll".as_ptr() as *const u8) };
        if m.is_null() {
            return Err("no vulkan-1.dll");
        }
        let gipa = unsafe { GetProcAddress(m, c"vkGetInstanceProcAddr".as_ptr() as *const u8) };
        if gipa == 0 {
            return Err("vulkan-1.dll has no vkGetInstanceProcAddr");
        }
        let entry = unsafe {
            ash::Entry::from_static_fn(ash::StaticFn { get_instance_proc_addr: core::mem::transmute::<usize, vk::PFN_vkGetInstanceProcAddr>(gipa) })
        };
        let app = vk::ApplicationInfo::default()
            .application_name(c"Armada2")
            .engine_name(c"Display8")
            .api_version(vk::make_api_version(0, 1, 1, 0));
        let offered = unsafe { entry.enumerate_instance_extension_properties(None) }.unwrap_or_default();
        let has = |n: &core::ffi::CStr| offered.iter().any(|e| e.extension_name_as_c_str().is_ok_and(|x| x == n));
        let mut exts = alloc::vec![ash::khr::surface::NAME.as_ptr(), ash::khr::win32_surface::NAME.as_ptr()];
        // MoltenVK lists its devices only to an instance that takes portability drivers
        // (display/vkprobe: Wine offers the extension, and the flag is harmless elsewhere).
        let mut flags = vk::InstanceCreateFlags::empty();
        if has(ash::khr::portability_enumeration::NAME) {
            exts.push(ash::khr::portability_enumeration::NAME.as_ptr());
            flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
        }
        // The validation layer: by name when the loader lists it; under Wine it does not
        // (winevulkan's vulkan-1.dll lists none) and the host loader takes it from
        // VK_INSTANCE_LAYERS instead. Either way its messages come through debug_utils.
        let listed = unsafe { entry.enumerate_instance_layer_properties() }
            .unwrap_or_default()
            .iter()
            .any(|l| l.layer_name_as_c_str().is_ok_and(|x| x == c"VK_LAYER_KHRONOS_validation"));
        let mut layers = Vec::new();
        let messenger = validate && has(ash::ext::debug_utils::NAME);
        if validate && listed {
            layers.push(c"VK_LAYER_KHRONOS_validation".as_ptr());
        }
        if messenger {
            exts.push(ash::ext::debug_utils::NAME.as_ptr());
        }
        let ici = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&exts).enabled_layer_names(&layers).flags(flags);
        let instance = unsafe { entry.create_instance(&ici, None) }.map_err(|_| "vkCreateInstance failed")?;
        // From here what is made is destroyed again if the start fails, so that another
        // backend can have the window (Backend=auto; README.md, "Choosing a backend").
        let surface_fn = ash::khr::surface::Instance::new(&entry, &instance);
        let mut part = Partial {
            instance: instance.clone(),
            surface_fn: surface_fn.clone(),
            surface: vk::SurfaceKHR::null(),
            du: None,
            messenger: vk::DebugUtilsMessengerEXT::null(),
        };
        if messenger {
            let du = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let ci = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
                .pfn_user_callback(Some(debug_callback));
            if let Ok(m) = unsafe { du.create_debug_utils_messenger(&ci, None) } {
                part.messenger = m;
                log!(
                    "vulkan: validation messages follow here ({})",
                    if listed { "the layer asked for by name" } else { "the layer only if VK_INSTANCE_LAYERS brings it" }
                );
            }
            part.du = Some(du);
        }
        let w32 = ash::khr::win32_surface::Instance::new(&entry, &instance);
        let sci = vk::Win32SurfaceCreateInfoKHR::default()
            .hinstance(unsafe { GetModuleHandleA(core::ptr::null()) } as vk::HINSTANCE)
            .hwnd(hwnd as vk::HWND);
        let surface = unsafe { w32.create_win32_surface(&sci, None) }.map_err(|_| "vkCreateWin32SurfaceKHR failed")?;
        part.surface = surface;

        // A device with one family that draws and presents; discrete first, then
        // integrated, virtual, other, and a CPU (software) device last; or only one whose
        // name has `Device=` in it (the bench: lavapipe, with --vulkan-device lavapipe).
        let pdevs = unsafe { instance.enumerate_physical_devices() }.map_err(|_| "no physical devices")?;
        let want = crate::rt::lower(crate::vk_device());
        let mut pick: Option<(vk::PhysicalDevice, u32, u32)> = None;
        for &pd in &pdevs {
            let props = unsafe { instance.get_physical_device_properties(pd) };
            let pname = unsafe { crate::rt::cstr_at(props.device_name.as_ptr() as *const u8, 256) };
            log!("vulkan: physical device {}", pname);
            if !want.is_empty() && !crate::rt::lower(pname).contains(want.as_str()) {
                continue;
            }
            let fams = unsafe { instance.get_physical_device_queue_family_properties(pd) };
            for (i, f) in fams.iter().enumerate() {
                let present = unsafe { surface_fn.get_physical_device_surface_support(pd, i as u32, surface) }.unwrap_or(false);
                if f.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present {
                    let rank = match props.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 4,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 3,
                        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                        vk::PhysicalDeviceType::CPU => 0,
                        _ => 1,
                    };
                    log!("vulkan:   type {}, rank {}", props.device_type.as_raw(), rank);
                    if pick.is_none() || rank > pick.unwrap().2 {
                        pick = Some((pd, i as u32, rank));
                    }
                    break;
                }
            }
        }
        let (pdev, qfam, rank) = pick.ok_or(if want.is_empty() { "no device that draws and presents on the window" } else { "no device named as Device= says that draws and presents" })?;
        let props = unsafe { instance.get_physical_device_properties(pdev) };
        let name = unsafe { crate::rt::cstr_at(props.device_name.as_ptr() as *const u8, 256) };
        log!(
            "vulkan: chose {} (type {}, rank {}; discrete > integrated > virtual > other > cpu)",
            name,
            props.device_type.as_raw(),
            rank
        );
        log!(
            "vulkan: {} (Vulkan {}.{}.{}, driver {:#x}), queue family {}",
            name,
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version),
            vk::api_version_patch(props.api_version),
            props.driver_version,
            qfam
        );
        let prio = [1.0f32];
        let qci = [vk::DeviceQueueCreateInfo::default().queue_family_index(qfam).queue_priorities(&prio)];
        let mut dexts = alloc::vec![ash::khr::swapchain::NAME.as_ptr()];
        // A portability device (MoltenVK) must have its subset enabled when it offers it.
        let dev_offered = unsafe { instance.enumerate_device_extension_properties(pdev) }.unwrap_or_default();
        if dev_offered.iter().any(|e| e.extension_name_as_c_str().is_ok_and(|x| x == c"VK_KHR_portability_subset")) {
            dexts.push(c"VK_KHR_portability_subset".as_ptr());
            log!("vulkan: a portability device (VK_KHR_portability_subset): no triangle fans, no point polygons");
        }
        // Anisotropic filtering where the device has it (gpu's samplers ask for it by
        // `Filter::Anisotropic`).
        let have = unsafe { instance.get_physical_device_features(pdev) };
        let feats = vk::PhysicalDeviceFeatures { sampler_anisotropy: have.sampler_anisotropy, ..Default::default() };
        let dci = vk::DeviceCreateInfo::default().queue_create_infos(&qci).enabled_extension_names(&dexts).enabled_features(&feats);
        let dev = unsafe { instance.create_device(pdev, &dci, None) }.map_err(|_| "vkCreateDevice failed")?;
        let mut features = 0;
        if have.sampler_anisotropy != 0 {
            features |= gpu::display::FEATURE_ANISOTROPY;
        }
        if crate::d3d9_centres() {
            features |= gpu::display::FEATURE_D3D9_CENTRES;
            log!("vulkan: PixelCentres=d3d9: the plugins draw with Direct3D 9's pixel centres (for comparing)");
        }
        let queue = unsafe { dev.get_device_queue(qfam, 0) };
        let mem = unsafe { instance.get_physical_device_memory_properties(pdev) };
        let swap_fn = ash::khr::swapchain::Device::new(&instance, &dev);
        // The pipeline cache, from disk when this device saved one (cache.rs).
        let saved = cache::load(&props);
        let cache = unsafe { dev.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default().initial_data(&saved), None) }
            .or_else(|_| unsafe { dev.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None) })
            .unwrap_or_default();
        let cache_saved = saved.len();
        drop(saved);

        let samplers = crate::dxvk_samplers();
        log!("vulkan: the plugins' samplers as dxvk.conf has DXVK make them: anisotropy {}, LOD bias {}", samplers.0, samplers.1);
        let mut me = Vk {
            _entry: entry,
            instance,
            surface_fn,
            swap_fn,
            pdev,
            dev,
            queue,
            qfam,
            mem,
            surface,
            swapchain: vk::SwapchainKHR::null(),
            sc_images: Vec::new(),
            sc_views: Vec::new(),
            sc_fbs: Vec::new(),
            sc_done: Vec::new(),
            sc_format: vk::Format::B8G8R8A8_UNORM,
            sc_extent: vk::Extent2D { width: w, height: h },
            present_mode: vk::PresentModeKHR::FIFO,
            color: Img::null(),
            depth: Img::null(),
            slots: Vec::new(),
            slot: 0,
            index: 0,
            recording: false,
            first: true,
            present_pass: vk::RenderPass::null(),
            clear_pass: vk::RenderPass::null(),
            clear_fb: vk::Framebuffer::null(),
            dsl: [vk::DescriptorSetLayout::null(); 2],
            layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            sampler: vk::Sampler::null(),
            ramp: Img::null(),
            ramp_values: None,
            cache,
            cache_saved,
            exported: VkDevice8 {
                get_instance_proc_addr: gipa,
                instance: 0,
                physical_device: 0,
                device: 0,
                queue: 0,
                queue_family: qfam,
                api_version: props.api_version,
                pipeline_cache: 0,
                frames_in_flight: SLOTS as u32,
                features,
                uniform_align: props.limits.min_uniform_buffer_offset_alignment as u32,
                sampler_anisotropy: samplers.0,
                sampler_lod_bias: samplers.1,
            },
            frame_out: VkFrame8::default(),
            uniform_align: props.limits.min_uniform_buffer_offset_alignment as u32,
            stale: true,
            messenger: (None, vk::DebugUtilsMessengerEXT::null()),
        };
        me.exported.instance = vk::Handle::as_raw(me.instance.handle()) as usize;
        me.exported.physical_device = vk::Handle::as_raw(pdev) as usize;
        me.exported.device = vk::Handle::as_raw(me.dev.handle()) as usize;
        me.exported.queue = vk::Handle::as_raw(queue) as usize;
        me.exported.pipeline_cache = vk::Handle::as_raw(cache);
        me.present_mode = me.choose_present_mode(interval);
        // `me` owns everything from here: a failure destroys it whole.
        me.messenger = (part.du.take(), core::mem::replace(&mut part.messenger, vk::DebugUtilsMessengerEXT::null()));
        core::mem::forget(part);
        if let Err(e) = me.build(w, h) {
            me.destroy();
            return Err(e);
        }
        log!(
            "vulkan: swap chain {}x{} format {}, present mode {}; targets {}x{} B8G8R8A8 + D16",
            me.sc_extent.width, me.sc_extent.height, me.sc_format.as_raw(), me.present_mode.as_raw(), w, h
        );
        unsafe { VK = Some(me) };
        Ok(())
    }

    /// The targets, passes, swap chain, pipeline and frame slots of a device just made.
    fn build(&mut self, w: u32, h: u32) -> Result<(), &'static str> {
        let me = self;
        me.color = me.image(w, h, 1, vk::Format::B8G8R8A8_UNORM, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, vk::ImageAspectFlags::COLOR)?;
        me.depth = me.image(
            w,
            h,
            1,
            vk::Format::D16_UNORM,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageAspectFlags::DEPTH,
        )?;
        me.ramp = me.image(256, 1, 1, vk::Format::B8G8R8A8_UNORM, vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST, vk::ImageAspectFlags::COLOR)?;
        me.make_passes()?;
        me.make_swapchain()?;
        me.make_pipeline()?;
        for _ in 0..SLOTS {
            let s = me.make_slot()?;
            me.slots.push(s);
        }
        // The bench's deliberate failure (FailBackend=vulkan), after everything is made,
        // so that the teardown is what the next backend starts after.
        if crate::fail_backend(crate::stub::Backend::Vulkan) {
            return Err("FailBackend=vulkan (the test bench's deliberate failure)");
        }
        Ok(())
    }

    /// Destroys everything `start` made, when it fails: the window is free again for the
    /// next backend (a surface or swap chain left on it keeps another's from being made).
    /// Null handles are no-ops in Vulkan.
    fn destroy(mut self) {
        unsafe {
            let _ = self.dev.device_wait_idle();
            for s in self.slots.drain(..) {
                for c in s.chunks {
                    self.dev.destroy_buffer(c.buf, None);
                    self.dev.free_memory(c.mem, None);
                }
                self.dev.destroy_semaphore(s.acquired, None);
                self.dev.destroy_descriptor_pool(s.dpool, None);
                self.dev.destroy_fence(s.fence, None);
                self.dev.destroy_command_pool(s.pool, None);
            }
            self.dev.destroy_pipeline(self.pipeline, None);
            self.dev.destroy_pipeline_layout(self.layout, None);
            for l in self.dsl {
                self.dev.destroy_descriptor_set_layout(l, None);
            }
            self.dev.destroy_sampler(self.sampler, None);
            for &fb in &self.sc_fbs {
                self.dev.destroy_framebuffer(fb, None);
            }
            for &v in &self.sc_views {
                self.dev.destroy_image_view(v, None);
            }
            for &s in &self.sc_done {
                self.dev.destroy_semaphore(s, None);
            }
            self.swap_fn.destroy_swapchain(self.swapchain, None);
            self.dev.destroy_render_pass(self.present_pass, None);
            self.dev.destroy_framebuffer(self.clear_fb, None);
            self.dev.destroy_render_pass(self.clear_pass, None);
            for img in [&self.color, &self.depth, &self.ramp] {
                self.dev.destroy_image_view(img.view, None);
                self.dev.destroy_image(img.image, None);
                self.dev.free_memory(img.mem, None);
            }
            self.dev.destroy_pipeline_cache(self.cache, None);
            self.dev.destroy_device(None);
            self.surface_fn.destroy_surface(self.surface, None);
            if let Some(du) = self.messenger.0.take() {
                du.destroy_debug_utils_messenger(self.messenger.1, None);
            }
            self.instance.destroy_instance(None);
        }
        log!("vulkan: what was made is destroyed again");
    }

    fn choose_present_mode(&self, interval: u32) -> vk::PresentModeKHR {
        let modes = unsafe { self.surface_fn.get_physical_device_surface_present_modes(self.pdev, self.surface) }.unwrap_or_default();
        // D3DPRESENT_INTERVAL_DEFAULT (0) and ONE: the vertical blank. IMMEDIATE (0x80000000):
        // mailbox, else immediate, by what the surface offers.
        if interval & 0x8000_0000 != 0 {
            for m in [vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE] {
                if modes.contains(&m) {
                    return m;
                }
            }
        }
        vk::PresentModeKHR::FIFO
    }

    pub fn image(&self, w: u32, h: u32, levels: u32, format: vk::Format, usage: vk::ImageUsageFlags, aspect: vk::ImageAspectFlags) -> Result<Img, &'static str> {
        let ici = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width: w, height: h, depth: 1 })
            .mip_levels(levels)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image = unsafe { self.dev.create_image(&ici, None) }.map_err(|_| "vkCreateImage failed")?;
        let req = unsafe { self.dev.get_image_memory_requirements(image) };
        let ty = find_memory(&self.mem, req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL).ok_or("no device-local memory")?;
        let mem = unsafe { self.dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) }
            .map_err(|_| "vkAllocateMemory failed (image)")?;
        unsafe { self.dev.bind_image_memory(image, mem, 0) }.map_err(|_| "vkBindImageMemory failed")?;
        let vci = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: 0, level_count: levels, base_array_layer: 0, layer_count: 1 });
        let view = unsafe { self.dev.create_image_view(&vci, None) }.map_err(|_| "vkCreateImageView failed")?;
        Ok(Img { image, view, mem, format, w, h, levels, layout: vk::ImageLayout::UNDEFINED })
    }

    fn make_passes(&mut self) -> Result<(), &'static str> {
        // The clear pass: the engine's Clear of its targets, by rectangles.
        let att = [
            vk::AttachmentDescription::default()
                .format(self.color.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::LOAD)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            vk::AttachmentDescription::default()
                .format(self.depth.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::LOAD)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];
        let cref = [vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }];
        let dref = vk::AttachmentReference { attachment: 1, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };
        let sub = [vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&cref).depth_stencil_attachment(&dref)];
        self.clear_pass = unsafe { self.dev.create_render_pass(&vk::RenderPassCreateInfo::default().attachments(&att).subpasses(&sub), None) }
            .map_err(|_| "vkCreateRenderPass failed (clear)")?;
        let views = [self.color.view, self.depth.view];
        self.clear_fb = unsafe {
            self.dev.create_framebuffer(
                &vk::FramebufferCreateInfo::default().render_pass(self.clear_pass).attachments(&views).width(self.color.w).height(self.color.h).layers(1),
                None,
            )
        }
        .map_err(|_| "vkCreateFramebuffer failed (clear)")?;
        Ok(())
    }

    fn make_present_pass(&mut self) -> Result<(), &'static str> {
        let att = [vk::AttachmentDescription::default()
            .format(self.sc_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::DONT_CARE)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let cref = [vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }];
        let sub = [vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&cref)];
        let dep = [vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
        self.present_pass = unsafe { self.dev.create_render_pass(&vk::RenderPassCreateInfo::default().attachments(&att).subpasses(&sub).dependencies(&dep), None) }
            .map_err(|_| "vkCreateRenderPass failed (present)")?;
        Ok(())
    }

    fn make_swapchain(&mut self) -> Result<(), &'static str> {
        let caps = unsafe { self.surface_fn.get_physical_device_surface_capabilities(self.pdev, self.surface) }.map_err(|_| "no surface capabilities")?;
        let formats = unsafe { self.surface_fn.get_physical_device_surface_formats(self.pdev, self.surface) }.map_err(|_| "no surface formats")?;
        let pick = formats
            .iter()
            .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
            .or_else(|| formats.iter().find(|f| f.format == vk::Format::R8G8B8A8_UNORM))
            .or(formats.first())
            .copied()
            .ok_or("the surface offers no format")?;
        let old_format = self.sc_format;
        self.sc_format = pick.format;
        let extent = if caps.current_extent.width != u32::MAX { caps.current_extent } else { vk::Extent2D { width: self.color.w, height: self.color.h } };
        if extent.width == 0 || extent.height == 0 {
            return Err("the window has no size");
        }
        self.sc_extent = extent;
        let mut count = caps.min_image_count + 1;
        if caps.max_image_count != 0 {
            count = count.min(caps.max_image_count);
        }
        let old = self.swapchain;
        let ci = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(count)
            .image_format(pick.format)
            .image_color_space(pick.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode)
            .clipped(true)
            .old_swapchain(old);
        self.swapchain = unsafe { self.swap_fn.create_swapchain(&ci, None) }.map_err(|_| "vkCreateSwapchainKHR failed")?;
        unsafe {
            for &fb in &self.sc_fbs {
                self.dev.destroy_framebuffer(fb, None);
            }
            for &v in &self.sc_views {
                self.dev.destroy_image_view(v, None);
            }
            for &s in &self.sc_done {
                self.dev.destroy_semaphore(s, None);
            }
            if old != vk::SwapchainKHR::null() {
                self.swap_fn.destroy_swapchain(old, None);
            }
        }
        if self.present_pass == vk::RenderPass::null() || old_format != self.sc_format {
            self.make_present_pass()?;
        }
        self.sc_images = unsafe { self.swap_fn.get_swapchain_images(self.swapchain) }.map_err(|_| "no swap chain images")?;
        self.sc_views.clear();
        self.sc_fbs.clear();
        self.sc_done.clear();
        for &img in &self.sc_images {
            let vci = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.sc_format)
                .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
            let v = unsafe { self.dev.create_image_view(&vci, None) }.map_err(|_| "vkCreateImageView failed (swap chain)")?;
            let views = [v];
            let fb = unsafe {
                self.dev.create_framebuffer(
                    &vk::FramebufferCreateInfo::default().render_pass(self.present_pass).attachments(&views).width(extent.width).height(extent.height).layers(1),
                    None,
                )
            }
            .map_err(|_| "vkCreateFramebuffer failed (swap chain)")?;
            let s = unsafe { self.dev.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.map_err(|_| "vkCreateSemaphore failed")?;
            self.sc_views.push(v);
            self.sc_fbs.push(fb);
            self.sc_done.push(s);
        }
        Ok(())
    }

    fn make_pipeline(&mut self) -> Result<(), &'static str> {
        let b0 = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];
        let b1 = [
            vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        self.dsl[0] = unsafe { self.dev.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b0), None) }.map_err(|_| "dsl0")?;
        self.dsl[1] = unsafe { self.dev.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b1), None) }.map_err(|_| "dsl1")?;
        self.layout = unsafe { self.dev.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().set_layouts(&self.dsl), None) }.map_err(|_| "pipeline layout")?;
        self.sampler = unsafe {
            self.dev.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .max_lod(0.0),
                None,
            )
        }
        .map_err(|_| "sampler")?;
        let vs = unsafe { self.dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&VS_PRESENT_SPV), None) }.map_err(|_| "vs module")?;
        let ps = unsafe { self.dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&PS_GAMMA_SPV), None) }.map_err(|_| "ps module")?;
        let stages = [
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vs).name(c"vs_present"),
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(ps).name(c"ps_gamma"),
        ];
        let vb = [vk::VertexInputBindingDescription { binding: 0, stride: 16, input_rate: vk::VertexInputRate::VERTEX }];
        let va = [
            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 },
            vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 8 },
        ];
        let vi = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&vb).vertex_attribute_descriptions(&va);
        let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_STRIP);
        let vp = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
        let rs = vk::PipelineRasterizationStateCreateInfo::default().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).line_width(1.0);
        let ms = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let cba = [vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA)];
        let cb = vk::PipelineColorBlendStateCreateInfo::default().attachments(&cba);
        let dyns = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let ds = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyns);
        let gpci = [vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vi)
            .input_assembly_state(&ia)
            .viewport_state(&vp)
            .rasterization_state(&rs)
            .multisample_state(&ms)
            .color_blend_state(&cb)
            .dynamic_state(&ds)
            .layout(self.layout)
            .render_pass(self.present_pass)
            .subpass(0)];
        let p = unsafe { self.dev.create_graphics_pipelines(self.cache, &gpci, None) }.map_err(|_| "the present pipeline")?;
        self.pipeline = p[0];
        unsafe {
            self.dev.destroy_shader_module(vs, None);
            self.dev.destroy_shader_module(ps, None);
        }
        Ok(())
    }

    fn make_slot(&self) -> Result<Slot, &'static str> {
        let pool = unsafe {
            self.dev.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(self.qfam).flags(vk::CommandPoolCreateFlags::TRANSIENT), None)
        }
        .map_err(|_| "command pool")?;
        let cmd = unsafe {
            self.dev.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default().command_pool(pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1))
        }
        .map_err(|_| "command buffer")?[0];
        let fence = unsafe { self.dev.create_fence(&vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED), None) }.map_err(|_| "fence")?;
        let sizes = [
            vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 16384 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, descriptor_count: 4096 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, descriptor_count: 32768 },
        ];
        let dpool = unsafe { self.dev.create_descriptor_pool(&vk::DescriptorPoolCreateInfo::default().max_sets(16384).pool_sizes(&sizes), None) }
            .map_err(|_| "descriptor pool")?;
        let acquired = unsafe { self.dev.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.map_err(|_| "semaphore")?;
        let mut s = Slot { pool, cmd, fence, dpool, acquired, chunks: Vec::new(), at: (0, 0), deferred: Vec::new() };
        s.chunks.push(self.chunk(CHUNK)?);
        Ok(s)
    }

    fn chunk(&self, size: u64) -> Result<Chunk, &'static str> {
        let bci = vk::BufferCreateInfo::default()
            .size(size)
            .usage(
                vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::INDEX_BUFFER
                    | vk::BufferUsageFlags::UNIFORM_BUFFER
                    | vk::BufferUsageFlags::TRANSFER_SRC,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { self.dev.create_buffer(&bci, None) }.map_err(|_| "ring buffer")?;
        let req = unsafe { self.dev.get_buffer_memory_requirements(buf) };
        let ty = find_memory(&self.mem, req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
            .ok_or("no host-visible memory")?;
        let mem = unsafe { self.dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) }
            .map_err(|_| "vkAllocateMemory failed (ring)")?;
        unsafe { self.dev.bind_buffer_memory(buf, mem, 0) }.map_err(|_| "bind ring")?;
        let ptr = unsafe { self.dev.map_memory(mem, 0, size, vk::MemoryMapFlags::empty()) }.map_err(|_| "map ring")? as *mut u8;
        Ok(Chunk { buf, mem, ptr, size })
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        self.slots[self.slot].cmd
    }

    /// The frame now, begun if no frame is being recorded.
    pub fn frame(&mut self) -> &VkFrame8 {
        if !self.recording {
            self.begin();
        }
        &self.frame_out
    }

    fn begin(&mut self) {
        let s = &mut self.slots[self.slot];
        unsafe {
            let _ = self.dev.wait_for_fences(&[s.fence], true, u64::MAX);
        }
        for (f, arg) in s.deferred.drain(..) {
            f(arg);
        }
        unsafe {
            let _ = self.dev.reset_command_pool(s.pool, vk::CommandPoolResetFlags::empty());
            let _ = self.dev.reset_descriptor_pool(s.dpool, vk::DescriptorPoolResetFlags::empty());
            let _ = self.dev.begin_command_buffer(s.cmd, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT));
        }
        s.at = (0, 0);
        self.recording = true;
        self.frame_out = VkFrame8 {
            index: self.index,
            slot: self.slot as u32,
            command_buffer: vk::Handle::as_raw(s.cmd) as usize,
            descriptor_pool: vk::Handle::as_raw(s.dpool),
        };
        if self.first {
            self.first = false;
            let (c, d) = (self.color.image, self.depth.image);
            self.barrier(c, vk::ImageAspectFlags::COLOR, vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            self.barrier(d, vk::ImageAspectFlags::DEPTH, vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
            self.color.layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
            self.depth.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
            let r = self.ramp.image;
            self.barrier(r, vk::ImageAspectFlags::COLOR, vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
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

    /// A layout change of a whole image, with the broad access masks of a frame that
    /// is recorded once: correct, if not the tightest.
    pub fn barrier(&self, image: vk::Image, aspect: vk::ImageAspectFlags, from: vk::ImageLayout, to: vk::ImageLayout) {
        self.barrier_levels(image, aspect, 0, vk::REMAINING_MIP_LEVELS, from, to);
    }

    pub fn barrier_levels(&self, image: vk::Image, aspect: vk::ImageAspectFlags, level: u32, count: u32, from: vk::ImageLayout, to: vk::ImageLayout) {
        let b = [vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
            .old_layout(from)
            .new_layout(to)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: level, level_count: count, base_array_layer: 0, layer_count: 1 })];
        unsafe {
            self.dev.cmd_pipeline_barrier(
                self.cmd(),
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &b,
            )
        };
    }

    /// Space in the frame's upload ring.
    pub fn alloc(&mut self, bytes: u32, align: u32) -> Option<VkAlloc8> {
        if !self.recording {
            self.begin();
        }
        let align = align.max(4) as u64;
        let need = bytes as u64;
        let fresh = self.chunk_for(need, align)?;
        let s = &mut self.slots[self.slot];
        let (ci, off) = (fresh.0, fresh.1);
        s.at = (ci, off + need);
        let c = &s.chunks[ci];
        Some(VkAlloc8 { buffer: vk::Handle::as_raw(c.buf), offset: off as u32, cpu: unsafe { c.ptr.add(off as usize) } })
    }

    fn chunk_for(&mut self, need: u64, align: u64) -> Option<(usize, u64)> {
        let (mut ci, at) = self.slots[self.slot].at;
        let mut off = at.div_ceil(align) * align;
        loop {
            let len = self.slots[self.slot].chunks.len();
            if ci < len {
                let size = self.slots[self.slot].chunks[ci].size;
                if off + need <= size {
                    return Some((ci, off));
                }
                ci += 1;
                off = 0;
                continue;
            }
            let c = match self.chunk(need.max(CHUNK)) {
                Ok(c) => c,
                Err(e) => {
                    log!("vulkan: the upload ring could not grow: {}", e);
                    return None;
                }
            };
            log!("vulkan: the upload ring of frame slot {} grew to {} chunks", self.slot, len + 1);
            self.slots[self.slot].chunks.push(c);
        }
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
        pass::close(self);
        let (w, h) = (self.color.w as i32, self.color.h as i32);
        let mut rs: Vec<vk::ClearRect> = Vec::new();
        let vr = [viewport[0] as i32, viewport[1] as i32, (viewport[0] + viewport[2]) as i32, (viewport[1] + viewport[3]) as i32];
        let list: Vec<[i32; 4]> = if rects.is_empty() { alloc::vec![vr] } else { rects.to_vec() };
        for r in list {
            let (x0, y0, x1, y1) = (r[0].max(0).max(vr[0]), r[1].max(0).max(vr[1]), r[2].min(w).min(vr[2]), r[3].min(h).min(vr[3]));
            if x1 > x0 && y1 > y0 {
                rs.push(vk::ClearRect {
                    rect: vk::Rect2D { offset: vk::Offset2D { x: x0, y: y0 }, extent: vk::Extent2D { width: (x1 - x0) as u32, height: (y1 - y0) as u32 } },
                    base_array_layer: 0,
                    layer_count: 1,
                });
            }
        }
        if rs.is_empty() {
            return;
        }
        let mut atts = Vec::new();
        if flags & 1 != 0 {
            let c = [((color >> 16) & 255) as f32 / 255.0, ((color >> 8) & 255) as f32 / 255.0, (color & 255) as f32 / 255.0, (color >> 24) as f32 / 255.0];
            atts.push(vk::ClearAttachment { aspect_mask: vk::ImageAspectFlags::COLOR, color_attachment: 0, clear_value: vk::ClearValue { color: vk::ClearColorValue { float32: c } } });
        }
        if flags & 2 != 0 {
            atts.push(vk::ClearAttachment {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                color_attachment: 0,
                clear_value: vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: z, stencil: 0 } },
            });
        }
        if atts.is_empty() {
            return;
        }
        let cmd = self.cmd();
        let rp = vk::RenderPassBeginInfo::default()
            .render_pass(self.clear_pass)
            .framebuffer(self.clear_fb)
            .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: self.color.w, height: self.color.h } });
        unsafe {
            self.dev.cmd_begin_render_pass(cmd, &rp, vk::SubpassContents::INLINE);
            self.dev.cmd_clear_attachments(cmd, &atts, &rs);
            self.dev.cmd_end_render_pass(cmd);
        }
    }

    /// Pixels into an image's level (`B8G8R8A8`-sized texels, `pitch` bytes a row), through
    /// the ring: a mirror's level, the engine's back buffer written by lock.
    pub fn upload(&mut self, img: vk::Image, aspect: vk::ImageAspectFlags, level: u32, w: u32, h: u32, src: &[u8], pitch: u32, bpp: u32, from: vk::ImageLayout, to: vk::ImageLayout) -> bool {
        if !self.recording {
            self.begin();
        }
        pass::close(self);
        let row = w * bpp;
        let Some(a) = self.alloc(row * h, 16) else { return false };
        for y in 0..h as usize {
            unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add(y * pitch as usize), a.cpu.add(y * row as usize), row as usize) };
        }
        self.barrier_levels(img, aspect, level, 1, from, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        let region = [vk::BufferImageCopy::default()
            .buffer_offset(a.offset as u64)
            .image_subresource(vk::ImageSubresourceLayers { aspect_mask: aspect, mip_level: level, base_array_layer: 0, layer_count: 1 })
            .image_extent(vk::Extent3D { width: w, height: h, depth: 1 })];
        unsafe {
            self.dev.cmd_copy_buffer_to_image(self.cmd(), vk::Handle::from_raw(a.buffer), img, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &region);
        }
        self.barrier_levels(img, aspect, level, 1, vk::ImageLayout::TRANSFER_DST_OPTIMAL, to);
        true
    }

    /// Levels of a colour image (`B8G8R8A8`-sized texels: level, width, height, pixels,
    /// pitch) through a staging buffer of their own, let go of once the frame is done.
    /// False, with nothing recorded, when the buffer cannot be made.
    pub fn upload_levels<'a>(&mut self, img: vk::Image, levels: impl Iterator<Item = (u32, u32, u32, &'a [u8], u32)>, from: vk::ImageLayout, to: vk::ImageLayout) -> bool {
        if !self.recording {
            self.begin();
        }
        pass::close(self);
        let levels: Vec<(u32, u32, u32, &[u8], u32)> = levels.collect();
        let mut at = 0u64;
        let mut offsets = Vec::with_capacity(levels.len());
        for &(_, w, h, _, _) in &levels {
            offsets.push(at);
            at += (w as u64 * h as u64 * 4).div_ceil(16) * 16;
        }
        if at == 0 {
            return true;
        }
        let bci = vk::BufferCreateInfo::default().size(at).usage(vk::BufferUsageFlags::TRANSFER_SRC).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let Ok(buf) = (unsafe { self.dev.create_buffer(&bci, None) }) else { return false };
        let req = unsafe { self.dev.get_buffer_memory_requirements(buf) };
        let made = find_memory(&self.mem, req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT).and_then(|ty| {
            let mem = unsafe { self.dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) }.ok()?;
            if unsafe { self.dev.bind_buffer_memory(buf, mem, 0) }.is_err() {
                unsafe { self.dev.free_memory(mem, None) };
                return None;
            }
            match unsafe { self.dev.map_memory(mem, 0, at, vk::MemoryMapFlags::empty()) } {
                Ok(p) => Some((mem, p as *mut u8)),
                Err(_) => {
                    unsafe { self.dev.free_memory(mem, None) };
                    None
                }
            }
        });
        let Some((mem, ptr)) = made else {
            unsafe { self.dev.destroy_buffer(buf, None) };
            return false;
        };
        for (n, &(_, w, h, src, pitch)) in levels.iter().enumerate() {
            let row = (w * 4) as usize;
            for y in 0..h as usize {
                unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add(y * pitch as usize), ptr.add(offsets[n] as usize + y * row), row) };
            }
        }
        unsafe { self.dev.unmap_memory(mem) };
        let aspect = vk::ImageAspectFlags::COLOR;
        for (n, &(level, w, h, _, _)) in levels.iter().enumerate() {
            self.barrier_levels(img, aspect, level, 1, from, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            let region = [vk::BufferImageCopy::default()
                .buffer_offset(offsets[n])
                .image_subresource(vk::ImageSubresourceLayers { aspect_mask: aspect, mip_level: level, base_array_layer: 0, layer_count: 1 })
                .image_extent(vk::Extent3D { width: w, height: h, depth: 1 })];
            unsafe { self.dev.cmd_copy_buffer_to_image(self.cmd(), buf, img, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &region) };
            self.barrier_levels(img, aspect, level, 1, vk::ImageLayout::TRANSFER_DST_OPTIMAL, to);
        }
        extern "C" fn free(p: *mut c_void) {
            let b = unsafe { alloc::boxed::Box::from_raw(p as *mut (vk::Buffer, vk::DeviceMemory)) };
            if let Some(v) = get() {
                unsafe {
                    v.dev.destroy_buffer(b.0, None);
                    v.dev.free_memory(b.1, None);
                }
            }
        }
        self.defer(free, alloc::boxed::Box::into_raw(alloc::boxed::Box::new((buf, mem))) as *mut c_void);
        true
    }

    pub fn set_ramp(&mut self, r: &[u16; 768]) {
        self.ramp_values = Some(*r);
        self.stale = true;
    }

    /// The engine's `Present`: the last pass into the swap chain's image, then submit
    /// and present.
    pub fn present(&mut self) -> bool {
        if !self.recording {
            self.begin();
        }
        pass::presented(self);
        if self.stale {
            self.stale = false;
            if let Some(r) = self.ramp_values {
                let mut px = [0u8; 1024];
                for i in 0..256 {
                    let hi = |v: u16| (v >> 8) as u8; // as Direct3D 9's stub draws it (stub/gamma.rs)
                    px[i * 4..i * 4 + 4].copy_from_slice(&[hi(r[512 + i]), hi(r[256 + i]), hi(r[i]), 255]);
                }
                let ramp = self.ramp.image;
                self.upload(ramp, vk::ImageAspectFlags::COLOR, 0, 256, 1, &px, 1024, 4, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            }
        }
        let s = self.slot;
        let acquired = self.slots[s].acquired;
        let image = match unsafe { self.swap_fn.acquire_next_image(self.swapchain, 5_000_000_000, acquired, vk::Fence::null()) } {
            Ok((i, _)) => i,
            Err(e) => {
                log!("vulkan: no swap chain image ({:?}); the swap chain is made again", e);
                let _ = unsafe { self.dev.device_wait_idle() };
                if self.make_swapchain().is_err() {
                    return false;
                }
                match unsafe { self.swap_fn.acquire_next_image(self.swapchain, 5_000_000_000, acquired, vk::Fence::null()) } {
                    Ok((i, _)) => i,
                    Err(_) => return false,
                }
            }
        };
        // The last pass: the colour target through the ramp, into the swap chain's image.
        let cmd = self.cmd();
        let color = self.color.image;
        self.barrier(color, vk::ImageAspectFlags::COLOR, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let quad: [f32; 16] = [-1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0];
        let shift = [0f32; 4];
        let ua = self.uniform_align;
        let (Some(va), Some(ub)) = (self.alloc(64, 16), self.alloc(16, ua)) else { return false };
        unsafe {
            core::ptr::copy_nonoverlapping(quad.as_ptr() as *const u8, va.cpu, 64);
            core::ptr::copy_nonoverlapping(shift.as_ptr() as *const u8, ub.cpu, 16);
        }
        let dpool = self.slots[s].dpool;
        let sets = unsafe { self.dev.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(dpool).set_layouts(&self.dsl)) };
        let Ok(sets) = sets else { return false };
        let bi = [vk::DescriptorBufferInfo { buffer: vk::Handle::from_raw(ub.buffer), offset: ub.offset as u64, range: 16 }];
        let ii0 = [vk::DescriptorImageInfo { sampler: self.sampler, image_view: self.color.view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
        let ii1 = [vk::DescriptorImageInfo { sampler: self.sampler, image_view: self.ramp.view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
        let writes = [
            vk::WriteDescriptorSet::default().dst_set(sets[0]).dst_binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER).buffer_info(&bi),
            vk::WriteDescriptorSet::default().dst_set(sets[1]).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&ii0),
            vk::WriteDescriptorSet::default().dst_set(sets[1]).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&ii1),
        ];
        let ext = self.sc_extent;
        unsafe {
            self.dev.update_descriptor_sets(&writes, &[]);
            let rp = vk::RenderPassBeginInfo::default()
                .render_pass(self.present_pass)
                .framebuffer(self.sc_fbs[image as usize])
                .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: ext });
            self.dev.cmd_begin_render_pass(cmd, &rp, vk::SubpassContents::INLINE);
            self.dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            self.dev.cmd_set_viewport(cmd, 0, &[vk::Viewport { x: 0.0, y: 0.0, width: ext.width as f32, height: ext.height as f32, min_depth: 0.0, max_depth: 1.0 }]);
            self.dev.cmd_set_scissor(cmd, 0, &[vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: ext }]);
            self.dev.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::GRAPHICS, self.layout, 0, &sets, &[]);
            self.dev.cmd_bind_vertex_buffers(cmd, 0, &[vk::Handle::from_raw(va.buffer)], &[va.offset as u64]);
            self.dev.cmd_draw(cmd, 4, 1, 0, 0);
            self.dev.cmd_end_render_pass(cmd);
        }
        self.barrier(color, vk::ImageAspectFlags::COLOR, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let done = self.sc_done[image as usize];
        let fence = self.slots[s].fence;
        unsafe {
            let _ = self.dev.end_command_buffer(cmd);
            let _ = self.dev.reset_fences(&[fence]);
            let waits = [acquired];
            let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let cmds = [cmd];
            let signals = [done];
            let submit = [vk::SubmitInfo::default().wait_semaphores(&waits).wait_dst_stage_mask(&stages).command_buffers(&cmds).signal_semaphores(&signals)];
            if self.dev.queue_submit(self.queue, &submit, fence).is_err() {
                log!("vulkan: vkQueueSubmit failed");
            }
            let scs = [self.swapchain];
            let idx = [image];
            let pi = vk::PresentInfoKHR::default().wait_semaphores(&signals).swapchains(&scs).image_indices(&idx);
            match self.swap_fn.queue_present(self.queue, &pi) {
                Ok(false) => {}
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    let _ = self.dev.device_wait_idle();
                    let _ = self.make_swapchain();
                }
                Err(e) => log!("vulkan: vkQueuePresentKHR: {:?}", e),
            }
        }
        self.recording = false;
        self.slot = (self.slot + 1) % SLOTS;
        self.index += 1;
        if self.index % cache::EVERY == 0 {
            self.cache_saved = cache::save(&self.dev, self.cache, self.cache_saved);
        }
        true
    }

    /// Submits what the frame has recorded and waits for it, then begins its command
    /// buffer again (`display8_vk_flush`: a plugin's read-back). The frame's ring, pool
    /// and deferred calls stay as they are; its fence is left signalled, as it was.
    pub fn flush(&mut self) -> bool {
        if !self.recording {
            return true;
        }
        pass::close(self);
        let s = &self.slots[self.slot];
        let (cmd, fence, pool) = (s.cmd, s.fence, s.pool);
        unsafe {
            if self.dev.end_command_buffer(cmd).is_err() {
                log!("vulkan: flush: vkEndCommandBuffer failed");
                return false;
            }
            let _ = self.dev.reset_fences(&[fence]);
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::default().command_buffers(&cmds)];
            if self.dev.queue_submit(self.queue, &submit, fence).is_err() {
                log!("vulkan: flush: vkQueueSubmit failed");
                return false;
            }
            let _ = self.dev.wait_for_fences(&[fence], true, u64::MAX);
            // The pool's one command buffer, begun again (a transient pool's buffers are
            // reset with their pool).
            let _ = self.dev.reset_command_pool(pool, vk::CommandPoolResetFlags::empty());
            let _ = self.dev.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT));
        }
        true
    }

    pub fn target(&self, which: u32) -> VkImage8 {
        let (i, layout) = if which == gpu::display::TARGET_DEPTH {
            (&self.depth, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        } else {
            (&self.color, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        };
        VkImage8 {
            image: vk::Handle::as_raw(i.image),
            view: vk::Handle::as_raw(i.view),
            format: i.format.as_raw() as u32,
            width: i.w,
            height: i.h,
            samples: 1,
            layout: layout.as_raw() as u32,
        }
    }
}

impl Img {
    pub const fn null() -> Img {
        Img {
            image: vk::Image::null(),
            view: vk::ImageView::null(),
            mem: vk::DeviceMemory::null(),
            format: vk::Format::UNDEFINED,
            w: 0,
            h: 0,
            levels: 0,
            layout: vk::ImageLayout::UNDEFINED,
        }
    }
}
