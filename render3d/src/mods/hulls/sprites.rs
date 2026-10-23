//! A hull's SOD sprite nodes (running lights and strobes, crew-damage fires) and the
//! billboard effect pinned to an object, drawn as the engine draws them
//! (`ST3D_SpriteNode::RenderInternal`, `ST3D_Instance::RenderSpriteNodeEffect`;
//! `spec-particles.md` sections 8.2-8.4):
//!
//! - a node's name up to its first underscore is a `@sprite_node`; its animation
//!   (`draw`, `colour`) plays on the instance's clock (from its start time, with its
//!   play flags), so every ship's strobes blink in phase, as stock;
//! - a billboard is a quad parallel to the image plane at the node, its size a half
//!   extent times the instance's scale; any other node is a quad of the full size in
//!   the node's own x-y plane;
//! - the sprite's own `@anim` (a flipbook) runs on the same clock;
//! - `additive` sprites are added, `alphathreshold` ones cut out at alpha 0.5, others
//!   blended by their alpha.
//!
//! They are collected while the hulls draw and drawn at the flush, after the faded and
//! cloaked hulls, where stock's depth sort puts them.
use super::look::Look;
use super::{anim, Posed};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use gpu::{attr, AttrFormat, Blend, Block, Compare, Depth, Format, Pipeline, PipelineDesc, Semantic, Topology, VertexAttr};
use crate::math::{v3, V3};
use crate::shaders::hulls::*;
use crate::shaders::hulls_spv::*;
use crate::shaders::hulls_dxil::*;
use crate::assets;
use a2fmt::sod::NodeKind;
use a2fmt::spr::SpriteTable;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Add,
    Cut,
    Alpha,
}

/// One sprite quad, its corners in world space.
pub struct Quad {
    pub tex: String,
    pub kind: Kind,
    pub corners: [V3; 4],
    pub uv: [[f32; 2]; 4],
    pub colour: [f32; 4],
    pub depth: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SVert {
    pos: [f32; 3],
    col: [f32; 4],
    uv: [f32; 2],
}

static SPRITE_LAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float4, Semantic::Color, 0),
    attr(28, AttrFormat::Float2, Semantic::TexCoord, 0),
];

pub struct Sprites {
    pipes: Pipes,
}

fn kind_of(material: &str) -> Kind {
    match material {
        "additive" | "add_nomipf" | "" => Kind::Add,
        "alphathreshold" => Kind::Cut,
        _ => Kind::Alpha,
    }
}

/// A sprite's rectangle in 0..1 units, its own flipbook offset added.
fn sprite_uv(t: &SpriteTable, sprite: &a2fmt::spr::Sprite, time: f32) -> [f32; 4] {
    let r = sprite.uv_rect().unwrap_or_else(|| {
        let rw = sprite.reference[0].unwrap_or(256.0).max(1.0);
        let rh = sprite.reference[1].unwrap_or(rw).max(1.0);
        [sprite.rect[0] / rw, sprite.rect[1] / rh, sprite.rect[2] / rw, sprite.rect[3] / rh]
    });
    let mut off = [0.0f32; 2];
    for id in &sprite.anims {
        if let Some(a) = t.animation(id) {
            let o = anim::offset_of(anim::value(a, time, anim::LOOP));
            off[0] += o[0];
            off[1] += o[1];
        }
    }
    [r[0] + off[0], r[1] + off[1], r[2], r[3]]
}

impl Sprites {
    pub fn new() -> Sprites {
        Sprites { pipes: Pipes::new() }
    }

