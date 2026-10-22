//! Stock's own shots, drawn by us as stock draws them: an `ST3D_Sprite` (the ODF's
//! `Sprite`, in the sprite table) either as a ribbon between two points
//! (`ST3D_Sprite::RenderAsPolygon` 0x63a810: phasers, pulses) or as a square facing the
//! camera (`ST3D_Sprite::DrawScaled3D` 0x63aa60: torpedoes, plasma, polerons,
//! artillery). Both end in `ST3D_Sprite::ClipAndRender3D` (0x63b4b0), which plays the
//! sprite's animation, takes its rectangle of the texture and draws one quad, its
//! texture times the colour, blended as the sprite's material says.
//!
//! What is read (README "Ordnance", the engine side):
//! - the sprite (the ordnance class +0x12c): its colour +0x24, its rectangle +0x38
//!   (u, v, w, h as fractions of the texture), its flags +0x50 (0x10: not animated;
//!   0x20: tiled along its length, `@tile_u`), its material +0x54 (blend factors
//!   +0x1c source, +0x20 destination, Direct3D's numbers), its texture +0x58 (named at
//!   +0x8, as every database element is);
//! - the animation: `ST3D_Sprite::Animate` (0x63a580) plays the sprite's channels and
//!   leaves the flipbook's offset in `ST3D_Sprite::m_texture_coordinate_offset`
//!   (0x7ad6d0). We call it where `ClipAndRender3D` would, so a flipbook shows the
//!   frame stock shows; it writes only the sprite's own drawing state.
//! - the brightness Storm3D +0x4 (times the colour) and the device's transparency +0x48
//!   (the vertex alpha).
use super::{eng, View};
use crate::assets;
use gpu::d3d9::raw::*;
use crate::engine::{self, field, field_f32, rd};
use crate::gfx::{self, Decl, DynVb, Shader};
use crate::math::{v3, V3};
use crate::shaders::ordnance::*;
use alloc::string::String;
use alloc::vec::Vec;

const FN_ANIMATE: usize = 0x63a580; // ST3D_Sprite::Animate(), thiscall
const TEX_OFFSET: usize = 0x7ad6d0; // ST3D_Sprite::m_texture_coordinate_offset, Vector2
/// The time the engine's animation channels are evaluated at (`ST3D_AnimationChannel::
/// Evaluate` reads it; `SetStartTime` sets it from Storm3D's wall clock): under the bench
/// clock, the sim time while we call `Animate`, then put back (README "The animation clock").
const ANIM_TIME: usize = 0x7a8e74;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SVert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}

static SDECL: [VertexElement; 4] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    ve(0, 20, DECLTYPE_FLOAT4, DECLUSAGE_TEXCOORD, 1),
    VE_END,
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
    vs: Shader,
    ps: Shader,
    decl: Decl,
    vb: DynVb,
    pub hot: f32,
    pub hot_from: f32,
    said: Vec<usize>,
}

