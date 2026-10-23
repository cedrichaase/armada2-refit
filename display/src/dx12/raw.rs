//! Direct3D 12 and DXGI as Display8 calls them, declared by hand from Wine's
//! `d3d12.idl`, `dxgi.idl` and `dxgi1_2.idl`..`dxgi1_4.idl` (the same as vkd3d-proton's):
//! the interface IDs, the slots it calls (`vcall!`, `this` first), and the structures
//! with their 32-bit layouts, each checked against its size below.
//!
//! **A method that returns a structure** (`GetCPUDescriptorHandleForHeapStart`,
//! `GetGPUDescriptorHandleForHeapStart`) returns it through a hidden pointer after
//! `this`, as Microsoft's C++ compiler does for any member function returning a
//! structure: `fn(this, *mut T) -> *mut T`. Wine's and vkd3d's headers declare them so.
#![allow(dead_code, clippy::upper_case_acronyms)]

use crate::com::Guid;
use core::ffi::c_void;

pub type P = *mut c_void;

const fn guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Guid {
    Guid { d1, d2, d3, d4 }
}

pub const IID_DEVICE: Guid = guid(0x189819f1, 0x1db6, 0x4b57, [0xbe, 0x54, 0x18, 0x21, 0x33, 0x9b, 0x85, 0xf7]);
pub const IID_COMMAND_QUEUE: Guid = guid(0x0ec870a6, 0x5d7e, 0x4c22, [0x8c, 0xfc, 0x5b, 0xaa, 0xe0, 0x76, 0x16, 0xed]);
pub const IID_COMMAND_ALLOCATOR: Guid = guid(0x6102dee4, 0xaf59, 0x4b09, [0xb9, 0x99, 0xb4, 0x4d, 0x73, 0xf0, 0x9b, 0x24]);
pub const IID_GRAPHICS_COMMAND_LIST: Guid = guid(0x5b160d0f, 0xac1b, 0x4185, [0x8b, 0xa8, 0xb3, 0xae, 0x42, 0xa5, 0xa4, 0x55]);
pub const IID_FENCE: Guid = guid(0x0a753dcf, 0xc4d8, 0x4b91, [0xad, 0xf6, 0xbe, 0x5a, 0x60, 0xd9, 0x5a, 0x76]);
pub const IID_DESCRIPTOR_HEAP: Guid = guid(0x8efb471d, 0x616c, 0x4f49, [0x90, 0xf7, 0x12, 0x7b, 0xb7, 0x63, 0xfa, 0x51]);
pub const IID_RESOURCE: Guid = guid(0x696442be, 0xa72e, 0x4059, [0xbc, 0x79, 0x5b, 0x5c, 0x98, 0x04, 0x0f, 0xad]);
pub const IID_ROOT_SIGNATURE: Guid = guid(0xc54a6b66, 0x72df, 0x4ee8, [0x8b, 0xe5, 0xa9, 0x46, 0xa1, 0x42, 0x92, 0x14]);
pub const IID_PIPELINE_STATE: Guid = guid(0x765a30f3, 0xf624, 0x4c6f, [0xa8, 0x28, 0xac, 0xe9, 0x48, 0x62, 0x24, 0x45]);
pub const IID_DEBUG: Guid = guid(0x344488b7, 0x6846, 0x474b, [0xb9, 0x89, 0xf0, 0x27, 0x44, 0x82, 0x45, 0xe0]);
pub const IID_DXGI_FACTORY2: Guid = guid(0x50c83a1c, 0xe072, 0x4c48, [0x87, 0xb0, 0x36, 0x30, 0xfa, 0x36, 0xa6, 0xd0]);
pub const IID_DXGI_SWAP_CHAIN3: Guid = guid(0x94d99bdb, 0xf1f8, 0x4ab0, [0xb2, 0x36, 0x7d, 0xa0, 0x17, 0x0e, 0xda, 0xb1]);

