//! The sprite batch: every quad of a view (particles, nebula puffs), grouped by
//! texture and material, drawn through one dynamic vertex buffer at
//! `Step::Translucent`, where stock draws its depth-sorted triangles. Additive draws
//! do not depend on order; the few translucent and cut-out ones are kept in the order
//! they were queued.
use super::look::Material;
use crate::assets;
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{M34, V3};
use crate::shaders::particles::*;
use crate::shaders::particles_spv::*;
use crate::shaders::particles_dxil::*;
use alloc::string::String;
use alloc::vec::Vec;
use gpu::{
    attr, Address, AlphaTest, AttrFormat, Blend, Block, Compare, Depth, Filter, Format, MipFilter, Pipeline,
    PipelineDesc, Semantic, Topology, VertexAttr,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}

static LAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(20, AttrFormat::Float4, Semantic::TexCoord, 1),
];

/// The camera of the view: its axes in the world and its position.
#[derive(Clone, Copy)]
pub struct Cam {
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    pub eye: V3,
    /// A pixel's size at unit distance.
    pub px: f32,
}

impl Cam {
    pub fn of(v: &View) -> Cam {
        let m = &v.view.0;
        let right = V3 { x: m[0][0], y: m[1][0], z: m[2][0] };
        let up = V3 { x: m[0][1], y: m[1][1], z: m[2][1] };
        let fwd = V3 { x: m[0][2], y: m[1][2], z: m[2][2] };
        let p5 = v.proj.0[1][1];
        let px = if p5 > 1e-6 { 2.0 / (p5 * v.rect.h.max(1) as f32) } else { 0.001 };
        Cam { right, up, fwd, eye: v.eye, px }
    }
}

pub struct Batch {
    groups: Vec<(Material, String, Vec<Vert>)>,
    last: usize,
    pipes: Pipes,
    pub quads: u32,
}

impl Batch {
    pub const fn new() -> Batch {
        Batch { groups: Vec::new(), last: 0, pipes: Pipes::new(), quads: 0 }
    }
    pub fn reset(&mut self) {
        self.groups.clear();
    }
    pub fn clear(&mut self) {
        for g in self.groups.iter_mut() {
            g.2.clear();
        }
        self.quads = 0;
    }

    fn group(&mut self, m: Material, tex: &str) -> &mut Vec<Vert> {
        // Few groups (one per texture and material): a search, the last one first.
        let hit = |g: &(Material, String, Vec<Vert>)| g.0 == m && g.1 == tex;
        let i = if self.groups.get(self.last).is_some_and(hit) {
            self.last
        } else if let Some(i) = self.groups.iter().position(hit) {
            i
        } else {
            self.groups.push((m, String::from(tex), Vec::new()));
            self.groups.len() - 1
        };
        self.last = i;
        &mut self.groups[i].2
    }

    /// A quad parallel to the image plane at `c`, half extents `hx`, `hy`.
    pub fn billboard(&mut self, cam: &Cam, m: Material, tex: &str, c: V3, hx: f32, hy: f32, uv: [f32; 4], col: [f32; 4]) {
        let r = cam.right * hx;
        let u = cam.up * hy;
        self.quad(m, tex, [c - r + u, c + r + u, c + r - u, c - r - u], uv, col);
    }

    /// A quad in a node's XY plane: (+-w/2, +-h/2, 0) through `world`.
    pub fn oriented(&mut self, m: Material, tex: &str, world: &M34, w: f32, h: f32, uv: [f32; 4], col: [f32; 4]) {
        let p = |x: f32, y: f32| world.transform_point(V3 { x, y, z: 0.0 });
        let (hw, hh) = (w * 0.5, h * 0.5);
        self.quad(m, tex, [p(-hw, hh), p(hw, hh), p(hw, -hh), p(-hw, -hh)], uv, col);
    }

    /// Corners clockwise from the top left.
    pub fn quad(&mut self, m: Material, tex: &str, c: [V3; 4], uv: [f32; 4], col: [f32; 4]) {
        // The rectangle's v runs up the quad: its bottom edge takes v, its top v + h.
        // Measured on the bench against the engine's own puffs at the same moment
        // (RMSE 0.002; the other way round 0.039).
        let t = [[uv[0], uv[1] + uv[3]], [uv[0] + uv[2], uv[1] + uv[3]], [uv[0] + uv[2], uv[1]], [uv[0], uv[1]]];
        let v = |i: usize| Vert { pos: [c[i].x, c[i].y, c[i].z], uv: t[i], col };
        let g = self.group(m, tex);
        g.extend_from_slice(&[v(0), v(1), v(2), v(0), v(2), v(3)]);
        self.quads += 1;
    }

    /// Draws everything queued, then empties the queue.
    pub fn draw(&mut self, v: &View) {
        if self.quads == 0 {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        if !self.pipe(Material::Additive).is_some_and(|p| p.ready(gfx::gpu())) {
            self.clear();
            return;
        }
        cmd.vs_block(Block::new(0, &gfx::vp_constants(v)));
        let groups = core::mem::take(&mut self.groups);
        for (m, tex, verts) in groups.iter() {
            if verts.is_empty() {
                continue;
            }
            let Some(t) = assets::texture(tex) else { continue };
            let Some(pipe) = self.pipe(*m) else { continue };
            cmd.set_pipeline(&pipe);
            let mip = if *m == Material::AdditiveNoMip { MipFilter::None } else { MipFilter::Linear };
            cmd.bind_texture(0, Some(&t), &gfx::sampler(Filter::Linear, mip, Address::Wrap, Address::Wrap));
            // In pieces, as the old dynamic buffer held them.
            for chunk in verts.chunks(6 * 8192) {
                if let Some(s) = cmd.vertices(chunk) {
                    cmd.draw(&s);
                }
            }
        }
        self.groups = groups;
        self.clear();
    }

    /// A material's pipeline: depth-tested (`LESSEQUAL`), red, green and blue written,
    /// no culling; added, blended by alpha, or cut out at alpha one half (writing depth).
    fn pipe(&mut self, m: Material) -> Option<Pipeline> {
        let key = match m {
            Material::Additive | Material::AdditiveNoMip => 0,
            Material::Translucent => 1,
            Material::Cutout => 2,
        };
        self.pipes.view(key, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_SPRITE, VS_SPRITE_SPV, VS_SPRITE_DXIL), gpu::code!(PS_SPRITE, PS_SPRITE_SPV, PS_SPRITE_DXIL, PS_SPRITE_AT_DXIL), &LAYOUT, 36, Topology::TriangleList, Format::Unknown);
            d.write_mask = 7;
            d.depth = Depth { test: true, write: key == 2, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = match key {
                0 => Some(Blend::ADD),
                1 => Some(Blend::ALPHA),
                _ => None,
            };
            d.alpha_test = if key == 2 { Some(AlphaTest::HALF) } else { None };
            d
        })
    }
}
