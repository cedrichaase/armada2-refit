//! The quads construction, the worker bees and the sparks draw with
//! `shaders/effects_construction.hlsl`: arcs' flares, sparks as streaks, hot spots,
//! cooling seams and welding beams, built on the CPU facing the camera and drawn
//! additively in one call, depth-tested against the hulls, writing no depth.
use super::Cam;
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use gpu::{Blend, Block, Topology};
use crate::math::{v3, V3};
use crate::shaders::effects_construction::*;
use alloc::vec::Vec;

pub const FLARE: f32 = 0.0;
pub const STREAK: f32 = 1.0;
pub const HOT: f32 = 2.0;
pub const SEAM: f32 = 3.0;
pub const BEAM: f32 = 4.0;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    p: [f32; 3],
    t0: [f32; 4],
    t1: [f32; 4],
}

const MAX_QUADS: usize = 4096;

pub struct Fx {
    pipes: Pipes,
    quads: Vec<Vert>,
    /// The flare's shape: core sharpness, glow sharpness, rays' share, rays' sharpness.
    pub flare: [f32; 4],
}

impl Fx {
    pub fn new() -> Fx {
        Fx { pipes: Pipes::new(), quads: Vec::new(), flare: [60.0, 3.5, 1.4, 30.0] }
    }

    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn quad(&mut self, pts: [V3; 4], kind: f32, w4: f32, col: V3, bright: f32, ua: f32, va: f32, ub: f32, vb: f32) {
        if self.quads.len() >= MAX_QUADS * 6 || !(bright > 0.0) {
            return;
        }
        let uv = [[ua, va], [ub, va], [ub, vb], [ua, vb]];
        for o in [0, 1, 2, 0, 2, 3] {
            self.quads.push(Vert { p: [pts[o].x, pts[o].y, pts[o].z], t0: [uv[o][0], uv[o][1], kind, w4], t1: [col.x, col.y, col.z, bright] });
        }
    }

    /// A square facing the camera at `ctr`, half size `h`, pulled `pull` towards it.
    #[allow(clippy::too_many_arguments)]
    pub fn billboard(&mut self, c: &Cam, ctr: V3, h: f32, pull: f32, kind: f32, w4: f32, col: V3, bright: f32) {
        let to = (c.eye - ctr).norm();
        let m = ctr + to * pull;
        let (r, u) = (c.right * h, c.up * h);
        self.quad([m - r - u, m + r - u, m + r + u, m - r + u], kind, w4, col, bright, -1.0, -1.0, 1.0, 1.0);
    }

    /// A spark: a capsule from `tail` to `head`, `hw` half wide.
    pub fn streak(&mut self, c: &Cam, tail: V3, head: V3, hw: f32, col: V3, bright: f32) {
        let d = head - tail;
        let len = d.len();
        if len < hw * 0.25 {
            return self.billboard(c, head, hw, 0.0, STREAK, 0.0, col, bright);
        }
        let dn = d * (1.0 / len);
        let side = dn.cross(head - c.eye);
        if side.len() < 1e-8 {
            return self.billboard(c, head, hw, 0.0, STREAK, 0.0, col, bright);
        }
        let side = side.norm() * hw;
        let t = tail - dn * hw;
        let h = head + dn * hw;
        self.quad([t - side, h - side, h + side, t + side], STREAK, len / hw, col, bright, 0.0, -1.0, 1.0, 1.0);
    }

    /// A band from `a` to `b`, `hw` half wide, turned to face the camera: x runs 0 at
    /// `a` to 1 at `b`, y -1..1 across.
    #[allow(clippy::too_many_arguments)]
    pub fn ribbon(&mut self, c: &Cam, a: V3, b: V3, hw: f32, kind: f32, w4: f32, col: V3, bright: f32) {
        let d = b - a;
        if d.len() < 1e-5 {
            return;
        }
        let side = d.cross((a + b) * 0.5 - c.eye);
        if side.len() < 1e-8 {
            return;
        }
        let side = side.norm() * hw;
        self.quad([a - side, b - side, b + side, a + side], kind, w4, col, bright, 0.0, -1.0, 1.0, 1.0);
    }

    /// Draws and empties the queue.
    pub fn draw(&mut self, v: &View, c: &Cam) {
        if self.quads.is_empty() {
            return;
        }
        let _ = v;
        let pipe = self.pipes.view(0, || super::fx_desc(&VS_CWELD, &PS_CWELD, &super::QUAD2, 44, Topology::TriangleList, Blend::ADD));
        if let (Some(mut cmd), Some(pipe)) = (gfx::cmd(), pipe) {
            if cmd.set_pipeline(&pipe) {
                cmd.vs_block(Block::new(0, &c.vp));
                cmd.ps_block(Block::new(0, &self.flare));
                if let Some(s) = cmd.vertices(&self.quads) {
                    cmd.draw(&s);
                }
            }
        }
        self.quads.clear();
    }
}

impl Default for Fx {
    fn default() -> Fx {
        Fx::new()
    }
}

/// A spark's colour as it cools, 1 white-hot to 0 dull red (the repair welding's ramp).
pub fn heat_colour(k: f32) -> V3 {
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

/// A unit vector from three 0..1 draws, uniform enough for spark directions.
pub fn rand_dir(a: f32, b: f32) -> V3 {
    let y = a * 2.0 - 1.0;
    let th = b * core::f32::consts::TAU;
    let q = libm::sqrtf((1.0 - y * y).max(0.0));
    v3(libm::sinf(th) * q, y, libm::cosf(th) * q)
}

/// One spark of a burst: where its tail and head are `age` seconds after it left
/// `from` along `dir` at `speed`, slowed by `drag` over its `life`; None once gone.
pub fn spark_at(from: V3, dir: V3, speed: f32, age: f32, life: f32, drag: f32, streak_time: f32) -> Option<(V3, V3, f32)> {
    if !(age > 0.0) || age >= life {
        return None;
    }
    let travel = |t: f32| speed * t * (1.0 - 0.5 * drag * t / life);
    let tail_t = if age > streak_time { age - streak_time } else { 0.0 };
    Some((from + dir * travel(tail_t), from + dir * travel(age), 1.0 - age / life))
}
