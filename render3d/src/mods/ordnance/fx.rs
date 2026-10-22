//! Specials.asi, ported: the warp flash and the special weapons drawn by shaders from
//! recipes (`render3d/ordnance.ini`, overridable in Render3D.ini; the looks
//! the user settled in Specials.asi are kept, but for the tractor, holding and
//! assimilation beams, cones since 0.4.0: `Fx::cone`). Every effect is built here as
//! world-space quads for `fx_ps` in `shaders/ordnance.hlsl`, drawn additively, depth
//! tested against the hulls, writing no depth.
//!
//! Differences from the plugin: the clock is the simulation's (`View::time`), so a
//! paused game holds every effect; the quads are drawn in the ordnance and warp steps
//! (and a wave where the engine submits it) instead of after `RenderChunks`; the
//! lights go to the engine's registry at the start of the next view (`lights.rs`).
use super::eng;
use super::ini::Settings;
use gpu::{attr, AttrFormat, Blend, Block, Semantic, Topology, VertexAttr};
use crate::engine::{field, field_f32};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{clamp, expf, powf, sinf, sqrtf, v3, V3};
use crate::shaders::ordnance::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Beam,
    Bolt,
    Field,
    Wave,
    /// A cone of light through vapour (the tractor, holding and assimilation beams).
    Cone,
}

const K_FLARE: f32 = 0.0;
const K_RING: f32 = 1.0;
const K_BEAM: f32 = 2.0;
const K_BOLT: f32 = 3.0;
const K_FIELD: f32 = 4.0;
const K_WAVE: f32 = 5.0;
const K_CONE: f32 = 6.0;

/// One recipe: `[Ordnance.<odf>]` or `[Wave.<odf>]`.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub name: String,
    pub kind: Kind,
    col: [f32; 3],
    core: [f32; 3],
    bright: f32,
    flicker: f32,
    flicker_rate: f32,
    width: f32,
    min_px: f32,
    core_w: f32,
    flow: [f32; 3],
    helix: [f32; 3],
    crackle: f32,
    emitter: f32,
    impact: f32,
    star: f32,
    taper: f32,
    size: f32,
    tail: f32,
    rays: f32,
    spin: f32,
    pulse: f32,
    pulse_rate: f32,
    on_owner: bool,
    radius: f32,
    rim: f32,
    swirl: [f32; 3],
    fill: f32,
    band: f32,
    ray_count: f32,
    ray_share: f32,
    turb: f32,
    light: f32,
    light_col: [f32; 3],
    light_reach: f32,
    fade_in: f32,
    /// A beam's impact flare and light where it enters its target's bounding sphere
    /// (times this; 0: at the beam's end, as for a hull's hardpoint).
    surface: f32,
    /// A cone: its wide end's half width in its target's bounding radii; how far past
    /// the beam's end it runs as it fades, and how far towards the camera its wide end
    /// is drawn (so it lies over the hull), both in those radii.
    spread: f32,
    beyond: f32,
    pull: f32,
    /// A cone's fans: how many (up to 4), the light between their edges (0..1), how
    /// often each comes and goes (a second); their edges' width in pixels.
    fans: [f32; 3],
    /// A second colour of a cone's fans (`Sprinkle`), at this share of its brightness
    /// (`SprinkleShare`, 0 for none): the auto-assimilator's green in its blue.
    sprinkle: [f32; 3],
    sprinkle_share: f32,
    edge_px: f32,
    /// A cone's vapour: depth, blobs per wide-end half width, speed along the beam
    /// (half widths a second); its brightness at the target against the emitter's.
    vapour: [f32; 3],
    fall: f32,
    /// A cone's wash on its target: the light sits this many target radii back from
    /// its centre towards the emitter, full out to `wash_reach` radii.
    wash_back: f32,
    wash_reach: f32,
}

impl Recipe {
    fn load(s: &Settings, sec: &str, name: &str, wave: bool) -> Recipe {
        let kind = if wave {
            Kind::Wave
        } else {
            let k = crate::rt::lower(&s.s(sec, "Kind", "beam"));
            if k.starts_with("bo") {
                Kind::Bolt
            } else if k.starts_with('f') {
                Kind::Field
            } else if k.starts_with('c') {
                Kind::Cone
            } else {
                Kind::Beam
            }
        };
        let col = s.rgb(sec, "Colour", [120.0, 200.0, 255.0]);
        let on = crate::rt::lower(&s.s(sec, "On", "target"));
        Recipe {
            name: String::from(name),
            kind,
            col,
            core: s.rgb(sec, "CoreColour", [235.0, 245.0, 255.0]),
            bright: s.f(sec, "Brightness", 1.0),
            flicker: clamp(s.f(sec, "Flicker", 0.0), 0.0, 1.0),
            flicker_rate: s.f(sec, "FlickerRate", 20.0),
            width: s.f(sec, "Width", 6.0).max(0.01),
            min_px: s.f(sec, "MinPixels", 3.0),
            core_w: s.f(sec, "CoreWidth", 0.25).max(0.01),
            flow: s.n(sec, "Flow", [0.0, 0.15, 8.0]),
            helix: s.n(sec, "Helix", [0.0, 0.12, 6.0]),
            crackle: s.f(sec, "Crackle", 0.0),
            emitter: s.f(sec, "Emitter", 2.5),
            impact: s.f(sec, "Impact", 4.0),
            star: s.f(sec, "Star", 0.4),
            taper: s.f(sec, "Taper", 1.0),
            size: s.f(sec, "Size", 5.0).max(0.01),
            tail: s.f(sec, "Tail", 4.0),
            rays: s.f(sec, "Rays", 0.3),
            spin: s.f(sec, "Spin", 3.0),
            pulse: s.f(sec, "Pulse", 0.0),
            pulse_rate: s.f(sec, "PulseRate", 8.0),
            on_owner: on.starts_with('o'),
            radius: s.f(sec, "Radius", 1.3),
            rim: s.f(sec, "Rim", 3.0),
            swirl: s.n(sec, "Swirl", [0.4, 5.0, 1.5]),
            fill: s.f(sec, "Fill", 0.05),
            band: s.f(sec, "Band", 0.06),
            ray_count: s.f(sec, "RayCount", 16.0),
            ray_share: s.f(sec, "RayShare", 0.5),
            turb: clamp(s.f(sec, "Turbulence", 0.4), 0.0, 1.0),
            light: s.f(sec, "Light", 1.5),
            light_col: s.rgb(sec, "LightColour", [col[0] * 255.0, col[1] * 255.0, col[2] * 255.0]),
            light_reach: s.f(sec, "LightReach", 80.0),
            fade_in: s.f(sec, "FadeIn", 0.15).max(0.001),
            surface: s.f(sec, "ImpactAtSurface", 0.0),
            spread: s.f(sec, "Spread", 0.8).max(0.01),
            beyond: s.f(sec, "Beyond", 0.5).max(0.0),
            pull: s.f(sec, "Pull", 1.0).max(0.0),
            fans: s.n(sec, "Fans", [3.0, 0.3, 0.7]),
            sprinkle: s.rgb(sec, "Sprinkle", [60.0, 255.0, 110.0]),
            sprinkle_share: s.f(sec, "SprinkleShare", 0.0).max(0.0),
            edge_px: s.f(sec, "EdgePixels", 2.0).max(0.1),
            vapour: s.n(sec, "Vapour", [0.5, 1.2, 0.5]),
            fall: s.f(sec, "Fall", 0.25).max(0.0),
            wash_back: s.f(sec, "WashBack", 1.2),
            wash_reach: s.f(sec, "WashReach", 1.4).max(0.05),
        }
    }
}

