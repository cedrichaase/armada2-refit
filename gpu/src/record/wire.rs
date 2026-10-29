//! The recording's file format (README.md, "Record and replay"): one stream per plugin,
//! little-endian throughout.
//!
//! A stream is the header, then records. The header is `MAGIC`, `VERSION` (u32), and a
//! string each for the plugin (its module's file name), the backend it was recorded on
//! and the recording's name, then Display8's frame number (u64). A record is
//!
//! | field | type | |
//! |---|---|---|
//! | `seq` | u32 | the order of the call among every plugin's (one counter per process) |
//! | `op` | u16 | `OP_*` |
//! | `_` | u16 | 0 |
//! | `len` | u32 | the payload's bytes |
//! | payload | `len` bytes | the call's arguments, as `W` writes them |
//!
//! Objects are numbered per stream (`id`, from 1; 0 is none). Bulk contents (a
//! texture's, a buffer's) are not in the stream: they are blobs, files of their own in
//! `blobs\` beside it, named by the FNV-1a hash of their bytes (`blob_name`), so a
//! texture two plugins use is written once.
use crate::types::*;
use alloc::string::String;
use alloc::vec::Vec;

pub const MAGIC: &[u8; 8] = b"A2GPUREC";
pub const VERSION: u32 = 1;

// Objects, and the contents of those the frame finds made (a snapshot: `SNAP_*`).
pub const OP_CODE: u16 = 1; // id, words
pub const OP_TEX_NEW: u16 = 2; // id, desc, blob
pub const OP_TEX_SNAP: u16 = 3; // id, desc, role, blob
pub const OP_TEX_WRITE: u16 = 4; // id, pitch, w, rows, bytes
pub const OP_BUF_NEW: u16 = 5; // id, desc, blob
pub const OP_BUF_SNAP: u16 = 6; // id, desc, blob
pub const OP_STO_NEW: u16 = 7; // id, desc
pub const OP_STO_SNAP: u16 = 8; // id, desc, blob
pub const OP_PIPE: u16 = 9; // id, PipelineDesc
pub const OP_CPIPE: u16 = 10; // id, ComputeDesc
pub const OP_FRAME_NEXT: u16 = 11; // frame
// Commands: each starts with the `Cmd`'s id.
pub const OP_BEGIN: u16 = 20; // cmd, frame
pub const OP_END: u16 = 21; // cmd
pub const OP_BEGIN_PASS: u16 = 22;
pub const OP_END_PASS: u16 = 23;
pub const OP_VIEWPORT: u16 = 24;
pub const OP_SCISSOR: u16 = 25;
pub const OP_CLEAR_RECTS: u16 = 26;
pub const OP_SET_PIPELINE: u16 = 27;
pub const OP_SET_SAMPLER: u16 = 28;
pub const OP_BIND_TEXTURE: u16 = 29;
pub const OP_VS_BLOCK: u16 = 30;
pub const OP_PS_BLOCK: u16 = 31;
pub const OP_CS_BLOCK: u16 = 32;
pub const OP_BIND_STORAGE: u16 = 33;
pub const OP_BIND_STORAGE_RW: u16 = 34;
pub const OP_BIND_STORAGE_IMAGE: u16 = 35;
pub const OP_WRITE_STORAGE: u16 = 36;
pub const OP_DISPATCH: u16 = 37;
pub const OP_BARRIER: u16 = 38;
pub const OP_VERTICES: u16 = 39; // cmd, alloc, stride, bytes
pub const OP_INDICES: u16 = 40; // cmd, alloc, bytes
pub const OP_SPAN_OF: u16 = 41; // cmd, alloc, buffer, stride
pub const OP_DRAW: u16 = 42;
pub const OP_DRAW_INDEXED: u16 = 43;
pub const OP_DRAW_BUFFER: u16 = 44;
pub const OP_GEN_MIPS: u16 = 45;
pub const OP_RESOLVE_DEPTH: u16 = 46;
pub const OP_RESOLVE: u16 = 47;
pub const OP_COPY: u16 = 48;
pub const OP_READ_BACK: u16 = 49; // a read-back the plugin made: nothing to replay
pub const OP_END_STREAM: u16 = 99;

/// What a snapshot is of (`OP_TEX_SNAP`).
pub const ROLE_OWN: u32 = 0;
/// The engine's back buffer (`Gpu::engine_target`).
pub const ROLE_ENGINE_COLOR: u32 = 1;
/// The engine's depth (`Gpu::engine_depth`).
pub const ROLE_ENGINE_DEPTH: u32 = 2;
/// One of Display8's mirrors of an engine texture (`Gpu::engine_texture`).
pub const ROLE_MIRROR: u32 = 3;

