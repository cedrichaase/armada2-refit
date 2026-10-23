//! Explosions, ported from `Explosions.asi`: each
//! `FireballExplosion` drawn as a volume of gas, ray-marched per pixel through a
//! sphere, and each `ShockwaveExplosion` as a band of fire matched to stock's disc. The
//! look and its tuning are the plugin's, set with the user; what is new is HDR: the
//! white-hot core and the flash go past white (`Hdr=`) and bloom by their energy.
//!
//! `ExplosionFeature::Render` (0x464ba0) walks `Explosion::explosionList` (a
//! `std::list<Explosion *>` at 0x7367c0, its head node at 0x7367c4) and, for each one
//! whose slot 2 says it is visible, calls its `Submit(cam)` (slot 15). We do the same,
//! noting fireballs (vtable 0x6afbd4) and shockwaves (0x6afdac) instead of submitting
//! them; any other class (`OmegaExplosion`) is submitted to the engine. The noted ones
//! are drawn at `Step::Chunks` (mod.rs).
//!
//! Read off an explosion: its class (+0x30) and the radius it was built with (+0x34,
//! the dead craft's bounding radius); its transform through `Entity::GetTransform`
//! (0x4cfd50). A fireball's time left is at +0xa8, its class's `length` at +0x4c. A
//! shockwave's age is at +0x4c, the radius it stops at at +0x50,
//! `ShockwaveExplosion::GetCurrentRadius` (0x467730) its radius now; its class holds
//! `color` at +0xd0 and `fadeFactor` at +0xe0.
//!
//! Each fireball is also a light (as `Lighting.asi` gave it one; render3d/README.md,
//! "effects"): an ordinary point light registered at the view's start, up to full in
//! its first 0.15 s, then down with the square of the time left; full out to its
//! entity's bounding radius (the explosion's entity `+4`, its bounding sphere at
//! `+0x34`, radius `+0x40`; at least 40 units), gone at `LightRange` radii.
use super::{ff, hash32, Cam, LightPool, Sec};
use crate::engine::{self, field, obj};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::log;
use crate::math::{sqrtf, V3};
use crate::shaders::effects_explosions::*;
use crate::shaders::effects_explosions_spv::*;
use crate::shaders::effects_explosions_dxil::*;
use alloc::vec::Vec;
use gpu::{Address, Block, Contents, Filter, Format, MipFilter, Sampler, Texture, TextureDesc, Topology, Usage};

const EXPLOSION_HEAD: usize = 0x7367c4;
const VT_FIREBALL: usize = 0x6afbd4;
const VT_SHOCKWAVE: usize = 0x6afdac;
const FN_SHOCK_RADIUS: usize = 0x467730;
const SLOT_VISIBLE: usize = 2;
const SLOT_SUBMIT: usize = 15;

struct Set {
    rings: bool,
    bright: f32,
    size: f32,
    minr: f32,
    grow: f32,
    smoke: f32,
    turb: f32,
    cool: f32,
    opacity: f32,
    flash: f32,
    glow: f32,
    detail: f32,
    ring_bright: f32,
    ring_width: f32,
    ring_break: f32,
    noise: u32,
    age: f32,
    hdr: f32,
    hdr_from: f32,
    light_col: V3,
    light_bright: f32,
    light_range: f32,
}

pub struct Explosions {
    pub on: bool,
    pub light_on: bool,
    s: Set,
    /// What `ExplosionFeature` handed us in this view: (explosion, is a ring).
    pub notes: Vec<(usize, bool)>,
    vol: Option<Texture>,
    vol_failed: bool,
    /// The fireball's and the shock ring's pipelines.
    pipes: Pipes,
    lights: LightPool,
    logged: u32,
}

