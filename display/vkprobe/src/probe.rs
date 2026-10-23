//! The probe itself, step by step, in the order Display8 brings Vulkan up
//! (`display/src/vk/mod.rs`): the loader, an instance, a win32 surface, a device that
//! draws and presents, a swap chain. Then the part that decides macOS: one 8 MB
//! host-visible, coherent buffer, mapped once and kept mapped, which every frame writes
//! its vertices, indices, uniform block and a texture upload into, as a ring two frames
//! in flight share (Display8's upload ring). Each frame draws into an offscreen target
//! (a checker quad sampled from the uploaded texture, a triangle whose position and
//! colour are a uniform block at a dynamic offset) and then that target into the swap
//! chain, as Display8's present pass does. The last frame's target is copied into a
//! second mapped buffer and its pixels checked against what the shaders must make.
use crate::report::{self, begin, fail, fmt, note, pass, section};
use crate::shaders::probe_spv::{PS_TEX_SPV, PS_TRI_SPV, VS_QUAD_SPV, VS_TRI_SPV};
use crate::user32;
use crate::win::*;
use alloc::string::String;
use alloc::vec::Vec;
use ash::vk;
use core::ffi::c_void;

const W: u32 = 640;
const H: u32 = 480;
const FRAMES: u32 = 300;
const SLOTS: usize = 2;
/// The ring: as big as a frame of the game's would want, with room to wrap.
const RING: u64 = 8 << 20;
/// What each frame writes into the ring besides its draws, standing in for the engine's
/// dynamic vertices: enough that the ring wraps three times in 300 frames.
const FILLER: u64 = 64 << 10;
const TEX: u32 = 64;
/// Five seconds, for every wait: a probe that hangs says so.
const TIMEOUT: u64 = 5_000_000_000;
const UBO_SIZE: u64 = 32;

const CLEAR: [f32; 4] = [0.0, 0.0, 0.2, 1.0];
const TRI_COLOR: [f32; 4] = [1.0, 0.8, 0.4, 1.0];
/// The triangle's tint, one per frame in turn: the last frame's is what the readback must
/// show, so a uniform block read from another frame's place in the ring shows up.
const TINTS: [[f32; 3]; 3] = [[1.0, 1.0, 1.0], [0.6, 1.0, 0.2], [0.2, 0.4, 1.0]];
/// The checker's two colours (RGBA); which comes first alternates frame by frame, so the
/// readback also shows that the last frame's upload is the one sampled.
const CHECK_A: [u8; 4] = [255, 204, 0, 255];
const CHECK_B: [u8; 4] = [0, 102, 255, 255];

type R<T> = Result<T, ()>;

fn vkres(r: vk::Result) -> String {
    fmt(format_args!("VK_{:?}", r))
}

/// A Vulkan call as a step: PASS with VK_SUCCESS, or FAIL with what it returned.
fn chk<T>(name: &str, r: ash::prelude::VkResult<T>) -> R<T> {
    match r {
        Ok(v) => {
            pass(name, "VK_SUCCESS");
            Ok(v)
        }
        Err(e) => {
            fail(name, &vkres(e));
            Err(())
        }
    }
}

/// A Vulkan call that is part of a step already named: FAIL only.
fn quiet<T>(name: &str, r: ash::prelude::VkResult<T>) -> R<T> {
    r.map_err(|e| fail(name, &vkres(e)))
}

fn version(v: u32) -> String {
    fmt(format_args!("{}.{}.{}", vk::api_version_major(v), vk::api_version_minor(v), vk::api_version_patch(v)))
}

fn name_of(raw: &[core::ffi::c_char]) -> String {
    report::cstr_at(raw.as_ptr() as *const u8, raw.len())
}

/// Names, several to a line.
fn list(names: &[String]) {
    let mut line = String::from(" ");
    for n in names {
        if line.len() + n.len() > 100 {
            report::line(&line);
            line = String::from(" ");
        }
        line.push(' ');
        line.push_str(n);
    }
    if line.len() > 1 {
        report::line(&line);
    }
}

static mut MESSAGES: Vec<String> = Vec::new();
static mut ERRORS: u32 = 0;
static mut WARNINGS: u32 = 0;

/// The validation layer's (and any driver's) messages, kept for the report.
unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _ty: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user: *mut c_void,
) -> vk::Bool32 {
    unsafe {
        let error = severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR);
        if error {
            ERRORS += 1;
        } else {
            WARNINGS += 1;
        }
        if MESSAGES.len() < 40 && !data.is_null() {
            let msg = report::cstr_at((*data).p_message as *const u8, 600);
            MESSAGES.push(fmt(format_args!("{} {}", if error { "ERROR" } else { "warning" }, msg)));
        }
    }
    vk::FALSE
}

fn find_memory(props: &vk::PhysicalDeviceMemoryProperties, bits: u32, want: vk::MemoryPropertyFlags) -> Option<u32> {
    (0..props.memory_type_count).find(|&i| bits & (1 << i) != 0 && props.memory_types[i as usize].property_flags.contains(want))
}

fn now() -> f64 {
    let (mut c, mut f) = (0i64, 0i64);
    unsafe {
        QueryPerformanceCounter(&mut c);
        QueryPerformanceFrequency(&mut f);
    }
    c as f64 / f as f64
}

fn align(v: u64, a: u64) -> u64 {
    v.div_ceil(a) * a
}

struct Img {
    image: vk::Image,
    view: vk::ImageView,
    mem: vk::DeviceMemory,
}

struct Buf {
    buf: vk::Buffer,
    mem: vk::DeviceMemory,
    ptr: *mut u8,
}

/// Everything the probe makes, so that it can be destroyed again in order (the
/// validation layer reports what is left behind) and the swap chain re-made.
struct Probe {
    entry: ash::Entry,
    instance: ash::Instance,
    debug: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    surface_fn: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    pdev: vk::PhysicalDevice,
    qfam: u32,
    mem: vk::PhysicalDeviceMemoryProperties,
    limits: vk::PhysicalDeviceLimits,
    dev: Option<ash::Device>,
    queue: vk::Queue,
    swap_fn: Option<ash::khr::swapchain::Device>,
    swapchain: vk::SwapchainKHR,
    sc_format: vk::SurfaceFormatKHR,
    sc_extent: vk::Extent2D,
    sc_views: Vec<vk::ImageView>,
    sc_fbs: Vec<vk::Framebuffer>,
    sc_done: Vec<vk::Semaphore>,
    composite: vk::CompositeAlphaFlagsKHR,
    present_mode: vk::PresentModeKHR,
    image_count: u32,
    ring: Option<Buf>,
    readback: Option<Buf>,
    tex: Option<Img>,
    target: Option<Img>,
    target_fb: vk::Framebuffer,
    target_pass: vk::RenderPass,
    present_pass: vk::RenderPass,
    dsl: [vk::DescriptorSetLayout; 2],
    layout: vk::PipelineLayout,
    pipelines: Vec<vk::Pipeline>,
    modules: Vec<vk::ShaderModule>,
    sampler: vk::Sampler,
    dpool: vk::DescriptorPool,
    pools: Vec<vk::CommandPool>,
    cmds: Vec<vk::CommandBuffer>,
    fences: Vec<vk::Fence>,
    acquired: Vec<vk::Semaphore>,
    /// The uniform block, the checker texture, the offscreen target.
    sets: [vk::DescriptorSet; 3],
}

pub fn run() -> bool {
    begin("open a window");
    let hwnd = user32::open(W as i32, H as i32);
    if hwnd.is_null() {
        fail("open a window", "CreateWindowExA failed");
        return false;
    }
    pass("open a window", "640x480");
    let mut p = None;
    let ok = go(hwnd, &mut p).is_ok();
    if let Some(p) = p.as_mut() {
        begin("tear down");
        p.destroy();
        pass("tear down", "");
    }
    validation_summary();
    user32::close(hwnd);
    ok && report_validation_ok()
}

