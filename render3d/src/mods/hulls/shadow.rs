//! Shadows from this frame's hulls: at the start of each view, every hull that can
//! throw a shadow into it is drawn into a depth map from the Key, before any step
//! draws. `Lighting.asi` could only use the last frame's hulls (it saw them only as the
//! engine drew them), and looked each draw up where it had been; here nothing lags.
//! The map is fitted round those hulls, its width in steps of 2^(1/4) and its centre
//! snapped to whole texels so a shadow's edge does not crawl as the camera moves
//! (lighting/README.md, "Shadows").
//!
//! The planets cast no shadow into the map: the hull shader tests up to eight of them
//! as spheres (`planets()`), as `Lighting.asi` did.
use super::look::Look;
use super::{draw, Posed};
use crate::d3d::*;
use crate::engine;
use crate::frame::steps::View;
use crate::gfx::{Decl, Guard, Shader};
use crate::math::{v3, V3};
use crate::scene::{self, Blend};
use crate::shaders::hulls::*;
use crate::log;
use alloc::collections::BTreeMap;
use core::ffi::c_void;

/// The shadow map of the view being drawn.
#[derive(Clone, Copy)]
pub struct ShadowMap {
    /// R32F: depth 0..1 from the Key, 1 where no hull is. Valid until the next view.
    pub tex: *mut c_void,
    /// World -> u, v, depth, as rows (x, y, z, w) dotted with (p, 1).
    pub rows: [[f32; 4]; 3],
    /// A texel, in u and v.
    pub texel: f32,
    /// Half a texel's depth, in depth units.
    pub bias: f32,
    /// A texel in world units (the lookup moves a point out along its normal by it).
    pub normal_offset: f32,
    /// Towards the Key.
    pub to_key: V3,
    /// The map's width in world units, and its depth range.
    pub width: f32,
    pub range: f32,
}

static mut CURRENT: Option<ShadowMap> = None;
static mut PLANETS: [[f32; 4]; 8] = [[0.0; 4]; 8];
static mut SUN: V3 = v3(0.0, 1.0, 0.0);

pub fn current() -> Option<ShadowMap> {
    unsafe { CURRENT }
}

pub fn clear_current() {
    unsafe { CURRENT = None };
}

/// The planets the shader tests (centre, radius) and the direction towards the Key.
pub fn planets() -> ([[f32; 4]; 8], V3) {
    unsafe { (PLANETS, SUN) }
}

const VT_PLANET: usize = 0x6b2b3c;

pub struct Shadow {
    vs: Shader,
    ps: Shader,
    decl: Decl,
    target: Option<(Com, Com, Com, u32)>,
    said: bool,
    failed: bool,
}

impl Shadow {
    pub fn new() -> Shadow {
        Shadow { vs: Shader::vertex(&VS_DEPTH), ps: Shader::pixel(&PS_DEPTH), decl: Decl::new(&scene::VERTEX), target: None, said: false, failed: false }
    }

    pub fn reset(&mut self) {
        self.target = None;
        self.failed = false;
        clear_current();
    }

    fn targets(&mut self, dev: Device, size: u32) -> Option<(*mut c_void, *mut c_void, *mut c_void)> {
        if self.target.as_ref().is_none_or(|t| t.3 != size) && !self.failed {
            self.target = None;
            let tex = dev.create_texture(size, size, 1, USAGE_RENDERTARGET, FMT_R32F, POOL_DEFAULT);
            let depth = dev.create_depth_stencil(size, size, FMT_D24X8, 0);
            match (tex, depth) {
                (Some(t), Some(d)) => {
                    let s = t.tex_surface(0)?;
                    self.target = Some((t, s, d, size));
                    log!("hulls: shadow map {}x{}", size, size);
                }
                _ => {
                    log!("hulls: no shadow map ({}x{} R32F)", size, size);
                    self.failed = true;
                }
            }
        }
        self.target.as_ref().map(|t| (t.0.ptr(), t.1.ptr(), t.2.ptr()))
    }