/// `[Warp]`: the warp flash.
struct Warp {
    on: bool,
    bright: f32,
    size: f32,
    min_px: f32,
    core: f32,
    glow: f32,
    star: f32,
    star_sharp: f32,
    streak: f32,
    streak_len: f32,
    streak_sharp: f32,
    col: [f32; 3],
    streak_col: [f32; 3],
    ring_col: [f32; 3],
    light_col: [f32; 3],
    rise: f32,
    burst: f32,
    after: f32,
    ring: f32,
    ring_speed: f32,
    ring_width: f32,
    ring_time: f32,
    light: f32,
    light_reach: f32,
}

impl Warp {
    fn load(s: &Settings) -> Warp {
        let w = "Warp";
        Warp {
            on: s.i(w, "Enable", 1) != 0,
            bright: s.f(w, "Brightness", 1.3),
            size: s.f(w, "Size", 1.0),
            min_px: s.f(w, "MinPixels", 40.0),
            core: s.f(w, "Core", 40.0),
            glow: s.f(w, "Glow", 4.0),
            star: s.f(w, "Star", 0.8),
            star_sharp: s.f(w, "StarSharpness", 18.0),
            streak: s.f(w, "Streak", 0.9),
            streak_len: s.f(w, "StreakLength", 4.0).max(1.0),
            streak_sharp: s.f(w, "StreakSharpness", 26.0),
            col: s.rgb(w, "Colour", [130.0, 175.0, 255.0]),
            streak_col: s.rgb(w, "StreakColour", [110.0, 160.0, 255.0]),
            ring_col: s.rgb(w, "RingColour", [150.0, 195.0, 255.0]),
            light_col: s.rgb(w, "LightColour", [160.0, 195.0, 255.0]),
            rise: s.f(w, "Rise", 2.0).max(0.1),
            burst: s.f(w, "Burst", 1.8),
            after: s.f(w, "Afterglow", 0.45).max(0.02),
            ring: s.f(w, "Ring", 1.0),
            ring_speed: s.f(w, "RingSpeed", 6.0),
            ring_width: s.f(w, "RingWidth", 0.06),
            ring_time: s.f(w, "RingTime", 0.6).max(0.05),
            light: s.f(w, "Light", 3.0),
            light_reach: s.f(w, "LightReach", 3.0),
        }
    }
}

/// One craft going to warp, by its handle: what `WarpSpriteEffect::Render` showed last,
/// and when the craft reached the point. It outlives the engine's effect by the
/// afterglow.
struct Flash {
    handle: i32,
    pos: V3,
    r: f32,
    k: f32,
    seen: f32,
    /// When the craft got there (sim time); None: not yet.
    passed: Option<f32>,
    _seed: u32,
    /// Noted in this view's warp step.
    noted: bool,
}

/// A shot's age and flight, by its address and recipe.
struct Age {
    ord: usize,
    rc: usize,
    first: f32,
    frame: u32,
    last: Option<V3>,
    dir: V3,
}

/// A light to register at the start of the next view.
#[derive(Clone, Copy)]
pub struct Lit {
    pub pos: V3,
    pub col: V3,
    pub start: f32,
    pub range: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FxVert {
    p: [f32; 3],
    t0: [f32; 4],
    t1: [f32; 4],
    t2: [f32; 4],
    t3: [f32; 4],
    t4: [f32; 4],
}

static FXLAYOUT: [VertexAttr; 6] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float4, Semantic::TexCoord, 0),
    attr(28, AttrFormat::Float4, Semantic::TexCoord, 1),
    attr(44, AttrFormat::Float4, Semantic::TexCoord, 2),
    attr(60, AttrFormat::Float4, Semantic::TexCoord, 3),
    attr(76, AttrFormat::Float4, Semantic::TexCoord, 4),
];
const FX_STRIDE: u32 = 92;

/// What a quad draws, the same at its four corners.
#[derive(Clone, Copy)]
struct Look {
    kind: f32,
    len: f32,
    col: V3,
    bright: f32,
    col2: V3,
    seed: f32,
    p3: [f32; 4],
    p4: [f32; 4],
}