impl Explosions {
    pub fn new() -> Explosions {
        let c = Sec::new("Explosions");
        let s = Set {
            rings: c.i("Rings", 1) != 0,
            bright: c.f("Brightness", 1.0),
            size: c.f("Size", 1.0).max(0.1),
            minr: c.f("MinRadius", 25.0),
            grow: c.f("Growth", 0.4).max(0.02),
            smoke: c.f("Smoke", 1.0),
            turb: c.f("Turbulence", 1.0),
            cool: c.f("Cooling", 1.0),
            opacity: c.f("Opacity", 0.3),
            flash: c.f("Flash", 1.0),
            glow: c.f("Glow", 0.25),
            detail: c.f("Detail", 1.6),
            ring_bright: c.f("RingBrightness", 1.0),
            ring_width: c.f("RingWidth", 0.52).clamp(0.02, 0.95),
            ring_break: c.f("RingBreakup", 1.0),
            noise: match c.i("NoiseSize", 64) {
                32 => 32,
                128 => 128,
                _ => 64,
            },
            age: c.f("Age", -1.0),
            hdr: c.f("Hdr", 0.5).max(0.0),
            hdr_from: c.f("HdrFrom", 1.5),
            light_col: c.v3("LightColour", [1.0, 0.62, 0.28]),
            light_bright: c.f("LightBrightness", 4.0),
            light_range: c.f("LightRange", 10.0),
        };
        Explosions {
            on: c.i("Enable", 1) != 0,
            light_on: c.i("Light", 1) != 0,
            s,
            notes: Vec::new(),
            vol: None,
            vol_failed: false,
            pipes: Pipes::new(),
            lights: LightPool::new(b"explosion\0"),
            logged: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vol = None;
    }

    /// `ExplosionFeature::Render` in the engine's place.
    pub fn feature(&mut self, v: &View) -> bool {
        let rings = self.s.rings;
        let mut notes = core::mem::take(&mut self.notes);
        for_each_explosion(|e| unsafe {
            let vt = engine::vtable(e);
            let vis: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vt, SLOT_VISIBLE * 4));
            if vis(e) == 0 {
                return;
            }
            // A special weapon's wave with a recipe is the ordnance module's
            // (`[Wave.<odf>]`); every other shockwave gets the ring.
            if vt == VT_SHOCKWAVE && crate::mods::ordnance::draw_wave(e, v) {
                return;
            }
            if vt == VT_FIREBALL || (vt == VT_SHOCKWAVE && rings) {
                if !notes.iter().any(|n| n.0 == e) {
                    notes.push((e, vt == VT_SHOCKWAVE));
                }
            } else {
                let submit: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(vt, SLOT_SUBMIT * 4));
                submit(e, v.cam);
            }
        });
        self.notes = notes;
        true
    }

    /// One point light per live fireball.
    pub fn lights(&mut self, _v: &View) {
        if !self.on && !self.light_on {
            return;
        }
        if !self.light_on {
            return;
        }
        let mut n = 0;
        let mut list = Vec::new();
        for_each_explosion(|e| {
            if engine::vtable(e) == VT_FIREBALL {
                list.push(e);
            }
        });
        for e in list {
            let cls = field(e, 0x30);
            let len = ff(cls, 0x4c);
            let left = ff(e, 0xa8);
            if cls == 0 || !(len > 0.0) || !(left > 0.0) {
                continue;
            }
            let age = len - left;
            let k = if age < 0.15 {
                age / 0.15
            } else {
                let k = left / (len - 0.15).max(0.01);
                if k > 1.0 { 1.0 } else { k * k }
            };
            let ent = field(e, 4);
            let br = if ent != 0 { ff(ent, 0x40) } else { 0.0 };
            let r = if br > 40.0 { br } else { 40.0 };
            let pos = obj::transform(e).p;
            let col = self.s.light_col * (self.s.light_bright * k);
            if self.lights.register(n, pos, col, r, r * (self.s.light_range - 1.0).max(0.01)) {
                n += 1;
            }
            if n >= 24 {
                break;
            }
        }
    }

    /// The fireballs' noise volume, made once (its texels kept, so the device can make
    /// it again after a `Reset`, as the managed pool did).
    fn volume(&mut self) -> Option<Texture> {
        if self.vol.is_none() {
            if self.vol_failed {
                return None;
            }
            let t0 = crate::rt::now();
            let sz = self.s.noise;
            let data = noise_fill(sz as usize);
            let g = gfx::gpu();
            let t = g.texture(TextureDesc::d3(sz, sz, sz, 1, Format::Bgra8, Usage::SAMPLED), Contents::Owned(data));
            let v = t.ready(g).then_some(t);
            log!("effects: explosion noise {}^3 in {:.0} ms{}", sz, (crate::rt::now() - t0) * 1000.0, if v.is_none() { " FAILED: no fireballs" } else { "" });
            match v {
                Some(v) => self.vol = Some(v),
                None => {
                    self.vol_failed = true;
                    return None;
                }
            }
        }
        self.vol.clone()
    }

    pub fn draw(&mut self, v: &View, c: &Cam) {
        if self.notes.is_empty() {
            return;
        }
        let alive: Vec<usize> = {
            let mut l = Vec::new();
            for_each_explosion(|e| l.push(e));
            l
        };
        let _ = v;
        let Some(vol) = self.volume() else { return };
        let Some(mut cmd) = gfx::cmd() else { return };
        let cmd = &mut cmd;
        let fire = self.pipes.view(0, || super::fx_desc(gpu::code!(VS_QUAD, VS_QUAD_SPV, VS_QUAD_DXIL), gpu::code!(PS_FIRE, PS_FIRE_SPV, PS_FIRE_DXIL), &super::XYZ, 12, Topology::TriangleStrip, super::PREMUL));
        let ring = self.pipes.view(1, || super::fx_desc(gpu::code!(VS_QUAD, VS_QUAD_SPV, VS_QUAD_DXIL), gpu::code!(PS_RING, PS_RING_SPV, PS_RING_DXIL), &super::XYZ, 12, Topology::TriangleStrip, super::PREMUL));
        let (Some(p_fire), Some(p_ring)) = (fire, ring) else { return };
        cmd.vs_block(Block::new(0, &c.vp));
        let wrap = Sampler {
            min: Filter::Linear,
            mag: Filter::Linear,
            mip: MipFilter::None,
            address_u: Address::Wrap,
            address_v: Address::Wrap,
            address_w: Address::Wrap,
            anisotropy: 1,
            srgb: false,
        };
        cmd.bind_texture(0, Some(&vol), &wrap);
        let s = &self.s;
        for &(e, ring) in &self.notes {
            if !alive.contains(&e) {
                continue;
            }
            let ctr = obj::transform(e);
            let cpos = ctr.p;
            let h = hash_pos(cpos);
            let mut k = [[0.0f32; 4]; 12];
            k[0] = [c.eye.x, c.eye.y, c.eye.z, c.zn];
            k[1] = [cpos.x, cpos.y, cpos.z, 0.0];
            k[4] = [c.p33, c.p43, 0.0, 0.0];
            k[5] = [c.fwd.x, c.fwd.y, c.fwd.z, 0.0];
            k[11] = [s.hdr, s.hdr_from, 0.0, 0.0];
            let rq;
            if !ring {
                let cls = field(e, 0x30);
                let mut r0 = ff(e, 0x34);
                let mut life = if cls != 0 { ff(cls, 0x4c) } else { 2.5 };
                let left = ff(e, 0xa8);
                if !(r0 > 0.5 && r0 < 5000.0) {
                    r0 = 30.0;
                }
                if !(life > 0.1 && life < 60.0) {
                    life = 2.5;
                }
                let mut age = (life - left).max(0.0);
                let mut sl = age / life;
                if s.age >= 0.0 {
                    sl = s.age;
                    age = sl * life;
                }
                let sl = sl.min(1.0);
                if self.logged < 6 {
                    self.logged += 1;
                    log!("effects: fireball at {:.0},{:.0},{:.0}, radius {:.1}, life {:.2} s, first drawn at {:.2} s", cpos.x, cpos.y, cpos.z, r0, life, age);
                }
                let rf = (s.size * r0).max(s.minr);
                let g = 1.0 - (sl / s.grow).min(1.0);
                let rn = rf * (0.2 + 0.8 * (1.0 - g * g * g)) * (1.0 + 0.12 * sl);
                let mut q = rn * 2.4;
                if age < 0.6 && rf * 1.7 > q {
                    q = rf * 1.7;
                }
                rq = q;
                k[1][3] = rf;
                k[2] = [rn, sl, age, s.bright];
                k[3] = [frac_of(h), frac_of(hash32(h.wrapping_add(1))), frac_of(hash32(h.wrapping_add(2))), s.detail * 0.5];
                k[4][2] = s.flash;
                k[4][3] = s.glow;
                k[6] = [s.smoke, s.turb, s.cool, s.opacity];
                if !cmd.set_pipeline(&p_fire) {
                    continue;
                }
            } else {
                let cls = field(e, 0x30);
                let r: f32 = unsafe {
                    let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_SHOCK_RADIUS);
                    f(e)
                };
                let mx = ff(e, 0x50);
                let age = ff(e, 0x4c);
                let fade = if cls != 0 { ff(cls, 0xe0) } else { 0.3 };
                if !(mx > 0.5) || !(r > 0.0) || cls == 0 {
                    continue;
                }
                let mut v1 = 1.0;
                if r > fade * mx && fade < 1.0 {
                    v1 = (mx - r) / ((1.0 - fade) * mx);
                }
                if v1 <= 0.0 {
                    continue;
                }
                if age < 0.08 {
                    v1 *= age / 0.08;
                }
                k[2][3] = s.bright;
                let up = ctr.r[1];
                let nl = up.len();
                k[8] = if nl < 1e-4 { [0.0, 1.0, 0.0, r] } else { [up.x / nl, up.y / nl, up.z / nl, r] };
                k[9] = [s.ring_width, s.ring_bright * v1, mx, frac_of(h) * 8.0];
                let col = super::rv3(cls + 0xd0);
                k[10] = [col.x, col.y, col.z, s.ring_break];
                rq = r * 1.03;
                if !cmd.set_pipeline(&p_ring) {
                    continue;
                }
            }
            let quad = quad(c, cpos, rq);
            cmd.ps_block(Block::new(0, k.as_flattened()));
            if let Some(s) = cmd.vertices(&quad) {
                cmd.draw(&s);
            }
        }
    }
}


