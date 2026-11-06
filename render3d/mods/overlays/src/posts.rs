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
use super::hooks;
use crate::log;
use r3d_core::frame::steps::View;
use r3d_core::gfx::{self, Pipes};
use crate::rt::settings;
use super::settings_table::OVERLAYS_HEIGHTPOSTS;
use r3d_core::math::V3;
use crate::shaders::overlays_spv::*;
use crate::shaders::overlays_dxil::*;
use alloc::vec::Vec;

/// The posts noted in the view being drawn: (top, the plane's height).
static mut POSTS: Vec<(V3, f32)> = Vec::new();
static mut IN_VIEW: bool = false;

#[allow(static_mut_refs)]
fn posts() -> &'static mut Vec<(V3, f32)> {
    unsafe { &mut POSTS }
}

/// Whether a view of ours is collecting posts (the detour in `hooks.rs` asks).
pub fn in_view() -> bool {
    unsafe { IN_VIEW }
}

/// A post noted by the detour; false when the view's list is full (the engine draws it).
pub fn note_raw(top: V3) -> bool {
    if posts().len() < 4096 {
        posts().push((top, hooks::terrain_y()));
        true
    } else {
        false
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
            posts().push((pos, hooks::terrain_y()));
        }
    }
    true
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Vtx {
    a: [f32; 3],
    b: [f32; 3],
    se: [f32; 2],
    col: [f32; 4],
}

pub(super) static VERTEX: [VertexAttr; 4] = [
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
            on: settings::on(ini, OVERLAYS_HEIGHTPOSTS),
            width: ini.float("Overlays", "HeightPostWidth", 1.5).max(0.5),
            soft: ini.float("Overlays", "HeightPostSoft", 1.0).max(0.25),
            bright: ini.float("Overlays", "HeightPostBrightness", 1.0),
            pipes: Pipes::new(),
            hooked: false,
            said: 0,
        }
    }

    /// Installs the engine's five calls (once, at the module's first enable): ours while
    /// the module is on, the engine's own when it is off or `HeightPosts` is. Installed
    /// whether or not `HeightPosts` is on, so the key can be switched live (`IN_VIEW`).
    pub fn install(&mut self, h: &mut r3d_core::hooks::Hooks) {
        if self.hooked {
            return;
        }
        self.hooked = true;
        let n = hooks::install_posts(h);
        crate::log!("overlays: height posts: {} of 5 calls ours", n);
    }

    pub fn reset(&mut self) {

    }

    /// `[Overlays] HeightPosts` again (`Render3D_SettingsChanged`); true if it changed.
    pub fn set_on(&mut self, on: bool) -> bool {
        let changed = on != self.on;
        self.on = on;
        changed
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
        let (green, red) = hooks::post_colours();
        let mut vs: Vec<Vtx> = Vec::with_capacity(ps.len() * 6);
        for &(top, y) in &ps {
            let c = if y < top.y { green } else { red };
            let col = [c[0] * self.bright, c[1] * self.bright, c[2] * self.bright, 1.0];
            segment(&mut vs, top, V3 { x: top.x, y, z: top.z }, col);
        }
        if self.said < 3 {
            self.said += 1;
            log!("overlays: {} height post(s), the first from {:.0},{:.0},{:.0} to y {:.0}", ps.len(), ps[0].0.x, ps[0].0.y, ps[0].0.z, ps[0].1);
        }
        let scale = (v.rect.h as f32 / 1080.0).max(0.5);
        draw_quads(&mut self.pipes, v, &vs, 0.5 * self.width * scale, self.soft);
    }
}

/// A line from `a` to `b` in `col`, as the six vertices of its quad (`vs_line`).
pub(super) fn segment(vs: &mut Vec<Vtx>, a: V3, b: V3, col: [f32; 4]) {
    let (a, b) = ([a.x, a.y, a.z], [b.x, b.y, b.z]);
    let q = |s: f32, e: f32| Vtx { a, b, se: [s, e], col };
    vs.extend_from_slice(&[q(-1.0, 0.0), q(1.0, 0.0), q(1.0, 1.0), q(-1.0, 0.0), q(1.0, 1.0), q(-1.0, 1.0)]);
}

/// Draws lines made by `segment`, `half` pixels either side of each centre line with a
/// soft edge `soft` pixels wide: depth-tested (`LESSEQUAL`) and not written, blended by
/// alpha, no culling. The posts' and the pivot marker's.
pub(super) fn draw_quads(pipes: &mut Pipes, v: &View, vs: &[Vtx], half: f32, soft: f32) {
    if vs.is_empty() {
        return;
    }
    let Some(mut cmd) = gfx::cmd() else { return };
    let pipe = pipes.view(0, || {
        let mut d = PipelineDesc::new(gpu::code!(VS_LINE_SPV, VS_LINE_DXIL), gpu::code!(PS_LINE_SPV, PS_LINE_DXIL), &VERTEX, 48, Topology::TriangleList, Format::Unknown);
        d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
        d.blend = Some(Blend::ALPHA);
        d
    });
    let Some(pipe) = pipe else { return };
    if !cmd.set_pipeline(&pipe) {
        return;
    }
    // The old dynamic buffer held 64 KB: more posts than that were not drawn.
    if core::mem::size_of_val(vs) > 64 * 1024 {
        return;
    }
    let Some(span) = cmd.vertices(vs) else { return };
    cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
    cmd.vs_block(Block::new(8, &[v.rect.w as f32, v.rect.h as f32, half, soft]));
    cmd.ps_block(Block::new(0, &[soft, 0.0, 0.0, 0.0]));
    cmd.bind_texture(0, None, &r3d_core::scene::SAMPLER);
    cmd.draw(&span);
}
