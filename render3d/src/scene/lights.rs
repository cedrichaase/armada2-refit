//! The lights the engine registered for this view (`ST3D_GraphicsEngine::RegisterLight`
//! from `GameObject_PreRenderAll` and `Ordnance::PreRenderAll`; `Storm3D+0x60`, a
//! `std::list` of records: `+0` the light, `+4` its colour, `+0x10` its transform).
//! Plugins add theirs the same way (shields, repairs, venting), so every light source
//! in the game is in this one list. Layouts as `lighting/README.md` established them.
use crate::engine::{field, rd, storm3d, vtable};
use crate::math::{M34, V3};
use alloc::vec::Vec;

const VT_DIRECTIONAL: usize = 0x6bc8fc; // ST3D_Directional_Light
const VT_POINT: usize = 0x6bc8ac; // ST3D_Point_Light

#[derive(Clone, Copy, Debug)]
pub struct Directional {
    /// The way the light travels (the record transform's third axis), unit length.
    pub dir: V3,
    pub colour: V3,
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub pos: V3,
    pub colour: V3,
    /// Full colour out to `start`, then linearly down to nothing at `start + range`.
    pub start: f32,
    pub range: f32,
    /// The light object (to tell a plugin's own lights apart, if a module must).
    pub light: usize,
}

#[derive(Default)]
pub struct Lights {
    pub dirs: Vec<Directional>,
    pub points: Vec<Point>,
}

impl Lights {
    pub fn gather() -> Lights {
        let mut l = Lights::default();
        let s = storm3d();
        let head = field(s, 0x60);
        if head == 0 {
            return l;
        }
        let mut node = field(head, 0);
        let mut guard = 0;
        while node != head && node != 0 && guard < 4096 {
            guard += 1;
            let rec = field(node, 8);
            node = field(node, 0);
            let light = field(rec, 0);
            if rec == 0 || light == 0 {
                continue;
            }
            let colour: V3 = unsafe { rd(rec + 4) };
            let m: M34 = unsafe { rd(rec + 0x10) };
            match vtable(light) {
                VT_DIRECTIONAL => {
                    let d = m.r[2];
                    if d.len() > 1e-6 {
                        l.dirs.push(Directional { dir: d.norm(), colour });
                    }
                }
                VT_POINT => {
                    let (start, range) = unsafe { (rd::<f32>(light + 0x100), rd::<f32>(light + 0x104)) };
                    l.points.push(Point { pos: m.p, colour, start, range, light });
                }
                // Another kind (a plugin's own vtable for a light the CPU path must not
                // take, as Lighting.asi's planet glow): read as a point light.
                _ => {
                    let (start, range) = unsafe { (rd::<f32>(light + 0x100), rd::<f32>(light + 0x104)) };
                    if start.is_finite() && range.is_finite() && start >= 0.0 && range >= 0.0 && start + range < 1.0e6 {
                        l.points.push(Point { pos: m.p, colour, start, range, light });
                    }
                }
            }
        }
        l
    }

    /// The point lights that reach a sphere, strongest first, at most `max`.
    pub fn points_near(&self, centre: V3, radius: f32, max: usize) -> Vec<Point> {
        let mut v: Vec<(f32, Point)> = self
            .points
            .iter()
            .filter_map(|p| {
                let d = ((p.pos - centre).len() - radius).max(0.0);
                if d >= p.start + p.range {
                    return None;
                }
                let f = if d <= p.start { 1.0 } else { 1.0 - (d - p.start) / p.range.max(1e-3) };
                let peak = p.colour.x.max(p.colour.y).max(p.colour.z);
                Some((f * peak, *p))
            })
            .collect();
        v.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(max);
        v.into_iter().map(|e| e.1).collect()
    }

    /// The brightest directional light (the map's Key).
    pub fn key(&self) -> Option<Directional> {
        self.dirs
            .iter()
            .copied()
            .max_by(|a, b| (a.colour.x + a.colour.y + a.colour.z).partial_cmp(&(b.colour.x + b.colour.y + b.colour.z)).unwrap_or(core::cmp::Ordering::Equal))
    }
}
