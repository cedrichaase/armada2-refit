//! Drawing a posed hull in the hull shaders (`shaders/hulls.hlsl`).
use super::look::{Look, Profile};
use super::{anim, game, shadow, Posed};
use crate::d3d::*;
use crate::engine::{self, field};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Guard, Shader};
use crate::math::{v3, M34, V3};
use crate::scene::{self, Blend, Model};
use crate::shaders::hulls::*;
use crate::{assets, log};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;

/// How a hull is drawn.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    /// Opaque and cut-out meshes, lit.
    Opaque,
    /// Into depth alone (the first pass of a faded hull).
    DepthOnly,
    /// Opaque and cut-out meshes blended by the fade, where they are nearest.
    Faded(f32),
    /// The additive meshes, times this.
    Additive(f32),
    /// As glass over a copy of the frame.
    Cloak,
    /// The build ghost (`ps_ghost`, constants from `Drawer::extra`), premultiplied,
    /// where it is nearest (after `DepthOnly`).
    Ghost,
    /// A hologram (`ps_holo`, constants from `Drawer::extra`), likewise.
    Holo,
}

/// The warp stretch, in world space (`spec-chunks-anim-lod.md` section 5).
#[derive(Clone, Copy)]
pub struct Warp {
    pub n: V3,
    pub d: f32,
    pub s1: f32,
    pub o: f32,
}

impl Warp {
    /// From the craft's ramp `b`, stretch `s`, its motion `n` (object space), its
    /// placement and the instance's bounding sphere (database space, which the
    /// instance's scale takes to the object's).
    pub fn new(b: f32, s: f32, n: V3, xf: &M34, inst: usize, scale: V3) -> Warp {
        let (mut n, mut s) = (n, s);
        if s < 0.0 {
            n = -n;
            s = -s;
        }
        let (c, r) = game::instance_sphere(inst);
        let c = v3(c.x * scale.x, c.y * scale.y, c.z * scale.z);
        let r = r * scale.x;
        let cw = xf.transform_point(c);
        let nw = super::norm_or(xf.transform_dir(n), n);
        let d = nw.dot(cw) + r - 2.0 * r * b;
        Warp { n: nw, d, s1: s - 1.0, o: (s - 1.0) * r * b }
    }
}

/// The wave light a draw of the object turns on (the first of them), as constants.
#[derive(Clone, Copy)]
pub struct Waves {
    pub colour: V3,
    pub dir: V3,
    /// 2 pi / wavelength.
    pub k: f32,
    /// Its phase now.
    pub phase: f32,
}

impl Waves {
    pub fn of(o: usize, craft: bool, now: f32) -> Option<Waves> {
        let two_pi = unsafe { engine::rd::<f32>(0x7913bc) };
        let two_pi = if two_pi.is_finite() && two_pi > 0.1 { two_pi } else { core::f32::consts::TAU };
        game::wave_lights(o, craft).iter().find_map(|&w| {
            let wv = game::wave(w)?;
            if !(wv.wavelength.abs() > 1e-4) {
                return None;
            }
            Some(Waves { colour: wv.colour, dir: wv.dir, k: two_pi / wv.wavelength, phase: now * wv.speed * two_pi })
        })
    }
}

/// How far a mesh reaches from its origin, at most, for picking a hard light.
const HARD_REACH: f32 = 150.0;

