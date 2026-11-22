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
//!   under `Display8.ini`'s `[Samplers]` or else the game's `dxvk.conf` (anisotropy, LOD bias), as the forward chain does.
use super::{get, Img, Vk};
use crate::passthrough::state::{self, Layout, PipeState, PS_AT, PS_BYTES, VS_BYTES};
use crate::passthrough::{fixed, say, Draw};
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
    stride: u32,
    layout: Layout,
    /// Topology, coordinate sets, blend, depth and cull (`state::pipe_state`).
    st: PipeState,
    /// The target's alpha is not kept (the engine's X8R8G8B8 back buffer): its
    /// destination alpha reads one.
    no_dst_alpha: bool,
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
    samplers: Vec<(state::SKey, vk::Sampler)>,
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
    last: Option<(state::Blocks, u64, u32)>,
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
        PASS_ENTER => {
            p.alive += 1;
            crate::host::drew();
        }
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

/// A blend factor, from Direct3D's number (`state::blend` made it one of 1 to 11).
fn blend_factor(f: u32) -> vk::BlendFactor {
    use vk::BlendFactor as B;
    match f {
        1 => B::ZERO,
        3 => B::SRC_COLOR,
        4 => B::ONE_MINUS_SRC_COLOR,
        5 => B::SRC_ALPHA,
        6 => B::ONE_MINUS_SRC_ALPHA,
        7 => B::DST_ALPHA,
        8 => B::ONE_MINUS_DST_ALPHA,
        9 => B::DST_COLOR,
        10 => B::ONE_MINUS_DST_COLOR,
        11 => B::SRC_ALPHA_SATURATE,
        _ => B::ONE,
    }
}

fn compare(f: u32) -> vk::CompareOp {
    match state::compare(f) {
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
    use state::Fmt;
    use vk::Format as F;
    let (attrs, n) = state::attributes(&k.layout, k.st.sets);
    attrs[..n]
        .iter()
        .map(|a| vk::VertexInputAttributeDescription {
            location: a.location,
            binding: a.binding,
            format: match a.fmt {
                Fmt::F1 => F::R32_SFLOAT,
                Fmt::F2 => F::R32G32_SFLOAT,
                Fmt::F3 => F::R32G32B32_SFLOAT,
                Fmt::F4 => F::R32G32B32A32_SFLOAT,
                Fmt::Bgra8 => F::B8G8R8A8_UNORM,
            },
            offset: a.offset,
        })
        .collect()
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
    let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(topology(k.st.topology));
    let vp = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    // Direct3D's culling on screen, as gpu's (the vertex shader turns y over): CW drops
    // what is clockwise on screen.
    let cull = match k.st.cull {
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
        .depth_test_enable(has_depth && k.st.depth[0] != 0)
        .depth_write_enable(has_depth && k.st.depth[0] != 0 && k.st.depth[1] != 0)
        .depth_compare_op(compare(k.st.depth[2]));
    let b = state::blend(k.st.blend, k.no_dst_alpha);
    let bop = match b.op {
        2 => vk::BlendOp::SUBTRACT,
        3 => vk::BlendOp::REVERSE_SUBTRACT,
        4 => vk::BlendOp::MIN,
        5 => vk::BlendOp::MAX,
        _ => vk::BlendOp::ADD,
    };
    let (sf, df) = (blend_factor(b.src), blend_factor(b.dst));
    let cba = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(b.on)
        .src_color_blend_factor(sf)
        .dst_color_blend_factor(df)
        .color_blend_op(bop)
        .src_alpha_blend_factor(sf)
        .dst_alpha_blend_factor(df)
        .alpha_blend_op(bop)
        .color_write_mask(vk::ColorComponentFlags::from_raw(b.mask))];
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
    let k = state::sampler_key(tss);
    if let Some(x) = p.samplers.iter().find(|x| x.0 == k) {
        return x.1;
    }
    let env = state::SamplerEnv {
        forced_anisotropy: v.exported.sampler_anisotropy,
        forced_bias: v.exported.sampler_lod_bias,
        have_anisotropy: v.exported.features & gpu::display::FEATURE_ANISOTROPY != 0,
        max_bias: p.limits.max_sampler_lod_bias,
        max_anisotropy: p.limits.max_sampler_anisotropy,
    };
    let q = state::sampler_params(&k, &env);
    let addr = |a: u32| match a {
        2 => vk::SamplerAddressMode::MIRRORED_REPEAT,
        3 => vk::SamplerAddressMode::CLAMP_TO_EDGE,
        4 => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        _ => vk::SamplerAddressMode::REPEAT,
    };
    let filt = |l: bool| if l { vk::Filter::LINEAR } else { vk::Filter::NEAREST };
    let ci = vk::SamplerCreateInfo::default()
        .mag_filter(filt(q.mag_linear))
        .min_filter(filt(q.min_linear))
        .mipmap_mode(if q.mip_linear { vk::SamplerMipmapMode::LINEAR } else { vk::SamplerMipmapMode::NEAREST })
        .address_mode_u(addr(q.address_u))
        .address_mode_v(addr(q.address_v))
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(q.anisotropy.is_some())
        .max_anisotropy(q.anisotropy.unwrap_or(1.0))
        .mip_lod_bias(q.lod_bias)
        .min_lod(q.min_lod)
        .max_lod(if q.max_lod >= state::LOD_NONE { vk::LOD_CLAMP_NONE } else { q.max_lod })
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
    let fx = fixed(dv);
    state::notes(&fx, &mut say);
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
    let vp = state::clamp_viewport(dv.viewport, tw, th);

    let depth_fmt: vk::Format = if pass.depth.image != 0 { vk::Format::from_raw(pass.depth.format as i32) } else { vk::Format::UNDEFINED };
    let key = Key {
        rp: RpKey { color: vk::Format::from_raw(pass.color.format as i32), depth: depth_fmt, samples: pass.color.samples.max(1) },
        stride: d.stride,
        layout: d.layout,
        st: state::pipe_state(&fx, d.prim),
        no_dst_alpha: is_own,
    };
    let pl = pipeline(v, p, &key);
    if pl == vk::Pipeline::null() {
        return;
    }

    // The constant blocks, unless the same as the last draw's.
    let ua = v.uniform_align.max(16);
    let bl = state::blocks(&fx, &d.layout, vp, v.exported.features & gpu::display::FEATURE_D3D9_CENTRES != 0, &mut say);
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
            let Some(a) = v.alloc(state::DEFAULTS_BYTES as u32, 16) else { return };
            let b = state::defaults();
            unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), a.cpu, b.len()) };
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
    crate::host::engine_drew();
    if p.drawn.is_power_of_two() && p.drawn >= 1024 {
        log!("passthrough: {} draws of the engine's so far", p.drawn);
    } else if p.drawn == 1 {
        log!("passthrough: the engine's first draw, into {}", if is_own { "the engine's targets" } else { "a plugin's pass" });
    }
}
