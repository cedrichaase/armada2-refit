//! Nebulae drawn as volumes of gas: the port of `Nebulae.asi` (whose
//! decisions hold here: the envelope, the bake, the slices on a ladder fixed in distance
//! from the eye kept per view, the blue-noise thresholds that stay put, the spectral
//! noise). A class is drawn as gas when
//! it has a recipe, `Nebulae\<odf>.ini` in the game directory (`render3d/nebulae/`);
//! anything else keeps the stock billboards, drawn where they always were.
//!
//! What changed in the move:
//! - The gas is drawn at `Step::Translucent`, just before the device's `Flush`: after
//!   the opaque scene and the particles, where the plugin drew it after
//!   `RenderParticleList`. Our scene target is multisampled, and RESZ resolves its depth
//!   into an INTZ copy that is the gas target's depth buffer.
//! - The gas is marched at full size, a ray per pixel through the baked volume up to the
//!   scene's depth (RESZ into INTZ), into a floating-point target, and filtered a pixel or
//!   two onto the frame: the plugin's slices (kept only without INTZ, or with `Bake=0`),
//!   half-size pass, depth-weighted composite and `EdgeRefine=` are gone, and with them
//!   `Resolution=` and `EdgeRefine=`.
//! - A class's billboards are skipped in the object loop (`Module::object`) rather than
//!   by patching `Nebula::Render`; the nebula is still marked on screen (`+0x25`, as
//!   `Entity::Render` marks it when its instance passes the frustum test), which the
//!   engine's nebula lights depend on.
//! - Classes are kept by ODF name, not by the engine's class pointer, so nothing has to
//!   be forgotten when a game frees its classes (`GameObjectClass_Clean`).
//! - The Key comes from the main view's registered lights (the brightest directional
//!   light), not from `Lighting.asi`'s export; the ships' shadows from the hulls
//!   module's shadow map of the Key (`hulls::shadow_map`), not `LightingShadowMap`.
//! - Everything on the device is in the default pool (staging copies are released), and
//!   a `Reset` drops it: the noise is made again and every class bakes again.
//! - Time: the gas's motion runs on the sim clock (`View::time`), so it stops with the
//!   game's pause.
//! - Ghosts: slot 3 of the `GameObjectSpirit` vtable (0x6b1ca4, `GameObjectSpirit::Render`
//!   0x4d5f70) is wrapped while this part runs, so a ghosted nebula of a class drawn as
//!   gas keeps its gas and loses its billboards, as with the plugin.
use crate::d3d::*;
use crate::engine::{self, field, field_u8, hook, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, PerView, Shader};
use crate::math;
use crate::rt::{self, Ini};
use crate::shaders::world_nebula::*;
use crate::{log, scene};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;

pub const VT_NEBULA: usize = 0x6b148c;
pub const VT_LATINUM: usize = 0x6b132c;
const NEBULA_LIST: usize = 0x73b1bc; // Nebula::nebulaList, std::vector<Nebula *> *
const GHOST_LIST: usize = 0x736c90; // Ghost::ghostList, std::list<Spirit *>
const VT_SPIRIT: usize = 0x6b1ca4;
const SLOT_SPIRIT_RENDER: usize = 3;
const FN_SPIRIT_RENDER: usize = 0x4d5f70;
const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), cdecl

const NCONST: usize = 21;
const MAX_NEB: usize = 256;
const ENV_MAX: u32 = 512;
const BN: usize = 64;
const FVF_XYZ: u32 = 0x2;
const FVF_SLICE: u32 = 0x2 | 0x100; // XYZ | TEX1 (a float2)
const RS_POINTSIZE: u32 = 154;
const RESZ_CODE: u32 = 0x7fa0_5000;

#[derive(Clone, Copy)]
struct Settings {
    enable: bool,
    slices: i32,
    steps: i32,
    denoise: f32,
    footprint: f32,
    min_step: f32,
    noise: u32,
    obscure: f32,
    samples: i32,
    key_share: f32,
    scatter: f32,
    bake: bool,
    vol_max: u32,
    vol_maxy: u32,
    vol_texel: f32,
    shadows: bool,
    shadow_soft: f32,
}

impl Settings {
    /// `[Nebulae]` in Render3D.ini, with `Nebulae.ini`'s keys and defaults.
    fn read() -> Settings {
        let ini = crate::config::ini();
        let s = "Nebulae";
        let noise = ini.int(s, "NoiseSize", 128);
        Settings {
            enable: ini.int(s, "Enable", 1) != 0,
            slices: ini.int(s, "Slices", 40).clamp(4, 160),
            steps: ini.int(s, "Steps", 8),
            denoise: math::clamp(ini.float(s, "Denoise", 1.5), 0.0, 3.0),
            footprint: math::clamp(ini.float(s, "Footprint", 1.0), 0.25, 8.0),
            min_step: ini.float(s, "MinStep", 6.0).max(1.0),
            noise: if noise == 32 || noise == 64 || noise == 128 { noise as u32 } else { 128 },
            obscure: math::saturate(ini.float(s, "Obscure", 0.5)),
            samples: if ini.int(s, "Samples", 2) >= 2 { 2 } else { 1 },
            key_share: math::saturate(ini.float(s, "KeyShare", 0.8)),
            scatter: math::clamp(ini.float(s, "Scatter", 0.2), 0.0, 0.8),
            bake: ini.int(s, "Bake", 1) != 0,
            vol_max: ini.int(s, "VolumeSize", 256).clamp(16, 512) as u32,
            vol_maxy: ini.int(s, "VolumeHeight", 160).clamp(8, 256) as u32,
            vol_texel: ini.float(s, "VolumeTexel", 3.0).max(0.5),
            shadows: ini.int(s, "Shadows", 1) != 0,
            shadow_soft: math::clamp(ini.float(s, "ShadowSoftness", 0.02), 0.0, 0.5),
        }
    }
}

/// A nebula class with its recipe, and what is on the device for it.
struct Cls {
    odf: String,
    path: String,
    bytes: Option<Vec<u8>>,
    have: bool,
    failed: bool,
    generation: u32,
    k: [[f32; 4]; NCONST],
    /// c24..c31.
    ex: [[f32; 4]; 8],
    /// c38.
    fz: [f32; 4],
    fuzz_speed: f32,
    fringe_reach: f32,
    /// c40.
    sw: [f32; 4],
    swirl_period: f32,
    ring_cycle: f32,
    pulse_speed: f32,
    pulse_wander: f32,
    pulse_phase: f32,
    pulse_last: f32,
    extent: f32,
    height: f32,
    radius: f32,
    radius0: f32,
    flow_speed: f32,
    edge_warp: f32,
    fil_reach: f32,
    edge_dim: f32,
    opacity: f32,
    key_share: f32,
    scatter: f32,
    shade_step: f32,
    light0: [f32; 3],
    ydev: f32,
    hue_cycle: f32,
    hue_swing: f32,
    bolt_rate: f32,
    bolt_size: f32,
    bolt_bright: f32,
    bolt_t0: [f32; 4],
    bolt_pos: [[f32; 3]; 4],
    bolt_amp: [f32; 4],
    bolt_last: f32,
    /// This frame's nebulae of the class, the fog of war applied.
    pos: Vec<[f32; 3]>,
    sig: u32,
    /// The signature this view draws with: the nebulae's and the Key's.
    cur: u32,
    env: Option<Com>,
    env_sig: u32,
    env_gen: u32,
    ex0: f32,
    ez0: f32,
    ex1: f32,
    ez1: f32,
    ey: f32,
    vol: Option<Com>,
    occ: Option<Com>,
    vol_sig: u32,
    vol_gen: u32,
    vx: u32,
    vy: u32,
    vz: u32,
    vlo: [f32; 3],
    vsize: [f32; 3],
    glo: [f32; 3],
    ghi: [f32; 3],
    gbox: bool,
}

/// `n` numbers under `key` of `[Nebula]`, or None unless all of them are there.
fn vals(f: &Ini, key: &str, n: usize) -> Option<Vec<f32>> {
    let s = f.string("Nebula", key)?;
    let v: Vec<f32> = s.split([',', ' ', '\t']).filter(|p| !p.is_empty()).filter_map(rt::parse_f32).collect();
    if v.len() >= n { Some(v[..n].to_vec()) } else { None }
}
fn one(f: &Ini, key: &str, def: f32) -> f32 {
    vals(f, key, 1).map_or(def, |v| v[0])
}
fn rgb(f: &Ini, key: &str) -> [f32; 4] {
    let v = vals(f, key, 3).unwrap_or(vec![0.0, 0.0, 0.0]);
    [v[0] / 255.0, v[1] / 255.0, v[2] / 255.0, 0.0]
}

impl Cls {
    fn new(odf: &str, latinum: bool, radius0: f32) -> Cls {
        let mut path = rt::game_path("Nebulae\\");
        path.push_str(odf);
        path.push_str(".ini");
        let mut c = Cls {
            odf: String::from(odf), path, bytes: None, have: false, failed: false, generation: 0,
            k: [[0.0; 4]; NCONST], ex: [[0.0; 4]; 8], fz: [0.0; 4], fuzz_speed: 0.0, fringe_reach: 0.0, sw: [0.0; 4],
            swirl_period: 40.0, ring_cycle: 6.0, pulse_speed: 0.5, pulse_wander: 0.5, pulse_phase: 0.0, pulse_last: -1.0,
            extent: 1.3, height: 3.0, radius: radius0, radius0, flow_speed: 0.0, edge_warp: 0.0, fil_reach: 0.0,
            edge_dim: 0.5, opacity: 0.0, key_share: -1.0, scatter: -1.0, shade_step: 60.0, light0: [0.0; 3], ydev: 1.0,
            hue_cycle: 0.0, hue_swing: 0.5, bolt_rate: 0.0, bolt_size: 0.3, bolt_bright: 1.0, bolt_t0: [0.0; 4],
            bolt_pos: [[0.0; 3]; 4], bolt_amp: [0.0; 4], bolt_last: -1.0, pos: Vec::new(), sig: 0, cur: 0, env: None, env_sig: 0,
            env_gen: u32::MAX, ex0: 0.0, ez0: 0.0, ex1: 0.0, ez1: 0.0, ey: 0.0, vol: None, occ: None, vol_sig: 0,
            vol_gen: u32::MAX, vx: 0, vy: 0, vz: 0, vlo: [0.0; 3], vsize: [0.0; 3], glo: [0.0; 3], ghi: [0.0; 3], gbox: false,
        };
        c.bytes = rt::read_file(&c.path);
        c.have = c.bytes.is_some() && c.read();
        log!(
            "world: nebula class {}{}, radius {:.0}: {}",
            odf, if latinum { " (latinum)" } else { "" }, c.radius,
            if c.have { "drawn as gas" } else { "no recipe, the stock billboards" }
        );
        c
    }

