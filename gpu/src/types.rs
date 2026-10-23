//! The descriptions resources and pipelines are made from, and the small values a
//! command takes. They name what a draw needs, not how Direct3D 9 spells it, so that
//! a Vulkan or Direct3D 12 backend can take the same ones; `d3d9.rs` maps each.
//! Everything here is plain data, `#[repr(C)]` or `#[repr(u32)]` where it may cross
//! the C boundary the display will export.

/// A texel format. The names say the order of the channels in memory, lowest byte
/// first, as Vulkan's and DXGI's do (`Bgra8` is Direct3D 9's `A8R8G8B8`).
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Format {
    /// Not a format this interface knows (an engine surface of another format).
    #[default]
    Unknown = 0,
    Bgra8,
    /// `Bgra8` with the alpha byte ignored (reads as 1).
    Bgrx8,
    B5G6R5,
    /// Direct3D 9's `A2R10G10B10`.
    Bgr10A2,
    /// Direct3D 9's `A2B10G10R10`.
    Rgb10A2,
    /// One unsigned-normalised channel. Direct3D 9 makes it `L8`, which samples as
    /// (r, r, r, 1); a backend without luminance formats swizzles to match.
    R8,
    /// Alpha only: samples as (0, 0, 0, a).
    A8,
    R16F,
    Rg16F,
    Rgba16F,
    R32F,
    Rg32F,
    Rgba32F,
    D16,
    D24S8,
    D24X8,
    D32,
    D15S1,
    D24X4S4,
}

impl Format {
    pub fn is_depth(self) -> bool {
        matches!(self, Format::D16 | Format::D24S8 | Format::D24X8 | Format::D32 | Format::D15S1 | Format::D24X4S4)
    }

    /// Bytes per texel of the colour formats; None for depth and unknown ones.
    pub fn bytes(self) -> Option<usize> {
        Some(match self {
            Format::R8 | Format::A8 => 1,
            Format::B5G6R5 | Format::R16F => 2,
            Format::Bgra8 | Format::Bgrx8 | Format::Bgr10A2 | Format::Rgb10A2 | Format::Rg16F | Format::R32F => 4,
            Format::Rgba16F | Format::Rg32F => 8,
            Format::Rgba32F => 16,
            _ => return None,
        })
    }

    /// Direct3D 9's name for it, which the frame dumps' sidecars print.
    pub fn name(self) -> &'static str {
        match self {
            Format::Unknown => "?",
            Format::Bgra8 => "A8R8G8B8",
            Format::Bgrx8 => "X8R8G8B8",
            Format::B5G6R5 => "R5G6B5",
            Format::Bgr10A2 => "A2R10G10B10",
            Format::Rgb10A2 => "A2B10G10R10",
            Format::R8 => "L8",
            Format::A8 => "A8",
            Format::R16F => "R16F",
            Format::Rg16F => "G16R16F",
            Format::Rgba16F => "A16B16G16R16F",
            Format::R32F => "R32F",
            Format::Rg32F => "G32R32F",
            Format::Rgba32F => "A32B32G32R32F",
            Format::D16 => "D16",
            Format::D24S8 => "D24S8",
            Format::D24X8 => "D24X8",
            Format::D32 => "D32",
            Format::D15S1 => "D15S1",
            Format::D24X4S4 => "D24X4S4",
        }
    }
}

/// What a texture is for, as flags. A later backend places its barriers from these;
/// Direct3D 9 picks its pool and usage from them.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Usage(pub u32);

impl Usage {
    /// Read by shaders.
    pub const SAMPLED: Usage = Usage(1);
    /// Drawn into as a colour target.
    pub const RENDER_TARGET: Usage = Usage(2);
    /// A depth (and stencil) target.
    pub const DEPTH: Usage = Usage(4);
    /// Its texels are rewritten from the CPU while it is in use (`Texture::write`).
    pub const DYNAMIC: Usage = Usage(8);
    /// Read back to the CPU (`Cmd::read_back`), or the source of a copy.
    pub const COPY_SRC: Usage = Usage(16);
    /// The destination of a copy or resolve.
    pub const COPY_DST: Usage = Usage(32);
    /// A colour target whose levels below the first are made from it by the device
    /// (`Cmd::generate_mips`): a blit chain on Vulkan, Direct3D 9's `AUTOGENMIPMAP`.
    pub const GEN_MIPS: Usage = Usage(64);
    /// Written by compute shaders as a storage image (`Cmd::bind_storage_image`):
    /// Vulkan's `STORAGE`, Direct3D 12's unordered access. Only where `Gpu::compute`
    /// says so; Direct3D 12 at feature level 11_0 stores any format but loads only the
    /// 32-bit single-channel ones (`R32F`).
    pub const STORAGE: Usage = Usage(128);

