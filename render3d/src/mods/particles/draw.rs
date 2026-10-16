//! The sprite batch: every quad of a view (particles, nebula puffs), grouped by
//! texture and material, drawn through one dynamic vertex buffer at
//! `Step::Translucent`, where stock draws its depth-sorted triangles. Additive draws
//! do not depend on order; the few translucent and cut-out ones are kept in the order
//! they were queued.
use super::look::Material;
use crate::assets;
use crate::d3d::*;
use crate::frame::steps::View;
use crate::gfx::{self, Decl, DynVb, Shader};
use crate::math::{M34, V3};
use crate::shaders::particles::*;
use alloc::string::String;
use alloc::vec::Vec;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}

static DECL: [VertexElement; 4] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    ve(0, 20, DECLTYPE_FLOAT4, DECLUSAGE_TEXCOORD, 1),
    VE_END,
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
    vb: DynVb,
    vs: Shader,
    ps: Shader,
    decl: Decl,
    pub quads: u32,
}

impl Batch {
    pub const fn new() -> Batch {
        Batch {
            groups: Vec::new(),
            last: 0,
            vb: DynVb::new(4 << 20),
            vs: Shader::vertex(&VS_SPRITE),
            ps: Shader::pixel(&PS_SPRITE),
            decl: Decl::new(&DECL),
            quads: 0,
        }
    }
    pub fn reset(&mut self) {
        self.vb.reset();
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
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            self.clear();
            return;
        }
        dev.vs_const(0, &gfx::vp_constants(v));
        for (s, val) in [
            (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
            (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 7), (RS_FILLMODE, 3), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_SRGBWRITEENABLE, 0), (RS_CULLMODE, CULL_NONE),
        ] {
            dev.set_rs(s, val);
        }
        dev.set_stream_freq(0, 1);
        dev.set_ss(0, SAMP_ADDRESSU, TADDRESS_WRAP);
        dev.set_ss(0, SAMP_ADDRESSV, TADDRESS_WRAP);
        dev.set_ss(0, SAMP_MAGFILTER, TEXF_LINEAR);
        dev.set_ss(0, SAMP_MINFILTER, TEXF_LINEAR);
        let groups = core::mem::take(&mut self.groups);
        for (m, tex, verts) in groups.iter() {
            if verts.is_empty() {
                continue;
            }
            let Some(t) = assets::texture(dev, tex) else { continue };
            dev.set_texture(0, t);
            dev.set_ss(0, SAMP_MIPFILTER, if *m == Material::AdditiveNoMip { TEXF_NONE } else { TEXF_LINEAR });
            match m {
                Material::Additive | Material::AdditiveNoMip => {
                    dev.set_rs(RS_ALPHABLENDENABLE, 1);
                    dev.set_rs(RS_SRCBLEND, BLEND_ONE);
                    dev.set_rs(RS_DESTBLEND, BLEND_ONE);
                    dev.set_rs(RS_ZWRITEENABLE, 0);
                    dev.set_rs(RS_ALPHATESTENABLE, 0);
                }
                Material::Translucent => {
                    dev.set_rs(RS_ALPHABLENDENABLE, 1);
                    dev.set_rs(RS_SRCBLEND, BLEND_SRCALPHA);
                    dev.set_rs(RS_DESTBLEND, BLEND_INVSRCALPHA);
                    dev.set_rs(RS_ZWRITEENABLE, 0);
                    dev.set_rs(RS_ALPHATESTENABLE, 0);
                }
                Material::Cutout => {
                    dev.set_rs(RS_ALPHABLENDENABLE, 0);
                    dev.set_rs(RS_ZWRITEENABLE, 1);
                    dev.set_rs(RS_ALPHATESTENABLE, 1);
                    dev.set_rs(RS_ALPHAREF, 128);
                    dev.set_rs(RS_ALPHAFUNC, CMP_GREATEREQUAL);
                }
            }
            // In pieces the buffer holds.
            for chunk in verts.chunks(6 * 8192) {
                if let Some(first) = self.vb.push(dev, chunk, core::mem::size_of::<Vert>() as u32) {
                    dev.draw_primitive(PT_TRIANGLELIST, first, chunk.len() as u32 / 3);
                }
            }
        }
        self.groups = groups;
        self.clear();
    }
}
