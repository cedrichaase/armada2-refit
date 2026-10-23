//! Drawing a posed hull in the hull shaders (`shaders/hulls.hlsl`).
use super::look::{Look, Profile};
use super::{anim, game, shadow, Posed};
use crate::engine;
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{v3, M34, V3};
use crate::scene::{self, Blend, Model};
use crate::shaders::hulls::*;
use crate::shaders::hulls_spv::*;
use crate::{assets, log};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use gpu::{
    Address, Blend as GBlend, BlendEq, BlendOp, Block, Compare, Contents, Cull, Depth, Factor, Filter, Format,
    MipFilter, Pipeline, PipelineDesc, Rect, Sampler, Texture, TextureDesc, Topology, Usage,
};

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

    /// A menu ship's departure (`crate::menu`), which has no instance: from a sphere
    /// and the way already in world space, and stretched forward only. The engine's
    /// shift back by o, seen from the camera behind the ships, threw the stretch across
    /// the screen towards it.
    pub fn world(b: f32, s: f32, n: V3, centre: V3, r: f32) -> Warp {
        let d = n.dot(centre) + r - 2.0 * r * b;
        Warp { n, d, s1: s - 1.0, o: 0.0 }
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
/// its shaders, at the mesh's
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
    match model_box(m) {
        Some((lo, hi)) => ((lo + hi) * 0.5, (hi - lo).len() * 0.5),
        None => (V3::ZERO, 1.0),
    }
}

/// The model's box, model space: the parts' spheres, placed by their nodes.
pub fn model_box(m: &Model) -> Option<(V3, V3)> {
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
    any.then_some((lo, hi))
}

/// The cloak's waves for this draw, as `vs_hull`'s c17..c19 (`ART-DIRECTION.md`,
/// "Cloaking"): bands along the hull's longest axis, `bands` of them over its length,
/// travelling at `speed` bands a second; they sway it by `sway` world units across the
/// screen, at right angles to the bands and never towards the camera, so the ship
/// ripples like an image rather than bending in depth (the user found a sway through
/// the hull's own axes too three-dimensional from the RTS camera, 2026-10-16).
fn cloak_waves(p: &Posed, look: &Look, time: f32, eye: V3, sway: f32) -> [f32; 12] {
    let Some((lo, hi)) = model_box(&p.model) else { return [0.0; 12] };
    let d = hi - lo;
    let ext = [d.x * p.world.r[0].len(), d.y * p.world.r[1].len(), d.z * p.world.r[2].len()];
    let mut ix = [0usize, 1, 2];
    ix.sort_by(|a, b| ext[*b].partial_cmp(&ext[*a]).unwrap_or(core::cmp::Ordering::Equal));
    let along = p.world.r[ix[0]].norm();
    let k = 2.0 * core::f32::consts::PI * look.cloak_wave_bands / ext[ix[0]].max(1e-3);
    let ph = along.dot(p.centre) * k + time * look.cloak_wave_speed * 2.0 * core::f32::consts::PI;
    let across = along.cross(p.centre - eye);
    let side = if across.len() < 1e-6 { p.world.r[ix[1]].norm() } else { across.norm() } * sway;
    [along.x * k, along.y * k, along.z * k, ph, side.x, side.y, side.z, 0.0, 0.0, 0.0, 0.0, 0.0]
}

/// The fade a hull is drawn at: the engine's far fade, or (crafts) Lighting.asi's near
/// fade, which waits for the near plane to cut into the hull's box.
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
    pipes: Pipes,
    /// Pixel constants c80.. for `Mode::Ghost` and `Mode::Holo`, set by the caller.
    pub extra: [f32; 8 * 4],
    /// Normal maps made from height maps, by name.
    nmaps: BTreeMap<String, Option<Texture>>,
    /// The copy of the frame a cloaked hull bends.
    copy: Option<Texture>,
    said_bump: bool,
}

/// The hull's pixel shaders (`Drawer::pipe`).
const PS_LIST: [gpu::Code; 5] = [gpu::code!(PS_HULL, PS_HULL_SPV), gpu::code!(PS_BUMP, PS_BUMP_SPV), gpu::code!(PS_CLOAK, PS_CLOAK_SPV), gpu::code!(PS_GHOST, PS_GHOST_SPV), gpu::code!(PS_HOLO, PS_HOLO_SPV)];
const S_HULL: usize = 0;
const S_BUMP: usize = 1;
const S_CLOAK: usize = 2;
const S_GHOST: usize = 3;
const S_HOLO: usize = 4;