fn report_validation_ok() -> bool {
    let errors = unsafe { ERRORS };
    if errors > 0 {
        fail("validation", &fmt(format_args!("{} error message(s); see \"validation messages\"", errors)));
        return false;
    }
    true
}

fn validation_summary() {
    let (e, w) = unsafe { (ERRORS, WARNINGS) };
    section("validation messages");
    report::line(&fmt(format_args!("{} errors, {} warnings (only with VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation, or a driver that reports)", e, w)));
    for m in unsafe { MESSAGES.iter() } {
        report::line(m);
    }
}

fn go(hwnd: HANDLE, slot: &mut Option<Probe>) -> R<()> {
    // The loader, as Display8 opens it: LoadLibraryA, then vkGetInstanceProcAddr.
    begin("load vulkan-1.dll");
    let m = unsafe { LoadLibraryA(c"vulkan-1.dll".as_ptr() as *const u8) };
    if m.is_null() {
        fail("load vulkan-1.dll", "LoadLibraryA failed: no Vulkan loader in this Wine or Windows");
        return Err(());
    }
    let gipa = unsafe { GetProcAddress(m, c"vkGetInstanceProcAddr".as_ptr() as *const u8) };
    if gipa == 0 {
        fail("load vulkan-1.dll", "no vkGetInstanceProcAddr");
        return Err(());
    }
    pass("load vulkan-1.dll", &fmt(format_args!("vkGetInstanceProcAddr at {:#010x}", gipa)));
    let entry = unsafe {
        ash::Entry::from_static_fn(ash::StaticFn { get_instance_proc_addr: core::mem::transmute::<usize, vk::PFN_vkGetInstanceProcAddr>(gipa) })
    };

    begin("vkCreateInstance");
    section("instance");
    let iv = unsafe { entry.try_enumerate_instance_version() }.ok().flatten().unwrap_or(vk::API_VERSION_1_0);
    out!("instance version: {}", version(iv));
    let layers: Vec<String> = unsafe { entry.enumerate_instance_layer_properties() }.unwrap_or_default().iter().map(|l| name_of(&l.layer_name)).collect();
    out!("layers ({}):", layers.len());
    list(&layers);
    let iexts: Vec<String> = unsafe { entry.enumerate_instance_extension_properties(None) }.unwrap_or_default().iter().map(|e| name_of(&e.extension_name)).collect();
    out!("instance extensions ({}):", iexts.len());
    list(&iexts);
    let has = |n: &core::ffi::CStr| iexts.iter().any(|e| e.as_bytes() == n.to_bytes());
    if iv < vk::API_VERSION_1_1 {
        fail("vkCreateInstance", &fmt(format_args!("the loader offers Vulkan {}; Display8 needs 1.1", version(iv))));
        return Err(());
    }
    let mut exts = alloc::vec![ash::khr::surface::NAME.as_ptr(), ash::khr::win32_surface::NAME.as_ptr()];
    for n in [ash::khr::surface::NAME, ash::khr::win32_surface::NAME] {
        if !has(n) {
            fail("vkCreateInstance", &fmt(format_args!("the instance does not offer {}", n.to_str().unwrap_or("?"))));
            return Err(());
        }
    }
    // MoltenVK is a portability driver: a loader of 1.3.216 or later lists it only to an
    // instance that enumerates portability drivers. Under Wine the loader is the host's,
    // behind winevulkan, so whether this is offered depends on the Wine.
    let mut flags = vk::InstanceCreateFlags::empty();
    let portability = has(ash::khr::portability_enumeration::NAME);
    if portability {
        exts.push(ash::khr::portability_enumeration::NAME.as_ptr());
        flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
    }
    let debug_utils = has(ash::ext::debug_utils::NAME);
    if debug_utils {
        exts.push(ash::ext::debug_utils::NAME.as_ptr());
    }
    let app = vk::ApplicationInfo::default().application_name(c"vkprobe").engine_name(c"Display8").api_version(vk::API_VERSION_1_1);
    let ici = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&exts).flags(flags);
    let instance = match unsafe { entry.create_instance(&ici, None) } {
        Ok(i) => i,
        Err(e) => {
            fail("vkCreateInstance", &vkres(e));
            return Err(());
        }
    };
    pass(
        "vkCreateInstance",
        &fmt(format_args!(
            "VK_SUCCESS, Vulkan 1.1, VK_KHR_surface + VK_KHR_win32_surface{}{}",
            if portability { " + VK_KHR_portability_enumeration (flag set)" } else { "" },
            if debug_utils { " + VK_EXT_debug_utils" } else { "" }
        )),
    );
    let debug = if debug_utils {
        let du = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let ci = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
            .pfn_user_callback(Some(debug_callback));
        match unsafe { du.create_debug_utils_messenger(&ci, None) } {
            Ok(m) => Some((du, m)),
            Err(_) => None,
        }
    } else {
        note("VK_EXT_debug_utils", "not offered: no validation messages can be collected");
        None
    };

    begin("vkCreateWin32SurfaceKHR");
    let surface_fn = ash::khr::surface::Instance::new(&entry, &instance);
    let w32 = ash::khr::win32_surface::Instance::new(&entry, &instance);
    let sci = vk::Win32SurfaceCreateInfoKHR::default()
        .hinstance(unsafe { GetModuleHandleA(core::ptr::null()) } as vk::HINSTANCE)
        .hwnd(hwnd as vk::HWND);
    let surface = match unsafe { w32.create_win32_surface(&sci, None) } {
        Ok(s) => {
            pass("vkCreateWin32SurfaceKHR", "VK_SUCCESS");
            s
        }
        Err(e) => {
            fail("vkCreateWin32SurfaceKHR", &vkres(e));
            unsafe {
                if let Some((du, m)) = &debug {
                    du.destroy_debug_utils_messenger(*m, None);
                }
                instance.destroy_instance(None);
            }
            return Err(());
        }
    };
    *slot = Some(Probe {
        entry,
        instance,
        debug,
        surface_fn,
        surface,
        pdev: vk::PhysicalDevice::null(),
        qfam: 0,
        mem: Default::default(),
        limits: Default::default(),
        dev: None,
        queue: vk::Queue::null(),
        swap_fn: None,
        swapchain: vk::SwapchainKHR::null(),
        sc_format: Default::default(),
        sc_extent: Default::default(),
        sc_views: Vec::new(),
        sc_fbs: Vec::new(),
        sc_done: Vec::new(),
        composite: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode: vk::PresentModeKHR::FIFO,
        image_count: 0,
        ring: None,
        readback: None,
        tex: None,
        target: None,
        target_fb: vk::Framebuffer::null(),
        target_pass: vk::RenderPass::null(),
        present_pass: vk::RenderPass::null(),
        dsl: [vk::DescriptorSetLayout::null(); 2],
        layout: vk::PipelineLayout::null(),
        pipelines: Vec::new(),
        modules: Vec::new(),
        sampler: vk::Sampler::null(),
        dpool: vk::DescriptorPool::null(),
        pools: Vec::new(),
        cmds: Vec::new(),
        fences: Vec::new(),
        acquired: Vec::new(),
        sets: [vk::DescriptorSet::null(); 3],
    });
    let p = slot.as_mut().unwrap();
    p.device()?;
    p.swapchain_first()?;
    p.ring()?;
    p.resources()?;
    p.frames()
}

impl Probe {
    fn dev(&self) -> &ash::Device {
        self.dev.as_ref().unwrap()
    }

