//! The venting plasma and gas, drawn as `Venting.asi` drew them (the look
//! the user settled is kept as it was): each vent particle a puff of gas
//! whose billows roll and cool as it flies, a moving ship's puffs linked into one
//! unbroken trail, and at each vent that sprays a glow, a surging jet core and a light
//! on the hull about it. The particles are ours now (`sim.rs`), not the engine's.
//!
//! Settings: `Render3D.ini [Particles]`, under the names `Venting.ini` used.
use super::draw::Cam;
use super::sim::{System, Vent};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use gpu::{attr, AttrFormat, Blend, Block, Code, Compare, Depth, Format, PipelineDesc, Semantic, Topology, VertexAttr};
use crate::math::{v3, V3};
use crate::rt::Ini;
use crate::shaders::particles_vent::*;
use alloc::vec::Vec;

/// Per kind of vent: [0] plasma, [1] gas.
pub struct Cfg {
    pub kind_on: [bool; 2],
    pub brightness: [f32; 2],
    pub size: [f32; 2],
    pub stretch: [f32; 2],
    pub min_px: [f32; 2],
    pub hot: V3,
    pub mid: V3,
    pub cool: V3,
    pub gas: V3,
    pub gas_cold: V3,
    pub scale: f32,
    pub flow: f32,
    pub contrast: f32,
    pub soft: f32,
    pub cooling: [f32; 2],
    pub glow: [f32; 2],
    pub glow_size: [f32; 2],
    pub glow_min_px: [f32; 2],
    pub glow_col: [V3; 2],
    pub jet: [f32; 2],
    pub jet_len: [f32; 2],
    pub jet_width: [f32; 2],
    pub light: [f32; 2],
    pub light_col: [V3; 2],
    pub light_reach: [f32; 2],
    pub light_lift: [f32; 2],
    pub flicker: [f32; 2],
    pub max_link: f32,
    pub pull: f32,
}

fn rgb(ini: &Ini, key: &str, d: [f32; 3]) -> V3 {
    let c = ini.floats::<3>("Particles", key, d);
    v3(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0)
}

impl Cfg {
    pub fn read(ini: &Ini) -> Cfg {
        let f = |k: &str, d: f32| ini.float("Particles", k, d);
        let two = |k: &str, p: f32, g: f32| {
            let mut s = alloc::string::String::from("Plasma");
            s.push_str(k);
            let mut t = alloc::string::String::from("Gas");
            t.push_str(k);
            [f(&s, p), f(&t, g)]
        };
        let two_rgb = |k: &str, p: [f32; 3], g: [f32; 3]| {
            let mut s = alloc::string::String::from("Plasma");
            s.push_str(k);
            let mut t = alloc::string::String::from("Gas");
            t.push_str(k);
            [rgb(ini, &s, p), rgb(ini, &t, g)]
        };
        let fl = two("Flicker", 0.35, 0.15);
        let co = two("Cooling", 3.0, 2.0);
        Cfg {
            kind_on: [ini.int("Particles", "Plasma", 1) != 0, ini.int("Particles", "Gas", 1) != 0],
            brightness: two("Brightness", 1.0, 0.55),
            size: two("Size", 1.0, 1.1),
            stretch: two("Stretch", 0.8, 0.6),
            min_px: two("MinPixels", 3.0, 3.0),
            hot: rgb(ini, "PlasmaHot", [255.0, 215.0, 110.0]),
            mid: rgb(ini, "PlasmaMid", [255.0, 140.0, 35.0]),
            cool: rgb(ini, "PlasmaCool", [150.0, 25.0, 8.0]),
            gas: rgb(ini, "GasBright", [170.0, 200.0, 255.0]),
            gas_cold: rgb(ini, "GasCold", [55.0, 80.0, 220.0]),
            scale: f("BillowScale", 2.4),
            flow: f("BillowFlow", 0.6),
            contrast: f("BillowContrast", 2.5),
            soft: f("EdgeSoftness", 0.3),
            cooling: [co[0].clamp(0.2, 6.0), co[1].clamp(0.2, 6.0)],
            glow: two("Glow", 1.4, 0.35),
            glow_size: two("GlowSize", 1.6, 1.0),
            glow_min_px: two("GlowMinPixels", 10.0, 6.0),
            glow_col: two_rgb("GlowColour", [255.0, 190.0, 90.0], [170.0, 200.0, 255.0]),
            jet: two("Jet", 1.2, 0.4),
            jet_len: two("JetLength", 0.5, 0.35),
            jet_width: two("JetWidth", 0.55, 0.5),
            light: two("Light", 1.6, 0.4),
            light_col: two_rgb("LightColour", [255.0, 130.0, 45.0], [150.0, 185.0, 255.0]),
            light_reach: two("LightReach", 0.35, 0.25),
            light_lift: two("LightLift", 0.05, 0.05),
            flicker: [fl[0].clamp(0.0, 1.0), fl[1].clamp(0.0, 1.0)],
            max_link: f("MaxLink", 8.0),
            pull: f("Pull", 1.5),
        }
    }
}

