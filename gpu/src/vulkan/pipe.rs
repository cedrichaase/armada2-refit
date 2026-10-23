//! Pipelines on Vulkan. A `PipelineDesc` names the states as Direct3D 9 defines them;
//! its `VkPipeline`s are made at first use, one for each kind of pass it is drawn in
//! (the formats and samples of the targets the pass really has: render3d's descriptions
//! name the scene's, the UI's the back buffer's, and a description that guessed wrong
//! must still draw as it did on Direct3D 9).
//!
//! - **Shaders** are the SPIR-V beside the d3dbc (`Code::spirv`), the pixel shader's
//!   inputs moved to its vertex shader's outputs of the same semantic and, with an alpha
//!   test, discarding as Direct3D 9's would (`spirv.rs`).
//! - **Vertex input**: each input the vertex shader declares, by semantic, from the
//!   layout's attribute of that semantic; one the layout lacks reads (0, 0, 0, 1) from a
//!   buffer of its own (binding 1, stride 0), Direct3D 9's default.
//! - **Rasteriser**: the culling as Direct3D 9 means it. The vertex shaders are compiled
//!   with `-fvk-invert-y`, so a triangle lands in the target as Direct3D 9 would put it,
//!   and is wound on screen as there: `CULL_CW` drops what is clockwise on screen, which
//!   is Vulkan's back face with a counter-clockwise front.
//! - **Depth bias** is Direct3D 9's in depth units; Vulkan's constant is in the depth
//!   format's smallest step, so it is scaled by 2^bits (2^23 for a float depth), as DXVK
//!   does.
use super::spirv::{self, Reflect, TexUse};
use super::{is_depth_vk, samples_flag, vlog, Garbage, Gpu, PassKey, Shared};
use crate::types::*;
use alloc::rc::Rc;
use alloc::vec::Vec;
use ash::vk;
use core::cell::{Cell, RefCell};

#[derive(Clone)]
pub struct Pipeline(pub(crate) Rc<PipeInner>);

pub(crate) struct PipeInner {
    pub desc: PipelineDesc,
    pub obj: RefCell<Option<PipeObj>>,
    failed: Cell<bool>,
}

pub(crate) struct PipeObj {
    sh: Rc<Shared>,
    vs: vk::ShaderModule,
    ps: vk::ShaderModule,
    attrs: Vec<vk::VertexInputAttributeDescription>,
    dummy: bool,
    variants: Vec<(PassKey, vk::Pipeline)>,
    /// The textures both stages read (by binding).
    pub textures: Vec<TexUse>,
    /// Bytes of each stage's register file the shaders read.
    pub vs_bytes: u32,
    pub ps_bytes: u32,
    pub needs_dummy_vb: bool,
}

impl Drop for PipeObj {
    fn drop(&mut self) {
        for &(_, p) in &self.variants {
            self.sh.retire(Garbage::Pipeline(p));
        }
        self.sh.retire(Garbage::Module(self.vs));
        self.sh.retire(Garbage::Module(self.ps));
    }
}

fn sem_name(s: Semantic) -> &'static str {
    match s {
        Semantic::Position => "POSITION",
        Semantic::Normal => "NORMAL",
        Semantic::TexCoord => "TEXCOORD",
        Semantic::Tangent => "TANGENT",
        Semantic::Color => "COLOR",
    }
}

fn attr_format(f: AttrFormat) -> vk::Format {
    match f {
        AttrFormat::Float1 => vk::Format::R32_SFLOAT,
        AttrFormat::Float2 => vk::Format::R32G32_SFLOAT,
        AttrFormat::Float3 => vk::Format::R32G32B32_SFLOAT,
        AttrFormat::Float4 => vk::Format::R32G32B32A32_SFLOAT,
        AttrFormat::Bgra8 => vk::Format::B8G8R8A8_UNORM,
        AttrFormat::UByte4 => vk::Format::R8G8B8A8_USCALED,
        AttrFormat::UByte4N => vk::Format::R8G8B8A8_UNORM,
        AttrFormat::Short2 => vk::Format::R16G16_SSCALED,
    }
}

