//! Direct3D 12 as the backend calls it, declared by hand from Wine's `d3d12.idl` (as
//! Display8's `display/src/dx12/raw.rs`, which this extends): the interface IDs, the
//! vtable slots (`crate::vcall!`, `this` first, `__stdcall`), and the structures with
//! their 32-bit layouts, each checked against its size below. A 64-bit handle or
//! address passed by value (`D3D12_GPU_DESCRIPTOR_HANDLE`, `D3D12_GPU_VIRTUAL_ADDRESS`)
//! is a `u64` argument; a CPU descriptor handle a `usize`.
#![allow(dead_code, clippy::upper_case_acronyms)]

use crate::d3d9::raw::Guid;
use core::ffi::c_void;

pub type P = *mut c_void;

pub const IID_RESOURCE: Guid = Guid(0x696442be, 0xa72e, 0x4059, [0xbc, 0x79, 0x5b, 0x5c, 0x98, 0x04, 0x0f, 0xad]);
pub const IID_ROOT_SIGNATURE: Guid = Guid(0xc54a6b66, 0x72df, 0x4ee8, [0x8b, 0xe5, 0xa9, 0x46, 0xa1, 0x42, 0x92, 0x14]);
pub const IID_PIPELINE_STATE: Guid = Guid(0x765a30f3, 0xf624, 0x4c6f, [0xa8, 0x28, 0xac, 0xe9, 0x48, 0x62, 0x24, 0x45]);
pub const IID_DESCRIPTOR_HEAP: Guid = Guid(0x8efb471d, 0x616c, 0x4f49, [0x90, 0xf7, 0x12, 0x7b, 0xb7, 0x63, 0xfa, 0x51]);
pub const IID_QUERY_HEAP: Guid = Guid(0x0d9658ae, 0xed45, 0x469e, [0xa6, 0x1d, 0x97, 0x0e, 0xc5, 0x83, 0xca, 0xb4]);
pub const IID_GRAPHICS_COMMAND_LIST1: Guid = Guid(0x553103fb, 0x1fe7, 0x4557, [0xbb, 0x38, 0x94, 0x6d, 0x7d, 0x0e, 0x7c, 0xa7]);

pub mod slot {
    pub const QUERY_INTERFACE: usize = 0;
    pub const RELEASE: usize = 2;
    // ID3D12Device
    pub const DEV_CREATE_GRAPHICS_PIPELINE_STATE: usize = 10;
    pub const DEV_CREATE_DESCRIPTOR_HEAP: usize = 14;
    pub const DEV_CREATE_ROOT_SIGNATURE: usize = 16;
    pub const DEV_CREATE_SHADER_RESOURCE_VIEW: usize = 18;
    pub const DEV_CREATE_RENDER_TARGET_VIEW: usize = 20;
    pub const DEV_CREATE_DEPTH_STENCIL_VIEW: usize = 21;
    pub const DEV_CREATE_SAMPLER: usize = 22;
    pub const DEV_COPY_DESCRIPTORS_SIMPLE: usize = 24;
    pub const DEV_CREATE_COMMITTED_RESOURCE: usize = 27;
    pub const DEV_CREATE_QUERY_HEAP: usize = 39;
    // ID3D12CommandQueue
    pub const QUEUE_GET_TIMESTAMP_FREQUENCY: usize = 16;
    // ID3D12GraphicsCommandList
    pub const LIST_DRAW_INSTANCED: usize = 12;
    pub const LIST_DRAW_INDEXED_INSTANCED: usize = 13;
    pub const LIST_COPY_BUFFER_REGION: usize = 15;
    pub const LIST_COPY_TEXTURE_REGION: usize = 16;
    pub const LIST_RESOLVE_SUBRESOURCE: usize = 19;
    pub const LIST_IA_SET_PRIMITIVE_TOPOLOGY: usize = 20;
    pub const LIST_RS_SET_VIEWPORTS: usize = 21;
    pub const LIST_RS_SET_SCISSOR_RECTS: usize = 22;
    pub const LIST_SET_PIPELINE_STATE: usize = 25;
    pub const LIST_RESOURCE_BARRIER: usize = 26;
    pub const LIST_SET_GRAPHICS_ROOT_SIGNATURE: usize = 30;
    pub const LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE: usize = 32;
    pub const LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS: usize = 36;
    pub const LIST_SET_GRAPHICS_ROOT_CONSTANT_BUFFER_VIEW: usize = 38;
    pub const LIST_IA_SET_INDEX_BUFFER: usize = 43;
    pub const LIST_IA_SET_VERTEX_BUFFERS: usize = 44;
    pub const LIST_OM_SET_RENDER_TARGETS: usize = 46;
    pub const LIST_CLEAR_DEPTH_STENCIL_VIEW: usize = 47;
    pub const LIST_CLEAR_RENDER_TARGET_VIEW: usize = 48;
    pub const LIST_END_QUERY: usize = 53;
    pub const LIST_RESOLVE_QUERY_DATA: usize = 54;
    // ID3D12GraphicsCommandList1
    pub const LIST1_RESOLVE_SUBRESOURCE_REGION: usize = 64;
    // ID3D12DescriptorHeap
    pub const HEAP_GET_CPU_START: usize = 9;
    // ID3D12Resource
    pub const RES_MAP: usize = 8;
    pub const RES_UNMAP: usize = 9;
    pub const RES_GET_GPU_VIRTUAL_ADDRESS: usize = 11;
    // ID3D10Blob
    pub const BLOB_GET_BUFFER_POINTER: usize = 3;
    pub const BLOB_GET_BUFFER_SIZE: usize = 4;
}