// Slots (Wine's IDLs; IUnknown is 0..2 everywhere).
pub mod slot {
    pub const QUERY_INTERFACE: usize = 0;
    pub const RELEASE: usize = 2;
    // ID3D12Device
    pub const DEV_CREATE_COMMAND_QUEUE: usize = 8;
    pub const DEV_CREATE_COMMAND_ALLOCATOR: usize = 9;
    pub const DEV_CREATE_GRAPHICS_PIPELINE_STATE: usize = 10;
    pub const DEV_CREATE_COMMAND_LIST: usize = 12;
    pub const DEV_CREATE_DESCRIPTOR_HEAP: usize = 14;
    pub const DEV_GET_DESCRIPTOR_HANDLE_INCREMENT_SIZE: usize = 15;
    pub const DEV_CREATE_ROOT_SIGNATURE: usize = 16;
    pub const DEV_CREATE_SHADER_RESOURCE_VIEW: usize = 18;
    pub const DEV_CREATE_RENDER_TARGET_VIEW: usize = 20;
    pub const DEV_CREATE_DEPTH_STENCIL_VIEW: usize = 21;
    pub const DEV_COPY_DESCRIPTORS_SIMPLE: usize = 24;
    pub const DEV_CREATE_COMMITTED_RESOURCE: usize = 27;
    pub const DEV_CREATE_FENCE: usize = 36;
    pub const DEV_GET_DEVICE_REMOVED_REASON: usize = 37;
    // ID3D12CommandQueue
    pub const QUEUE_EXECUTE_COMMAND_LISTS: usize = 10;
    pub const QUEUE_SIGNAL: usize = 14;
    // ID3D12Fence
    pub const FENCE_GET_COMPLETED_VALUE: usize = 8;
    pub const FENCE_SET_EVENT_ON_COMPLETION: usize = 9;
    // ID3D12CommandAllocator
    pub const ALLOC_RESET: usize = 8;
    // ID3D12GraphicsCommandList
    pub const LIST_CLOSE: usize = 9;
    pub const LIST_RESET: usize = 10;
    pub const LIST_DRAW_INSTANCED: usize = 12;
    pub const LIST_COPY_TEXTURE_REGION: usize = 16;
    pub const LIST_IA_SET_PRIMITIVE_TOPOLOGY: usize = 20;
    pub const LIST_RS_SET_VIEWPORTS: usize = 21;
    pub const LIST_RS_SET_SCISSOR_RECTS: usize = 22;
    pub const LIST_SET_PIPELINE_STATE: usize = 25;
    pub const LIST_RESOURCE_BARRIER: usize = 26;
    pub const LIST_SET_DESCRIPTOR_HEAPS: usize = 28;
    pub const LIST_SET_GRAPHICS_ROOT_SIGNATURE: usize = 30;
    pub const LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE: usize = 32;
    pub const LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS: usize = 36;
    pub const LIST_IA_SET_VERTEX_BUFFERS: usize = 44;
    pub const LIST_OM_SET_RENDER_TARGETS: usize = 46;
    pub const LIST_CLEAR_DEPTH_STENCIL_VIEW: usize = 47;
    pub const LIST_CLEAR_RENDER_TARGET_VIEW: usize = 48;
    // ID3D12DescriptorHeap
    pub const HEAP_GET_CPU_START: usize = 9;
    pub const HEAP_GET_GPU_START: usize = 10;
    // ID3D12Resource
    pub const RES_MAP: usize = 8;
    pub const RES_GET_GPU_VIRTUAL_ADDRESS: usize = 11;
    // ID3D12Debug
    pub const DEBUG_ENABLE_DEBUG_LAYER: usize = 3;
    // ID3D10Blob
    pub const BLOB_GET_BUFFER_POINTER: usize = 3;
    pub const BLOB_GET_BUFFER_SIZE: usize = 4;
    // IDXGIFactory2
    pub const FACTORY_MAKE_WINDOW_ASSOCIATION: usize = 8;
    pub const FACTORY_CREATE_SWAP_CHAIN_FOR_HWND: usize = 15;
    // IDXGISwapChain3
    pub const SWAP_PRESENT: usize = 8;
    pub const SWAP_GET_BUFFER: usize = 9;
    pub const SWAP_RESIZE_BUFFERS: usize = 13;
    pub const SWAP_GET_CURRENT_BACK_BUFFER_INDEX: usize = 36;
}

pub const FEATURE_LEVEL_11_0: u32 = 0xb000;

pub const COMMAND_LIST_TYPE_DIRECT: u32 = 0;
pub const DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV: u32 = 0;
pub const DESCRIPTOR_HEAP_TYPE_SAMPLER: u32 = 1;
pub const DESCRIPTOR_HEAP_TYPE_RTV: u32 = 2;
pub const DESCRIPTOR_HEAP_TYPE_DSV: u32 = 3;
pub const DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE: u32 = 1;

