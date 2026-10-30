//! Passthrough on Vulkan (README.md, "Passthrough"): each of the engine's draws that
//! `passthrough.rs` lets through, recorded into the frame's command buffer when it is
//! made, through `shaders/ff.hlsl`.
//!
//! - **Where it lands**: the pass a plugin published at its last `Cmd`'s end in this
//!   frame (`display8v1_vk_pass`: render3d's scene pass while its body runs, so what it
//!   hands back is drawn into its scene, tone-mapped and bloomed with the rest), else
//!   the engine's targets (Display8's colour and depth). Nothing is recorded while a
//!   plugin's `Cmd` is alive (a draw then is logged once and not drawn).
//! - **One render pass for a run of draws**: opened at a draw, left open, and closed
//!   at the next call into Display8 that records or hands the frame over (`close`), so
//!   no recorder ever finds it open. Draws are never merged or reordered.
//! - **Pipelines** by what Vulkan bakes into one: the target's formats and samples,
//!   topology, vertex layout, blend, depth, cull. What varies per draw goes in two
//!   constant blocks (`vs_block`, `ps_block`), in the upload ring, bound as dynamic
//!   uniform buffers of one set per ring chunk; a block equal to the last is not sent
//!   again.
//! - **Textures** are the stub's mirrors (`mirror::passthrough`), made at a texture's
//!   first draw with every level; samplers by the stage's state, made as DXVK makes them
//!   under the game's `dxvk.conf` (anisotropy, LOD bias), as the forward chain does.
use super::{get, Img, Vk};
use crate::passthrough::{Draw, Layout};
use crate::stub::device::Dev;
use crate::stub::obj::SObj;
use crate::trace::once;
use crate::{log, sfmt};
use alloc::boxed::Box;
use alloc::vec::Vec;
use ash::vk;
use core::ffi::c_void;
use gpu::display::{VkPass8, PASS_ENTER, PASS_PUBLISH};

use crate::shaders::ff_spv::{PS_FF_SPV, VS_BUMP_SPV, VS_FF_SPV};

/// Bytes of the vertex block (56 registers) and the pixel block (7), and where the
/// pixel block starts in their one allocation.
const VS_BYTES: usize = 56 * 16;
const PS_BYTES: usize = 7 * 16;
const PS_AT: u32 = 1024;

/// The render pass a pipeline is made for: what Vulkan makes them compatible by.
#[derive(Clone, Copy, PartialEq, Eq)]
struct RpKey {
    color: vk::Format,
    depth: vk::Format,
    samples: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    rp: RpKey,
    topology: vk::PrimitiveTopology,
    stride: u32,
    layout: Layout,
    /// The texture coordinate sets stages 0 and 1 read.
    sets: [u8; 2],
    /// ALPHABLENDENABLE, SRCBLEND, DESTBLEND, BLENDOP, COLORWRITEENABLE.
    blend: [u32; 5],
    /// ZENABLE, ZWRITEENABLE, ZFUNC (with no depth attachment: all off).
    depth: [u32; 3],
    cull: u32,
    /// The target's alpha is not kept (the engine's X8R8G8B8 back buffer): its
    /// destination alpha reads one.
    no_dst_alpha: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SKey {
    /// ADDRESSU, ADDRESSV, MAGFILTER, MINFILTER, MIPFILTER, MAXANISOTROPY, MIPMAPLODBIAS
    /// (f32 bits), MAXMIPLEVEL.
    tss: [u32; 8],
}

/// The pass a draw records into.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    pass: VkPass8,
}

struct Open {
    target: Target,
    /// What is bound inside it now.
    pipeline: vk::Pipeline,
    sets: [vk::DescriptorSet; 2],
    offsets: [u32; 2],
    viewport: [u32; 6],
}

pub struct Pt {
    dsl: [vk::DescriptorSetLayout; 2],
    layout: vk::PipelineLayout,
    vs: vk::ShaderModule,
    vs_bump: vk::ShaderModule,
    ps: vk::ShaderModule,
    pipelines: Vec<(Key, vk::Pipeline)>,
    passes: Vec<(RpKey, vk::RenderPass)>,
    samplers: Vec<(SKey, vk::Sampler)>,
    white: Img,
    white_ready: bool,
    limits: vk::PhysicalDeviceLimits,
    /// The frame (Display8's index) the per-frame state below belongs to.
    frame: u64,
    ubo_sets: Vec<(u64, vk::DescriptorSet)>,
    tex_sets: Vec<([u64; 4], vk::DescriptorSet)>,
    fbs: Vec<([u64; 2], vk::RenderPass, vk::Framebuffer)>,
    /// What every vertex reads where its FVF has nothing: (buffer, offset).
    defaults: Option<(u64, u32)>,
    last: Option<([u8; VS_BYTES + PS_BYTES], u64, u32)>,
    open: Option<Open>,
    /// The pass a plugin published last, and the frame it is for.
    published: Option<(VkPass8, u64)>,
    /// Plugins' `Cmd`s alive now.
    alive: u32,
    drawn: u64,
    failed: bool,
}