pub const DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV: u32 = 0;
pub const DESCRIPTOR_HEAP_TYPE_SAMPLER: u32 = 1;
pub const DESCRIPTOR_HEAP_TYPE_RTV: u32 = 2;
pub const DESCRIPTOR_HEAP_TYPE_DSV: u32 = 3;

pub const HEAP_TYPE_DEFAULT: u32 = 1;
pub const HEAP_TYPE_UPLOAD: u32 = 2;
pub const HEAP_TYPE_READBACK: u32 = 3;

pub const RESOURCE_DIMENSION_BUFFER: u32 = 1;
pub const RESOURCE_DIMENSION_TEXTURE2D: u32 = 3;
pub const RESOURCE_DIMENSION_TEXTURE3D: u32 = 4;
pub const TEXTURE_LAYOUT_UNKNOWN: u32 = 0;
pub const TEXTURE_LAYOUT_ROW_MAJOR: u32 = 1;
pub const RESOURCE_FLAG_ALLOW_RENDER_TARGET: u32 = 1;
pub const RESOURCE_FLAG_ALLOW_DEPTH_STENCIL: u32 = 2;

pub const STATE_COMMON: u32 = 0;
pub const STATE_VERTEX_AND_INDEX: u32 = 0x1 | 0x2;
pub const STATE_RENDER_TARGET: u32 = 0x4;
pub const STATE_DEPTH_WRITE: u32 = 0x10;
pub const STATE_SHADER_RESOURCE: u32 = 0x40 | 0x80;
pub const STATE_COPY_DEST: u32 = 0x400;
pub const STATE_COPY_SOURCE: u32 = 0x800;
pub const STATE_RESOLVE_DEST: u32 = 0x1000;
pub const STATE_RESOLVE_SOURCE: u32 = 0x2000;
pub const STATE_GENERIC_READ: u32 = 0xac3;
pub const ALL_SUBRESOURCES: u32 = 0xffff_ffff;

pub const BARRIER_TYPE_TRANSITION: u32 = 0;
pub const TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX: u32 = 0;
pub const TEXTURE_COPY_TYPE_PLACED_FOOTPRINT: u32 = 1;
pub const TEXTURE_DATA_PITCH_ALIGNMENT: u32 = 256;
pub const TEXTURE_DATA_PLACEMENT_ALIGNMENT: u32 = 512;
pub const CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT: u32 = 256;

pub const CLEAR_FLAG_DEPTH: u32 = 1;

