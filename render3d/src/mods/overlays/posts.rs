//! Height posts: `DrawHeightPost(const Vector3&, ST3D_Camera*)` (0x593840, cdecl) draws
//! a line from a point straight down (or up) to the grid's plane (`Terrain_Geometry`
//! `+0x1c`, 0x77215c), in `ST3D_Colour_Green` (0x7a8f68) when the point is above the
//! plane and `ST3D_Colour_Red` (0x7a8f78) when not, through `ST3D_Camera::
//! ClipAndRenderLine` (0x618590): one pixel wide, aliased. Its callers: the build ghost
//! (`PlaceholderFeature::sRenderInternal`, 0x473bbc, now drawn by hulls, which asks
//! `height_post` directly), and the overview's `mNormalModeRender` (0x522080),
//! `mRenderWhere` (0x5222fd, the move target), `mDrawPosts` (0x524c02, every object
//! when posts are on) and its one-point helper (0x524c2d), all from
//! `DisplayInterface::PreRenderAll`, inside the body.
//!
//! Each of those five calls is re-pointed at `hook_post` (only where it still calls
//! 0x593840): while one of our views runs, the post is noted for the view and drawn at
//! the flush as an anti-aliased line of a fixed pixel width (`shaders/overlays.hlsl`,
//! `vs_line`/`ps_line`), depth-tested, not written; otherwise the engine's is called.
use gpu::{attr, AttrFormat, Blend, Block, Compare, Depth, Format, PipelineDesc, Semantic, Topology, VertexAttr};
use crate::engine::{hook, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::log;
use crate::math::V3;
use crate::shaders::overlays::*;
use crate::shaders::overlays_spv::*;
use crate::shaders::overlays_dxil::*;
use alloc::vec::Vec;

const FN_HEIGHT_POST: usize = 0x593840;
const SITES: [usize; 5] = [0x473bbc, 0x522080, 0x5222fd, 0x524c02, 0x524c2d];
const TERRAIN_Y: usize = 0x77215c;
const GREEN: usize = 0x7a8f68;
const RED: usize = 0x7a8f78;

/// The posts noted in the view being drawn: (top, the plane's height).
static mut POSTS: Vec<(V3, f32)> = Vec::new();
static mut IN_VIEW: bool = false;

#[allow(static_mut_refs)]
fn posts() -> &'static mut Vec<(V3, f32)> {
    unsafe { &mut POSTS }
}

unsafe extern "C" fn hook_post(pos: *const V3, cam: usize) {
    unsafe {
        if IN_VIEW && !pos.is_null() && posts().len() < 4096 {
            posts().push((core::ptr::read_unaligned(pos), rd::<f32>(TERRAIN_Y)));
            return;
        }
        let f: unsafe extern "C" fn(*const V3, usize) = core::mem::transmute(FN_HEIGHT_POST);
        f(pos, cam);
    }
}

/// A post noted directly (the build ghost drawn by hulls); false when posts are not
/// ours (the caller draws the engine's).
pub fn note(pos: V3) -> bool {
    unsafe {
        if !IN_VIEW {
            return false;
        }
        if posts().len() < 4096 {
            posts().push((pos, rd::<f32>(TERRAIN_Y)));
        }
    }
    true
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx {
    a: [f32; 3],
    b: [f32; 3],
    se: [f32; 2],
    col: [f32; 4],
}

static VERTEX: [VertexAttr; 4] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float3, Semantic::TexCoord, 0),
    attr(24, AttrFormat::Float2, Semantic::TexCoord, 1),
    attr(32, AttrFormat::Float4, Semantic::TexCoord, 2),
];

pub struct Posts {
    pub on: bool,
    /// The line's width in pixels at 1080 lines (scaled with the view's height), and its
    /// soft edge in pixels.
    width: f32,
    soft: f32,
    /// The colour times this (above 1 the post glows).
    bright: f32,
    pipes: Pipes,
    hooked: bool,
    said: u32,
}

impl Posts {
    pub fn new(ini: &crate::rt::Ini) -> Posts {
        Posts {
            on: ini.int("Overlays", "HeightPosts", 1) != 0,
            width: ini.float("Overlays", "HeightPostWidth", 1.5).max(0.5),
            soft: ini.float("Overlays", "HeightPostSoft", 1.0).max(0.25),
            bright: ini.float("Overlays", "HeightPostBrightness", 1.0),
            pipes: Pipes::new(),
            hooked: false,
            said: 0,
        }
    }

    /// Re-points the engine's calls (once).
    pub fn install(&mut self) {
        if !self.on || self.hooked {
            return;
        }
        self.hooked = true;
        let mut n = 0;
        for s in SITES {
            if hook::patch_call(s, FN_HEIGHT_POST, hook_post as *const () as usize) {
                n += 1;
            } else {
                log!("overlays: the height post's call at {:#x} is not stock's: the engine draws it", s);
            }
        }
        log!("overlays: height posts: {} of {} calls ours", n, SITES.len());
    }

    pub fn reset(&mut self) {

    }

    pub fn begin_view(&mut self) {
        posts().clear();
        unsafe { IN_VIEW = self.on && self.hooked };
    }

    pub fn end_view(&mut self) {
        unsafe { IN_VIEW = false };
        posts().clear();
    }

    /// Every post noted in this view.
    pub fn draw(&mut self, v: &View) {
        let ps = core::mem::take(posts());
        if ps.is_empty() {
            return;
        }
        let green: [f32; 3] = unsafe { rd(GREEN) };
        let red: [f32; 3] = unsafe { rd(RED) };
        let mut vs: Vec<Vtx> = Vec::with_capacity(ps.len() * 6);
        for &(top, y) in &ps {
            let c = if y < top.y { green } else { red };
            let col = [c[0] * self.bright, c[1] * self.bright, c[2] * self.bright, 1.0];
            let a = [top.x, top.y, top.z];
            let b = [top.x, y, top.z];
            let q = |s: f32, e: f32| Vtx { a, b, se: [s, e], col };
            vs.extend_from_slice(&[q(-1.0, 0.0), q(1.0, 0.0), q(1.0, 1.0), q(-1.0, 0.0), q(1.0, 1.0), q(-1.0, 1.0)]);
        }
        if self.said < 3 {
            self.said += 1;
            log!("overlays: {} height post(s), the first from {:.0},{:.0},{:.0} to y {:.0}", ps.len(), ps[0].0.x, ps[0].0.y, ps[0].0.z, ps[0].1);
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        // Depth-tested (`LESSEQUAL`) and not written, blended by alpha, no culling.
        let pipe = self.pipes.view(0, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_LINE, VS_LINE_SPV, VS_LINE_DXIL), gpu::code!(PS_LINE, PS_LINE_SPV, PS_LINE_DXIL), &VERTEX, 48, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = Some(Blend::ALPHA);
            d
        });
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        // The old dynamic buffer held 64 KB: more posts than that were not drawn.
        if core::mem::size_of_val(&vs[..]) > 64 * 1024 {
            return;
        }
        let Some(span) = cmd.vertices(&vs) else { return };
        let scale = (v.rect.h as f32 / 1080.0).max(0.5);
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.vs_block(Block::new(8, &[v.rect.w as f32, v.rect.h as f32, 0.5 * self.width * scale, self.soft]));
        cmd.ps_block(Block::new(0, &[self.soft, 0.0, 0.0, 0.0]));
        cmd.bind_texture(0, None, &crate::scene::SAMPLER);
        cmd.draw(&span);
    }
}
