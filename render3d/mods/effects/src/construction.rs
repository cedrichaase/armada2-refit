//! Construction: `BuildingEffect` taken through `feature()`, in place of
//! `BuildingEffect::Render` (0x469660). render3d/README.md, "effects", has the engine
//! findings.
//!
//! What stock draws, per construction (`BuildingEffect::m_list`, a `std::map` at
//! 0x736bd0 whose node `+0x14` holds the `Info`): the station being built, an
//! instance embedded in the `Info` (`+0x20`) drawn with a face limit (the faces built
//! so far, `+0xcc`), in its builder's team colour; and at every welding edge (the list
//! at `+0xfc`, made by `AddNewWeldingEdges` from the edges of the faces added since the
//! last frame, each with its start and end in the world and a progress 0..1 that
//! `BuildingEffect::Simulate` moves at `weldingSpeed` a second) one `xspark` sprite
//! sliding from the edge's start to its end.
//!
//! Ours: the station through the hulls module's hull look (`hulls::draw_instance`),
//! faces limited as stock limits them; and at each edge the welding as the repair
//! welding draws its arc (Repair.asi's look): a flickering arc with turning rays at the
//! weld, a hot spot, sparks streaking off, and behind it the seam it leaves, white-hot
//! at the weld and cooling along the edge to orange and dull red, which goes on cooling
//! after the edge is done. A few arcs per construction light the hull
//! (`Module::lights`).
use super::hooks::{construction as hk, sees_all};
use super::{blank, rnd, Cam, LightPool, Sec};
use super::weldfx::{self, Fx};
use r3d_core::frame::steps::View;
use crate::log;
use r3d_core::math::{v3, M34, V3};
use alloc::vec::Vec;

/// A welding edge, live (the engine's) or done and cooling (ours).
#[derive(Clone, Copy)]
struct Edge {
    node: usize,
    info: usize,
    s: V3,
    e: V3,
    t: f32,
    /// Its progress a second.
    speed: f32,
    seed: u32,
    /// Sim time it was found done (gone from the engine's list), or < 0 while live.
    done: f32,
    /// The station's centre and radius, for the sparks' and the light's reach.
    centre: V3,
    radius: f32,
    /// Its construction was visible to the player when last seen.
    visible: bool,
    seen: bool,
    /// Its construction had all its faces when last seen (a construction that is gone
    /// without, cancelled or destroyed, takes its seams with it: its chunks fly off).
    complete: bool,
}

struct Set {
    sparks: usize,
    brightness: f32,
    flicker_rate: f32,
    stutter: f32,
    flare: f32,
    flare_min_px: f32,
    arc_share: f32,
    arc: V3,
    hot_col: V3,
    light_col: V3,
    hot: f32,
    seam: f32,
    seam_width: f32,
    seam_min_px: f32,
    cool: f32,
    spark_speed: f32,
    spark_spread: f32,
    spark_life: [f32; 2],
    spark_drag: f32,
    spark_width: f32,
    spark_min_px: f32,
    spark_bright: f32,
    streak_time: f32,
    lights: usize,
    light: f32,
    light_reach: f32,
}

pub struct Construction {
    pub on: bool,
    /// `Enable=`: the part wants to run.
    pub want: bool,
    s: Set,
    edges: Vec<Edge>,
    made: u32,
    lights: LightPool,
    said: u32,
    said_fallback: bool,
}

/// `BuildingEffect::Render`'s test: the builder is seen, or its team is one the player
/// sees, or the editor is on, or the view is the one that shows everything.
fn visible(info: usize) -> bool {
    let b = hk::builder(info);
    if b != 0 && r3d_core::engine::obj::can_user_see(b) {
        return true;
    }
    hk::teams_seen(info) || sees_all()
}

fn scale_sod(odf: &str) -> f32 {
    r3d_core::assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0)
}

impl Construction {
    pub fn new() -> Construction {
        let c = Sec::with(blank(), "Construction");
        Construction {
            on: false,
            want: false,
            s: Self::read(&c),
            edges: Vec::new(),
            made: 0,
            lights: LightPool::new(b"construction weld\0"),
            said: 0,
            said_fallback: false,
        }
    }