// DXGI_FORMAT
pub const FORMAT_UNKNOWN: u32 = 0;
pub const FORMAT_R32G32B32A32_FLOAT: u32 = 2;
pub const FORMAT_R32G32B32_FLOAT: u32 = 6;
pub const FORMAT_R16G16B16A16_FLOAT: u32 = 10;
pub const FORMAT_R32G32_FLOAT: u32 = 16;
pub const FORMAT_R10G10B10A2_UNORM: u32 = 24;
pub const FORMAT_R8G8B8A8_UNORM: u32 = 28;
pub const FORMAT_R8G8B8A8_UINT: u32 = 30;
pub const FORMAT_R16G16_FLOAT: u32 = 34;
pub const FORMAT_R16G16_SINT: u32 = 38;
pub const FORMAT_R32_TYPELESS: u32 = 39;
pub const FORMAT_D32_FLOAT: u32 = 40;
pub const FORMAT_R32_FLOAT: u32 = 41;
pub const FORMAT_R32_UINT: u32 = 42;
pub const FORMAT_R24G8_TYPELESS: u32 = 44;
pub const FORMAT_D24_UNORM_S8_UINT: u32 = 45;
pub const FORMAT_R24_UNORM_X8_TYPELESS: u32 = 46;
pub const FORMAT_R16_TYPELESS: u32 = 53;
pub const FORMAT_R16_FLOAT: u32 = 54;
pub const FORMAT_D16_UNORM: u32 = 55;
pub const FORMAT_R16_UNORM: u32 = 56;
pub const FORMAT_R16_UINT: u32 = 57;
pub const FORMAT_R8_UNORM: u32 = 61;
pub const FORMAT_A8_UNORM: u32 = 65;
pub const FORMAT_B5G6R5_UNORM: u32 = 85;
pub const FORMAT_B8G8R8A8_UNORM: u32 = 87;

pub const SRV_DIMENSION_TEXTURE2D: u32 = 4;
pub const SRV_DIMENSION_TEXTURE2DARRAY: u32 = 5;
pub const SRV_DIMENSION_TEXTURE2DMS: u32 = 6;
pub const SRV_DIMENSION_TEXTURE3D: u32 = 8;
pub const SRV_DIMENSION_TEXTURECUBE: u32 = 9;
pub const RTV_DIMENSION_TEXTURE2D: u32 = 4;
pub const RTV_DIMENSION_TEXTURE2DARRAY: u32 = 5;
pub const RTV_DIMENSION_TEXTURE2DMS: u32 = 6;
pub const DSV_DIMENSION_TEXTURE2D: u32 = 3;
pub const DSV_DIMENSION_TEXTURE2DMS: u32 = 5;

/// `D3D12_ENCODE_SHADER_4_COMPONENT_MAPPING`: 0..3 a channel, 4 zero, 5 one.
pub const fn mapping(r: u32, g: u32, b: u32, a: u32) -> u32 {
    r | (g << 3) | (b << 6) | (a << 9) | (1 << 12)
}
pub const DEFAULT_COMPONENT_MAPPING: u32 = mapping(0, 1, 2, 3);

pub const ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE: u32 = 0;
pub const ROOT_PARAMETER_TYPE_32BIT_CONSTANTS: u32 = 1;
pub const ROOT_PARAMETER_TYPE_CBV: u32 = 2;
pub const DESCRIPTOR_RANGE_TYPE_SRV: u32 = 0;
pub const DESCRIPTOR_RANGE_TYPE_SAMPLER: u32 = 3;
pub const SHADER_VISIBILITY_ALL: u32 = 0;
pub const SHADER_VISIBILITY_VERTEX: u32 = 1;
pub const SHADER_VISIBILITY_PIXEL: u32 = 5;
pub const ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT: u32 = 1;
pub const ROOT_SIGNATURE_VERSION_1_0: u32 = 1;

// D3D12_FILTER: min, mag, mip as bits 4, 2, 0 (1 = linear); 0x55 anisotropic.
pub const FILTER_ANISOTROPIC: u32 = 0x55;
pub const TEXTURE_ADDRESS_MODE_WRAP: u32 = 1;
pub const TEXTURE_ADDRESS_MODE_MIRROR: u32 = 2;
pub const TEXTURE_ADDRESS_MODE_CLAMP: u32 = 3;
pub const COMPARISON_FUNC_NEVER: u32 = 1;
pub const COMPARISON_FUNC_ALWAYS: u32 = 8;
pub const FLOAT32_MAX: f32 = 3.402_823_5e38;