/// FNV-1a, 64 bits: a blob's name.
pub fn fnv(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A blob's file name in `blobs\`: its hash and its length.
pub fn blob_name(hash: u64, len: u32) -> String {
    let mut s = String::with_capacity(32);
    hex(&mut s, hash, 16);
    s.push('-');
    hex(&mut s, len as u64, 8);
    s.push_str(".bin");
    s
}

fn hex(s: &mut String, v: u64, digits: u32) {
    for i in (0..digits).rev() {
        let d = ((v >> (i * 4)) & 0xf) as u8;
        s.push(char::from(if d < 10 { b'0' + d } else { b'a' + d - 10 }));
    }
}

/// A blob reference in a record: none (len 0, hash 0), or a file in `blobs\`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Blob {
    pub hash: u64,
    pub len: u32,
}

/// A record's payload, written.
#[derive(Default)]
pub struct W {
    pub buf: Vec<u8>,
}

impl W {
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.u32(v.to_bits())
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(v as u8)
    }
    /// A length (u32) and the bytes.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.u32(b.len() as u32);
        self.buf.extend_from_slice(b);
        self
    }
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.bytes(s.as_bytes())
    }
    pub fn blob(&mut self, b: Blob) -> &mut Self {
        self.u64(b.hash).u32(b.len)
    }
    pub fn rect(&mut self, r: Rect) -> &mut Self {
        self.u32(r.x).u32(r.y).u32(r.w).u32(r.h)
    }
    pub fn opt_rect(&mut self, r: Option<Rect>) -> &mut Self {
        match r {
            Some(r) => self.u8(1).rect(r),
            None => self.u8(0),
        }
    }
    pub fn tex_desc(&mut self, d: &TextureDesc) -> &mut Self {
        self.u32(d.width).u32(d.height).u32(d.levels).u32(d.format as u32).u32(d.samples).u32(d.usage.0).u32(d.kind as u32).u32(d.depth)
    }
    pub fn buf_desc(&mut self, d: &BufferDesc) -> &mut Self {
        self.u32(d.size).u32(d.kind as u32).bool(d.dynamic)
    }
    pub fn sampler(&mut self, s: &Sampler) -> &mut Self {
        self.u32(s.min as u32)
            .u32(s.mag as u32)
            .u32(s.mip as u32)
            .u32(s.address_u as u32)
            .u32(s.address_v as u32)
            .u32(s.address_w as u32)
            .u32(s.anisotropy)
            .bool(s.srgb)
    }
    pub fn floats(&mut self, f: &[f32]) -> &mut Self {
        self.u32(f.len() as u32);
        for v in f {
            self.f32(*v);
        }
        self
    }
    fn blend_eq(&mut self, e: &BlendEq) -> &mut Self {
        self.u32(e.src as u32).u32(e.dst as u32).u32(e.op as u32)
    }
    /// A pipeline's description; `code` names each of its shaders' four blobs.
    pub fn pipe_desc(&mut self, d: &PipelineDesc, vs: [u32; 4], ps: [u32; 4]) -> &mut Self {
        for c in vs.iter().chain(ps.iter()) {
            self.u32(*c);
        }
        self.u32(d.layout.len() as u32);
        for a in d.layout {
            self.u16(a.offset).u8(a.format as u8).u8(a.semantic as u8).u8(a.index);
        }
        self.u32(d.stride).u32(d.topology as u32);
        match &d.blend {
            Some(b) => {
                self.u8(1).blend_eq(&b.color);
                match &b.alpha {
                    Some(a) => self.u8(1).blend_eq(a),
                    None => self.u8(0),
                };
            }
            None => {
                self.u8(0);
            }
        }
        self.u8(d.write_mask);
        self.bool(d.depth.test).bool(d.depth.write).u32(d.depth.compare as u32).f32(d.depth.bias).f32(d.depth.slope_bias);
        self.u32(d.cull as u32).bool(d.scissor);
        match d.alpha_test {
            Some(a) => self.u8(1).u32(a.compare as u32).u8(a.reference),
            None => self.u8(0),
        };
        for c in d.colors {
            self.u32(c as u32);
        }
        match d.depth_format {
            Some(f) => self.u8(1).u32(f as u32),
            None => self.u8(0),
        };
        self.u32(d.samples)
    }
}

/// A record's payload, read. Reading past its end gives zeros and sets `bad`.
pub struct R<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub bad: bool,
}

/// An enum of `#[repr(u32)]` (or `u8`) with variants 0..=`max`, from its number; the
/// first variant when out of range (and `bad` is set).
macro_rules! from_u32 {
    ($name:ident, $t:ty, $max:expr) => {
        pub fn $name(&mut self) -> $t {
            let v = self.u32();
            if v > $max {
                self.bad = true;
                return unsafe { core::mem::transmute::<u32, $t>(0) };
            }
            unsafe { core::mem::transmute::<u32, $t>(v) }
        }
    };
}
macro_rules! from_u8 {
    ($name:ident, $t:ty, $max:expr) => {
        pub fn $name(&mut self) -> $t {
            let v = self.u8();
            if v > $max {
                self.bad = true;
                return unsafe { core::mem::transmute::<u8, $t>(0) };
            }
            unsafe { core::mem::transmute::<u8, $t>(v) }
        }
    };
}