    fn read(c: &Sec) -> Set {
        let life = c.pair("SparkLife", [0.2, 0.6]);
        let l0 = life[0].max(0.02);
        Set {
                sparks: c.i("Sparks", 3).clamp(0, 16) as usize,
                brightness: c.f("Brightness", 1.2),
                flicker_rate: c.f("FlickerRate", 24.0).max(1.0),
                stutter: c.f("Stutter", 0.2).clamp(0.0, 0.95),
                flare: c.f("Flare", 2.0),
                flare_min_px: c.f("FlareMinPixels", 12.0),
                arc_share: c.f("ArcShare", 0.3).clamp(0.0, 1.0),
                arc: c.rgb("ArcColour", [120.0, 165.0, 255.0]),
                hot_col: c.rgb("HotColour", [255.0, 120.0, 30.0]),
                light_col: c.rgb("LightColour", [150.0, 185.0, 255.0]),
                hot: c.f("HotSpot", 1.0),
                seam: c.f("Seam", 1.6),
                seam_width: c.f("SeamWidth", 0.35),
                seam_min_px: c.f("SeamMinPixels", 1.5),
                cool: c.f("CoolTime", 0.8).max(0.05),
                spark_speed: c.f("SparkSpeed", 0.12),
                spark_spread: c.f("SparkSpread", 0.9),
                spark_life: [l0, life[1].max(l0)],
                spark_drag: c.f("SparkDrag", 0.8).clamp(0.0, 1.9),
                spark_width: c.f("SparkWidth", 0.002),
                spark_min_px: c.f("SparkMinPixels", 1.4),
                spark_bright: c.f("SparkBrightness", 2.0),
                streak_time: c.f("StreakTime", 0.035),
                lights: c.i("Lights", 4).clamp(0, 8) as usize,
                light: c.f("Light", 1.5),
                light_reach: c.f("LightReach", 0.35),
        }
    }

    /// `[Construction]`.
    pub fn configure(&mut self, ini: &crate::rt::Ini) {
        let c = Sec::of(ini, "Construction");
        self.want = c.i("Enable", 1) != 0;
        self.s = Self::read(&c);
    }

    /// The engine's edges into ours: new ones taken, gone ones marked done.
    fn gather(&mut self, now: f32) {
        for e in &mut self.edges {
            e.seen = false;
        }
        let mut infos = Vec::new();
        hk::each_info(|i| infos.push(i));
        for &info in &infos {
            let vis = visible(info);
            let complete = hk::faces(info) >= hk::faces_total(info);
            for e in self.edges.iter_mut().filter(|e| e.info == info) {
                e.complete = complete;
            }
            let speed = hk::speed(info).max(0.01);
            let inst = hk::instance(info);
            let mut r = super::hooks::instance_radius(inst);
            if !(r > 1.0 && r < 1e5) {
                r = 100.0;
            }
            let centre = super::hooks::instance_centre(inst);
            let head = hk::edges_head(info);
            if head == 0 {
                continue;
            }
            let mut n = hk::edge_next(head);
            let mut guard = 0;
            while n != 0 && n != head && guard < 4096 {
                guard += 1;
                let s = hk::edge_start(n);
                let e = hk::edge_end(n);
                let t = hk::edge_t(n).clamp(0.0, 1.0);
                match self.edges.iter_mut().find(|x| x.done < 0.0 && x.node == n && (x.s - s).len() < 1e-3 && (x.e - e).len() < 1e-3) {
                    Some(x) => {
                        x.t = t;
                        x.seen = true;
                        x.visible = vis;
                        x.centre = centre;
                        x.radius = r;
                    }
                    None => {
                        if self.edges.len() < 4096 {
                            self.made = self.made.wrapping_add(1);
                            self.edges.push(Edge {
                                node: n,
                                info,
                                s,
                                e,
                                t,
                                speed,
                                seed: super::hash32(self.made.wrapping_mul(2654435761) ^ n as u32),
                                done: -1.0,
                                centre,
                                radius: r,
                                visible: vis,
                                seen: true,
                                complete,
                            });
                        }
                    }
                }
                n = hk::edge_next(n);
            }
        }
        self.edges.retain(|e| e.complete || infos.contains(&e.info));
        let keep = self.s.cool * 5.0 + self.s.spark_life[1];
        for e in &mut self.edges {
            if !e.seen && e.done < 0.0 {
                e.done = now;
                e.t = 1.0;
            }
        }
        self.edges.retain(|e| e.done < 0.0 || (now - e.done < keep && now >= e.done));
    }

