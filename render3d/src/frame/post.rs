//! The frame's own targets and the passes after the scene: resolve, bloom, tone map.
//!
//! One set of targets the size of the back buffer serves both views, since a view's
//! viewport is a rectangle of the back buffer in either. Every pass works inside the
//! view's rectangle (scaled down for each bloom level), and every tap is clamped to it.
use crate::config::Config;
use crate::d3d::*;
use crate::shaders::*;
use alloc::vec::Vec;
use crate::log;
use core::ffi::c_void;

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
    pub fn d3d(self) -> Rect {
        Rect { left: self.x as i32, top: self.y as i32, right: (self.x + self.w) as i32, bottom: (self.y + self.h) as i32 }
    }
}

struct Level {
    tex: Com,
    surf: Com,
    w: u32,
    h: u32,
}

/// Everything the frame holds on one device. All of it is dropped before a `Reset`.
pub struct Targets {
    pub w: u32,
    pub h: u32,
    pub fmt: u32,
    pub samples: u32,
    /// The scene target: multisampled, or the resolve texture's own surface at 1x.
    pub scene: Com,
    pub depth: Com,
    resolve: Com,
    resolve_surf: Com,
    levels: Vec<Level>,
}

pub struct Post {
    vs: Com,
    ps_prefilter: Com,
    ps_down: Com,
    ps_up: Com,
    ps_final: Com,
    decl: Com,
    sb: Com,
}

fn level_size(n: u32, k: usize) -> u32 {
    ((n + (1 << (k + 1)) - 1) >> (k + 1)).max(1)
}

impl Targets {
    /// Targets of `w` x `h` for a depth format `zfmt`, trying the configuration first,
    /// then without MSAA, then 8-bit. None when the device makes none of them.
    pub fn create(dev: Device, w: u32, h: u32, zfmt: u32, cfg: &Config) -> Option<Targets> {
        let ms = if cfg.msaa >= 2 { cfg.msaa } else { 0 };
        let mut tries: Vec<(u32, u32)> = Vec::new();
        if cfg.hdr {
            tries.push((FMT_A16B16G16R16F, ms));
            if ms != 0 {
                tries.push((FMT_A16B16G16R16F, 0));
            }
        }
        tries.push((FMT_A8R8G8B8, ms));
        if ms != 0 {
            tries.push((FMT_A8R8G8B8, 0));
        }
        for (fmt, samples) in tries {
            if let Some(t) = Self::try_create(dev, w, h, fmt, samples, zfmt) {
                if fmt != FMT_A16B16G16R16F && cfg.hdr || samples != ms {
                    log!("frame: fell back to format {} with {} samples", fmt, samples);
                }
                return Some(t);
            }
        }
        None
    }

