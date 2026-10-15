//! The repair welding, ported from `Repair.asi` (repair/README.md, which holds the
//! look settled with the user and the engine facts below): at each spark of a repair
//! an arc that flickers with turning rays and lights the hull, a hot spot cooling on
//! the hull, sparks streaking off and cooling.
//!
//! `Craft::RepairSystem` adds each spark with `NodeParticleEffect::AddParticle`
//! (0x4733c0, its call at 0x4c90c0): we chain that call and note the new tail of the
//! particle list (head node at [0x736cfc], count at 0x736d00) as a weld, with the
//! handle (+8), hardpoint (+0xc) and sprite (+0x1c) it holds, so a node freed and
//! reused by another effect is not taken for one. A weld cannot be told by its sprite:
//! hull-hit sparks share the `xspark` art.
//!
//! `NodeParticleEffect::Render` stays the engine's (it also prunes particles of dead
//! objects): its call to `ST3D_Sprite::DrawScaled3D` at 0x473767 goes through a thunk
//! that notes the particle's node (in `esi` there), and a weld is skipped. Every other
//! spark is stock. The welds are drawn at `Step::Chunks` (mod.rs).
//!
//! A particle: position from `Entity::GetWorldPosition(ST3D_Node *)` (0x4cff70) at its
//! hardpoint, or +0x10 in the object's frame; size +0x2c, age +0x34, life +0x38.
use super::{object_of, rnd, Cam, LightPool, Sec};
use crate::d3d::*;
use crate::engine::{self, field, hook, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Shader};
use crate::log;
use crate::math::{v3, V3};
use crate::shaders::effects_repair::*;
use alloc::vec::Vec;

const SITE_ADD: usize = 0x4c90c0;
const FN_ADD: usize = 0x4733c0;
const SITE_SPRITE: usize = 0x473767;
const FN_SPRITE: usize = 0x63aa60;
const NP_LIST: usize = 0x736cfc;
const NP_COUNT: usize = 0x736d00;
const FN_WORLD_POS: usize = 0x4cff70;

const P_HANDLE: usize = 0x08;
const P_NODE: usize = 0x0c;
const P_LOCAL: usize = 0x10;
const P_SPRITE: usize = 0x1c;
const P_SIZE: usize = 0x2c;
const P_AGE: usize = 0x34;
const P_LIFE: usize = 0x38;

#[derive(Clone, Copy)]
struct Weld {
    node: usize,
    handle: i32,
    hp: usize,
    sprite: usize,
    seed: u32,
}

impl Weld {
    fn matches(&self, node: usize) -> bool {
        self.node == node && field(node, P_HANDLE) as i32 == self.handle && field(node, P_NODE) == self.hp && field(node, P_SPRITE) == self.sprite
    }
}

static mut WELDS: Vec<Weld> = Vec::new();
static mut MADE: u32 = 0;
static mut PREV_ADD: usize = 0;
static mut PREV_SPRITE: usize = 0;
static mut RP_NODE: usize = 0;
/// The welds are ours: the engine's sprite for them is skipped.
static mut DRAWING: bool = false;

#[allow(static_mut_refs)]
fn welds() -> &'static mut Vec<Weld> {
    unsafe { &mut WELDS }
}

unsafe extern "C" fn hook_add(name: *const u8, colour: *const f32, o: usize, hp: usize, size: f32, life: f32) {
    unsafe {
        let before = rd::<u32>(NP_COUNT);
        let f: unsafe extern "C" fn(*const u8, *const f32, usize, usize, f32, f32) = core::mem::transmute(PREV_ADD);
        f(name, colour, o, hp, size, life);
        let head = rd::<usize>(NP_LIST);
        if head == 0 || rd::<u32>(NP_COUNT) != before.wrapping_add(1) || !DRAWING {
            return;
        }
        let node = field(head, 4); // the new tail
        if node == 0 || node == head || welds().len() >= 256 {
            return;
        }
        MADE = MADE.wrapping_add(1);
        welds().push(Weld {
            node,
            handle: field(node, P_HANDLE) as i32,
            hp: field(node, P_NODE),
            sprite: field(node, P_SPRITE),
            seed: super::hash32(MADE.wrapping_mul(2654435761).wrapping_add(node as u32)),
        });
        if MADE <= 8 {
            log!("effects: weld on {}, size {:.2}, life {:.2}{}", crate::engine::obj::odf_name(o), size, life, if hp != 0 { ", at a hardpoint" } else { "" });
        }
    }
}