    /// As Nebulae.asi read a recipe.
    fn read(&mut self) -> bool {
        let f = Ini::open(&self.path);
        if !f.exists() {
            return false;
        }
        self.k = [[0.0; 4]; NCONST];
        let k = &mut self.k;
        self.radius = one(&f, "Radius", self.radius0);
        if !(self.radius > 10.0 && self.radius < 5000.0) {
            self.radius = self.radius0;
        }
        k[0][3] = one(&f, "NearFade", 60.0);
        self.ex = [[0.0; 4]; 8];
        let ex = &mut self.ex;
        ex[0][0] = one(&f, "Core", 0.0);
        ex[0][1] = one(&f, "CoreSharpness", 4.0);
        ex[1][0] = one(&f, "Ring", 0.0);
        ex[1][1] = one(&f, "RingAt", 0.4);
        ex[1][2] = one(&f, "RingWidth", 0.3).max(0.01);
        ex[2] = [one(&f, "RingScale", 0.35), one(&f, "RingPatch", 0.5), one(&f, "RingSoftness", 0.15), one(&f, "RingSpread", 0.5)];
        ex[3] = rgb(&f, "RingA");
        ex[4] = rgb(&f, "RingB");
        ex[5] = rgb(&f, "RingC");
        ex[6] = rgb(&f, "RingD");
        ex[0][2] = one(&f, "Pulse", 0.0);
        self.pulse_speed = one(&f, "PulseSpeed", 0.5);
        self.pulse_wander = one(&f, "PulseWander", 0.5);
        self.ring_cycle = one(&f, "RingCycle", 6.0).max(0.1);
        self.key_share = one(&f, "KeyShare", -1.0).min(1.0);
        self.scatter = one(&f, "Scatter", -1.0).min(0.8);
        self.opacity = one(&f, "Opacity", 0.0).max(0.0);
        k[1] = rgb(&f, "GasA");
        k[2] = rgb(&f, "GasB");
        k[3] = rgb(&f, "Glow");
        k[1][3] = one(&f, "Brightness", 1.0);
        k[2][3] = one(&f, "HueScale", 2.0);
        k[3][3] = one(&f, "Gamma", 1.0).max(0.05);
        k[4] = [1.0 / one(&f, "Tile", 900.0), one(&f, "Warp", 0.35), one(&f, "Coverage", 0.0), one(&f, "Softness", 2.5).max(0.01)];
        let seed = one(&f, "Seed", 1.0).abs();
        k[5] = [math::fmodf(seed * 0.129898, 1.0), math::fmodf(seed * 0.78233, 1.0), math::fmodf(seed * 0.37719, 1.0), one(&f, "Detail", 0.3)];
        k[7][2] = 1.0 / 255.0;
        k[8] = [one(&f, "Ridge", 0.0), one(&f, "HueMix", 0.4), one(&f, "WarpScale", 0.45), one(&f, "Lumps", 0.35)];
        k[9] = [one(&f, "Knots", 1.5), one(&f, "KnotScale", 0.35), one(&f, "Lanes", 0.5), one(&f, "LaneScale", 1.7)];
        {
            let l = vals(&f, "Light", 2).unwrap_or(vec![30.0, 60.0]);
            let st = one(&f, "ShadeStep", 60.0);
            self.shade_step = st;
            let (y, pt) = (l[0] * 0.017_453_293, l[1] * 0.017_453_293);
            k[10] = [math::cosf(pt) * math::sinf(y) * st, math::sinf(pt) * st, math::cosf(pt) * math::cosf(y) * st, one(&f, "Shading", 1.0)];
            self.light0 = [k[10][0], k[10][1], k[10][2]];
        }
        k[11][3] = one(&f, "FineDetail", 0.35);
        self.flow_speed = one(&f, "FlowSpeed", 0.03);
        k[13][1] = one(&f, "Flow", 60.0);
        k[13][2] = one(&f, "FlowScale", 2.0);
        self.edge_warp = one(&f, "EdgeWarp", 0.35);
        k[14][0] = one(&f, "Filaments", 0.0);
        k[14][1] = one(&f, "FilamentScale", 0.6);
        k[14][2] = one(&f, "FilamentSharpness", 3.0);
        self.fil_reach = one(&f, "FilamentReach", 1.8);
        self.fz = [one(&f, "Fringe", 0.0), one(&f, "FringeScale", 0.5), one(&f, "Fuzz", 0.0), 0.0];
        self.fringe_reach = one(&f, "FringeReach", 1.6);
        self.fuzz_speed = one(&f, "FuzzSpeed", 0.01);
        self.sw = [one(&f, "Swirl", 0.0).max(0.0), one(&f, "SwirlScale", 1.0), 0.0, 0.0];
        self.swirl_period = one(&f, "SwirlPeriod", 40.0).max(1.0);
        self.edge_dim = one(&f, "EdgeOnDim", 0.5);
        self.hue_cycle = one(&f, "HueCycle", 0.0);
        self.hue_swing = one(&f, "HueSwing", 0.5);
        self.bolt_rate = one(&f, "Lightning", 0.0);
        self.bolt_size = one(&f, "LightningSize", 0.3);
        self.bolt_bright = one(&f, "LightningBrightness", 1.0);
        let k = &mut self.k;
        k[20] = rgb(&f, "LightningColour");
        if k[20][0] + k[20][1] + k[20][2] <= 0.0 {
            k[20] = [1.0, 0.92, 0.8, 0.0];
        }
        self.bolt_amp = [0.0; 4];
        self.extent = one(&f, "Extent", 1.3);
        self.height = one(&f, "Height", 3.0);
        self.generation = self.generation.wrapping_add(1);
        self.failed = false;
        log!("world: nebula recipe {} read", self.path);
        true
    }

    fn reload(&mut self) {
        let b = rt::read_file(&self.path);
        if b != self.bytes {
            self.bytes = b;
            self.have = self.bytes.is_some() && self.read();
        }
    }

    fn drop_device(&mut self) {
        self.env = None;
        self.vol = None;
        self.occ = None;
        self.env_gen = u32::MAX;
        self.vol_gen = u32::MAX;
        self.failed = false;
    }

    fn reach(&self) -> f32 {
        self.radius * self.extent
    }

    /// c6, c7, c11, c13.w, c14.w, c31 for the class's gas where it is now.
    fn place(&mut self) {
        let reach = self.reach();
        self.k[6] = [self.ex0, self.ez0, 1.0 / (self.ex1 - self.ex0), 1.0 / (self.ez1 - self.ez0)];
        self.k[7][0] = self.ey;
        self.k[7][1] = reach * self.height;
        self.k[7][3] = 1.0 / reach;
        self.ex[7][0] = self.opacity / reach;
        self.ex[7][1] = self.ydev;
        self.k[11][0] = self.vlo[0];
        self.k[11][1] = self.vlo[1];
        self.k[11][2] = self.vlo[2];
        self.k[13][3] = self.edge_warp * reach;
        self.k[14][3] = self.k[7][1] * 0.8;
    }
}

// ---- Direct3D 9 helpers this part needs beyond the shared layer ----------------------

#[repr(C)]
struct LockedBox {
    row: i32,
    slice: i32,
    bits: *mut u8,
}

fn create_offscreen(dev: Device, w: u32, h: u32, fmt: u32, pool: u32) -> Option<Com> {
    let mut o: *mut c_void = core::ptr::null_mut();
    let hr = vcall!(dev.p(), 36, fn(u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT, w, h, fmt, pool, &mut o, core::ptr::null_mut());
    if hr < 0 || o.is_null() { None } else { Some(Com(o)) }
}
fn rt_data(dev: Device, rt: &Com, sys: &Com) -> bool {
    vcall!(dev.p(), 32, fn(*mut c_void, *mut c_void) -> HRESULT, rt.ptr(), sys.ptr()) >= 0
}
fn surf_lock(s: &Com, flags: u32) -> Option<LockedRect> {
    let mut lr = LockedRect { pitch: 0, bits: core::ptr::null_mut() };
    let hr = vcall!(s.ptr(), 13, fn(*mut LockedRect, *const Rect, u32) -> HRESULT, &mut lr, core::ptr::null(), flags);
    if hr < 0 { None } else { Some(lr) }
}
fn surf_unlock(s: &Com) {
    vcall!(s.ptr(), 14, fn() -> HRESULT,);
}
fn vol_lock(v: &Com, level: u32) -> Option<LockedBox> {
    let mut b = LockedBox { row: 0, slice: 0, bits: core::ptr::null_mut() };
    let hr = vcall!(v.ptr(), 19, fn(u32, *mut LockedBox, *const u32, u32) -> HRESULT, level, &mut b, core::ptr::null(), 0);
    if hr < 0 { None } else { Some(b) }
}
fn vol_unlock(v: &Com, level: u32) {
    vcall!(v.ptr(), 20, fn(u32) -> HRESULT, level);
}

/// A slice or quad vertex: position, then (thickness, rung).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vert {
    x: f32,
    y: f32,
    z: f32,
    thick: f32,
    k: f32,
}

fn quad(dev: Device) {
    let mut v = [Vert::default(); 4];
    for (j, p) in v.iter_mut().enumerate() {
        p.x = if j & 1 != 0 { 1.0 } else { -1.0 };
        p.y = if j & 2 != 0 { -1.0 } else { 1.0 };
    }
    dev.draw_primitive_up(PT_TRIANGLESTRIP, 2, v.as_ptr() as *const u8, 20);
}

fn ss(dev: Device, s: u32, list: &[(u32, u32)]) {
    for &(k, v) in list {
        dev.set_ss(s, k, v);
    }
}
const SS_CLAMP_POINT: [(u32, u32); 7] = [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_ADDRESSW, TADDRESS_CLAMP), (SAMP_MAGFILTER, TEXF_POINT), (SAMP_MINFILTER, TEXF_POINT), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)];
const SS_CLAMP_LINEAR: [(u32, u32); 7] = [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_ADDRESSW, TADDRESS_CLAMP), (SAMP_MAGFILTER, TEXF_LINEAR), (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)];

// ---- the noise volume ----------------------------------------------------------------

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

struct Fft {
    tw_c: Vec<f32>,
    tw_s: Vec<f32>,
}