    pub const fn has(self, o: Usage) -> bool {
        self.0 & o.0 == o.0
    }
}

impl core::ops::BitOr for Usage {
    type Output = Usage;
    fn bitor(self, o: Usage) -> Usage {
        Usage(self.0 | o.0)
    }
}

/// What shape a texture is. On Vulkan a cube is an image of six layers with a cube view,
/// and a volume a 3D image; on Direct3D 12 a 2D array of six with a cube SRV, and a 3D
/// resource.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextureKind {
    #[default]
    D2,
    /// Six square faces, in Direct3D's order (+x, -x, +y, -y, +z, -z).
    Cube,
    /// A volume of `depth` slices.
    D3,
}

/// A texture's description. `samples` is 1 for an ordinary texture; more makes a
/// multisampled target, which is drawn into and resolved, never sampled. `levels` 0
/// is the whole chain. A depth texture that is also `SAMPLED` can be read by shaders
/// (Direct3D 9's `INTZ`) and is filled by `Cmd::resolve_depth`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    pub format: Format,
    pub samples: u32,
    pub usage: Usage,
    pub kind: TextureKind,
    /// Slices of a volume; 1 otherwise.
    pub depth: u32,
}

impl TextureDesc {
    /// One level, one sample.
    pub const fn d2(width: u32, height: u32, format: Format, usage: Usage) -> TextureDesc {
        TextureDesc { width, height, levels: 1, format, samples: 1, usage, kind: TextureKind::D2, depth: 1 }
    }
    /// A cube of `edge` on a side, with `levels` levels (0: the whole chain).
    pub const fn cube(edge: u32, levels: u32, format: Format, usage: Usage) -> TextureDesc {
        TextureDesc { width: edge, height: edge, levels, format, samples: 1, usage, kind: TextureKind::Cube, depth: 1 }
    }
    /// A volume of `width` x `height` x `depth`, with `levels` levels.
    pub const fn d3(width: u32, height: u32, depth: u32, levels: u32, format: Format, usage: Usage) -> TextureDesc {
        TextureDesc { width, height, levels, format, samples: 1, usage, kind: TextureKind::D3, depth }
    }
    /// The same with `levels` levels.
    pub const fn with_levels(mut self, levels: u32) -> TextureDesc {
        self.levels = levels;
        self
    }
    /// How many levels it has: `levels`, or the whole chain for 0.
    pub fn level_count(&self) -> u32 {
        if self.levels > 0 {
            return self.levels;
        }
        let d = if self.kind == TextureKind::D3 { self.depth } else { 1 };
        let m = self.width.max(self.height).max(d).max(1);
        32 - m.leading_zeros()
    }
}