/// The point lights one mesh takes, as `Lighting.asi`'s `pick_points` chose them for
/// its shaders (lighting/README.md, "Point lights on the GPU path"), at the mesh's
/// origin `at`:
/// - a hard light (it fades over less than a quarter of its start: a torpedo's, a
///   discharge's, a weld's; or a module's light styled hard: a phaser's) is picked
///   within its reach plus `HARD_REACH`, scored by its peak times how near it is;
/// - a soft one (a nebula's, an explosion's) within its reach, scored by its falloff
///   at the origin times its peak;
/// - of lights of one colour only the strongest counts (a field of nebulae, a
///   volley), unless styled distinct (phasers fire from several banks at once), and
///   nothing scoring under 0.004.
/// The shader then takes each where it is (the shaders' convention, not the mirrored
/// one Lighting.asi used for Direct3D's fixed slots), its falloff per pixel, at its
/// full colour. Constants: position and start (c16+), colour and 1 / fade (c32+),
/// wrap and power (c48+), for `max` lights, the rest zero.
pub fn pick_points(at: V3, max: usize) -> [f32; 48 * 4] {
    struct Pick {
        col: V3,
        pos: V3,
        score: f32,
        start: f32,
        fade: f32,
        wrap: f32,
        power: f32,
        distinct: bool,
    }
    let mut out: Vec<Pick> = Vec::new();
    let max = max.min(16);
    let lights = scene::lights();
    for p in &lights.points {
        // A planet's day side lights only what faces it (scene::lights::set_day_side).
        let share = lights.share(p, at);
        if share <= 0.0 {
            continue;
        }
        let (start, range, col) = (p.start, p.range, p.colour * share);
        let d = (at - p.pos).len();
        let peak = col.x.max(col.y).max(col.z);
        let hard = p.style.hard || range < 0.25 * start;
        let score = if hard {
            let reach = start + range + HARD_REACH;
            if d >= reach {
                continue;
            }
            peak * (1.0 - d / reach)
        } else {
            if d >= start + range {
                continue;
            }
            let f = if d <= start { 1.0 } else { 1.0 - (d - start) / range.max(1e-3) };
            f * peak
        };
        if !(score >= 0.004) {
            continue;
        }
        let distinct = p.style.distinct;
        if !distinct {
            if let Some(i) = out.iter().position(|q| !q.distinct && q.col == col) {
                if out[i].score >= score {
                    continue;
                }
                out.remove(i);
            }
        }
        let i = out.iter().position(|q| q.score < score).unwrap_or(out.len());
        if i >= max {
            continue;
        }
        out.insert(i, Pick { col, pos: p.pos, score, start, fade: range, wrap: p.style.wrap, power: p.style.power, distinct });
        out.truncate(max);
    }
    let mut k = [0.0f32; 48 * 4];
    for i in 0..16 {
        k[(32 + i) * 4..(32 + i) * 4 + 4].copy_from_slice(&[0.0, 1.0, 0.0, 0.0]);
    }
    for (i, q) in out.iter().enumerate() {
        k[i * 4..i * 4 + 4].copy_from_slice(&[q.pos.x, q.pos.y, q.pos.z, q.start]);
        let inv = if q.fade > 1e-3 { 1.0 / q.fade } else { 1000.0 };
        k[(16 + i) * 4..(16 + i) * 4 + 4].copy_from_slice(&[q.col.x, q.col.y, q.col.z, inv]);
        k[(32 + i) * 4..(32 + i) * 4 + 4].copy_from_slice(&[q.wrap, q.power, 0.0, 0.0]);
    }
    k
}

/// The model's bounding sphere in its own space (every mesh's, static pose).
pub fn model_sphere(m: &Model) -> (V3, f32) {
    let mut lo = v3(f32::MAX, f32::MAX, f32::MAX);
    let mut hi = v3(f32::MIN, f32::MIN, f32::MIN);
    let mut any = false;
    for part in &m.parts {
        let xf = crate::scene::pose::m34(&m.sod.model_transform(part.node));
        let c = xf.transform_point(part.centre);
        let s = xf.r[0].len().max(xf.r[1].len()).max(xf.r[2].len());
        let r = part.radius * s;
        lo = v3(lo.x.min(c.x - r), lo.y.min(c.y - r), lo.z.min(c.z - r));
        hi = v3(hi.x.max(c.x + r), hi.y.max(c.y + r), hi.z.max(c.z + r));
        any = true;
    }
    if !any {
        return (V3::ZERO, 1.0);
    }
    let c = (lo + hi) * 0.5;
    (c, (hi - lo).len() * 0.5)
}