fn factor(f: Factor) -> vk::BlendFactor {
    match f {
        Factor::Zero => vk::BlendFactor::ZERO,
        Factor::One => vk::BlendFactor::ONE,
        Factor::SrcColor => vk::BlendFactor::SRC_COLOR,
        Factor::InvSrcColor => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
        Factor::SrcAlpha => vk::BlendFactor::SRC_ALPHA,
        Factor::InvSrcAlpha => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        Factor::DstAlpha => vk::BlendFactor::DST_ALPHA,
        Factor::InvDstAlpha => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
        Factor::DstColor => vk::BlendFactor::DST_COLOR,
        Factor::InvDstColor => vk::BlendFactor::ONE_MINUS_DST_COLOR,
        Factor::SrcAlphaSat => vk::BlendFactor::SRC_ALPHA_SATURATE,
    }
}

fn blend_op(o: BlendOp) -> vk::BlendOp {
    match o {
        BlendOp::Add => vk::BlendOp::ADD,
        BlendOp::Subtract => vk::BlendOp::SUBTRACT,
        BlendOp::RevSubtract => vk::BlendOp::REVERSE_SUBTRACT,
        BlendOp::Min => vk::BlendOp::MIN,
        BlendOp::Max => vk::BlendOp::MAX,
    }
}

pub(crate) fn compare(c: Compare) -> vk::CompareOp {
    match c {
        Compare::Never => vk::CompareOp::NEVER,
        Compare::Less => vk::CompareOp::LESS,
        Compare::Equal => vk::CompareOp::EQUAL,
        Compare::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
        Compare::Greater => vk::CompareOp::GREATER,
        Compare::NotEqual => vk::CompareOp::NOT_EQUAL,
        Compare::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
        Compare::Always => vk::CompareOp::ALWAYS,
    }
}

fn topology(t: Topology) -> vk::PrimitiveTopology {
    match t {
        Topology::PointList => vk::PrimitiveTopology::POINT_LIST,
        Topology::LineList => vk::PrimitiveTopology::LINE_LIST,
        Topology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
        Topology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
        Topology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
    }
}

/// Direct3D 9's depth bias unit in Vulkan's for `f`: 2^bits of a fixed-point depth,
/// 2^23 of a float one.
fn bias_scale(f: vk::Format) -> f32 {
    match f {
        vk::Format::D16_UNORM | vk::Format::D16_UNORM_S8_UINT => 65536.0,
        vk::Format::X8_D24_UNORM_PACK32 | vk::Format::D24_UNORM_S8_UINT => 16_777_216.0,
        _ => 8_388_608.0,
    }
}

pub(crate) fn module(sh: &Shared, code: &[u32]) -> Option<vk::ShaderModule> {
    unsafe { sh.dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code), None) }.ok()
}

impl Pipeline {
    pub(crate) fn new(desc: PipelineDesc) -> Pipeline {
        Pipeline(Rc::new(PipeInner { desc, obj: RefCell::new(None), failed: Cell::new(false) }))
    }

    pub fn desc(&self) -> &PipelineDesc {
        &self.0.desc
    }

    /// Its shaders made: false when there is no SPIR-V, or the device refuses it.
    pub fn ready(&self, gpu: &Gpu) -> bool {
        self.realize(gpu)
    }

    pub(crate) fn realize(&self, gpu: &Gpu) -> bool {
        let p = &self.0;
        if p.obj.borrow().is_some() {
            return true;
        }
        if p.failed.get() {
            return false;
        }
        match self.make(gpu) {
            Some(o) => {
                *p.obj.borrow_mut() = Some(o);
                true
            }
            None => {
                p.failed.set(true);
                false
            }
        }
    }