static mut PT: Option<Pt> = None;

#[allow(static_mut_refs)]
fn pt() -> Option<&'static mut Pt> {
    unsafe { PT.as_mut() }
}

/// The render pass of passthrough's own, if one is open, closed: before anything else
/// records into the frame, or the frame is handed over.
pub fn close(v: &Vk) {
    if let Some(p) = pt() {
        end(v, p);
    }
}

fn end(v: &Vk, p: &mut Pt) {
    if p.open.take().is_some() {
        unsafe { v.dev.cmd_end_render_pass(v.cmd()) };
    }
}

/// The engine's `Present`: what was published is for the frame presented.
pub fn presented(v: &Vk) {
    close(v);
    if let Some(p) = pt() {
        p.published = None;
    }
}

/// `display8v1_vk_pass`: a plugin's `Cmd` begins, ends, or ends with its pass now.
pub fn event(event: u32, pass: *const VkPass8) {
    let Some(v) = get() else { return };
    close(v);
    if pt().is_none() {
        // Kept from the first event on, so a `Cmd` alive at the first draw is known.
        if !crate::passthrough::on() {
            return;
        }
        if !make(v) {
            return;
        }
    }
    let Some(p) = pt() else { return };
    match event {
        PASS_ENTER => p.alive += 1,
        _ => {
            p.alive = p.alive.saturating_sub(1);
            if event == PASS_PUBLISH && !pass.is_null() {
                // Only what the plugin's size says (gpu/src/display.rs, "Versions").
                p.published = Some((unsafe { gpu::display::read(pass) }, v.index));
            }
        }
    }
}

fn make(v: &Vk) -> bool {
    if unsafe { PT.is_some() } {
        return true;
    }
    let made = (|| unsafe {
        let d = &v.dev;
        let (vsf, psf) = (vk::ShaderStageFlags::VERTEX, vk::ShaderStageFlags::FRAGMENT);
        let b0 = [
            vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).descriptor_count(1).stage_flags(vsf),
            vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).descriptor_count(1).stage_flags(psf),
        ];
        let b1 = [
            vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(psf),
            vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(psf),
        ];
        let s0 = d.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b0), None).ok()?;
        let s1 = d.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::default().bindings(&b1), None).ok()?;
        let dsl = [s0, s1];
        let layout = d.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().set_layouts(&dsl), None).ok()?;
        let vs = d.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&VS_FF_SPV), None).ok()?;
        let ps = d.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&PS_FF_SPV), None).ok()?;
        let vs_bump = d.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&VS_BUMP_SPV), None).ok()?;
        let white = v.image(1, 1, 1, vk::Format::B8G8R8A8_UNORM, vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST, vk::ImageAspectFlags::COLOR).ok()?;
        Some((dsl, layout, vs, vs_bump, ps, white))
    })();
    let Some((dsl, layout, vs, vs_bump, ps, white)) = made else {
        log!("passthrough: its pipeline layout or shaders could not be made: the engine's draws are dropped");
        return false;
    };
    let limits = unsafe { v.instance.get_physical_device_properties(v.pdev) }.limits;
    unsafe {
        PT = Some(Pt {
            dsl,
            layout,
            vs,
            vs_bump,
            ps,
            pipelines: Vec::new(),
            passes: Vec::new(),
            samplers: Vec::new(),
            white,
            white_ready: false,
            limits,
            frame: u64::MAX,
            ubo_sets: Vec::new(),
            tex_sets: Vec::new(),
            fbs: Vec::new(),
            defaults: None,
            last: None,
            open: None,
            published: None,
            alive: 0,
            drawn: 0,
            failed: false,
        })
    };
    log!("passthrough: on Vulkan; the engine's draws go into the pass render3d publishes, else the engine's targets");
    true
}

extern "C" fn destroy_fb(p: *mut c_void) {
    let fb = unsafe { Box::from_raw(p as *mut vk::Framebuffer) };
    if let Some(v) = get() {
        unsafe { v.dev.destroy_framebuffer(*fb, None) };
    }
}

/// A new frame: what the last one made per frame is let go of.
fn new_frame(v: &mut Vk, p: &mut Pt) {
    if p.frame == v.index {
        return;
    }
    p.frame = v.index;
    p.ubo_sets.clear();
    p.tex_sets.clear();
    for (_, _, fb) in p.fbs.drain(..) {
        // Let go of once the frame that last used it is done: deferred into the slot now,
        // whose fence covers the frame before (its last use).
        v.defer(destroy_fb, Box::into_raw(Box::new(fb)) as *mut c_void);
    }
    p.defaults = None;
    p.last = None;
}