    /// Every physical device into the report; the first with one queue family that
    /// draws and presents on the window (a discrete GPU before any other) is used.
    fn device(&mut self) -> R<()> {
        begin("pick a physical device");
        let pdevs = chk("vkEnumeratePhysicalDevices", unsafe { self.instance.enumerate_physical_devices() })?;
        section("physical devices");
        let mut pick: Option<(vk::PhysicalDevice, u32, bool)> = None;
        for (n, &pd) in pdevs.iter().enumerate() {
            let props = unsafe { self.instance.get_physical_device_properties(pd) };
            out!(
                "[{}] {} -- {:?}, Vulkan {}, driver version {} (raw {:#010x}), vendor {:#06x}, device {:#06x}",
                n,
                name_of(&props.device_name),
                props.device_type,
                version(props.api_version),
                version(props.driver_version),
                props.driver_version,
                props.vendor_id,
                props.device_id
            );
            let fams = unsafe { self.instance.get_physical_device_queue_family_properties(pd) };
            for (i, f) in fams.iter().enumerate() {
                let present = unsafe { self.surface_fn.get_physical_device_surface_support(pd, i as u32, self.surface) };
                out!("    queue family {}: {:?} x{}, presents on the window: {}", i, f.queue_flags, f.queue_count, match present {
                    Ok(b) => fmt(format_args!("{}", b)),
                    Err(e) => vkres(e),
                });
                if f.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present == Ok(true) {
                    let discrete = props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
                    if pick.is_none() || (discrete && !pick.unwrap().2) {
                        pick = Some((pd, i as u32, discrete));
                    }
                }
            }
        }
        let Some((pd, qfam, _)) = pick else {
            fail("pick a physical device", "no device has a queue family that draws and presents on the window");
            return Err(());
        };
        self.pdev = pd;
        self.qfam = qfam;
        let props = unsafe { self.instance.get_physical_device_properties(pd) };
        self.limits = props.limits;
        self.mem = unsafe { self.instance.get_physical_device_memory_properties(pd) };
        pass("pick a physical device", &fmt(format_args!("{}, queue family {}", name_of(&props.device_name), qfam)));

        section(&fmt(format_args!("device: {}", name_of(&props.device_name))));
        let dexts: Vec<String> = unsafe { self.instance.enumerate_device_extension_properties(pd) }.unwrap_or_default().iter().map(|e| name_of(&e.extension_name)).collect();
        let has = |n: &core::ffi::CStr| dexts.iter().any(|e| e.as_bytes() == n.to_bytes());
        // The two ways winevulkan can put a mapping below 4 GB for a 32-bit process in
        // new-style WoW64; whether the application is shown them depends on the Wine.
        out!(
            "of note: VK_KHR_portability_subset {}, VK_EXT_map_memory_placed {}, VK_EXT_external_memory_host {}",
            if has(ash::khr::portability_subset::NAME) { "yes" } else { "no" },
            if has(ash::ext::map_memory_placed::NAME) { "yes" } else { "no" },
            if has(ash::ext::external_memory_host::NAME) { "yes" } else { "no" }
        );
        let l = &self.limits;
        out!(
            "limits: minUniformBufferOffsetAlignment {}, maxUniformBufferRange {}, maxMemoryAllocationCount {}, nonCoherentAtomSize {}, minMemoryMapAlignment {}, optimalBufferCopyOffsetAlignment {}, maxImageDimension2D {}, maxBoundDescriptorSets {}",
            l.min_uniform_buffer_offset_alignment,
            l.max_uniform_buffer_range,
            l.max_memory_allocation_count,
            l.non_coherent_atom_size,
            l.min_memory_map_alignment,
            l.optimal_buffer_copy_offset_alignment,
            l.max_image_dimension2_d,
            l.max_bound_descriptor_sets
        );
        out!("memory heaps:");
        for i in 0..self.mem.memory_heap_count as usize {
            let h = self.mem.memory_heaps[i];
            out!("    heap {}: {} MB {:?}", i, h.size >> 20, h.flags);
        }
        out!("memory types:");
        for i in 0..self.mem.memory_type_count as usize {
            let t = self.mem.memory_types[i];
            out!("    type {}: heap {} {:?}", i, t.heap_index, t.property_flags);
        }
        if has(ash::khr::portability_subset::NAME) {
            // What a portability driver (MoltenVK) leaves out: triangle fans and point
            // polygons above all, both of which the engine's Direct3D 8 draws can ask for.
            let mut ps = vk::PhysicalDevicePortabilitySubsetFeaturesKHR::default();
            let mut pp = vk::PhysicalDevicePortabilitySubsetPropertiesKHR::default();
            unsafe {
                let mut f2 = vk::PhysicalDeviceFeatures2::default().push_next(&mut ps);
                self.instance.get_physical_device_features2(pd, &mut f2);
                let mut p2 = vk::PhysicalDeviceProperties2::default().push_next(&mut pp);
                self.instance.get_physical_device_properties2(pd, &mut p2);
            }
            let named = [
                ("constantAlphaColorBlendFactors", ps.constant_alpha_color_blend_factors),
                ("events", ps.events),
                ("imageViewFormatReinterpretation", ps.image_view_format_reinterpretation),
                ("imageViewFormatSwizzle", ps.image_view_format_swizzle),
                ("imageView2DOn3DImage", ps.image_view2_d_on3_d_image),
                ("multisampleArrayImage", ps.multisample_array_image),
                ("mutableComparisonSamplers", ps.mutable_comparison_samplers),
                ("pointPolygons", ps.point_polygons),
                ("samplerMipLodBias", ps.sampler_mip_lod_bias),
                ("separateStencilMaskRef", ps.separate_stencil_mask_ref),
                ("shaderSampleRateInterpolationFunctions", ps.shader_sample_rate_interpolation_functions),
                ("tessellationIsolines", ps.tessellation_isolines),
                ("tessellationPointMode", ps.tessellation_point_mode),
                ("triangleFans", ps.triangle_fans),
                ("vertexAttributeAccessBeyondStride", ps.vertex_attribute_access_beyond_stride),
            ];
            let missing: Vec<String> = named.iter().filter(|n| n.1 == vk::FALSE).map(|n| String::from(n.0)).collect();
            out!("portability subset: minVertexInputBindingStrideAlignment {}; not supported ({}):", pp.min_vertex_input_binding_stride_alignment, missing.len());
            list(&missing);
        }
        out!("device extensions ({}):", dexts.len());
        list(&dexts);

        begin("vkCreateDevice");
        let mut want = alloc::vec![ash::khr::swapchain::NAME.as_ptr()];
        if !has(ash::khr::swapchain::NAME) {
            fail("vkCreateDevice", "the device does not offer VK_KHR_swapchain");
            return Err(());
        }
        // A portability driver (MoltenVK) must be told the application knows it is one.
        let subset = has(ash::khr::portability_subset::NAME);
        if subset {
            want.push(ash::khr::portability_subset::NAME.as_ptr());
        }
        let prio = [1.0f32];
        let qci = [vk::DeviceQueueCreateInfo::default().queue_family_index(qfam).queue_priorities(&prio)];
        let dci = vk::DeviceCreateInfo::default().queue_create_infos(&qci).enabled_extension_names(&want);
        let dev = match unsafe { self.instance.create_device(pd, &dci, None) } {
            Ok(d) => d,
            Err(e) => {
                fail("vkCreateDevice", &vkres(e));
                return Err(());
            }
        };
        pass("vkCreateDevice", &fmt(format_args!("VK_SUCCESS, VK_KHR_swapchain{}", if subset { " + VK_KHR_portability_subset" } else { "" })));
        self.queue = unsafe { dev.get_device_queue(qfam, 0) };
        self.swap_fn = Some(ash::khr::swapchain::Device::new(&self.instance, &dev));
        self.dev = Some(dev);
        Ok(())
    }