pub const PRIMITIVE_TOPOLOGY_TYPE_POINT: u32 = 1;
pub const PRIMITIVE_TOPOLOGY_TYPE_LINE: u32 = 2;
pub const PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE: u32 = 3;
pub const PRIMITIVE_TOPOLOGY_POINTLIST: u32 = 1;
pub const PRIMITIVE_TOPOLOGY_LINELIST: u32 = 2;
pub const PRIMITIVE_TOPOLOGY_LINESTRIP: u32 = 3;
pub const PRIMITIVE_TOPOLOGY_TRIANGLELIST: u32 = 4;
pub const PRIMITIVE_TOPOLOGY_TRIANGLESTRIP: u32 = 5;
pub const INPUT_CLASSIFICATION_PER_VERTEX_DATA: u32 = 0;
pub const INPUT_CLASSIFICATION_PER_INSTANCE_DATA: u32 = 1;
pub const FILL_MODE_SOLID: u32 = 3;
pub const CULL_MODE_NONE: u32 = 1;
pub const CULL_MODE_FRONT: u32 = 2;
pub const CULL_MODE_BACK: u32 = 3;
pub const LOGIC_OP_NOOP: u32 = 4;
pub const STENCIL_OP_KEEP: u32 = 1;
pub const DEPTH_WRITE_MASK_ZERO: u32 = 0;
pub const DEPTH_WRITE_MASK_ALL: u32 = 1;

pub const QUERY_HEAP_TYPE_TIMESTAMP: u32 = 1;
pub const QUERY_TYPE_TIMESTAMP: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DescriptorHeapDesc {
    pub ty: u32,
    pub count: u32,
    pub flags: u32,
    pub node_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct HeapProperties {
    pub ty: u32,
    pub cpu_page_property: u32,
    pub memory_pool_preference: u32,
    pub creation_node_mask: u32,
    pub visible_node_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SampleDesc {
    pub count: u32,
    pub quality: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ResourceDesc {
    pub dimension: u32,
    pub alignment: u64,
    pub width: u64,
    pub height: u32,
    pub depth_or_array_size: u16,
    pub mip_levels: u16,
    pub format: u32,
    pub sample: SampleDesc,
    pub layout: u32,
    pub flags: u32,
}

/// `D3D12_CLEAR_VALUE`: the colour, or the depth in `color[0]`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ClearValue {
    pub format: u32,
    pub color: [f32; 4],
}

/// `D3D12_RESOURCE_BARRIER` of the transition kind.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Barrier {
    pub ty: u32,
    pub flags: u32,
    pub resource: P,
    pub subresource: u32,
    pub before: u32,
    pub after: u32,
}

/// `D3D12_TEXTURE_COPY_LOCATION`: `offset` is the footprint's offset, or the
/// subresource index in its low word.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CopyLocation {
    pub resource: P,
    pub ty: u32,
    pub offset: u64,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub row_pitch: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Box3 {
    pub left: u32,
    pub top: u32,
    pub front: u32,
    pub right: u32,
    pub bottom: u32,
    pub back: u32,
}

/// `D3D12_SHADER_RESOURCE_VIEW_DESC`: the union (8-aligned) as six words, in the order
/// the dimension's member has them.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SrvDesc {
    pub format: u32,
    pub dimension: u32,
    pub mapping: u32,
    pub _pad: u32,
    pub u: [u32; 6],
}

/// `D3D12_RENDER_TARGET_VIEW_DESC` (the union is 16 bytes, 8-aligned).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RtvDesc {
    pub format: u32,
    pub dimension: u32,
    pub u: [u32; 4],
}

