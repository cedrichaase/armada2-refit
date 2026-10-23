//! The background's sprites, drawn by us: the starfield's quads, read from the
//! engine's own starfield every frame, and the billboards of the background objects'
//! sprite nodes (the suns). Additive, as stock draws them, with the brightest texels
//! lifted a little past white so a star's or a sun's core blooms.
use crate::assets;
use crate::engine::{field, rd};
use crate::math::{M4, V3, v3};
use crate::rt;
use crate::shaders::sky::VS_SPRITE;
use crate::shaders::sky_spv::VS_SPRITE_SPV;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use gpu::{
    attr, Address, AttrFormat, Blend, Block, Cmd, Compare, Depth, Filter, Format, MipFilter, Pipeline, PipelineDesc,
    Sampler, Semantic, Topology, VertexAttr,
};

/// One vertex: world position, texture coordinate, colour (a: the lift past white).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SprVert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}

pub static SPRITE_LAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(20, AttrFormat::Float4, Semantic::Color, 0),
];

/// What a sprite draws: its texture and the rectangle of it, 0..1.
#[derive(Clone)]
pub struct SprInfo {
    pub texture: String,
    pub uv: [f32; 4],
}

/// A sprite by name, from the sprite table.
pub fn sprite_info(name: &str) -> Option<SprInfo> {
    let t = assets::sprites();
    let s = t.sprite(name)?;
    Some(SprInfo { texture: s.texture.clone(), uv: s.uv_rect().unwrap_or([0.0, 0.0, 1.0, 1.0]) })
}

/// Quads gathered for one draw, by texture.
#[derive(Default)]
pub struct Batch {
    pub by_tex: BTreeMap<String, Vec<SprVert>>,
}

impl Batch {
    /// A quad from its four corners in order around it, the first at the image's
    /// top-left.
    pub fn quad(&mut self, info: &SprInfo, c: [V3; 4], col: [f32; 4]) {
        let [u, v, w, h] = info.uv;
        let uv = [[u, v], [u + w, v], [u + w, v + h], [u, v + h]];
        let vtx = |i: usize| SprVert { pos: [c[i].x, c[i].y, c[i].z], uv: uv[i], col };
        let list = self.by_tex.entry(rt::lower(&info.texture)).or_default();
        for i in [0, 1, 2, 0, 2, 3] {
            list.push(vtx(i));
        }
    }
    /// A billboard about `p` facing the camera (`right`, `up` in world space), half
    /// extents `w`, `h`.
    pub fn billboard(&mut self, info: &SprInfo, p: V3, right: V3, up: V3, w: f32, h: f32, col: [f32; 4]) {
        let (r, u) = (right * w, up * h);
        self.quad(info, [p - r + u, p + r + u, p + r - u, p - r - u], col);
    }
    pub fn is_empty(&self) -> bool {
        self.by_tex.values().all(|l| l.is_empty())
    }
    /// Draws every quad with `pipe` (the sprite shaders, added; `sprite_desc`); `vp` is
    /// view x projection.
    pub fn draw(&mut self, cmd: &mut Cmd, pipe: &Pipeline, vp: &M4) {
        if cmd.set_pipeline(pipe) {
            cmd.vs_block(Block::new(0, vp.transpose().as_floats()));
            for (tex, list) in self.by_tex.iter() {
                if list.is_empty() {
                    continue;
                }
                let Some(t) = assets::texture(tex) else { continue };
                cmd.bind_texture(0, Some(&t), &SAMPLER);
                // In chunks well under any vertex limit, as before.
                for chunk in list.chunks(6 * 2000) {
                    if let Some(s) = cmd.vertices(chunk) {
                        cmd.draw(&s);
                    }
                }
            }
        }
        for l in self.by_tex.values_mut() {
            l.clear();
        }
    }
}

/// The sprites' texture sampler: clamped, filtered through the levels.
const SAMPLER: Sampler = crate::gfx::sampler(Filter::Linear, MipFilter::Linear, Address::Clamp, Address::Clamp);

