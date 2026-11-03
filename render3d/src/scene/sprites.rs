//! Stock's own sprites, drawn by us as stock draws them (the engine side is
//! `engine::sprite`): the ordnance module's shots and the effects module's sparks
//! queue quads here, in the order stock would have drawn them, and draw them batched by
//! texture and blend.
use crate::assets;
use crate::engine::sprite::{self as eng, BLEND_ONE};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{v3, V3};
use crate::shaders::ordnance::*;
use crate::shaders::ordnance_dxil::*;
use crate::shaders::ordnance_spv::*;
use alloc::string::String;
use alloc::vec::Vec;
use gpu::{attr, AttrFormat, Blend, BlendEq, BlendOp, Block, Compare, Depth, Factor, Format, Pipeline, PipelineDesc, Semantic, Topology, VertexAttr};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SVert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}

static SLAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(20, AttrFormat::Float4, Semantic::TexCoord, 1),
];

/// One quad waiting to be drawn, in the order stock would have drawn it.
struct Quad {
    tex: String,
    src: u32,
    dst: u32,
    v: [SVert; 4],
}

pub struct Sprites {
    quads: Vec<Quad>,
    pipes: Pipes,
    pub hot: f32,
    pub hot_from: f32,
    said: Vec<usize>,
}

impl Sprites {
    pub fn new() -> Sprites {
        Sprites {
            quads: Vec::new(),
            pipes: Pipes::new(),
            hot: 0.6,
            hot_from: 0.7,
            said: Vec::new(),
        }
    }

    pub fn reset(&mut self) {

        self.quads.clear();
    }

    /// The sprite's look now: texture, rectangle (after its animation), blend, flags.
    fn look(&mut self, sprite: usize) -> Option<(String, [f32; 4], u32, u32, u8)> {
        let l = eng::look(sprite)?;
        let r = l.rect;
        if !self.said.contains(&sprite) && self.said.len() < 64 {
            self.said.push(sprite);
            crate::log!(
                "ordnance: sprite {} ({}): rect {:.3} {:.3} {:.3} {:.3}, flags {:#x}, blend {} {}",
                l.texture, sprite, r[0], r[1], r[2], r[3], l.flags, l.src, l.dst
            );
        }
        Some((l.texture, r, l.src, l.dst, l.flags))
    }

    fn push(&mut self, sprite: usize, p: [V3; 4], col: V3) {
        let Some((tex, r, src, dst, flags)) = self.look(sprite) else { return };
        let (u0, v0) = (r[0], r[1]);
        let mut u1 = u0 + r[2];
        let v1 = v0 + r[3];
        if flags & 0x20 != 0 {
            // Tiled: the rectangle repeats along the ribbon, once per its width.
            let w = eng::tile_width(sprite);
            if w > 1e-6 {
                u1 = u0 + (p[0] - p[1]).len() / w;
            }
        }
        let c = eng::colour(col);
        let uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        let mut v = [SVert { pos: [0.0; 3], uv: [0.0; 2], col: c }; 4];
        for k in 0..4 {
            v[k].pos = [p[k].x, p[k].y, p[k].z];
            v[k].uv = uv[k];
        }
        self.quads.push(Quad { tex, src, dst, v });
    }

    /// `RenderAsPolygon(a, b, wa, wb)`: a quad from a to b, wa wide at a and wb at b,
    /// turned about its length to face the camera (its side is (a − eye) × (b − eye)).
    pub fn polygon(&mut self, v: &View, sprite: usize, a: V3, b: V3, wa: f32, wb: f32, col: V3) {
        let eye = eng::camera_position(v.cam);
        let s = (a - eye).cross(b - eye);
        let l = s.len();
        if !(l > 1e-12) {
            return;
        }
        let s = s * (1.0 / l);
        let (ha, hb) = (s * (wa * 0.5), s * (wb * 0.5));
        self.push(sprite, [a + ha, b + hb, b - hb, a - ha], col);
    }