pub const HEAP_TYPE_DEFAULT: u32 = 1;
pub const HEAP_TYPE_UPLOAD: u32 = 2;

pub const RESOURCE_DIMENSION_BUFFER: u32 = 1;
pub const RESOURCE_DIMENSION_TEXTURE2D: u32 = 3;
pub const TEXTURE_LAYOUT_UNKNOWN: u32 = 0;
pub const TEXTURE_LAYOUT_ROW_MAJOR: u32 = 1;
pub const RESOURCE_FLAG_ALLOW_RENDER_TARGET: u32 = 1;
pub const RESOURCE_FLAG_ALLOW_DEPTH_STENCIL: u32 = 2;

pub const STATE_COMMON: u32 = 0;
pub const STATE_PRESENT: u32 = 0;
pub const STATE_RENDER_TARGET: u32 = 0x4;
pub const STATE_DEPTH_WRITE: u32 = 0x10;
pub const STATE_NON_PIXEL_SHADER_RESOURCE: u32 = 0x40;
pub const STATE_PIXEL_SHADER_RESOURCE: u32 = 0x80;
pub const STATE_SHADER_RESOURCE: u32 = STATE_NON_PIXEL_SHADER_RESOURCE | STATE_PIXEL_SHADER_RESOURCE;
pub const STATE_COPY_DEST: u32 = 0x400;
pub const STATE_GENERIC_READ: u32 = 0xac3;
pub const ALL_SUBRESOURCES: u32 = 0xffff_ffff;

pub const BARRIER_TYPE_TRANSITION: u32 = 0;
pub const TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX: u32 = 0;
pub const TEXTURE_COPY_TYPE_PLACED_FOOTPRINT: u32 = 1;
pub const TEXTURE_DATA_PITCH_ALIGNMENT: u32 = 256;
pub const TEXTURE_DATA_PLACEMENT_ALIGNMENT: u32 = 512;
pub const CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT: u32 = 256;

pub const CLEAR_FLAG_DEPTH: u32 = 1;

pub const FORMAT_UNKNOWN: u32 = 0;
pub const FORMAT_R32G32_FLOAT: u32 = 16;
pub const FORMAT_R8G8B8A8_UNORM: u32 = 28;
pub const FORMAT_D16_UNORM: u32 = 55;
pub const FORMAT_B8G8R8A8_UNORM: u32 = 87;

pub const SRV_DIMENSION_TEXTURE2D: u32 = 4;
/// `D3D12_ENCODE_SHADER_4_COMPONENT_MAPPING(0, 1, 2, 3)`.
pub const DEFAULT_COMPONENT_MAPPING: u32 = 0x1688;
/// Red, green and blue as they are, alpha forced to one: X8R8G8B8 sampled as
/// B8G8R8A8 (`FORCE_VALUE_1` is 5).
pub const ALPHA_ONE_COMPONENT_MAPPING: u32 = (1 << 12) | (5 << 9) | (2 << 6) | (1 << 3);

pub const ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE: u32 = 0;
pub const ROOT_PARAMETER_TYPE_32BIT_CONSTANTS: u32 = 1;
pub const DESCRIPTOR_RANGE_TYPE_SRV: u32 = 0;
pub const SHADER_VISIBILITY_VERTEX: u32 = 1;
pub const SHADER_VISIBILITY_PIXEL: u32 = 5;
pub const ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT: u32 = 1;
pub const ROOT_SIGNATURE_VERSION_1_0: u32 = 1;

pub const FILTER_MIN_MAG_MIP_POINT: u32 = 0;
pub const TEXTURE_ADDRESS_MODE_CLAMP: u32 = 3;
pub const COMPARISON_FUNC_ALWAYS: u32 = 8;
pub const STATIC_BORDER_COLOR_OPAQUE_BLACK: u32 = 1;

pub const PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE: u32 = 3;
pub const PRIMITIVE_TOPOLOGY_TRIANGLESTRIP: u32 = 5;
pub const INPUT_CLASSIFICATION_PER_VERTEX_DATA: u32 = 0;
pub const FILL_MODE_SOLID: u32 = 3;
pub const CULL_MODE_NONE: u32 = 1;
pub const BLEND_ZERO: u32 = 1;
pub const BLEND_ONE: u32 = 2;
pub const BLEND_OP_ADD: u32 = 1;
pub const LOGIC_OP_NOOP: u32 = 4;
pub const STENCIL_OP_KEEP: u32 = 1;
pub const COLOR_WRITE_ENABLE_ALL: u8 = 15;