/// The fade a hull is drawn at: the engine's far fade, or (crafts) Lighting.asi's near
/// fade, which waits for the near plane to cut into the hull's box
/// (lighting/README.md, "The near fade").
pub fn fade(p: &Posed, v: &View, look: &Look) -> f32 {
    if engine::field(engine::storm3d(), 0xe8) == 0 {
        return 1.0;
    }
    if let Some(f) = game::far_fade(p.inst) {
        return f.clamp(0.0, 1.0);
    }
    if !game::is_craft_instance(p.inst) || !game::near_fade_on() {
        return 1.0;
    }
    if !look.near_fade {
        // Stock's near fade: the bounding sphere over the distance.
        let inst = p.inst;
        let q = engine::field_f32(inst, 0x40) / engine::field_f32(inst, 0x8c).max(1e-3);
        let (minf, maxf, maxa) = unsafe { (engine::rd::<f32>(0x70444c), engine::rd::<f32>(0x704450), engine::rd::<f32>(0x704454)) };
        if !(q > minf) || !(maxf > minf) {
            return 1.0;
        }
        return 1.0 - (q.min(maxf) - minf) * maxa / (maxf - minf);
    }
    // The near plane from the projection (left-handed: z' = a z + b, near = -b / a).
    let a = v.proj.0[2][2];
    let b = v.proj.0[3][2];
    if a.abs() < 1e-9 {
        return 1.0;
    }
    let near = -b / a;
    // The hull's box: the model's sphere taken as a box about its centre, in view space.
    let c = v.view.transform_point(p.centre);
    let r = p.radius;
    let zmin = c.z - r;
    if zmin >= near {
        return 1.0;
    }
    let cut = ((near - zmin) / (2.0 * r).max(1e-3)) / look.near_fade_depth;
    let t = cut.clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - (1.0 - look.near_fade_min) * s
}


pub struct Drawer {
    vs: Shader,
    ps: Shader,
    ps_bump: Shader,
    ps_cloak: Shader,
    ps_ghost: Shader,
    ps_holo: Shader,
    decl: Decl,
    /// Pixel constants c80.. for `Mode::Ghost` and `Mode::Holo`, set by the caller.
    pub extra: [f32; 8 * 4],
    /// Normal maps made from height maps, by name.
    nmaps: BTreeMap<String, Option<Com>>,
    /// The copy of the frame a cloaked hull bends.
    copy: Option<(Com, Com, u32, u32, u32)>,
    said_bump: bool,
}

impl Drawer {
    pub fn new() -> Drawer {
        Drawer {
            vs: Shader::vertex(&VS_HULL),
            ps: Shader::pixel(&PS_HULL),
            ps_bump: Shader::pixel(&PS_BUMP),
            ps_cloak: Shader::pixel(&PS_CLOAK),
            ps_ghost: Shader::pixel(&PS_GHOST),
            ps_holo: Shader::pixel(&PS_HOLO),
            decl: Decl::new(&scene::VERTEX),
            extra: [0.0; 8 * 4],
            nmaps: BTreeMap::new(),
            copy: None,
            said_bump: false,
        }
    }

    pub fn reset(&mut self) {
        self.nmaps.clear();
        self.copy = None;
    }

    pub fn begin_view(&mut self, _v: &View, _look: &Look) {}

