//! The fog of war and the shroud, drawn from the engine's own textures with smooth
//! edges: the port of `Fog.asi` (the decisions
//! settled with the user are kept here).
//!
//! Stock draws each layer as one mesh over the map (`Terrain_Geometry` `+0x2c`, or the
//! subdivided `+0x30`), textured with `fogTexture` (`+0x34`, texture material `fog`:
//! blended) or `shroudTexture` (`+0x38`, `alphathreshold`: alpha-tested at one half),
//! with the depth test off and no depth writes, its constant lighting material white.
//! Each texture holds one texel per cell of the scanner's grid (`Scanner::m_fog`,
//! `m_shroud` at 0x738a54 / 0x738a58: width `+0xc`, height `+0x10`), centred in it.
//! Here the same quad is drawn by us, and the pixel shader reconstructs the mask with
//! a cubic filter and draws its edge as a smoothstep never narrower than about a pixel;
//! the shroud is blended so that edge shows. The textures are never resized: there is
//! nothing finer to put in them, and resizing them froze the game (`UImid`).
use crate::engine::{field, field_f32, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use gpu::{
    attr, Address, AttrFormat, Blend, Block, Code, Compare, Depth, Filter, Format, MipFilter, PipelineDesc, Semantic,
    Topology, VertexAttr,
};
use crate::log;
use crate::rt::Ini;
use crate::shaders::overlays::*;

const SCAN_FOG: usize = 0x738a54;
const SCAN_SHROUD: usize = 0x738a58;
const FOG_OPACITY: f32 = 0.7;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Fog,
    Shroud,
}

/// One layer's edge (`Fog.ini`'s keys, now in `Render3D.ini` `[Overlays]`).
#[derive(Clone, Copy)]
pub struct Edge {
    pub threshold: f32,
    pub softness: f32,
    pub rim: f32,
}

pub struct Settings {
    pub catrom: bool,
    pub fog: Edge,
    pub shroud: Edge,
    pub rim_width: f32,
    pub antialias: f32,
}