/// Calls `f` with every live explosion, in the list's order.
pub fn for_each_explosion(mut f: impl FnMut(usize)) {
    let head = unsafe { engine::rd::<usize>(EXPLOSION_HEAD) };
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != 0 && n != head && guard < 100_000 {
        let next = field(n, 0);
        f(field(n, 8));
        n = next;
        guard += 1;
    }
}

fn hash_pos(p: V3) -> u32 {
    let mut h = 0x811c9dc5u32;
    for x in [p.x, p.y, p.z] {
        h = hash32(h ^ x.to_bits());
    }
    h
}

fn frac_of(h: u32) -> f32 {
    (h >> 8) as f32 * (1.0 / 16777216.0)
}

/// A quad covering a sphere of radius `rq` about `ctr` on screen: a billboard through
/// it, widened so the silhouette fits, or the near plane when the camera is near or in
/// it. Four corners as a strip.
fn quad(w: &Cam, ctr: V3, rq: f32) -> [V3; 4] {
    let dv = ctr - w.eye;
    let d = dv.len();
    let mut out = [V3::ZERO; 4];
    if d - rq > w.zn * 2.0 {
        let ww = dv * (1.0 / d);
        let mut a = w.right - ww * w.right.dot(ww);
        let l = a.len();
        a = if l < 1e-4 { w.up } else { a * (1.0 / l) };
        let b = ww.cross(a);
        let h = rq * d / sqrtf(d * d - rq * rq) * 1.02;
        for (j, o) in out.iter_mut().enumerate() {
            let sa = if j & 1 != 0 { h } else { -h };
            let sb = if j & 2 != 0 { h } else { -h };
            *o = ctr + a * sa + b * sb;
        }
    } else {
        let zq = w.zn * 1.5;
        let hx = zq / w.px * 1.05;
        let hy = zq / w.py * 1.05;
        for (j, o) in out.iter_mut().enumerate() {
            let sa = if j & 1 != 0 { hx } else { -hx };
            let sb = if j & 2 != 0 { hy } else { -hy };
            *o = w.eye + w.fwd * zq + w.right * sa + w.up * sb;
        }
    }
    out
}