    /// Draws the view's shadow map from its posed hulls, and finds its planets.
    pub fn render(&mut self, v: &View, look: &Look, posed: &BTreeMap<usize, Posed>, _d: &mut draw::Drawer) {
        clear_current();
        let ls = scene::lights();
        let Some(key) = ls.key() else { return };
        let to_key = -key.dir;
        unsafe { SUN = to_key };
        self.find_planets();
        // Casters: the hulls whose sphere, stretched back towards the Key, reaches the
        // view (a hull just outside it can still shade one inside).
        let vp = v.view.mul(&v.proj);
        let mut casters: alloc::vec::Vec<&Posed> = alloc::vec::Vec::new();
        for p in posed.values() {
            if let Some((_, vis, _)) = p.cloak {
                if vis < 0.5 {
                    continue;
                }
            }
            if in_view(&vp, p.centre, p.radius * 2.0) || in_view(&vp, p.centre + to_key * (p.radius * 4.0), p.radius * 3.0) {
                casters.push(p);
            }
        }
        if casters.is_empty() {
            return;
        }
        // The map's frame: u, v across the Key, w along its travel.
        let w = key.dir;
        let a = if w.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
        let u = a.cross(w).norm();
        let vv = w.cross(u).norm();
        let (mut lo, mut hi) = (v3(f32::MAX, f32::MAX, f32::MAX), v3(f32::MIN, f32::MIN, f32::MIN));
        for p in &casters {
            let c = v3(p.centre.dot(u), p.centre.dot(vv), p.centre.dot(w));
            let r = p.radius;
            lo = v3(lo.x.min(c.x - r), lo.y.min(c.y - r), lo.z.min(c.z - r));
            hi = v3(hi.x.max(c.x + r), hi.y.max(c.y + r), hi.z.max(c.z + r));
        }
        let size = look.shadow_size;
        let ext = (hi.x - lo.x).max(hi.y - lo.y).max(1.0) * 1.02;
        // The width in steps of 2^(1/4), the centre on whole texels.
        let step = libm::ceilf(libm::log2f(ext) * 4.0) / 4.0;
        let width = libm::exp2f(step);
        let texel_w = width / size as f32;
        let cu = libm::floorf((lo.x + hi.x) * 0.5 / texel_w) * texel_w;
        let cv = libm::floorf((lo.y + hi.y) * 0.5 / texel_w) * texel_w;
        let (z0, z1) = (lo.z - 1.0, hi.z + 1.0);
        let range = (z1 - z0).max(1.0);
        let rows = [
            [u.x / width, u.y / width, u.z / width, 0.5 - cu / width],
            [-vv.x / width, -vv.y / width, -vv.z / width, 0.5 + cv / width],
            [w.x / range, w.y / range, w.z / range, -z0 / range],
        ];
        let dev = v.dev;
        let Some((tex, surf, depth)) = self.targets(dev, size) else { return };
        let _g = Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            return;
        }
        let (Some(rt0), ds0) = (dev.get_render_target(0), dev.get_depth_stencil()) else { return };
        let vp0 = dev.get_viewport();
        dev.set_render_target(0, surf);
        dev.set_depth_stencil(depth);
        dev.set_viewport(&Viewport { x: 0, y: 0, w: size, h: size, min_z: 0.0, max_z: 1.0 });
        // Depth 1 (no hull) everywhere; R32F clears from the colour's red.
        dev.clear(&[], CLEAR_TARGET | CLEAR_ZBUFFER, 0xffff_ffff, 1.0, 0);
        for (s, val) in [
            (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHABLENDENABLE, 0),
            (RS_ALPHATESTENABLE, 0), (RS_CULLMODE, CULL_NONE), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
            (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_MULTISAMPLEANTIALIAS, 0),
        ] {
            dev.set_rs(s, val);
        }
        dev.set_stream_freq(0, 1);
        let mut vk = [0.0f32; 4 * 10];
        for r in 0..3 {
            vk[8 + r * 4..12 + r * 4].copy_from_slice(&rows[r]);
        }
        dev.vs_const(17, &[1.0 / size as f32, 0.0, 0.0, 0.0]);
        let mut parts = 0;
        for p in &casters {
            let mut k = vk;
            if let Some(wp) = p.warp {
                k[0..4].copy_from_slice(&[wp.n.x, wp.n.y, wp.n.z, wp.d]);
                k[4..8].copy_from_slice(&[wp.s1, wp.o, 1.0, 0.0]);
            }
            dev.vs_const(8, &k[..20]);
            let limit = engine::field(p.inst, 0x10) as i32;
            let mut faces_left: i64 = if limit >= 0 { limit as i64 } else { i64::MAX };
            for part in &p.model.parts {
                if part.blend != Blend::Opaque || !p.pose.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                // Cut-out meshes (a `default` mesh with a group that is not
                // self-illuminating) cast nothing: the depth pass has no texture, and
                // would cast their holes (as Lighting.asi left alpha-tested draws out).
                let default_mat = p.model.sod.nodes[part.node].mesh().is_some_and(|ms| ms.texture_material.eq_ignore_ascii_case("default"));
                if default_mat && part.groups.iter().any(|g| !g.self_illum) {
                    continue;
                }
                if faces_left <= 0 {
                    break;
                }
                let world = p.pose.world[part.node];
                dev.vs_const(0, world.to_m4().transpose().as_floats());
                dev.set_stream(0, part.vb.ptr(), 0, 32);
                dev.set_indices(part.ib.ptr());
                for g in &part.groups {
                    let tris = (g.tris as i64).min(faces_left) as u32;
                    if tris == 0 {
                        break;
                    }
                    faces_left -= tris as i64;
                    dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, tris);
                }
                parts += 1;
            }
        }
        dev.set_render_target(0, rt0.ptr());
        dev.set_depth_stencil(ds0.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
        dev.set_viewport(&vp0);
        let texel = 1.0 / size as f32;
        unsafe {
            CURRENT = Some(ShadowMap {
                tex,
                rows,
                texel,
                bias: 0.5 * texel_w / range,
                normal_offset: texel_w,
                to_key,
                width,
                range,
            });
        }
        if !self.said {
            self.said = true;
            log!("hulls: shadow map of {} hulls ({} meshes), {:.0} units wide, {:.3} a texel", casters.len(), parts, width, texel_w);
        }
    }

    fn find_planets(&mut self) {
        let mut n = 0;
        let mut out = [[0.0f32; 4]; 8];
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if n >= 8 || o == 0 || engine::vtable(o) != VT_PLANET {
                return;
            }
            let inst = engine::field(o, 4);
            let r = engine::field_f32(inst, 0x40);
            if !(r > 0.0) {
                return;
            }
            let c = engine::field_v3(inst, 0x34);
            let t = engine::field_v3(inst, 0x44 + 0x24);
            out[n] = [t.x + c.x, t.y + c.y, t.z + c.z, r];
            n += 1;
        });
        unsafe { PLANETS = out };
    }
}

/// Whether a sphere is (nearly) in the view: its centre through view x projection
/// against the clip volume, grown by its radius.
fn in_view(vp: &crate::math::M4, c: V3, r: f32) -> bool {
    let q = vp.transform([c.x, c.y, c.z, 1.0]);
    let w = q[3];
    if w + r < 0.0 {
        return false;
    }
    // Clip coordinates scale with w; a radius r spans about r times the projection's
    // scale, which is under 3 for any sane field of view.
    let m = r * 3.0;
    q[0].abs() <= w + m && q[1].abs() <= w + m
}