fn rgb(c: [f32; 3]) -> V3 {
    v3(c[0], c[1], c[2])
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}
fn rnd(seed: u32) -> f32 {
    (hash32(seed) & 0xffffff) as f32 / 16777216.0
}

/// The camera, for building quads.
struct Cam {
    eye: V3,
    right: V3,
    up: V3,
    /// A pixel's size at unit distance.
    px: f32,
}

pub struct Fx {
    pub specials: bool,
    pub waves: bool,
    warp: Warp,
    pub recipes: Vec<Recipe>,
    by_name: BTreeMap<(String, bool), usize>,
    flashes: Vec<Flash>,
    flashes_made: u32,
    ages: Vec<Age>,
    frame: u32,
    verts: Vec<FxVert>,
    /// The lights of this frame's effects, for the next view's start.
    pub lights: Vec<Lit>,
    lights_next: Vec<Lit>,
    /// The view being drawn is the main one (only its effects make lights).
    main: bool,
    pipes: Pipes,
    said: Vec<usize>,
}

impl Fx {
    pub fn new(s: &Settings) -> Fx {
        let mut recipes = Vec::new();
        let mut by_name = BTreeMap::new();
        for sec in s.sections() {
            let (wave, name) = if let Some(n) = sec.strip_prefix("Ordnance.") {
                (false, n)
            } else if let Some(n) = sec.strip_prefix("Wave.") {
                (true, n)
            } else {
                continue;
            };
            let key = (crate::rt::lower(name), wave);
            if by_name.contains_key(&key) {
                continue;
            }
            by_name.insert(key, recipes.len());
            recipes.push(Recipe::load(s, &sec, name, wave));
        }
        crate::log!("ordnance: {} recipes for special weapons and their waves", recipes.len());
        Fx {
            specials: s.i("Ordnance", "Specials", 1) != 0,
            waves: s.i("Ordnance", "Waves", 1) != 0,
            warp: Warp::load(s),
            recipes,
            by_name,
            flashes: Vec::new(),
            flashes_made: 0,
            ages: Vec::new(),
            frame: 0,
            verts: Vec::new(),
            lights: Vec::new(),
            lights_next: Vec::new(),
            main: true,
            pipes: Pipes::new(),
            said: Vec::new(),
        }
    }

    pub fn reset(&mut self) {

    }

    pub fn warp_on(&self) -> bool {
        self.warp.on
    }

    /// The recipe for an ODF (`tracbm.odf` or `tracbm`), a wave's or a shot's.
    pub fn recipe_of(&self, odf: &str, wave: bool) -> Option<usize> {
        if odf.is_empty() {
            return None;
        }
        self.by_name.get(&(eng::stem(odf), wave)).copied()
    }

    // ---- quads ---------------------------------------------------------------------

    fn quad(&mut self, p: [V3; 4], lk: &Look, ua: f32, va: f32, ub: f32, vb: f32) {
        if lk.bright <= 0.0 || self.verts.len() >= 6 * 8192 {
            return;
        }
        let uv = [[ua, va], [ub, va], [ub, vb], [ua, vb]];
        for o in [0usize, 1, 2, 0, 2, 3] {
            self.verts.push(FxVert {
                p: [p[o].x, p[o].y, p[o].z],
                t0: [uv[o][0], uv[o][1], lk.kind, lk.len],
                t1: [lk.col.x, lk.col.y, lk.col.z, lk.bright],
                t2: [lk.col2.x, lk.col2.y, lk.col2.z, lk.seed],
                t3: lk.p3,
                t4: lk.p4,
            });
        }
    }

    /// A quad whose vertices carry their own u, v times w, and w (`t2.y`): `cone`'s
    /// projective v.
    fn quad_w(&mut self, p: [V3; 4], lk: &Look, uvw: [[f32; 3]; 4]) {
        if lk.bright <= 0.0 || self.verts.len() >= 6 * 8192 {
            return;
        }
        for o in [0usize, 1, 2, 0, 2, 3] {
            self.verts.push(FxVert {
                p: [p[o].x, p[o].y, p[o].z],
                t0: [uvw[o][0], uvw[o][1], lk.kind, lk.len],
                t1: [lk.col.x, lk.col.y, lk.col.z, lk.bright],
                t2: [lk.col2.x, uvw[o][2], lk.col2.z, lk.seed],
                t3: lk.p3,
                t4: lk.p4,
            });
        }
    }

    /// A camera-facing rectangle about c, half size hx by hy (uv -ax..ax, -1..1),
    /// pulled towards the camera by `pull`.
    fn billboard(&mut self, cam: &Cam, c: V3, hx: f32, hy: f32, ax: f32, pull: f32, lk: &Look) {
        let to = (cam.eye - c).norm();
        let m = c + to * pull;
        let (r, u) = (cam.right * hx, cam.up * hy);
        self.quad([m - r - u, m + r - u, m + r + u, m - r + u], lk, -ax, -1.0, ax, 1.0);
    }