/// A vent spraying this frame.
#[derive(Clone, Copy)]
pub struct VentAt {
    pub key: (usize, u32, u16),
    pub kind: Vent,
    pub pos: V3,
    pub dir: V3,
    pub size: f32,
    pub speed: f32,
    /// The ship's bounding radius.
    pub r: f32,
    pub seed: u32,
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}
fn rnd(seed: u32) -> f32 {
    (hash32(seed) & 0xff_ffff) as f32 / 16_777_216.0
}

/// Animation seconds (`clock::anim_now`: the wall clock, or the bench's), kept small.
fn wall() -> f32 {
    let t = crate::clock::anim_now();
    (t - libm::floor(t / 1000.0) * 1000.0) as f32
}

/// A vent's flicker now: two slow waves and a quick one, out of phase per vent.
pub fn flicker(seed: u32, depth: f32) -> f32 {
    let t = wall();
    let ph = rnd(seed) * 6.283_185_3;
    let w = 0.5 * libm::sinf(t * 7.3 + ph) + 0.3 * libm::sinf(t * 13.1 + ph * 1.7) + 0.2 * libm::sinf(t * 29.0 + ph * 2.3);
    1.0 - depth * 0.5 * (1.0 - w)
}

/// The vents spraying this frame (the emitters shown with a vent template).
pub fn vents(sys: &System, cfg: &Cfg) -> Vec<VentAt> {
    let mut out = Vec::new();
    for (k, ow) in sys.owners.iter() {
        let Some(set) = ow.set.as_ref() else { continue };
        for (i, e) in set.emitters.iter().enumerate() {
            let Some(kind) = e.tpl.vent else { continue };
            let st = &ow.ems[i];
            if !st.shown || !cfg.kind_on[kind as usize] {
                continue;
            }
            let d = st.world.r[2];
            let dir = if d.len() > 1e-6 { d.norm() } else { v3(0.0, 1.0, 0.0) };
            let size = if e.tpl.look.size[0] > 0.0 { e.tpl.look.size[0] } else { 1.0 };
            out.push(VentAt {
                key: (k.0, k.1, i as u16),
                kind,
                pos: st.world.p,
                dir,
                size,
                speed: e.tpl.speed,
                r: ow.radius,
                seed: hash32(k.0 as u32 ^ (k.1.wrapping_mul(2_654_435_761)) ^ ((i as u32) << 24)),
            });
        }
    }
    out
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    p: [f32; 3],
    t0: [f32; 4],
    t1: [f32; 4],
    t2: [f32; 4],
}

static LAYOUT: [VertexAttr; 4] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float4, Semantic::TexCoord, 0),
    attr(28, AttrFormat::Float4, Semantic::TexCoord, 1),
    attr(44, AttrFormat::Float4, Semantic::TexCoord, 2),
];

pub struct Drawer {
    verts: Vec<Vert>,
    pipes: Pipes,
}

/// A puff as the draw sees it.
struct Puff {
    owner: (usize, u32, u16),
    pos: V3,
    dir: V3,
    age: f32,
    life: f32,
    size: f32,
    grow: f32,
    seed: u32,
    kind: Vent,
}

impl Drawer {
    pub const fn new() -> Drawer {
        Drawer { verts: Vec::new(), pipes: Pipes::new() }
    }
    pub fn reset(&mut self) {}

