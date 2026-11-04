//! The sparks: every particle of `NodeParticleEffect` that is not a repair weld,
//! drawn by us. render3d/README.md, "effects", has the engine findings.
//!
//! Stock: `NodeParticleEffect::AddParticle` (0x4733c0 at a hardpoint, 0x4734c0 at a
//! point in the object's frame) is called by `Craft::DamageAlloc` and
//! `Craft::DramaticDeathSequence` (a system blowing up: `systemexpl1..7`, half size the
//! bounding radius times 0.14, at least 4, for a second), `Phaser::Init` (the phaser's
//! impact sprite, in its colour) and `Beam::Simulate` (`beam explosion1`), besides
//! the repair welds. `NodeParticleEffect::Render` draws each as its sprite facing the
//! camera (`ST3D_Sprite::DrawScaled3D`, coloured by the particle's `+0x20`, half size
//! `+0x2c`) at its hardpoint (`Entity::GetWorldPosition`) or its point (`+0x10`) in the
//! object's frame; `Simulate` ages it (`+0x34`) and erases it at its life (`+0x38`).
//!
//! `Render` stays the engine's (it also erases the particles of dead objects); its
//! draw goes through repair.rs's thunk, which skips every particle while the sparks are
//! on. Ours, at `Step::Chunks`: stock's sprite and flipbook, drawn as stock draws it
//! (the ordnance module's `Sprites`, its hottest texels past white), and new: a burst
//! of sparks streaking off it, cooling, and a flash of light on the hull.
use super::hooks::{nodes, object_of};
use super::weldfx::{self, Fx};
use super::{blank, rnd, Cam, LightPool, Sec};
use r3d_core::frame::steps::View;
use crate::log;
use r3d_core::math::{v3, V3};
use r3d_core::scene::sprites::Sprites;
use alloc::vec::Vec;

static mut TAKE: bool = false;

/// Whether the sparks draw `NodeParticleEffect`'s particles (the engine's draw skips
/// them all then).
pub fn takes() -> bool {
    unsafe { TAKE }
}

/// The bench's A/B switch (mod.rs).
pub fn set_taking(on: bool) {
    unsafe { TAKE = on };
}

#[derive(Clone, Copy)]
struct Spark {
    pos: V3,
    out: V3,
    size: f32,
    age: f32,

    colour: V3,
    /// A system blowing up (`systemexpl`), not a weapon's impact.
    blast: bool,
    seed: u32,
    sprite: usize,
}

struct Set {
    blast_sparks: usize,
    hit_sparks: usize,
    spark_speed: f32,
    spark_spread: f32,
    spark_life: [f32; 2],
    spark_bright: f32,
    spark_min_px: f32,
    burst: f32,
    blast_col: V3,
    light: f32,
    light_reach: f32,
    light_time: f32,
}

pub struct SparkFx {
    pub on: bool,
    /// `Enable=`: the part wants to run (it needs repair's hooks in as well).
    pub want: bool,
    s: Set,
    sprites: Sprites,
    live: Vec<Spark>,
    lights: LightPool,
    said: u32,
}

impl SparkFx {
    pub fn new() -> SparkFx {
        let c = Sec::with(blank(), "Sparks");
        SparkFx {
            on: false,
            want: false,
            s: Self::read(&c),
            sprites: Sprites::new(),
            live: Vec::new(),
            lights: LightPool::new(b"spark\0"),
            said: 0,
        }
    }

    fn read(c: &Sec) -> Set {
        let life = c.pair("SparkLife", [0.3, 0.9]);
        let l0 = life[0].max(0.02);
        Set {
                blast_sparks: c.i("BlastSparks", 14).clamp(0, 48) as usize,
                hit_sparks: c.i("HitSparks", 6).clamp(0, 48) as usize,
                spark_speed: c.f("SparkSpeed", 4.0),
                spark_spread: c.f("SparkSpread", 1.0),
                spark_life: [l0, life[1].max(l0)],
                spark_bright: c.f("SparkBrightness", 2.5),
                spark_min_px: c.f("SparkMinPixels", 1.4),
                burst: c.f("BurstTime", 0.15).max(0.0),
                blast_col: c.rgb("BlastLight", [255.0, 150.0, 60.0]),
                light: c.f("Light", 2.0),
                light_reach: c.f("LightReach", 6.0),
                light_time: c.f("LightTime", 0.5).max(0.05),
        }
    }

    /// Switched off: the particles gathered go.
    pub fn clear(&mut self) {
        self.live.clear();
    }

    /// `[Sparks]`.
    pub fn configure(&mut self, ini: &crate::rt::Ini) {
        let c = Sec::of(ini, "Sparks");
        self.want = c.i("Enable", 1) != 0;
        self.s = Self::read(&c);
        self.sprites.hot = c.f("Hot", 0.6);
        self.sprites.hot_from = c.f("HotFrom", 0.7);
    }