pub const DXGI_USAGE_RENDER_TARGET_OUTPUT: u32 = 0x20;
pub const DXGI_SWAP_EFFECT_FLIP_DISCARD: u32 = 4;
pub const DXGI_SCALING_STRETCH: u32 = 0;
pub const DXGI_MWA_NO_WINDOW_CHANGES: u32 = 1;
pub const DXGI_MWA_NO_ALT_ENTER: u32 = 2;
pub const DXGI_ERROR_DEVICE_REMOVED: i32 = 0x887a_0005u32 as i32;
pub const DXGI_ERROR_DEVICE_RESET: i32 = 0x887a_0007u32 as i32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CommandQueueDesc {
    pub ty: u32,
    pub priority: i32,
    pub flags: u32,
    pub node_mask: u32,
}

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

/// `D3D12_CLEAR_VALUE`: the colour, or the depth in `color[0]` and the stencil in the
/// low byte of `color[1]`.
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

/// `D3D12_SHADER_RESOURCE_VIEW_DESC` for a 2D texture (the union is 24 bytes, 8-aligned).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SrvDesc {
    pub format: u32,
    pub dimension: u32,
    pub mapping: u32,
    /// The union is 8-aligned (a buffer view's first element is a UINT64).
    pub _pad: u32,
    pub most_detailed_mip: u32,
    pub mip_levels: u32,
    pub plane_slice: u32,
    pub min_lod_clamp: f32,
    pub _rest: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VertexBufferView {
    pub location: u64,
    pub size: u32,
    pub stride: u32,
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
/// constants: register, space, count).
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
#[derive(Clone, Copy)]
pub struct StaticSampler {
    pub filter: u32,
    pub address_u: u32,
    pub address_v: u32,
    pub address_w: u32,
    pub mip_lod_bias: f32,
    pub max_anisotropy: u32,
    pub comparison: u32,
    pub border: u32,
    pub min_lod: f32,
    pub max_lod: f32,
    pub register: u32,
    pub space: u32,
    pub visibility: u32,
}

#[repr(C)]
pub struct RootSignatureDesc {
    pub count: u32,
    pub params: *const RootParameter,
    pub samplers_count: u32,
    pub samplers: *const StaticSampler,
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
pub struct SwapChainDesc1 {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub stereo: i32,
    pub sample: SampleDesc,
    pub usage: u32,
    pub buffers: u32,
    pub scaling: u32,
    pub swap_effect: u32,
    pub alpha_mode: u32,
    pub flags: u32,
}

// The layouts, against the sizes Microsoft's compiler gives them on x86.
const _: () = {
    use core::mem::size_of;
    assert!(size_of::<CommandQueueDesc>() == 16);
    assert!(size_of::<DescriptorHeapDesc>() == 16);
    assert!(size_of::<HeapProperties>() == 20);
    assert!(size_of::<ResourceDesc>() == 56);
    assert!(size_of::<ClearValue>() == 20);
    assert!(size_of::<Barrier>() == 24);
    assert!(size_of::<CopyLocation>() == 40);
    assert!(size_of::<SrvDesc>() == 40);
    assert!(size_of::<Viewport>() == 24);
    assert!(size_of::<VertexBufferView>() == 16);
    assert!(size_of::<DescriptorRange>() == 20);
    assert!(size_of::<RootParameter>() == 20);
    assert!(size_of::<StaticSampler>() == 52);
    assert!(size_of::<RootSignatureDesc>() == 20);
    assert!(size_of::<RenderTargetBlend>() == 40);
    assert!(size_of::<BlendDesc>() == 328);
    assert!(size_of::<RasterizerDesc>() == 44);
    assert!(size_of::<DepthStencilDesc>() == 52);
    assert!(size_of::<InputElement>() == 28);
    assert!(size_of::<GraphicsPipelineDesc>() == 572);
    assert!(size_of::<SwapChainDesc1>() == 48);
    assert!(core::mem::offset_of!(ResourceDesc, alignment) == 8);
    assert!(core::mem::offset_of!(CopyLocation, offset) == 8);
    assert!(core::mem::offset_of!(SrvDesc, most_detailed_mip) == 16);
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
