//! Where every node of a SOD is, this frame, for one instance: its animation channels
//! played on the sim clock and its level of detail chosen, as the engine's
//! `ST3D_Instance::RenderInternal` does (`spec-chunks-anim-lod.md`, sections 2 and 4),
//! but per instance, with nothing written into shared data.
use crate::engine::{self, field, field_u8, rd, storm3d};
use crate::math::{v3, M34, V3};
use a2fmt::math::Matrix34;
use a2fmt::sod::{ChannelKeys, Role, Sod};
use alloc::vec::Vec;

pub fn m34(m: &Matrix34) -> M34 {
    let a = &m.m;
    M34 { r: [v3(a[0], a[1], a[2]), v3(a[3], a[4], a[5]), v3(a[6], a[7], a[8])], p: v3(a[9], a[10], a[11]) }
}

/// An instance's animation clock: the time since its start (0 when frozen at the
/// first key) and its play flags (bit 0 loop, bit 2 reverse).
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub elapsed: f32,
    pub flags: u32,
}

impl Clock {
    /// Looping from sim time 0: what effects and instances without a game object get.
    pub fn looping(now: f32) -> Clock {
        Clock { elapsed: now.max(0.0), flags: 3 }
    }
    /// Frozen on the first key.
    pub const FROZEN: Clock = Clock { elapsed: 0.0, flags: 3 };
    /// From an `ST3D_Instance`: start time `+0x18`, flags `+0xc`, animating `+0x76`.
    pub fn of_instance(inst: usize, now: f32) -> Clock {
        if inst == 0 {
            return Clock::looping(now);
        }
        let flags = field(inst, 0xc) as u32;
        if field_u8(inst, 0x76) == 0 {
            return Clock { elapsed: 0.0, flags };
        }
        let start = unsafe { rd::<f32>(inst + 0x18) };
        Clock { elapsed: (now - start).max(0.0), flags }
    }
}

/// The key a channel of `n` evenly spaced keys over `period` shows at time `t`, and
/// the fraction towards the next (`ST3D_AnimationChannel::Evaluate`, with its +0.05 s
/// in the divisor). `None` for the fraction when a play-once channel has finished
/// (it holds its last key, or its first when reversed).
pub fn key_at(t: f32, period: f32, n: usize, flags: u32) -> (usize, Option<f32>) {
    if n == 0 || !(period > 0.0) {
        return (0, None);
    }
    if flags & 1 == 0 && t >= period {
        return (if flags & 4 != 0 { 0 } else { n - 1 }, None);
    }
    let mut local = t - libm::floorf(t / period) * period;
    if flags & 4 != 0 {
        local = period - local;
    }
    let k = (libm::floorf(local * n as f32 / (period + 0.05)) as usize).min(n - 1);
    let seg = period / n as f32;
    (k, Some((local - k as f32 * seg) / seg))
}

/// What a SOD needs worked out once: which node each channel drives, and each LOD
/// switch's levels and bounding sphere.
pub struct SodInfo {
    /// For each channel, its node, or None (unknown, or a hardpoint: never animated).
    pub channel_node: Vec<Option<usize>>,
    pub lods: Vec<LodSwitch>,
}

pub struct LodSwitch {
    pub node: usize,
    /// Its `lod*` children, coarsest first.
    pub levels: Vec<usize>,
    /// The bounding sphere of its subtree in the switch node's space.
    pub centre: V3,
    pub radius: f32,
}

impl SodInfo {
    pub fn new(sod: &Sod) -> SodInfo {
        let channel_node = sod
            .channels
            .iter()
            .map(|c| sod.find(&c.node).filter(|&i| !matches!(sod.nodes[i].role(), Role::Hardpoint(_))))
            .collect();
        let mut lods = Vec::new();
        for (i, n) in sod.nodes.iter().enumerate() {
            if n.role() != Role::Geometry {
                continue;
            }
            let levels = sod.lod_levels(i);
            let (centre, radius) = subtree_sphere(sod, i);
            lods.push(LodSwitch { node: i, levels, centre, radius });
        }
        SodInfo { channel_node, lods }
    }
}

/// A sphere around every mesh vertex under `root`, in `root`'s space (static pose).
fn subtree_sphere(sod: &Sod, root: usize) -> (V3, f32) {
    let mut lo = v3(f32::MAX, f32::MAX, f32::MAX);
    let mut hi = v3(f32::MIN, f32::MIN, f32::MIN);
    let mut pts: Vec<V3> = Vec::new();
    let mut stack = alloc::vec![(root, M34::IDENTITY)];
    let mut guard = 0;
    while let Some((i, m)) = stack.pop() {
        guard += 1;
        if guard > sod.nodes.len() * 2 {
            break;
        }
        if let Some(mesh) = sod.nodes[i].mesh() {
            for p in &mesh.positions {
                let w = m.transform_point(v3(p[0], p[1], p[2]));
                lo = v3(lo.x.min(w.x), lo.y.min(w.y), lo.z.min(w.z));
                hi = v3(hi.x.max(w.x), hi.y.max(w.y), hi.z.max(w.z));
                pts.push(w);
            }
        }
        for &c in &sod.nodes[i].children {
            stack.push((c, m34(&sod.nodes[c].transform).then(&m)));
        }
    }
    if pts.is_empty() {
        return (V3::ZERO, 0.0);
    }
    let c = (lo + hi) * 0.5;
    let r = pts.iter().map(|p| (*p - c).len()).fold(0.0f32, f32::max);
    (c, r)
}

