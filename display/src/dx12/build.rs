//! What Display8 builds on Direct3D 12 that does not need its device: the present pass's
//! root signature and pipeline description, the passthrough's root signature and its
//! pipelines. They are here, taking only a device, so that `display/d12probe` builds
//! the very same ones from this file (`#[path]`) and Microsoft's runtime and debug layer
//! judge exactly what Display8 makes. Names only `raw.rs`, `passthrough::state` and the
//! DXIL; the caller says what to do with a message (`note`).
#![allow(dead_code)]

use crate::dx12::raw::*;
use crate::passthrough::state::{self, Fmt, Layout, PipeState};
use crate::shaders::ff_dxil::{PS_FF_DXIL, VS_BUMP_DXIL, VS_FF_DXIL};
use crate::shaders::present_dxil::{PS_GAMMA_DXIL, VS_PRESENT_DXIL};
use alloc::vec::Vec;
use core::ffi::c_void;

/// What failed, and its HRESULT (0 when it has none).
pub type Fail = (&'static str, i32);

/// A pipeline's key: the target, the vertices, and `state::PipeState`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub color: u32,
    pub depth: u32,
    pub samples: u32,
    pub stride: u32,
    pub layout: Layout,
    pub st: PipeState,
    /// The target's alpha is not kept (the engine's X8R8G8B8 back buffer).
    pub no_dst_alpha: bool,
}


/// Serializes `desc` (version 1.0, through `D3D12SerializeRootSignature` at `serialize`)
/// and makes the root signature on `dev`.
fn root(dev: P, serialize: usize, desc: &RootSignatureDesc, note: &dyn Fn(&str)) -> Result<P, Fail> {
    let serialize: unsafe extern "system" fn(*const RootSignatureDesc, u32, *mut P, *mut P) -> i32 = unsafe { core::mem::transmute(serialize) };
    let (mut blob, mut err): (P, P) = (core::ptr::null_mut(), core::ptr::null_mut());
    let hr = unsafe { serialize(desc, ROOT_SIGNATURE_VERSION_1_0, &mut blob, &mut err) };
    if hr < 0 || blob.is_null() {
        if !err.is_null() {
            let msg = vcall!(err, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const u8,);
            let n = vcall!(err, slot::BLOB_GET_BUFFER_SIZE, fn() -> usize,).min(1000);
            let text = unsafe { core::slice::from_raw_parts(msg, n) };
            note(core::str::from_utf8(text).unwrap_or("(not text)"));
        }
        release(err);
        return Err(("D3D12SerializeRootSignature failed", hr));
    }
    release(err);
    let (ptr, len) = (vcall!(blob, slot::BLOB_GET_BUFFER_POINTER, fn() -> *const c_void,), vcall!(blob, slot::BLOB_GET_BUFFER_SIZE, fn() -> usize,));
    let mut root: P = core::ptr::null_mut();
    let hr = vcall!(dev, slot::DEV_CREATE_ROOT_SIGNATURE, fn(u32, *const c_void, usize, *const crate::com::Guid, *mut P) -> i32, 0, ptr, len, &IID_ROOT_SIGNATURE, &mut root);
    release(blob);
    if hr < 0 || root.is_null() {
        return Err(("CreateRootSignature", hr));
    }
    Ok(root)
}

/// The present pass's: b0 the vertex shader's register file, four root constants (the
/// half-pixel shift, nought here); t0, t1 the frame and the ramp; s0, s1 point, clamped.
pub fn present_root(dev: P, serialize: usize, note: &dyn Fn(&str)) -> Result<P, Fail> {
    let range = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SRV, count: 2, base_register: 0, space: 0, offset_in_table: 0 }];
    let params = [
        RootParameter { ty: ROOT_PARAMETER_TYPE_32BIT_CONSTANTS, a: 0, b: 0, c: 4, visibility: SHADER_VISIBILITY_VERTEX },
        RootParameter { ty: ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, a: 1, b: range.as_ptr() as usize, c: 0, visibility: SHADER_VISIBILITY_PIXEL },
    ];
    let sampler = |register| StaticSampler {
        filter: FILTER_MIN_MAG_MIP_POINT,
        address_u: TEXTURE_ADDRESS_MODE_CLAMP,
        address_v: TEXTURE_ADDRESS_MODE_CLAMP,
        address_w: TEXTURE_ADDRESS_MODE_CLAMP,
        mip_lod_bias: 0.0,
        max_anisotropy: 1,
        comparison: COMPARISON_FUNC_ALWAYS,
        border: STATIC_BORDER_COLOR_OPAQUE_BLACK,
        min_lod: 0.0,
        max_lod: 0.0,
        register,
        space: 0,
        visibility: SHADER_VISIBILITY_PIXEL,
    };
    let samplers = [sampler(0), sampler(1)];
    let desc = RootSignatureDesc {
        count: params.len() as u32,
        params: params.as_ptr(),
        samplers_count: samplers.len() as u32,
        samplers: samplers.as_ptr(),
        flags: ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
    };
    root(dev, serialize, &desc, note)
}