fn render_pass(v: &Vk, p: &mut Pt, k: RpKey) -> vk::RenderPass {
    if let Some(r) = p.passes.iter().find(|x| x.0 == k) {
        return r.1;
    }
    let samples = samples_flag(k.samples);
    let mut atts = alloc::vec![vk::AttachmentDescription::default()
        .format(k.color)
        .samples(samples)
        .load_op(vk::AttachmentLoadOp::LOAD)
        .store_op(vk::AttachmentStoreOp::STORE)
        .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    if k.depth != vk::Format::UNDEFINED {
        atts.push(
            vk::AttachmentDescription::default()
                .format(k.depth)
                .samples(samples)
                .load_op(vk::AttachmentLoadOp::LOAD)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::LOAD)
                .stencil_store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        );
    }
    let cref = [vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }];
    let dref = vk::AttachmentReference { attachment: 1, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };
    let mut sub = vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&cref);
    if k.depth != vk::Format::UNDEFINED {
        sub = sub.depth_stencil_attachment(&dref);
    }
    let subs = [sub];
    // As gpu's: everything before is done before this pass reads or writes, and it is
    // done before what follows (a frame recorded once: correct before tight).
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
    let rp = unsafe { v.dev.create_render_pass(&vk::RenderPassCreateInfo::default().attachments(&atts).subpasses(&subs).dependencies(&deps), None) }.unwrap_or_default();
    p.passes.push((k, rp));
    rp
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

fn blend_factor(f: u32, no_dst_alpha: bool) -> vk::BlendFactor {
    use vk::BlendFactor as B;
    match f {
        1 => B::ZERO,
        2 => B::ONE,
        3 => B::SRC_COLOR,
        4 => B::ONE_MINUS_SRC_COLOR,
        5 => B::SRC_ALPHA,
        6 => B::ONE_MINUS_SRC_ALPHA,
        7 if no_dst_alpha => B::ONE,
        7 => B::DST_ALPHA,
        8 if no_dst_alpha => B::ZERO,
        8 => B::ONE_MINUS_DST_ALPHA,
        9 => B::DST_COLOR,
        10 => B::ONE_MINUS_DST_COLOR,
        11 => B::SRC_ALPHA_SATURATE,
        _ => B::ONE,
    }
}

fn compare(f: u32) -> vk::CompareOp {
    match f {
        1 => vk::CompareOp::NEVER,
        2 => vk::CompareOp::LESS,
        3 => vk::CompareOp::EQUAL,
        4 => vk::CompareOp::LESS_OR_EQUAL,
        5 => vk::CompareOp::GREATER,
        6 => vk::CompareOp::NOT_EQUAL,
        7 => vk::CompareOp::GREATER_OR_EQUAL,
        _ => vk::CompareOp::ALWAYS,
    }
}

/// The vertex attributes: (location, binding, format, offset). Binding 1 is the
/// defaults (stride 0) for what the FVF lacks: position (0, 0, 0, 1) at 0, a zero normal
/// at 16, white diffuse at 28, black specular at 32, (0, 0) coordinates at 36.
fn attributes(k: &Key) -> Vec<vk::VertexInputAttributeDescription> {
    use vk::Format as F;
    let l = &k.layout;
    if let Some(o) = l.bump {
        // vs_bump: position, coordinates, tangent, binormal, normal.
        return alloc::vec![
            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: F::R32G32B32_SFLOAT, offset: o[0] },
            vk::VertexInputAttributeDescription { location: 1, binding: 0, format: F::R32G32_SFLOAT, offset: o[1] },
            vk::VertexInputAttributeDescription { location: 2, binding: 0, format: F::R32G32B32_SFLOAT, offset: o[2] },
            vk::VertexInputAttributeDescription { location: 3, binding: 0, format: F::R32G32B32_SFLOAT, offset: o[3] },
            vk::VertexInputAttributeDescription { location: 4, binding: 0, format: F::R32G32B32_SFLOAT, offset: o[4] },
        ];
    }
    let a = |loc: u32, own: Option<(u32, F)>, dflt: (u32, F)| {
        let (binding, (offset, format)) = match own {
            Some(o) => (0, o),
            None => (1, dflt),
        };
        vk::VertexInputAttributeDescription { location: loc, binding, format, offset }
    };
    let tex = |s: u8| {
        l.tex[s as usize].map(|(o, n)| (o, if n == 1 { F::R32_SFLOAT } else { F::R32G32_SFLOAT }))
    };
    alloc::vec![
        a(0, Some((0, if l.rhw { F::R32G32B32A32_SFLOAT } else { F::R32G32B32_SFLOAT })), (0, F::R32G32B32A32_SFLOAT)),
        a(1, l.normal.map(|o| (o, F::R32G32B32_SFLOAT)), (16, F::R32G32B32_SFLOAT)),
        a(2, l.diffuse.map(|o| (o, F::B8G8R8A8_UNORM)), (28, F::B8G8R8A8_UNORM)),
        a(3, l.specular.map(|o| (o, F::B8G8R8A8_UNORM)), (32, F::B8G8R8A8_UNORM)),
        a(4, tex(k.sets[0]), (36, F::R32G32_SFLOAT)),
        a(5, tex(k.sets[1]), (36, F::R32G32_SFLOAT)),
    ]
}