    /// A quad for a sprite node look (`@sprite_node` id) at a node's world matrix.
    #[allow(clippy::too_many_arguments)]
    fn node_quad(
        &self, v: &View, t: &SpriteTable, id: &str, at: &crate::math::M34, scale: f32, time: f32, flags: u32,
        colour_override: Option<V3>, boost: f32, half_override: Option<f32>, out: &mut Vec<Quad>,
    ) {
        let Some(nd) = t.sprite_node(id) else { return };
        let Some(sprite) = t.sprite(&nd.sprite) else { return };
        // The node data's animation: its draw flag and colour.
        let mut colour = nd.colour;
        if let Some(a) = nd.animation.as_deref().and_then(|a| t.animation(a)) {
            match anim::value(a, time, flags) {
                Some(anim::Value::Draw(false)) => return,
                Some(anim::Value::Colour(c)) => colour = c,
                _ => {}
            }
        }
        if let Some(c) = colour_override {
            colour = [c.x, c.y, c.z];
        }
        let uv = sprite_uv(t, sprite, time);
        let (u0, v0, u1, v1) = (uv[0], uv[1], uv[0] + uv[2], uv[1] + uv[3]);
        let kind = kind_of(&sprite.material);
        let pos = at.p;
        let corners = if nd.billboard || half_override.is_some() {
            // Parallel to the image plane: the camera's right and up axes.
            let vw = &v.view.0;
            let right = v3(vw[0][0], vw[1][0], vw[2][0]);
            let up = v3(vw[0][1], vw[1][1], vw[2][1]);
            let (hx, hy) = match half_override {
                Some(h) => (h, h),
                None => (nd.size[0] * scale, nd.size[1] * scale),
            };
            [pos - right * hx + up * hy, pos + right * hx + up * hy, pos + right * hx - up * hy, pos - right * hx - up * hy]
        } else {
            // In the node's x-y plane, the full size.
            let ax = super::norm_or(at.r[0], v3(1.0, 0.0, 0.0)) * scale;
            let ay = super::norm_or(at.r[1], v3(0.0, 1.0, 0.0)) * scale;
            let (hx, hy) = (nd.size[0] * 0.5, nd.size[1] * 0.5);
            [pos - ax * hx + ay * hy, pos + ax * hx + ay * hy, pos + ax * hx - ay * hy, pos - ax * hx - ay * hy]
        };
        let c = [colour[0] * boost, colour[1] * boost, colour[2] * boost, 1.0];
        out.push(Quad {
            tex: sprite.texture.clone(),
            kind,
            corners,
            uv: [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            colour: c,
            depth: v.view.transform_point(pos).z,
        });
    }

    /// The posed hull's shown sprite nodes.
    pub fn collect(&self, v: &View, look: &Look, p: &Posed, out: &mut Vec<Quad>) {
        let t = assets::sprites();
        let scale = p.scale.x;
        for (i, n) in p.model.sod.nodes.iter().enumerate() {
            if !matches!(n.kind, NodeKind::Sprite) || !p.pose.shown.get(i).copied().unwrap_or(false) {
                continue;
            }
            let id = n.sprite_node_id();
            let glow = if id.eq_ignore_ascii_case("glowdmoon") { p.glow } else { None };
            self.node_quad(v, &t, id, &p.pose.world[i], scale, p.since_start, p.flags, glow, look.sprites, None, out);
        }
    }

    /// The billboard effect pinned to the object: at its bounding sphere's centre
    /// pulled towards the camera by its radius, its half size the radius, looped.
    pub fn billboard(&self, v: &View, look: &Look, p: &Posed, name: &str, out: &mut Vec<Quad>) {
        let t = assets::sprites();
        let inst = p.inst;
        let (c, r) = super::game::instance_sphere(inst);
        let base = crate::engine::field_v3(inst, 0x68) + c;
        let to_eye = super::norm_or(v.eye - base, v3(0.0, 1.0, 0.0));
        let mut at = crate::math::M34::IDENTITY;
        at.p = base + to_eye * r;
        self.node_quad(v, &t, name, &at, 1.0, p.since_start, anim::LOOP, None, look.sprites, Some(r), out);
    }

    /// Every collected quad: the cut-out ones, then the blended and added ones far to
    /// near.
    pub fn draw(&mut self, v: &View, quads: &[Quad]) {
        if quads.is_empty() {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        let pipes: Option<[Pipeline; 3]> = (|| Some([self.pipe(Kind::Add)?, self.pipe(Kind::Cut)?, self.pipe(Kind::Alpha)?]))();
        let Some(pipes) = pipes.filter(|p| p.iter().all(|p| p.ready(gfx::gpu()))) else { return };
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.set_sampler(0, &crate::scene::SAMPLER);
        let mut order: Vec<usize> = (0..quads.len()).collect();
        order.sort_by(|&a, &b| {
            let ka = (quads[a].kind != Kind::Cut) as u8;
            let kb = (quads[b].kind != Kind::Cut) as u8;
            ka.cmp(&kb).then(quads[b].depth.partial_cmp(&quads[a].depth).unwrap_or(core::cmp::Ordering::Equal))
        });
        let mut last_kind = None;
        let mut last_tex: &str = "";
        for i in order {
            let q = &quads[i];
            if last_kind != Some(q.kind) {
                last_kind = Some(q.kind);
                let (p, k) = match q.kind {
                    Kind::Add => (&pipes[0], [1.0, 0.0, 0.0, 0.0]),
                    Kind::Cut => (&pipes[1], [1.0, 1.0, 0.0, 0.0]),
                    Kind::Alpha => (&pipes[2], [1.0, 0.0, 0.0, 0.0]),
                };
                cmd.set_pipeline(p);
                cmd.ps_block(Block::new(0, &k));
            }
            if q.tex != last_tex {
                cmd.bind_texture(0, assets::texture(&q.tex).as_ref(), &crate::scene::SAMPLER);
                last_tex = &q.tex;
            }
            let vs: [SVert; 6] = {
                let mk = |k: usize| SVert { pos: [q.corners[k].x, q.corners[k].y, q.corners[k].z], col: q.colour, uv: q.uv[k] };
                [mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)]
            };
            if let Some(s) = cmd.vertices(&vs) {
                cmd.draw(&s);
            }
        }
    }

    /// The sprites' pipeline for a kind: depth-tested, no culling; added, cut out
    /// (by the shader, writing depth) or blended by alpha.
    fn pipe(&mut self, kind: Kind) -> Option<Pipeline> {
        self.pipes.view(kind as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_SPRITE, VS_SPRITE_SPV, VS_SPRITE_DXIL), gpu::code!(PS_SPRITE, PS_SPRITE_SPV, PS_SPRITE_DXIL), &SPRITE_LAYOUT, 36, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: kind == Kind::Cut, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = match kind {
                Kind::Add => Some(Blend::ADD),
                Kind::Cut => None,
                Kind::Alpha => Some(Blend::ALPHA),
            };
            d
        })
    }
}