/// `D3D12_DEPTH_STENCIL_VIEW_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DsvDesc {
    pub format: u32,
    pub dimension: u32,
    pub flags: u32,
    pub u: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SamplerDesc {
    pub filter: u32,
    pub address_u: u32,
    pub address_v: u32,
    pub address_w: u32,
    pub mip_lod_bias: f32,
    pub max_anisotropy: u32,
    pub comparison: u32,
    pub border: [f32; 4],
    pub min_lod: f32,
    pub max_lod: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct D3dViewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct D3dRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct VertexBufferView {
    pub location: u64,
    pub size: u32,
    pub stride: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexBufferView {
    pub location: u64,
    pub size: u32,
    pub format: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DescriptorRange {
    pub ty: u32,
    pub count: u32,
    pub base_register: u32,
    pub space: u32,
    pub offset_in_table: u32,
}

/// `D3D12_ROOT_PARAMETER`: `a`, `b`, `c` are the union (a table: count, ranges; root
/// constants: register, space, count; a root descriptor: register, space).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RootParameter {
    pub ty: u32,
    pub a: u32,
    pub b: usize,
    pub c: u32,
    pub visibility: u32,
}

#[repr(C)]
pub struct RootSignatureDesc {
    pub count: u32,
    pub params: *const RootParameter,
    pub samplers_count: u32,
    pub samplers: *const c_void,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ShaderBytecode {
    pub code: *const c_void,
    pub len: usize,
}

impl ShaderBytecode {
    pub const NONE: ShaderBytecode = ShaderBytecode { code: core::ptr::null(), len: 0 };
    pub fn of(words: &[u32]) -> ShaderBytecode {
        ShaderBytecode { code: words.as_ptr() as *const c_void, len: words.len() * 4 }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StreamOutputDesc {
    pub decl: *const c_void,
    pub entries: u32,
    pub strides: *const u32,
    pub nstrides: u32,
    pub rasterized_stream: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RenderTargetBlend {
    pub blend: i32,
    pub logic_op_enable: i32,
    pub src: u32,
    pub dst: u32,
    pub op: u32,
    pub src_alpha: u32,
    pub dst_alpha: u32,
    pub op_alpha: u32,
    pub logic_op: u32,
    pub write_mask: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BlendDesc {
    pub alpha_to_coverage: i32,
    pub independent: i32,
    pub targets: [RenderTargetBlend; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RasterizerDesc {
    pub fill: u32,
    pub cull: u32,
    pub front_ccw: i32,
    pub depth_bias: i32,
    pub depth_bias_clamp: f32,
    pub slope_scaled_depth_bias: f32,
    pub depth_clip: i32,
    pub multisample: i32,
    pub antialiased_line: i32,
    pub forced_sample_count: u32,
    pub conservative: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StencilOpDesc {
    pub fail: u32,
    pub depth_fail: u32,
    pub pass: u32,
    pub func: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DepthStencilDesc {
    pub depth: i32,
    pub write_mask: u32,
    pub func: u32,
    pub stencil: i32,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front: StencilOpDesc,
    pub back: StencilOpDesc,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InputElement {
    pub semantic: *const u8,
    pub index: u32,
    pub format: u32,
    pub slot: u32,
    pub offset: u32,
    pub class: u32,
    pub step_rate: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InputLayoutDesc {
    pub elements: *const InputElement,
    pub count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CachedPso {
    pub blob: *const c_void,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GraphicsPipelineDesc {
    pub root: P,
    pub vs: ShaderBytecode,
    pub ps: ShaderBytecode,
    pub ds: ShaderBytecode,
    pub hs: ShaderBytecode,
    pub gs: ShaderBytecode,
    pub stream_output: StreamOutputDesc,
    pub blend: BlendDesc,
    pub sample_mask: u32,
    pub rasterizer: RasterizerDesc,
    pub depth_stencil: DepthStencilDesc,
    pub input_layout: InputLayoutDesc,
    pub strip_cut: u32,
    pub topology: u32,
    pub num_targets: u32,
    pub formats: [u32; 8],
    pub dsv_format: u32,
    pub sample: SampleDesc,
    pub node_mask: u32,
    pub cached: CachedPso,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct QueryHeapDesc {
    pub ty: u32,
    pub count: u32,
    pub node_mask: u32,
}

// The layouts, against the sizes Microsoft's compiler gives them on x86.
const _: () = {
    use core::mem::size_of;
    assert!(size_of::<DescriptorHeapDesc>() == 16);
    assert!(size_of::<HeapProperties>() == 20);
    assert!(size_of::<ResourceDesc>() == 56);
    assert!(size_of::<ClearValue>() == 20);
    assert!(size_of::<Barrier>() == 24);
    assert!(size_of::<CopyLocation>() == 40);
    assert!(size_of::<Box3>() == 24);
    assert!(size_of::<SrvDesc>() == 40);
    assert!(size_of::<RtvDesc>() == 24);
    assert!(size_of::<DsvDesc>() == 24);
    assert!(size_of::<SamplerDesc>() == 52);
    assert!(size_of::<D3dViewport>() == 24);
    assert!(size_of::<VertexBufferView>() == 16);
    assert!(size_of::<IndexBufferView>() == 16);
    assert!(size_of::<DescriptorRange>() == 20);
    assert!(size_of::<RootParameter>() == 20);
    assert!(size_of::<RootSignatureDesc>() == 20);
    assert!(size_of::<RenderTargetBlend>() == 40);
    assert!(size_of::<BlendDesc>() == 328);
    assert!(size_of::<RasterizerDesc>() == 44);
    assert!(size_of::<DepthStencilDesc>() == 52);
    assert!(size_of::<InputElement>() == 28);
    assert!(size_of::<GraphicsPipelineDesc>() == 572);
    assert!(size_of::<QueryHeapDesc>() == 12);
    assert!(core::mem::offset_of!(ResourceDesc, alignment) == 8);
    assert!(core::mem::offset_of!(CopyLocation, offset) == 8);
    assert!(core::mem::offset_of!(SrvDesc, u) == 16);
    assert!(core::mem::offset_of!(RtvDesc, u) == 8);
    assert!(core::mem::offset_of!(GraphicsPipelineDesc, blend) == 64);
    assert!(core::mem::offset_of!(GraphicsPipelineDesc, rasterizer) == 396);
    assert!(core::mem::offset_of!(GraphicsPipelineDesc, input_layout) == 492);
    assert!(core::mem::offset_of!(GraphicsPipelineDesc, formats) == 512);
    assert!(core::mem::offset_of!(GraphicsPipelineDesc, flags) == 568);
};

/// `Release` of a COM object, if there is one.
pub fn release(o: P) {
    if !o.is_null() {
        crate::vcall!(o, slot::RELEASE, fn() -> u32,);
    }
}

/// A transition of one subresource (or all: `ALL_SUBRESOURCES`) of `res`.
pub fn barrier(list: P, res: P, sub: u32, before: u32, after: u32) {
    if before == after {
        return;
    }
    let b = [Barrier { ty: BARRIER_TYPE_TRANSITION, flags: 0, resource: res, subresource: sub, before, after }];
    crate::vcall!(list, slot::LIST_RESOURCE_BARRIER, fn(u32, *const Barrier) -> (), 1, b.as_ptr());
}

/// A committed resource. None: refused.
pub fn committed(dev: P, heap: u32, desc: &ResourceDesc, state: u32, clear: Option<&ClearValue>) -> Option<P> {
    let hp = HeapProperties { ty: heap, ..Default::default() };
    let cv: *const ClearValue = clear.map_or(core::ptr::null(), |c| c as *const ClearValue);
    let mut res: P = core::ptr::null_mut();
    let hr = crate::vcall!(dev, slot::DEV_CREATE_COMMITTED_RESOURCE, fn(*const HeapProperties, u32, *const ResourceDesc, u32, *const ClearValue, *const Guid, *mut P) -> i32,
        &hp, 0, desc, state, cv, &IID_RESOURCE, &mut res);
    (hr >= 0 && !res.is_null()).then_some(res)
}

/// A buffer of `size` bytes in `heap`.
pub fn buffer_desc(size: u64) -> ResourceDesc {
    ResourceDesc {
        dimension: RESOURCE_DIMENSION_BUFFER,
        alignment: 0,
        width: size.max(4),
        height: 1,
        depth_or_array_size: 1,
        mip_levels: 1,
        format: FORMAT_UNKNOWN,
        sample: SampleDesc { count: 1, quality: 0 },
        layout: TEXTURE_LAYOUT_ROW_MAJOR,
        flags: 0,
    }
}

pub fn gpu_address(res: P) -> u64 {
    crate::vcall!(res, slot::RES_GET_GPU_VIRTUAL_ADDRESS, fn() -> u64,)
}

/// Maps subresource 0 of a buffer for the CPU (`read`: the whole of it is read).
pub fn map(res: P, read: bool) -> Option<*mut u8> {
    let none = [0usize; 2];
    let range: *const [usize; 2] = if read { core::ptr::null() } else { &none };
    let mut p: P = core::ptr::null_mut();
    let hr = crate::vcall!(res, slot::RES_MAP, fn(u32, *const [usize; 2], *mut P) -> i32, 0, range, &mut p);
    (hr >= 0 && !p.is_null()).then_some(p as *mut u8)
}

/// Unmaps it (`written`: the CPU wrote all of it; otherwise nothing).
pub fn unmap(res: P, written: bool) {
    let none = [0usize; 2];
    let range: *const [usize; 2] = if written { core::ptr::null() } else { &none };
    crate::vcall!(res, slot::RES_UNMAP, fn(u32, *const [usize; 2]) -> (), 0, range);
}