/// What a texture starts with: its levels, tightly packed and in order (rows
/// top-down, `Format::bytes` per texel). A cube's are its faces in order, each with its
/// levels; a volume's levels are each its slices in order. The texture keeps them, so
/// that the backend can make it again after the device is lost (a static slice costs
/// nothing to keep).
pub enum Contents {
    None,
    Static(&'static [u8]),
    Owned(alloc::vec::Vec<u8>),
    /// Let go of once the texture is made, to hold no memory for it (render3d's game
    /// textures: the process has 2 GB of address space). After the device is lost such
    /// a texture cannot be made again and draws nothing (`Texture::ready` is false): its
    /// owner makes another.
    Transient(alloc::vec::Vec<u8>),
}

impl Contents {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Contents::None => &[],
            Contents::Static(s) => s,
            Contents::Owned(v) | Contents::Transient(v) => v,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BufferKind {
    Vertex,
    Index16,
    Index32,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BufferDesc {
    pub size: u32,
    pub kind: BufferKind,
    /// Rewritten from the CPU every frame (the frame's rings); otherwise filled once.
    pub dynamic: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    Point,
    Linear,
    Anisotropic,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MipFilter {
    None,
    Point,
    Linear,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Address {
    Wrap,
    Mirror,
    Clamp,
}

/// How a texture is sampled. Bound with the texture, in a binding group's slot.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sampler {
    pub min: Filter,
    pub mag: Filter,
    pub mip: MipFilter,
    pub address_u: Address,
    pub address_v: Address,
    pub address_w: Address,
    /// The most anisotropy, with `Filter::Anisotropic`.
    pub anisotropy: u32,
    /// The texels are sRGB, converted to linear when read.
    pub srgb: bool,
}

impl Sampler {
    pub const LINEAR_CLAMP: Sampler = Sampler {
        min: Filter::Linear,
        mag: Filter::Linear,
        mip: MipFilter::None,
        address_u: Address::Clamp,
        address_v: Address::Clamp,
        address_w: Address::Clamp,
        anisotropy: 1,
        srgb: false,
    };
    pub const POINT_CLAMP: Sampler = Sampler { min: Filter::Point, mag: Filter::Point, ..Sampler::LINEAR_CLAMP };
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Factor {
    Zero,
    One,
    SrcColor,
    InvSrcColor,
    SrcAlpha,
    InvSrcAlpha,
    DstAlpha,
    InvDstAlpha,
    DstColor,
    InvDstColor,
    /// min(src alpha, 1 - dst alpha) for colour, 1 for alpha.
    SrcAlphaSat,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlendOp {
    Add,
    Subtract,
    RevSubtract,
    Min,
    Max,
}

/// One blend equation: `src * s op dst * d`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlendEq {
    pub src: Factor,
    pub dst: Factor,
    pub op: BlendOp,
}

/// Blending, when on: the colour's equation, and the alpha's when it differs.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Blend {
    pub color: BlendEq,
    pub alpha: Option<BlendEq>,
}

impl Blend {
    /// Straight alpha over what is there.
    pub const ALPHA: Blend =
        Blend { color: BlendEq { src: Factor::SrcAlpha, dst: Factor::InvSrcAlpha, op: BlendOp::Add }, alpha: None };
    /// Added onto what is there.
    pub const ADD: Blend = Blend { color: BlendEq { src: Factor::One, dst: Factor::One, op: BlendOp::Add }, alpha: None };
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Compare {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

/// The depth test. `bias` and `slope_bias` are Direct3D 9's units (a bias in depth,
/// and a factor of the slope), which a later backend converts.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Depth {
    pub test: bool,
    pub write: bool,
    pub compare: Compare,
    pub bias: f32,
    pub slope_bias: f32,
}

impl Depth {
    pub const OFF: Depth = Depth { test: false, write: false, compare: Compare::Always, bias: 0.0, slope_bias: 0.0 };
    pub const TEST_WRITE: Depth = Depth { test: true, write: true, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cull {
    None,
    /// Triangles wound clockwise on screen are dropped (Direct3D 9's `CULL_CW`).
    Clockwise,
    CounterClockwise,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Topology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

impl Topology {
    /// How many primitives `n` vertices (or indices) make.
    pub fn primitives(self, n: u32) -> u32 {
        match self {
            Topology::PointList => n,
            Topology::LineList => n / 2,
            Topology::LineStrip => n.saturating_sub(1),
            Topology::TriangleList => n / 3,
            Topology::TriangleStrip => n.saturating_sub(2),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttrFormat {
    Float1,
    Float2,
    Float3,
    Float4,
    /// Four bytes B, G, R, A, read as unsigned-normalised R, G, B, A (`D3DCOLOR`).
    Bgra8,
    UByte4,
    UByte4N,
    Short2,
}

/// What an attribute feeds in the vertex shader. With Direct3D 9 it is the semantic
/// (`TEXCOORD3` is `TexCoord` 3); a later backend numbers its locations in the order
/// the layout lists them.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Semantic {
    Position,
    Normal,
    TexCoord,
    Tangent,
    Color,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VertexAttr {
    pub offset: u16,
    pub format: AttrFormat,
    pub semantic: Semantic,
    pub index: u8,
}

pub const fn attr(offset: u16, format: AttrFormat, semantic: Semantic, index: u8) -> VertexAttr {
    VertexAttr { offset, format, semantic, index }
}

/// Shader code for each backend a pipeline may be made on: Direct3D 9's bytecode
/// (vkd3d's d3dbc), SPIR-V (dxc's) and DXIL (the pinned dxc's, signed), compiled from
/// one HLSL source (render3d/README.md, "One source, two compilers"). `dxil_at` is a
/// pixel shader's alpha-tested twin (an entry marked `:at`, `render3d/dxil-alpha.py`):
/// Direct3D 12 has no alpha test, so a pipeline with one draws with the twin. Empty
/// where a form was not built.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Code {
    pub d3d9: &'static [u32],
    pub spirv: &'static [u32],
    pub dxil: &'static [u32],
    pub dxil_at: &'static [u32],
}

impl Code {
    /// The same shader as d3dbc and as SPIR-V (`SYMBOL` and `SYMBOL_SPV`), no DXIL.
    pub const fn new(d3d9: &'static [u32], spirv: &'static [u32]) -> Code {
        Code { d3d9, spirv, dxil: &[], dxil_at: &[] }
    }
    /// With its DXIL (`SYMBOL_DXIL`).
    pub const fn with_dxil(d3d9: &'static [u32], spirv: &'static [u32], dxil: &'static [u32]) -> Code {
        Code { d3d9, spirv, dxil, dxil_at: &[] }
    }
    /// With its DXIL and its alpha-tested twin (`SYMBOL_AT_DXIL`).
    pub const fn with_dxil_at(d3d9: &'static [u32], spirv: &'static [u32], dxil: &'static [u32], dxil_at: &'static [u32]) -> Code {
        Code { d3d9, spirv, dxil, dxil_at }
    }
}

/// A shader's forms from the generated statics (`shaders/<name>.rs`, `<name>_spv.rs`,
/// `<name>_dxil.rs`), all in scope: `code!(PS_X, PS_X_SPV, PS_X_DXIL)`, and
/// `PS_X_AT_DXIL` after them for a pixel shader drawn with an alpha test.
#[macro_export]
macro_rules! code {
    ($d3d9:ident, $spirv:ident) => {
        $crate::Code::new(&$d3d9, &$spirv)
    };
    ($d3d9:ident, $spirv:ident, $dxil:ident) => {
        $crate::Code::with_dxil(&$d3d9, &$spirv, &$dxil)
    };
    ($d3d9:ident, $spirv:ident, $dxil:ident, $at:ident) => {
        $crate::Code::with_dxil_at(&$d3d9, &$spirv, &$dxil, &$at)
    };
}

pub const MAX_COLORS: usize = 4;

/// The alpha test: a fragment is kept when its alpha (0..255) passes `compare` against
/// `reference`. Direct3D 9 has it as fixed-function state, Vulkan and Direct3D 12 do
/// not: the Vulkan backend patches a discard into the pixel shader's SPIR-V, the
/// Direct3D 12 one draws with its alpha-tested twin (`Code::dxil_at`).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AlphaTest {
    pub compare: Compare,
    pub reference: u8,
}

impl AlphaTest {
    /// Kept from alpha 128 (one half) up: what render3d's cut-out textures use.
    pub const HALF: AlphaTest = AlphaTest { compare: Compare::GreaterEqual, reference: 128 };
}

/// Everything a draw's fixed-function state depends on, made into a `Pipeline` once.
/// The target formats and sample count are those of the passes it is drawn in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PipelineDesc {
    pub vs: Code,
    pub ps: Code,
    pub layout: &'static [VertexAttr],
    /// Bytes from one vertex to the next.
    pub stride: u32,
    pub topology: Topology,
    pub blend: Option<Blend>,
    /// Which of R, G, B, A (bits 0..3) are written.
    pub write_mask: u8,
    pub depth: Depth,
    pub cull: Cull,
    /// Whether draws are clipped to `Cmd::set_scissor`'s rectangle.
    pub scissor: bool,
    pub alpha_test: Option<AlphaTest>,
    /// The colour targets' formats, `Format::Unknown` past the last.
    pub colors: [Format; MAX_COLORS],
    pub depth_format: Option<Format>,
    pub samples: u32,
}

impl PipelineDesc {
    /// A pipeline with these shaders and vertices, drawing into one target of `color`:
    /// no blending, no depth, no culling, no scissor, all channels written.
    pub const fn new(vs: Code, ps: Code, layout: &'static [VertexAttr], stride: u32, topology: Topology, color: Format) -> PipelineDesc {
        PipelineDesc {
            vs,
            ps,
            layout,
            stride,
            topology,
            blend: None,
            write_mask: 0xf,
            depth: Depth::OFF,
            cull: Cull::None,
            scissor: false,
            alpha_test: None,
            colors: [color, Format::Unknown, Format::Unknown, Format::Unknown],
            depth_format: None,
            samples: 1,
        }
    }
}

/// A rectangle in pixels of a target.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }
}

/// Where a pass's draws land, and the range of depth they map to.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Viewport {
    pub const fn of(r: Rect) -> Viewport {
        Viewport { x: r.x, y: r.y, w: r.w, h: r.h, min_z: 0.0, max_z: 1.0 }
    }
}

/// What a pass does with a target's contents as it begins.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Load<T> {
    /// Keeps them.
    Load,
    /// Sets them to this (inside the pass's area).
    Clear(T),
    /// The pass draws all of what it reads after: they may be anything.
    DontCare,
}

/// A colour target of a pass, and its clear value as R, G, B, A. `layer` is a cube's
/// face (0 otherwise); the pass draws into its first level.
pub struct ColorTarget<'a> {
    pub texture: &'a crate::Texture,
    pub load: Load<[f32; 4]>,
    pub layer: u32,
}

impl<'a> ColorTarget<'a> {
    pub const fn new(texture: &'a crate::Texture, load: Load<[f32; 4]>) -> ColorTarget<'a> {
        ColorTarget { texture, load, layer: 0 }
    }
    /// Face `layer` of a cube.
    pub const fn face(texture: &'a crate::Texture, layer: u32, load: Load<[f32; 4]>) -> ColorTarget<'a> {
        ColorTarget { texture, load, layer }
    }
}

/// The depth target of a pass, and its clear depth. The stencil is always kept.
pub struct DepthTarget<'a> {
    pub texture: &'a crate::Texture,
    pub load: Load<f32>,
}

/// A render pass: its targets and the area it draws in (None: the whole of them).
/// Its load operations act inside the area only.
pub struct Pass<'a> {
    pub colors: &'a [ColorTarget<'a>],
    pub depth: Option<DepthTarget<'a>>,
    pub area: Option<Rect>,
}

/// A uniform block: float4 registers from `base` on, which is where the shader
/// declares them (Direct3D 9's constant registers). A later backend gives each block
/// a buffer binding instead, in the order a binding group lists them.
#[derive(Clone, Copy)]
pub struct Block<'a> {
    pub base: u32,
    pub data: &'a [f32],
}

impl<'a> Block<'a> {
    pub const fn new(base: u32, data: &'a [f32]) -> Block<'a> {
        Block { base, data }
    }
    /// A `#[repr(C)]` struct of floats as a block; its size must be a whole number
    /// of float4 registers.
    pub fn of<T: Copy>(base: u32, v: &'a T) -> Block<'a> {
        let n = core::mem::size_of::<T>();
        assert!(n % 16 == 0 && core::mem::align_of::<T>() <= 16, "a uniform block is whole float4 registers");
        Block { base, data: unsafe { core::slice::from_raw_parts(v as *const T as *const f32, n / 4) } }
    }
}

/// A texture slot of a binding group: the texture (None: nothing bound) and how it
/// is sampled.
#[derive(Clone, Copy)]
pub struct TexSlot<'a> {
    pub slot: u32,
    pub texture: Option<&'a crate::Texture>,
    pub sampler: Sampler,
}

/// What a draw reads besides its vertices: a binding group. On Direct3D 9 it sets the
/// samplers and constant registers named; a later backend makes it a descriptor set
/// (Vulkan) or a root table (Direct3D 12).
#[derive(Clone, Copy)]
pub struct Bindings<'a> {
    pub textures: &'a [TexSlot<'a>],
    pub vs: &'a [Block<'a>],
    pub ps: &'a [Block<'a>],
}

