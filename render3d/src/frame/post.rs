//! The frame's own targets and the passes after the scene: resolve, bloom, tone map,
//! all through `gpu` (gpu/README.md).
//!
//! One set of targets the size of the back buffer serves both views, since a view's
//! viewport is a rectangle of the back buffer in either. Every pass works inside the
//! view's rectangle (scaled down for each bloom level), and every tap is clamped to it.
use crate::config::Config;
use crate::dump::{Area, Dump, Kind};
use crate::log;
use crate::shaders::post::*;
use alloc::vec::Vec;
use gpu::{
    attr, AttrFormat, Blend, Block, Cmd, Code, ColorTarget, Contents, DepthTarget, Format, Gpu, Load, Pass, Pipeline,
    PipelineDesc, Rect, Sampler, Semantic, Texture, TextureDesc, Topology, Usage, VertexAttr,
};

pub const BLOOM_LEVELS: usize = 5;

/// A rectangle in pixels, `x, y, w, h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PRect {
    /// **Transitional**: as a Direct3D 9 rectangle, for the modules not yet on `gpu`.
    pub fn d3d(self) -> gpu::d3d9::raw::Rect {
        gpu::d3d9::raw::Rect { left: self.x as i32, top: self.y as i32, right: (self.x + self.w) as i32, bottom: (self.y + self.h) as i32 }
    }

    pub fn rect(self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}

struct Level {
    tex: Texture,
    w: u32,
    h: u32,
}

/// Everything the frame draws the view into. `gpu` makes the targets again after a
/// `Reset` or on a new device; the frame drops them then all the same (`frame::reset`).
pub struct Targets {
    pub w: u32,
    pub h: u32,
    pub fmt: Format,
    /// Samples of the scene target; 0 without MSAA.
    pub samples: u32,
    /// The scene target: multisampled, or the resolve texture itself at 1x.
    pub scene: Texture,
    pub depth: Texture,
    resolve: Texture,
    levels: Vec<Level>,
}

/// The post passes' pipelines, for the targets' format and the back buffer's.
pub struct Post {
    prefilter: Pipeline,
    down: Pipeline,
    up: Pipeline,
    fin: Pipeline,
    /// The back buffer's format `fin` was made for.
    pub out: Format,
}

fn level_size(n: u32, k: usize) -> u32 {
    ((n + (1 << (k + 1)) - 1) >> (k + 1)).max(1)
}

impl Targets {
    /// Targets of `w` x `h` for a depth format `zfmt`, trying the configuration first,
    /// then without MSAA, then 8-bit. None when the device makes none of them.
    pub fn create(gpu: &Gpu, w: u32, h: u32, zfmt: Format, cfg: &Config) -> Option<Targets> {
        let ms = if cfg.msaa >= 2 { cfg.msaa } else { 0 };
        let mut tries: Vec<(Format, u32)> = Vec::new();
        if cfg.hdr {
            tries.push((Format::Rgba16F, ms));
            if ms != 0 {
                tries.push((Format::Rgba16F, 0));
            }
        }
        tries.push((Format::Bgra8, ms));
        if ms != 0 {
            tries.push((Format::Bgra8, 0));
        }
        for (fmt, samples) in tries {
            if let Some(t) = Self::try_create(gpu, w, h, fmt, samples, zfmt) {
                if fmt != Format::Rgba16F && cfg.hdr || samples != ms {
                    log!("frame: fell back to format {} with {} samples", gpu::d3d9::format_code(fmt), samples);
                }
                return Some(t);
            }
        }
        None
    }