impl Sprites {
    pub fn new() -> Sprites {
        Sprites {
            quads: Vec::new(),
            vs: Shader::vertex(&VS_ORD_SPRITE),
            ps: Shader::pixel(&PS_ORD_SPRITE),
            decl: Decl::new(&SDECL),
            vb: DynVb::new(36 * 6 * 1024),
            hot: 0.6,
            hot_from: 0.7,
            said: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.vb.reset();
        self.quads.clear();
    }

    /// The sprite's look now: texture, rectangle (after its animation), blend, flags.
    fn look(&mut self, sprite: usize) -> Option<(String, [f32; 4], u32, u32, u8)> {
        let tex = field(sprite, 0x58);
        let name = unsafe { crate::rt::cstr_at(field(tex, 8) as *const u8, 260) };
        if name.is_empty() {
            return None;
        }
        // The file name without a path or an extension.
        let base = name.rsplit(['\\', '/']).next().unwrap_or(name);
        let stem = base.split('.').next().unwrap_or(base);
        let flags = engine::field_u8(sprite, 0x50);
        if flags & 0x10 == 0 {
            unsafe {
                let f: unsafe extern "thiscall" fn(usize) = core::mem::transmute(FN_ANIMATE);
                if crate::clock::bench() {
                    let t = ANIM_TIME as *mut f32;
                    let saved = t.read_unaligned();
                    t.write_unaligned(crate::clock::anim_now() as f32);
                    f(sprite);
                    t.write_unaligned(saved);
                } else {
                    f(sprite);
                }
            }
        }
        let off: [f32; 2] = unsafe { rd(TEX_OFFSET) };
        let r: [f32; 4] = unsafe { rd(sprite + 0x38) };
        let mat = field(sprite, 0x54);
        let (mut src, mut dst) = (field(mat, 0x1c) as u32, field(mat, 0x20) as u32);
        if mat == 0 || !(1..=13).contains(&src) || !(1..=13).contains(&dst) {
            src = BLEND_ONE;
            dst = BLEND_ONE;
        }
        if !self.said.contains(&sprite) && self.said.len() < 64 {
            self.said.push(sprite);
            crate::log!(
                "ordnance: sprite {} ({}): rect {:.3} {:.3} {:.3} {:.3}, flags {:#x}, blend {} {}",
                stem, sprite, r[0], r[1], r[2], r[3], flags, src, dst
            );
        }
        Some((String::from(stem), [off[0] + r[0], off[1] + r[1], r[2], r[3]], src, dst, flags))
    }

    /// The vertex colour stock gives a sprite: its colour times the engine's
    /// brightness, the device's transparency as alpha.
    fn colour(col: V3) -> [f32; 4] {
        let b = field_f32(engine::storm3d(), 4);
        let a = field_f32(engine::st3d_device(), 0x48);
        let c = |x: f32| (x * b).clamp(0.0, 1.0);
        [c(col.x), c(col.y), c(col.z), a.clamp(0.0, 1.0)]
    }

    fn push(&mut self, sprite: usize, p: [V3; 4], col: V3) {
        let Some((tex, r, src, dst, flags)) = self.look(sprite) else { return };
        let (u0, v0) = (r[0], r[1]);
        let mut u1 = u0 + r[2];
        let v1 = v0 + r[3];
        if flags & 0x20 != 0 {
            // Tiled: the rectangle repeats along the ribbon, once per its width.
            let w = field_f32(sprite, 0x40);
            if w > 1e-6 {
                u1 = u0 + (p[0] - p[1]).len() / w;
            }
        }
        let c = Self::colour(col);
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
        let inst = field(engine::storm3d(), 0x100);
        let k = field_f32(inst, 0x28);
        if inst != 0 && k != 1.0 {
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
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            return;
        }
        dev.vs_const(0, &gfx::vp_constants(v));
        dev.ps_const(0, &[self.hot, self.hot_from, 0.0, 0.0]);
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 0), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHATESTENABLE, 0),
            (RS_ALPHABLENDENABLE, 1), (RS_BLENDOP, BLENDOP_ADD), (RS_SEPARATEALPHABLENDENABLE, 0),
            (RS_FOGENABLE, 0), (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3), (RS_STENCILENABLE, 0),
            (RS_COLORWRITEENABLE, 0xf), (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        for (s, val) in [
            (SAMP_ADDRESSU, TADDRESS_WRAP), (SAMP_ADDRESSV, TADDRESS_WRAP), (SAMP_MAGFILTER, TEXF_LINEAR),
            (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_LINEAR), (SAMP_SRGBTEXTURE, 0),
        ] {
            dev.set_ss(0, s, val);
        }
        dev.set_stream_freq(0, 1);
        let mut i = 0;
        while i < quads.len() {
            // A run of quads that share a texture and a blend: one draw.
            let q0 = &quads[i];
            let mut j = i + 1;
            while j < quads.len() && quads[j].tex == q0.tex && quads[j].src == q0.src && quads[j].dst == q0.dst && j - i < 1000 {
                j += 1;
            }
            let tex = assets::texture(&q0.tex).map(|t| t.raw_texture(crate::gfx::gpu())).unwrap_or(core::ptr::null_mut());
            if !tex.is_null() {
                let mut verts: Vec<SVert> = Vec::with_capacity((j - i) * 6);
                for q in &quads[i..j] {
                    for k in [0, 1, 2, 0, 2, 3] {
                        verts.push(q.v[k]);
                    }
                }
                dev.set_texture(0, tex);
                dev.set_rs(RS_SRCBLEND, q0.src);
                dev.set_rs(RS_DESTBLEND, q0.dst);
                if let Some(first) = self.vb.push(dev, &verts, 36) {
                    dev.draw_primitive(PT_TRIANGLELIST, first, (verts.len() / 3) as u32);
                }
            }
            i = j;
        }
    }
}