/// The present pass's pipeline description over `root`, handed to `f` (the description
/// points at locals).
pub fn present_desc<R>(root: P, f: impl FnOnce(&GraphicsPipelineDesc) -> R) -> R {
    let inputs = [
        InputElement { semantic: c"POSITION".as_ptr() as *const u8, index: 0, format: FORMAT_R32G32_FLOAT, slot: 0, offset: 0, class: INPUT_CLASSIFICATION_PER_VERTEX_DATA, step_rate: 0 },
        InputElement { semantic: c"TEXCOORD".as_ptr() as *const u8, index: 0, format: FORMAT_R32G32_FLOAT, slot: 0, offset: 8, class: INPUT_CLASSIFICATION_PER_VERTEX_DATA, step_rate: 0 },
    ];
    let mut blend = BlendDesc::default();
    blend.targets[0] = RenderTargetBlend {
        src: BLEND_ONE,
        dst: BLEND_ZERO,
        op: BLEND_OP_ADD,
        src_alpha: BLEND_ONE,
        dst_alpha: BLEND_ZERO,
        op_alpha: BLEND_OP_ADD,
        logic_op: LOGIC_OP_NOOP,
        write_mask: COLOR_WRITE_ENABLE_ALL,
        ..Default::default()
    };
    let keep = StencilOpDesc { fail: STENCIL_OP_KEEP, depth_fail: STENCIL_OP_KEEP, pass: STENCIL_OP_KEEP, func: COMPARISON_FUNC_ALWAYS };
    let mut formats = [FORMAT_UNKNOWN; 8];
    formats[0] = FORMAT_B8G8R8A8_UNORM;
    let pd = GraphicsPipelineDesc {
        root,
        vs: ShaderBytecode::of(&VS_PRESENT_DXIL),
        ps: ShaderBytecode::of(&PS_GAMMA_DXIL),
        ds: ShaderBytecode::NONE,
        hs: ShaderBytecode::NONE,
        gs: ShaderBytecode::NONE,
        stream_output: StreamOutputDesc { decl: core::ptr::null(), entries: 0, strides: core::ptr::null(), nstrides: 0, rasterized_stream: 0 },
        blend,
        sample_mask: u32::MAX,
        rasterizer: RasterizerDesc { fill: FILL_MODE_SOLID, cull: CULL_MODE_NONE, depth_clip: 1, ..Default::default() },
        depth_stencil: DepthStencilDesc { depth: 0, write_mask: 0, func: COMPARISON_FUNC_ALWAYS, front: keep, back: keep, ..Default::default() },
        input_layout: InputLayoutDesc { elements: inputs.as_ptr(), count: inputs.len() as u32 },
        strip_cut: 0,
        topology: PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        num_targets: 1,
        formats,
        dsv_format: FORMAT_UNKNOWN,
        sample: SampleDesc { count: 1, quality: 0 },
        node_mask: 0,
        cached: CachedPso { blob: core::ptr::null(), len: 0 },
        flags: 0,
    };
    f(&pd)
}

/// The passthrough's: parameters 0 and 1 the vertex block's root CBVs (b0, spaces 0 and
/// 1: `vs_ff` and `vs_bump`), 2 the pixel block's (b1), 3 the table of t0 and t1, 4 that
/// of s0 and s1.
pub fn pass_root(dev: P, serialize: usize, note: &dyn Fn(&str)) -> Result<P, Fail> {
    let srv = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SRV, count: 2, base_register: 0, space: 0, offset_in_table: 0 }];
    let smp = [DescriptorRange { ty: DESCRIPTOR_RANGE_TYPE_SAMPLER, count: 2, base_register: 0, space: 0, offset_in_table: 0 }];
    let params = [
        RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 0, b: 0, c: 0, visibility: SHADER_VISIBILITY_VERTEX },
        RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 0, b: 1, c: 0, visibility: SHADER_VISIBILITY_VERTEX },
        RootParameter { ty: ROOT_PARAMETER_TYPE_CBV, a: 1, b: 0, c: 0, visibility: SHADER_VISIBILITY_PIXEL },
        RootParameter { ty: ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, a: 1, b: srv.as_ptr() as usize, c: 0, visibility: SHADER_VISIBILITY_PIXEL },
        RootParameter { ty: ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, a: 1, b: smp.as_ptr() as usize, c: 0, visibility: SHADER_VISIBILITY_PIXEL },
    ];
    let desc = RootSignatureDesc {
        count: params.len() as u32,
        params: params.as_ptr(),
        samplers_count: 0,
        samplers: core::ptr::null(),
        flags: ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
    };
    root(dev, serialize, &desc, note)
}

pub fn semantic(bump: bool, location: u32) -> (&'static core::ffi::CStr, u32) {
    if bump {
        match location {
            0 => (c"POSITION", 0),
            1 => (c"TEXCOORD", 0),
            2 => (c"TANGENT", 0),
            3 => (c"BINORMAL", 0),
            _ => (c"NORMAL", 0),
        }
    } else {
        match location {
            0 => (c"POSITION", 0),
            1 => (c"NORMAL", 0),
            2 => (c"COLOR", 0),
            3 => (c"COLOR", 1),
            4 => (c"TEXCOORD", 0),
            _ => (c"TEXCOORD", 1),
        }
    }
}