// ---- the noise volume ----------------------------------------------------------
//
// The same spectral noise as Nebulae.asi's and Explosions.asi's: random phases over a
// 1/f-like spectrum through an inverse FFT, so it tiles and has no lattice.

fn fft1(re: &mut [f32], im: &mut [f32], tc: &[f32], ts: &[f32]) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut b = n >> 1;
        while j & b != 0 {
            j ^= b;
            b >>= 1;
        }
        j ^= b;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let h = len / 2;
        let step = n / len;
        let mut i = 0;
        while i < n {
            for k in 0..h {
                let (wr, wi) = (tc[k * step], ts[k * step]);
                let (a, b) = (i + k, i + k + h);
                let vr = re[b] * wr - im[b] * wi;
                let vi = re[b] * wi + im[b] * wr;
                re[b] = re[a] - vr;
                im[b] = im[a] - vi;
                re[a] += vr;
                im[a] += vi;
            }
            i += len;
        }
        len <<= 1;
    }
}

fn fft3(re: &mut [f32], im: &mut [f32], s: usize) {
    let tc: Vec<f32> = (0..s / 2).map(|k| libm::cosf(6.2831853 * k as f32 / s as f32)).collect();
    let ts: Vec<f32> = (0..s / 2).map(|k| libm::sinf(6.2831853 * k as f32 / s as f32)).collect();
    for a in 0..s * s {
        fft1(&mut re[a * s..(a + 1) * s], &mut im[a * s..(a + 1) * s], &tc, &ts);
    }
    let mut lr = alloc::vec![0.0f32; s];
    let mut li = alloc::vec![0.0f32; s];
    for ax in 1..3 {
        let stride = if ax == 1 { s } else { s * s };
        for a in 0..s {
            for b in 0..s {
                let base = if ax == 1 { a * s * s + b } else { a * s + b };
                for k in 0..s {
                    lr[k] = re[base + k * stride];
                    li[k] = im[base + k * stride];
                }
                fft1(&mut lr, &mut li, &tc, &ts);
                for k in 0..s {
                    re[base + k * stride] = lr[k];
                    im[base + k * stride] = li[k];
                }
            }
        }
    }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn spectrum(re: &mut [f32], im: &mut [f32], s: usize, k0: f32, k1: f32, seed: u32) {
    let nk = 3 * (s / 2) * (s / 2) + 1;
    let amp: Vec<f32> = (0..nk)
        .map(|i| {
            let k = sqrtf(i as f32);
            if k > 0.5 && k < k1 * 1.6 {
                libm::powf(k, -2.5) * smooth(k0 * 0.6, k0, k) * (1.0 - smooth(k1, k1 * 1.6, k))
            } else {
                0.0
            }
        })
        .collect();
    let si = s as i64;
    for z in 0..s {
        for y in 0..s {
            for x in 0..s {
                let f = |c: usize| { let c = c as i64; if c < si / 2 { c } else { c - si } };
                let (kx, ky, kz) = (f(x), f(y), f(z));
                let a = amp[(kx * kx + ky * ky + kz * kz) as usize];
                let i = (z * s + y) * s + x;
                if a > 0.0 {
                    let ph = (hash32((i as u32).wrapping_mul(0x9e3779b9) ^ seed) >> 22) as f32 * (6.2831853 / 1024.0);
                    re[i] = a * libm::cosf(ph);
                    im[i] = a * libm::sinf(ph);
                } else {
                    re[i] = 0.0;
                    im[i] = 0.0;
                }
            }
        }
    }
    fft3(re, im, s);
}

fn channel_out(dst: &mut [u8], v: &[f32], byte: usize, want: f32) {
    let n = v.len();
    let (mut sum, mut sq) = (0.0f64, 0.0f64);
    for &x in v {
        sum += x as f64;
        sq += x as f64 * x as f64;
    }
    let mean = (sum / n as f64) as f32;
    let sd = sqrtf((sq / n as f64 - (sum / n as f64) * (sum / n as f64)) as f32).max(1e-6);
    for (i, &x) in v.iter().enumerate() {
        let b = ((0.5 + (x - mean) / sd * want) * 255.0 + 0.5) as i32;
        dst[i * 4 + byte] = b.clamp(0, 255) as u8;
    }
}

/// r: the fractal field (2..S/4 periods across the tile, mean 0.5, spread 0.125);
/// g, b, a: three soft fields of 1..4 periods for the warp (spread 0.2). BGRA bytes.
fn noise_fill(s: usize) -> Vec<u8> {
    let n = s * s * s;
    let mut dst = alloc::vec![0u8; n * 4];
    let mut re = alloc::vec![0.0f32; n];
    let mut im = alloc::vec![0.0f32; n];
    spectrum(&mut re, &mut im, s, 2.0, s as f32 / 4.0, 0x51f15e5d);
    channel_out(&mut dst, &re, 2, 0.125);
    spectrum(&mut re, &mut im, s, 1.0, 4.0, 0x2545f491);
    channel_out(&mut dst, &re, 1, 0.2);
    channel_out(&mut dst, &im, 0, 0.2);
    spectrum(&mut re, &mut im, s, 1.0, 4.0, 0x6a09e667);
    channel_out(&mut dst, &re, 3, 0.2);
    dst
}

impl Default for Explosions {
    fn default() -> Explosions {
        Explosions::new()
    }
}