/// A storage buffer (`Gpu::storage`): `size` bytes of elements of `stride` bytes (a
/// structured buffer's element; 4 for a plain array of words). Read by any shader as a
/// `StructuredBuffer` (`Cmd::bind_storage`), written by compute shaders as a
/// `RWStructuredBuffer` (`Cmd::bind_storage_rw`) or from the CPU through the frame's
/// ring (`Cmd::write_storage`). Its contents start undefined.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StorageDesc {
    pub size: u32,
    pub stride: u32,
}

/// A compute pipeline: one compute shader (dxc's only: `Code::spirv`, `Code::dxil`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ComputeDesc {
    pub cs: Code,
}

/// The storage slots a `Cmd` binds: read-only buffers (`t16 + n`, SPIR-V set 2 binding
/// n, every stage), read-write buffers (`u n`, set 3 binding n, compute) and storage
/// images (`u 8 + n`, set 3 binding 8 + n, compute). `gpu/shaders/compat.hlsli` spells
/// them `STORAGE(n)`, `STORAGE_RW(n)` and `STORAGE_IMAGE(n)`.
pub const STORAGE_SLOTS: usize = 8;

/// What a finished `Timer` holds besides its stamps.
#[derive(Clone, Copy, Debug)]
pub struct TimerResult {
    /// The GPU's clock changed in between: the stamps mean nothing.
    pub disjoint: bool,
    /// Ticks per second.
    pub frequency: u64,
}