    #[allow(clippy::too_many_arguments)]
    fn quad(&mut self, c: V3, a: V3, s: V3, hl: f32, hw: f32, jet: bool, kind: f32, seed: f32, col: V3, bright: f32, t2: [f32; 4]) {
        if bright <= 0.0 {
            return;
        }
        const CU: [f32; 4] = [-1.0, 1.0, 1.0, -1.0];
        const CV: [f32; 4] = [-1.0, -1.0, 1.0, 1.0];
        for o in [0usize, 1, 2, 0, 2, 3] {
            let p = c + a * (hl * CU[o]) + s * (hw * CV[o]);
            self.verts.push(Vert {
                p: [p.x, p.y, p.z],
                t0: [if jet { if CU[o] < 0.0 { 0.0 } else { 1.0 } } else { CU[o] }, CV[o], kind, seed],
                t1: [col.x, col.y, col.z, bright],
                t2,
            });
        }
    }

    /// Everything for one view: every vent particle of `sys` and the vents.
    pub fn draw(&mut self, v: &View, cam: &Cam, sys: &System, vents: &[VentAt], cfg: &Cfg) {
        self.verts.clear();
        let mut puffs: Vec<Puff> = Vec::new();
        for (k, ow) in sys.owners.iter() {
            if !ow.visible {
                continue;
            }
            for p in &ow.parts {
                let t = &p.tpl;
                let Some(kind) = t.vent else { continue };
                if !cfg.kind_on[kind as usize] {
                    continue;
                }
                let d = if p.vel.len() > 1e-8 { p.vel.norm() } else { v3(0.0, 1.0, 0.0) };
                puffs.push(Puff {
                    owner: (k.0, k.1, p.em),
                    pos: p.pos,
                    dir: d,
                    age: p.age.max(0.0),
                    life: if t.life > 0.05 { t.life } else { 2.0 },
                    size: if t.look.size[0] > 0.0 { t.look.size[0] } else { 1.0 },
                    grow: t.growth,
                    seed: p.seed,
                    kind,
                });
            }
        }
        if puffs.is_empty() && vents.is_empty() {
            return;
        }
        let now = wall();
        // Each emitter's puffs in order of age, each linked back to the next younger,
        // the youngest to its vent.
        puffs.sort_by(|a, b| a.owner.cmp(&b.owner).then(a.age.partial_cmp(&b.age).unwrap_or(core::cmp::Ordering::Equal)));
        for i in 0..puffs.len() {
            let p = &puffs[i];
            let mut to: Option<V3> = None;
            if i > 0 && puffs[i - 1].owner == p.owner {
                if p.age - puffs[i - 1].age < 0.5 {
                    to = Some(puffs[i - 1].pos);
                }
            } else if p.age < 0.5 {
                to = vents.iter().find(|vt| vt.key == p.owner).map(|vt| vt.pos);
            }
            self.puff(cam, cfg, p, to, now);
        }
        for vt in vents {
            if sys.owners.get(&(vt.key.0, vt.key.1)).is_some_and(|o| o.visible) {
                self.vent(cam, cfg, vt, now);
            }
        }
        if self.verts.is_empty() {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        // Added, depth-tested and not written, red, green and blue, no culling.
        let pipe = self.pipes.view(0, || {
            let mut d = PipelineDesc::new(Code::d3d9(&VS_VENT), Code::d3d9(&PS_VENT), &LAYOUT, 60, Topology::TriangleList, Format::Unknown);
            d.blend = Some(Blend::ADD);
            d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.write_mask = 7;
            d
        });
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        cmd.vs_block(Block::new(0, &gfx::vp_constants(v)));
        let c4 = |c: V3| [c.x, c.y, c.z, 0.0];
        let mut pc = [0.0f32; 28];
        pc[0..4].copy_from_slice(&c4(cfg.hot));
        pc[4..8].copy_from_slice(&c4(cfg.mid));
        pc[8..12].copy_from_slice(&c4(cfg.cool));
        pc[12..16].copy_from_slice(&c4(cfg.gas));
        pc[16..20].copy_from_slice(&c4(cfg.gas_cold));
        pc[20..24].copy_from_slice(&[cfg.scale, cfg.flow, cfg.contrast, cfg.soft]);
        pc[24..28].copy_from_slice(&[cfg.cooling[0], cfg.cooling[1], 0.0, 0.0]);
        cmd.ps_block(Block::new(0, &pc));
        let verts = core::mem::take(&mut self.verts);
        for chunk in verts.chunks(6 * 4096) {
            if let Some(s) = cmd.vertices(chunk) {
                cmd.draw(&s);
            }
        }
        self.verts = verts;
    }

    /// The axis along `d` as the camera sees it at `at`, with the side across it, and
    /// how much of `d` shows (1 across the view, 0 straight at the camera).
    fn screen_axis(cam: &Cam, at: V3, d: V3) -> (V3, V3, f32) {
        let view = (at - cam.eye).norm();
        let axis = d - view * d.dot(view);
        let l = axis.len();
        if l < 1e-4 {
            return (cam.right, cam.up, 0.0);
        }
        let axis = axis * (1.0 / l);
        let side = view.cross(axis).norm();
        (axis, side, l)
    }

    fn puff(&mut self, cam: &Cam, cfg: &Cfg, p: &Puff, to: Option<V3>, now: f32) {
        let k = p.kind as usize;
        let mut h = (p.size + p.grow * p.age) * cfg.size[k];
        let px = cam.px * (p.pos - cam.eye).len();
        let heat = (1.0 - p.age / p.life).clamp(0.0, 1.0);
        let mut dim = 1.0;
        if h < cfg.min_px[k] * px {
            dim = h / (cfg.min_px[k] * px);
            h = cfg.min_px[k] * px;
        }
        if !(h > 0.0) {
            return;
        }
        let (mut axis, mut side, show) = Self::screen_axis(cam, p.pos, p.dir);
        let mut hl = h * (1.0 + cfg.stretch[k] * show);
        let mut c = p.pos;
        if let Some(to) = to {
            let d = to - p.pos;
            let len = d.len();
            if len > 1e-4 && len < cfg.max_link * h {
                let mid = p.pos + d * 0.5;
                let (la, ls, sh) = Self::screen_axis(cam, mid, d * (1.0 / len));
                let lhl = len * 0.5 * sh + h;
                if lhl > hl {
                    hl = lhl;
                    c = mid;
                    axis = la;
                    side = ls;
                }
            }
        }
        // Fade in over its first tenth of a second, out over the last fifth of its life.
        dim *= (p.age / 0.1).clamp(0.0, 1.0) * (heat / 0.2).clamp(0.0, 1.0);
        let t2 = [heat, hl / h, p.age, now];
        self.quad(c, axis, side, hl, h, false, k as f32, rnd(p.seed), v3(1.0, 1.0, 1.0), cfg.brightness[k] * dim, t2);
    }

    fn vent(&mut self, cam: &Cam, cfg: &Cfg, vt: &VentAt, now: f32) {
        let k = vt.kind as usize;
        let px = cam.px * (vt.pos - cam.eye).len();
        let fl = flicker(vt.seed, cfg.flicker[k]);
        let mut t2 = [1.0, 0.0, 0.0, now];
        // The jet core: from the vent along its axis, narrowing.
        if cfg.jet[k] > 0.0 {
            let len = vt.speed * cfg.jet_len[k];
            let hw = (vt.size * cfg.jet_width[k]).max(cfg.min_px[k] * px * 0.5);
            let (axis, side, show) = Self::screen_axis(cam, vt.pos, vt.dir);
            let hl = len * show * 0.5;
            if hl > hw * 0.5 {
                let c = vt.pos + axis * hl;
                t2[1] = hl * 2.0 / hw;
                self.quad(c, axis, side, hl, hw, true, 3.0, rnd(vt.seed.wrapping_add(1)), cfg.glow_col[k], cfg.jet[k] * fl, t2);
            }
        }
        // The glow where it leaves the hull, pulled towards the camera so the hull it
        // sits in does not hide it.
        if cfg.glow[k] > 0.0 {
            let h = (vt.size * cfg.glow_size[k]).max(cfg.glow_min_px[k] * px * 0.5);
            let to = (cam.eye - vt.pos).norm();
            let c = vt.pos + to * (h * cfg.pull);
            t2[1] = 1.0;
            self.quad(c, cam.right, cam.up, h, h, false, 2.0, rnd(vt.seed.wrapping_add(2)), cfg.glow_col[k], cfg.glow[k] * fl, t2);
        }
    }
}