unsafe extern "thiscall" fn repair_sprite(sprite: usize, cam: usize, pos: *const f32, size: f32) {
    unsafe {
        let node = RP_NODE;
        if DRAWING && node != 0 && welds().iter().any(|w| w.matches(node)) {
            return;
        }
        let f: unsafe extern "thiscall" fn(usize, usize, *const f32, f32) = core::mem::transmute(PREV_SPRITE);
        f(sprite, cam, pos, size);
    }
}

/// `NodeParticleEffect::Render` holds the particle's node in `esi` at the call.
#[unsafe(naked)]
unsafe extern "C" fn sprite_thunk() {
    core::arch::naked_asm!(
        "mov dword ptr [{node}], esi",
        "jmp {f}",
        node = sym RP_NODE,
        f = sym repair_sprite,
    )
}

struct Set {
    sparks: usize,
    brightness: f32,
    arc_share: f32,
    flicker_rate: f32,
    stutter: f32,
    flare: f32,
    flare_min_px: f32,
    core: f32,
    glow: f32,
    rays: f32,
    ray_sharp: f32,
    arc: V3,
    hot_col: V3,
    light_col: V3,
    spark_speed: f32,
    spark_spread: f32,
    spark_life: [f32; 2],
    spark_drag: f32,
    spark_width: f32,
    spark_min_px: f32,
    streak_time: f32,
    spark_bright: f32,
    hot: f32,
    hot_size: f32,
    hot_time: f32,
    light: f32,
    light_reach: f32,
    light_lift: f32,
    pull: f32,
}

/// A weld in the world.
struct Place {
    p: V3,
    n: V3,
    r: f32,
    size: f32,
    age: f32,
    life: f32,
    o: usize,
    seed: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    p: [f32; 3],
    t0: [f32; 4],
    t1: [f32; 4],
}

pub struct Repair {
    pub on: bool,
    s: Set,
    vs: Shader,
    ps: Shader,
    lights: LightPool,
    quads: Vec<Vert>,
    said_first: bool,
}

impl Repair {
    pub fn new() -> Repair {
        let c = Sec::new("Repair");
        let life = c.pair("SparkLife", [0.25, 0.8]);
        let l0 = life[0].max(0.02);
        Repair {
            on: c.i("Enable", 1) != 0,
            s: Set {
                sparks: c.i("Sparks", 20).clamp(0, 48) as usize,
                brightness: c.f("Brightness", 1.5),
                arc_share: c.f("ArcTime", 0.7).clamp(0.05, 1.0),
                flicker_rate: c.f("FlickerRate", 24.0).max(1.0),
                stutter: c.f("Stutter", 0.2).clamp(0.0, 0.95),
                flare: c.f("Flare", 2.2),
                flare_min_px: c.f("FlareMinPixels", 26.0),
                core: c.f("FlareCore", 60.0),
                glow: c.f("FlareGlow", 3.5),
                rays: c.f("FlareRays", 1.4),
                ray_sharp: c.f("FlareRaySharpness", 30.0).max(1.0),
                arc: c.rgb("ArcColour", [120.0, 165.0, 255.0]),
                hot_col: c.rgb("HotColour", [255.0, 120.0, 30.0]),
                light_col: c.rgb("LightColour", [150.0, 185.0, 255.0]),
                spark_speed: c.f("SparkSpeed", 0.9),
                spark_spread: c.f("SparkSpread", 0.9),
                spark_life: [l0, life[1].max(l0)],
                spark_drag: c.f("SparkDrag", 0.8).clamp(0.0, 1.9),
                spark_width: c.f("SparkWidth", 0.004),
                spark_min_px: c.f("SparkMinPixels", 1.6),
                streak_time: c.f("StreakTime", 0.035),
                spark_bright: c.f("SparkBrightness", 2.5),
                hot: c.f("HotSpot", 1.0),
                hot_size: c.f("HotSize", 0.5),
                hot_time: c.f("HotTime", 0.35).max(0.01),
                light: c.f("Light", 2.5),
                light_reach: c.f("LightReach", 0.5),
                light_lift: c.f("LightLift", 0.06),
                pull: c.f("Pull", 2.0),
            },
            vs: Shader::vertex(&VS_WELD),
            ps: Shader::pixel(&PS_WELD),
            lights: LightPool::new(b"weld\0"),
            quads: Vec::new(),
            said_first: false,
        }
    }

