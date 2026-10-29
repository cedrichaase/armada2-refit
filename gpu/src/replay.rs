//! Draws a recorded frame (`record`) again, on whatever backend the `Gpu`s given are on
//! (README.md, "Record and replay"). The replayer (`gpu/replay/`) makes the device the
//! way the game does, through Display8, gives one `Gpu` per stream (each plugin had its
//! own), and reads the engine's back buffer back when every record has been replayed.
//!
//! The calls are made again in the order they were made, across the streams, with the
//! objects they name: those the frame made, as they were made; those it found made, from
//! their snapshots. A target the frame found made (a sky's cube, a bake) and only reads
//! is a sampled texture holding its snapshot; one the frame also draws into is made as a
//! target at that first draw, its first level copied from the snapshot (`promote`).
use crate::record::wire::*;
use crate::types::*;
use crate::{Buffer, Cmd, ComputePipeline, Frame, Gpu, Pipeline, Span, Storage, Texture};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub type Log = fn(core::fmt::Arguments);

/// A stream's header.
#[derive(Clone, Debug, Default)]
pub struct Header {
    pub version: u32,
    pub module: String,
    pub backend: String,
    pub name: String,
    pub frame: u64,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub depth_format: u32,
    pub d3d9_centres: bool,
}

/// Reads a stream's header; the offset its records start at.
pub fn header(data: &[u8]) -> Result<(Header, usize), &'static str> {
    if data.len() < 12 || &data[..8] != MAGIC {
        return Err("not a recording (no A2GPUREC)");
    }
    let n = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    if 12 + n > data.len() {
        return Err("the header is cut short");
    }
    let mut r = R::new(&data[12..12 + n]);
    let version = r.u32();
    if version != VERSION {
        return Err("a recording of another version");
    }
    let h = Header {
        version,
        module: r.str(),
        backend: r.str(),
        name: r.str(),
        frame: r.u64(),
        width: r.u32(),
        height: r.u32(),
        format: r.u32(),
        depth_format: r.u32(),
        d3d9_centres: r.bool(),
    };
    if r.bad {
        return Err("the header is cut short");
    }
    Ok((h, 12 + n))
}

/// What the replay did.
#[derive(Default, Debug)]
pub struct Summary {
    pub records: u32,
    pub draws: u32,
    pub dispatches: u32,
    /// Records that could not be replayed (an object or a span not there, a `Cmd` not
    /// begun): the log says which, the first few of each kind.
    pub skipped: u32,
    pub promoted: u32,
    pub bad: u32,
}

struct Tex {
    t: Texture,
    /// The target's description while `t` is the sampled stand-in holding its snapshot.
    target: Option<TextureDesc>,
    /// The stand-in's snapshot (a cube's faces are drawn from it one by one).
    blob: Blob,
}

/// The blit's vertices: position, then texture coordinates.
static BLIT_LAYOUT: [VertexAttr; 2] = [attr(0, AttrFormat::Float2, Semantic::Position, 0), attr(8, AttrFormat::Float2, Semantic::TexCoord, 0)];

type Loader<'a> = &'a dyn Fn(Blob) -> Option<Vec<u8>>;

struct St {
    gpu: &'static Gpu,
    tex: BTreeMap<u32, Tex>,
    bufs: BTreeMap<u32, Buffer>,
    stos: BTreeMap<u32, Storage>,
    pipes: BTreeMap<u32, Pipeline>,
    cpipes: BTreeMap<u32, ComputePipeline>,
    codes: BTreeMap<u32, &'static [u32]>,
    allocs: BTreeMap<u32, Span>,
    cmds: BTreeMap<u32, Cmd<'static>>,
    /// The `Cmd`s begun and not ended, last on top.
    open: Vec<u32>,
    frames: BTreeMap<u32, *mut Frame>,
    target: Option<Texture>,
    depth: Option<Texture>,
    /// The blit's pipeline per target format.
    blits: BTreeMap<u32, Pipeline>,
}