    /// The surface's formats and modes into the report; B8G8R8A8_UNORM (what Display8
    /// renders in) if it is offered, FIFO (the game's vertical blank).
    fn swapchain_first(&mut self) -> R<()> {
        begin("vkCreateSwapchainKHR");
        let caps = quiet("vkGetPhysicalDeviceSurfaceCapabilitiesKHR", unsafe { self.surface_fn.get_physical_device_surface_capabilities(self.pdev, self.surface) })?;
        let formats = quiet("vkGetPhysicalDeviceSurfaceFormatsKHR", unsafe { self.surface_fn.get_physical_device_surface_formats(self.pdev, self.surface) })?;
        let modes = quiet("vkGetPhysicalDeviceSurfacePresentModesKHR", unsafe { self.surface_fn.get_physical_device_surface_present_modes(self.pdev, self.surface) })?;
        section("surface");
        out!(
            "images {}..{}, current extent {}x{}, extent {}x{}..{}x{}, usage {:?}, composite alpha {:?}, transforms {:?}",
            caps.min_image_count,
            caps.max_image_count,
            caps.current_extent.width,
            caps.current_extent.height,
            caps.min_image_extent.width,
            caps.min_image_extent.height,
            caps.max_image_extent.width,
            caps.max_image_extent.height,
            caps.supported_usage_flags,
            caps.supported_composite_alpha,
            caps.supported_transforms
        );
        let fl: Vec<String> = formats.iter().map(|f| fmt(format_args!("{:?}/{:?}", f.format, f.color_space))).collect();
        out!("formats ({}):", fl.len());
        list(&fl);
        let ml: Vec<String> = modes.iter().map(|m| fmt(format_args!("{:?}", m))).collect();
        out!("present modes: {}", ml.join(", "));
        let Some(first) = formats.first() else {
            fail("vkCreateSwapchainKHR", "the surface offers no format");
            return Err(());
        };
        self.sc_format = *formats
            .iter()
            .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
            .or_else(|| formats.iter().find(|f| f.format == vk::Format::R8G8B8A8_UNORM))
            .unwrap_or(first);
        self.composite = [vk::CompositeAlphaFlagsKHR::OPAQUE, vk::CompositeAlphaFlagsKHR::INHERIT, vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED, vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED]
            .into_iter()
            .find(|&c| caps.supported_composite_alpha.contains(c))
            .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE);
        self.present_mode = vk::PresentModeKHR::FIFO;
        self.image_count = (caps.min_image_count + 1).min(if caps.max_image_count == 0 { u32::MAX } else { caps.max_image_count });
        self.present_pass = self.render_pass(self.sc_format.format, vk::ImageLayout::PRESENT_SRC_KHR, false)?;
        self.make_swapchain()?;
        pass(
            "vkCreateSwapchainKHR",
            &fmt(format_args!(
                "VK_SUCCESS, {}x{} {:?}/{:?}, {} images, FIFO, composite {:?}",
                self.sc_extent.width,
                self.sc_extent.height,
                self.sc_format.format,
                self.sc_format.color_space,
                self.sc_views.len(),
                self.composite
            )),
        );
        Ok(())
    }

    /// (Re)makes the swap chain at the surface's extent, with its views, framebuffers
    /// and one semaphore per image for the present to wait on.
    fn make_swapchain(&mut self) -> R<()> {
        let caps = quiet("vkGetPhysicalDeviceSurfaceCapabilitiesKHR", unsafe { self.surface_fn.get_physical_device_surface_capabilities(self.pdev, self.surface) })?;
        self.sc_extent = if caps.current_extent.width == u32::MAX {
            vk::Extent2D {
                width: W.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
                height: H.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
            }
        } else {
            caps.current_extent
        };
        if self.sc_extent.width == 0 || self.sc_extent.height == 0 {
            fail("vkCreateSwapchainKHR", "the window's surface has no area (minimised?)");
            return Err(());
        }
        let old = self.swapchain;
        let ci = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(self.image_count.max(caps.min_image_count))
            .image_format(self.sc_format.format)
            .image_color_space(self.sc_format.color_space)
            .image_extent(self.sc_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(self.composite)
            .present_mode(self.present_mode)
            .clipped(true)
            .old_swapchain(old);
        let swap_fn = self.swap_fn.clone().unwrap();
        let sc = quiet("vkCreateSwapchainKHR", unsafe { swap_fn.create_swapchain(&ci, None) })?;
        self.drop_swapchain_parts();
        if old != vk::SwapchainKHR::null() {
            unsafe { swap_fn.destroy_swapchain(old, None) };
        }
        self.swapchain = sc;
        let images = quiet("vkGetSwapchainImagesKHR", unsafe { swap_fn.get_swapchain_images(sc) })?;
        let dev = self.dev.as_ref().unwrap();
        for &im in &images {
            let view = quiet("vkCreateImageView (swap chain)", unsafe { dev.create_image_view(&view_info(im, self.sc_format.format), None) })?;
            self.sc_views.push(view);
            let att = [view];
            let fci = vk::FramebufferCreateInfo::default().render_pass(self.present_pass).attachments(&att).width(self.sc_extent.width).height(self.sc_extent.height).layers(1);
            self.sc_fbs.push(quiet("vkCreateFramebuffer (swap chain)", unsafe { dev.create_framebuffer(&fci, None) })?);
            self.sc_done.push(quiet("vkCreateSemaphore", unsafe { dev.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) })?);
        }
        Ok(())
    }

    fn drop_swapchain_parts(&mut self) {
        let dev = self.dev.as_ref().unwrap();
        unsafe {
            for f in self.sc_fbs.drain(..) {
                dev.destroy_framebuffer(f, None);
            }
            for v in self.sc_views.drain(..) {
                dev.destroy_image_view(v, None);
            }
            for s in self.sc_done.drain(..) {
                dev.destroy_semaphore(s, None);
            }
        }
    }

    fn render_pass(&self, format: vk::Format, final_layout: vk::ImageLayout, offscreen: bool) -> R<vk::RenderPass> {
        let att = [vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(final_layout)];
        let refs = [vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }];
        let sub = [vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&refs)];
        // In: the swap chain image's acquire (waited for at the colour output), or for
        // the offscreen target the previous frame's present pass still sampling it. Out:
        // the target to the present pass's sampling and the last frame's copy.
        let deps_in = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | if offscreen { vk::PipelineStageFlags::FRAGMENT_SHADER } else { vk::PipelineStageFlags::empty() },
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        };
        let deps_out = vk::SubpassDependency {
            src_subpass: 0,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::TRANSFER,
            src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::TRANSFER_READ,
            dependency_flags: vk::DependencyFlags::empty(),
        };
        let deps_on = [deps_in, deps_out];
        let deps = if offscreen { &deps_on[..] } else { &deps_on[..1] };
        let ci = vk::RenderPassCreateInfo::default().attachments(&att).subpasses(&sub).dependencies(deps);
        quiet("vkCreateRenderPass", unsafe { self.dev().create_render_pass(&ci, None) })
    }

    fn buffer(&self, what: &str, size: u64, usage: vk::BufferUsageFlags, prefer: vk::MemoryPropertyFlags) -> R<(Buf, u32)> {
        let dev = self.dev();
        let buf = quiet(what, unsafe { dev.create_buffer(&vk::BufferCreateInfo::default().size(size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE), None) })?;
        let req = unsafe { dev.get_buffer_memory_requirements(buf) };
        let need = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let Some(ty) = find_memory(&self.mem, req.memory_type_bits, need | prefer).or_else(|| find_memory(&self.mem, req.memory_type_bits, need)) else {
            fail(what, "no HOST_VISIBLE | HOST_COHERENT memory type for the buffer");
            return Err(());
        };
        let mem = quiet(what, unsafe { dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) })?;
        quiet(what, unsafe { dev.bind_buffer_memory(buf, mem, 0) })?;
        Ok((Buf { buf, mem, ptr: core::ptr::null_mut() }, ty))
    }

    /// The crux: the ring, mapped into this 32-bit process, checked to be a real,
    /// writable mapping below 4 GB, and written and read back whole.
    fn ring(&mut self) -> R<()> {
        begin("allocate the ring (8 MB HOST_VISIBLE | HOST_COHERENT)");
        let usage = vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC;
        let (mut ring, ty) = self.buffer("allocate the ring (8 MB HOST_VISIBLE | HOST_COHERENT)", RING, usage, vk::MemoryPropertyFlags::empty())?;
        pass(
            "allocate the ring (8 MB HOST_VISIBLE | HOST_COHERENT)",
            &fmt(format_args!("VK_SUCCESS, memory type {} {:?}", ty, self.mem.memory_types[ty as usize].property_flags)),
        );
        begin("vkMapMemory (the ring)");
        let ptr = match unsafe { self.dev().map_memory(ring.mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()) } {
            Ok(p) => p as *mut u8,
            Err(e) => {
                fail("vkMapMemory (the ring)", &vkres(e));
                self.ring = Some(ring);
                return Err(());
            }
        };
        ring.ptr = ptr;
        self.ring = Some(ring);
        let at = ptr as usize;
        section("the mapping");
        out!("vkMapMemory: {:#010x}..{:#010x} ({} MB)", at, at as u64 + RING, RING >> 20);
        let mut mi = MemInfo::default();
        let q = unsafe { VirtualQuery(ptr, &mut mi, core::mem::size_of::<MemInfo>()) };
        if q != 0 {
            out!(
                "VirtualQuery: region {:#010x} + {:#x}, allocation {:#010x}, state {:#x} ({}), protect {:#x}, type {:#x} (0x20000 private, 0x40000 mapped)",
                mi.base,
                mi.size,
                mi.alloc_base,
                mi.state,
                match mi.state {
                    0x1000 => "committed",
                    0x2000 => "reserved",
                    0x10000 => "free",
                    _ => "?",
                },
                mi.protect,
                mi.mtype
            );
            // What Wine's two WoW64 modes were seen to give (README.md, "Result here"):
            // new-style places the mapping itself, through the 32-bit side's own memory
            // manager; old-style lets a 32-bit host driver mmap it where it likes.
            if mi.state == 0x1000 && mi.protect & 0xcc != 0 {
                out!("    a committed, writable region Windows knows: as winevulkan places a mapping in new-style WoW64 (or a Windows driver maps one)");
            } else {
                out!("    not a committed region to Windows: mapped behind its back, as a 32-bit host driver does in old-style WoW64; never judge such a pointer by VirtualQuery");
            }
        } else {
            out!("VirtualQuery: failed (the address is not one this process knows)");
        }
        if at >= 0x8000_0000 {
            out!("    above 2 GB in a process that is not large-address-aware: a pointer here is negative as a signed 32-bit number");
        }
        if ptr.is_null() || (at as u64) + RING > 0x1_0000_0000 {
            fail("vkMapMemory (the ring)", &fmt(format_args!("VK_SUCCESS but the pointer {:#010x} does not hold 8 MB below 4 GB", at)));
            return Err(());
        }
        pass("vkMapMemory (the ring)", &fmt(format_args!("VK_SUCCESS at {:#010x}", at)));
        // Writable and coherent with itself: every word written, then every word read.
        begin("write and read back all 8 MB of the mapping");
        let words = (RING / 4) as usize;
        let t0 = now();
        let w = ptr as *mut u32;
        for i in 0..words {
            unsafe { w.add(i).write_volatile(i as u32 ^ 0xA5A5_0000) };
        }
        let t1 = now();
        let mut bad = 0usize;
        for i in 0..words {
            if unsafe { w.add(i).read_volatile() } != i as u32 ^ 0xA5A5_0000 {
                bad += 1;
            }
        }
        let t2 = now();
        out!(
            "8 MB written in {:.2} ms ({:.0} MB/s), read in {:.2} ms ({:.0} MB/s)",
            (t1 - t0) * 1e3,
            8.0 / (t1 - t0),
            (t2 - t1) * 1e3,
            8.0 / (t2 - t1)
        );
        if bad != 0 {
            fail("write and read back all 8 MB of the mapping", &fmt(format_args!("{} of {} words read back wrong", bad, words)));
            return Err(());
        }
        pass("write and read back all 8 MB of the mapping", &fmt(format_args!("{:.0} MB/s written", 8.0 / (t1 - t0))));

        begin("allocate and map the readback buffer");
        let (mut rb, ty) = self.buffer("allocate and map the readback buffer", (W * H * 4) as u64, vk::BufferUsageFlags::TRANSFER_DST, vk::MemoryPropertyFlags::HOST_CACHED)?;
        let rp = match unsafe { self.dev().map_memory(rb.mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()) } {
            Ok(p) => p as *mut u8,
            Err(e) => {
                fail("allocate and map the readback buffer", &vkres(e));
                self.readback = Some(rb);
                return Err(());
            }
        };
        rb.ptr = rp;
        self.readback = Some(rb);
        out!("readback buffer: memory type {} {:?}, mapped at {:#010x}", ty, self.mem.memory_types[ty as usize].property_flags, rp as usize);
        pass("allocate and map the readback buffer", &fmt(format_args!("VK_SUCCESS at {:#010x}", rp as usize)));
        Ok(())
    }

    fn image(&self, what: &str, w: u32, h: u32, format: vk::Format, usage: vk::ImageUsageFlags) -> R<Img> {
        let dev = self.dev();
        let ici = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width: w, height: h, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image = quiet(what, unsafe { dev.create_image(&ici, None) })?;
        let req = unsafe { dev.get_image_memory_requirements(image) };
        let Some(ty) = find_memory(&self.mem, req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL).or_else(|| find_memory(&self.mem, req.memory_type_bits, vk::MemoryPropertyFlags::empty())) else {
            fail(what, "no memory type for the image");
            return Err(());
        };
        let mem = quiet(what, unsafe { dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) })?;
        quiet(what, unsafe { dev.bind_image_memory(image, mem, 0) })?;
        let view = quiet(what, unsafe { dev.create_image_view(&view_info(image, format), None) })?;
        Ok(Img { image, view, mem })
    }

    /// The texture, the offscreen target, the pipelines, descriptors and per-slot
    /// command buffers, fences and semaphores.
    fn resources(&mut self) -> R<()> {
        begin("images, pipelines and descriptors");
        const STEP: &str = "images, pipelines and descriptors";
        self.tex = Some(self.image(STEP, TEX, TEX, vk::Format::R8G8B8A8_UNORM, vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)?);
        self.target = Some(self.image(
            STEP,
            W,
            H,
            vk::Format::B8G8R8A8_UNORM,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        )?);
        self.target_pass = self.render_pass(vk::Format::B8G8R8A8_UNORM, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, true)?;
        let att = [self.target.as_ref().unwrap().view];
        let fci = vk::FramebufferCreateInfo::default().render_pass(self.target_pass).attachments(&att).width(W).height(H).layers(1);
        self.target_fb = quiet(STEP, unsafe { self.dev().create_framebuffer(&fci, None) })?;

        let dev = self.dev.as_ref().unwrap();
        let b0 = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];
        let b1 = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        self.dsl[0] = quiet(STEP, unsafe { dev.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b0), None) })?;
        self.dsl[1] = quiet(STEP, unsafe { dev.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b1), None) })?;
        self.layout = quiet(STEP, unsafe { dev.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().set_layouts(&self.dsl), None) })?;
        self.sampler = quiet(STEP, unsafe {
            dev.create_sampler(
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
        })?;
        for code in [&VS_TRI_SPV[..], &PS_TRI_SPV[..], &VS_QUAD_SPV[..], &PS_TEX_SPV[..]] {
            let m = quiet(STEP, unsafe { dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code), None) })?;
            self.modules.push(m);
        }
        let tri_attrs = [
            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 },
            vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32B32A32_SFLOAT, offset: 8 },
        ];
        let quad_attrs = [
            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 },
            vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 8 },
        ];
        let (vt, pt, vq, pq) = (self.modules[0], self.modules[1], self.modules[2], self.modules[3]);
        let tri = self.pipeline((vt, c"vs_tri"), (pt, c"ps_tri"), 24, &tri_attrs, self.target_pass)?;
        let tex = self.pipeline((vq, c"vs_quad"), (pq, c"ps_tex"), 16, &quad_attrs, self.target_pass)?;
        let blit = self.pipeline((vq, c"vs_quad"), (pq, c"ps_tex"), 16, &quad_attrs, self.present_pass)?;
        self.pipelines = alloc::vec![tri, tex, blit];

        let dev = self.dev.as_ref().unwrap();
        let sizes = [
            vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, descriptor_count: 1 },
            vk::DescriptorPoolSize { ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, descriptor_count: 2 },
        ];
        self.dpool = quiet(STEP, unsafe { dev.create_descriptor_pool(&vk::DescriptorPoolCreateInfo::default().max_sets(3).pool_sizes(&sizes), None) })?;
        let ring = self.ring.as_ref().unwrap();
        let sets = quiet(STEP, unsafe {
            dev.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(self.dpool).set_layouts(&[self.dsl[0], self.dsl[1], self.dsl[1]]))
        })?;
        let ubo = [vk::DescriptorBufferInfo { buffer: ring.buf, offset: 0, range: UBO_SIZE }];
        let texi = [vk::DescriptorImageInfo { sampler: self.sampler, image_view: self.tex.as_ref().unwrap().view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
        let tgti = [vk::DescriptorImageInfo { sampler: self.sampler, image_view: self.target.as_ref().unwrap().view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
        let writes = [
            vk::WriteDescriptorSet::default().dst_set(sets[0]).dst_binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).buffer_info(&ubo),
            vk::WriteDescriptorSet::default().dst_set(sets[1]).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&texi),
            vk::WriteDescriptorSet::default().dst_set(sets[2]).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&tgti),
        ];
        unsafe { dev.update_descriptor_sets(&writes, &[]) };
        self.sets = [sets[0], sets[1], sets[2]];

        for _ in 0..SLOTS {
            let pool = quiet(STEP, unsafe {
                dev.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(self.qfam).flags(vk::CommandPoolCreateFlags::TRANSIENT), None)
            })?;
            self.pools.push(pool);
            let cmd = quiet(STEP, unsafe {
                dev.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default().command_pool(pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1))
            })?;
            self.cmds.push(cmd[0]);
            self.fences.push(quiet(STEP, unsafe { dev.create_fence(&vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED), None) })?);
            self.acquired.push(quiet(STEP, unsafe { dev.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) })?);
        }
        pass(STEP, "a 64x64 texture, a 640x480 B8G8R8A8 target, 3 pipelines, a dynamic uniform buffer descriptor");
        Ok(())
    }

    fn pipeline(&self, vs: (vk::ShaderModule, &core::ffi::CStr), ps: (vk::ShaderModule, &core::ffi::CStr), stride: u32, attrs: &[vk::VertexInputAttributeDescription], pass: vk::RenderPass) -> R<vk::Pipeline> {
        let stages = [
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vs.0).name(vs.1),
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(ps.0).name(ps.1),
        ];
        let vb = [vk::VertexInputBindingDescription { binding: 0, stride, input_rate: vk::VertexInputRate::VERTEX }];
        let vi = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&vb).vertex_attribute_descriptions(attrs);
        let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
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
            .render_pass(pass)
            .subpass(0)];
        match unsafe { self.dev().create_graphics_pipelines(vk::PipelineCache::null(), &gpci, None) } {
            Ok(p) => Ok(p[0]),
            Err((_, e)) => {
                fail("images, pipelines and descriptors", &fmt(format_args!("vkCreateGraphicsPipelines: {}", vkres(e))));
                Err(())
            }
        }
    }

    /// 300 frames, two in flight, everything they draw written into the ring.
    fn frames(&mut self) -> R<()> {
        const STEP: &str = "draw 300 frames from the mapped ring and present them";
        begin(STEP);
        let ubo_align = self.limits.min_uniform_buffer_offset_alignment.max(16);
        let base = self.ring.as_ref().unwrap().ptr;
        let mut ring = Ring { head: 0, wraps: 0, used: [Vec::new(), Vec::new()], clashes: 0 };
        let mut times: Vec<f64> = Vec::new();
        let mut last = now();
        let (mut recreated, mut suboptimal, mut out_of_date) = (0u32, 0u32, 0u32);
        let mut stale = false;
        let mut f = 0;
        while f < FRAMES {
            if !user32::pump() {
                fail(STEP, &fmt(format_args!("the window was closed at frame {}", f)));
                return Err(());
            }
            let s = f as usize % SLOTS;
            let fence = self.fences[s];
            match unsafe { self.dev().wait_for_fences(&[fence], true, TIMEOUT) } {
                Ok(()) => {}
                Err(e) => {
                    fail(STEP, &fmt(format_args!("frame {}: vkWaitForFences: {}{}", f, vkres(e), if e == vk::Result::TIMEOUT { " (the GPU did not finish a frame in 5 s)" } else { "" })));
                    return Err(());
                }
            }
            if stale {
                recreated += 1;
                if recreated > 50 {
                    fail(STEP, "the swap chain went out of date more than 50 times");
                    return Err(());
                }
                quiet(STEP, unsafe { self.dev().device_wait_idle() })?;
                self.make_swapchain()?;
                stale = false;
            }
            let swap_fn = self.swap_fn.as_ref().unwrap();
            let image = match unsafe { swap_fn.acquire_next_image(self.swapchain, TIMEOUT, self.acquired[s], vk::Fence::null()) } {
                Ok((i, sub)) => {
                    if sub {
                        suboptimal += 1;
                        stale = true;
                    }
                    i
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    out_of_date += 1;
                    // Drawn again on a new swap chain; the fence was not reset, so the
                    // next round's wait passes at once.
                    stale = true;
                    continue;
                }
                Err(e) => {
                    fail(STEP, &fmt(format_args!("frame {}: vkAcquireNextImageKHR: {}", f, vkres(e))));
                    return Err(());
                }
            } as usize;
            quiet(STEP, unsafe { self.dev().reset_fences(&[fence]) })?;
            ring.used[s].clear();

            // The frame's writes into the mapping, each where a ring would put it.
            let filler = ring.alloc(s, FILLER, 16);
            unsafe {
                let w = base.add(filler as usize) as *mut u32;
                for i in 0..(FILLER / 4) as usize {
                    w.add(i).write((f << 20) ^ i as u32);
                }
            }
            let phase = f % 2;
            let texoff = ring.alloc(s, (TEX * TEX * 4) as u64, 16);
            unsafe {
                let t = base.add(texoff as usize);
                for y in 0..TEX {
                    for x in 0..TEX {
                        let c = if ((x / 8) + (y / 8) + phase) % 2 == 0 { CHECK_A } else { CHECK_B };
                        core::ptr::copy_nonoverlapping(c.as_ptr(), t.add(((y * TEX + x) * 4) as usize), 4);
                    }
                }
            }
            let offx = ((f % 20) as f32 - 10.0) * 0.005;
            let tint = TINTS[(f % 3) as usize];
            let uboff = ring.alloc(s, UBO_SIZE, ubo_align);
            put(base, uboff, &[offx, 0.0, 0.0, 0.0, tint[0], tint[1], tint[2], 1.0]);
            let c = TRI_COLOR;
            let trioff = ring.alloc(s, 3 * 24, 16);
            put(base, trioff, &[0.1, 0.1, c[0], c[1], c[2], c[3], 0.9, 0.1, c[0], c[1], c[2], c[3], 0.5, 0.9, c[0], c[1], c[2], c[3]]);
            let quadoff = ring.alloc(s, 4 * 16, 16);
            put(base, quadoff, &[-0.9f32, -0.1, 0.0, 0.0, -0.1, -0.1, 1.0, 0.0, -0.9, -0.9, 0.0, 1.0, -0.1, -0.9, 1.0, 1.0]);
            let blitoff = ring.alloc(s, 4 * 16, 16);
            put(base, blitoff, &[-1.0f32, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0]);
            let idxoff = ring.alloc(s, 12, 4);
            put(base, idxoff, &[0u16, 1, 2, 2, 1, 3]);

            self.record(s, image, f, f == FRAMES - 1, Offsets { tex: texoff, ubo: uboff, tri: trioff, quad: quadoff, blit: blitoff, idx: idxoff })?;

            let wait = [self.acquired[s]];
            let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let cmds = [self.cmds[s]];
            let sig = [self.sc_done[image]];
            let submit = [vk::SubmitInfo::default().wait_semaphores(&wait).wait_dst_stage_mask(&stages).command_buffers(&cmds).signal_semaphores(&sig)];
            if let Err(e) = unsafe { self.dev().queue_submit(self.queue, &submit, fence) } {
                fail(STEP, &fmt(format_args!("frame {}: vkQueueSubmit: {}", f, vkres(e))));
                return Err(());
            }
            let scs = [self.swapchain];
            let idx = [image as u32];
            let pi = vk::PresentInfoKHR::default().wait_semaphores(&sig).swapchains(&scs).image_indices(&idx);
            match unsafe { self.swap_fn.as_ref().unwrap().queue_present(self.queue, &pi) } {
                Ok(sub) => {
                    if sub {
                        suboptimal += 1;
                        stale = true;
                    }
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    out_of_date += 1;
                    stale = true;
                }
                Err(e) => {
                    fail(STEP, &fmt(format_args!("frame {}: vkQueuePresentKHR: {}", f, vkres(e))));
                    return Err(());
                }
            }
            let t = now();
            if f > 0 {
                times.push(t - last);
            }
            last = t;
            f += 1;
        }
        quiet(STEP, unsafe { self.dev().wait_for_fences(&self.fences, true, TIMEOUT) })?;
        let n = times.len().max(1) as f64;
        let mean = times.iter().sum::<f64>() / n;
        let max = times.iter().cloned().fold(0.0, f64::max);
        let min = times.iter().cloned().fold(f64::MAX, f64::min);
        section("frames");
        out!(
            "{} frames, mean frame time {:.2} ms ({:.1} fps), min {:.2} ms, max {:.2} ms (FIFO: on a real display its refresh rate bounds this)",
            FRAMES,
            mean * 1e3,
            1.0 / mean,
            min * 1e3,
            max * 1e3
        );
        out!("ring: wrapped {} times; {} overlaps with the frame in flight (must be 0)", ring.wraps, ring.clashes);
        out!(
            "swap chain: re-made {} times ({} results SUBOPTIMAL, {} OUT_OF_DATE: the window manager resizing the window), last extent {}x{}",
            recreated,
            suboptimal,
            out_of_date,
            self.sc_extent.width,
            self.sc_extent.height
        );
        if ring.clashes != 0 {
            fail(STEP, "the ring handed out memory the other frame in flight still used");
            return Err(());
        }
        pass(STEP, &fmt(format_args!("mean frame time {:.2} ms, ring wrapped {} times", mean * 1e3, ring.wraps)));
        self.verify()
    }

    fn record(&self, s: usize, image: usize, f: u32, last: bool, o: Offsets) -> R<()> {
        const STEP: &str = "draw 300 frames from the mapped ring and present them";
        let dev = self.dev();
        let cmd = self.cmds[s];
        let ring = self.ring.as_ref().unwrap().buf;
        let tex = self.tex.as_ref().unwrap().image;
        let target = self.target.as_ref().unwrap().image;
        let color = vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 };
        unsafe {
            quiet(STEP, dev.reset_command_pool(self.pools[s], vk::CommandPoolResetFlags::empty()))?;
            quiet(STEP, dev.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)))?;
            // The texture upload, from the ring, after the previous frame's sampling.
            let (old, src) = if f == 0 {
                (vk::ImageLayout::UNDEFINED, vk::PipelineStageFlags::TOP_OF_PIPE)
            } else {
                (vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::PipelineStageFlags::FRAGMENT_SHADER)
            };
            let to_dst = [vk::ImageMemoryBarrier::default()
                .old_layout(old)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(tex)
                .subresource_range(color)];
            dev.cmd_pipeline_barrier(cmd, src, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &to_dst);
            let region = [vk::BufferImageCopy {
                buffer_offset: o.tex,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 },
                image_offset: vk::Offset3D::default(),
                image_extent: vk::Extent3D { width: TEX, height: TEX, depth: 1 },
            }];
            dev.cmd_copy_buffer_to_image(cmd, ring, tex, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &region);
            let to_read = [vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(tex)
                .subresource_range(color)];
            dev.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &to_read);

            // The offscreen target: the checker, then the triangle over it.
            let clear = [vk::ClearValue { color: vk::ClearColorValue { float32: CLEAR } }];
            let full = vk::Rect2D { offset: vk::Offset2D::default(), extent: vk::Extent2D { width: W, height: H } };
            dev.cmd_begin_render_pass(cmd, &vk::RenderPassBeginInfo::default().render_pass(self.target_pass).framebuffer(self.target_fb).render_area(full).clear_values(&clear), vk::SubpassContents::INLINE);
            dev.cmd_set_viewport(cmd, 0, &[vk::Viewport { x: 0.0, y: 0.0, width: W as f32, height: H as f32, min_depth: 0.0, max_depth: 1.0 }]);
            dev.cmd_set_scissor(cmd, 0, &[full]);
            dev.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::GRAPHICS, self.layout, 0, &[self.sets[0], self.sets[1]], &[o.ubo as u32]);
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipelines[1]);
            dev.cmd_bind_vertex_buffers(cmd, 0, &[ring], &[o.quad]);
            dev.cmd_bind_index_buffer(cmd, ring, o.idx, vk::IndexType::UINT16);
            dev.cmd_draw_indexed(cmd, 6, 1, 0, 0, 0);
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipelines[0]);
            dev.cmd_bind_vertex_buffers(cmd, 0, &[ring], &[o.tri]);
            dev.cmd_draw(cmd, 3, 1, 0, 0);
            dev.cmd_end_render_pass(cmd);

            // The target into the swap chain, as Display8's present pass.
            let sc = vk::Rect2D { offset: vk::Offset2D::default(), extent: self.sc_extent };
            let black = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
            dev.cmd_begin_render_pass(cmd, &vk::RenderPassBeginInfo::default().render_pass(self.present_pass).framebuffer(self.sc_fbs[image]).render_area(sc).clear_values(&black), vk::SubpassContents::INLINE);
            dev.cmd_set_viewport(cmd, 0, &[vk::Viewport { x: 0.0, y: 0.0, width: sc.extent.width as f32, height: sc.extent.height as f32, min_depth: 0.0, max_depth: 1.0 }]);
            dev.cmd_set_scissor(cmd, 0, &[sc]);
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipelines[2]);
            dev.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::GRAPHICS, self.layout, 0, &[self.sets[0], self.sets[2]], &[o.ubo as u32]);
            dev.cmd_bind_vertex_buffers(cmd, 0, &[ring], &[o.blit]);
            dev.cmd_bind_index_buffer(cmd, ring, o.idx, vk::IndexType::UINT16);
            dev.cmd_draw_indexed(cmd, 6, 1, 0, 0, 0);
            dev.cmd_end_render_pass(cmd);

            if last {
                // The last frame's target into the readback buffer, for the host.
                let to_src = [vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(target)
                    .subresource_range(color)];
                dev.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &to_src,
                );
                let region = [vk::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 },
                    image_offset: vk::Offset3D::default(),
                    image_extent: vk::Extent3D { width: W, height: H, depth: 1 },
                }];
                dev.cmd_copy_image_to_buffer(cmd, target, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, self.readback.as_ref().unwrap().buf, &region);
                let to_host = [vk::BufferMemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::HOST_READ)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(self.readback.as_ref().unwrap().buf)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)];
                dev.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::HOST, vk::DependencyFlags::empty(), &[], &to_host, &[]);
            }
            quiet(STEP, dev.end_command_buffer(cmd))?;
        }
        Ok(())
    }

    /// The last frame's pixels, against what the shaders must have made of it: the
    /// clear, both checker colours in the order of the last upload, and the triangle in
    /// the last frame's tint (B8G8R8A8, top row first).
    fn verify(&self) -> R<()> {
        const STEP: &str = "read the last frame back and check its pixels";
        begin(STEP);
        let rb = self.readback.as_ref().unwrap().ptr;
        let last = FRAMES - 1;
        let px = |c: [f32; 3]| -> [u8; 3] { [(c[0] * 255.0 + 0.5) as u8, (c[1] * 255.0 + 0.5) as u8, (c[2] * 255.0 + 0.5) as u8] };
        let tint = TINTS[(last % 3) as usize];
        let tri = px([TRI_COLOR[0] * tint[0], TRI_COLOR[1] * tint[1], TRI_COLOR[2] * tint[2]]);
        let clear = px([CLEAR[0], CLEAR[1], CLEAR[2]]);
        let (first, second) = if last % 2 == 0 { (CHECK_A, CHECK_B) } else { (CHECK_B, CHECK_A) };
        let a = [first[0], first[1], first[2]];
        let b = [second[0], second[1], second[2]];
        // The checker quad covers x 32..288, y 264..456: cells of 32x24 pixels.
        let checks: [(&str, u32, u32, [u8; 3]); 7] = [
            ("clear, top left", 10, 10, clear),
            ("clear, bottom right", 600, 440, clear),
            ("checker cell (0,0)", 48, 276, a),
            ("checker cell (1,0)", 80, 276, b),
            ("checker cell (0,1)", 48, 300, b),
            ("checker cell (7,7)", 272, 444, a),
            ("triangle, last frame's tint", 480, 152, tri),
        ];
        section("readback (RGB of the last frame's target)");
        let mut bad = 0;
        for (what, x, y, want) in checks {
            let i = ((y * W + x) * 4) as usize;
            let got = unsafe { [*rb.add(i + 2), *rb.add(i + 1), *rb.add(i)] };
            let ok = (0..3).all(|k| (got[k] as i32 - want[k] as i32).abs() <= 2);
            if !ok {
                bad += 1;
            }
            out!("{} {:<30} ({:>3},{:>3}): got {:>3} {:>3} {:>3}, want {:>3} {:>3} {:>3}", if ok { "ok  " } else { "BAD " }, what, x, y, got[0], got[1], got[2], want[0], want[1], want[2]);
        }
        if bad != 0 {
            fail(STEP, &fmt(format_args!("{} of {} pixels wrong", bad, checks.len())));
            return Err(());
        }
        pass(STEP, &fmt(format_args!("all {} pixels as the shaders make them", checks.len())));
        Ok(())
    }

    fn destroy(&mut self) {
        unsafe {
            if let Some(dev) = self.dev.as_ref() {
                let _ = dev.device_wait_idle();
                for &p in &self.pipelines {
                    dev.destroy_pipeline(p, None);
                }
                for &m in &self.modules {
                    dev.destroy_shader_module(m, None);
                }
                dev.destroy_pipeline_layout(self.layout, None);
                dev.destroy_descriptor_pool(self.dpool, None);
                for &l in &self.dsl {
                    dev.destroy_descriptor_set_layout(l, None);
                }
                dev.destroy_sampler(self.sampler, None);
                for i in [self.tex.take(), self.target.take()].into_iter().flatten() {
                    dev.destroy_image_view(i.view, None);
                    dev.destroy_image(i.image, None);
                    dev.free_memory(i.mem, None);
                }
                for b in [self.ring.take(), self.readback.take()].into_iter().flatten() {
                    if !b.ptr.is_null() {
                        dev.unmap_memory(b.mem);
                    }
                    dev.destroy_buffer(b.buf, None);
                    dev.free_memory(b.mem, None);
                }
                dev.destroy_framebuffer(self.target_fb, None);
                for &f in &self.fences {
                    dev.destroy_fence(f, None);
                }
                for &s in &self.acquired {
                    dev.destroy_semaphore(s, None);
                }
                for &p in &self.pools {
                    dev.destroy_command_pool(p, None);
                }
                self.drop_swapchain_parts();
                let dev = self.dev.as_ref().unwrap();
                dev.destroy_render_pass(self.target_pass, None);
                dev.destroy_render_pass(self.present_pass, None);
                if self.swapchain != vk::SwapchainKHR::null() {
                    self.swap_fn.as_ref().unwrap().destroy_swapchain(self.swapchain, None);
                }
                dev.destroy_device(None);
            }
            self.surface_fn.destroy_surface(self.surface, None);
            if let Some((du, m)) = self.debug.take() {
                du.destroy_debug_utils_messenger(m, None);
            }
            self.instance.destroy_instance(None);
        }
        let _ = &self.entry;
    }
}