    fn make(&self, gpu: &Gpu) -> Option<PipeObj> {
        let sh = gpu.sh()?;
        let d = &self.0.desc;
        if d.vs.spirv.is_empty() || d.ps.spirv.is_empty() {
            vlog!(gpu, "gpu: Vulkan: a pipeline without SPIR-V; what draws with it stands down");
            return None;
        }
        let (Some(vr), Some(pr)) = (spirv::reflect(d.vs.spirv), spirv::reflect(d.ps.spirv)) else {
            vlog!(gpu, "gpu: Vulkan: a shader that is not SPIR-V");
            return None;
        };
        let mut ps = spirv::link(d.ps.spirv, &vr.outputs);
        if let Some(a) = d.alpha_test {
            match spirv::alpha_interval(a.compare as u32, a.reference) {
                None => {}
                Some((lo, hi, inv)) => match spirv::alpha_test(&ps, lo, hi, inv) {
                    Some(w) => ps = w,
                    None => vlog!(gpu, "gpu: Vulkan: a pixel shader the alpha test cannot be put in; drawn without it"),
                },
            }
        }
        let vs_mod = module(&sh, d.vs.spirv);
        let ps_mod = module(&sh, &ps);
        let (Some(vs), Some(psm)) = (vs_mod, ps_mod) else {
            vlog!(gpu, "gpu: Vulkan: the device refused a shader module");
            if let Some(m) = vs_mod {
                unsafe { sh.dev.destroy_shader_module(m, None) };
            }
            if let Some(m) = ps_mod {
                unsafe { sh.dev.destroy_shader_module(m, None) };
            }
            return None;
        };
        let (attrs, dummy) = vertex_attrs(d, &vr);
        let mut textures: Vec<TexUse> = Vec::new();
        for t in vr.textures.iter().chain(pr.textures.iter()) {
            if !textures.iter().any(|x| x.binding == t.binding) && (t.binding as usize) < super::SLOTS {
                textures.push(*t);
            }
        }
        Some(PipeObj {
            sh,
            vs,
            ps: psm,
            attrs,
            dummy,
            variants: Vec::new(),
            textures,
            vs_bytes: vr.ubo_bytes.div_ceil(16) * 16,
            ps_bytes: pr.ubo_bytes.div_ceil(16) * 16,
            needs_dummy_vb: dummy,
        })
    }

    /// Its `VkPipeline` for a pass of `key` (made at the first draw in such a pass).
    pub(crate) fn variant(&self, gpu: &Gpu, key: &PassKey) -> Option<vk::Pipeline> {
        if !self.realize(gpu) {
            return None;
        }
        if let Some(p) = self.0.obj.borrow().as_ref().and_then(|o| o.variants.iter().find(|v| v.0 == *key).map(|v| v.1)) {
            return Some(p);
        }
        let rp = gpu.render_pass(key, 0);
        let layout = gpu.with(|s| s.layout)?;
        let made = {
            let o = self.0.obj.borrow();
            let o = o.as_ref()?;
            make_variant(&o.sh, &self.0.desc, o, key, rp, layout)
        };
        match made {
            Some(p) => {
                self.0.obj.borrow_mut().as_mut()?.variants.push((*key, p));
                Some(p)
            }
            None => {
                vlog!(gpu, "gpu: Vulkan: the device refused a pipeline ({} colour targets, depth format {}, {} samples)", key.ncolors, key.depth.as_raw(), key.samples);
                None
            }
        }
    }
}

/// The vertex input: each input of the vertex shader from the layout's attribute of
/// its semantic (the layout's order when the shader has no names), or the placeholder.
fn vertex_attrs(d: &PipelineDesc, vr: &Reflect) -> (Vec<vk::VertexInputAttributeDescription>, bool) {
    let mut v = Vec::new();
    let mut dummy = false;
    for (k, (loc, sem)) in vr.inputs.iter().enumerate() {
        let a = match sem {
            Some(s) => d.layout.iter().find(|a| {
                let n = sem_name(a.semantic);
                a.index as u32 == s.index && (s.name == n || (s.name == "SV_POSITION" && n == "POSITION"))
            }),
            None => d.layout.get(k),
        };
        match a {
            Some(a) => v.push(vk::VertexInputAttributeDescription { location: *loc, binding: 0, format: attr_format(a.format), offset: a.offset as u32 }),
            None => {
                dummy = true;
                v.push(vk::VertexInputAttributeDescription { location: *loc, binding: 1, format: vk::Format::R32G32B32A32_SFLOAT, offset: 0 });
            }
        }
    }
    (v, dummy)
}