/// The sprites' pipeline with pixel shader `ps`: added, depth-tested (`LESSEQUAL`) and
/// not written, no culling.
pub fn sprite_desc(ps: gpu::Code) -> PipelineDesc {
    let mut d = PipelineDesc::new(gpu::code!(VS_SPRITE, VS_SPRITE_SPV), ps, &SPRITE_LAYOUT, 36, Topology::TriangleList, Format::Unknown);
    d.blend = Some(Blend::ADD);
    d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
    d
}

// ---- the starfield ------------------------------------------------------------------

/// `g_starfield`: the `Starfield` the map's starfield ODF built (null when none).
const G_STARFIELD: usize = 0x773d04;

/// The starfield's sprites by `ST3D_Sprite*`, as found in the sprite table by the name
/// the engine keeps (every `ST3D_DatabaseElement` holds its name at +0x8).
#[derive(Default)]
pub struct Starfield {
    sprites: BTreeMap<usize, Option<SprInfo>>,
    root: usize,
    said: bool,
}

impl Starfield {
    /// Adds every star of the engine's starfield to `b`. The stars are quads fixed in
    /// world space (`Starfield::mCreateStarSprite` places their four corners once, at
    /// a random distance and turn about the starfield's centre), so they show parallax
    /// as stock's do. Returns the number of stars.
    pub fn gather(&mut self, b: &mut Batch, lift: f32) -> usize {
        let sf = unsafe { rd::<usize>(G_STARFIELD) };
        let root = field(sf, 4);
        if root == 0 {
            return 0;
        }
        if root != self.root {
            self.root = root;
            self.said = false;
        }
        let mut n = 0;
        let mut stack: Vec<usize> = alloc::vec![root];
        let mut guard = 0;
        while let Some(node) = stack.pop() {
            guard += 1;
            if guard > 100_000 {
                break;
            }
            // `OctTree_Node`: its objects (+0x30) and its children (+0x3c), each a
            // `std::list` of `auto_ptr`s, whose pointer sits at node +0xc.
            each_in(field(node, 0x30), |star| {
                if self.star(b, star, lift) {
                    n += 1;
                }
            });
            each_in(field(node, 0x3c), |child| stack.push(child));
        }
        if !self.said {
            self.said = true;
            crate::log!("sky: starfield of {} stars, {} sprites", n, self.sprites.len());
        }
        n
    }

    /// One `Starfield` star (`mCreateStarSprite`): `+0` its `ST3D_Sprite*`, `+0xc` its
    /// four corners in world space, which `OctTree_Node::Render` hands to
    /// `ST3D_Sprite::DrawQuad` as they are.
    fn star(&mut self, b: &mut Batch, star: usize, lift: f32) -> bool {
        let sp = field(star, 0);
        if sp == 0 {
            return false;
        }
        let info = self.sprites.entry(sp).or_insert_with(|| {
            let name = unsafe { rt::cstr_at(field(sp, 8) as *const u8, 64) };
            let i = sprite_info(name);
            match &i {
                None => crate::log!("sky: starfield sprite {:?} not in the sprite table", name),
                Some(i) => crate::log!("sky: starfield sprite {:?}: {} {:?}", name, i.texture, i.uv),
            }
            i
        });
        let Some(info) = info.as_ref() else { return false };
        let c: [V3; 4] = unsafe { rd(star + 0xc) };
        // The sprite's colour (`ST3D_Sprite+0x24`, white unless the table sets one).
        let col: [f32; 3] = unsafe { rd(sp + 0x24) };
        // Only the point stars are lifted past white: the big blobs and clusters
        // (`blobby`, `starcluster`) are gas, and lifted they glowed over the sky.
        let lift = if info.texture.eq_ignore_ascii_case("mstars") { lift } else { 0.0 };
        b.quad(info, c, [col[0], col[1], col[2], lift]);
        true
    }
}

/// Each element of a VC6 `std::list<auto_ptr<T>>` given its head node.
fn each_in(head: usize, mut f: impl FnMut(usize)) {
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 100_000 {
        let v = field(n, 0xc);
        if v != 0 {
            f(v);
        }
        n = field(n, 0);
        guard += 1;
    }
}

/// The camera's right and up in world space, from the view matrix (row vectors).
pub fn camera_axes(view: &M4) -> (V3, V3) {
    let m = &view.0;
    (v3(m[0][0], m[1][0], m[2][0]).norm(), v3(m[0][1], m[1][1], m[2][1]).norm())
}