impl<'a> R<'a> {
    pub fn new(data: &'a [u8]) -> R<'a> {
        R { data, pos: 0, bad: false }
    }
    pub fn take(&mut self, n: usize) -> &'a [u8] {
        if self.pos + n > self.data.len() {
            self.bad = true;
            self.pos = self.data.len();
            return &[];
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        s
    }
    fn arr<const N: usize>(&mut self) -> [u8; N] {
        let s = self.take(N);
        let mut a = [0u8; N];
        if s.len() == N {
            a.copy_from_slice(s);
        }
        a
    }
    pub fn u8(&mut self) -> u8 {
        self.arr::<1>()[0]
    }
    pub fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.arr())
    }
    pub fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.arr())
    }
    pub fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.arr())
    }
    pub fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
    pub fn bool(&mut self) -> bool {
        self.u8() != 0
    }
    pub fn bytes(&mut self) -> &'a [u8] {
        let n = self.u32() as usize;
        self.take(n)
    }
    pub fn str(&mut self) -> String {
        String::from_utf8_lossy(self.bytes()).into_owned()
    }
    pub fn blob(&mut self) -> Blob {
        Blob { hash: self.u64(), len: self.u32() }
    }
    pub fn rect(&mut self) -> Rect {
        Rect { x: self.u32(), y: self.u32(), w: self.u32(), h: self.u32() }
    }
    pub fn opt_rect(&mut self) -> Option<Rect> {
        if self.u8() != 0 { Some(self.rect()) } else { None }
    }
    from_u32!(format, Format, 19);
    from_u32!(kind, TextureKind, 2);
    from_u32!(buf_kind, BufferKind, 2);
    from_u32!(filter, Filter, 2);
    from_u32!(mip_filter, MipFilter, 2);
    from_u32!(address, Address, 2);
    from_u32!(factor, Factor, 10);
    from_u32!(blend_op, BlendOp, 4);
    from_u32!(compare, Compare, 7);
    from_u32!(cull, Cull, 2);
    from_u32!(topology, Topology, 4);
    from_u8!(attr_format, AttrFormat, 7);
    from_u8!(semantic, Semantic, 4);

    pub fn tex_desc(&mut self) -> TextureDesc {
        TextureDesc {
            width: self.u32(),
            height: self.u32(),
            levels: self.u32(),
            format: self.format(),
            samples: self.u32(),
            usage: Usage(self.u32()),
            kind: self.kind(),
            depth: self.u32(),
        }
    }
    pub fn buf_desc(&mut self) -> BufferDesc {
        BufferDesc { size: self.u32(), kind: self.buf_kind(), dynamic: self.bool() }
    }
    pub fn sampler(&mut self) -> Sampler {
        Sampler {
            min: self.filter(),
            mag: self.filter(),
            mip: self.mip_filter(),
            address_u: self.address(),
            address_v: self.address(),
            address_w: self.address(),
            anisotropy: self.u32(),
            srgb: self.bool(),
        }
    }
    pub fn floats(&mut self) -> Vec<f32> {
        let n = self.u32() as usize;
        let mut v = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            if self.bad {
                break;
            }
            v.push(self.f32());
        }
        v
    }
    fn blend_eq(&mut self) -> BlendEq {
        BlendEq { src: self.factor(), dst: self.factor(), op: self.blend_op() }
    }
    /// A pipeline's description: the code ids of its shaders' blobs (vertex shader's
    /// four, then the pixel shader's), its layout, and the rest with the layout and code
    /// left empty (the caller puts them in).
    pub fn pipe_desc(&mut self) -> ([u32; 8], Vec<VertexAttr>, PipelineDesc) {
        let mut codes = [0u32; 8];
        for c in codes.iter_mut() {
            *c = self.u32();
        }
        let n = self.u32() as usize;
        let mut layout = Vec::with_capacity(n.min(64));
        for _ in 0..n {
            if self.bad {
                break;
            }
            layout.push(VertexAttr { offset: self.u16(), format: self.attr_format(), semantic: self.semantic(), index: self.u8() });
        }
        let stride = self.u32();
        let topology = self.topology();
        let blend = if self.u8() != 0 {
            let color = self.blend_eq();
            let alpha = if self.u8() != 0 { Some(self.blend_eq()) } else { None };
            Some(Blend { color, alpha })
        } else {
            None
        };
        let write_mask = self.u8();
        let depth = Depth { test: self.bool(), write: self.bool(), compare: self.compare(), bias: self.f32(), slope_bias: self.f32() };
        let cull = self.cull();
        let scissor = self.bool();
        let alpha_test = if self.u8() != 0 { Some(AlphaTest { compare: self.compare(), reference: self.u8() }) } else { None };
        let mut colors = [Format::Unknown; MAX_COLORS];
        for c in colors.iter_mut() {
            *c = self.format();
        }
        let depth_format = if self.u8() != 0 { Some(self.format()) } else { None };
        let samples = self.u32();
        let empty = Code::new(&[], &[]);
        let d = PipelineDesc {
            vs: empty,
            ps: empty,
            layout: &[],
            stride,
            topology,
            blend,
            write_mask,
            depth,
            cull,
            scissor,
            alpha_test,
            colors,
            depth_format,
            samples,
        };
        (codes, layout, d)
    }
}