fn make_variant(sh: &Shared, d: &PipelineDesc, o: &PipeObj, key: &PassKey, rp: vk::RenderPass, layout: vk::PipelineLayout) -> Option<vk::Pipeline> {
    let stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(o.vs).name(c"main_vs_placeholder"),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(o.ps).name(c"main_ps_placeholder"),
    ];
    // The entry points are dxc's: the HLSL function's name, the first OpEntryPoint's.
    let vs_name = entry_name(d.vs.spirv)?;
    let ps_name = entry_name(d.ps.spirv)?;
    let stages = [stages[0].name(&vs_name), stages[1].name(&ps_name)];
    let mut binds = alloc::vec![vk::VertexInputBindingDescription { binding: 0, stride: d.stride, input_rate: vk::VertexInputRate::VERTEX }];
    if o.dummy {
        binds.push(vk::VertexInputBindingDescription { binding: 1, stride: 0, input_rate: vk::VertexInputRate::VERTEX });
    }
    let vi = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&binds).vertex_attribute_descriptions(&o.attrs);
    let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(topology(d.topology));
    let vp = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    let has_depth = key.depth != vk::Format::UNDEFINED;
    let bias = has_depth && d.depth.test && (d.depth.bias != 0.0 || d.depth.slope_bias != 0.0);
    let (cull, front) = match d.cull {
        Cull::None => (vk::CullModeFlags::NONE, vk::FrontFace::COUNTER_CLOCKWISE),
        Cull::Clockwise => (vk::CullModeFlags::BACK, vk::FrontFace::COUNTER_CLOCKWISE),
        Cull::CounterClockwise => (vk::CullModeFlags::FRONT, vk::FrontFace::COUNTER_CLOCKWISE),
    };
    let rs = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(cull)
        .front_face(front)
        .depth_bias_enable(bias)
        .depth_bias_constant_factor(if bias { d.depth.bias * bias_scale(key.depth) } else { 0.0 })
        .depth_bias_slope_factor(if bias { d.depth.slope_bias } else { 0.0 })
        .line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(samples_flag(key.samples));
    let ds = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(has_depth && d.depth.test)
        .depth_write_enable(has_depth && d.depth.test && d.depth.write)
        .depth_compare_op(if d.depth.test { compare(d.depth.compare) } else { vk::CompareOp::ALWAYS });
    let mask = vk::ColorComponentFlags::from_raw(d.write_mask as u32 & 0xf);
    let mut cba = Vec::new();
    for _ in 0..key.ncolors {
        let mut a = vk::PipelineColorBlendAttachmentState::default().color_write_mask(mask);
        if let Some(b) = d.blend {
            let al = b.alpha.unwrap_or(b.color);
            a = a
                .blend_enable(true)
                .src_color_blend_factor(factor(b.color.src))
                .dst_color_blend_factor(factor(b.color.dst))
                .color_blend_op(blend_op(b.color.op))
                .src_alpha_blend_factor(factor(al.src))
                .dst_alpha_blend_factor(factor(al.dst))
                .alpha_blend_op(blend_op(al.op));
        }
        cba.push(a);
    }
    let cb = vk::PipelineColorBlendStateCreateInfo::default().attachments(&cba);
    let dyns = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dy = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyns);
    let mut ci = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .color_blend_state(&cb)
        .dynamic_state(&dy)
        .layout(layout)
        .render_pass(rp)
        .subpass(0);
    if has_depth {
        ci = ci.depth_stencil_state(&ds);
    }
    let _ = is_depth_vk;
    let r = unsafe { sh.dev.create_graphics_pipelines(sh.cache, &[ci], None) };
    match r {
        Ok(v) => v.first().copied(),
        Err((v, _)) => {
            for p in v {
                if p != vk::Pipeline::null() {
                    unsafe { sh.dev.destroy_pipeline(p, None) };
                }
            }
            None
        }
    }
}

/// The name of a SPIR-V module's first entry point, NUL-terminated.
pub(crate) fn entry_name(w: &[u32]) -> Option<alloc::ffi::CString> {
    let mut i = 5;
    while i < w.len() {
        let n = (w[i] >> 16) as usize;
        if n == 0 {
            break;
        }
        if w[i] & 0xffff == 15 && n > 3 {
            let mut s = Vec::new();
            'o: for &x in &w[i + 3..i + n] {
                for k in 0..4 {
                    let b = (x >> (8 * k)) as u8;
                    if b == 0 {
                        break 'o;
                    }
                    s.push(b);
                }
            }
            return alloc::ffi::CString::new(s).ok();
        }
        i += n;
    }
    None
}