    /// The `AddParticle` call in `Craft::RepairSystem` and the `DrawScaled3D` call in
    /// `NodeParticleEffect::Render`, chained.
    pub fn install(&mut self) {
        unsafe {
            let ok_list = rd::<usize>(0x473677) == NP_LIST;
            let (Some(add), Some(spr)) = (hook::call_target(SITE_ADD), hook::call_target(SITE_SPRITE)) else {
                log!("effects: repair: the call sites are not calls here: stock sparks");
                self.on = false;
                return;
            };
            if !ok_list {
                log!("effects: repair: NodeParticleEffect::Render differs: stock sparks");
                self.on = false;
                return;
            }
            if hook::patch_call(SITE_ADD, add, hook_add as *const () as usize) {
                PREV_ADD = add;
            }
            if hook::patch_call(SITE_SPRITE, spr, sprite_thunk as *const () as usize) {
                PREV_SPRITE = spr;
            }
            log!(
                "effects: repair: AddParticle{} and DrawScaled3D{} hooked",
                if add != FN_ADD { " (chained)" } else { "" },
                if spr != FN_SPRITE { " (chained)" } else { "" }
            );
            DRAWING = PREV_ADD != 0 && PREV_SPRITE != 0;
            if !DRAWING {
                self.on = false;
            }
        }
    }

    /// Drop the welds whose particle the engine has erased.
    fn gather(&self) {
        let head = unsafe { rd::<usize>(NP_LIST) };
        let mut live = Vec::new();
        if head != 0 {
            let mut n = field(head, 0);
            let mut guard = 0;
            while n != 0 && n != head && guard < 1024 {
                live.push(n);
                n = field(n, 0);
                guard += 1;
            }
        }
        welds().retain(|w| live.iter().any(|&n| w.matches(n)));
    }

    fn place(&self, w: &Weld) -> Option<Place> {
        let o = object_of(w.handle);
        if o == 0 {
            return None;
        }
        let inst = field(o, 4);
        let tm: [f32; 12] = unsafe { rd(inst + 0x44) };
        let tr = [v3(tm[0], tm[1], tm[2]), v3(tm[3], tm[4], tm[5]), v3(tm[6], tm[7], tm[8])];
        let tp = v3(tm[9], tm[10], tm[11]);
        let p = if w.hp != 0 {
            let mut out = V3::ZERO;
            unsafe {
                let f: unsafe extern "thiscall" fn(usize, *mut V3, usize) -> *mut V3 = core::mem::transmute(FN_WORLD_POS);
                f(o, &mut out, w.hp);
            }
            out
        } else {
            let l = super::rv3(w.node + P_LOCAL);
            tr[0] * l.x + tr[1] * l.y + tr[2] * l.z + tp
        };
        let mut r = engine::field_f32(inst, 0x40);
        if !(r > 0.01) {
            r = 50.0;
        }
        let mut life = engine::field_f32(w.node, P_LIFE);
        if !(life > 0.01) {
            life = 1.0;
        }
        // The outward direction: the gradient of the shield ellipsoid at the weld.
        let n = match super::shields::ship_xform(o) {
            Some((a, t, _)) => {
                let m = [a[0].x, a[0].y, a[0].z, a[1].x, a[1].y, a[1].z, a[2].x, a[2].y, a[2].z];
                match inv3(&m) {
                    Some(b) => {
                        let d = p - t;
                        let u = v3(d.x * b[0] + d.y * b[3] + d.z * b[6], d.x * b[1] + d.y * b[4] + d.z * b[7], d.x * b[2] + d.y * b[5] + d.z * b[8]);
                        v3(u.x * b[0] + u.y * b[1] + u.z * b[2], u.x * b[3] + u.y * b[4] + u.z * b[5], u.x * b[6] + u.y * b[7] + u.z * b[8])
                    }
                    None => p - tp,
                }
            }
            None => p - tp,
        };
        let n = if n.len() < 1e-6 { v3(0.0, 1.0, 0.0) } else { n.norm() };
        Some(Place { p, n, r, size: engine::field_f32(w.node, P_SIZE), age: engine::field_f32(w.node, P_AGE), life, o, seed: w.seed })
    }

    fn arc_now(&self, pl: &Place, now: f64) -> f32 {
        let end = pl.life * self.s.arc_share;
        if pl.age >= end {
            return 0.0;
        }
        let tick = (now * self.s.flicker_rate as f64) as i64 as u32;
        let on = if rnd(pl.seed ^ ((now * 9.0) as i64 as u32).wrapping_mul(2246822519)) < self.s.stutter { 0.15 } else { 1.0 };
        let a = (pl.age / 0.04).clamp(0.0, 1.0) * ((end - pl.age) / 0.06).clamp(0.0, 1.0);
        a * on * (0.45 + 0.55 * rnd(pl.seed.wrapping_add(tick.wrapping_mul(747796405))))
    }