fn pipeline(v: &Vk, p: &mut Pt, k: &Key) -> vk::Pipeline {
    if let Some(x) = p.pipelines.iter().find(|x| x.0 == *k) {
        return x.1;
    }
    let rp = render_pass(v, p, k.rp);
    let stages = [
        if k.layout.bump.is_some() {
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(p.vs_bump).name(c"vs_bump")
        } else {
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(p.vs).name(c"vs_ff")
        },
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(p.ps).name(c"ps_ff"),
    ];
    let vb = [
        vk::VertexInputBindingDescription { binding: 0, stride: k.stride, input_rate: vk::VertexInputRate::VERTEX },
        vk::VertexInputBindingDescription { binding: 1, stride: 0, input_rate: vk::VertexInputRate::VERTEX },
    ];
    let va = attributes(k);
    let vi = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&vb).vertex_attribute_descriptions(&va);
    let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(k.topology);
    let vp = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    // Direct3D's culling on screen, as gpu's (the vertex shader turns y over): CW drops
    // what is clockwise on screen.
    let cull = match k.cull {
        2 => vk::CullModeFlags::BACK,
        3 => vk::CullModeFlags::FRONT,
        _ => vk::CullModeFlags::NONE,
    };
    let rs = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(cull)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(samples_flag(k.rp.samples));
    let has_depth = k.rp.depth != vk::Format::UNDEFINED;
    let ds = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(has_depth && k.depth[0] != 0)
        .depth_write_enable(has_depth && k.depth[0] != 0 && k.depth[1] != 0)
        .depth_compare_op(compare(k.depth[2]));
    let [on, src, dst, op, mask] = k.blend;
    let (src, dst) = match src {
        12 => (5, 6),
        13 => (6, 5),
        _ => (src, dst),
    };
    let bop = match op {
        2 => vk::BlendOp::SUBTRACT,
        3 => vk::BlendOp::REVERSE_SUBTRACT,
        4 => vk::BlendOp::MIN,
        5 => vk::BlendOp::MAX,
        _ => vk::BlendOp::ADD,
    };
    let (sf, df) = (blend_factor(src, k.no_dst_alpha), blend_factor(dst, k.no_dst_alpha));
    let cba = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(on != 0)
        .src_color_blend_factor(sf)
        .dst_color_blend_factor(df)
        .color_blend_op(bop)
        .src_alpha_blend_factor(sf)
        .dst_alpha_blend_factor(df)
        .alpha_blend_op(bop)
        .color_write_mask(vk::ColorComponentFlags::from_raw(mask & 0xf))];
    let cb = vk::PipelineColorBlendStateCreateInfo::default().attachments(&cba);
    let dyns = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dy = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyns);
    let gpci = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb)
        .dynamic_state(&dy)
        .layout(p.layout)
        .render_pass(rp)
        .subpass(0)];
    let pl = match unsafe { v.dev.create_graphics_pipelines(v.cache, &gpci, None) } {
        Ok(x) => x[0],
        Err(_) => {
            once("PASSTHROUGH a pipeline could not be made: its draws are not drawn");
            vk::Pipeline::null()
        }
    };
    p.pipelines.push((*k, pl));
    if p.pipelines.len().is_power_of_two() {
        log!("passthrough: {} pipelines", p.pipelines.len());
    }
    pl
}

fn sampler(v: &Vk, p: &mut Pt, tss: &[u32; 32]) -> vk::Sampler {
    let k = SKey { tss: [tss[13], tss[14], tss[16], tss[17], tss[18], tss[21], tss[19], tss[20]] };
    if let Some(x) = p.samplers.iter().find(|x| x.0 == k) {
        return x.1;
    }
    let addr = |a: u32| match a {
        2 | 5 => vk::SamplerAddressMode::MIRRORED_REPEAT,
        3 => vk::SamplerAddressMode::CLAMP_TO_EDGE,
        4 => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        _ => vk::SamplerAddressMode::REPEAT,
    };
    let filt = |f: u32| if f >= 2 { vk::Filter::LINEAR } else { vk::Filter::NEAREST };
    let [au, av, mag, min, mip, most, bias, maxmip] = k.tss;
    // As DXVK makes it under the game's dxvk.conf (README.md, "Samplers as DXVK makes
    // them"): a linear filter anisotropic at the forced amount, the forced bias added
    // on a sampler with mips.
    let forced = v.exported.sampler_anisotropy;
    let have = v.exported.features & gpu::display::FEATURE_ANISOTROPY != 0;
    let wants = |f: u32| f == 3 || (forced >= 0 && f == 2);
    let aniso = have && (wants(min) || wants(mag));
    let amount = if forced >= 0 { forced as u32 } else { most.max(1) };
    let lim = p.limits.max_sampler_lod_bias;
    let lod = if mip == 0 { 0.0 } else { (f32::from_bits(bias) + v.exported.sampler_lod_bias).clamp(-lim, lim) };
    let ci = vk::SamplerCreateInfo::default()
        .mag_filter(filt(mag))
        .min_filter(filt(min))
        .mipmap_mode(if mip >= 2 { vk::SamplerMipmapMode::LINEAR } else { vk::SamplerMipmapMode::NEAREST })
        .address_mode_u(addr(au))
        .address_mode_v(addr(av))
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(aniso)
        .max_anisotropy(if aniso { (amount.clamp(1, 16) as f32).min(p.limits.max_sampler_anisotropy) } else { 1.0 })
        .mip_lod_bias(lod)
        .min_lod(if mip == 0 { 0.0 } else { maxmip as f32 })
        .max_lod(if mip == 0 { 0.0 } else { vk::LOD_CLAMP_NONE })
        .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK);
    let s = unsafe { v.dev.create_sampler(&ci, None) }.unwrap_or_default();
    p.samplers.push((k, s));
    s
}