impl Settings {
    pub fn read(ini: &Ini) -> Settings {
        let f = |k: &str, d: f32, lo: f32, hi: f32| ini.float("Overlays", k, d).clamp(lo, hi);
        let filter = ini.string("Overlays", "Filter").unwrap_or_default();
        Settings {
            catrom: filter.trim().as_bytes().first().is_some_and(|c| c | 0x20 == b'c'),
            fog: Edge { threshold: f("FogThreshold", 0.5, 0.05, 0.95), softness: f("FogSoftness", 0.2, 0.0, 0.5), rim: f("FogRim", 1.0, 0.0, 1.0) },
            shroud: Edge {
                threshold: f("ShroudThreshold", 0.5, 0.05, 0.95),
                softness: f("ShroudSoftness", 0.1, 0.0, 0.5),
                rim: f("ShroudRim", 1.0, 0.0, 1.0),
            },
            rim_width: f("RimWidth", 0.25, 0.1, 4.0),
            antialias: f("Antialias", 1.0, 0.0, 8.0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx {
    pos: [f32; 3],
    uv: [f32; 2],
}

static VERTEX: [VertexAttr; 2] = [attr(0, AttrFormat::Float3, Semantic::Position, 0), attr(12, AttrFormat::Float2, Semantic::TexCoord, 0)];

pub struct Fog {
    pipes: Pipes,
    said: [bool; 2],
    said_fail: bool,
}

impl Fog {
    pub fn new() -> Fog {
        Fog {
            pipes: Pipes::new(),
            said: [false; 2],
            said_fail: false,
        }
    }

    pub fn reset(&mut self) {}

    /// Draws one layer over the map. `tg` is `g_terrain_geometry`.
    pub fn draw(&mut self, v: &View, tg: usize, layer: Layer, s: &Settings) {
        let (tex_off, grid_ptr, edge) = match layer {
            Layer::Fog => (0x34, SCAN_FOG, s.fog),
            Layer::Shroud => (0x38, SCAN_SHROUD, s.shroud),
        };
        let Some(tex) = gfx::engine_texture(field(tg, tex_off)) else {
            if !self.said_fail {
                self.said_fail = true;
                log!("overlays: the fog's or shroud's texture has no Direct3D 9 texture behind it; that layer is not drawn");
            }
            return;
        };
        let d = tex.desc();
        let n = d.width;
        let grid = unsafe { rd::<usize>(grid_ptr) };
        let (gw, gh) = (field(grid, 0xc) as i32, field(grid, 0x10) as i32);
        if n < 4 || n != d.height || gw < 1 || gh < 1 || gw > n as i32 || gh > n as i32 {
            if !self.said_fail {
                self.said_fail = true;
                log!("overlays: no square layer texture or no scan grid that fits it ({}x{}, grid {}x{}); not drawn", d.width, d.height, gw, gh);
            }
            return;
        }
        let x0 = (n as i32 - gw) / 2;
        let y0 = (n as i32 - gh) / 2;
        // The quad: the map's rectangle at the grid's height, its UVs as
        // `Terrain_Geometry::SetVertexPositions` makes them (the map centred in the
        // texture's unit square, square texels, v running against z).
        let (mx0, mz0, mx1, mz1, y) = (field_f32(tg, 0x4), field_f32(tg, 0xc), field_f32(tg, 0x10), field_f32(tg, 0x18), field_f32(tg, 0x1c));
        let (w, h) = (mx1 - mx0, mz1 - mz0);
        let size = w.max(h);
        if !(size > 0.0) {
            return;
        }
        let u0 = (size - w) / (2.0 * size);
        let v1 = (size - h) / (2.0 * size) + h / size;
        let uv = |x: f32, z: f32| [u0 + (x - mx0) / size, v1 - (z - mz0) / size];
        let q = [
            Vtx { pos: [mx0, y, mz0], uv: uv(mx0, mz0) },
            Vtx { pos: [mx1, y, mz0], uv: uv(mx1, mz0) },
            Vtx { pos: [mx0, y, mz1], uv: uv(mx0, mz1) },
            Vtx { pos: [mx1, y, mz1], uv: uv(mx1, mz1) },
        ];
        let li = (layer == Layer::Shroud) as usize;
        if !self.said[li] {
            self.said[li] = true;
            log!(
                "overlays: first {} drawn: texture {}, grid {}x{} at {},{}, map {:.0},{:.0} to {:.0},{:.0} at y {:.0}",
                if li == 1 { "shroud" } else { "fog" }, n, gw, gh, x0, y0, mx0, mz0, mx1, mz1, y
            );
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        let ps: &'static [u32] = if s.catrom { &PS_FOG_CATROM } else { &PS_FOG_BSPLINE };
        // The depth test on and always passing (as stock: `ZENABLE` on, `ALWAYS`), no
        // depth written, blended by alpha, no culling.
        let pipe = self.pipes.view(s.catrom as u64, || {
            let mut d = PipelineDesc::new(Code::d3d9(&VS_PLANE), Code::d3d9(ps), &VERTEX, 20, Topology::TriangleStrip, Format::Unknown);
            d.depth = Depth { test: true, write: false, compare: Compare::Always, bias: 0.0, slope_bias: 0.0 };
            d.blend = Some(Blend::ALPHA);
            d
        });
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        let nf = n as f32;
        let (soft, aa, rim) = (edge.softness, s.antialias, edge.rim);
        // The texture material's opacity: `fog` is made translucent 0.3 (opacity 0.7,
        // `ST3D_TextureMaterial+0x24`), `alphathreshold` opaque.
        let opacity = if layer == Layer::Fog { FOG_OPACITY } else { 1.0 };
        let k: [f32; 16] = [
            nf, 1.0 / nf, edge.threshold, soft,
            x0 as f32 + 0.5, y0 as f32 + 0.5, (x0 + gw) as f32 - 0.5, (y0 + gh) as f32 - 0.5,
            aa, rim, 1.0 / s.rim_width, 0.0,
            1.0, 1.0, 1.0, opacity,
        ];
        cmd.ps_block(Block::new(0, &k));
        cmd.bind_texture(0, Some(&tex), &gfx::sampler(Filter::Linear, MipFilter::None, Address::Wrap, Address::Wrap));
        if let Some(sp) = cmd.vertices(&q) {
            cmd.draw(&sp);
        }
    }
}