    fn hot_now(&self, pl: &Place) -> f32 {
        let end = pl.life * self.s.arc_share;
        let up = (pl.age / 0.25).clamp(0.0, 1.0);
        if pl.age < end { up } else { up * libm::expf(-(pl.age - end) / self.s.hot_time) }
    }

    /// The arcs' light on the hull.
    pub fn lights(&mut self, v: &View) {
        if !self.on || self.s.light <= 0.0 {
            return;
        }
        let now = v.time as f64;
        let ws: Vec<Weld> = welds().clone();
        let mut n = 0;
        for w in &ws {
            if n >= 16 {
                break;
            }
            let Some(pl) = self.place(w) else { continue };
            if !crate::engine::obj::can_user_see(pl.o) {
                continue;
            }
            let arc = self.arc_now(&pl, now);
            if arc < 0.02 {
                continue;
            }
            let reach = self.s.light_reach * pl.r;
            if self.lights.register(n, pl.p + pl.n * (self.s.light_lift * pl.r), self.s.light_col * (self.s.light * arc), reach, reach * 0.2) {
                n += 1;
            }
        }
    }

    pub fn draw(&mut self, v: &View, c: &Cam) {
        self.gather();
        if welds().is_empty() {
            return;
        }
        let now = v.time as f64;
        self.quads.clear();
        let ws: Vec<Weld> = welds().clone();
        for w in &ws {
            let Some(pl) = self.place(w) else { continue };
            if !crate::engine::obj::can_user_see(pl.o) {
                continue;
            }
            self.weld_quads(c, &pl, now);
        }
        if self.quads.is_empty() {
            return;
        }
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev)) {
            return;
        }
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 0), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHATESTENABLE, 0),
            (RS_ALPHABLENDENABLE, 1), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3),
            (RS_STENCILENABLE, 0), (152, 0), (RS_COLORWRITEENABLE, 7), (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        // XYZ, two sets of four-float texture coordinates
        dev.set_fvf(0x2 | 0x200 | (2 << 16) | (2 << 18));
        dev.vs_const(0, &c.vp);
        dev.ps_const(0, &[self.s.core, self.s.glow, self.s.rays, self.s.ray_sharp]);
        dev.draw_primitive_up(PT_TRIANGLELIST, (self.quads.len() / 3) as u32, self.quads.as_ptr() as *const u8, core::mem::size_of::<Vert>() as u32);
        if !self.said_first {
            self.said_first = true;
            log!("effects: first welds drawn: {} weld(s), {} quads", ws.len(), self.quads.len() / 6);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn quad(&mut self, pts: [V3; 4], kind: f32, w4: f32, col: V3, bright: f32, ua: f32, va: f32, ub: f32, vb: f32) {
        if self.quads.len() >= 2048 * 6 || bright <= 0.0 {
            return;
        }
        let uv = [[ua, va], [ub, va], [ub, vb], [ua, vb]];
        for o in [0, 1, 2, 0, 2, 3] {
            self.quads.push(Vert { p: [pts[o].x, pts[o].y, pts[o].z], t0: [uv[o][0], uv[o][1], kind, w4], t1: [col.x, col.y, col.z, bright] });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn billboard(&mut self, c: &Cam, ctr: V3, h: f32, pull: f32, kind: f32, w4: f32, col: V3, bright: f32) {
        let to = (c.eye - ctr).norm();
        let m = ctr + to * pull;
        let (r, u) = (c.right * h, c.up * h);
        self.quad([m - r - u, m + r - u, m + r + u, m - r + u], kind, w4, col, bright, -1.0, -1.0, 1.0, 1.0);
    }

    fn streak(&mut self, c: &Cam, tail: V3, head: V3, hw: f32, col: V3, bright: f32) {
        let d = head - tail;
        let len = d.len();
        if len < hw * 0.25 {
            return self.billboard(c, head, hw, 0.0, 1.0, 0.0, col, bright);
        }
        let dn = d * (1.0 / len);
        let side = dn.cross(head - c.eye);
        if side.len() < 1e-8 {
            return self.billboard(c, head, hw, 0.0, 1.0, 0.0, col, bright);
        }
        let side = side.norm() * hw;
        let t = tail - dn * hw;
        let h = head + dn * hw;
        self.quad([t - side, h - side, h + side, t + side], 1.0, len / hw, col, bright, 0.0, -1.0, 1.0, 1.0);
    }

    fn weld_quads(&mut self, c: &Cam, pl: &Place, now: f64) {
        let arc = self.arc_now(pl, now);
        let hot = self.hot_now(pl);
        let px = c.pixel * (pl.p - c.eye).len();
        let s_end = pl.life * self.s.arc_share;
        let seed = pl.seed;
        for k in 0..self.s.sparks {
            let s = &self.s;
            let sd = seed.wrapping_add((k as u32).wrapping_mul(0x9e3779b9));
            let t_emit = rnd(sd) * s_end * 0.95;
            let life = s.spark_life[0] + (s.spark_life[1] - s.spark_life[0]) * rnd(sd.wrapping_add(1));
            let age = pl.age - t_emit;
            if age <= 0.0 || age >= life {
                continue;
            }
            let rd3 = v3(rnd(sd.wrapping_add(2)) * 2.0 - 1.0, rnd(sd.wrapping_add(3)) * 2.0 - 1.0, rnd(sd.wrapping_add(4)) * 2.0 - 1.0).norm();
            let dir = (pl.n + rd3 * s.spark_spread).norm();
            let sp = s.spark_speed * pl.r * (0.3 + 0.7 * rnd(sd.wrapping_add(5)));
            let travel = sp * age * (1.0 - 0.5 * s.spark_drag * age / life);
            let tail_t = if age > s.streak_time { age - s.streak_time } else { 0.0 };
            let head = pl.p + dir * travel;
            let tail = pl.p + dir * (sp * tail_t * (1.0 - 0.5 * s.spark_drag * tail_t / life));
            let kh = 1.0 - age / life;
            let col = heat_colour(kh);
            let phys = s.spark_width * pl.r;
            let hw = phys.max(s.spark_min_px * px * 0.5);
            let bright = s.spark_bright * s.brightness * kh * (0.5 + 0.5 * kh) * libm::sqrtf(phys / hw);
            self.streak(c, tail, head, hw, col, bright);
        }
        if self.s.hot > 0.0 && hot > 0.01 {
            let h = self.s.hot_size * pl.size * 0.5;
            let m = self.s.flare_min_px * px * 0.25;
            self.billboard(c, pl.p, h.max(m), h * self.s.pull, 2.0, 0.0, self.s.hot_col, self.s.hot * self.s.brightness * hot);
        }
        if arc > 0.01 {
            let h = self.s.flare * pl.size * 0.5;
            let m = self.s.flare_min_px * px * 0.5;
            let ang = rnd(seed.wrapping_add(((now * self.s.flicker_rate as f64) as i64 as u32).wrapping_mul(2891336453))) * core::f32::consts::PI;
            let hh = h.max(m);
            self.billboard(c, pl.p, hh, hh * self.s.pull, 0.0, ang, self.s.arc, self.s.brightness * arc);
        }
    }
}

impl Default for Repair {
    fn default() -> Repair {
        Repair::new()
    }
}

fn heat_colour(k: f32) -> V3 {
    let k = k.clamp(0.0, 1.0);
    if k > 0.66 {
        let f = (k - 0.66) / 0.34;
        v3(1.0, 0.62 + 0.33 * f, 0.22 + 0.58 * f)
    } else if k > 0.33 {
        let f = (k - 0.33) / 0.33;
        v3(1.0, 0.30 + 0.32 * f, 0.06 + 0.16 * f)
    } else {
        let f = k / 0.33;
        v3(0.55 + 0.45 * f, 0.08 + 0.22 * f, 0.02 + 0.04 * f)
    }
}

fn inv3(a: &[f32; 9]) -> Option<[f32; 9]> {
    let det = a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6]) + a[2] * (a[3] * a[7] - a[4] * a[6]);
    if det.abs() < 1e-12 {
        return None;
    }
    let d = 1.0 / det;
    Some([
        (a[4] * a[8] - a[5] * a[7]) * d,
        (a[2] * a[7] - a[1] * a[8]) * d,
        (a[1] * a[5] - a[2] * a[4]) * d,
        (a[5] * a[6] - a[3] * a[8]) * d,
        (a[0] * a[8] - a[2] * a[6]) * d,
        (a[2] * a[3] - a[0] * a[5]) * d,
        (a[3] * a[7] - a[4] * a[6]) * d,
        (a[1] * a[6] - a[0] * a[7]) * d,
        (a[0] * a[4] - a[1] * a[3]) * d,
    ])
}