    /// A quad along p0..p1, half width hw, facing the camera as far as it can; u from
    /// u0 at p0 to u1 at p1, v -1..1 across.
    fn ribbon(&mut self, cam: &Cam, p0: V3, p1: V3, hw: f32, lk: &Look, u0: f32, u1: f32) {
        let d = p1 - p0;
        let view = (p0 + p1) * 0.5 - cam.eye;
        let side = d.cross(view);
        if side.len() < 1e-8 {
            return;
        }
        let s = side.norm() * hw;
        self.quad([p0 - s, p1 - s, p1 + s, p0 + s], lk, u0, -1.0, u1, 1.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn flare(col: V3, bright: f32, core: f32, glow: f32, star: f32, star_sharp: f32, streak: f32, streak_sharp: f32, streak_col: V3, len: f32, angle: f32) -> Look {
        Look {
            kind: K_FLARE,
            len,
            col,
            bright,
            col2: streak_col,
            seed: 0.0,
            p3: [core, glow, star, star_sharp],
            p4: [streak, streak_sharp, angle, 0.0],
        }
    }

    fn cam(v: &View) -> Cam {
        let m = &v.view.0;
        let h = v.rect.h.max(1) as f32;
        let p11 = v.proj.0[1][1];
        Cam {
            eye: v.eye,
            right: v3(m[0][0], m[1][0], m[2][0]),
            up: v3(m[0][1], m[1][1], m[2][1]),
            px: if p11 > 1e-6 { 2.0 / (p11 * h) } else { 0.001 },
        }
    }

    // ---- the warp flash ---------------------------------------------------------------

    /// `WarpSpriteEffect::Render` drew (we kept it from drawing) stock's sprite for the
    /// effect `eff` at `p` (its +4: its handle is at +0, its distance at +0x10).
    pub fn note_flash(&mut self, eff: usize, p: V3, now: f32) {
        let handle = unsafe { crate::engine::rd::<i32>(eff) };
        let dist = field_f32(eff, 0x10);
        let o = eng::object_of(handle);
        if o == 0 {
            return;
        }
        let i = match self.flashes.iter().position(|f| f.handle == handle) {
            Some(i) if self.flashes[i].passed.is_some_and(|t| now - t > self.warp.after * 4.0 + self.warp.ring_time) => {
                self.flashes[i].passed = None;
                i
            }
            Some(i) => i,
            None => {
                if self.flashes.len() >= 32 {
                    return;
                }
                self.flashes_made += 1;
                if self.flashes_made <= 8 {
                    crate::log!("ordnance: warp flash: {}, radius {:.1}, made {:.0} ahead", eng::odf_name(o), eng::radius_of(o), dist);
                }
                self.flashes.push(Flash {
                    handle,
                    pos: p,
                    r: 0.0,
                    k: 0.0,
                    seen: now,
                    passed: None,
                    _seed: hash32((handle as u32).wrapping_mul(2654435761).wrapping_add(self.flashes_made)),
                    noted: false,
                });
                self.flashes.len() - 1
            }
        };
        let (pos_o, vel) = (eng::obj_pos(o), eng::obj_vel(o));
        let d = p - pos_o;
        let vl = vel.len();
        let s = if vl > 1e-4 { d.dot(vel) / vl } else { d.len() };
        let r = eng::radius_of(o);
        let f = &mut self.flashes[i];
        f.pos = p;
        f.r = r;
        f.k = if dist > 1e-3 { clamp(1.0 - s.abs() / dist, 0.0, 1.0) } else { 0.0 };
        f.seen = now;
        f.noted = true;
        if s <= 0.0 && f.passed.is_none() {
            f.passed = Some(now);
        }
    }

    /// How bright a flash is now: k^Rise while the craft closes on the point; once it
    /// is there a burst, then the afterglow.
    fn flash_now(w: &Warp, f: &Flash, now: f32) -> f32 {
        match f.passed {
            None => powf(f.k.max(0.0), w.rise),
            Some(t0) => {
                let t = (now - t0).max(0.0);
                (1.0 + w.burst * expf(-t / (w.after * 0.35))) * expf(-t / w.after)
            }
        }
    }

    fn age_flashes(&mut self, now: f32) {
        let w = &self.warp;
        let keep = |f: &Flash| {
            let end = match f.passed {
                Some(t) => t + w.after * 4.0 + w.ring_time,
                None => f.seen + 0.25, // the engine dropped it before the craft got there
            };
            // A clock that went back (a new mission) forgets them too.
            now <= end && now + 1.0 >= f.seen
        };
        self.flashes.retain(keep);
    }

    fn flash_quads(&mut self, cam: &Cam, i: usize, now: f32) {
        let (pos, r, k, passed) = {
            let f = &self.flashes[i];
            (f.pos, f.r, f.k, f.passed)
        };
        let w = &self.warp;
        let mut b = Self::flash_now(w, &self.flashes[i], now);
        let dist = (pos - cam.eye).len();
        let px = cam.px * dist;
        let grow = match passed {
            Some(t) => 1.0 + 0.4 * clamp((now - t) / w.after, 0.0, 1.0),
            None => 0.35 + 0.65 * k,
        };
        let mut h = w.size * 2.5 * r * grow;
        let hmin = w.min_px * px * 0.5;
        if h < hmin {
            b *= sqrtf(h / hmin);
            h = hmin;
        }
        if b < 0.004 {
            return;
        }
        let lk = Self::flare(rgb(w.col), w.bright * b, w.core, w.glow, w.star, w.star_sharp, w.streak, w.streak_sharp, rgb(w.streak_col), w.streak_len, 0.0);
        let (sl, ring, ring_time, ring_speed, ring_width, ring_col, bright) =
            (w.streak_len, w.ring, w.ring_time, w.ring_speed, w.ring_width, rgb(w.ring_col), w.bright);
        self.billboard(cam, pos, h * sl, h, sl, h * 0.5, &lk);
        if let (true, Some(t0)) = (ring > 0.0, passed) {
            let t = now - t0;
            if t < ring_time {
                let rr = ring_speed * r * t + h * 0.2;
                let kk = 1.0 - t / ring_time;
                let mut lk = lk;
                lk.kind = K_RING;
                lk.len = 0.0;
                lk.bright = ring * bright * kk * kk;
                lk.col = ring_col;
                lk.p3[0] = 1.0 / (1.0 + ring_width);
                lk.p3[1] = ring_width / (1.0 + ring_width);
                self.billboard(cam, pos, rr * (1.0 + ring_width), rr * (1.0 + ring_width), 1.0, 0.0, &lk);
            }
        }
    }

    /// The warp step: draws every flash noted in it (and the afterglows of those the
    /// engine has dropped).
    pub fn warp_draw(&mut self, v: &View) {
        let now = v.time;
        self.age_flashes(now);
        if self.flashes.is_empty() {
            return;
        }
        let cam = Self::cam(v);
        for i in 0..self.flashes.len() {
            let f = &self.flashes[i];
            // A flash the engine no longer draws (the craft is past it) is ours alone,
            // seen where it was; one the engine did not note this view was not visible.
            if !f.noted && f.passed.is_none() {
                continue;
            }
            self.flash_quads(&cam, i, now);
        }
        for f in self.flashes.iter_mut() {
            f.noted = false;
        }
        self.draw(v);
    }

    /// The flashes' lights for the next view.
    fn flash_lights(&mut self, now: f32) {
        let w = &self.warp;
        if !w.on || w.light <= 0.0 {
            return;
        }
        for f in &self.flashes {
            let b = Self::flash_now(w, f, now);
            if b < 0.02 {
                continue;
            }
            let c = rgb(w.light_col) * (w.light * b.min(3.0));
            let reach = w.light_reach * f.r;
            self.lights_next.push(Lit { pos: f.pos, col: c, start: reach, range: reach * 0.2 });
        }
    }

    // ---- the special weapons' shots ------------------------------------------------------

    fn flicker(rc: &Recipe, seed: u32, now: f32) -> f32 {
        if rc.flicker <= 0.0 {
            return 1.0;
        }
        let tick = (now * rc.flicker_rate) as i32 as u32;
        1.0 - rc.flicker * rnd(seed.wrapping_add(tick.wrapping_mul(747796405)))
    }

    fn age_of(&mut self, ord: usize, rc: usize, now: f32) -> usize {
        let frame = self.frame;
        if let Some(i) = self.ages.iter().position(|a| a.ord == ord && a.rc == rc) {
            self.ages[i].frame = frame;
            return i;
        }
        let a = Age { ord, rc, first: now, frame, last: None, dir: V3::ZERO };
        if let Some(i) = self.ages.iter().position(|a| frame.wrapping_sub(a.frame) > 30) {
            self.ages[i] = a;
            return i;
        }
        self.ages.push(a);
        self.ages.len() - 1
    }

    fn add_light(&mut self, pos: V3, col: [f32; 3], k: f32, reach: f32) {
        if !self.main || k <= 0.01 || self.lights_next.len() >= 24 {
            return;
        }
        // Full to the reach, gone a fifth of it further.
        self.lights_next.push(Lit { pos, col: rgb(col) * k, start: reach, range: reach * 0.2 });
    }

    fn beam(&mut self, cam: &Cam, ord: usize, rc: usize, age: usize, now: f32) {
        let r = self.recipes[rc].clone();
        let a: V3 = unsafe { crate::engine::rd(ord + 0xbc) };
        let b: V3 = unsafe { crate::engine::rd(ord + 0xc8) };
        let d = b - a;
        let len = d.len();
        if len < 1e-3 {
            return;
        }
        let d = d * (1.0 / len);
        let seed = hash32(crate::clock::entity_key(ord) as u32);
        let px = cam.px * (((a + b) * 0.5) - cam.eye).len();
        let mut hw = r.width * 0.5;
        let hmin = r.min_px * px * 0.5;
        let mut bright = r.bright * clamp((now - self.ages[age].first) / r.fade_in, 0.0, 1.0) * Self::flicker(&r, seed, now);
        if hw < hmin {
            bright *= sqrtf(hw / hmin);
            hw = hmin;
        }
        if bright <= 0.004 {
            return;
        }
        let l = len / hw;
        let (p0, p1) = (a - d * hw, b + d * hw);
        let sd = (seed & 1023) as f32 * 0.01;
        let lk = Look {
            kind: K_BEAM,
            len: l,
            col: rgb(r.col),
            bright,
            col2: rgb(r.core),
            seed: sd,
            p3: [r.core_w, r.flow[0], r.flow[1], r.flow[2]],
            p4: [r.helix[0], r.helix[1], r.helix[2], r.crackle],
        };
        if r.taper > 0.98 && r.taper < 1.02 {
            self.ribbon(cam, p0, p1, hw, &lk, -1.0, l + 1.0);
        } else {
            // A cone: twelve segments widening from hw to Taper hw; u stays in units of
            // the start's half width, v -1..1 across each.
            let n = 12;
            for k in 0..n {
                let (f0, f1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                let (q0, q1) = (p0 + (p1 - p0) * f0, p0 + (p1 - p0) * f1);
                let w = hw * (1.0 + (r.taper - 1.0) * (f0 + f1) * 0.5);
                self.ribbon(cam, q0, q1, w, &lk, -1.0 + (l + 2.0) * f0, -1.0 + (l + 2.0) * f1);
            }
        }
        if r.emitter > 0.0 {
            let h = r.emitter * hw;
            let fl = Self::flare(rgb(r.col), bright * 0.6, 30.0, 3.5, r.star, 18.0, 0.0, 20.0, V3::ZERO, 1.0, now * 0.7 + sd);
            self.billboard(cam, a, h, h, 1.0, h * 0.5, &fl);
        }
        // The impact: the beam's end, or where it enters its target's bounding sphere
        // (a mining beam ends inside the moon, where a flare and a light show nothing).
        let mut b = b;
        if r.surface > 0.0 {
            let tgt = eng::object_of(unsafe { crate::engine::rd::<i32>(ord + 0x60) });
            if tgt != 0 {
                let (c, rad) = (eng::obj_pos(tgt), eng::radius_of(tgt) * r.surface);
                // a + d t, |a + d t - c| = rad: the first root within the beam.
                let m = a - c;
                let bq = m.dot(d);
                let cq = m.dot(m) - rad * rad;
                let disc = bq * bq - cq;
                if cq > 0.0 && disc > 0.0 {
                    let t = -bq - sqrtf(disc);
                    if t > 0.0 && t < len {
                        b = a + d * t;
                    }
                }
            }
        }
        if r.impact > 0.0 {
            let h = r.impact * hw * if r.taper > 1.0 { sqrtf(r.taper) } else { 1.0 };
            let fl = Self::flare(rgb(r.col), bright * 0.6, 30.0, 3.0, r.star, 18.0, 0.0, 20.0, V3::ZERO, 1.0, -now * 0.9 + sd);
            self.billboard(cam, b, h, h, 1.0, h * 0.5, &fl);
        }
        if r.light > 0.0 {
            self.add_light(b, r.light_col, r.light * bright, r.light_reach);
        }
    }

    /// A tractor, holding or assimilation beam (ART-DIRECTION.md): a translucent cone
    /// from a point on the emitter (+0xbc) widening onto its target, `Spread` of the
    /// target's bounding radius wide where the beam ends (+0xc8), and on, fading, for
    /// `Beyond` of it. Built of camera-facing segments, each node's side its own (so
    /// the segments meet); the wide end is drawn towards the camera along the line to
    /// the eye (`Pull` target radii, growing as the square of the way along), which
    /// leaves it where it was on screen but in front of the hull it holds. The fans
    /// (wedges inside it that come and go) and the vapour are the shader's. Its
    /// light is a wash on the side of the target the cone falls on.
    fn cone(&mut self, cam: &Cam, ord: usize, rc: usize, age: usize, now: f32) {
        let r = self.recipes[rc].clone();
        let a: V3 = unsafe { crate::engine::rd(ord + 0xbc) };
        let b: V3 = unsafe { crate::engine::rd(ord + 0xc8) };
        let d = b - a;
        let len = d.len();
        if len < 1e-3 {
            return;
        }
        let d = d * (1.0 / len);
        let tgt = eng::object_of(unsafe { crate::engine::rd::<i32>(ord + 0x60) });
        // The target's size, or without one a cone an eighth as wide as it is long.
        let (tc, tr) = if tgt != 0 { (eng::obj_pos(tgt), eng::radius_of(tgt)) } else { (b, (len * 0.125).max(r.width)) };
        let seed = hash32(crate::clock::entity_key(ord) as u32);
        let bright = r.bright * clamp((now - self.ages[age].first) / r.fade_in, 0.0, 1.0) * Self::flicker(&r, seed, now);
        if bright <= 0.004 {
            return;
        }
        let hw0 = (r.width * 0.5).max(r.min_px * cam.px * (a - cam.eye).len() * 0.5);
        // The wide end takes in Spread of the target as it lies across the beam, seen
        // from here: its meshes' boxes measured along the cone's side at the end (a
        // ship met nose on is narrow, broadside long), not its bounding sphere.
        let across = if tgt != 0 { target_across(tgt, d, b - cam.eye) } else { None };
        let hw1 = (r.spread * across.unwrap_or(tr)).max(hw0);
        let s_end = 1.0 + r.beyond * tr / len;
        let sd = (seed & 1023) as f32 * 0.01;
        let lk = Look {
            kind: K_CONE,
            len: len / hw1,
            col: rgb(r.col),
            bright,
            col2: v3(s_end, 0.0, 0.0),
            seed: sd,
            p3: [clamp(r.fans[0], 0.0, 4.0), clamp(r.fans[1], 0.0, 1.0), r.fans[2], r.edge_px],
            p4: [clamp(r.vapour[0], 0.0, 1.0), r.vapour[1], r.vapour[2], r.fall],
        };
        const N: usize = 16;
        let node = |k: usize| -> (V3, V3, f32) {
            let f = s_end * k as f32 / N as f32;
            let c = a + d * (len * f);
            let side = d.cross(c - cam.eye);
            let side = if side.len() < 1e-8 { V3::ZERO } else { side.norm() * (hw0 + (hw1 - hw0) * f) };
            let pull = r.pull * tr * clamp(f, 0.0, 1.0) * clamp(f, 0.0, 1.0);
            let towards = |p: V3| -> V3 {
                let to = cam.eye - p;
                let l = to.len();
                if l < 1e-3 { p } else { p + to * (pull.min(l * 0.5) / l) }
            };
            (towards(c - side), towards(c + side), f)
        };
        // Each segment is a trapezoid, so v across it is sent as (v w, w), w the
        // half width there, and divided out per pixel: interpolated straight, v bends
        // at the seam of the quad's two triangles and the streaks zigzag.
        let wid = |f: f32| (hw0 + (hw1 - hw0) * f) / hw1;
        let mut prev = node(0);
        for k in 1..=N {
            let next = node(k);
            let (w0, w1) = (wid(prev.2), wid(next.2));
            let p = [prev.0, next.0, next.1, prev.1];
            let uvw = [[prev.2, -w0, w0], [next.2, -w1, w1], [next.2, w1, w1], [prev.2, w0, w0]];
            self.quad_w(p, &lk, uvw);
            prev = next;
        }
        if r.sprinkle_share > 0.0 {
            // The second colour: the same cone again, two fans of its own (its own
            // seed, a little quicker), so now and then a fan of it comes and goes.
            let lk2 = Look {
                col: rgb(r.sprinkle),
                bright: bright * r.sprinkle_share,
                seed: sd + 37.3,
                p3: [lk.p3[0].min(2.0), lk.p3[1], lk.p3[2] * 1.3, lk.p3[3]],
                ..lk
            };
            let mut prev = node(0);
            for k in 1..=N {
                let next = node(k);
                let (w0, w1) = (wid(prev.2), wid(next.2));
                let p = [prev.0, next.0, next.1, prev.1];
                let uvw = [[prev.2, -w0, w0], [next.2, -w1, w1], [next.2, w1, w1], [prev.2, w0, w0]];
                self.quad_w(p, &lk2, uvw);
                prev = next;
            }
        }
        if r.emitter > 0.0 {
            // The small bright point on the emitter where the cone starts.
            let h = r.emitter * hw0;
            let fl = Self::flare(rgb(r.col), bright * 0.6, 30.0, 3.5, r.star, 18.0, 0.0, 20.0, V3::ZERO, 1.0, sd);
            self.billboard(cam, a, h, h, 1.0, h * 0.5, &fl);
        }
        if r.light > 0.0 && self.main && self.lights_next.len() < 24 {
            // The wash: a soft light back from the target towards the emitter, so the
            // side the cone falls on takes it and the far side, turned away, does not.
            let pos = if tgt != 0 { tc - d * (tr * r.wash_back) } else { b };
            let reach = tr * r.wash_reach;
            let k = r.light * bright;
            if k > 0.01 {
                self.lights_next.push(Lit { pos, col: rgb(r.light_col) * k, start: reach, range: reach * 0.6 });
            }
        }
    }

    fn bolt(&mut self, cam: &Cam, ord: usize, rc: usize, age: usize, now: f32) {
        let r = self.recipes[rc].clone();
        let t = eng::transform(ord);
        let pos = t.p;
        // The way it flies: from where it was last; at first, toward its target.
        let ag = &mut self.ages[age];
        match ag.last {
            Some(last) => {
                let m = pos - last;
                if m.len() > 1e-3 {
                    ag.dir = m.norm();
                }
            }
            None => {
                let tgt = eng::object_of(unsafe { crate::engine::rd::<i32>(ord + 0x60) });
                ag.dir = if tgt != 0 { (eng::obj_pos(tgt) - pos).norm() } else { t.r[2].norm() };
            }
        }
        ag.last = Some(pos);
        let dir = ag.dir;
        let first = ag.first;
        let seed = hash32(crate::clock::entity_key(ord) as u32);
        let px = cam.px * (pos - cam.eye).len();
        let mut rad = r.size;
        let hmin = r.min_px * px * 0.5;
        let mut bright = r.bright * clamp((now - first) / r.fade_in, 0.0, 1.0) * Self::flicker(&r, seed, now);
        if r.pulse > 0.0 {
            bright *= 1.0 + r.pulse * sinf(now * r.pulse_rate + (seed & 255) as f32);
        }
        if rad < hmin {
            bright *= sqrtf(rad / hmin);
            rad = hmin;
        }
        if bright <= 0.004 {
            return;
        }
        let (p0, p1) = (pos - dir * (rad * r.tail), pos + dir * rad);
        let sd = (seed & 1023) as f32 * 0.01;
        let ang = now * r.spin + sd;
        let lk = Look {
            kind: K_BOLT,
            len: r.tail,
            col: rgb(r.col),
            bright,
            col2: rgb(r.core),
            seed: sd,
            p3: [18.0, 3.0, r.rays, ang],
            p4: [0.0; 4],
        };
        self.ribbon(cam, p0, p1, rad, &lk, -r.tail, 1.0);
        // Seen head on the ribbon is edge on: a flare holds the head.
        let fl = Self::flare(rgb(r.col), bright * 0.6, 25.0, 3.5, r.rays, 14.0, 0.0, 20.0, V3::ZERO, 1.0, ang);
        self.billboard(cam, pos, rad, rad, 1.0, 0.0, &fl);
        if r.light > 0.0 {
            self.add_light(pos, r.light_col, r.light * bright, r.light_reach);
        }
    }

    fn field_(&mut self, cam: &Cam, ord: usize, rc: usize, age: usize, now: f32) {
        let r = self.recipes[rc].clone();
        let h = unsafe { crate::engine::rd::<i32>(ord + if r.on_owner { 0x5c } else { 0x60 }) };
        let o = eng::object_of(h);
        let (c, rad) = if o != 0 { (eng::obj_pos(o), r.radius * eng::radius_of(o)) } else { (eng::transform(ord).p, r.radius * 50.0) };
        let seed = hash32(crate::clock::entity_key(ord) as u32);
        let bright = r.bright * clamp((now - self.ages[age].first) / r.fade_in, 0.0, 1.0) * Self::flicker(&r, seed, now);
        if bright <= 0.004 {
            return;
        }
        let lk = Look {
            kind: K_FIELD,
            len: 0.0,
            col: rgb(r.col),
            bright,
            col2: rgb(r.core),
            seed: (seed & 1023) as f32 * 0.01,
            p3: [r.rim, r.swirl[0], r.swirl[1], r.swirl[2]],
            p4: [r.fill, 0.0, 0.0, 0.0],
        };
        self.billboard(cam, c, rad, rad, 1.0, 0.0, &lk);
        if r.light > 0.0 {
            self.add_light(c, r.light_col, r.light * bright, r.light_reach);
        }
    }

    /// A special weapon's shot with a recipe: its quads, queued for `draw`.
    pub fn shot(&mut self, v: &View, ord: usize, rc: usize) {
        let now = v.time;
        let cam = Self::cam(v);
        let age = self.age_of(ord, rc, now);
        match self.recipes[rc].kind {
            Kind::Bolt => self.bolt(&cam, ord, rc, age, now),
            Kind::Field => self.field_(&cam, ord, rc, age, now),
            Kind::Cone => self.cone(&cam, ord, rc, age, now),
            _ => self.beam(&cam, ord, rc, age, now),
        }
        if !self.said.contains(&rc) && self.said.len() < 128 {
            self.said.push(rc);
            crate::log!("ordnance: drawing special {} ({:?})", self.recipes[rc].name, self.recipes[rc].kind);
        }
    }

    /// A glowing core at `c` (a mine's heart, a pod's engines): a flare of half size
    /// `h` (never under `min_px` across), `col` times `bright`, with a four-pointed star
    /// of share `star` at `angle`, pulled towards the camera by `pull`; and its light,
    /// `light` times `col`, full to `reach`. For `models.rs`.
    #[allow(clippy::too_many_arguments)]
    pub fn glow(&mut self, v: &View, c: V3, h: f32, min_px: f32, col: V3, bright: f32, star: f32, angle: f32, pull: f32, light: f32, reach: f32) {
        let cam = Self::cam(v);
        let px = cam.px * (c - cam.eye).len();
        let hmin = min_px * px * 0.5;
        let (mut h, mut b) = (h, bright);
        if h < hmin {
            b *= sqrtf(h / hmin);
            h = hmin;
        }
        if b > 0.004 {
            let fl = Self::flare(col, b, 30.0, 3.0, star, 16.0, 0.0, 20.0, V3::ZERO, 1.0, angle);
            self.billboard(&cam, c, h, h, 1.0, pull, &fl);
        }
        if light > 0.0 {
            self.add_light(c, [col.x, col.y, col.z], light * bright, reach);
        }
    }

    /// The start of a view's ordnance step.
    pub fn begin_shots(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// A special weapon's wave (a `ShockwaveExplosion` with a `[Wave.]` recipe), drawn
    /// now: in its own plane about its position, as far out as the engine has run it,
    /// fading after its class's `fadeFactor` (+0xe0) of the way, as stock fades its
    /// disc.
    pub fn wave(&mut self, v: &View, ex: usize, rc: usize) {
        const FN_SHOCK_RADIUS: usize = 0x467730; // ShockwaveExplosion::GetCurrentRadius() const
        let r = self.recipes[rc].clone();
        let cls = field(ex, 0x30);
        let t = eng::transform(ex);
        let rad = unsafe {
            let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_SHOCK_RADIUS);
            f(ex)
        };
        let end = field_f32(ex, 0x50);
        let fade = if cls != 0 { field_f32(cls, 0xe0) } else { 0.5 };
        if !(rad > 0.01) {
            return;
        }
        let mut k = 1.0;
        if end > 0.01 && rad > fade * end {
            k = clamp((end - rad) / ((1.0 - fade) * end + 1e-3), 0.0, 1.0);
        }
        let seed = hash32(crate::clock::entity_key(ex) as u32);
        let bright = r.bright * k * Self::flicker(&r, seed, v.time);
        if bright <= 0.004 {
            return;
        }
        let up = t.r[1].norm();
        let tt = if up.x.abs() > 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
        let e1 = up.cross(tt).norm();
        let e2 = up.cross(e1).norm();
        let hh = rad * (1.0 + r.band * 3.0);
        let c = t.p;
        let lk = Look {
            kind: K_WAVE,
            len: 0.0,
            col: rgb(r.col),
            bright,
            col2: rgb(r.core),
            seed: (seed & 1023) as f32 * 0.01,
            p3: [rad / hh, r.band * rad / hh, r.ray_count, r.ray_share],
            p4: [r.fill, r.turb, 0.0, 0.0],
        };
        let (a1, a2) = (e1 * hh, e2 * hh);
        self.quad([c - a1 - a2, c + a1 - a2, c + a1 + a2, c - a1 + a2], &lk, -1.0, -1.0, 1.0, 1.0);
        if r.light > 0.0 {
            self.add_light(c, r.light_col, r.light * bright, rad * 1.2);
        }
        if !self.said.contains(&rc) && self.said.len() < 128 {
            self.said.push(rc);
            crate::log!("ordnance: drawing wave {}", r.name);
        }
        self.draw(v);
    }

    /// At the start of a view: the lights to register in it. At the main view's start
    /// the last frame's effects' lights (and the flashes') become them; the inset
    /// registers the same.
    pub fn take_lights(&mut self, now: f32, main: bool) -> &[Lit] {
        self.main = main;
        if main {
            self.flash_lights(now);
            self.lights = core::mem::take(&mut self.lights_next);
        }
        &self.lights
    }

    /// Draws every queued quad.
    pub fn draw(&mut self, v: &View) {
        if self.verts.is_empty() {
            return;
        }
        let verts = core::mem::take(&mut self.verts);
        let Some(mut cmd) = gfx::cmd() else { return };
        // Added, depth-tested and not written, red, green and blue, no culling.
        let pipe = self.pipes.view(0, || crate::mods::effects::fx_desc(&VS_ORD_FX, &PS_ORD_FX, &FXLAYOUT, FX_STRIDE, Topology::TriangleList, Blend::ADD));
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        cmd.vs_block(Block::new(0, &gfx::vp_constants(v)));
        let t = v.time - (v.time / 1000.0) as i32 as f32 * 1000.0;
        cmd.ps_block(Block::new(0, &[t, 0.0, 0.0, 0.0]));
        for chunk in verts.chunks(6 * 2048) {
            if let Some(s) = cmd.vertices(chunk) {
                cmd.draw(&s);
            }
        }
    }
}

/// Half the width of an object across a beam along `d`, in the direction the cone
/// opens on screen (`d` x `view`): its shown meshes' boxes as the hulls module posed
/// them this view. None if the object is not posed.
fn target_across(o: usize, d: V3, view: V3) -> Option<f32> {
    let p = crate::mods::hulls::posed(o)?;
    let side = d.cross(view);
    if side.len() < 1e-8 {
        return None;
    }
    let side = side.norm();
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for part in &p.model.parts {
        if !p.pose.shown.get(part.node).copied().unwrap_or(false) {
            continue;
        }
        let Some(w) = p.pose.world.get(part.node) else { continue };
        let c = w.transform_point((part.lo + part.hi) * 0.5).dot(side);
        let h = (part.hi - part.lo) * 0.5;
        let e = w.r[0].dot(side).abs() * h.x + w.r[1].dot(side).abs() * h.y + w.r[2].dot(side).abs() * h.z;
        lo = lo.min(c - e);
        hi = hi.max(c + e);
    }
    (hi > lo).then_some((hi - lo) * 0.5)
}