    fn arc_now(&self, e: &Edge, now: f32) -> f32 {
        if e.done >= 0.0 {
            return 0.0;
        }
        // Of the many edges welded at once only some arc at a time, each in bursts.
        let burst = (now * 8.0) as i64 as u32;
        if rnd(e.seed ^ burst.wrapping_mul(2654435761)) >= self.s.arc_share {
            return 0.0;
        }
        let tick = (now * self.s.flicker_rate) as i64 as u32;
        let on = if rnd(e.seed ^ ((now * 9.0) as i64 as u32).wrapping_mul(2246822519)) < self.s.stutter { 0.15 } else { 1.0 };
        on * (0.45 + 0.55 * rnd(e.seed.wrapping_add(tick.wrapping_mul(747796405))))
    }

    /// A few arcs per construction light its hull.
    pub fn lights(&mut self, v: &View) {
        if !self.on || self.s.lights == 0 || self.s.light <= 0.0 {
            return;
        }
        let now = v.time;
        let mut n = 0;
        let mut infos: Vec<usize> = Vec::new();
        for e in &self.edges {
            if e.done < 0.0 && e.visible && !infos.contains(&e.info) {
                infos.push(e.info);
            }
        }
        for info in infos {
            let live: Vec<Edge> = self.edges.iter().filter(|e| e.info == info && e.done < 0.0 && e.visible).copied().collect();
            let k = live.len().min(self.s.lights);
            for j in 0..k {
                if n >= 16 {
                    return;
                }
                let e = live[j * live.len() / k];
                let arc = self.arc_now(&e, now);
                if arc < 0.02 {
                    continue;
                }
                let p = e.s + (e.e - e.s) * e.t;
                let out = (p - e.centre).norm();
                let reach = self.s.light_reach * e.radius;
                if self.lights.register_in(n, p + out * (0.03 * e.radius), self.s.light_col * (self.s.light * arc), reach * 0.3, reach * 0.7, super::SPARK.id()) {
                    n += 1;
                }
            }
        }
    }

    /// `BuildingEffect::Render` in our place: the stations being built, through the
    /// hulls module; the welds are noted for `draw`. False (the engine draws the whole
    /// feature) when the hulls module does not run.
    pub fn feature(&mut self, v: &View) -> bool {
        let Some(hulls) = r3d_core::services::hulls() else {
            if !self.said_fallback {
                self.said_fallback = true;
                log!("effects: construction: the hulls module does not run: the engine draws construction");
            }
            return false;
        };
        self.gather(v.time);
        let mut infos = Vec::new();
        hk::each_info(|i| infos.push(i));
        for info in infos {
            if !visible(info) {
                continue;
            }
            let faces = hk::faces(info);
            let inst = hk::instance(info);
            if faces <= 0 {
                continue;
            }
            let pos = super::hooks::instance_centre(inst);
            if hk::distance_culled(inst, &v.eye, &pos) {
                continue;
            }
            let b = hk::builder(info);
            let colour = hk::team_colour(info, b);
            // As stock: the scale effects into the instance, its frustum test, its face
            // limit. Each writes only the effect's own instance.
            let shown = hk::prepare(inst, info, faces, v.cam);
            if shown {
                let name = hk::class_odf(info);
                let s = r3d_core::scene::instance::instance_scale(inst) * scale_sod(name);
                let mut world: M34 = super::hooks::instance_world(inst);
                world.r[0] = world.r[0] * s.x;
                world.r[1] = world.r[1] * s.y;
                world.r[2] = world.r[2] * s.z;
                let ours = !name.is_empty() && hulls.draw_instance(v, inst, inst, true, &r3d_core::assets::sod_of(name), &world, Some(colour));
                if ours {
                    r3d_core::particles::feed(inst, name, &world, v);
                    self.said += 1;
                    if self.said <= 4 || self.said % 600 == 0 {
                        log!(
                            "effects: construction: {} at {:.0},{:.0},{:.0}: {} of {} faces, {} welding edge(s) live",
                            name, world.p.x, world.p.y, world.p.z, faces, hk::faces_total(info),
                            self.edges.iter().filter(|e| e.info == info && e.done < 0.0).count()
                        );
                    }
                } else {
                    // The engine's own draw of the instance, as stock makes it.
                    hk::engine_draw(inst, colour, v.cam);
                }
            }
            // The construction object's on-screen byte, which the HUD reads.
            hk::mark_on_screen(b, shown);
        }
        true
    }