    fn try_create(gpu: &Gpu, w: u32, h: u32, fmt: Format, samples: u32, zfmt: Format) -> Option<Targets> {
        let make = |w: u32, h: u32, f: Format, s: u32, usage: Usage| {
            let t = gpu.texture(TextureDesc { samples: s.max(1), ..TextureDesc::d2(w, h, f, usage) }, Contents::None);
            t.ready(gpu).then_some(t)
        };
        let target = Usage::RENDER_TARGET | Usage::SAMPLED | Usage::COPY_SRC;
        let resolve = make(w, h, fmt, 1, target | Usage::COPY_DST)?;
        let scene = if samples != 0 { make(w, h, fmt, samples, Usage::RENDER_TARGET | Usage::COPY_SRC)? } else { resolve.clone() };
        let depth = make(w, h, zfmt, samples, Usage::DEPTH)?;
        let mut levels = Vec::new();
        for k in 0..BLOOM_LEVELS {
            let (lw, lh) = (level_size(w, k), level_size(h, k));
            levels.push(Level { tex: make(lw, lh, fmt, 1, target)?, w: lw, h: lh });
        }
        Some(Targets { w, h, fmt, samples, scene, depth, resolve, levels })
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx([f32; 8]);

static LAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float4, Semantic::Position, 0),
    attr(16, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(24, AttrFormat::Float2, Semantic::TexCoord, 1),
];

/// A quad over `r` of a `dw` x `dh` target, `a` and `b` the two sets of texture
/// coordinates at its corners (u0, v0, u1, v1). Positions are shifted half a pixel:
/// Direct3D 9 puts pixel centres on integers.
fn quad(cmd: &mut Cmd, dw: u32, dh: u32, r: PRect, a: [f32; 4], b: [f32; 4]) {
    let x0 = (r.x as f32 - 0.5) / dw as f32 * 2.0 - 1.0;
    let x1 = ((r.x + r.w) as f32 - 0.5) / dw as f32 * 2.0 - 1.0;
    let y0 = 1.0 - (r.y as f32 - 0.5) / dh as f32 * 2.0;
    let y1 = 1.0 - ((r.y + r.h) as f32 - 0.5) / dh as f32 * 2.0;
    let v = [
        Vtx([x0, y0, 0.5, 1.0, a[0], a[1], b[0], b[1]]),
        Vtx([x1, y0, 0.5, 1.0, a[2], a[1], b[2], b[1]]),
        Vtx([x0, y1, 0.5, 1.0, a[0], a[3], b[0], b[3]]),
        Vtx([x1, y1, 0.5, 1.0, a[2], a[3], b[2], b[3]]),
    ];
    if let Some(s) = cmd.vertices(&v) {
        cmd.draw(&s);
    }
}

/// Texture coordinates of rectangle `r` in a `w` x `h` texture, and the clamp that
/// keeps bilinear taps inside it (half a texel in from each edge).
fn uv(r: PRect, w: u32, h: u32) -> ([f32; 4], [f32; 4]) {
    let (fw, fh) = (w as f32, h as f32);
    let rect = [r.x as f32 / fw, r.y as f32 / fh, (r.x + r.w) as f32 / fw, (r.y + r.h) as f32 / fh];
    let clamp = [(r.x as f32 + 0.5) / fw, (r.y as f32 + 0.5) / fh, ((r.x + r.w) as f32 - 0.5) / fw, ((r.y + r.h) as f32 - 0.5) / fh];
    (rect, clamp)
}

fn area(r: PRect) -> Area {
    Area { x: r.x, y: r.y, w: r.w, h: r.h }
}

fn level_rect(view: PRect, k: usize) -> PRect {
    PRect { x: 0, y: 0, w: level_size(view.w, k), h: level_size(view.h, k) }
}

/// The pixel shader's four constants (c0..c3: post.hlsl's `k0`..`k3`).
#[repr(C)]
#[derive(Clone, Copy)]
struct Consts([[f32; 4]; 4]);

/// A pass into one target, keeping what is there, with no depth: a multisampled
/// depth may not go with a 1x target.
fn into(cmd: &mut Cmd, t: &Texture) -> bool {
    cmd.begin_pass(&Pass { colors: &[ColorTarget::new(t, Load::Load)], depth: None, area: None })
}

impl Post {
    /// The pipelines, for targets of `fmt` and a back buffer of `out`. None if the
    /// device refuses a shader.
    pub fn create(gpu: &Gpu, fmt: Format, out: Format) -> Option<Post> {
        let p = |ps: &'static [u32], target: Format, blend: Option<Blend>| {
            let mut d = PipelineDesc::new(Code::d3d9(&VS_QUAD), Code::d3d9(ps), &LAYOUT, 32, Topology::TriangleStrip, target);
            d.blend = blend;
            let p = gpu.pipeline(d);
            p.ready(gpu).then_some(p)
        };
        Some(Post {
            prefilter: p(&PS_PREFILTER, fmt, None)?,
            down: p(&PS_DOWN, fmt, None)?,
            up: p(&PS_UP, fmt, Some(Blend::ADD))?,
            fin: p(&PS_FINAL, out, None)?,
            out,
        })
    }

    fn ps(cmd: &mut Cmd, p: &Pipeline, src: Option<&Texture>, k: [[f32; 4]; 4]) {
        cmd.set_pipeline(p);
        cmd.bind_texture(0, src, &Sampler::LINEAR_CLAMP);
        let c = Consts(k);
        cmd.ps_block(Block::of(0, &c));
    }

