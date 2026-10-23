//! The procedural sky baked into a cube once per map, and
//! the fine detail's noise volume. Both on the GPU only: each face is drawn by the
//! `bake_*_ps` chain (five passes, `gpu::chain`: one shader would be longer than some
//! cards take) into a target twice its edge, then `down_ps` averages each 2x2 into the
//! cube face, a render target itself. The cube is half floats, so the recipe's stars
//! can pass white; a `Reset` loses what a target holds, so it is baked again after.
use super::recipe::{NCONST, Recipe};
use crate::gfx::{self, Pipes};
use crate::log;
use crate::rt;
use crate::shaders::sky::*;
use crate::shaders::sky_spv::*;
use crate::shaders::sky_dxil::*;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use gpu::chain::{Chain, ChainDest, ChainPass};
use gpu::{
    attr, Address, AttrFormat, Bindings, Block, Cmd, ColorTarget, Contents, Filter, Format, Load, MipFilter, Pass,
    PipelineDesc, Sampler, Semantic, Texture, TextureDesc, Topology, Usage, VertexAttr,
};

/// Direct3D's cube faces in order (+x, -x, +y, -y, +z, -z): the direction at a face's
/// centre, and the directions its texel column and row grow along.
const FACES: [[[f32; 3]; 3]; 6] = [
    [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]],
    [[-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
    [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
    [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
    [[0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
];

/// A quad over the whole target: clip position and (unused here) ray.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QuadVert {
    pub pos: [f32; 3],
    pub ray: [f32; 3],
}

pub static QUAD_LAYOUT: [VertexAttr; 2] = [attr(0, AttrFormat::Float3, Semantic::Position, 0), attr(12, AttrFormat::Float3, Semantic::TexCoord, 0)];

/// A pipeline of `sky_vs` and `ps` over a `QuadVert` strip, writing a target of `fmt`
/// whole: no depth, no blending.
pub fn quad_desc(ps: gpu::Code, fmt: Format) -> PipelineDesc {
    PipelineDesc::new(gpu::code!(VS_SKY, VS_SKY_SPV, VS_SKY_DXIL), ps, &QUAD_LAYOUT, 24, Topology::TriangleStrip, fmt)
}

/// Draws a quad of `QuadVert`s, a strip.
pub fn draw_quad(cmd: &mut Cmd, quad: &[QuadVert; 4]) {
    if let Some(s) = cmd.vertices(quad) {
        cmd.draw(&s);
    }
}

pub struct Baker {
    pub cube: Option<Texture>,
    pub edge: u32,
    /// The recipe text the cube holds (a changed text bakes again).
    pub baked: Option<(String, String)>,
    pub fine: Option<Texture>,
    fine_failed: bool,
    failed: Option<String>,
    /// The chain's pipelines (the warp, the field, the gas, two slabs of stars, the last
    /// slab) and `down_ps`'s, by format.
    pipes: Pipes,
}

/// The chain's pixel shaders, in order.
const BAKE_PS: [gpu::Code; 5] = [gpu::code!(PS_BAKE_WARP, PS_BAKE_WARP_SPV, PS_BAKE_WARP_DXIL), gpu::code!(PS_BAKE_FIELD, PS_BAKE_FIELD_SPV, PS_BAKE_FIELD_DXIL), gpu::code!(PS_BAKE_GAS, PS_BAKE_GAS_SPV, PS_BAKE_GAS_DXIL), gpu::code!(PS_BAKE_STARS, PS_BAKE_STARS_SPV, PS_BAKE_STARS_DXIL), gpu::code!(PS_BAKE_END, PS_BAKE_END_SPV, PS_BAKE_END_DXIL)];

impl Baker {
    pub const fn new() -> Baker {
        Baker { cube: None, edge: 0, baked: None, fine: None, fine_failed: false, failed: None, pipes: Pipes::new() }
    }

    pub fn reset(&mut self) {
        self.cube = None;
        self.baked = None;
        self.edge = 0;
        self.failed = None;
    }

    /// Whether the cube holds this recipe now.
    pub fn holds(&self, r: &Recipe, edge: u32) -> bool {
        self.cube.is_some() && self.edge == edge && self.baked.as_ref().is_some_and(|(n, t)| *n == r.name && *t == r.text)
    }

    /// Bakes `r` into the cube. It draws into targets of its own: the caller begins its
    /// pass again after. One try per recipe text.
    pub fn bake(&mut self, cmd: &mut Cmd, r: &Recipe, edge: u32, dump: bool) -> bool {
        if self.failed.as_deref() == Some(r.text.as_str()) {
            return false;
        }
        let t0 = rt::now();
        let ok = self.bake_inner(cmd, r, edge, dump);
        log!(
            "sky: bake \"{}\" ({}) {}: six faces of {} in {:.0} ms",
            r.name, r.source, if ok { "done" } else { "FAILED" }, edge, (rt::now() - t0) * 1000.0
        );
        if ok {
            self.baked = Some((r.name.clone(), r.text.clone()));
        } else {
            self.failed = Some(r.text.clone());
            self.cube = None;
            self.baked = None;
        }
        ok
    }

    fn bake_inner(&mut self, cmd: &mut Cmd, r: &Recipe, edge: u32, dump: bool) -> bool {
        let g = gfx::gpu();
        let s = edge;
        let ss = if 2 * s <= 4096 { 2 } else { 1 };
        let big = s * ss;
        if self.cube.is_none() || self.edge != s {
            self.cube = None;
            for fmt in [Format::Rgba16F, Format::Rgb10A2, Format::Bgra8] {
                let c = g.texture(TextureDesc::cube(s, 1, fmt, Usage::RENDER_TARGET | Usage::SAMPLED | Usage::COPY_SRC), Contents::None);
                if c.ready(g) {
                    log!("sky: cube {}x{} in format {}", s, s, gpu::d3d9::format_code(fmt));
                    self.cube = Some(c);
                    break;
                }
            }
            self.edge = s;
        }
        let Some(cube) = self.cube.clone() else { return false };
        let fmt = cube.desc().format;
        let fc = gpu::d3d9::format_code(fmt) as u64;
        // Targets 0: the warp, then the stars of the first two slabs; 1: the field; 2: the gas.
        let spec: [(usize, &'static [usize], Option<usize>); 5] =
            [(0, &[], Some(0)), (1, &[0], Some(1)), (2, &[0, 1], Some(2)), (3, &[2], Some(0)), (4, &[2, 0], None)];
        let pls: Vec<gpu::Pipeline> = spec
            .iter()
            .map(|&(i, _, to)| {
                let f = if to.is_some() { gpu::chain::FORMAT } else { fmt };
                self.pipes.own((i as u64) << 16 | gpu::d3d9::format_code(f) as u64, || quad_desc(BAKE_PS[i], f))
            })
            .collect();
        let down = self.pipes.own(9 << 16 | fc, || quad_desc(gpu::code!(PS_DOWN, PS_DOWN_SPV, PS_DOWN_DXIL), fmt));
        if !pls.iter().chain(core::iter::once(&down)).all(|p| p.ready(g)) {
            return false;
        }
        let passes: Vec<ChainPass> = spec.iter().zip(pls.iter()).map(|(&(_, reads, to), p)| ChainPass { pipeline: p, reads, to }).collect();
        let Some(chain) = Chain::new(g, 3, big) else {
            log!("sky: no float targets for the bake");
            return false;
        };
        let sup = g.texture(TextureDesc::d2(big, big, fmt, Usage::RENDER_TARGET | Usage::SAMPLED), Contents::None);
        if !sup.ready(g) {
            return false;
        }
        let quad: [QuadVert; 4] = [
            QuadVert { pos: [-1.0, 1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [1.0, 1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [-1.0, -1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [1.0, -1.0, 0.0], ray: [0.0; 3] },
        ];
        let down_s = Sampler {
            min: if ss == 2 { Filter::Linear } else { Filter::Point },
            mag: Filter::Linear,
            mip: MipFilter::None,
            address_u: Address::Clamp,
            address_v: Address::Clamp,
            address_w: Address::Clamp,
            anisotropy: 1,
            srgb: false,
        };
        let mut ok = true;
        for (f, axes) in FACES.iter().enumerate() {
            // Pass 1: the samples, by the chain.
            let mut c = [0.0f32; 16];
            for i in 0..3 {
                c[i * 4..i * 4 + 3].copy_from_slice(&axes[i]);
            }
            // target pixel a -> face coordinate: a texel's samples at i -+ 1/4 (sky.hlsl)
            c[12] = 2.0 / (ss * (s - 1)) as f32;
            c[13] = if ss == 1 { -1.0 } else { -1.0 - 0.5 / (s - 1) as f32 };
            cmd.bind_texture(2, None, &down_s);
            let common = Bindings { textures: &[], vs: &[], ps: &[Block::new(0, r.floats()), Block::new(NCONST as u32, &c)] };
            let draw = |cmd: &mut Cmd| draw_quad(cmd, &quad);
            if !chain.draw(cmd, ChainDest { texture: &sup, layer: 0 }, big, &passes, &common, &draw) {
                ok = false;
                break;
            }
            // Pass 2: four samples into each texel of the face.
            let colors = [ColorTarget::face(&cube, f as u32, Load::Load)];
            if !cmd.begin_pass(&Pass { colors: &colors, depth: None, area: None }) || !cmd.set_pipeline(&down) {
                ok = false;
                break;
            }
            cmd.bind_texture(2, Some(&sup), &down_s);
            cmd.ps_block(Block::new(26, &[if ss == 2 { 1.0 / big as f32 } else { 0.5 / big as f32 }, 0.0, 0.0, 0.0]));
            draw_quad(cmd, &quad);
            cmd.bind_texture(2, None, &down_s);
            cmd.end_pass();
            if dump {
                dump_face(cmd, r, f, &cube, s, fmt);
            }
        }
        ok
    }

    /// The fine detail's noise (`Sky.asi`'s `fine_volume`): white noise blurred by a
    /// Gaussian of 1.2 texels with wrap-around, so it tiles and is smooth under
    /// trilinear filtering; L8 at 0.5 + z/8 for z in units of its own spread, with a
    /// box-filtered chain so an octave finer than a pixel fades to the mean. Made once,
    /// 300 KB, kept (made again on the device after a `Reset` from its copy).
    pub fn fine(&mut self) -> Option<Texture> {
        if let Some(f) = &self.fine {
            return Some(f.clone());
        }
        if self.fine_failed {
            return None;
        }
        let f = make_fine();
        if f.is_none() {
            self.fine_failed = true;
            log!("sky: no noise volume for the fine detail: the baked sky alone");
        } else {
            log!("sky: fine detail: noise volume made");
        }
        self.fine = f;
        self.fine.clone()
    }
}

const FINE_N: usize = 64;

fn fine_hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x9e37_79b1);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    x
}

/// One axis of the blur, src to dst; stride 1, N or N*N picks the axis.
fn fine_blur(src: &[f32], dst: &mut [f32], stride: usize) {
    const W: [f32; 5] = [1.0, 0.7066, 0.2494, 0.04394, 0.003866]; // exp(-k^2 / 2.88)
    let n3 = FINE_N * FINE_N * FINE_N;
    for i in 0..n3 {
        let c = (i / stride) % FINE_N;
        let base = i - c * stride;
        let mut s = W[0] * src[i];
        for (k, w) in W.iter().enumerate().skip(1) {
            s += w * (src[base + ((c + k) & (FINE_N - 1)) * stride] + src[base + ((c + FINE_N - k) & (FINE_N - 1)) * stride]);
        }
        dst[i] = s;
    }
}

fn make_fine() -> Option<Texture> {
    let n3 = FINE_N * FINE_N * FINE_N;
    let levels = 7u32; // 64 .. 1
    let mut a: Vec<f32> = (0..n3).map(|i| (fine_hash(i as u32 + 0x51) >> 8) as f32 / 16_777_216.0 - 0.5).collect();
    let mut b = vec![0.0f32; n3];
    fine_blur(&a, &mut b, 1);
    fine_blur(&b, &mut a, FINE_N);
    fine_blur(&a, &mut b, FINE_N * FINE_N);
    let sd = crate::math::sqrtf(b.iter().map(|v| v * v).sum::<f32>() / n3 as f32);
    let mut cur: Vec<f32> = b.iter().map(|v| 127.5 + v / sd * 32.0).collect();
    let mut n = FINE_N;
    let mut px: Vec<u8> = Vec::with_capacity(n3 + n3 / 7 + 1);
    for _ in 0..levels {
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    let v = cur[(z * n + y) * n + x];
                    px.push(if v < 0.0 { 0 } else if v > 255.0 { 255 } else { (v + 0.5) as u8 });
                }
            }
        }
        if n > 1 {
            let h = n / 2;
            let mut next = vec![0.0f32; h * h * h];
            for z in 0..h {
                for y in 0..h {
                    for x in 0..h {
                        let mut s = 0.0;
                        for c in 0..2 {
                            for bb in 0..2 {
                                for aa in 0..2 {
                                    s += cur[((2 * z + c) * n + 2 * y + bb) * n + 2 * x + aa];
                                }
                            }
                        }
                        next[(z * h + y) * h + x] = s * 0.125;
                    }
                }
            }
            cur = next;
            n = h;
        }
    }
    let g = gfx::gpu();
    let f = FINE_N as u32;
    let t = g.texture(TextureDesc::d3(f, f, f, levels, Format::R8, Usage::SAMPLED), Contents::Owned(px));
    t.ready(g).then_some(t)
}

/// A half float to a float.
fn half(h: u16) -> f32 {
    let s = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = ((h >> 10) & 0x1f) as i32;
    let m = (h & 0x3ff) as f32;
    s * match e {
        0 => m * (1.0 / 16_777_216.0),
        31 => 65504.0,
        _ => (1.0 + m / 1024.0) * crate::math::powf(2.0, (e - 15) as f32),
    }
}

/// `Dump=1`: a baked face as a 24-bit TGA, `Sky\<name>.dump<f>.tga` in the game
/// directory, for `render3d/perf/sky-grade.py`.
fn dump_face(cmd: &mut Cmd, r: &Recipe, f: usize, cube: &Texture, s: u32, fmt: Format) {
    let mut out = Vec::with_capacity(18 + (s * s * 3) as usize);
    out.extend_from_slice(&[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&(s as u16).to_le_bytes());
    out.extend_from_slice(&(s as u16).to_le_bytes());
    out.extend_from_slice(&[24, 0x20]);
    let to8 = |v: f32| (crate::math::saturate(v) * 255.0 + 0.5) as u8;
    let read = cmd.read_back_layer(cube, f as u32, |m| {
        for y in 0..s as usize {
            let row = unsafe { m.ptr().add(y * m.pitch()) };
            for x in 0..s as usize {
                let (rr, gg, bb) = unsafe {
                    match fmt {
                        Format::Rgba16F => {
                            let p = row.add(x * 8) as *const u16;
                            (half(p.read_unaligned()), half(p.add(1).read_unaligned()), half(p.add(2).read_unaligned()))
                        }
                        Format::Rgb10A2 => {
                            let v = (row.add(x * 4) as *const u32).read_unaligned();
                            ((v & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, ((v >> 20) & 1023) as f32 / 1023.0)
                        }
                        _ => {
                            let v = (row.add(x * 4) as *const u32).read_unaligned();
                            (((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0)
                        }
                    }
                };
                out.extend_from_slice(&[to8(bb), to8(gg), to8(rr)]);
            }
        }
    });
    if read.is_err() {
        return;
    }
    let mut path = String::from("Sky\\");
    path.push_str(&r.name);
    path.push_str(".dump");
    path.push((b'0' + f as u8) as char);
    path.push_str(".tga");
    rt::write_file(&rt::game_path(&path), &out);
}