struct Offsets {
    tex: u64,
    ubo: u64,
    tri: u64,
    quad: u64,
    blit: u64,
    idx: u64,
}

/// The ring over the mapped buffer: allocations run on from the last, wrap to the start
/// when the end is reached, and are checked against what the other frame in flight was
/// given (it may still be reading it).
struct Ring {
    head: u64,
    wraps: u32,
    used: [Vec<(u64, u64)>; SLOTS],
    clashes: u32,
}

impl Ring {
    fn alloc(&mut self, slot: usize, n: u64, a: u64) -> u64 {
        let mut off = align(self.head, a);
        if off + n > RING {
            off = 0;
            self.wraps += 1;
        }
        self.head = off + n;
        for (s, e) in &self.used[1 - slot] {
            if off < *e && *s < off + n {
                self.clashes += 1;
            }
        }
        self.used[slot].push((off, off + n));
        off
    }
}

fn put<T: Copy>(base: *mut u8, off: u64, data: &[T]) {
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, base.add(off as usize), core::mem::size_of_val(data)) };
}

fn view_info(image: vk::Image, format: vk::Format) -> vk::ImageViewCreateInfo<'static> {
    vk::ImageViewCreateInfo::default().image(image).view_type(vk::ImageViewType::TYPE_2D).format(format).subresource_range(vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    })
}