    /// Everything after the scene for one view: `view` is the view's rectangle in the
    /// scene target, `out` the engine's render target (the back buffer), `out_depth`
    /// the engine's depth, which is cleared in the rectangle for the HUD. Leaves them
    /// bound. With `dump`, each stage is read back as it is made (`crate::dump`,
    /// README.md, "Frame dumps"), its files named after `label` (the view).
    pub fn run(&self, cmd: &mut Cmd, t: &Targets, view: PRect, out: &Texture, out_depth: Option<&Texture>, cfg: &Config,
        mut dump: Option<(&mut Dump, &str)>) {
        let gpu = cmd.gpu();
        // 1. Resolve the view's samples.
        if t.samples != 0 {
            cmd.resolve(&t.scene, &t.resolve, view.rect());
        }
        if let Some((d, label)) = dump.as_mut() {
            d.note(&crate::sfmt!("view {} {}x{} at {},{}", label, view.w, view.h, view.x, view.y));
            d.surface(gpu, &t.resolve, Some(area(view)), &crate::sfmt!("{}-scene", label), Kind::Pfm);
        }
        let hdr = t.fmt == Format::Rgba16F;
        let bloom = hdr && cfg.bloom > 0.0;
        // 2. Bloom: prefilter into level 0, down the chain, back up with additive blends.
        if bloom {
            let (src_uv, src_clamp) = uv(view, t.w, t.h);
            let l0 = &t.levels[0];
            into(cmd, &l0.tex);
            Self::ps(cmd, &self.prefilter, Some(&t.resolve), [[1.0 / t.w as f32, 1.0 / t.h as f32, 0.0, 0.0], src_clamp,
                [cfg.bloom_threshold, cfg.bloom_knee, 0.0, 0.0], [0.0; 4]]);
            quad(cmd, l0.w, l0.h, level_rect(view, 0), src_uv, src_uv);
            for k in 1..BLOOM_LEVELS {
                let (s, d) = (&t.levels[k - 1], &t.levels[k]);
                let (su, sc) = uv(level_rect(view, k - 1), s.w, s.h);
                into(cmd, &d.tex);
                Self::ps(cmd, &self.down, Some(&s.tex), [[1.0 / s.w as f32, 1.0 / s.h as f32, 0.0, 0.0], sc, [0.0; 4], [0.0; 4]]);
                quad(cmd, d.w, d.h, level_rect(view, k), su, su);
            }
            if let Some((d, label)) = dump.as_mut() {
                for (k, l) in t.levels.iter().enumerate() {
                    d.surface(gpu, &l.tex, Some(area(level_rect(view, k))), &crate::sfmt!("{}-bloom{}-down", label, k), Kind::Pfm);
                }
            }
            for k in (1..BLOOM_LEVELS).rev() {
                let (s, d) = (&t.levels[k], &t.levels[k - 1]);
                let (su, sc) = uv(level_rect(view, k), s.w, s.h);
                into(cmd, &d.tex);
                Self::ps(cmd, &self.up, Some(&s.tex), [[1.0 / s.w as f32, 1.0 / s.h as f32, 0.0, 0.0], sc, [1.0, 0.0, 0.0, 0.0], [0.0; 4]]);
                quad(cmd, d.w, d.h, level_rect(view, k - 1), su, su);
            }
            // The way up adds each level onto the one above; the last, level 4, is as the
            // way down left it.
            if let Some((d, label)) = dump.as_mut() {
                for (k, l) in t.levels.iter().enumerate().take(BLOOM_LEVELS - 1) {
                    d.surface(gpu, &l.tex, Some(area(level_rect(view, k))), &crate::sfmt!("{}-bloom{}-up", label, k), Kind::Pfm);
                }
            }
        } else if let Some((d, _)) = dump.as_mut() {
            d.note("bloom: off (an 8-bit scene target, or Bloom=0): no levels");
        }
        // 3. Tone map into the back buffer at the view's rectangle, with the engine's
        // depth bound again (the pipeline does not test it). The scene is read a texel
        // per pixel. With MSAA, the view's first column and row take their neighbours':
        // the scene starts half a pixel in (DXVK's viewport offset), so half their
        // samples are never drawn.
        let colors = [ColorTarget::new(out, Load::Load)];
        let depth = out_depth.map(|z| DepthTarget { texture: z, load: Load::Load });
        if !cmd.begin_pass(&Pass { colors: &colors, depth, area: None }) {
            return;
        }
        let (ow, oh) = out.size();
        let l0 = &t.levels[0];
        let (su, mut sc) = uv(view, t.w, t.h);
        if t.samples != 0 && view.w > 2 && view.h > 2 {
            sc[0] += 1.0 / t.w as f32;
            sc[1] += 1.0 / t.h as f32;
        }
        let (bu, bc) = uv(level_rect(view, 0), l0.w, l0.h);
        let shoulder = if hdr { cfg.shoulder.clamp(0.0, 0.99) } else { 1.0e9 };
        cmd.set_pipeline(&self.fin);
        cmd.bind_texture(0, Some(&t.resolve), &Sampler::POINT_CLAMP);
        cmd.bind_texture(1, if bloom { Some(&l0.tex) } else { None }, &Sampler::LINEAR_CLAMP);
        let c = Consts([[0.0; 4], sc, [if bloom { cfg.bloom } else { 0.0 }, cfg.exposure, shoulder, 0.0], bc]);
        cmd.ps_block(Block::of(0, &c));
        quad(cmd, ow, oh, view, su, bu);
        if let Some((d, label)) = dump.as_mut() {
            d.surface(gpu, out, None, &crate::sfmt!("{}-tonemapped", label), Kind::Ppm);
            d.note("depth: not dumped (Direct3D 9 cannot read a depth surface back with GetRenderTargetData)");
        }
        // 4. The engine's depth, for the HUD drawn over the view next.
        if out_depth.is_some() {
            cmd.clear_rects(&[view.rect()], None, Some(1.0));
        }
        cmd.end_pass();
    }
}