pub struct Replay {
    log: Log,
    streams: Vec<St>,
    /// (seq, stream, op, start, len) of every record, in order.
    order: Vec<(u32, usize, u16, usize, usize)>,
    data: Vec<Vec<u8>>,
    pub summary: Summary,
    said: BTreeMap<u16, u32>,
    /// The engine's back buffer and depth have been given the contents the frame found
    /// them with: one image for every plugin, so by the first plugin to use each (the
    /// snapshot a later plugin took holds what the earlier ones drew).
    engine_color: bool,
    engine_depth: bool,
    /// The replayer's blit (vertex and pixel shader, `gpu/replay/shaders/blit.hlsl`):
    /// how a snapshot goes into a target on every backend (Direct3D 9 copies only
    /// between targets). Without it, a copy, which works on Vulkan and Direct3D 12.
    blit: Option<(Code, Code)>,
}

fn leak_words(w: Vec<u32>) -> &'static [u32] {
    Box::leak(w.into_boxed_slice())
}

/// `n` bytes of `bytes` as little-endian words.
fn words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Vertices of `stride` bytes, as the plugin wrote them (`Cmd::vertices` of a type that
/// size).
fn vertices(cmd: &mut Cmd, bytes: &[u8], stride: u32) -> Option<Span> {
    macro_rules! by {
        ($n:expr, $w:expr; $($k:literal)*) => {
            match $n {
                $( $k => {
                    let s: &[[u32; $k]] = unsafe { core::slice::from_raw_parts($w.as_ptr() as *const [u32; $k], $w.len() / $k) };
                    cmd.vertices(s)
                } )*
                _ => None,
            }
        };
    }
    macro_rules! by_bytes {
        ($n:expr, $b:expr; $($k:literal)*) => {
            match $n {
                $( $k => {
                    let s: &[[u8; $k]] = unsafe { core::slice::from_raw_parts($b.as_ptr() as *const [u8; $k], $b.len() / $k) };
                    cmd.vertices(s)
                } )*
                _ => None,
            }
        };
    }
    if stride % 4 == 0 {
        let w = words(bytes);
        by!(stride / 4, w; 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32
            33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64)
    } else {
        by_bytes!(stride, bytes; 1 2 3 5 6 7 9 10 11 13 14 15 17 18 19 21 22 23 25 26 27 29 30 31)
    }
}

impl Replay {
    /// The streams (each a whole `.a2s` file), one `Gpu` for each (attached to the device
    /// already).
    pub fn new(log: Log, data: Vec<Vec<u8>>, gpus: &[&'static Gpu], blit: Option<(Code, Code)>) -> Result<Replay, &'static str> {
        let mut order = Vec::new();
        let mut streams = Vec::new();
        for (i, d) in data.iter().enumerate() {
            let (_, mut at) = header(d)?;
            while at + 12 <= d.len() {
                let seq = u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]);
                let op = u16::from_le_bytes([d[at + 4], d[at + 5]]);
                let len = u32::from_le_bytes([d[at + 8], d[at + 9], d[at + 10], d[at + 11]]) as usize;
                if at + 12 + len > d.len() {
                    return Err("a record is cut short");
                }
                order.push((seq, i, op, at + 12, len));
                at += 12 + len;
            }
            let g = gpus[i.min(gpus.len() - 1)];
            streams.push(St {
                gpu: g,
                tex: BTreeMap::new(),
                bufs: BTreeMap::new(),
                stos: BTreeMap::new(),
                pipes: BTreeMap::new(),
                cpipes: BTreeMap::new(),
                codes: BTreeMap::new(),
                allocs: BTreeMap::new(),
                cmds: BTreeMap::new(),
                open: Vec::new(),
                frames: BTreeMap::new(),
                target: g.engine_target(),
                depth: g.engine_depth(),
                blits: BTreeMap::new(),
            });
        }
        // One counter for every stream: the order the calls were made in.
        order.sort_by_key(|r| r.0);
        Ok(Replay { log, streams, order, data, summary: Summary::default(), said: BTreeMap::new(), engine_color: false, engine_depth: false, blit })
    }

    /// Replays every record.
    /// Replays every record before `stop` (`seq`; u32::MAX: all of them).
    pub fn run(&mut self, blob: &dyn Fn(Blob) -> Option<Vec<u8>>, stop: u32) {
        let order = core::mem::take(&mut self.order);
        let data = core::mem::take(&mut self.data);
        for &(seq, s, op, at, len) in &order {
            if seq >= stop {
                break;
            }
            let mut r = R::new(&data[s][at..at + len]);
            self.summary.records += 1;
            let ok = self.one(s, op, &mut r, blob);
            if r.bad {
                self.summary.bad += 1;
            }
            if !ok {
                self.summary.skipped += 1;
                let n = self.said.entry(op).or_insert(0);
                *n += 1;
                if *n <= 3 {
                    (self.log)(format_args!("replay: record {} (op {}) of stream {} could not be replayed", self.summary.records, op, s));
                }
            }
        }
        for st in &mut self.streams {
            while let Some(c) = st.open.pop() {
                st.cmds.remove(&c);
            }
            st.cmds.clear();
        }
    }

    /// The engine's back buffer now (the frame as it would be presented), as 8-bit RGB
    /// rows top-down.
    pub fn picture(&self) -> Result<(u32, u32, Vec<u8>), &'static str> {
        let st = self.streams.iter().find(|s| s.target.is_some()).ok_or("no engine target")?;
        let t = st.target.as_ref().ok_or("no engine target")?;
        st.gpu.read_back(t, |m| {
            let bpp = m.format.bytes().unwrap_or(4);
            let mut out = Vec::with_capacity(m.width as usize * m.height as usize * 3);
            for y in 0..m.height {
                for px in m.row(0, y, m.width).chunks_exact(bpp) {
                    out.extend_from_slice(&m.format.rgb_u8(px));
                }
            }
            (m.width, m.height, out)
        })
    }

    fn frame(st: &mut St, id: u32) -> *mut Frame {
        *st.frames.entry(id).or_insert_with(|| Box::leak(Box::new(Frame::new())) as *mut Frame)
    }

    /// `f` with the `Cmd` begun last in the stream, or one of its own when none is open.
    fn with_cmd(st: &mut St, f: impl FnOnce(&mut Cmd)) {
        if let Some(&c) = st.open.last() {
            if let Some(cmd) = st.cmds.get_mut(&c) {
                f(cmd);
                return;
            }
        }
        let fp = Self::frame(st, 0);
        if let Some(mut cmd) = st.gpu.begin(unsafe { &mut *fp }) {
            f(&mut cmd);
        }
    }

    fn texture(st: &St, id: u32) -> Option<Texture> {
        st.tex.get(&id).map(|t| t.t.clone())
    }

    /// The texture `id` as a target: a stand-in made into the target it stands for, its
    /// first level (each face's, of a cube) drawn from the snapshot.
    fn promote(&mut self, s: usize, id: u32, blob: Loader) -> Option<Texture> {
        let blit = self.blit;
        let st = &mut self.streams[s];
        let e = st.tex.get(&id)?;
        let Some(desc) = e.target else { return Some(e.t.clone()) };
        let (seed, b) = (e.t.clone(), e.blob);
        let real = st.gpu.texture(desc, Contents::None);
        let mut ok = false;
        if desc.samples <= 1 && !desc.format.is_depth() {
            match desc.kind {
                TextureKind::D2 => ok = Self::blit(st, blit, &seed, &real, 0),
                TextureKind::Cube => {
                    // Each face's first level as a 2D texture of its own (a cube's
                    // contents are its faces, each with its levels).
                    let bpp = desc.format.bytes().unwrap_or(4);
                    let face: usize = (0..desc.level_count()).map(|l| ((desc.width >> l).max(1) as usize).pow(2) * bpp).sum();
                    let l0 = desc.width as usize * desc.height as usize * bpp;
                    if let Some(v) = Self::contents(blob, b) {
                        ok = v.len() >= face * 6;
                        for f in 0..6 {
                            if !ok {
                                break;
                            }
                            let px = v[f * face..f * face + l0].to_vec();
                            let t = st.gpu.texture(TextureDesc::d2(desc.width, desc.height, desc.format, Usage::SAMPLED), Contents::Owned(px));
                            ok = Self::blit(st, blit, &t, &real, f as u32);
                        }
                    }
                }
                TextureKind::D3 => {}
            }
            if ok && desc.usage.has(Usage::GEN_MIPS) {
                let r2 = real.clone();
                Self::with_cmd(st, |c| c.generate_mips(&r2));
            }
        }
        if !ok {
            (self.log)(format_args!(
                "replay: texture {} ({}x{} {} {:?}) is drawn into: made a target, without what the frame found in it",
                id, desc.width, desc.height, desc.format.name(), desc.kind
            ));
        }
        let st = &mut self.streams[s];
        st.tex.insert(id, Tex { t: real.clone(), target: None, blob: Blob::default() });
        self.summary.promoted += 1;
        Some(real)
    }

    /// `src` drawn into face `layer` of `dst` (its first level) texel for texel; a copy
    /// without the replayer's blit. False when neither could.
    fn blit(st: &mut St, blit: Option<(Code, Code)>, src: &Texture, dst: &Texture, layer: u32) -> bool {
        let d = dst.desc();
        let Some((vs, ps)) = blit else {
            let mut ok = false;
            Self::with_cmd(st, |c| ok = c.copy(src, None, dst, None, Filter::Point));
            return ok;
        };
        if d.samples > 1 || d.format.is_depth() {
            return false;
        }
        let gpu = st.gpu;
        let p = st.blits.entry(d.format as u32).or_insert_with(|| gpu.pipeline(PipelineDesc::new(vs, ps, &BLIT_LAYOUT, 16, Topology::TriangleStrip, d.format))).clone();
        // Direct3D 9's pixel centres are on whole coordinates: the quad moves half a
        // pixel up and left, so each pixel's centre samples its texel's.
        let hp = gpu.half_pixel();
        let (ox, oy) = (-hp * 2.0 / d.width as f32, hp * 2.0 / d.height as f32);
        let v: [[f32; 4]; 4] = [[-1.0 + ox, 1.0 + oy, 0.0, 0.0], [1.0 + ox, 1.0 + oy, 1.0, 0.0], [-1.0 + ox, -1.0 + oy, 0.0, 1.0], [1.0 + ox, -1.0 + oy, 1.0, 1.0]];
        let mut ok = false;
        Self::with_cmd(st, |c| {
            let colors = [ColorTarget { texture: dst, load: Load::DontCare, layer }];
            if !c.begin_pass(&Pass { colors: &colors, depth: None, area: None }) {
                return;
            }
            c.set_viewport(&Viewport::of(Rect::new(0, 0, d.width, d.height)));
            if c.set_pipeline(&p) {
                c.bind_texture(0, Some(src), &Sampler::POINT_CLAMP);
                if let Some(sp) = c.vertices(&v) {
                    c.draw(&sp);
                    ok = true;
                }
            }
            c.end_pass();
        });
        ok
    }

    fn code(st: &St, id: u32) -> &'static [u32] {
        if id == 0 {
            return &[];
        }
        st.codes.get(&id).copied().unwrap_or(&[])
    }

    fn contents(blob: &dyn Fn(Blob) -> Option<Vec<u8>>, b: Blob) -> Option<Vec<u8>> {
        if b.len == 0 {
            return None;
        }
        blob(b)
    }

    /// The engine's back buffer as the frame found it: a clear when it was one colour,
    /// else a copy of the snapshot.
    fn engine_init(st: &mut St, blit: Option<(Code, Code)>, desc: &TextureDesc, bytes: &[u8]) {
        let Some(t) = st.target.clone() else { return };
        let bpp = desc.format.bytes().unwrap_or(4);
        let first = &bytes[..bpp.min(bytes.len())];
        let one = bpp >= 3 && bytes.chunks_exact(bpp).all(|p| p[..3] == first[..3]);
        if one {
            let c = [first[2] as f32 / 255.0, first[1] as f32 / 255.0, first[0] as f32 / 255.0, 1.0];
            Self::with_cmd(st, |cmd| {
                let colors = [ColorTarget::new(&t, Load::Clear(c))];
                if cmd.begin_pass(&Pass { colors: &colors, depth: None, area: None }) {
                    cmd.end_pass();
                }
            });
        } else {
            let seed = st.gpu.texture(TextureDesc::d2(desc.width, desc.height, desc.format, Usage::SAMPLED | Usage::COPY_SRC), Contents::Owned(bytes.to_vec()));
            Self::blit(st, blit, &seed, &t, 0);
        }
    }

    fn engine_depth_init(st: &mut St) {
        let (Some(t), Some(d)) = (st.target.clone(), st.depth.clone()) else { return };
        Self::with_cmd(st, |cmd| {
            let colors = [ColorTarget::new(&t, Load::Load)];
            if cmd.begin_pass(&Pass { colors: &colors, depth: Some(DepthTarget { texture: &d, load: Load::Clear(1.0) }), area: None }) {
                cmd.end_pass();
            }
        });
    }

    fn load_color(r: &mut R) -> Load<[f32; 4]> {
        let k = r.u8();
        let v = r.floats();
        match k {
            1 if v.len() == 4 => Load::Clear([v[0], v[1], v[2], v[3]]),
            2 => Load::DontCare,
            _ => Load::Load,
        }
    }

    /// One record; false when it could not be replayed.
    fn one(&mut self, s: usize, op: u16, r: &mut R, blob: &dyn Fn(Blob) -> Option<Vec<u8>>) -> bool {
        match op {
            OP_CODE => {
                let id = r.u32();
                let n = r.u32() as usize;
                let w = words(r.take(n * 4));
                self.streams[s].codes.insert(id, leak_words(w));
            }
            OP_TEX_NEW => {
                let (id, desc, b) = (r.u32(), r.tex_desc(), r.blob());
                let st = &mut self.streams[s];
                let c = match Self::contents(blob, b) {
                    Some(v) => Contents::Owned(v),
                    None if b.len > 0 => return false,
                    None => Contents::None,
                };
                let t = st.gpu.texture(desc, c);
                st.tex.insert(id, Tex { t, target: None, blob: Blob::default() });
            }
            OP_TEX_SNAP => {
                let (id, desc, role, b) = (r.u32(), r.tex_desc(), r.u32(), r.blob());
                let first_color = role == ROLE_ENGINE_COLOR && !core::mem::replace(&mut self.engine_color, true);
                let first_depth = role == ROLE_ENGINE_DEPTH && !core::mem::replace(&mut self.engine_depth, true);
                let bytes = if role == ROLE_ENGINE_COLOR && !first_color { None } else { Self::contents(blob, b) };
                let blit = self.blit;
                let st = &mut self.streams[s];
                match role {
                    ROLE_ENGINE_COLOR => {
                        let Some(t) = st.target.clone() else { return false };
                        if let Some(v) = &bytes {
                            Self::engine_init(st, blit, &desc, v);
                        }
                        st.tex.insert(id, Tex { t, target: None, blob: Blob::default() });
                    }
                    ROLE_ENGINE_DEPTH => {
                        let Some(t) = st.depth.clone() else { return false };
                        if first_depth {
                            Self::engine_depth_init(st);
                        }
                        st.tex.insert(id, Tex { t, target: None, blob: Blob::default() });
                    }
                    _ => {
                        let drawn = Usage(Usage::RENDER_TARGET.0 | Usage::DEPTH.0 | Usage::STORAGE.0);
                        let is_target = desc.usage.0 & drawn.0 != 0 || desc.samples > 1;
                        let e = match bytes {
                            Some(v) if is_target => {
                                // The snapshot as a sampled texture; the target at its first draw.
                                let mut sd = desc;
                                sd.usage = Usage::SAMPLED | Usage::COPY_SRC;
                                sd.samples = 1;
                                Tex { t: st.gpu.texture(sd, Contents::Owned(v)), target: Some(desc), blob: b }
                            }
                            Some(v) => {
                                let mut sd = desc;
                                // A mirror's: Display8's image, sampled and copied from.
                                if role == ROLE_MIRROR {
                                    sd.usage = Usage::SAMPLED | Usage::COPY_SRC;
                                }
                                Tex { t: st.gpu.texture(sd, Contents::Owned(v)), target: None, blob: Blob::default() }
                            }
                            None => Tex { t: st.gpu.texture(desc, Contents::None), target: None, blob: Blob::default() },
                        };
                        st.tex.insert(id, e);
                    }
                }
            }
            OP_TEX_WRITE => {
                let (id, pitch, w, rows) = (r.u32(), r.u32(), r.u32(), r.u32());
                let data = r.bytes();
                let Some(t) = Self::texture(&self.streams[s], id) else { return false };
                if data.len() < pitch as usize * rows as usize {
                    return false;
                }
                t.write(data.as_ptr(), pitch as usize, w, rows);
            }
            OP_BUF_NEW | OP_BUF_SNAP => {
                let (id, desc, b) = (r.u32(), r.buf_desc(), r.blob());
                let v = Self::contents(blob, b).unwrap_or_default();
                if b.len > 0 && v.is_empty() {
                    return false;
                }
                let st = &mut self.streams[s];
                let buf = st.gpu.buffer(desc, &v);
                st.bufs.insert(id, buf);
            }
            OP_STO_NEW | OP_STO_SNAP => {
                let (id, size, stride) = (r.u32(), r.u32(), r.u32());
                let b = if op == OP_STO_SNAP { r.blob() } else { Blob::default() };
                let st = &mut self.streams[s];
                let sto = st.gpu.storage(StorageDesc { size, stride });
                if let Some(v) = Self::contents(blob, b) {
                    let s2 = sto.clone();
                    Self::with_cmd(st, |c| c.write_storage(&s2, 0, &v));
                }
                st.stos.insert(id, sto);
            }
            OP_PIPE => {
                let id = r.u32();
                let (codes, layout, mut d) = r.pipe_desc();
                let st = &mut self.streams[s];
                let c = |i: usize| Self::code(st, codes[i]);
                d.vs = Code::with_dxil_at(c(0), c(1), c(2), c(3));
                d.ps = Code::with_dxil_at(c(4), c(5), c(6), c(7));
                d.layout = Box::leak(layout.into_boxed_slice());
                let p = st.gpu.pipeline(d);
                st.pipes.insert(id, p);
            }
            OP_CPIPE => {
                let id = r.u32();
                let codes = [r.u32(), r.u32(), r.u32(), r.u32()];
                let st = &mut self.streams[s];
                let c = |i: usize| Self::code(st, codes[i]);
                let p = st.gpu.compute_pipeline(ComputeDesc { cs: Code::with_dxil_at(c(0), c(1), c(2), c(3)) });
                st.cpipes.insert(id, p);
            }
            OP_FRAME_NEXT => {
                let id = r.u32();
                let f = Self::frame(&mut self.streams[s], id);
                unsafe { (*f).next() };
            }
            OP_BEGIN => {
                let (cmd, fid) = (r.u32(), r.u32());
                // The viewport the `Cmd` started with in the game: the engine's.
                let vp = if r.u8() != 0 { Some(Viewport { x: r.u32(), y: r.u32(), w: r.u32(), h: r.u32(), min_z: r.f32(), max_z: r.f32() }) } else { None };
                let st = &mut self.streams[s];
                let f = Self::frame(st, fid);
                let Some(mut c) = st.gpu.begin(unsafe { &mut *f }) else { return false };
                if let Some(v) = vp {
                    c.set_viewport(&v);
                }
                st.cmds.insert(cmd, c);
                st.open.push(cmd);
            }
            OP_END => {
                let cmd = r.u32();
                let st = &mut self.streams[s];
                st.open.retain(|&c| c != cmd);
                return st.cmds.remove(&cmd).is_some();
            }
            OP_END_STREAM | OP_READ_BACK => {}
            _ => return self.command(s, op, r, blob),
        }
        true
    }

    /// A command recorded into a `Cmd`.
    fn command(&mut self, s: usize, op: u16, r: &mut R, blob: Loader) -> bool {
        let cmd = r.u32();
        if !self.streams[s].cmds.contains_key(&cmd) {
            return false;
        }
        // Targets first: a stand-in becomes its target before the command draws into it.
        match op {
            OP_BEGIN_PASS => {
                let n = r.u32() as usize;
                let mut colors: Vec<(Texture, u32, Load<[f32; 4]>)> = Vec::new();
                for _ in 0..n.min(MAX_COLORS) {
                    let (id, layer) = (r.u32(), r.u32());
                    let load = Self::load_color(r);
                    let Some(t) = self.promote(s, id, blob) else { return false };
                    colors.push((t, layer, load));
                }
                let depth = if r.u8() != 0 {
                    let id = r.u32();
                    let k = r.u8();
                    let v = r.f32();
                    let load = match k {
                        1 => Load::Clear(v),
                        2 => Load::DontCare,
                        _ => Load::Load,
                    };
                    match self.promote(s, id, blob) {
                        Some(t) => Some((t, load)),
                        None => return false,
                    }
                } else {
                    None
                };
                let area = r.opt_rect();
                let cts: Vec<ColorTarget> = colors.iter().map(|(t, layer, load)| ColorTarget { texture: t, load: *load, layer: *layer }).collect();
                let pass = Pass { colors: &cts, depth: depth.as_ref().map(|(t, l)| DepthTarget { texture: t, load: *l }), area };
                let c = self.streams[s].cmds.get_mut(&cmd).unwrap();
                c.begin_pass(&pass);
                return true;
            }
            OP_GEN_MIPS => {
                let id = r.u32();
                let Some(t) = self.promote(s, id, blob) else { return false };
                self.streams[s].cmds.get_mut(&cmd).unwrap().generate_mips(&t);
                return true;
            }
            OP_RESOLVE_DEPTH => {
                let (a, b) = (r.u32(), r.u32());
                let (Some(src), Some(dst)) = (Self::texture(&self.streams[s], a), self.promote(s, b, blob)) else { return false };
                self.streams[s].cmds.get_mut(&cmd).unwrap().resolve_depth(&src, &dst);
                return true;
            }
            OP_RESOLVE => {
                let (a, b, rc) = (r.u32(), r.u32(), r.rect());
                let (Some(src), Some(dst)) = (Self::texture(&self.streams[s], a), self.promote(s, b, blob)) else { return false };
                self.streams[s].cmds.get_mut(&cmd).unwrap().resolve(&src, &dst, rc);
                return true;
            }
            OP_COPY => {
                let (a, sr, b, dr) = (r.u32(), r.opt_rect(), r.u32(), r.opt_rect());
                let f = r.filter();
                let (Some(src), Some(dst)) = (Self::texture(&self.streams[s], a), self.promote(s, b, blob)) else { return false };
                self.streams[s].cmds.get_mut(&cmd).unwrap().copy(&src, sr, &dst, dr, f);
                return true;
            }
            OP_BIND_STORAGE_IMAGE => {
                let (slot, id) = (r.u32(), r.u32());
                let t = if id == 0 {
                    None
                } else {
                    match self.promote(s, id, blob) {
                        Some(t) => Some(t),
                        None => return false,
                    }
                };
                self.streams[s].cmds.get_mut(&cmd).unwrap().bind_storage_image(slot, t.as_ref());
                return true;
            }
            _ => {}
        }
        let st = &mut self.streams[s];
        let St { cmds, tex, bufs, stos, pipes, cpipes, allocs, .. } = st;
        let c = cmds.get_mut(&cmd).unwrap();
        match op {
            OP_END_PASS => c.end_pass(),
            OP_VIEWPORT => {
                let vp = Viewport { x: r.u32(), y: r.u32(), w: r.u32(), h: r.u32(), min_z: r.f32(), max_z: r.f32() };
                c.set_viewport(&vp);
            }
            OP_SCISSOR => c.set_scissor(r.rect()),
            OP_CLEAR_RECTS => {
                let n = r.u32() as usize;
                let mut rects = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    if r.bad {
                        break;
                    }
                    rects.push(r.rect());
                }
                let color = if r.u8() != 0 {
                    let v = r.floats();
                    (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
                } else {
                    None
                };
                let depth = if r.u8() != 0 { Some(r.f32()) } else { None };
                c.clear_rects(&rects, color, depth);
            }
            OP_SET_PIPELINE => {
                let Some(p) = pipes.get(&r.u32()) else { return false };
                c.set_pipeline(p);
            }
            OP_SET_SAMPLER => {
                let slot = r.u32();
                c.set_sampler(slot, &r.sampler());
            }
            OP_BIND_TEXTURE => {
                let (slot, id) = (r.u32(), r.u32());
                let smp = r.sampler();
                let t = if id == 0 {
                    None
                } else {
                    match tex.get(&id) {
                        Some(t) => Some(t.t.clone()),
                        None => return false,
                    }
                };
                c.bind_texture(slot, t.as_ref(), &smp);
            }
            OP_VS_BLOCK | OP_PS_BLOCK | OP_CS_BLOCK => {
                let base = r.u32();
                let f = r.floats();
                let b = Block::new(base, &f);
                match op {
                    OP_VS_BLOCK => c.vs_block(b),
                    OP_PS_BLOCK => c.ps_block(b),
                    _ => c.cs_block(b),
                }
            }
            OP_BIND_STORAGE | OP_BIND_STORAGE_RW => {
                let (slot, id) = (r.u32(), r.u32());
                let st = if id == 0 { None } else { stos.get(&id) };
                if id != 0 && st.is_none() {
                    return false;
                }
                if op == OP_BIND_STORAGE {
                    c.bind_storage(slot, st);
                } else {
                    c.bind_storage_rw(slot, st);
                }
            }
            OP_WRITE_STORAGE => {
                let (id, offset) = (r.u32(), r.u32());
                let data = r.bytes();
                let Some(st) = stos.get(&id) else { return false };
                c.write_storage(st, offset, data);
            }
            OP_DISPATCH => {
                let id = r.u32();
                let (x, y, z) = (r.u32(), r.u32(), r.u32());
                let Some(p) = cpipes.get(&id) else { return false };
                c.dispatch(p, x, y, z);
                self.summary.dispatches += 1;
            }
            OP_BARRIER => c.barrier(),
            OP_VERTICES | OP_INDICES | OP_SPAN_OF => {
                let (alloc, stride) = (r.u32(), r.u32());
                let got = match op {
                    OP_VERTICES => {
                        let b = r.bytes();
                        vertices(c, b, stride)
                    }
                    OP_INDICES => {
                        let b = r.bytes();
                        let idx: Vec<u16> = b.chunks_exact(2).map(|x| u16::from_le_bytes([x[0], x[1]])).collect();
                        c.indices(&idx)
                    }
                    _ => {
                        let id = r.u32();
                        let Some(b) = bufs.get(&id) else { return false };
                        c.span_of(b, stride)
                    }
                };
                let wanted = r.bool();
                match got {
                    Some(sp) => {
                        allocs.insert(alloc, sp);
                    }
                    None => return !wanted,
                }
            }
            OP_DRAW => {
                let Some(sp) = St::spans(allocs, r) else { return false };
                c.draw(&sp);
                self.summary.draws += 1;
            }
            OP_DRAW_INDEXED => {
                let st2 = St::spans(allocs, r);
                let (bv, nv) = (r.u32(), r.u32());
                let st3 = St::spans(allocs, r);
                let (first, count) = (r.u32(), r.u32());
                let (Some(v), Some(i)) = (st2, st3) else { return false };
                c.draw_indexed(&v, bv, nv, &i, first, count);
                self.summary.draws += 1;
            }
            OP_DRAW_BUFFER => {
                let (id, stride, first, count) = (r.u32(), r.u32(), r.u32(), r.u32());
                let Some(b) = bufs.get(&id) else { return false };
                c.draw_buffer(b, stride, first, count);
                self.summary.draws += 1;
            }
            _ => return false,
        }
        true
    }
}

impl St {
    /// The span a draw reads, from the span written it is part of.
    fn spans(allocs: &BTreeMap<u32, Span>, r: &mut R) -> Option<Span> {
        let (id, delta, count, stride) = (r.u32(), r.u32(), r.u32(), r.u32());
        let mut sp = *allocs.get(&id)?;
        if sp.stride != stride {
            return None;
        }
        sp.first += delta;
        sp.count = count;
        Some(sp)
    }
}