pub fn dxgi(f: Fmt) -> u32 {
    match f {
        Fmt::F1 => FORMAT_R32_FLOAT,
        Fmt::F2 => FORMAT_R32G32_FLOAT,
        Fmt::F3 => FORMAT_R32G32B32_FLOAT,
        Fmt::F4 => FORMAT_R32G32B32A32_FLOAT,
        Fmt::Bgra8 => FORMAT_B8G8R8A8_UNORM,
    }
}


/// The passthrough pipeline for `k` over `root`; the HRESULT when it cannot be made.
/// `D3D12_COMPARISON_FUNC` is Direct3D's own numbering.
pub fn pass_pipeline(dev: P, root: P, k: &Key) -> Result<P, i32> {
    let bump = k.layout.bump.is_some();
    let (attrs, n) = state::attributes(&k.layout, k.st.sets);
    let mut elements: Vec<InputElement> = Vec::with_capacity(n);
    for a in &attrs[..n] {
        let (name, index) = semantic(bump, a.location);
        elements.push(InputElement {
            semantic: name.as_ptr() as *const u8,
            index,
            format: dxgi(a.fmt),
            slot: a.binding,
            offset: a.offset,
            // The defaults are instance data, read once by the draw's one instance.
            class: if a.binding == 0 { INPUT_CLASSIFICATION_PER_VERTEX_DATA } else { INPUT_CLASSIFICATION_PER_INSTANCE_DATA },
            step_rate: if a.binding == 0 { 0 } else { 1 },
        });
    }
    let b = state::blend(k.st.blend, k.no_dst_alpha);
    let mut blend = BlendDesc::default();
    blend.targets[0] = RenderTargetBlend {
        blend: b.on as i32,
        logic_op_enable: 0,
        src: b.src,
        dst: b.dst,
        op: b.op,
        src_alpha: state::alpha_factor(b.src),
        dst_alpha: state::alpha_factor(b.dst),
        op_alpha: b.op,
        logic_op: LOGIC_OP_NOOP,
        write_mask: b.mask as u8,
    };
    // Direct3D's culling on screen: CW (2) drops what is clockwise, which with the
    // front face clockwise is the front.
    let cull = match k.st.cull {
        2 => 2,
        3 => 3,
        _ => CULL_MODE_NONE,
    };
    let has_depth = k.depth != FORMAT_UNKNOWN;
    let test = has_depth && k.st.depth[0] != 0;
    let keep = StencilOpDesc { fail: STENCIL_OP_KEEP, depth_fail: STENCIL_OP_KEEP, pass: STENCIL_OP_KEEP, func: COMPARISON_FUNC_ALWAYS };
    let mut formats = [FORMAT_UNKNOWN; 8];
    formats[0] = k.color;
    let ttype = match k.st.topology {
        1 => 1,
        2 | 3 => 2,
        _ => PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
    };
    let pd = GraphicsPipelineDesc {
        root,
        vs: ShaderBytecode::of(if bump { &VS_BUMP_DXIL } else { &VS_FF_DXIL }),
        ps: ShaderBytecode::of(&PS_FF_DXIL),
        ds: ShaderBytecode::NONE,
        hs: ShaderBytecode::NONE,
        gs: ShaderBytecode::NONE,
        stream_output: StreamOutputDesc { decl: core::ptr::null(), entries: 0, strides: core::ptr::null(), nstrides: 0, rasterized_stream: 0 },
        blend,
        sample_mask: u32::MAX,
        rasterizer: RasterizerDesc { fill: FILL_MODE_SOLID, cull, front_ccw: 0, depth_clip: 1, ..Default::default() },
        depth_stencil: DepthStencilDesc {
            depth: test as i32,
            write_mask: (test && k.st.depth[1] != 0) as u32,
            func: state::compare(k.st.depth[2]),
            stencil: 0,
            stencil_read_mask: 0xff,
            stencil_write_mask: 0xff,
            front: keep,
            back: keep,
        },
        input_layout: InputLayoutDesc { elements: elements.as_ptr(), count: elements.len() as u32 },
        strip_cut: 0,
        topology: ttype,
        num_targets: 1,
        formats,
        dsv_format: k.depth,
        sample: SampleDesc { count: k.samples.max(1), quality: 0 },
        node_mask: 0,
        cached: CachedPso { blob: core::ptr::null(), len: 0 },
        flags: 0,
    };
    let mut pso: P = core::ptr::null_mut();
    let hr = vcall!(dev, slot::DEV_CREATE_GRAPHICS_PIPELINE_STATE, fn(*const GraphicsPipelineDesc, *const crate::com::Guid, *mut P) -> i32, &pd, &IID_PIPELINE_STATE, &mut pso);
    if hr < 0 || pso.is_null() {
        return Err(if hr < 0 { hr } else { -1 });
    }
    Ok(pso)
}