/// The texture stage `s` reads (a mirror, made at its first draw), or the white one.
/// A mirror made or brought up to date records outside a render pass: ours is closed
/// first.
fn texture(v: &mut Vk, p: &mut Pt, dv: &Dev, s: usize) -> vk::ImageView {
    let t = dv.textures[s];
    if !t.is_null() && crate::stub::obj::is_stub(t) {
        let o = t as *mut SObj;
        if unsafe { (*o).kind } == crate::com::Kind::Texture {
            if super::mirror::stale(o) {
                end(v, p);
            }
            if let Some(m) = super::mirror::passthrough(o) {
                return vk::Handle::from_raw(m.view);
            }
        } else {
            once("PASSTHROUGH a cube or volume texture: drawn with a white one");
        }
    }
    white(v, p)
}

/// The 1x1 white texture a stage without one reads.
fn white(v: &mut Vk, p: &mut Pt) -> vk::ImageView {
    if !p.white_ready {
        end(v, p);
        p.white_ready = true;
        let img = p.white.image;
        v.upload(img, vk::ImageAspectFlags::COLOR, 0, 1, 1, &[255, 255, 255, 255], 4, 4, vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    }
    p.white.view
}

fn f4(b: &mut [u8], reg: usize, v: [f32; 4]) {
    for (i, x) in v.iter().enumerate() {
        b[reg * 16 + i * 4..reg * 16 + i * 4 + 4].copy_from_slice(&x.to_le_bytes());
    }
}

fn color(c: u32) -> [f32; 4] {
    [((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, (c >> 24) as f32 / 255.0]
}

fn transform(dv: &Dev, state: u32) -> [f32; 16] {
    dv.transforms.iter().find(|t| t.0 == state).map_or([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0], |t| t.1)
}

fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut r = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            r[i * 4 + j] = (0..4).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
        }
    }
    r
}

/// The two constant blocks (`shaders/ff.hlsl`, `ff_vs` and `ff_ps`), one after the other.
fn blocks(v: &Vk, dv: &Dev, d: &Draw, vp: [u32; 6]) -> [u8; VS_BYTES + PS_BYTES] {
    let mut b = [0u8; VS_BYTES + PS_BYTES];
    let rs = &dv.rs;
    let world = transform(dv, 256);
    let wvp = mul(&mul(&world, &transform(dv, 2)), &transform(dv, 3));
    for r in 0..4 {
        f4(&mut b, r, [wvp[r * 4], wvp[r * 4 + 1], wvp[r * 4 + 2], wvp[r * 4 + 3]]);
        f4(&mut b, 4 + r, [world[r * 4], world[r * 4 + 1], world[r * 4 + 2], world[r * 4 + 3]]);
    }
    f4(&mut b, 8, [vp[0] as f32, vp[1] as f32, vp[2].max(1) as f32, vp[3].max(1) as f32]);
    // Direct3D 9's pixel centres, as DXVK keeps them (gpu/README.md, "Pixel centres"):
    // the engine's screen-space draws always; its 3D ones in the comparison mode, as the
    // plugins' scenes are.
    let centre = 0.5 - 1.0 / 128.0;
    let off = if d.layout.rhw || v.exported.features & gpu::display::FEATURE_D3D9_CENTRES != 0 { centre } else { 0.0 };
    f4(&mut b, 9, [f32::from_bits(vp[4]), f32::from_bits(vp[5]), off, 0.0]);
    let lit = rs[137] != 0 && !d.layout.rhw;
    let lights: Vec<&(u32, [u32; 26], bool)> = if lit { dv.lights.iter().filter(|l| l.2).take(8).collect() } else { Vec::new() };
    let vcol = if rs[141] != 0 { d.layout.diffuse.is_some() as u32 + 2 * d.layout.specular.is_some() as u32 } else { 0 };
    f4(&mut b, 10, [d.layout.rhw as u32 as f32, lit as u32 as f32, lights.len() as f32, vcol as f32]);
    let m = &dv.material;
    f4(&mut b, 11, [m[0], m[1], m[2], m[3]]);
    f4(&mut b, 12, [m[4], m[5], m[6], m[7]]);
    f4(&mut b, 13, [m[12], m[13], m[14], m[15]]);
    f4(&mut b, 14, color(rs[139]));
    f4(&mut b, 15, [rs[145] as f32, rs[147] as f32, rs[148] as f32, 0.0]);
    if d.layout.bump.is_some() {
        // The bump shader's constants 0 to 6 (vs_bump).
        for r in 0..7 {
            f4(&mut b, 16 + r, dv.vs_const[r]);
        }
    }
    for (i, l) in lights.iter().enumerate() {
        // D3DLIGHT8: type; diffuse, specular, ambient (4 each); position, direction (3
        // each); range, falloff, attenuation 0, 1, 2, theta, phi.
        let f = |n: usize| f32::from_bits(l.1[n]);
        if l.1[0] == 2 {
            once("PASSTHROUGH a spot light: lit as a point light");
        }
        let r = 16 + 5 * i;
        f4(&mut b, r, [f(13), f(14), f(15), l.1[0] as f32]);
        f4(&mut b, r + 1, [f(16), f(17), f(18), f(19)]);
        f4(&mut b, r + 2, [f(1), f(2), f(3), f(4)]);
        f4(&mut b, r + 3, [f(9), f(10), f(11), f(21)]);
        f4(&mut b, r + 4, [f(22), f(23), 0.0, 0.0]);
    }
    // The pixel block.
    let ps = &mut b[VS_BYTES..];
    let t = &dv.tss;
    let st = |s: usize| [t[s][1] as f32, t[s][2] as f32, t[s][3] as f32, t[s][4] as f32];
    let stb = |s: usize| [t[s][5] as f32, t[s][6] as f32, t[s][26] as f32, t[s][27] as f32];
    f4(ps, 0, st(0));
    f4(ps, 1, stb(0));
    f4(ps, 2, st(1));
    f4(ps, 3, stb(1));
    f4(ps, 4, color(rs[60]));
    f4(ps, 5, [(rs[15] != 0) as u32 as f32, rs[25] as f32, (rs[24] & 255) as f32, (rs[29] != 0) as u32 as f32]);
    f4(ps, 6, [(t[0][28] == 5) as u32 as f32, (t[1][28] == 5) as u32 as f32, 0.0, 0.0]);
    b
}

