//! The tactical grid: `Terrain_Geometry::DrawGrid` (0x593480), when `GridVisible()`
//! (0x51e180, the current `View_Record`'s `GridRenderState`) says so.
//!
//! Stock draws one line every `GRID_SIZE` (0x71a78c, 100 units) across the map's
//! rectangle at the grid's height, `Terrain_Geometry` `+0x4` x0, `+0xc` z0, `+0x10` x1,
//! `+0x18` z1, `+0x1c` y, with `+0x20` + 1 lines along z and `+0x24` + 1 along x, each
//! through `ST3D_Camera::ClipAndRenderLine` in `ST3D_Colour_Dark_Orange` (0x7a8ee8),
//! depth-tested and written. (`DRAW_PIZZA_BOX`'s blue lines are a debug switch.)
use gpu::d3d9::raw::*;
use crate::engine::{field, field_f32, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, DynVb, Shader};
use crate::shaders::overlays::*;
use alloc::vec::Vec;

const GRID_SIZE: usize = 0x71a78c;
const DARK_ORANGE: usize = 0x7a8ee8;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx {
    pos: [f32; 3],
    uv: [f32; 2],
}

static VERTEX: [VertexElement; 3] =
    [ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0), ve(0, 12, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0), VE_END];

pub struct Grid {
    vs: Shader,
    ps: Shader,
    decl: Decl,
    vb: DynVb,
}

impl Grid {
    pub fn new() -> Grid {
        Grid { vs: Shader::vertex(&VS_PLANE), ps: Shader::pixel(&PS_COLOUR), decl: Decl::new(&VERTEX), vb: DynVb::new(64 * 1024) }
    }

    pub fn reset(&mut self) {
        self.vb.reset();
    }

    pub fn draw(&mut self, v: &View, tg: usize) {
        let step = unsafe { rd::<f32>(GRID_SIZE) };
        let (x0, z0, x1, z1, y) = (field_f32(tg, 0x4), field_f32(tg, 0xc), field_f32(tg, 0x10), field_f32(tg, 0x18), field_f32(tg, 0x1c));
        let (nx, nz) = (field(tg, 0x20) as i32, field(tg, 0x24) as i32);
        if !(step > 0.0) || nx < 0 || nz < 0 || nx > 2000 || nz > 2000 {
            return;
        }
        let mut lines: Vec<Vtx> = Vec::with_capacity(((nx + nz + 2) * 2) as usize);
        let p = |x: f32, z: f32| Vtx { pos: [x, y, z], uv: [0.0, 0.0] };
        // As stock: the coordinate is summed a step at a time.
        let mut x = x0;
        for _ in 0..=nx {
            lines.push(p(x, z0));
            lines.push(p(x, z1));
            x += step;
        }
        let mut z = z0;
        for _ in 0..=nz {
            lines.push(p(x0, z));
            lines.push(p(x1, z));
            z += step;
        }
        let c: [f32; 3] = unsafe { rd(DARK_ORANGE) };
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            return;
        }
        let Some(first) = self.vb.push(dev, &lines, core::mem::size_of::<Vtx>() as u32) else { return };
        dev.vs_const(4, &gfx::vp_constants(v));
        dev.ps_const(0, &[c[0], c[1], c[2], 1.0]);
        dev.set_texture(0, core::ptr::null_mut());
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ZWRITEENABLE, 1), (RS_CULLMODE, CULL_NONE),
            (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
            (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        dev.set_stream_freq(0, 1);
        dev.draw_primitive(PT_LINELIST, first, lines.len() as u32 / 2);
    }
}