    /// `DrawScaled3D(pos, size)`: a square of half size `size` facing the camera, in
    /// the camera's own axes (the first corner left and below).
    pub fn scaled(&mut self, v: &View, sprite: usize, pos: V3, size: f32, col: V3) {
        let mut s = (size, size);
        // An instance being drawn with a scale (Storm3D +0x100, its +0x28) scales it.
        if let Some(k) = eng::draw_scale() {
            s = (size * k, size * k);
        }
        let m = &v.view.0;
        let right = v3(m[0][0], m[1][0], m[2][0]);
        let up = v3(m[0][1], m[1][1], m[2][1]);
        let (rx, uy) = (right * s.0, up * s.1);
        self.push(sprite, [pos - rx - uy, pos + rx - uy, pos + rx + uy, pos - rx + uy], col);
    }

    /// Everything queued this step, in order, batched by texture and blend.
    pub fn draw(&mut self, v: &View) {
        if self.quads.is_empty() {
            return;
        }
        let quads = core::mem::take(&mut self.quads);
        let Some(mut cmd) = gfx::cmd() else { return };
        if !self.pipe(BLEND_ONE, BLEND_ONE).is_some_and(|p| p.ready(gfx::gpu())) {
            return;
        }
        cmd.vs_block(Block::new(0, &gfx::vp_constants(v)));
        cmd.ps_block(Block::new(0, &[self.hot, self.hot_from, 0.0, 0.0]));
        cmd.set_sampler(0, &crate::scene::SAMPLER);
        let mut i = 0;
        while i < quads.len() {
            // A run of quads that share a texture and a blend: one draw.
            let q0 = &quads[i];
            let mut j = i + 1;
            while j < quads.len() && quads[j].tex == q0.tex && quads[j].src == q0.src && quads[j].dst == q0.dst && j - i < 1000 {
                j += 1;
            }
            if let Some(tex) = assets::texture(&q0.tex) {
                let mut verts: Vec<SVert> = Vec::with_capacity((j - i) * 6);
                for q in &quads[i..j] {
                    for k in [0, 1, 2, 0, 2, 3] {
                        verts.push(q.v[k]);
                    }
                }
                if let Some(p) = self.pipe(q0.src, q0.dst) {
                    cmd.set_pipeline(&p);
                    cmd.bind_texture(0, Some(&tex), &crate::scene::SAMPLER);
                    if let Some(s) = cmd.vertices(&verts) {
                        cmd.draw(&s);
                    }
                }
            }
            i = j;
        }
    }

    /// The pipeline for a sprite material's blend (Direct3D's `D3DBLEND` codes, as the
    /// engine's material holds them): depth-tested and not written, no culling.
    fn pipe(&mut self, src: u32, dst: u32) -> Option<Pipeline> {
        self.pipes.view((src as u64) << 8 | dst as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_ORD_SPRITE, VS_ORD_SPRITE_SPV, VS_ORD_SPRITE_DXIL), gpu::code!(PS_ORD_SPRITE, PS_ORD_SPRITE_SPV, PS_ORD_SPRITE_DXIL), &SLAYOUT, 36, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = Some(blend_of(src, dst));
            d
        })
    }
}

/// A `D3DBLEND` pair as a blend. `BOTHSRCALPHA` (12) and `BOTHINVSRCALPHA` (13) as the
/// source set both factors, as Direct3D does.
fn blend_of(src: u32, dst: u32) -> Blend {
    let f = |c: u32| match c {
        1 => Factor::Zero,
        3 => Factor::SrcColor,
        4 => Factor::InvSrcColor,
        5 => Factor::SrcAlpha,
        6 => Factor::InvSrcAlpha,
        7 => Factor::DstAlpha,
        8 => Factor::InvDstAlpha,
        9 => Factor::DstColor,
        10 => Factor::InvDstColor,
        11 => Factor::SrcAlphaSat,
        _ => Factor::One,
    };
    let (s, d) = match src {
        12 => (Factor::SrcAlpha, Factor::InvSrcAlpha),
        13 => (Factor::InvSrcAlpha, Factor::SrcAlpha),
        _ => (f(src), f(dst)),
    };
    Blend { color: BlendEq { src: s, dst: d, op: BlendOp::Add }, alpha: None }
}