    fn try_create(dev: Device, w: u32, h: u32, fmt: u32, samples: u32, zfmt: u32) -> Option<Targets> {
        let resolve = dev.create_texture(w, h, 1, USAGE_RENDERTARGET, fmt, POOL_DEFAULT)?;
        let resolve_surf = resolve.tex_surface(0)?;
        let scene = if samples != 0 {
            dev.create_render_target(w, h, fmt, samples, false)?
        } else {
            Com::from_borrowed(resolve_surf.ptr())
        };
        let depth = dev.create_depth_stencil(w, h, zfmt, samples)?;
        let mut levels = Vec::new();
        for k in 0..BLOOM_LEVELS {
            let (lw, lh) = (level_size(w, k), level_size(h, k));
            let tex = dev.create_texture(lw, lh, 1, USAGE_RENDERTARGET, fmt, POOL_DEFAULT)?;
            let surf = tex.tex_surface(0)?;
            levels.push(Level { tex, surf, w: lw, h: lh });
        }
        Some(Targets { w, h, fmt, samples, scene, depth, resolve, resolve_surf, levels })
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx([f32; 8]);

/// A quad over `r` of a `dw` x `dh` target, `a` and `b` the two sets of texture
/// coordinates at its corners (u0, v0, u1, v1). Positions are shifted half a pixel:
/// Direct3D 9 puts pixel centres on integers.
fn quad(dev: Device, dw: u32, dh: u32, r: PRect, a: [f32; 4], b: [f32; 4]) {
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
    dev.draw_primitive_up(PT_TRIANGLESTRIP, 2, v.as_ptr() as *const u8, 32);
}

/// Texture coordinates of rectangle `r` in a `w` x `h` texture, and the clamp that
/// keeps bilinear taps inside it (half a texel in from each edge).
fn uv(r: PRect, w: u32, h: u32) -> ([f32; 4], [f32; 4]) {
    let (fw, fh) = (w as f32, h as f32);
    let rect = [r.x as f32 / fw, r.y as f32 / fh, (r.x + r.w) as f32 / fw, (r.y + r.h) as f32 / fh];
    let clamp = [(r.x as f32 + 0.5) / fw, (r.y as f32 + 0.5) / fh, ((r.x + r.w) as f32 - 0.5) / fw, ((r.y + r.h) as f32 - 0.5) / fh];
    (rect, clamp)
}

fn level_rect(view: PRect, k: usize) -> PRect {
    PRect { x: 0, y: 0, w: level_size(view.w, k), h: level_size(view.h, k) }
}

impl Post {
    pub fn create(dev: Device) -> Option<Post> {
        let decl = dev.create_vertex_decl(&[
            ve(0, 0, DECLTYPE_FLOAT4, DECLUSAGE_POSITION, 0),
            ve(0, 16, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
            ve(0, 24, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 1),
            VE_END,
        ])?;
        Some(Post {
            vs: dev.create_vs(&VS_QUAD)?,
            ps_prefilter: dev.create_ps(&PS_PREFILTER)?,
            ps_down: dev.create_ps(&PS_DOWN)?,
            ps_up: dev.create_ps(&PS_UP)?,
            ps_final: dev.create_ps(&PS_FINAL)?,
            decl,
            sb: dev.create_state_block(SBT_ALL)?,
        })
    }

    fn states(&self, dev: Device) {
        for (s, v) in [
            (RS_ZENABLE, 0), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0),
            (RS_CULLMODE, CULL_NONE), (RS_STENCILENABLE, 0), (RS_SCISSORTESTENABLE, 0), (RS_FOGENABLE, 0),
            (RS_COLORWRITEENABLE, 0xf), (RS_SRGBWRITEENABLE, 0), (RS_FILLMODE, 3), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_CLIPPING, 1),
        ] {
            dev.set_rs(s, v);
        }
        for s in 0..2 {
            dev.set_ss(s, SAMP_ADDRESSU, TADDRESS_CLAMP);
            dev.set_ss(s, SAMP_ADDRESSV, TADDRESS_CLAMP);
            dev.set_ss(s, SAMP_MAGFILTER, TEXF_LINEAR);
            dev.set_ss(s, SAMP_MINFILTER, TEXF_LINEAR);
            dev.set_ss(s, SAMP_MIPFILTER, TEXF_NONE);
            dev.set_ss(s, SAMP_SRGBTEXTURE, 0);
        }
        dev.set_stream_freq(0, 1);
        dev.set_vertex_decl(self.decl.ptr());
        dev.set_vs(self.vs.ptr());
    }

    fn ps(&self, dev: Device, ps: &Com, k0: [f32; 4], k1: [f32; 4], k2: [f32; 4], k3: [f32; 4]) {
        dev.set_ps(ps.ptr());
        let c: [f32; 16] = [
            k0[0], k0[1], k0[2], k0[3], k1[0], k1[1], k1[2], k1[3], k2[0], k2[1], k2[2], k2[3], k3[0], k3[1], k3[2], k3[3],
        ];
        dev.ps_const(0, &c);
    }

    /// Everything after the scene for one view: `view` is the view's rectangle in the
    /// scene target, `out` the engine's render target (the back buffer), `out_depth`
    /// the engine's depth, which is cleared in the rectangle for the HUD. The caller has
    /// put the engine's target and depth back already or not; this leaves them set.
    pub fn run(&self, dev: Device, t: &Targets, view: PRect, out: &Com, out_depth: Option<&Com>, cfg: &Config) {
        self.sb.sb_capture();
        // 1. Resolve the view's samples.
        if t.samples != 0 {
            let r = view.d3d();
            dev.stretch_rect(&t.scene, Some(&r), &t.resolve_surf, Some(&r), TEXF_NONE);
        }
        // No depth while the bloom levels are drawn: a multisampled depth may not go with
        // a 1x target.
        dev.set_depth_stencil(core::ptr::null_mut());
        self.states(dev);
        let hdr = t.fmt == FMT_A16B16G16R16F;
        let bloom = hdr && cfg.bloom > 0.0;
        // 2. Bloom: prefilter into level 0, down the chain, back up with additive blends.
        if bloom {
            let (src_uv, src_clamp) = uv(view, t.w, t.h);
            let l0 = &t.levels[0];
            dev.set_render_target(0, l0.surf.ptr());
            dev.set_texture(0, t.resolve.ptr());
            self.ps(dev, &self.ps_prefilter, [1.0 / t.w as f32, 1.0 / t.h as f32, 0.0, 0.0], src_clamp,
                [cfg.bloom_threshold, cfg.bloom_knee, 0.0, 0.0], [0.0; 4]);
            quad(dev, l0.w, l0.h, level_rect(view, 0), src_uv, src_uv);
            for k in 1..BLOOM_LEVELS {
                let (s, d) = (&t.levels[k - 1], &t.levels[k]);
                let (su, sc) = uv(level_rect(view, k - 1), s.w, s.h);
                dev.set_render_target(0, d.surf.ptr());
                dev.set_texture(0, s.tex.ptr());
                self.ps(dev, &self.ps_down, [1.0 / s.w as f32, 1.0 / s.h as f32, 0.0, 0.0], sc, [0.0; 4], [0.0; 4]);
                quad(dev, d.w, d.h, level_rect(view, k), su, su);
            }
            dev.set_rs(RS_ALPHABLENDENABLE, 1);
            dev.set_rs(RS_SRCBLEND, BLEND_ONE);
            dev.set_rs(RS_DESTBLEND, BLEND_ONE);
            for k in (1..BLOOM_LEVELS).rev() {
                let (s, d) = (&t.levels[k], &t.levels[k - 1]);
                let (su, sc) = uv(level_rect(view, k), s.w, s.h);
                dev.set_render_target(0, d.surf.ptr());
                dev.set_texture(0, s.tex.ptr());
                self.ps(dev, &self.ps_up, [1.0 / s.w as f32, 1.0 / s.h as f32, 0.0, 0.0], sc, [1.0, 0.0, 0.0, 0.0], [0.0; 4]);
                quad(dev, d.w, d.h, level_rect(view, k - 1), su, su);
            }
            dev.set_rs(RS_ALPHABLENDENABLE, 0);
        }
        // 3. Tone map into the back buffer at the view's rectangle. The scene is read a
        // texel per pixel. With MSAA, the view's first column and row take their
        // neighbours': the scene starts half a pixel in (DXVK's viewport offset), so
        // half their samples are never drawn (msaa/README.md, "The top row and left
        // column").
        dev.set_render_target(0, out.ptr());
        let od = out.surf_desc();
        dev.set_texture(0, t.resolve.ptr());
        let l0 = &t.levels[0];
        dev.set_texture(1, if bloom { l0.tex.ptr() } else { core::ptr::null_mut::<c_void>() });
        dev.set_ss(0, SAMP_MAGFILTER, TEXF_POINT);
        dev.set_ss(0, SAMP_MINFILTER, TEXF_POINT);
        let (su, mut sc) = uv(view, t.w, t.h);
        if t.samples != 0 && view.w > 2 && view.h > 2 {
            sc[0] += 1.0 / t.w as f32;
            sc[1] += 1.0 / t.h as f32;
        }
        let (bu, bc) = uv(level_rect(view, 0), l0.w, l0.h);
        let shoulder = if hdr { cfg.shoulder.clamp(0.0, 0.99) } else { 1.0e9 };
        self.ps(dev, &self.ps_final, [0.0; 4], sc, [if bloom { cfg.bloom } else { 0.0 }, cfg.exposure, shoulder, 0.0], bc);
        quad(dev, od.width, od.height, view, su, bu);
        // 4. The engine's depth, for the HUD drawn over the view next.
        if let Some(z) = out_depth {
            dev.set_depth_stencil(z.ptr());
            let vp = Viewport { x: 0, y: 0, w: od.width, h: od.height, min_z: 0.0, max_z: 1.0 };
            dev.set_viewport(&vp);
            dev.clear(&[view.d3d()], CLEAR_ZBUFFER, 0, 1.0, 0);
        }
        dev.set_texture(0, core::ptr::null_mut());
        dev.set_texture(1, core::ptr::null_mut());
        self.sb.sb_apply();
    }
}