    /// The normal map of a height map (`ST3D_CreateNormalMap`): from each texel's
    /// neighbours across and down, wrapping, (-dh/du, -dh/dv, 1) made unit, height in
    /// 0..1 from the red channel.
    fn normal_map(&mut self, dev: Device, name: &str) -> *mut c_void {
        let key = crate::rt::lower(name);
        if let Some(t) = self.nmaps.get(&key) {
            return t.as_ref().map_or(core::ptr::null_mut(), |c| c.ptr());
        }
        let t = assets::image(&key).and_then(|img| {
            let (w, h) = (img.width as i32, img.height as i32);
            if w < 2 || h < 2 || !(w as u32).is_power_of_two() || !(h as u32).is_power_of_two() {
                return None;
            }
            let hgt = |x: i32, y: i32| -> f32 {
                let x = x.rem_euclid(w);
                let y = y.rem_euclid(h);
                img.pixels[((y * w + x) * 4 + 2) as usize] as f32 / 255.0
            };
            let mut levels: Vec<Vec<u8>> = Vec::new();
            let mut px = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let du = hgt(x + 1, y) - hgt(x - 1, y);
                    let dv = hgt(x, y + 1) - hgt(x, y - 1);
                    let l = libm::sqrtf(du * du + dv * dv + 1.0);
                    let n = [-du / l, -dv / l, 1.0 / l];
                    let b = |c: f32| ((c * 0.5 + 0.5) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
                    px.extend_from_slice(&[b(n[2]), b(n[1]), b(n[0]), 255]);
                }
            }
            levels.push(px);
            // The rest of the chain, box-filtered (and renormalised in the shader).
            let (mut lw, mut lh) = (w as usize, h as usize);
            while lw > 1 || lh > 1 {
                let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
                let src = levels.last().unwrap();
                let mut out = Vec::with_capacity(nw * nh * 4);
                for y in 0..nh {
                    for x in 0..nw {
                        for c in 0..4 {
                            let mut s = 0u32;
                            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                                let sx = (x * 2 + dx).min(lw - 1);
                                let sy = (y * 2 + dy).min(lh - 1);
                                s += src[(sy * lw + sx) * 4 + c] as u32;
                            }
                            out.push(((s + 2) / 4) as u8);
                        }
                    }
                }
                levels.push(out);
                lw = nw;
                lh = nh;
            }
            let refs: Vec<&[u8]> = levels.iter().map(|l| l.as_slice()).collect();
            upload_texture(dev, w as u32, h as u32, FMT_A8R8G8B8, &refs)
        });
        if t.is_none() {
            log!("hulls: no normal map from {}", key);
        }
        let p = t.as_ref().map_or(core::ptr::null_mut(), |c| c.ptr());
        self.nmaps.insert(key, t);
        p
    }

    /// The constants every hull of the view shares: lights, eye, shadow, planets (pixel
    /// c4..c72) for one profile.
    fn shared(&self, v: &View, look: &Look, prof: &Profile, centre: V3, radius: f32) -> [f32; 74 * 4] {
        let mut k = [0.0f32; 74 * 4];
        let set = |k: &mut [f32], reg: usize, a: [f32; 4]| k[reg * 4..reg * 4 + 4].copy_from_slice(&a);
        let ls = scene::lights();
        // Directional lights, the brightest (the Key) times the profile's sun.
        let key = ls.key();
        for (i, d) in ls.dirs.iter().take(4).enumerate() {
            let is_key = key.is_some_and(|k| (k.dir - d.dir).len() < 1e-5 && (k.colour - d.colour).len() < 1e-5);
            let c = if is_key { d.colour * prof.sun } else { d.colour };
            let to = -d.dir;
            set(&mut k, 8 + i, [to.x, to.y, to.z, 0.0]);
            set(&mut k, 12 + i, [c.x, c.y, c.z, if is_key { 1.0 } else { 0.0 }]);
        }
        let _ = (centre, radius);
        set(&mut k, 3, [prof.rim.x, prof.rim.y, prof.rim.z, look.rim_power]);
        set(&mut k, 4, [v.eye.x, v.eye.y, v.eye.z, 1.0]);
        if let Some(sm) = shadow::current() {
            set(&mut k, 5, [1.0, sm.texel, sm.bias, look.shadow_strength]);
        }
        let (planets, sun) = shadow::planets();
        for (i, pl) in planets.iter().take(8).enumerate() {
            set(&mut k, 64 + i, *pl);
        }
        set(&mut k, 72, [sun.x, sun.y, sun.z, if look.planet_shadows { look.shadow_strength } else { 0.0 }]);
        set(&mut k, 73, [0.0, look.bump, look.knee, 0.0]);
        k
    }

    /// Draws one hull in one mode.
    pub fn draw(&mut self, v: &View, look: &Look, p: &Posed, mode: Mode) {
        let dev = v.dev;
        let m = &p.model;
        // Which parts this mode draws.
        let want = |b: Blend| match mode {
            Mode::Additive(_) => b == Blend::Additive,
            _ => b != Blend::Additive,
        };
        if !m.parts.iter().enumerate().any(|(_, part)| want(part.blend) && p.pose.shown.get(part.node).copied().unwrap_or(false)) {
            return;
        }
        let _g = Guard::new(dev);
        let ps_ok = match mode {
            Mode::Cloak => self.ps_cloak.bind(dev),
            Mode::Ghost => self.ps_ghost.bind(dev),
            Mode::Holo => self.ps_holo.bind(dev),
            _ => self.ps.bind(dev),
        };
        let own_ps = matches!(mode, Mode::Cloak | Mode::Ghost | Mode::Holo);
        if matches!(mode, Mode::Ghost | Mode::Holo) {
            dev.ps_const(80, &self.extra);
        }
        if !(self.vs.bind(dev) && ps_ok && self.decl.bind(dev)) {
            return;
        }
        let cloak_copy = if mode == Mode::Cloak { self.copy_frame(v) } else { None };
        if mode == Mode::Cloak && cloak_copy.is_none() {
            // No copy: drawn faded instead, as stock fades it.
            drop(_g);
            let a = p.cloak.map_or(0.2, |c| 0.2 + 0.8 * c.1);
            self.draw(v, look, p, Mode::DepthOnly);
            self.draw(v, look, p, Mode::Faded(a));
            return;
        }
        // Vertex constants every part shares: VP (c4), the warp (c8, c9), the shadow
        // map's rows (c10..c13), the Key (c14), the wave light (c15, c16).
        dev.vs_const(4, &gfx::vp_constants(v));
        let mut vk = [0.0f32; 13 * 4];
        if let Some(w) = p.warp {
            vk[0..4].copy_from_slice(&[w.n.x, w.n.y, w.n.z, w.d]);
            vk[4..7].copy_from_slice(&[w.s1, w.o, 1.0]);
        }
        vk[7] = scene::NORMAL_SIGN;
        if let Some(sm) = shadow::current() {
            for r in 0..3 {
                vk[8 + r * 4..12 + r * 4].copy_from_slice(&sm.rows[r]);
            }
            vk[20] = sm.normal_offset;
            vk[24..28].copy_from_slice(&[sm.to_key.x, sm.to_key.y, sm.to_key.z, 0.0]);
        }
        if let Some(wv) = p.waves {
            vk[28..32].copy_from_slice(&[wv.dir.x, wv.dir.y, wv.dir.z, wv.k]);
            vk[32..36].copy_from_slice(&[wv.phase, 1.0, 0.0, 0.0]);
        }
        dev.vs_const(8, &vk[..36]);
        // States.
        for (s, val) in [
            (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
            (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_SRGBWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0),
        ] {
            dev.set_rs(s, val);
        }
        match mode {
            Mode::Opaque | Mode::Cloak => {
                dev.set_rs(RS_ALPHABLENDENABLE, 0);
                dev.set_rs(RS_ZWRITEENABLE, 1);
            }
            Mode::DepthOnly => {
                dev.set_rs(RS_ALPHABLENDENABLE, 0);
                dev.set_rs(RS_ZWRITEENABLE, 1);
                dev.set_rs(RS_COLORWRITEENABLE, 0);
            }
            Mode::Faded(_) => {
                dev.set_rs(RS_ALPHABLENDENABLE, 1);
                dev.set_rs(RS_SRCBLEND, BLEND_SRCALPHA);
                dev.set_rs(RS_DESTBLEND, BLEND_INVSRCALPHA);
                dev.set_rs(RS_ZWRITEENABLE, 0);
                dev.set_rs(RS_ZFUNC, CMP_LESSEQUAL);
            }
            Mode::Additive(_) => {
                dev.set_rs(RS_ALPHABLENDENABLE, 1);
                dev.set_rs(RS_SRCBLEND, BLEND_ONE);
                dev.set_rs(RS_DESTBLEND, BLEND_ONE);
                dev.set_rs(RS_ZWRITEENABLE, 0);
            }
            Mode::Ghost | Mode::Holo => {
                dev.set_rs(RS_ALPHABLENDENABLE, 1);
                dev.set_rs(RS_SRCBLEND, BLEND_ONE);
                dev.set_rs(RS_DESTBLEND, BLEND_INVSRCALPHA);
                dev.set_rs(RS_ZWRITEENABLE, 0);
            }
        }
        for s in [0u32, 1] {
            dev.set_ss(s, SAMP_ADDRESSU, TADDRESS_WRAP);
            dev.set_ss(s, SAMP_ADDRESSV, TADDRESS_WRAP);
            dev.set_ss(s, SAMP_MAGFILTER, TEXF_LINEAR);
            dev.set_ss(s, SAMP_MINFILTER, TEXF_LINEAR);
            dev.set_ss(s, SAMP_MIPFILTER, TEXF_LINEAR);
        }
        dev.set_stream_freq(0, 1);
        if let Some(sm) = shadow::current() {
            dev.set_texture(3, sm.tex);
            dev.set_ss(3, SAMP_ADDRESSU, TADDRESS_CLAMP);
            dev.set_ss(3, SAMP_ADDRESSV, TADDRESS_CLAMP);
            dev.set_ss(3, SAMP_MAGFILTER, TEXF_POINT);
            dev.set_ss(3, SAMP_MINFILTER, TEXF_POINT);
            dev.set_ss(3, SAMP_MIPFILTER, TEXF_NONE);
        }
        if let Some((tex, ox, oy, cw, ch)) = cloak_copy {
            dev.set_texture(6, tex);
            dev.set_ss(6, SAMP_ADDRESSU, TADDRESS_CLAMP);
            dev.set_ss(6, SAMP_ADDRESSV, TADDRESS_CLAMP);
            dev.set_ss(6, SAMP_MAGFILTER, TEXF_LINEAR);
            dev.set_ss(6, SAMP_MINFILTER, TEXF_LINEAR);
            dev.set_ss(6, SAMP_MIPFILTER, TEXF_NONE);
            let (st, vis, _) = p.cloak.unwrap_or((2, 0.0, 0.0));
            let _ = st;
            let vw = &v.view.0;
            let mut ck = [0.0f32; 11 * 4];
            for r in 0..3 {
                ck[r * 4..r * 4 + 3].copy_from_slice(&[vw[0][r], vw[1][r], vw[2][r]]);
            }
            ck[12..16].copy_from_slice(&[v.proj.0[0][0] * 0.5 * v.rect.w as f32 / cw as f32, v.proj.0[1][1] * 0.5 * v.rect.h as f32 / ch as f32, 1.0 / cw as f32, 1.0 / ch as f32]);
            // The instance's sphere, as Lighting.asi took it: the radius `+0x40` about
            // the placement's origin (`+0x44`'s translation). Its patches are laid
            // out over it.
            let ir = engine::field_f32(p.inst, 0x40);
            let (cc, cr) = if ir > 0.0 && ir < 1e5 { (engine::field_v3(p.inst, 0x68), ir) } else { (p.centre, p.radius) };
            ck[16..20].copy_from_slice(&[cc.x, cc.y, cc.z, cr]);
            ck[20..24].copy_from_slice(&[vis, v.time * look.cloak_speed, look.cloak_bend, look.cloak_split]);
            ck[24..28].copy_from_slice(&[look.cloak_ripple, look.cloak_rim, look.cloak_glint, look.cloak_hull]);
            let t = look.cloak_tint;
            ck[28..32].copy_from_slice(&[t.x, t.y, t.z, look.cloak_front]);
            let r = look.cloak_rim_colour;
            ck[32..36].copy_from_slice(&[r.x, r.y, r.z, 0.0]);
            let e = look.cloak_front_colour;
            ck[36..40].copy_from_slice(&[e.x, e.y, e.z, 0.0]);
            ck[40..44].copy_from_slice(&[ox as f32, oy as f32, 0.0, 0.0]);
            dev.ps_const(80, &ck);
        }
        let mut shared: Option<(bool, [f32; 74 * 4])> = None;
        // The faces the instance draws (`sm_faces_remaining`, `+0x10`: construction).
        // (None for a model that is not an instance's: inst 0.)
        let limit = if p.inst == 0 { -1 } else { engine::field(p.inst, 0x10) as i32 };
        let mut faces_left: i64 = if limit >= 0 { limit as i64 } else { i64::MAX };
        // With a face limit the faces are counted in the engine's walk of the node tree
        // (children in its order, the reverse of the file's: `ST3D_Node::AddChild` puts
        // each new child first), which is also the order `BuildingEffect` adds its
        // welding edges in; without one, the order does not matter.
        let order: Vec<usize> = if limit >= 0 { engine_order(m) } else { (0..m.parts.len()).collect() };
        for part in order.iter().map(|&i| &m.parts[i]) {
            if !want(part.blend) || !p.pose.shown.get(part.node).copied().unwrap_or(false) {
                continue;
            }
            if faces_left <= 0 {
                break;
            }
            let bump = part.bump.is_some();
            let prof = if bump { &look.borg } else { &look.hull };
            if shared.is_none_or(|s| s.0 != bump) {
                shared = Some((bump, self.shared(v, look, prof, p.centre, p.radius)));
                dev.ps_const(3, &shared.as_ref().unwrap().1[3 * 4..]);
            }
            // Bump-mapped meshes go through ps_bump with their normal map.
            let nmap = if bump && mode != Mode::DepthOnly && !own_ps {
                let n = self.normal_map(dev, part.bump.as_deref().unwrap_or(""));
                if !self.said_bump {
                    self.said_bump = true;
                    log!("hulls: bump-mapped meshes (the Borg profile), normal map from {}", part.bump.as_deref().unwrap_or(""));
                }
                n
            } else {
                core::ptr::null_mut()
            };
            if !nmap.is_null() {
                self.ps_bump.bind(dev);
                dev.set_texture(1, nmap);
            } else if !own_ps {
                self.ps.bind(dev);
            }
            dev.ps_const(73, &[if nmap.is_null() { 0.0 } else { 1.0 }, look.bump, look.knee, 0.0]);
            let world = p.pose.world[part.node];
            dev.vs_const(0, world.to_m4().transpose().as_floats());
            if mode != Mode::DepthOnly {
                dev.ps_const(16, &pick_points(world.p, look.point_lights));
            }
            let mirrored = world.det3() < 0.0;
            // A face-limited instance (a station being built) shows its inside through
            // the faces not built yet: drawn two-sided, as stock shows it.
            dev.set_rs(RS_CULLMODE, if part.two_sided || limit >= 0 { CULL_NONE } else if mirrored { CULL_CCW } else { CULL_CW });
            let tex_name = if p.assimilated { second_texture(m, part.node).or(part.texture.as_deref()) } else { part.texture.as_deref() };
            let tex = tex_name.and_then(|t| assets::texture(dev, t)).unwrap_or(core::ptr::null_mut());
            dev.set_texture(0, tex);
            let uv = flipbook(m, part.node, p);
            let cut = part.blend == Blend::Cutout;
            // A `default` mesh's texture alpha is a cut-out where its lighting material
            // is not self-illuminating (seen on the research station's girders, which
            // stock and Lighting.asi draw open); see render3d/README.md, "Hulls".
            let default_mat = m.sod.nodes[part.node].mesh().is_some_and(|ms| ms.texture_material.eq_ignore_ascii_case("default"));
            let wave = if p.waves.is_some() { 1.0 } else { 0.0 };
            dev.ps_const(6, &[uv[0], uv[1], wave, if cut { 1.0 } else { 0.0 }]);
            if let Some(wv) = p.waves {
                dev.ps_const(7, &[wv.colour.x, wv.colour.y, wv.colour.z, 0.0]);
            }
            let (fade, tex_alpha) = match mode {
                Mode::Faded(f) => (f, 0.0),
                _ => (1.0, 0.0),
            };
            dev.ps_const(2, &[fade, prof.specular, prof.specular_power, tex_alpha]);
            dev.set_stream(0, part.vb.ptr(), 0, 32);
            dev.set_indices(part.ib.ptr());
            for g in &part.groups {
                if faces_left <= 0 {
                    break;
                }
                let tris = (g.tris as i64).min(faces_left) as u32;
                faces_left -= tris as i64;
                let team = if g.material.starts_with("team") { p.team } else { None };
                let t = team.unwrap_or(v3(1.0, 1.0, 1.0));
                let unlit = matches!(mode, Mode::Additive(_));
                let tint = match mode {
                    Mode::Additive(f) => t * (look.additive * f),
                    _ => t,
                };
                let selfi = if g.self_illum { prof.self_illum } else { 0.0 };
                if default_mat && !cut {
                    dev.ps_const(6, &[uv[0], uv[1], wave, if g.self_illum { 0.0 } else { 1.0 }]);
                }
                dev.ps_const(0, &[prof.ambient.x, prof.ambient.y, prof.ambient.z, selfi]);
                dev.ps_const(1, &[tint.x, tint.y, tint.z, if unlit { 1.0 } else { 0.0 }]);
                dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, tris);
            }
        }
        let _ = field;
    }

    /// A copy of the view's part of the frame, for a cloaked hull to bend (the farther
    /// cloaked hulls already in it): (texture, its top left in the target, its size).
    fn copy_frame(&mut self, v: &View) -> Option<(*mut c_void, u32, u32, u32, u32)> {
        let dev = v.dev;
        let rt = dev.get_render_target(0)?;
        let d = rt.surf_desc();
        let (w, h) = (v.rect.w.max(1), v.rect.h.max(1));
        if self.copy.as_ref().is_none_or(|c| c.2 != w || c.3 != h || c.4 != d.format) {
            self.copy = None;
            let tex = dev.create_texture(w, h, 1, USAGE_RENDERTARGET, d.format, POOL_DEFAULT)?;
            let surf = tex.tex_surface(0)?;
            self.copy = Some((tex, surf, w, h, d.format));
        }
        let (tex, surf, ..) = self.copy.as_ref()?;
        let src = v.rect.d3d();
        let dst = Rect { left: 0, top: 0, right: w as i32, bottom: h as i32 };
        if dev.stretch_rect(&rt, Some(&src), surf, Some(&dst), TEXF_LINEAR) < 0 {
            return None;
        }
        Some((tex.ptr(), v.rect.x, v.rect.y, w, h))
    }
}

