//! The tactical grid: `Terrain_Geometry::DrawGrid` (0x593480), when `GridVisible()`
//! (0x51e180, the current `View_Record`'s `GridRenderState`) says so.
//!
//! Stock draws one line every `GRID_SIZE` (0x71a78c, 100 units) across the map's
//! rectangle at the grid's height, `Terrain_Geometry` `+0x4` x0, `+0xc` z0, `+0x10` x1,
//! `+0x18` z1, `+0x1c` y, with `+0x20` + 1 lines along z and `+0x24` + 1 along x, each
//! through `ST3D_Camera::ClipAndRenderLine` in `ST3D_Colour_Dark_Orange` (0x7a8ee8),
//! depth-tested and written. (`DRAW_PIZZA_BOX`'s blue lines are a debug switch.)
use super::hooks;
use r3d_core::frame::steps::View;
use r3d_core::gfx::{self, Pipes};
use crate::shaders::overlays_spv::*;
use crate::shaders::overlays_dxil::*;
use alloc::vec::Vec;
use gpu::{attr, AttrFormat, Block, Depth, Format, PipelineDesc, Semantic, Topology, VertexAttr};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx {
    pos: [f32; 3],
    uv: [f32; 2],
}

static VERTEX: [VertexAttr; 2] = [attr(0, AttrFormat::Float3, Semantic::Position, 0), attr(12, AttrFormat::Float2, Semantic::TexCoord, 0)];

pub struct Grid {
    pipes: Pipes,
}

impl Grid {
    pub fn new() -> Grid {
        Grid { pipes: Pipes::new() }
    }

    pub fn reset(&mut self) {}

    pub fn draw(&mut self, v: &View, tg: usize) {
        let step = hooks::grid_step();
        let (x0, z0, x1, z1, y) = hooks::terrain_rect(tg);
        let (nx, nz) = hooks::terrain_cells(tg);
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
        let c = hooks::grid_colour();
        static mut SAID: bool = false;
        unsafe {
            if !SAID {
                SAID = true;
                crate::log!("overlays: first grid drawn: {}x{} cells, colour {:.2},{:.2},{:.2} (read from the engine's colour pointer)", nx, nz, c[0], c[1], c[2]);
            }
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        // Lines, depth-tested (`LESSEQUAL`) and written, opaque.
        let pipe = self.pipes.view(0, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_PLANE_SPV, VS_PLANE_DXIL), gpu::code!(PS_COLOUR_SPV, PS_COLOUR_DXIL), &VERTEX, 20, Topology::LineList, Format::Unknown);
            d.depth = Depth::TEST_WRITE;
            d
        });
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        // The old dynamic buffer held 64 KB: a grid of more lines was not drawn.
        if core::mem::size_of_val(&lines[..]) > 64 * 1024 {
            return;
        }
        let Some(span) = cmd.vertices(&lines) else { return };
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.ps_block(Block::new(0, &[c[0], c[1], c[2], 1.0]));
        cmd.bind_texture(0, None, &r3d_core::scene::SAMPLER);
        cmd.draw(&span);
    }
}