/// The hull's textures and normal maps: wrapped, filtered through their levels.
const WRAP: Sampler = gfx::sampler(Filter::Linear, MipFilter::Linear, Address::Wrap, Address::Wrap);
/// The shadow map: clamped, a texel each.
const SHADOW: Sampler = gfx::sampler(Filter::Point, MipFilter::None, Address::Clamp, Address::Clamp);
/// The frame's copy under a cloaked hull: clamped, filtered.
const COPY: Sampler = gfx::sampler(Filter::Linear, MipFilter::None, Address::Clamp, Address::Clamp);

impl Drawer {
    pub fn new() -> Drawer {
        Drawer { pipes: Pipes::new(), extra: [0.0; 8 * 4], nmaps: BTreeMap::new(), copy: None, said_bump: false }
    }

    /// Before a `Reset`: the normal maps keep no copy of their texels, and the frame's
    /// copy is a target.
    pub fn reset(&mut self) {
        self.nmaps.clear();
        self.copy = None;
    }

    pub fn begin_view(&mut self, _v: &View, _look: &Look) {}

    /// The pipeline of pixel shader `ps` (`S_*`) for a mode and a cull: depth-tested
    /// against the scene (`LESSEQUAL`); opaque and cloaked hulls write depth, a depth
    /// pass writes depth alone, a faded hull blends by alpha, additive meshes add, a
    /// ghost or hologram goes over what is there premultiplied.
    fn pipe(&mut self, ps: usize, mode: Mode, cull: Cull) -> Option<Pipeline> {
        let m = match mode {
            Mode::Opaque | Mode::Cloak => 0u64,
            Mode::DepthOnly => 1,
            Mode::Faded(_) => 2,
            Mode::Additive(_) => 3,
            Mode::Ghost | Mode::Holo => 4,
        };
        self.pipes.view((ps as u64 * 8 + m) * 4 + cull as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_HULL, VS_HULL_SPV), PS_LIST[ps], &scene::VERTEX, 32, Topology::TriangleList, Format::Unknown);
            d.cull = cull;
            let (blend, write, mask) = match m {
                0 => (None, true, 0xf),
                1 => (None, true, 0),
                2 => (Some(GBlend::ALPHA), false, 0xf),
                3 => (Some(GBlend::ADD), false, 0xf),
                _ => (Some(GBlend { color: BlendEq { src: Factor::One, dst: Factor::InvSrcAlpha, op: BlendOp::Add }, alpha: None }), false, 0xf),
            };
            d.blend = blend;
            d.write_mask = mask;
            d.depth = Depth { test: true, write, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d
        })
    }

    /// The normal map of a height map (`ST3D_CreateNormalMap`): from each texel's
    /// neighbours across and down, wrapping, (-dh/du, -dh/dv, 1) made unit, height in
    /// 0..1 from the red channel.
    fn normal_map(&mut self, name: &str) -> Option<Texture> {
        let key = crate::rt::lower(name);
        if let Some(t) = self.nmaps.get(&key) {
            return t.clone();
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
            let n = levels.len() as u32;
            let px: Vec<u8> = levels.concat();
            let g = gfx::gpu();
            let d = TextureDesc::d2(w as u32, h as u32, Format::Bgra8, Usage::SAMPLED).with_levels(n);
            let t = g.texture(d, Contents::Transient(px));
            t.ready(g).then_some(t)
        });
        if t.is_none() {
            log!("hulls: no normal map from {}", key);
        }
        self.nmaps.insert(key, t.clone());
        t
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
        let _z = crate::timing::zone("hulls: draw");
        let m = &p.model;
        // Which parts this mode draws.
        let want = |b: Blend| match mode {
            Mode::Additive(_) => b == Blend::Additive,
            _ => b != Blend::Additive,
        };
        if !m.parts.iter().enumerate().any(|(_, part)| want(part.blend) && p.pose.shown.get(part.node).copied().unwrap_or(false)) {
            return;
        }
        let own = match mode {
            Mode::Cloak => Some(S_CLOAK),
            Mode::Ghost => Some(S_GHOST),
            Mode::Holo => Some(S_HOLO),
            _ => None,
        };
        let own_ps = own.is_some();
        if !self.pipe(own.unwrap_or(S_HULL), mode, Cull::None).is_some_and(|pl| pl.ready(gfx::gpu())) {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        if matches!(mode, Mode::Ghost | Mode::Holo) {
            cmd.ps_block(Block::new(80, &self.extra));
        }
        let cloak_copy = if mode == Mode::Cloak { self.copy_frame(&mut cmd, v) } else { None };
        if mode == Mode::Cloak && cloak_copy.is_none() {
            // No copy: drawn faded instead, as stock fades it.
            drop(cmd);
            let a = p.cloak.map_or(0.2, |c| 0.2 + 0.8 * c.1);
            self.draw(v, look, p, Mode::DepthOnly);
            self.draw(v, look, p, Mode::Faded(a));
            return;
        }
        let cmd = &mut cmd;
        // Vertex constants every part shares: VP (c4), the warp (c8, c9), the shadow
        // map's rows (c10..c13), the Key (c14), the wave light (c15, c16).
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
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
        // The cloak's waves sway the geometry while it cloaks or decloaks (c17..c19).
        let mut ck_wave = [0.0f32; 4];
        if let (Mode::Cloak, Some((st, vis, _))) = (mode, p.cloak) {
            let env = if st == 1 || st == 3 { 4.0 * vis * (1.0 - vis) } else { 0.0 };
            let w = cloak_waves(p, look, v.time, v.eye, look.cloak_wave * p.radius * env);
            vk[36..48].copy_from_slice(&w);
            ck_wave.copy_from_slice(&w[0..4]);
        }
        cmd.vs_block(Block::new(8, &vk[..48]));
        cmd.set_sampler(0, &WRAP);
        cmd.set_sampler(1, &WRAP);
        if let Some(sm) = shadow::current() {
            cmd.bind_texture(3, Some(&sm.tex), &SHADOW);
        }
        if let Some((tex, ox, oy, cw, ch)) = cloak_copy {
            cmd.bind_texture(6, Some(&tex), &COPY);
            let (st, vis, _) = p.cloak.unwrap_or((2, 0.0, 0.0));
            let _ = st;
            let vw = &v.view.0;
            let mut ck = [0.0f32; 13 * 4];
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
            // The player's own ships keep a trace of their hull; no one else's does.
            let floor = if game::is_users(p.obj) { look.cloak_hull } else { look.cloak_hull_others };
            ck[24..28].copy_from_slice(&[look.cloak_ripple, look.cloak_rim, look.cloak_glint, floor]);
            let t = look.cloak_tint;
            ck[28..32].copy_from_slice(&[t.x, t.y, t.z, look.cloak_front]);
            let r = look.cloak_rim_colour;
            ck[32..36].copy_from_slice(&[r.x, r.y, r.z, 0.0]);
            let e = look.cloak_front_colour;
            ck[36..40].copy_from_slice(&[e.x, e.y, e.z, 0.0]);
            ck[40..44].copy_from_slice(&[ox as f32, oy as f32, 0.0, 0.0]);
            ck[44..48].copy_from_slice(&ck_wave);
            ck[48..52].copy_from_slice(&[look.cloak_shimmer, look.cloak_wave_tilt, v.time * look.cloak_shimmer_speed, 0.0]);
            cmd.ps_block(Block::new(80, &ck));
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
                cmd.ps_block(Block::new(3, &shared.as_ref().unwrap().1[3 * 4..]));
            }
            // Bump-mapped meshes go through ps_bump with their normal map.
            let nmap = if bump && mode != Mode::DepthOnly && !own_ps {
                let n = self.normal_map(part.bump.as_deref().unwrap_or(""));
                if !self.said_bump {
                    self.said_bump = true;
                    log!("hulls: bump-mapped meshes (the Borg profile), normal map from {}", part.bump.as_deref().unwrap_or(""));
                }
                n
            } else {
                None
            };
            let world = p.pose.world[part.node];
            let mirrored = world.det3() < 0.0;
            // A face-limited instance (a station being built) shows its inside through
            // the faces not built yet: drawn two-sided, as stock shows it.
            let cull = if part.two_sided || limit >= 0 { Cull::None } else if mirrored { Cull::CounterClockwise } else { Cull::Clockwise };
            let ps = if nmap.is_some() { S_BUMP } else { own.unwrap_or(S_HULL) };
            let Some(pipe) = self.pipe(ps, mode, cull) else { continue };
            if !cmd.set_pipeline(&pipe) {
                continue;
            }
            if let Some(n) = nmap.as_ref() {
                cmd.bind_texture(1, Some(n), &WRAP);
            }
            cmd.ps_block(Block::new(73, &[if nmap.is_none() { 0.0 } else { 1.0 }, look.bump, look.knee, 0.0]));
            cmd.vs_block(Block::new(0, world.to_m4().transpose().as_floats()));
            if mode != Mode::DepthOnly {
                cmd.ps_block(Block::new(16, &pick_points(world.p, look.point_lights)));
            }
            let tex_name = if p.assimilated { second_texture(m, part.node).or(part.texture.as_deref()) } else { part.texture.as_deref() };
            let tex = tex_name.and_then(assets::texture);
            cmd.bind_texture(0, tex.as_ref(), &WRAP);
            let uv = flipbook(m, part.node, p);
            let cut = part.blend == Blend::Cutout;
            // A `default` mesh's texture alpha is a cut-out where its lighting material
            // is not self-illuminating (seen on the research station's girders, which
            // stock and Lighting.asi draw open); see render3d/README.md, "Hulls".
            let default_mat = m.sod.nodes[part.node].mesh().is_some_and(|ms| ms.texture_material.eq_ignore_ascii_case("default"));
            let wave = if p.waves.is_some() { 1.0 } else { 0.0 };
            cmd.ps_block(Block::new(6, &[uv[0], uv[1], wave, if cut { 1.0 } else { 0.0 }]));
            if let Some(wv) = p.waves {
                cmd.ps_block(Block::new(7, &[wv.colour.x, wv.colour.y, wv.colour.z, 0.0]));
            }
            let (fade, tex_alpha) = match mode {
                Mode::Faded(f) => (f, 0.0),
                _ => (1.0, 0.0),
            };
            cmd.ps_block(Block::new(2, &[fade, prof.specular, prof.specular_power, tex_alpha]));
            for g in &part.groups {
                if faces_left <= 0 {
                    break;
                }
                let tris = (g.tris as i64).min(faces_left) as u32;
                faces_left -= tris as i64;
                let team = if g.material.starts_with("team") { p.team } else { None };
                // A freighter's cargo markings: the resource's colour or black, unlit (the
                // engine's material colour; `game::cargo_markings`).
                let cargo = p.cargo.iter().find(|(k, _)| *k == g.material).map(|(_, c)| *c);
                let t = cargo.or(team).unwrap_or(v3(1.0, 1.0, 1.0));
                let unlit = matches!(mode, Mode::Additive(_)) || cargo.is_some();
                let tint = match mode {
                    Mode::Additive(f) => t * (look.additive * f),
                    _ => t,
                };
                let selfi = if g.self_illum { prof.self_illum } else { 0.0 };
                if default_mat && !cut {
                    cmd.ps_block(Block::new(6, &[uv[0], uv[1], wave, if g.self_illum { 0.0 } else { 1.0 }]));
                }
                cmd.ps_block(Block::new(0, &[prof.ambient.x, prof.ambient.y, prof.ambient.z, selfi]));
                cmd.ps_block(Block::new(1, &[tint.x, tint.y, tint.z, if unlit { 1.0 } else { 0.0 }]));
                part.draw(cmd, g.start, tris);
            }
        }
    }

    /// A copy of the view's part of the frame, for a cloaked hull to bend (the farther
    /// cloaked hulls already in it): (texture, its top left in the target, its size).
    fn copy_frame(&mut self, cmd: &mut gpu::Cmd, v: &View) -> Option<(Texture, u32, u32, u32, u32)> {
        let (scene, _) = crate::frame::scene_targets()?;
        let fmt = scene.desc().format;
        let (w, h) = (v.rect.w.max(1), v.rect.h.max(1));
        if self.copy.as_ref().is_none_or(|c| c.size() != (w, h) || c.desc().format != fmt) {
            self.copy = None;
            let g = gfx::gpu();
            let t = g.texture(TextureDesc::d2(w, h, fmt, Usage::RENDER_TARGET | Usage::SAMPLED | Usage::COPY_DST), Contents::None);
            if !t.ready(g) {
                return None;
            }
            self.copy = Some(t);
        }
        let tex = self.copy.clone()?;
        if !cmd.copy(&scene, Some(v.rect.rect()), &tex, Some(Rect::new(0, 0, w, h)), Filter::Linear) {
            return None;
        }
        Some((tex, v.rect.x, v.rect.y, w, h))
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
