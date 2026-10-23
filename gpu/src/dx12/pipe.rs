//! Pipelines on Direct3D 12. A `PipelineDesc` names the states as Direct3D 9 defines
//! them; its pipeline state objects are made at first use, one for each kind of pass it
//! is drawn in (the formats and samples of the targets the pass really has, as on
//! Vulkan: render3d's descriptions name the scene's, the UI's the back buffer's).
//!
//! - **Shaders** are the DXIL beside the d3dbc (`Code::dxil`); what they read (the
//!   vertex inputs by semantic, the textures with their dimension, the bytes of each
//!   register file) is read out of the SPIR-V of the same source (`vulkan::spirv`), since
//!   the DXIL is built without reflection. With an alpha test, the pixel shader is its
//!   alpha-tested twin (`Code::dxil_at`), given the interval at b2.
//! - **Vertex input**: each input of the vertex shader, by semantic, from the layout's
//!   attribute of that semantic; one the layout lacks reads (0, 0, 0, 1) from a buffer
//!   of its own (slot 1, per instance), Direct3D 9's default.
//! - **Rasteriser**: Direct3D 12 winds and places triangles as Direct3D 9 does, so the
//!   cull is Direct3D 9's as it is. The **depth bias** is Direct3D 9's in depth units,
//!   scaled to the format's step, as on Vulkan.
use super::raw::*;
use super::{dlog, Gpu, PassKey, Shared, K_WORDS};
use crate::d3d9::raw::Guid;
use crate::types::*;
use crate::vulkan::spirv::{self, TexUse};
use alloc::ffi::CString;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

#[derive(Clone)]
pub struct Pipeline(pub(crate) Rc<PipeInner>);

pub(crate) struct PipeInner {
    pub desc: PipelineDesc,
    pub obj: RefCell<Option<PipeObj>>,
    failed: Cell<bool>,
}

struct Input {
    name: CString,
    index: u32,
    format: u32,
    slot: u32,
    offset: u32,
}

pub(crate) struct PipeObj {
    sh: Rc<Shared>,
    ps: &'static [u32],
    inputs: Vec<Input>,
    variants: Vec<(PassKey, P)>,
    /// The textures both stages read (by stage).
    pub textures: Vec<TexUse>,
    /// Bytes of each stage's register file the shaders read.
    pub vs_bytes: u32,
    pub ps_bytes: u32,
    pub needs_dummy_vb: bool,
    /// The root constants at b2: the alpha test's interval (lo, hi, invert).
    pub k: [f32; K_WORDS as usize],
    pub topology: u32,
}