/// What is logged once of state the passthrough does not draw as Direct3D 8 would.
fn note_unsupported(dv: &Dev) {
    let rs = &dv.rs;
    if rs[28] != 0 {
        once("PASSTHROUGH fog is on: drawn without fog");
    }
    if rs[8] != 3 && rs[8] != 0 {
        once(&sfmt!("PASSTHROUGH FILLMODE {}: drawn solid", rs[8]));
    }
    if rs[47] != 0 {
        once(&sfmt!("PASSTHROUGH ZBIAS {}: drawn without a depth bias", rs[47]));
    }
    for s in 0..2 {
        let op = dv.tss[s][1];
        if matches!(op, 17 | 22 | 23) || op > 26 {
            once(&sfmt!("PASSTHROUGH stage {} COLOROP {}: drawn as SELECTARG1", s, op));
        }
        if dv.tss[s][24] != 0 {
            once("PASSTHROUGH a texture transform: drawn without it");
        }
        if dv.tss[s][11] > 0xffff {
            once("PASSTHROUGH generated texture coordinates: drawn with the set's own");
        }
    }
    if dv.tss[2][1] > 1 {
        once("PASSTHROUGH a third texture stage: drawn with two");
    }
    if dv.ps != 0 {
        once("PASSTHROUGH a pixel shader: drawn with fixed function");
    }
}

fn topology(prim: u32) -> vk::PrimitiveTopology {
    match prim {
        1 => vk::PrimitiveTopology::POINT_LIST,
        2 => vk::PrimitiveTopology::LINE_LIST,
        3 => vk::PrimitiveTopology::LINE_STRIP,
        5 => vk::PrimitiveTopology::TRIANGLE_STRIP,
        _ => vk::PrimitiveTopology::TRIANGLE_LIST,
    }
}

/// A frame's descriptor set from Display8's pool.
fn set(v: &Vk, p: &Pt, which: usize) -> Option<vk::DescriptorSet> {
    let pool: vk::DescriptorPool = vk::Handle::from_raw(v.frame_out.descriptor_pool);
    let l = [p.dsl[which]];
    unsafe { v.dev.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&l)) }.ok().map(|s| s[0])
}