/// The engine's level-of-detail settings: the detail level and the four switch
/// distances (`ST3D_GraphicsEngine_Configuration` at `Storm3D+0x104`: `+0x40`, and
/// `+0x30..+0x3c`).
#[derive(Clone, Copy, Debug)]
pub struct LodSettings {
    pub detail: u32,
    pub dist: [f32; 4],
}

impl LodSettings {
    pub fn read() -> LodSettings {
        let cfg = field(storm3d(), 0x104);
        if cfg == 0 {
            return LodSettings { detail: 4, dist: [700.0, 300.0, 150.0, 75.0] };
        }
        unsafe {
            LodSettings {
                detail: rd::<u32>(cfg + 0x40),
                dist: [rd(cfg + 0x30), rd(cfg + 0x34), rd(cfg + 0x38), rd(cfg + 0x3c)],
            }
        }
    }
    /// The level `k` (0 = coarsest wanted) at distance `d` from the sphere's surface.
    pub fn level(&self, d: f32) -> usize {
        let k = if d < self.dist[3] {
            4
        } else if d < self.dist[2] {
            3
        } else if d < self.dist[1] {
            2
        } else if d < self.dist[0] {
            1
        } else {
            0
        };
        k.min(self.detail as usize)
    }
}

/// Every node's world transform and whether it is drawn.
pub struct Pose {
    pub world: Vec<M34>,
    /// False under a LOD level that is not the one chosen.
    pub shown: Vec<bool>,
}

/// Which LOD level to show: chosen from the camera, or forced (an index into the
/// sorted levels; `BuildingEffect` forces 0, the coarsest).
#[derive(Clone, Copy)]
pub enum Lod {
    Auto { eye: V3, settings: LodSettings },
    Forced(usize),
}

/// Poses `sod` placed at `object` (its world transform) at clock `clock`.
pub fn pose(sod: &Sod, info: &SodInfo, object: &M34, clock: Clock, lod: Lod) -> Pose {
    let n = sod.nodes.len();
    let mut local: Vec<M34> = sod.nodes.iter().map(|nd| m34(&nd.transform)).collect();
    let mut scale: Vec<f32> = alloc::vec![1.0; n];
    for (ci, ch) in sod.channels.iter().enumerate() {
        let Some(node) = info.channel_node.get(ci).copied().flatten() else { continue };
        let (k, _) = key_at(clock.elapsed, ch.period, ch.key_count(), clock.flags);
        match &ch.keys {
            ChannelKeys::Matrix(keys) => {
                if let Some(m) = keys.get(k) {
                    local[node] = m34(m);
                }
            }
            ChannelKeys::Scale(keys) => {
                if let Some(s) = keys.get(k) {
                    scale[node] = *s;
                }
            }
        }
    }
    for i in 0..n {
        if scale[i] != 1.0 {
            for r in 0..3 {
                local[i].r[r] = local[i].r[r] * scale[i];
            }
        }
    }
    // World transforms, parents first (file order does not guarantee it).
    let mut world: Vec<Option<M34>> = alloc::vec![None; n];
    for i in 0..n {
        resolve(sod, i, &local, object, &mut world, 0);
    }
    let world: Vec<M34> = world.into_iter().map(|w| w.unwrap_or(*object)).collect();
    // Levels of detail.
    let mut shown = alloc::vec![true; n];
    for sw in &info.lods {
        if sw.levels.is_empty() {
            continue;
        }
        let pick = match lod {
            Lod::Forced(i) => i,
            Lod::Auto { eye, settings } => {
                let w = &world[sw.node];
                let c = w.transform_point(sw.centre);
                let s = w.r[0].len().max(w.r[1].len()).max(w.r[2].len());
                settings.level((c - eye).len() - sw.radius * s)
            }
        }
        .min(sw.levels.len() - 1);
        for (li, &lv) in sw.levels.iter().enumerate() {
            if li != pick {
                hide(sod, lv, &mut shown);
            }
        }
    }
    Pose { world, shown }
}

fn resolve(sod: &Sod, i: usize, local: &[M34], object: &M34, world: &mut [Option<M34>], depth: usize) -> M34 {
    if let Some(w) = world[i] {
        return w;
    }
    let parent = match sod.nodes[i].parent {
        Some(p) if depth < sod.nodes.len() => resolve(sod, p, local, object, world, depth + 1),
        _ => *object,
    };
    let w = local[i].then(&parent);
    world[i] = Some(w);
    w
}

fn hide(sod: &Sod, root: usize, shown: &mut [bool]) {
    let mut stack = alloc::vec![root];
    let mut guard = 0;
    while let Some(i) = stack.pop() {
        guard += 1;
        if guard > sod.nodes.len() {
            break;
        }
        shown[i] = false;
        stack.extend_from_slice(&sod.nodes[i].children);
    }
}

/// The detail level's LOD settings and the eye, for `Lod::Auto`.
pub fn auto(eye: V3) -> Lod {
    Lod::Auto { eye, settings: LodSettings::read() }
}

/// Sim time, for clocks.
pub fn now() -> f32 {
    engine::sim_time()
}