    pub fn reset(&mut self) {
        self.sprites.reset();
    }

    /// Every live particle that is not a weld, as `NodeParticleEffect::Render` places it.
    fn gather(&mut self) {
        self.live.clear();
        let mut nodes_now = Vec::new();
        nodes::each(|n| nodes_now.push(n));
        for node in nodes_now {
            if super::repair::is_weld(node) {
                continue;
            }
            let o = object_of(nodes::handle(node));
            if o == 0 || !r3d_core::engine::obj::can_user_see(o) {
                continue;
            }
            let tm = r3d_core::engine::obj::transform(o);
            let hp = nodes::hardpoint(node);
            let pos = if hp != 0 { nodes::world_pos(o, hp) } else { tm.transform_point(nodes::local(node)) };
            let sprite = nodes::sprite(node);
            if sprite == 0 {
                continue;
            }
            let name = r3d_core::scene::instance::element_name(sprite).unwrap_or("");
            let blast = name.len() >= 10 && name[..10].eq_ignore_ascii_case("systemexpl");
            let c = nodes::colour(node);
            let out = pos - tm.p;
            let out = if out.len() > 1e-4 { out.norm() } else { v3(0.0, 1.0, 0.0) };
            self.live.push(Spark {
                pos,
                out,
                size: nodes::size(node),
                age: nodes::age(node),

                colour: v3(c[0], c[1], c[2]),
                blast,
                // Under the bench clock not the node's address (clock.rs): when it was born.
                seed: if r3d_core::clock::bench() {
                    super::hash32((r3d_core::engine::sim_time() - nodes::age(node)).to_bits() ^ nodes::handle(node) as u32)
                } else {
                    super::hash32(node as u32 ^ nodes::handle(node) as u32)
                },
                sprite,
            });
        }
    }

    /// A flash of light on the hull from each blast and impact, gone in `LightTime`.
    pub fn lights(&mut self, _v: &View) {
        if !self.on || self.s.light <= 0.0 {
            return;
        }
        self.gather();
        let mut n = 0;
        let live = self.live.clone();
        for p in &live {
            if n >= 8 {
                break;
            }
            let k = (p.age / 0.05).clamp(0.0, 1.0) * libm::expf(-p.age / self.s.light_time);
            if k < 0.03 {
                continue;
            }
            let col = if p.blast { self.s.blast_col } else { p.colour };
            let reach = self.s.light_reach * p.size.max(1.0);
            if self.lights.register(n, p.pos + p.out * (p.size * 0.5), col * (self.s.light * k), reach * 0.3, reach * 0.7) {
                n += 1;
            }
        }
    }

    /// The sprites (stock's look), then the sparks into `fx`.
    pub fn draw(&mut self, v: &View, c: &Cam, fx: &mut Fx) {
        self.gather();
        if self.live.is_empty() {
            return;
        }
        let live = core::mem::take(&mut self.live);
        for p in &live {
            self.sprites.scaled(v, p.sprite, p.pos, p.size, p.colour);
        }
        self.sprites.draw(v);
        let s = &self.s;
        for p in &live {
            let count = if p.blast { s.blast_sparks } else { s.hit_sparks };
            let px = c.pixel * (p.pos - c.eye).len();
            for k in 0..count {
                let sd = p.seed.wrapping_add((k as u32).wrapping_mul(0x9e3779b9));
                let te = rnd(sd) * s.burst;
                let life = s.spark_life[0] + (s.spark_life[1] - s.spark_life[0]) * rnd(sd.wrapping_add(1));
                let dir = (p.out + weldfx::rand_dir(rnd(sd.wrapping_add(2)), rnd(sd.wrapping_add(3))) * s.spark_spread).norm();
                let sp = s.spark_speed * p.size * (0.3 + 0.7 * rnd(sd.wrapping_add(5)));
                if let Some((tail, head, kh)) = weldfx::spark_at(p.pos, dir, sp, p.age - te, life, 0.8, 0.04) {
                    let hw = (s.spark_min_px * px * 0.5).max(0.02);
                    fx.streak(c, tail, head, hw, weldfx::heat_colour(kh), s.spark_bright * kh * (0.5 + 0.5 * kh));
                }
            }
        }
        self.said += 1;
        if self.said <= 4 || self.said % 300 == 0 {
            let p = &live[0];
            log!(
                "effects: sparks: {} particle(s) drawn; first: {}, size {:.1}, age {:.2}, at {:.0},{:.0},{:.0}",
                live.len(), r3d_core::scene::instance::element_name(p.sprite).unwrap_or("?"), p.size, p.age, p.pos.x, p.pos.y, p.pos.z
            );
        }
        self.live = live;
    }
}

impl Default for SparkFx {
    fn default() -> SparkFx {
        SparkFx::new()
    }
}