/// Records one draw of the engine's (`passthrough::draw`).
pub fn draw(v: &mut Vk, dv: &Dev, d: &Draw, fp: usize) {
    if pt().is_none() && !make(v) {
        return;
    }
    let Some(p) = pt() else { return };
    if p.failed {
        return;
    }
    v.frame();
    new_frame(v, p);
    if p.alive > 0 {
        once(&sfmt!("PASSTHROUGH an engine draw while a plugin records: not drawn, by {}", crate::trace::caller(fp)));
        return;
    }
    note_unsupported(dv);
    // Where it goes: the pass published in this frame, unless that is the engine's own
    // back buffer (then the engine's targets, with their depth).
    let own = VkPass8 { size: core::mem::size_of::<VkPass8>() as u32, color: v.target(gpu::display::TARGET_COLOR), depth: v.target(gpu::display::TARGET_DEPTH) };
    let pass = match p.published {
        Some((x, f)) if f == v.index && x.color.image != own.color.image => x,
        _ => own,
    };
    let target = Target { pass };
    let is_own = pass.color.image == own.color.image;
    let (tw, th) = (pass.color.width, pass.color.height);

    // Textures first: a mirror made or brought up to date records outside a render pass.
    let stage_on = |s: usize| dv.tss[s][1] > 1 && (s == 0 || dv.tss[0][1] > 1);
    let mut views = [vk::ImageView::null(); 2];
    let mut samps = [vk::Sampler::null(); 2];
    for s in 0..2 {
        views[s] = if stage_on(s) { texture(v, p, dv, s) } else { white(v, p) };
        samps[s] = sampler(v, p, &dv.tss[s]);
    }

    // The viewport, inside the target.
    let mut vp = dv.viewport;
    vp[0] = vp[0].min(tw.saturating_sub(1));
    vp[1] = vp[1].min(th.saturating_sub(1));
    vp[2] = vp[2].min(tw - vp[0]).max(1);
    vp[3] = vp[3].min(th - vp[1]).max(1);

    let rs = &dv.rs;
    let depth_fmt: vk::Format = if pass.depth.image != 0 { vk::Format::from_raw(pass.depth.format as i32) } else { vk::Format::UNDEFINED };
    let zbuffer = !dv.zbuffer.is_null();
    let key = Key {
        rp: RpKey { color: vk::Format::from_raw(pass.color.format as i32), depth: depth_fmt, samples: pass.color.samples.max(1) },
        topology: topology(d.prim),
        stride: d.stride,
        layout: d.layout,
        sets: [(dv.tss[0][11] & 7) as u8, (dv.tss[1][11] & 7) as u8],
        blend: [rs[27], rs[19], rs[20], rs[171], rs[168]],
        depth: if zbuffer && rs[7] != 0 { [1, rs[14], rs[23]] } else { [0, 0, 8] },
        cull: rs[22],
        no_dst_alpha: is_own,
    };
    let pl = pipeline(v, p, &key);
    if pl == vk::Pipeline::null() {
        return;
    }

    // The constant blocks, unless the same as the last draw's.
    let ua = v.uniform_align.max(16);
    let bl = blocks(v, dv, d, vp);
    let (ubuf, uoff) = match &p.last {
        Some((b, buf, off)) if b[..] == bl[..] => (*buf, *off),
        _ => {
            let Some(a) = v.alloc(PS_AT + 256, ua) else { return };
            unsafe {
                core::ptr::copy_nonoverlapping(bl.as_ptr(), a.cpu, VS_BYTES);
                core::ptr::copy_nonoverlapping(bl.as_ptr().add(VS_BYTES), a.cpu.add(PS_AT as usize), PS_BYTES);
            }
            p.last = Some((bl, a.buffer, a.offset));
            (a.buffer, a.offset)
        }
    };
    // The vertices, the indices, the defaults.
    let Some(va) = v.alloc(d.verts.len() as u32, 16) else { return };
    unsafe { core::ptr::copy_nonoverlapping(d.verts.as_ptr(), va.cpu, d.verts.len()) };
    let ia = match d.indices {
        Some((ix, _)) => {
            let Some(a) = v.alloc(ix.len() as u32, 4) else { return };
            unsafe { core::ptr::copy_nonoverlapping(ix.as_ptr(), a.cpu, ix.len()) };
            Some(a)
        }
        None => None,
    };
    let defaults = match p.defaults {
        Some(x) => x,
        None => {
            let Some(a) = v.alloc(48, 16) else { return };
            let mut b = [0u8; 48];
            b[12..16].copy_from_slice(&1.0f32.to_le_bytes());
            b[28..32].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
            unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), a.cpu, 48) };
            p.defaults = Some((a.buffer, a.offset));
            (a.buffer, a.offset)
        }
    };

    // The sets: the blocks' (one per ring chunk in a frame) and the textures'.
    let uset = match p.ubo_sets.iter().find(|x| x.0 == ubuf) {
        Some(x) => x.1,
        None => {
            let Some(s) = set(v, p, 0) else {
                once("PASSTHROUGH the frame's descriptor pool is full: draws are not drawn");
                return;
            };
            let b0 = [vk::DescriptorBufferInfo { buffer: vk::Handle::from_raw(ubuf), offset: 0, range: VS_BYTES as u64 }];
            let b1 = [vk::DescriptorBufferInfo { buffer: vk::Handle::from_raw(ubuf), offset: 0, range: PS_BYTES as u64 }];
            let w = [
                vk::WriteDescriptorSet::default().dst_set(s).dst_binding(0).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).buffer_info(&b0),
                vk::WriteDescriptorSet::default().dst_set(s).dst_binding(1).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC).buffer_info(&b1),
            ];
            unsafe { v.dev.update_descriptor_sets(&w, &[]) };
            p.ubo_sets.push((ubuf, s));
            s
        }
    };
    let tkey = [vk::Handle::as_raw(views[0]), vk::Handle::as_raw(samps[0]), vk::Handle::as_raw(views[1]), vk::Handle::as_raw(samps[1])];
    let tset = match p.tex_sets.iter().find(|x| x.0 == tkey) {
        Some(x) => x.1,
        None => {
            let Some(s) = set(v, p, 1) else {
                once("PASSTHROUGH the frame's descriptor pool is full: draws are not drawn");
                return;
            };
            let i0 = [vk::DescriptorImageInfo { sampler: samps[0], image_view: views[0], image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
            let i1 = [vk::DescriptorImageInfo { sampler: samps[1], image_view: views[1], image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
            let w = [
                vk::WriteDescriptorSet::default().dst_set(s).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&i0),
                vk::WriteDescriptorSet::default().dst_set(s).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&i1),
            ];
            unsafe { v.dev.update_descriptor_sets(&w, &[]) };
            p.tex_sets.push((tkey, s));
            s
        }
    };

    // The render pass: the one open if it is on the same target, else one opened.
    let cmd = v.cmd();
    if p.open.as_ref().is_some_and(|o| o.target != target) {
        end(v, p);
    }
    if p.open.is_none() {
        let rp = render_pass(v, p, key.rp);
        let vkey = [pass.color.view, pass.depth.view];
        let fb = match p.fbs.iter().find(|x| x.0 == vkey && x.1 == rp) {
            Some(x) => x.2,
            None => {
                let mut atts = alloc::vec![vk::Handle::from_raw(pass.color.view)];
                if depth_fmt != vk::Format::UNDEFINED {
                    atts.push(vk::Handle::from_raw(pass.depth.view));
                }
                let fb = unsafe {
                    v.dev.create_framebuffer(&vk::FramebufferCreateInfo::default().render_pass(rp).attachments(&atts).width(tw).height(th).layers(1), None)
                };
                let Ok(fb) = fb else {
                    once("PASSTHROUGH a framebuffer could not be made: draws are not drawn");
                    return;
                };
                p.fbs.push((vkey, rp, fb));
                fb
            }
        };
        let bi = vk::RenderPassBeginInfo::default()
            .render_pass(rp)
            .framebuffer(fb)
            .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: tw, height: th } });
        unsafe { v.dev.cmd_begin_render_pass(cmd, &bi, vk::SubpassContents::INLINE) };
        p.open = Some(Open { target, pipeline: vk::Pipeline::null(), sets: [vk::DescriptorSet::null(); 2], offsets: [u32::MAX; 2], viewport: [u32::MAX; 6] });
    }
    let o = p.open.as_mut().unwrap();
    unsafe {
        let dev = &v.dev;
        if o.pipeline != pl {
            dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pl);
            o.pipeline = pl;
        }
        let offs = [uoff, uoff + PS_AT];
        if o.sets != [uset, tset] || o.offsets != offs {
            dev.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::GRAPHICS, p.layout, 0, &[uset, tset], &offs);
            o.sets = [uset, tset];
            o.offsets = offs;
        }
        if o.viewport != vp {
            let (mz, xz) = (f32::from_bits(vp[4]).clamp(0.0, 1.0), f32::from_bits(vp[5]).clamp(0.0, 1.0));
            dev.cmd_set_viewport(cmd, 0, &[vk::Viewport { x: vp[0] as f32, y: vp[1] as f32, width: vp[2] as f32, height: vp[3] as f32, min_depth: mz, max_depth: xz }]);
            dev.cmd_set_scissor(cmd, 0, &[vk::Rect2D { offset: vk::Offset2D { x: vp[0] as i32, y: vp[1] as i32 }, extent: vk::Extent2D { width: vp[2], height: vp[3] } }]);
            o.viewport = vp;
        }
        dev.cmd_bind_vertex_buffers(cmd, 0, &[vk::Handle::from_raw(va.buffer), vk::Handle::from_raw(defaults.0)], &[va.offset as u64, defaults.1 as u64]);
        match (ia, d.indices) {
            (Some(a), Some((_, wide))) => {
                dev.cmd_bind_index_buffer(cmd, vk::Handle::from_raw(a.buffer), a.offset as u64, if wide { vk::IndexType::UINT32 } else { vk::IndexType::UINT16 });
                dev.cmd_draw_indexed(cmd, d.count, 1, 0, d.base, 0);
            }
            _ => dev.cmd_draw(cmd, d.count, 1, 0, 0),
        }
    }
    p.drawn += 1;
    if p.drawn.is_power_of_two() && p.drawn >= 1024 {
        log!("passthrough: {} draws of the engine's so far", p.drawn);
    } else if p.drawn == 1 {
        log!("passthrough: the engine's first draw, into {}", if is_own { "the engine's targets" } else { "a plugin's pass" });
    }
}