impl Drop for PipeObj {
    fn drop(&mut self) {
        for &(_, p) in &self.variants {
            self.sh.retire(super::Garbage::Com(p));
        }
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

fn attr_format(f: AttrFormat) -> u32 {
    match f {
        AttrFormat::Float1 => FORMAT_R32_FLOAT,
        AttrFormat::Float2 => FORMAT_R32G32_FLOAT,
        AttrFormat::Float3 => FORMAT_R32G32B32_FLOAT,
        AttrFormat::Float4 => FORMAT_R32G32B32A32_FLOAT,
        AttrFormat::Bgra8 => FORMAT_B8G8R8A8_UNORM,
        AttrFormat::UByte4 => FORMAT_R8G8B8A8_UINT,
        AttrFormat::UByte4N => FORMAT_R8G8B8A8_UNORM,
        AttrFormat::Short2 => FORMAT_R16G16_SINT,
    }
}

/// `D3D12_BLEND`; for the alpha equation a colour factor is its alpha one (Direct3D 12
/// takes no colour factor there, and on alpha the two are the same).
fn factor(f: Factor, alpha: bool) -> u32 {
    match f {
        Factor::Zero => 1,
        Factor::One => 2,
        Factor::SrcColor => if alpha { 5 } else { 3 },
        Factor::InvSrcColor => if alpha { 6 } else { 4 },
        Factor::SrcAlpha => 5,
        Factor::InvSrcAlpha => 6,
        Factor::DstAlpha => 7,
        Factor::InvDstAlpha => 8,
        Factor::DstColor => if alpha { 7 } else { 9 },
        Factor::InvDstColor => if alpha { 8 } else { 10 },
        Factor::SrcAlphaSat => 11,
    }
}

fn blend_op(o: BlendOp) -> u32 {
    match o {
        BlendOp::Add => 1,
        BlendOp::Subtract => 2,
        BlendOp::RevSubtract => 3,
        BlendOp::Min => 4,
        BlendOp::Max => 5,
    }
}

pub(crate) fn compare(c: Compare) -> u32 {
    c as u32 + 1
}

/// The topology's type (for the pipeline) and the topology (for the list).
fn topology(t: Topology) -> (u32, u32) {
    match t {
        Topology::PointList => (PRIMITIVE_TOPOLOGY_TYPE_POINT, PRIMITIVE_TOPOLOGY_POINTLIST),
        Topology::LineList => (PRIMITIVE_TOPOLOGY_TYPE_LINE, PRIMITIVE_TOPOLOGY_LINELIST),
        Topology::LineStrip => (PRIMITIVE_TOPOLOGY_TYPE_LINE, PRIMITIVE_TOPOLOGY_LINESTRIP),
        Topology::TriangleList => (PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, PRIMITIVE_TOPOLOGY_TRIANGLELIST),
        Topology::TriangleStrip => (PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, PRIMITIVE_TOPOLOGY_TRIANGLESTRIP),
    }
}

/// Direct3D 9's depth bias unit in Direct3D 12's for the depth view format `f`.
fn bias_scale(f: u32) -> f32 {
    match f {
        FORMAT_D16_UNORM => 65536.0,
        FORMAT_D24_UNORM_S8_UINT => 16_777_216.0,
        _ => 8_388_608.0,
    }
}

impl Pipeline {
    pub(crate) fn new(desc: PipelineDesc) -> Pipeline {
        Pipeline(Rc::new(PipeInner { desc, obj: RefCell::new(None), failed: Cell::new(false) }))
    }

    pub fn desc(&self) -> &PipelineDesc {
        &self.0.desc
    }

    /// Its shaders read: false when there is no DXIL, or no SPIR-V to read them from.
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
        if d.vs.dxil.is_empty() || d.ps.dxil.is_empty() {
            dlog!(gpu, "gpu: Direct3D 12: a pipeline without DXIL; what draws with it stands down");
            return None;
        }
        let (Some(vr), Some(pr)) = (spirv::reflect(d.vs.spirv), spirv::reflect(d.ps.spirv)) else {
            dlog!(gpu, "gpu: Direct3D 12: a pipeline without the SPIR-V its shaders are read from");
            return None;
        };
        let mut k = [0f32; K_WORDS as usize];
        let mut ps = d.ps.dxil;
        if let Some(a) = d.alpha_test {
            if let Some((lo, hi, inv)) = spirv::alpha_interval(a.compare as u32, a.reference) {
                if d.ps.dxil_at.is_empty() {
                    gpu.once(1 << 24, format_args!("gpu: Direct3D 12: a pixel shader drawn with an alpha test has no alpha-tested twin (:at); drawn without it"));
                } else {
                    ps = d.ps.dxil_at;
                    k[..3].copy_from_slice(&[lo, hi, if inv { 1.0 } else { 0.0 }]);
                }
            }
        }
        // The vertex input: every input the DXIL's signature declares (as SPIR-V names
        // them, `in.var.TEXCOORD3`, when it has none), from the layout, or the placeholder.
        let sems: Vec<(u32, Option<spirv::Sem>)> = match dxil_inputs(d.vs.dxil) {
            Some(v) => v.into_iter().map(|s| (0, Some(s))).collect(),
            None => vr.inputs.clone(),
        };
        let mut inputs = Vec::new();
        let mut dummy = false;
        for (n, (_, sem)) in sems.iter().enumerate() {
            let (name, index, a) = match sem {
                Some(s) => {
                    let a = d.layout.iter().find(|a| {
                        let n = sem_name(a.semantic);
                        a.index as u32 == s.index && (s.name == n || (s.name == "SV_POSITION" && n == "POSITION"))
                    });
                    (s.name.clone(), s.index, a)
                }
                None => match d.layout.get(n) {
                    Some(a) => (alloc::string::String::from(sem_name(a.semantic)), a.index as u32, Some(a)),
                    None => continue,
                },
            };
            let name = CString::new(name).ok()?;
            match a {
                Some(a) => inputs.push(Input { name, index, format: attr_format(a.format), slot: 0, offset: a.offset as u32 }),
                None => {
                    dummy = true;
                    inputs.push(Input { name, index, format: FORMAT_R32G32B32A32_FLOAT, slot: 1, offset: 0 });
                }
            }
        }
        let mut textures: Vec<TexUse> = Vec::new();
        for t in vr.textures.iter().chain(pr.textures.iter()) {
            if !textures.iter().any(|x| x.binding == t.binding) && (t.binding as usize) < super::SLOTS {
                textures.push(*t);
            }
        }
        Some(PipeObj {
            sh,
            ps,
            inputs,
            variants: Vec::new(),
            textures,
            vs_bytes: vr.ubo_bytes.div_ceil(16) * 16,
            ps_bytes: pr.ubo_bytes.div_ceil(16) * 16,
            needs_dummy_vb: dummy,
            k,
            topology: topology(d.topology).1,
        })
    }

    /// Its pipeline state for a pass of `key` (made at the first draw in such a pass).
    pub(crate) fn variant(&self, gpu: &Gpu, key: &PassKey) -> Option<P> {
        if !self.realize(gpu) {
            return None;
        }
        if let Some(p) = self.0.obj.borrow().as_ref().and_then(|o| o.variants.iter().find(|v| v.0 == *key).map(|v| v.1)) {
            return Some(p);
        }
        let made = {
            let o = self.0.obj.borrow();
            let o = o.as_ref()?;
            make_pso(&o.sh, &self.0.desc, o, key)
        };
        match made {
            Some(p) => {
                self.0.obj.borrow_mut().as_mut()?.variants.push((*key, p));
                Some(p)
            }
            None => {
                dlog!(gpu, "gpu: Direct3D 12: the device refused a pipeline ({} colour targets, depth format {}, {} samples)", key.ncolors, key.depth, key.samples);
                None
            }
        }
    }
}

fn make_pso(sh: &Shared, d: &PipelineDesc, o: &PipeObj, key: &PassKey) -> Option<P> {
    let elems: Vec<InputElement> = o
        .inputs
        .iter()
        .map(|i| InputElement {
            semantic: i.name.as_ptr() as *const u8,
            index: i.index,
            format: i.format,
            slot: i.slot,
            offset: i.offset,
            class: if i.slot == 1 { INPUT_CLASSIFICATION_PER_INSTANCE_DATA } else { INPUT_CLASSIFICATION_PER_VERTEX_DATA },
            step_rate: if i.slot == 1 { 1 } else { 0 },
        })
        .collect();
    let has_depth = key.depth != FORMAT_UNKNOWN;
    let bias = has_depth && d.depth.test && (d.depth.bias != 0.0 || d.depth.slope_bias != 0.0);
    let mut blend = BlendDesc::default();
    let mask = d.write_mask & 0xf;
    for t in blend.targets.iter_mut().take(key.ncolors as usize) {
        *t = RenderTargetBlend { logic_op: LOGIC_OP_NOOP, write_mask: mask, src: 2, dst: 1, op: 1, src_alpha: 2, dst_alpha: 1, op_alpha: 1, ..Default::default() };
        if let Some(b) = d.blend {
            let al = b.alpha.unwrap_or(b.color);
            t.blend = 1;
            t.src = factor(b.color.src, false);
            t.dst = factor(b.color.dst, false);
            t.op = blend_op(b.color.op);
            t.src_alpha = factor(al.src, true);
            t.dst_alpha = factor(al.dst, true);
            t.op_alpha = blend_op(al.op);
        }
    }
    let cull = match d.cull {
        Cull::None => CULL_MODE_NONE,
        // Direct3D 12's front face is clockwise unless told otherwise, as Direct3D 9's.
        Cull::Clockwise => CULL_MODE_FRONT,
        Cull::CounterClockwise => CULL_MODE_BACK,
    };
    let keep = StencilOpDesc { fail: STENCIL_OP_KEEP, depth_fail: STENCIL_OP_KEEP, pass: STENCIL_OP_KEEP, func: COMPARISON_FUNC_ALWAYS };
    let mut formats = [FORMAT_UNKNOWN; 8];
    formats[..key.ncolors as usize].copy_from_slice(&key.colors[..key.ncolors as usize]);
    let pd = GraphicsPipelineDesc {
        root: sh.root,
        vs: ShaderBytecode::of(d.vs.dxil),
        ps: ShaderBytecode::of(o.ps),
        ds: ShaderBytecode::NONE,
        hs: ShaderBytecode::NONE,
        gs: ShaderBytecode::NONE,
        stream_output: StreamOutputDesc { decl: core::ptr::null(), entries: 0, strides: core::ptr::null(), nstrides: 0, rasterized_stream: 0 },
        blend,
        sample_mask: u32::MAX,
        rasterizer: RasterizerDesc {
            fill: FILL_MODE_SOLID,
            cull,
            front_ccw: 0,
            depth_bias: if bias { { let v = (d.depth.bias * bias_scale(key.depth)).clamp(-2.0e9, 2.0e9); (v + 0.5f32.copysign(v)) as i32 } } else { 0 },
            depth_bias_clamp: 0.0,
            slope_scaled_depth_bias: if bias { d.depth.slope_bias } else { 0.0 },
            depth_clip: 1,
            multisample: (key.samples > 1) as i32,
            antialiased_line: 0,
            forced_sample_count: 0,
            conservative: 0,
        },
        depth_stencil: DepthStencilDesc {
            depth: (has_depth && d.depth.test) as i32,
            write_mask: if has_depth && d.depth.test && d.depth.write { DEPTH_WRITE_MASK_ALL } else { DEPTH_WRITE_MASK_ZERO },
            func: if d.depth.test { compare(d.depth.compare) } else { COMPARISON_FUNC_ALWAYS },
            stencil: 0,
            stencil_read_mask: 0xff,
            stencil_write_mask: 0xff,
            front: keep,
            back: keep,
        },
        input_layout: InputLayoutDesc { elements: elems.as_ptr(), count: elems.len() as u32 },
        strip_cut: 0,
        topology: topology(d.topology).0,
        num_targets: key.ncolors,
        formats,
        dsv_format: key.depth,
        sample: SampleDesc { count: key.samples.max(1), quality: 0 },
        node_mask: 0,
        cached: CachedPso { blob: core::ptr::null(), len: 0 },
        flags: 0,
    };
    create(sh, &pd)
}

/// The inputs of a DXIL container's input signature (`ISG1`) that are not system
/// values, as semantics (upper case). None: no such part.
fn dxil_inputs(w: &[u32]) -> Option<Vec<spirv::Sem>> {
    let b = unsafe { core::slice::from_raw_parts(w.as_ptr() as *const u8, w.len() * 4) };
    let u32_at = |o: usize| -> Option<u32> { Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?)) };
    if b.get(..4)? != b"DXBC" {
        return None;
    }
    let parts = u32_at(28)? as usize;
    for i in 0..parts {
        let off = u32_at(32 + 4 * i)? as usize;
        if b.get(off..off + 4)? != b"ISG1" {
            continue;
        }
        let p = off + 8;
        let count = u32_at(p)? as usize;
        let first = u32_at(p + 4)? as usize;
        let mut out = Vec::new();
        for e in 0..count {
            let at = p + first + e * 32;
            let (name_off, index, sv) = (u32_at(at + 4)? as usize, u32_at(at + 8)?, u32_at(at + 12)?);
            if sv != 0 {
                continue;
            }
            let name = b.get(p + name_off..)?;
            let end = name.iter().position(|&c| c == 0)?;
            let mut s = alloc::string::String::new();
            for &c in &name[..end] {
                s.push(c.to_ascii_uppercase() as char);
            }
            out.push(spirv::Sem { name: s, index });
        }
        return Some(out);
    }
    None
}

pub(crate) fn create(sh: &Shared, pd: &GraphicsPipelineDesc) -> Option<P> {
    let mut pso: P = core::ptr::null_mut();
    let hr = vcall!(sh.dev, slot::DEV_CREATE_GRAPHICS_PIPELINE_STATE, fn(*const GraphicsPipelineDesc, *const Guid, *mut P) -> i32, pd, &IID_PIPELINE_STATE, &mut pso);
    (hr >= 0 && !pso.is_null()).then_some(pso)
}