    /// The welds, the seams and the sparks, at `Step::Chunks` (every hull in depth).
    pub fn draw(&mut self, v: &View, c: &Cam, fx: &mut Fx) {
        let now = v.time;
        let s = &self.s;
        for e in self.edges.iter().filter(|e| e.visible) {
            let len = (e.e - e.s).len();
            if len < 1e-4 {
                continue;
            }
            let wr = hk::welding_radius(e.info);
            let wr = if e.done < 0.0 && wr > 0.0 && wr < 100.0 { wr } else { 1.0 };
            let edge_time = 1.0 / e.speed;
            let (end, age, fade) = if e.done < 0.0 {
                (e.s + (e.e - e.s) * e.t, e.t * edge_time, 1.0)
            } else {
                (e.e, edge_time + (now - e.done), libm::expf(-(now - e.done) / s.cool))
            };
            let px = c.pixel * (end - c.eye).len();
            // The seam: from the edge's start to the weld, cooling behind it.
            if s.seam > 0.0 && fade > 0.01 {
                let hw = (s.seam_width * wr).max(s.seam_min_px * px * 0.5);
                let k = (e.t * edge_time) / s.cool;
                let thin = libm::sqrtf((s.seam_width * wr / hw).min(1.0));
                fx.ribbon(c, e.s, end, hw, weldfx::SEAM, k, v3(1.0, 1.0, 1.0), s.seam * s.brightness * fade * thin);
            }
            // The sparks, emitted while the weld ran.
            let out = (end - e.centre).norm();
            for k in 0..s.sparks {
                let sd = e.seed.wrapping_add((k as u32).wrapping_mul(0x9e3779b9));
                let te = rnd(sd) * edge_time;
                let life = s.spark_life[0] + (s.spark_life[1] - s.spark_life[0]) * rnd(sd.wrapping_add(1));
                let from = e.s + (e.e - e.s) * (te / edge_time).clamp(0.0, 1.0);
                let dir = (out + weldfx::rand_dir(rnd(sd.wrapping_add(2)), rnd(sd.wrapping_add(3))) * s.spark_spread).norm();
                let sp = s.spark_speed * e.radius * (0.3 + 0.7 * rnd(sd.wrapping_add(5)));
                if let Some((tail, head, kh)) = weldfx::spark_at(from, dir, sp, age - te, life, s.spark_drag, s.streak_time) {
                    let phys = s.spark_width * e.radius;
                    let hw = phys.max(s.spark_min_px * px * 0.5);
                    let b = s.spark_bright * s.brightness * kh * (0.5 + 0.5 * kh) * libm::sqrtf(phys / hw);
                    fx.streak(c, tail, head, hw, weldfx::heat_colour(kh), b);
                }
            }
            if e.done >= 0.0 {
                continue;
            }
            // The weld: a hot spot and the arc's flare.
            let m = s.flare_min_px * px * 0.5;
            if s.hot > 0.0 {
                let h = (wr * 0.6).max(m * 0.5);
                fx.billboard(c, end, h, h, weldfx::HOT, 0.0, s.hot_col, s.hot * s.brightness);
            }
            let arc = self.arc_now(e, now);
            if arc > 0.01 {
                let h = (s.flare * wr).max(m);
                let ang = rnd(e.seed.wrapping_add(((now * s.flicker_rate) as i64 as u32).wrapping_mul(2891336453))) * core::f32::consts::PI;
                fx.billboard(c, end, h, h, weldfx::FLARE, ang, s.arc, s.brightness * arc);
            }
        }
    }
}

impl Default for Construction {
    fn default() -> Construction {
        Construction::new()
    }
}