impl Fft {
    fn new(n: usize) -> Fft {
        let mut tw_c = vec![0.0; n / 2];
        let mut tw_s = vec![0.0; n / 2];
        for k in 0..n / 2 {
            let a = 6.283_185_3 * k as f32 / n as f32;
            tw_c[k] = math::cosf(a);
            tw_s[k] = math::sinf(a);
        }
        Fft { tw_c, tw_s }
    }
    /// One line of n (a power of two), in place, radix 2.
    fn line(&self, re: &mut [f32], im: &mut [f32]) {
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
            let (h, step) = (len / 2, n / len);
            let mut i = 0;
            while i < n {
                for j in 0..h {
                    let (wr, wi) = (self.tw_c[j * step], self.tw_s[j * step]);
                    let (a, b) = (i + j, i + j + h);
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
    /// The whole S^3 volume, along each axis in turn.
    fn volume(&self, re: &mut [f32], im: &mut [f32], s: usize) {
        for a in 0..s * s {
            self.line(&mut re[a * s..a * s + s], &mut im[a * s..a * s + s]);
        }
        let mut lr = vec![0.0f32; s];
        let mut li = vec![0.0f32; s];
        for ax in 1..3 {
            let stride = if ax == 1 { s } else { s * s };
            for a in 0..s {
                for b in 0..s {
                    let base = if ax == 1 { a * s * s + b } else { a * s + b };
                    for k in 0..s {
                        lr[k] = re[base + k * stride];
                        li[k] = im[base + k * stride];
                    }
                    self.line(&mut lr, &mut li);
                    for k in 0..s {
                        re[base + k * stride] = lr[k];
                        im[base + k * stride] = li[k];
                    }
                }
            }
        }
    }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = math::saturate((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

/// Waves from k0 to k1 periods across the volume, amplitude k^-2.5, random phases: two
/// independent tiling fields in the real and imaginary parts.
fn spectrum(fft: &Fft, re: &mut [f32], im: &mut [f32], s: usize, k0: f32, k1: f32, seed: u32) {
    let n2 = 3 * (s / 2) * (s / 2) + 1;
    let mut amp = vec![0.0f32; n2];
    for (i, a) in amp.iter_mut().enumerate() {
        let k = math::sqrtf(i as f32);
        *a = if k > 0.5 && k < k1 * 1.6 { math::powf(k, -2.5) * smooth(k0 * 0.6, k0, k) * (1.0 - smooth(k1, k1 * 1.6, k)) } else { 0.0 };
    }
    let (mut pc, mut ps) = (vec![0.0f32; 1024], vec![0.0f32; 1024]);
    for i in 0..1024 {
        let a = 6.283_185_3 * i as f32 / 1024.0;
        pc[i] = math::cosf(a);
        ps[i] = math::sinf(a);
    }
    let h = s as i32 / 2;
    for z in 0..s {
        for y in 0..s {
            for x in 0..s {
                let f = |v: usize| if (v as i32) < h { v as i32 } else { v as i32 - s as i32 };
                let (kx, ky, kz) = (f(x), f(y), f(z));
                let a = amp[(kx * kx + ky * ky + kz * kz) as usize];
                let i = (z * s + y) * s + x;
                if a > 0.0 {
                    let ph = (hash32((i as u32).wrapping_mul(0x9e37_79b9) ^ seed) >> 22) as usize;
                    re[i] = a * pc[ph];
                    im[i] = a * ps[ph];
                } else {
                    re[i] = 0.0;
                    im[i] = 0.0;
                }
            }
        }
    }
    fft.volume(re, im, s);
}

/// A field into one byte of each texel, normalised to mean 0.5 and the given spread.
fn channel_out(dst: &mut [u8], v: &[f32], byte: usize, want: f32) {
    let n = v.len();
    let (mut sum, mut sq) = (0.0f64, 0.0f64);
    for &x in v {
        sum += x as f64;
        sq += x as f64 * x as f64;
    }
    let mean = (sum / n as f64) as f32;
    let sd = math::sqrtf((sq / n as f64 - (sum / n as f64) * (sum / n as f64)) as f32).max(1e-6);
    for i in 0..n {
        let b = ((0.5 + (v[i] - mean) / sd * want) * 255.0 + 0.5) as i32;
        dst[i * 4 + byte] = b.clamp(0, 255) as u8;
    }
}

/// r: the fractal field (2..S/4 periods, spread 0.125); g, b, a: three soft fields (1..4
/// periods, spread 0.2). A8R8G8B8 bytes b, g, r, a.
fn noise_fill(s: usize) -> Vec<u8> {
    let n = s * s * s;
    let fft = Fft::new(s);
    let mut out = vec![0u8; n * 4];
    let mut re = vec![0.0f32; n];
    let mut im = vec![0.0f32; n];
    spectrum(&fft, &mut re, &mut im, s, 2.0, s as f32 / 4.0, 0x51f1_5e5d);
    channel_out(&mut out, &re, 2, 0.125);
    spectrum(&fft, &mut re, &mut im, s, 1.0, 4.0, 0x2545_f491);
    channel_out(&mut out, &re, 1, 0.2);
    channel_out(&mut out, &im, 0, 0.2);
    spectrum(&fft, &mut re, &mut im, s, 1.0, 4.0, 0x6a09_e667);
    channel_out(&mut out, &re, 3, 0.2);
    out
}

/// A volume texture in the default pool from levels of BGRA8 bytes, through a staging
/// copy in system memory.
fn upload_volume(dev: Device, w: u32, h: u32, d: u32, levels: &[Vec<u8>]) -> Option<Com> {
    let n = levels.len() as u32;
    let stage = dev.create_volume_texture(w, h, d, n, 0, FMT_A8R8G8B8, POOL_SYSTEMMEM)?;
    for (l, data) in levels.iter().enumerate() {
        let (lw, lh, ld) = ((w >> l).max(1) as usize, (h >> l).max(1) as usize, (d >> l).max(1) as usize);
        let b = vol_lock(&stage, l as u32)?;
        for z in 0..ld {
            for y in 0..lh {
                let src = &data[((z * lh + y) * lw) * 4..((z * lh + y) * lw + lw) * 4];
                unsafe {
                    core::ptr::copy_nonoverlapping(src.as_ptr(), b.bits.add(z * b.slice as usize + y * b.row as usize), lw * 4);
                }
            }
        }
        vol_unlock(&stage, l as u32);
    }
    let v = dev.create_volume_texture(w, h, d, n, 0, FMT_A8R8G8B8, POOL_DEFAULT)?;
    if dev.update_texture(&stage, &v) < 0 {
        return None;
    }
    Some(v)
}

/// The noise volume with its chain (box-filtered, wrapping).
fn make_noise(dev: Device, s: u32) -> Option<Com> {
    let t0 = rt::now();
    let su = s as usize;
    let mut levels = vec![noise_fill(su)];
    let mut n = su;
    while n > 1 {
        let h = n / 2;
        let src = levels.last().unwrap();
        let mut nxt = vec![0u8; h * h * h * 4];
        for z in 0..h {
            for y in 0..h {
                for x in 0..h {
                    for c in 0..4 {
                        let mut sum = 0u32;
                        for dz in 0..2 {
                            for dy in 0..2 {
                                for dx in 0..2 {
                                    sum += src[(((2 * z + dz) * n + (2 * y + dy)) * n + (2 * x + dx)) * 4 + c] as u32;
                                }
                            }
                        }
                        nxt[((z * h + y) * h + x) * 4 + c] = ((sum + 4) / 8) as u8;
                    }
                }
            }
        }
        levels.push(nxt);
        n = h;
    }
    let v = upload_volume(dev, s, s, s, &levels);
    log!("world: nebula noise {}^3, {} levels, {} in {:.0} ms", s, levels.len(), if v.is_some() { "made" } else { "FAILED" }, (rt::now() - t0) * 1000.0);
    v
}

// ---- the blue noise --------------------------------------------------------------------

/// A 64x64 blue-noise tile by void and cluster (Ulichney 1993), as Nebulae.asi makes it:
/// red the rank, green the rank of the tile shifted by half (a second threshold).
fn blue_noise() -> Vec<u8> {
    let n = BN * BN;
    let mut kern = vec![0.0f32; n];
    for y in 0..BN {
        for x in 0..BN {
            let dx = if x > BN / 2 { BN - x } else { x };
            let dy = if y > BN / 2 { BN - y } else { y };
            let d2 = (dx * dx + dy * dy) as f32;
            kern[y * BN + x] = math::expf(-d2 / (2.0 * 1.5 * 1.5));
        }
    }
    let mut e = vec![0.0f32; n];
    let mut on = vec![false; n];
    let mut rank = vec![0u16; n];
    let add = |e: &mut [f32], i: usize, s: f32| {
        let (x0, y0) = (i % BN, i / BN);
        for y in 0..BN {
            for x in 0..BN {
                e[y * BN + x] += s * kern[((y + BN - y0) % BN) * BN + (x + BN - x0) % BN];
            }
        }
    };
    let find = |e: &[f32], on: &[bool], want: bool, most: bool| -> usize {
        let mut best = usize::MAX;
        let mut v = 0.0f32;
        for i in 0..n {
            if on[i] != want {
                continue;
            }
            if best == usize::MAX || (if most { e[i] > v } else { e[i] < v }) {
                best = i;
                v = e[i];
            }
        }
        best
    };
    let mut h = 0x2545_f491u32;
    let mut ones = 0;
    for i in 0..n / 10 {
        h = hash32(h.wrapping_add(i as u32));
        let r = (h as usize) % n;
        if !on[r] {
            on[r] = true;
            add(&mut e, r, 1.0);
            ones += 1;
        }
    }
    let mut guard = 0;
    loop {
        let c = find(&e, &on, true, true);
        on[c] = false;
        add(&mut e, c, -1.0);
        let v = find(&e, &on, false, false);
        on[v] = true;
        add(&mut e, v, 1.0);
        guard += 1;
        if v == c || guard > 100_000 {
            break;
        }
    }
    let init = on.clone();
    for r in (0..ones).rev() {
        let c = find(&e, &on, true, true);
        on[c] = false;
        add(&mut e, c, -1.0);
        rank[c] = r as u16;
    }
    e.iter_mut().for_each(|x| *x = 0.0);
    for i in 0..n {
        on[i] = init[i];
        if init[i] {
            add(&mut e, i, 1.0);
        }
    }
    for r in ones..n {
        let v = find(&e, &on, false, false);
        on[v] = true;
        add(&mut e, v, 1.0);
        rank[v] = r as u16;
    }
    let mut px = vec![0u8; n * 4];
    for y in 0..BN {
        for x in 0..BN {
            let a = (rank[y * BN + x] >> 4) as u8;
            let b = (rank[((y + BN / 2) % BN) * BN + (x + 17) % BN] >> 4) as u8;
            let o = (y * BN + x) * 4;
            px[o] = 0;
            px[o + 1] = b;
            px[o + 2] = a;
            px[o + 3] = 255;
        }
    }
    px
}

// ---- the slices --------------------------------------------------------------------------

/// The polygon where the plane dot(x - eye, f) = d cuts the box lo..hi, ordered round its
/// centre in the plane's basis (r, u).
fn cut_box(lo: &[f32; 3], hi: &[f32; 3], eye: &[f32; 3], f: &[f32; 3], r: &[f32; 3], u: &[f32; 3], d: f32, out: &mut [[f32; 3]; 6]) -> usize {
    const E: [[usize; 2]; 12] = [[0, 1], [2, 3], [4, 5], [6, 7], [0, 2], [1, 3], [4, 6], [5, 7], [0, 4], [1, 5], [2, 6], [3, 7]];
    let mut c = [[0.0f32; 3]; 8];
    let mut s = [0.0f32; 8];
    for i in 0..8 {
        c[i] = [if i & 1 != 0 { hi[0] } else { lo[0] }, if i & 2 != 0 { hi[1] } else { lo[1] }, if i & 4 != 0 { hi[2] } else { lo[2] }];
        s[i] = (c[i][0] - eye[0]) * f[0] + (c[i][1] - eye[1]) * f[1] + (c[i][2] - eye[2]) * f[2] - d;
    }
    let mut n = 0;
    for e in E {
        if n >= 6 {
            break;
        }
        let (a, b) = (s[e[0]], s[e[1]]);
        if (a < 0.0) == (b < 0.0) {
            continue;
        }
        let t = a / (a - b);
        for j in 0..3 {
            out[n][j] = c[e[0]][j] + (c[e[1]][j] - c[e[0]][j]) * t;
        }
        n += 1;
    }
    if n < 3 {
        return 0;
    }
    let mut ctr = [0.0f32; 3];
    for p in out.iter().take(n) {
        for j in 0..3 {
            ctr[j] += p[j] / n as f32;
        }
    }
    let mut ang = [0.0f32; 6];
    for i in 0..n {
        let p = [out[i][0] - ctr[0], out[i][1] - ctr[1], out[i][2] - ctr[2]];
        let a = p[0] * r[0] + p[1] * r[1] + p[2] * r[2];
        let b = p[0] * u[0] + p[1] * u[1] + p[2] * u[2];
        let q = a.abs() + b.abs();
        let t = if q > 0.0 { a / q } else { 0.0 };
        ang[i] = if b >= 0.0 { 1.0 - t } else { 3.0 + t };
    }
    for i in 1..n {
        let (ta, tp) = (ang[i], out[i]);
        let mut j = i;
        while j > 0 && ang[j - 1] > ta {
            ang[j] = ang[j - 1];
            out[j] = out[j - 1];
            j -= 1;
        }
        ang[j] = ta;
        out[j] = tp;
    }
    n
}

// ---- the part ------------------------------------------------------------------------------

/// What the device holds for every class: the noise, the blue noise, the copy of the
/// scene's depth the march reads, and the gas target it adds up into.
#[derive(Default)]
struct Gpu {
    noise: Option<Com>,
    blue: Option<Com>,
    failed: bool,
    zt: Option<Com>,
    fb: Option<Com>,
    zt_wh: (u32, u32),
    zt_broken: bool,
}

struct Shaders {
    vs: Shader,
    ps: Shader,
    bake_vs: Shader,
    bake_ps: Shader,
    vol_ps: Shader,
    march_vs: Shader,
    /// The march in 8, 12, 16 and 24 steps (`Steps=`).
    march: [Shader; 4],
    quad_vs: Shader,
    comp_ps: Shader,
}

/// Per view: the slices' ladder per class (rungs per doubling of distance).
#[derive(Default)]
struct ViewState {
    rungs: BTreeMap<String, f32>,
}

/// The march of one view: c21 (Obscure=, 1 / the depth copy's size) and c41..c46 (the
/// rays per pixel, the projection's depth terms), the same for every class but the box;
/// the frame's target and depth to go back to, the gas target, the viewport.
struct March {
    k: [[f32; 4]; 7],
    ps: usize,
    rt0: Com,
    ds0: Option<Com>,
    fs: Com,
    vp: Viewport,
    drawn: bool,
}

pub struct Nebulae {
    cfg: Settings,
    classes: BTreeMap<String, Cls>,
    /// This frame's nebula objects, by class; their stock draw skipped.
    gpu: Gpu,
    dev: Option<Device>,
    sh: Shaders,
    views: PerView<ViewState>,
    ghosts: Vec<i32>,
    bolt_seed: u32,
    /// The time of the last gather (sim seconds), so the action camera's second draw
    /// of the frame does not gather again.
    checked: f64,
    /// The main view's Key (towards it): the bake's signature takes it, so the action
    /// camera's view, whose lights may differ, uses the main view's and does not bake again.
    key_main: Option<[f32; 3]>,
    /// This view's shadow map from the hulls (a texture), or 0.
    sh_tex: usize,
    said: u32,
}

static mut SPIRIT_OLD: usize = 0;
static mut SPIRIT_ACTIVE: bool = false;
/// The ODFs whose gas is drawn now (for the ghost hook, which has no `self`).
static mut GAS_ODFS: Vec<String> = Vec::new();

/// `GameObjectSpirit::Render`, wrapped: a ghost of a nebula whose class is drawn as gas
/// draws nothing (its gas is drawn with the class's).
unsafe extern "thiscall" fn hook_spirit(sp: usize, cam: usize) {
    unsafe {
        if SPIRIT_ACTIVE && sp != 0 {
            let get: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(FN_ENTITY_GET);
            let o = get(rd::<i32>(sp + 8));
            if o != 0 && (engine::vtable(o) == VT_NEBULA || engine::vtable(o) == VT_LATINUM) {
                let odf = rt::lower(obj::odf_name(o));
                #[allow(static_mut_refs)]
                if GAS_ODFS.iter().any(|s| *s == odf) {
                    return;
                }
            }
        }
        if SPIRIT_OLD != 0 {
            let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(SPIRIT_OLD);
            f(sp, cam);
        }
    }
}

impl Nebulae {
    pub fn new() -> Nebulae {
        Nebulae {
            cfg: Settings::read(),
            classes: BTreeMap::new(),
            gpu: Gpu::default(),
            dev: None,
            sh: Shaders {
                vs: Shader::vertex(&VS_NEB),
                ps: Shader::pixel(&PS_NEB),
                bake_vs: Shader::vertex(&VS_NBAKE),
                bake_ps: Shader::pixel(&PS_NBAKE),
                vol_ps: Shader::pixel(&PS_VOL),
                march_vs: Shader::vertex(&VS_MARCH),
                march: [Shader::pixel(&PS_MARCH8), Shader::pixel(&PS_MARCH12), Shader::pixel(&PS_MARCH16), Shader::pixel(&PS_MARCH24)],
                quad_vs: Shader::vertex(&VS_QUAD),
                comp_ps: Shader::pixel(&PS_COMP),
            },
            views: PerView::new(),
            ghosts: Vec::new(),
            bolt_seed: 0x9e37_79b9,
            checked: -1.0,
            key_main: None,
            sh_tex: 0,
            said: 0,
        }
    }

    fn note(&mut self, bit: u32, what: &str) {
        if self.said & (1 << bit) == 0 {
            self.said |= 1 << bit;
            log!("world: nebulae: {}", what);
        }
    }

    /// Wraps the ghosts' draw (once).
    pub fn install(&mut self) {
        unsafe {
            if SPIRIT_OLD == 0 && field(VT_SPIRIT, SLOT_SPIRIT_RENDER * 4) == FN_SPIRIT_RENDER {
                if let Some(old) = hook::patch_slot(VT_SPIRIT, SLOT_SPIRIT_RENDER, hook_spirit as *const () as usize) {
                    SPIRIT_OLD = old;
                    log!("world: nebulae: the ghosts' Render wrapped");
                }
            } else if SPIRIT_OLD == 0 {
                log!("world: nebulae: the ghosts' Render is not stock's: a fogged nebula keeps its billboards");
            }
        }
    }

    pub fn set_active(&mut self, on: bool) {
        unsafe { SPIRIT_ACTIVE = on && self.cfg.enable };
    }

    pub fn reset(&mut self) {
        self.gpu = Gpu::default();
        for c in self.classes.values_mut() {
            c.drop_device();
        }
        self.dev = None;
    }

    /// Settings and recipes again (with `Reload=1`, once a second).
    pub fn reload(&mut self) {
        let old = self.cfg;
        self.cfg = Settings::read();
        if old.vol_max != self.cfg.vol_max || old.vol_maxy != self.cfg.vol_maxy || old.vol_texel != self.cfg.vol_texel || old.noise != self.cfg.noise {
            for c in self.classes.values_mut() {
                c.vol_gen = u32::MAX;
                c.failed = false;
            }
            if old.noise != self.cfg.noise {
                self.gpu.noise = None;
            }
            self.gpu.failed = false;
        }
        for c in self.classes.values_mut() {
            c.reload();
        }
        unsafe { SPIRIT_ACTIVE = SPIRIT_ACTIVE && self.cfg.enable };
    }

    /// The class of a nebula object (made on first sight).
    fn class_of(&mut self, o: usize) -> Option<&mut Cls> {
        let name = rt::lower(obj::odf_name(o));
        if name.is_empty() {
            return None;
        }
        if !self.classes.contains_key(&name) {
            let latinum = engine::vtable(o) == VT_LATINUM;
            let r0 = if latinum {
                100.0
            } else {
                let r = engine::field_f32(obj::class(o), 0x1e0);
                if r > 10.0 && r < 5000.0 { r } else { 300.0 }
            };
            self.classes.insert(name.clone(), Cls::new(&name, latinum, r0));
        }
        self.classes.get_mut(&name)
    }

    /// Whether this nebula's billboards are skipped (its class is drawn as gas); marks it
    /// on screen as `Entity::Render` would.
    ///
    /// Every nebula is marked, whoever draws its billboards (the particles module does
    /// not mark them): `Nebula::Simulate` registers its light only while it is marked.
    pub fn object(&mut self, o: usize, v: &View) -> bool {
        if engine::vtable(o) == VT_NEBULA {
            mark_on_screen(o, v.cam);
        }
        if !self.cfg.enable || self.gpu.failed {
            return false;
        }
        let Some(c) = self.class_of(o) else { return false };
        if !c.have || c.failed {
            return false;
        }
        if engine::vtable(o) != VT_NEBULA {
            mark_on_screen(o, v.cam);
        }
        true
    }

    /// The handles the engine keeps ghosts of this frame.
    fn gather_ghosts(&mut self) {
        self.ghosts.clear();
        let head = field(GHOST_LIST, 4);
        if head == 0 {
            return;
        }
        let mut n = field(head, 0);
        let mut guard = 0;
        while n != 0 && n != head && guard < 100_000 {
            let sp = field(n, 8);
            if sp != 0 {
                self.ghosts.push(unsafe { rd::<i32>(sp + 8) });
            }
            n = field(n, 0);
            guard += 1;
        }
    }

    fn gather_one(&mut self, o: usize) -> bool {
        if o == 0 {
            return false;
        }
        let ghosted = {
            let h = unsafe { rd::<i32>(o + 0x28) };
            self.ghosts.contains(&h)
        };
        let seen = obj::can_user_see(o);
        let Some(c) = self.class_of(o) else { return false };
        if !c.have || c.failed || c.pos.len() >= MAX_NEB {
            return false;
        }
        if !seen && !ghosted {
            return false;
        }
        let p: [f32; 3] = unsafe { rd(o + 0xac) };
        c.pos.push(p);
        for x in p {
            c.sig = (c.sig ^ (x.to_bits() >> 4)).wrapping_mul(16_777_619);
        }
        true
    }

    /// This frame's nebulae, by class, the fog of war applied.
    fn gather(&mut self) -> bool {
        for c in self.classes.values_mut() {
            c.pos.clear();
            c.sig = 2_166_136_261;
        }
        self.gather_ghosts();
        let mut any = false;
        let vecp = unsafe { rd::<usize>(NEBULA_LIST) };
        if vecp != 0 {
            let (mut it, end) = (field(vecp, 4), field(vecp, 8));
            while it != 0 && it < end {
                any |= self.gather_one(field(it, 0));
                it += 4;
            }
        }
        let mut lat = Vec::new();
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if o != 0 && engine::vtable(o) == VT_LATINUM {
                lat.push(o);
            }
        });
        for o in lat {
            any |= self.gather_one(o);
        }
        unsafe {
            #[allow(static_mut_refs)]
            {
                GAS_ODFS.clear();
                for (k, c) in self.classes.iter() {
                    if c.have && !c.failed {
                        GAS_ODFS.push(k.clone());
                    }
                    // The recipe's hue for the nebula's light (lights.rs): the mean of
                    // GasA, GasB and Glow, brightest channel 1.
                    let mut h = [0.0f32; 3];
                    for i in 0..3 {
                        h[i] = (c.k[1][i] + c.k[2][i] + c.k[3][i]) / 3.0;
                    }
                    let m = h[0].max(h[1]).max(h[2]);
                    let hue = if c.have && !c.failed && m > 0.0 { Some(crate::math::v3(h[0] / m, h[1] / m, h[2] / m)) } else { None };
                    super::lights::set_hue(k, hue);
                }
            }
        }
        any
    }

    fn rnd01(&mut self) -> f32 {
        self.bolt_seed = hash32(self.bolt_seed.wrapping_add(0x6d2b_79f5));
        (self.bolt_seed >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// Lightning: flashes scheduled per class (as Nebulae.asi's).
    fn lightning(&mut self, key: &str, now: f32, dt: f32) {
        let r0 = self.rnd01();
        let (r1, r2, r3, r4, r5) = (self.rnd01(), self.rnd01(), self.rnd01(), self.rnd01(), self.rnd01());
        let Some(c) = self.classes.get_mut(key) else { return };
        let r = c.reach();
        for i in 0..4 {
            c.k[16 + i][3] = 0.0;
            if c.bolt_amp[i] > 0.0 {
                let a = now - c.bolt_t0[i];
                if !(0.0..=0.8).contains(&a) {
                    c.bolt_amp[i] = 0.0;
                } else {
                    c.k[16 + i] = [c.bolt_pos[i][0], c.bolt_pos[i][1], c.bolt_pos[i][2], c.bolt_amp[i] * flicker(a)];
                }
            }
        }
        if c.bolt_rate <= 0.0 || c.pos.is_empty() || dt <= 0.0 || dt > 0.5 {
            return;
        }
        if r0 < c.bolt_rate / 60.0 * c.pos.len() as f32 * dt {
            if let Some(i) = (0..4).find(|&i| c.bolt_amp[i] <= 0.0) {
                let p = c.pos[((r1 * c.pos.len() as f32) as usize) % c.pos.len()];
                let (a, d) = (r2 * 6.283_185_3, math::sqrtf(r3) * r * 0.6);
                c.bolt_pos[i] = [p[0] + d * math::cosf(a), p[1] + (r4 * 2.0 - 1.0) * c.k[7][1] * 0.45, p[2] + d * math::sinf(a)];
                c.bolt_t0[i] = now;
                c.bolt_amp[i] = c.bolt_bright * (0.6 + 0.4 * r5);
            }
        }
    }

    /// The noise and the blue noise on this device.
    fn device_ready(&mut self, dev: Device) -> bool {
        if self.dev != Some(dev) {
            self.reset();
            self.dev = Some(dev);
        }
        if self.gpu.failed {
            return false;
        }
        if self.gpu.noise.is_none() {
            self.gpu.noise = make_noise(dev, self.cfg.noise);
        }
        if self.gpu.blue.is_none() {
            let t0 = rt::now();
            let px = blue_noise();
            self.gpu.blue = upload_texture(dev, BN as u32, BN as u32, FMT_A8R8G8B8, &[&px]);
            log!("world: nebula blue noise in {:.0} ms", (rt::now() - t0) * 1000.0);
        }
        if self.gpu.noise.is_none() || self.gpu.blue.is_none() {
            self.gpu.failed = true;
            self.note(0, "the noise could not be made: the stock billboards");
            return false;
        }
        true
    }

    /// Draws every class's gas into the view (at the translucent step).
    pub fn draw(&mut self, v: &View) {
        if !self.cfg.enable {
            return;
        }
        let dev = v.dev;
        if !self.device_ready(dev) {
            return;
        }
        // The nebulae are gathered once a frame (the action camera's view draws the same).
        if v.main || self.checked < 0.0 {
            self.checked = v.time as f64;
            if !self.gather() {
                return;
            }
        } else if self.classes.values().all(|c| c.pos.is_empty()) {
            return;
        }
        let p = v.proj.0;
        if p[2][2] == 0.0 || p[2][3] == 0.0 {
            return;
        }
        let vp = v.view.mul(&v.proj);
        let mut cols = [[0.0f32; 4]; 4];
        for i in 0..4 {
            cols[i] = [vp.0[0][i], vp.0[1][i], vp.0[2][i], vp.0[3][i]];
        }
        let vw = v.view.0;
        let r = [vw[0][0], vw[1][0], vw[2][0]];
        let u = [vw[0][1], vw[1][1], vw[2][1]];
        let f = [vw[0][2], vw[1][2], vw[2][2]];
        let eye = [v.eye.x, v.eye.y, v.eye.z];
        let zn = -p[3][2] / p[2][2];

        // The Key: the brightest of the view's directional lights; towards it.
        if v.main {
            self.key_main = scene::lights().key().map(|k| [-k.dir.x, -k.dir.y, -k.dir.z]);
        }
        let key = self.key_main;
        let _g = gfx::Guard::new(dev);
        // Ships' shadows in the gas: the hulls' shadow map of the Key, this view's
        // (bound by draw_states).
        let shadow = if self.cfg.shadows && key.is_some() { crate::mods::hulls::shadow_map() } else { None };
        self.sh_tex = shadow.as_ref().map_or(0, |sm| sm.tex as usize);
        if shadow.is_some() {
            self.note(4, "ships' shadows in the gas: the hulls' shadow map");
        }
        let mut mr: Option<March> = None;
        let mut began = false;
        let keys: Vec<String> = self.classes.keys().cloned().collect();
        for key_name in keys {
            {
                let c = &self.classes[&key_name];
                if !c.have || c.failed || c.pos.is_empty() {
                    continue;
                }
            }
            if !began {
                began = true;
                if self.cfg.bake {
                    mr = self.march_begin(dev, &p, &r, &u, &f);
                }
                self.draw_states(dev, &cols);
            }
            let share = {
                let c = self.classes.get_mut(&key_name).unwrap();
                c.cur = c.sig;
                let s = if key.is_some() { if c.key_share >= 0.0 { c.key_share } else { self.cfg.key_share } } else { 0.0 };
                if s > 0.0 {
                    let kd = key.unwrap();
                    for i in 0..3 {
                        c.k[10][i] = kd[i] * c.shade_step;
                        c.cur = (c.cur ^ (kd[i].to_bits() >> 8)).wrapping_mul(16_777_619);
                    }
                } else {
                    c.k[10][0] = c.light0[0];
                    c.k[10][1] = c.light0[1];
                    c.k[10][2] = c.light0[2];
                }
                s
            };
            // The envelope, and the bake, when the class's nebulae or recipe changed.
            {
                let c = self.classes.get_mut(&key_name).unwrap();
                if (c.env.is_none() || c.env_sig != c.cur || c.env_gen != c.generation) && !bake_envelope(dev, c) {
                    c.failed = true;
                    continue;
                }
            }
            let half = {
                let c = &self.classes[&key_name];
                c.reach() * c.height * (1.0 + c.k[8][3]) + c.ydev
            };
            let need_bake = {
                let c = &self.classes[&key_name];
                self.cfg.bake && (c.vol.is_none() || c.vol_sig != c.cur || c.vol_gen != c.generation)
            };
            if need_bake {
                let cfg = self.cfg;
                let c = self.classes.get_mut(&key_name).unwrap();
                if let Some(env) = c.env.as_ref() {
                    dev.set_texture(1, env.ptr());
                }
                let ok = bake_volume(dev, c, half, &cfg, &mut self.sh, self.gpu.noise.as_ref());
                if !ok {
                    c.failed = true;
                }
                self.draw_states(dev, &cols);
                if let Some(m) = mr.as_ref() {
                    march_rebind(dev, m);
                }
                if !ok {
                    continue;
                }
            }
            let c = self.classes.get_mut(&key_name).unwrap();
            if mr.is_some() && c.vol.is_none() {
                continue;
            }
            let (mut lo, mut hi) = ([c.ex0, c.ey - half, c.ez0], [c.ex1, c.ey + half, c.ez1]);
            if self.cfg.bake && c.vol.is_some() && c.gbox {
                lo = c.glo;
                hi = c.ghi;
            }
            let (mut dmin, mut dmax) = (1e30f32, -1e30f32);
            for i in 0..8 {
                let x = (if i & 1 != 0 { hi[0] } else { lo[0] }) - eye[0];
                let y = (if i & 2 != 0 { hi[1] } else { lo[1] }) - eye[1];
                let z = (if i & 4 != 0 { hi[2] } else { lo[2] }) - eye[2];
                let d = x * f[0] + y * f[1] + z * f[2];
                dmin = dmin.min(d);
                dmax = dmax.max(d);
            }
            let dnear = zn * 1.05 + 1.0;
            let mut d0 = dmin.max(dnear);
            if dmax <= d0 + 1.0 {
                continue;
            }
            let mut verts: Vec<Vert> = Vec::new();
            let mut faces: Vec<[f32; 3]> = Vec::new();
            if mr.is_some() {
                // The march: the box's faces (box_faces). Within this of the box, the near
                // plane may cut it, and the back faces are drawn.
                let m = 2.0 * dnear * math::sqrtf(1.0 + (1.0 + p[2][0].abs()) * (1.0 + p[2][0].abs()) / (p[0][0] * p[0][0]) + (1.0 + p[2][1].abs()) * (1.0 + p[2][1].abs()) / (p[1][1] * p[1][1]));
                box_faces(&lo, &hi, &eye, m, &mut faces);
            } else {
            // The ladder.
            let (mut n2, mut f2) = (0.0f32, 0.0f32);
            for i in 0..3 {
                let (a, b) = (lo[i] - eye[i], eye[i] - hi[i]);
                let g = a.max(b);
                let w = a.abs().max(b.abs());
                if g > 0.0 {
                    n2 += g * g;
                }
                f2 += w * w;
            }
            let mut snear = math::sqrtf(n2) / math::sqrtf(1.0 + 1.0 / (p[0][0] * p[0][0]) + 1.0 / (p[1][1] * p[1][1]));
            snear = snear.max(dnear);
            let sfar = math::sqrtf(f2);
            d0 = d0.max(snear);
            if dmax <= d0 + 1.0 {
                continue;
            }
            let mut n = self.cfg.slices;
            if (sfar - snear) / self.cfg.min_step < n as f32 {
                n = ((sfar - snear) / self.cfg.min_step) as i32 + 1;
            }
            n = n.max(2);
            let span = log2(sfar / snear);
            let want = n as f32 / span.max(0.05);
            let rg = self.views.get(v.key()).rungs.entry(key_name.clone()).or_insert(0.0);
            if *rg <= 0.0 || *rg > want * 1.3 || *rg < want / 1.3 {
                *rg = want;
            }
            let m = *rg;
            let j0 = (m * log2(d0)) as i32;
            let j1 = (m * log2(dmax)) as i32 + 1;
            for k in j0..=j1 {
                if verts.len() + 12 > 160 * 4 * 3 {
                    break;
                }
                let a = exp2(k as f32 / m);
                let b = exp2((k + 1) as f32 / m);
                let mut mid = (a + b) * 0.5;
                if mid > dmax {
                    break;
                }
                if mid < d0 {
                    if d0 > dnear {
                        continue;
                    }
                    mid = d0;
                }
                let mut poly = [[0.0f32; 3]; 6];
                let np = cut_box(&lo, &hi, &eye, &f, &r, &u, mid, &mut poly);
                for t in 1..np.saturating_sub(1) {
                    for q in [0, t, t + 1] {
                        verts.push(Vert { x: poly[q][0], y: poly[q][1], z: poly[q][2], thick: b - a, k: k as f32 });
                    }
                }
            }
            }
            if verts.is_empty() && faces.is_empty() {
                continue;
            }
            c.k[0][0] = eye[0];
            c.k[0][1] = eye[1];
            c.k[0][2] = eye[2];
            c.place();
            // Motion, on the sim clock.
            let sec = math::fmodf(v.time, 100_000.0);
            c.k[13][0] = math::fmodf(sec * c.flow_speed, 100.0);
            c.fz[3] = math::fmodf(sec * c.fuzz_speed, 100.0);
            c.sw[2] = 6.283_185_3 * math::fmodf(sec / c.swirl_period, 1.0);
            c.sw[3] = 6.283_185_3 * math::fmodf(sec * 1.618_034 / c.swirl_period, 1.0);
            c.ex[1][3] = math::fmodf(sec / c.ring_cycle, 1.0);
            {
                let dt = if c.pulse_last < 0.0 || sec < c.pulse_last || sec - c.pulse_last > 0.5 { 0.0 } else { sec - c.pulse_last };
                let spd = c.pulse_speed * (1.0 + c.pulse_wander * math::sinf(sec * 0.37) * math::cosf(sec * 0.13));
                c.pulse_last = sec;
                c.pulse_phase = math::fmodf(c.pulse_phase + dt * spd * 6.283_185_3, 6.283_185_3);
                c.ex[0][3] = c.pulse_phase;
            }
            {
                // Edge-on dimming (without Opacity= in the gas target).
                let fy = f[1].abs();
                let reach = c.reach();
                let rf = 2.8 * reach;
                let lim = (c.ex1 - c.ex0) + (c.ez1 - c.ez0);
                let path = if fy > 1e-3 { (2.0 * c.k[7][1] / fy).min(lim) } else { lim };
                let mut s = if path > rf { math::powf(rf / path, c.edge_dim) } else { 1.0 };
                if mr.is_some() && c.opacity > 0.0 {
                    s = 1.0;
                }
                c.k[7][3] = s / reach;
            }
            c.k[12][3] = 0.0;
            c.k[15][0] = if self.cfg.bake && c.vol.is_some() { self.cfg.samples as f32 } else { 1.0 };
            c.k[15][1] = if c.k[15][0] > 1.5 { 0.5 } else { 1.0 };
            c.k[15][2] = if c.hue_cycle > 0.0 { c.hue_swing * math::sinf(sec * 6.283_185_3 / c.hue_cycle) } else { 0.0 };
            c.k[15][3] = if c.bolt_rate > 0.0 { 1.0 } else { 0.0 };
            let bs = c.bolt_size * c.reach();
            c.k[20][3] = 1.0 / (bs * bs).max(1e-6);
            let last = c.bolt_last;
            c.bolt_last = sec;
            let dt = if last > 0.0 && v.main { sec - last } else { 0.0 };
            self.lightning(&key_name, sec, dt);
            let c = self.classes.get_mut(&key_name).unwrap();
            let baked = self.cfg.bake && c.vol.is_some();
            if baked {
                for i in 0..3 {
                    c.k[12][i] = 1.0 / c.vsize[i];
                }
                let ok = match mr.as_ref() {
                    Some(m) => self.sh.march_vs.bind(dev) && self.sh.march[m.ps].bind(dev),
                    None => self.sh.vs.bind(dev) && self.sh.vol_ps.bind(dev),
                };
                if !ok {
                    continue;
                }
                dev.set_texture(2, c.vol.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
                dev.set_texture(3, c.occ.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
            } else if !(self.sh.vs.bind(dev) && self.sh.ps.bind(dev)) {
                continue;
            }
            dev.ps_const(0, flat(&c.k));
            dev.ps_const(24, flat(&c.ex));
            dev.ps_const(38, &c.fz);
            dev.ps_const(40, &c.sw);
            {
                // c32..c35: the hulls' shadow map of the Key (as Lighting.asi's export was:
                // twice the hulls' bias, since the gas has no faces to fight it; the
                // penumbra, map depth -> world -> u, v); c36: towards the Key, w the
                // share; c37: scattering.
                let g = if c.scatter >= 0.0 { c.scatter } else { self.cfg.scatter };
                let kd = key.unwrap_or([0.0, 1.0, 0.0]);
                let mut k = [0.0f32; 24];
                if let Some(sm) = shadow.as_ref() {
                    for r in 0..3 {
                        k[r * 4..r * 4 + 4].copy_from_slice(&sm.rows[r]);
                    }
                    let pen = if sm.width > 0.0 { self.cfg.shadow_soft * sm.range / sm.width } else { 0.0 };
                    k[12..16].copy_from_slice(&[1.0, sm.bias * 2.0, pen, sm.texel]);
                }
                k[16..24].copy_from_slice(&[kd[0], kd[1], kd[2], share, 1.0 + g * g, 2.0 * g, 0.0, 0.0]);
                dev.ps_const(32, &k);
            }
            dev.set_texture(1, c.env.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
            if let Some(m) = mr.as_mut() {
                // The box, and the states the march draws with: its sums are added into
                // the gas target, all four channels, with no depth buffer (the march
                // reads the scene's depth itself).
                m.k[4] = [lo[0], lo[1], lo[2], dnear];
                m.k[5] = [hi[0], hi[1], hi[2], 0.0];
                dev.ps_const(21, &m.k[0]);
                dev.ps_const(41, &flat(&m.k)[4..]);
                dev.vs_const(4, &[1.0, 0.0, 0.0, 0.0]);
                for (s, val) in [(RS_ZENABLE, 0), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_COLORWRITEENABLE, 15)] {
                    dev.set_rs(s, val);
                }
                m.drawn = true;
                dev.set_fvf(FVF_XYZ);
                dev.draw_primitive_up(PT_TRIANGLELIST, faces.len() as u32 / 3, faces.as_ptr() as *const u8, 12);
                dev.set_fvf(FVF_SLICE);
                self.note(1, "gas marched");
            } else {
                dev.draw_primitive_up(PT_TRIANGLELIST, verts.len() as u32 / 3, verts.as_ptr() as *const u8, 20);
                self.note(1, "gas drawn");
            }
        }
        if let Some(m) = mr {
            self.march_end(dev, m);
        }
    }

    /// The states every slice draw sets; the guard puts the engine's back.
    fn draw_states(&mut self, dev: Device, cols: &[[f32; 4]; 4]) {
        for (s, val) in [
            (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 0), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 1),
            (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD), (RS_SEPARATEALPHABLENDENABLE, 0),
            (RS_FOGENABLE, 0), (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3), (RS_STENCILENABLE, 0), (152, 0),
            (RS_COLORWRITEENABLE, 7), (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(s, val);
        }
        dev.set_fvf(FVF_SLICE);
        dev.set_stream_freq(0, 1);
        self.sh.vs.bind(dev);
        dev.vs_const(0, flat(cols));
        dev.set_texture(0, self.gpu.noise.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
        dev.set_texture(4, self.gpu.blue.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
        ss(dev, 0, &[(SAMP_ADDRESSU, TADDRESS_WRAP), (SAMP_ADDRESSV, TADDRESS_WRAP), (SAMP_ADDRESSW, TADDRESS_WRAP), (SAMP_MAGFILTER, TEXF_LINEAR), (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_LINEAR), (SAMP_SRGBTEXTURE, 0)]);
        ss(dev, 4, &[(SAMP_ADDRESSU, TADDRESS_WRAP), (SAMP_ADDRESSV, TADDRESS_WRAP), (SAMP_ADDRESSW, TADDRESS_WRAP), (SAMP_MAGFILTER, TEXF_POINT), (SAMP_MINFILTER, TEXF_POINT), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)]);
        ss(dev, 1, &SS_CLAMP_LINEAR);
        ss(dev, 2, &SS_CLAMP_LINEAR);
        ss(dev, 3, &SS_CLAMP_POINT);
        ss(dev, 8, &SS_CLAMP_POINT);
        // The hulls' shadow map, while this view has one (the constants per class).
        dev.set_texture(8, self.sh_tex as *mut core::ffi::c_void);
        dev.ps_const(32, &[0.0; 12]);
    }

    // ---- the march, with a copy of the scene's depth ------------------------------------

    fn z_target(&mut self, dev: Device, w: u32, h: u32) -> bool {
        let g = &mut self.gpu;
        if g.zt.is_some() && g.zt_wh != (w, h) {
            g.zt = None;
            g.fb = None;
        }
        if g.zt.is_none() {
            g.zt = dev.create_texture(w, h, 1, USAGE_DEPTHSTENCIL, FMT_INTZ, POOL_DEFAULT);
            g.fb = dev.create_texture(w, h, 1, USAGE_RENDERTARGET, FMT_A16B16G16R16F, POOL_DEFAULT);
            if g.zt.is_none() || g.fb.is_none() {
                g.zt = None;
                g.fb = None;
                return false;
            }
            g.zt_wh = (w, h);
            log!("world: nebulae: marched, with the scene's depth at {}x{}", w, h);
        }
        true
    }

    /// Copies the scene's depth (RESZ) into the INTZ texture the march reads, binds it,
    /// and works out the march's constants for this view (p the projection; r, u, f the
    /// view's axes in the world). None: no INTZ, and the gas is sliced as the plugin's
    /// was, added to the frame directly.
    fn march_begin(&mut self, dev: Device, p: &[[f32; 4]; 4], r: &[f32; 3], u: &[f32; 3], f: &[f32; 3]) -> Option<March> {
        if self.gpu.zt_broken {
            return None;
        }
        // Timing=1: the translucent step split at the gas (setup, march, filter).
        crate::timing::mark("translucent before gas");
        let rt0 = dev.get_render_target(0)?;
        let ds0 = dev.get_depth_stencil();
        let dd = ds0.as_ref()?.surf_desc();
        if !self.z_target(dev, dd.width, dd.height) {
            self.gpu.zt_broken = true;
            self.note(2, "no INTZ depth texture or float target: the gas sliced and added to the frame directly");
            return None;
        }
        let ps = match self.cfg.steps {
            ..=8 => 0,
            9..=12 => 1,
            13..=16 => 2,
            _ => 3,
        };
        if self.sh.march_vs.get(dev).is_null() || self.sh.march[ps].get(dev).is_null() || self.sh.quad_vs.get(dev).is_null() || self.sh.comp_ps.get(dev).is_null() {
            return None;
        }
        // RESZ: the depth buffer into the INTZ texture bound at stage 0.
        let zt = self.gpu.zt.as_ref()?.ptr();
        dev.set_vs(core::ptr::null_mut());
        dev.set_ps(core::ptr::null_mut());
        dev.set_fvf(FVF_XYZ);
        dev.set_rs(RS_ZENABLE, 0);
        dev.set_rs(RS_ZWRITEENABLE, 0);
        dev.set_rs(RS_COLORWRITEENABLE, 0);
        dev.set_texture(0, zt);
        let pt = [0.0f32; 3];
        dev.draw_primitive_up(PT_POINTLIST, 1, pt.as_ptr() as *const u8, 12);
        dev.set_rs(RS_POINTSIZE, RESZ_CODE);
        dev.set_rs(RS_POINTSIZE, 0x3f80_0000);
        dev.set_texture(0, core::ptr::null_mut());
        dev.set_texture(5, zt);
        ss(dev, 5, &SS_CLAMP_POINT);
        // The ray through a pixel's centre (x, y in the target), scaled to view depth 1:
        // f + r (ndc.x - _31) / _11 + u (ndc.y - _32) / _22, which is linear in x and y.
        let vp = dev.get_viewport();
        let (vx, vy, vw, vh) = (vp.x as f32, vp.y as f32, vp.w.max(1) as f32, vp.h.max(1) as f32);
        // ndc.x = 2 (x - vx) / vw - 1, and ndc.y = 1 - 2 (y - vy) / vh: y runs down.
        let (ax, by) = (2.0 / vw / p[0][0], -2.0 / vh / p[1][1]);
        let (cx, cy) = ((-2.0 * vx / vw - 1.0 - p[2][0]) / p[0][0], (2.0 * vy / vh + 1.0 - p[2][1]) / p[1][1]);
        let (w, h) = self.gpu.zt_wh;
        let fs = self.gpu.fb.as_ref()?.tex_surface(0)?;
        // The gas target, cleared to no gas and no optical depth (in the viewport).
        dev.set_render_target(0, fs.ptr());
        dev.set_depth_stencil(core::ptr::null_mut());
        dev.set_viewport(&vp);
        dev.set_rs(RS_SCISSORTESTENABLE, 0);
        dev.set_rs(RS_COLORWRITEENABLE, 15);
        dev.clear(&[], CLEAR_TARGET, 0, 1.0, 0);
        crate::timing::mark("gas: depth copy, clear");
        Some(March {
            k: [
                [self.cfg.obscure, 0.0, 1.0 / w as f32, 1.0 / h as f32],
                [r[0] * ax, r[1] * ax, r[2] * ax, self.cfg.footprint],
                [u[0] * by, u[1] * by, u[2] * by, 0.0],
                [f[0] + r[0] * cx + u[0] * cy, f[1] + r[1] * cx + u[1] * cy, f[2] + r[2] * cx + u[2] * cy, 0.0],
                [0.0; 4],
                [0.0; 4],
                [p[2][2], p[3][2], 0.0, 0.0],
            ],
            ps,
            rt0,
            ds0,
            fs,
            vp,
            drawn: false,
        })
    }

    /// The gas onto the frame, through the filter (comp_ps); the frame's target and depth
    /// back.
    fn march_end(&mut self, dev: Device, m: March) {
        crate::timing::mark("gas: march");
        dev.set_render_target(0, m.rt0.ptr());
        dev.set_depth_stencil(m.ds0.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
        dev.set_viewport(&m.vp);
        if m.drawn {
            for (s, val) in [(RS_ZENABLE, 0), (RS_ALPHABLENDENABLE, 1), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_INVSRCALPHA), (RS_COLORWRITEENABLE, 7), (RS_SCISSORTESTENABLE, 0)] {
                dev.set_rs(s, val);
            }
            dev.set_fvf(FVF_SLICE);
            self.sh.quad_vs.bind(dev);
            self.sh.comp_ps.bind(dev);
            let sg = self.cfg.denoise;
            let (x0, y0) = (m.vp.x as f32 + 0.5, m.vp.y as f32 + 0.5);
            let k = [
                if sg > 0.05 { 0.5 / (sg * sg) } else { 100.0 }, 0.05, 0.0, 0.0,
                x0, y0, x0 + m.vp.w.max(1) as f32 - 1.0, y0 + m.vp.h.max(1) as f32 - 1.0,
            ];
            dev.ps_const(21, &m.k[0]);
            dev.ps_const(46, &m.k[6]);
            dev.ps_const(47, &k);
            ss(dev, 7, &SS_CLAMP_POINT);
            dev.set_texture(7, self.gpu.fb.as_ref().map_or(core::ptr::null_mut(), |x| x.ptr()));
            quad(dev);
            dev.set_texture(7, core::ptr::null_mut());
        }
        dev.set_texture(5, core::ptr::null_mut());
        crate::timing::mark("gas: filter, onto the frame");
    }
}

/// After a bake or `draw_states`: the gas target, no depth buffer, the view's viewport.
fn march_rebind(dev: Device, m: &March) {
    dev.set_render_target(0, m.fs.ptr());
    dev.set_depth_stencil(core::ptr::null_mut());
    dev.set_viewport(&m.vp);
}

/// The faces of the box lo..hi the march draws, each pixel of its outline once: those
/// facing the eye, or, with the eye within `m` of the box (where the near plane may cut
/// its front faces), those facing away, which cover its outline from inside it too.
/// Chosen here rather than by culling, so the engine's winding does not matter. No depth
/// test: the march reads the scene's depth itself. True: the back faces.
fn box_faces(lo: &[f32; 3], hi: &[f32; 3], eye: &[f32; 3], m: f32, out: &mut Vec<[f32; 3]>) -> bool {
    let inside = (0..3).all(|i| eye[i] > lo[i] - m && eye[i] < hi[i] + m);
    for a in 0..3 {
        let (a1, a2) = ((a + 1) % 3, (a + 2) % 3);
        for side in 0..2 {
            let facing = if side == 0 { eye[a] < lo[a] } else { eye[a] > hi[a] };
            if facing == inside {
                continue;
            }
            let mut c = [[0.0f32; 3]; 4];
            for (k, (s1, s2)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
                c[k][a] = if side == 0 { lo[a] } else { hi[a] };
                c[k][a1] = if s1 == 0 { lo[a1] } else { hi[a1] };
                c[k][a2] = if s2 == 0 { lo[a2] } else { hi[a2] };
            }
            out.extend_from_slice(&[c[0], c[1], c[2], c[0], c[2], c[3]]);
        }
    }
    inside
}

fn flat<const N: usize>(a: &[[f32; 4]; N]) -> &[f32] {
    unsafe { core::slice::from_raw_parts(a.as_ptr() as *const f32, N * 4) }
}

fn log2(x: f32) -> f32 {
    math::logf(x) * core::f32::consts::LOG2_E
}
fn exp2(x: f32) -> f32 {
    math::expf(x * core::f32::consts::LN_2)
}

/// A flash's brightness t seconds after it starts: three flickers, 65 ms half-lives.
fn flicker(t: f32) -> f32 {
    let at = [0.0, 0.12, 0.3];
    let amp = [1.0, 0.6, 0.85];
    let mut s = 0.0;
    for i in 0..3 {
        let u = t - at[i];
        if (0.0..0.5).contains(&u) {
            s += amp[i] * exp2(-u * 15.0);
        }
    }
    s
}

/// Marks a nebula on screen as `GameObject::Render` would (not hidden, seen by the
/// player, its instance in the frustum: slot 3 of the instance's vtable), for the engine's
/// nebula lights.
fn mark_on_screen(o: usize, cam: usize) {
    let inst = obj::instance(o);
    if field_u8(o, 0x27) != 0 || inst == 0 || cam == 0 || !obj::can_user_see(o) {
        return;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(engine::vtable(inst), 3 * 4));
        if f(inst, cam) != 0 {
            core::ptr::write((o + 0x25) as *mut u8, 1);
        }
    }
}

/// Where a class's gas may be, from above (Nebulae.asi's bake_envelope): alpha each nebula
/// a soft disc out to its reach, red the domes' height, green the filaments' (or the
/// fringe's) wider disc, blue the nebulae's height there.
fn bake_envelope(dev: Device, c: &mut Cls) -> bool {
    let reach = c.reach();
    let mut freach = reach * if c.k[14][0] > 0.0 && c.fil_reach > 1.0 { c.fil_reach } else { 1.0 };
    if c.fz[0] > 0.0 && reach * c.fringe_reach > freach {
        freach = reach * c.fringe_reach;
    }
    let outer = freach + reach * c.edge_warp;
    c.ex0 = 1e30;
    c.ez0 = 1e30;
    c.ex1 = -1e30;
    c.ez1 = -1e30;
    let mut ys = 0.0;
    for p in &c.pos {
        c.ex0 = c.ex0.min(p[0] - outer);
        c.ex1 = c.ex1.max(p[0] + outer);
        c.ez0 = c.ez0.min(p[2] - outer);
        c.ez1 = c.ez1.max(p[2] + outer);
        ys += p[1];
    }
    c.ey = ys / c.pos.len() as f32;
    c.ydev = 1.0;
    for p in &c.pos {
        c.ydev = c.ydev.max((p[1] - c.ey).abs());
    }
    let cell = c.radius / 16.0;
    let w = (((c.ex1 - c.ex0) / cell) as u32 + 2).clamp(4, ENV_MAX);
    let h = (((c.ez1 - c.ez0) / cell) as u32 + 2).clamp(4, ENV_MAX);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for z in 0..h {
        let wz = c.ez0 + (z as f32 + 0.5) * (c.ez1 - c.ez0) / h as f32;
        for x in 0..w {
            let wx = c.ex0 + (x as f32 + 0.5) * (c.ex1 - c.ex0) / w as f32;
            let (mut s, mut hh, mut g, mut wy, mut ww) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for p in &c.pos {
                let (dx, dz) = (wx - p[0], wz - p[2]);
                let d2 = dx * dx + dz * dz;
                if d2 >= freach * freach {
                    continue;
                }
                let d = math::sqrtf(d2);
                g += smooth(1.0, 0.3, d / freach);
                let q = smooth(1.0, 0.0, d / freach) + 1e-4;
                wy += q * (p[1] - c.ey);
                ww += q;
                if d >= reach {
                    continue;
                }
                s += smooth(1.0, 0.6, d / reach);
                let q = d / reach;
                hh = hh.max(1.0 - q * q * q);
            }
            let a = (s.min(1.0) * 255.0 + 0.5) as i32;
            let r = (hh * 255.0 + 0.5) as i32;
            let gg = (g.min(1.0) * 255.0 + 0.5) as i32;
            let bb = ((0.5 + 0.5 * if ww > 0.0 { wy / ww } else { 0.0 } / c.ydev) * 255.0 + 0.5) as i32;
            let o = ((z * w + x) * 4) as usize;
            px[o] = bb.clamp(0, 255) as u8;
            px[o + 1] = gg.clamp(0, 255) as u8;
            px[o + 2] = r.clamp(0, 255) as u8;
            px[o + 3] = a.clamp(0, 255) as u8;
        }
    }
    c.env = upload_texture(dev, w, h, FMT_A8R8G8B8, &[&px]);
    if c.env.is_none() {
        log!("world: nebulae: envelope {}: no texture", c.odf);
        return false;
    }
    c.env_sig = c.cur;
    c.env_gen = c.generation;
    log!(
        "world: nebulae: envelope {}: {} nebulae, x {:.0}..{:.0}, z {:.0}..{:.0}, y {:.0} +-{:.0}, {}x{}",
        c.odf, c.pos.len(), c.ex0, c.ex1, c.ez0, c.ez1, c.ey, c.ydev, w, h
    );
    true
}

/// Bakes a class's gas into its volume: each layer drawn by `bake_ps` into a render
/// target, read back and gathered into a system-memory volume, then copied to the
/// device; the per-column record of where gas is (`occ`) beside it.
fn bake_volume(dev: Device, c: &mut Cls, half: f32, cfg: &Settings, sh: &mut Shaders, noise: Option<&Com>) -> bool {
    let t0 = rt::now();
    c.vlo = [c.ex0, c.ey - half, c.ez0];
    let size = [c.ex1 - c.ex0, 2.0 * half, c.ez1 - c.ez0];
    let along = size[0].max(size[2]);
    let texel = (along / cfg.vol_max as f32).max(cfg.vol_texel);
    let x_n = ((size[0] / texel) as u32 + 1).clamp(8, cfg.vol_max);
    let z_n = ((size[2] / texel) as u32 + 1).clamp(8, cfg.vol_max);
    let y_n = ((size[1] / texel) as u32 + 1).clamp(8, cfg.vol_maxy);
    c.vsize = size;
    let (xn, yn, zn) = (x_n as usize, y_n as usize, z_n as usize);
    let mut occ_max = vec![0u8; xn * zn];
    let mut occ_lo = vec![255u8; xn * zn];
    let mut occ_hi = vec![0u8; xn * zn];
    if !(sh.bake_vs.bind(dev) && sh.bake_ps.bind(dev)) {
        return false;
    }
    let Some(rt) = dev.create_texture(x_n, z_n, 1, USAGE_RENDERTARGET, FMT_A8R8G8B8, POOL_DEFAULT) else { return false };
    let Some(rts) = rt.tex_surface(0) else { return false };
    let Some(sys) = create_offscreen(dev, x_n, z_n, FMT_A8R8G8B8, POOL_SYSTEMMEM) else { return false };
    let Some(stage) = dev.create_volume_texture(x_n, z_n, y_n, 1, 0, FMT_A8R8G8B8, POOL_SYSTEMMEM) else { return false };
    let old_vp = dev.get_viewport();
    let Some(old_rt) = dev.get_render_target(0) else { return false };
    let old_ds = dev.get_depth_stencil();
    dev.set_render_target(0, rts.ptr());
    dev.set_depth_stencil(core::ptr::null_mut());
    dev.set_rs(RS_ZENABLE, 0);
    dev.set_rs(RS_ALPHABLENDENABLE, 0);
    dev.set_rs(RS_COLORWRITEENABLE, 15);
    dev.set_rs(RS_SCISSORTESTENABLE, 0);
    dev.set_fvf(FVF_SLICE);
    dev.set_texture(0, noise.map_or(core::ptr::null_mut(), |n| n.ptr()));
    c.place();
    let mut ok = true;
    let Some(dst) = vol_lock(&stage, 0) else {
        dev.set_render_target(0, old_rt.ptr());
        dev.set_depth_stencil(old_ds.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
        dev.set_viewport(&old_vp);
        return false;
    };
    for j in 0..yn {
        c.k[12] = [size[0] / x_n as f32, size[1] / y_n as f32, size[2] / z_n as f32, c.vlo[1] + (j as f32 + 0.5) * size[1] / y_n as f32];
        dev.ps_const(0, flat(&c.k));
        dev.ps_const(24, flat(&c.ex));
        dev.ps_const(38, &c.fz);
        quad(dev);
        if !rt_data(dev, &rts, &sys) {
            ok = false;
            break;
        }
        let Some(src) = surf_lock(&sys, LOCK_READONLY) else {
            ok = false;
            break;
        };
        let lay = ((j * 255 + (yn - 1) / 2) / (yn - 1).max(1)) as u8;
        for r in 0..zn {
            let srow = unsafe { core::slice::from_raw_parts(src.bits.add(r * src.pitch as usize) as *const u32, xn) };
            unsafe {
                core::ptr::copy_nonoverlapping(srow.as_ptr() as *const u8, dst.bits.add(j * dst.slice as usize + r * dst.row as usize), xn * 4);
            }
            for x in 0..xn {
                let d = (srow[x] >> 16) as u8;
                if d > 1 {
                    let i = r * xn + x;
                    occ_max[i] = occ_max[i].max(d);
                    occ_lo[i] = occ_lo[i].min(lay);
                    occ_hi[i] = occ_hi[i].max(lay);
                }
            }
        }
        surf_unlock(&sys);
    }
    vol_unlock(&stage, 0);
    dev.set_render_target(0, old_rt.ptr());
    dev.set_depth_stencil(old_ds.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
    dev.set_viewport(&old_vp);
    if ok {
        let v = dev.create_volume_texture(x_n, z_n, y_n, 1, 0, FMT_A8R8G8B8, POOL_DEFAULT);
        ok = match v {
            Some(v) if dev.update_texture(&stage, &v) >= 0 => {
                c.vol = Some(v);
                true
            }
            _ => false,
        };
    }
    if ok {
        // The occupancy: widened by the flow and the swirl, then uploaded.
        let rr = (((c.k[13][1] + c.sw[0]) / (size[0] / x_n as f32)) as i32 + 2).min(16);
        occ_dilate(&mut occ_max, &mut occ_lo, &mut occ_hi, xn, zn, rr);
        let (mut x0, mut x1, mut z0, mut z1, mut y0, mut y1) = (xn as i32, -1i32, zn as i32, -1i32, 255i32, 0i32);
        for r in 0..zn {
            for x in 0..xn {
                let i = r * xn + x;
                if occ_max[i] < 2 {
                    continue;
                }
                x0 = x0.min(x as i32);
                x1 = x1.max(x as i32);
                z0 = z0.min(r as i32);
                z1 = z1.max(r as i32);
                y0 = y0.min(occ_lo[i] as i32);
                y1 = y1.max(occ_hi[i] as i32);
            }
        }
        c.gbox = x1 >= x0;
        if c.gbox {
            let m = c.k[13][1] + c.sw[0] + size[1] / y_n as f32;
            c.glo = [c.vlo[0] + x0 as f32 * size[0] / x_n as f32, c.vlo[1] + y0 as f32 / 255.0 * size[1] - m, c.vlo[2] + z0 as f32 * size[2] / z_n as f32];
            c.ghi = [c.vlo[0] + (x1 + 1) as f32 * size[0] / x_n as f32, c.vlo[1] + y1 as f32 / 255.0 * size[1] + m, c.vlo[2] + (z1 + 1) as f32 * size[2] / z_n as f32];
        }
        let mut px = vec![0u8; xn * zn * 4];
        for i in 0..xn * zn {
            px[i * 4] = occ_lo[i];
            px[i * 4 + 1] = occ_hi[i];
            px[i * 4 + 2] = occ_max[i];
            px[i * 4 + 3] = 0;
        }
        c.occ = upload_texture(dev, x_n, z_n, FMT_A8R8G8B8, &[&px]);
        ok = c.occ.is_some();
    }
    c.vx = x_n;
    c.vy = y_n;
    c.vz = z_n;
    if ok {
        c.vol_sig = c.cur;
        c.vol_gen = c.generation;
    } else {
        c.vol = None;
        c.occ = None;
    }
    log!(
        "world: nebulae: bake {} {}: {}x{}x{} texels of {:.1} units, in {:.0} ms",
        c.odf, if ok { "done" } else { "FAILED" }, x_n, y_n, z_n, size[0] / x_n as f32, (rt::now() - t0) * 1000.0
    );
    ok
}

/// Widens every column's record to its neighbours within `r` texels, x then z.
fn occ_dilate(m: &mut [u8], lo: &mut [u8], hi: &mut [u8], xn: usize, zn: usize, r: i32) {
    for pass in 0..2 {
        let (sm, sl, sh) = (m.to_vec(), lo.to_vec(), hi.to_vec());
        for z in 0..zn {
            for x in 0..xn {
                let (mut a, mut l, mut h) = (0u8, 255u8, 0u8);
                for k in -r..=r {
                    let (xx, zz) = if pass == 0 { (x as i32 + k, z as i32) } else { (x as i32, z as i32 + k) };
                    if xx < 0 || xx >= xn as i32 || zz < 0 || zz >= zn as i32 {
                        continue;
                    }
                    let i = zz as usize * xn + xx as usize;
                    a = a.max(sm[i]);
                    l = l.min(sl[i]);
                    h = h.max(sh[i]);
                }
                let o = z * xn + x;
                m[o] = a;
                lo[o] = l;
                hi[o] = h;
            }
        }
    }
}