/// The model's parts in the engine's walk of its node tree: depth first from each
/// root, children in reverse file order.
fn engine_order(m: &Model) -> Vec<usize> {
    let nodes = &m.sod.nodes;
    let mut rank = alloc::vec![usize::MAX; nodes.len()];
    let mut next = 0;
    let mut stack: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].parent.is_none()).rev().collect();
    let mut guard = 0;
    while let Some(i) = stack.pop() {
        guard += 1;
        if guard > nodes.len() * 2 || rank[i] != usize::MAX {
            continue;
        }
        rank[i] = next;
        next += 1;
        // Pushed in file order, so the last child (the engine's first) is popped first.
        stack.extend(nodes[i].children.iter().copied());
    }
    let mut order: Vec<usize> = (0..m.parts.len()).collect();
    order.sort_by_key(|&pi| rank.get(m.parts[pi].node).copied().unwrap_or(usize::MAX));
    order
}

/// A mesh's second texture (the Borg's assimilated look, `CraftClass::
/// SetBorgMeshTextures`).
fn second_texture(m: &Model, node: usize) -> Option<&str> {
    let mesh = m.sod.nodes.get(node)?.mesh()?;
    if !mesh.texture2.is_empty() {
        return Some(mesh.texture2.as_str());
    }
    mesh.textures.iter().filter(|t| t.flags & a2fmt::sod::MeshTexture::HEIGHT_MAP == 0).nth(1).map(|t| t.name.as_str())
}

/// A mesh's texture flipbook (SOD animation reference): its UV offset now, on the
/// instance's clock plus the reference's offset.
fn flipbook(m: &Model, node: usize, p: &Posed) -> [f32; 2] {
    let name = &m.sod.nodes[node].name;
    let Some(r) = m.sod.anim_refs.iter().find(|r| r.node.eq_ignore_ascii_case(name)) else { return [0.0, 0.0] };
    let table = assets::sprites();
    let Some(a) = table.animation(&r.animation) else { return [0.0, 0.0] };
    anim::offset_of(anim::value(a, p.clock.elapsed + r.offset, p.clock.flags))
}
