//! The shots' light, at the start of each view (after `Ordnance::PreRenderAll` has
//! registered every torpedo's and pulse's own light):
//!
//! - **Recolouring** (from `Lighting.asi`):
//!   an ODF's `lightColor` ignores its projectile (every Federation photon is cyan,
//!   nearly every other torpedo and pulse green). The class's light (class +0x10, an
//!   `ST3D_Point_Light`, its colour +0xf4) takes the hue of its sprite instead: the mean
//!   of the sprite's first frame (its rectangle +0x38 of its texture +0x58) of the
//!   installed TGA, scaled to the ODF colour's peak. Written into this view's registry
//!   record (Storm3D +0x60: record +0 the light, +4 its colour), not into the class.
//! - **Phasers** (also `Lighting.asi`'s): a light at the emitter and one at the impact
//!   of every visible phaser (vtable 0x6b8ee4), in the beam's own colour (its texture's
//!   mean, scaled to a peak of 1, times its tint +0xf0), up in 0.06 s and down over the
//!   last 0.25 s of its life (time left +0xac of the class's lifeSpan +0x1c).
//! - **Muzzles** (new in render3d): a torpedo or pulse lights its own ship where it
//!   leaves it. The engine's light rides with the shot (an ODF's `lightFalloffStart` +
//!   `Range`, 55 on every pulse), which is clear of the hull in a frame or two, so the
//!   firing ship took nothing of its own discharge. A shot seen for the first time (not
//!   in the last main view's list) leaves a light at that point, kept at the same
//!   place on its owner (owner +0x5c, its position +0xac), in the shot's light colour
//!   (recoloured as above), reaching `MuzzleReach` times the owner's bounding radius,
//!   down over `MuzzleTime` seconds of the sim clock.
//! - **The special weapons and the warp flash** (Specials.asi's): `fx.rs` says where.
//!
//! The new lights are ordinary `ST3D_Point_Light`s (made with the engine's own
//! constructor, kept for the session) registered with `RegisterLight`, so a hull the
//! engine draws takes them as it takes a torpedo's; then the scene's list is read
//! again for ours. With `Lighting.asi` loaded, it recolours and lights phasers itself
//! and this module leaves both to it.
use super::hooks;
use r3d_core::engine;
use r3d_core::assets;
use r3d_core::math::{v3, M34, V3};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use r3d_core::engine::light::{fn_new, fn_point_light, fn_register_light};

pub struct Lights {
    pool: Vec<usize>,
    /// Per texture (lower-case stem): the mean colour of a rectangle (u, v, w, h in
    /// thousandths), or None if it could not be read.
    means: BTreeMap<(String, [i32; 4]), Option<V3>>,
    pub recolour: bool,
    pub phasers: bool,
    /// Phaser lights: emitter brightness, start, range, lift; impact brightness,
    /// start, range, lift (Lighting.ini's defaults).
    pub ph: [f32; 8],
    /// Muzzle lights: brightness, seconds, reach in the owner's radii (0 brightness: none).
    pub muzzle: [f32; 3],
    /// The torpedoes and pulses of the last main view, and the muzzles still lit.
    shots_seen: Vec<usize>,
    muzzles: Vec<Muzzle>,
    said: u32,
    said_muzzle: u32,
    /// This view's lights go to the clusters (`emit`), and the points kept for them.
    cl: bool,
    points: Vec<r3d_core::scene::lights::Point>,
    /// The beams' lights with clustered lights (`[Lights] Beam*=`, `ConeSpot=`).
    pub beam: Beam,
}

/// A torpedo's or pulse's own light to the gas: a bolt (`[Nebulae] LitBolt=`).
static BOLT: r3d_core::scene::lights::Medium = r3d_core::scene::lights::Medium::new("bolt", r3d_core::scene::media::BOLT);

/// `[Lights]` of Render3D.ini, the beams' part (README.md, "Clustered lights",
/// "Beams"): a phaser beam as a line light (`scene::clusters::add_line`), and a tractor
/// or assimilation beam's wash as a spot in its cone (`ConeSpot=`, else a point).
/// Read by `configure`.
pub struct Beam {
    pub line: r3d_core::scene::clusters::LineParams,
    pub cone_spot: bool,
}

/// `BeamLight`'s default.
pub const BEAM_LIGHT: f32 = 4.0;

impl Default for Beam {
    /// What `read` gives an ini without the keys.
    fn default() -> Beam {
        Beam {
            line: r3d_core::scene::clusters::LineParams { light: BEAM_LIGHT, core: 8.0, reach: 200.0, wrap: 0.4, gas_reach: 40.0 },
            cone_spot: true,
        }
    }
}

impl Beam {
    pub fn read(ini: &crate::rt::Ini) -> Beam {
        let s = "Lights";
        Beam {
            line: r3d_core::scene::clusters::LineParams {
                light: ini.float(s, "BeamLight", BEAM_LIGHT).max(0.0),
                core: ini.float(s, "BeamCore", 8.0).max(0.5),
                reach: ini.float(s, "BeamReach", 200.0).max(1.0),
                wrap: ini.float(s, "BeamWrap", 0.4).clamp(0.0, 1.0),
                gas_reach: ini.float(s, "BeamGasReach", 40.0).max(1.0),
            },
            cone_spot: ini.int(s, "ConeSpot", 1) != 0,
        }
    }
}

/// Where a shot left its ship: its owner's handle, the point's offset from the
/// owner's position, its colour, when, and its reach.
struct Muzzle {
    owner: i32,
    off: V3,
    col: V3,
    t0: f32,
    reach: f32,
}

impl Lights {
    pub fn new() -> Lights {
        Lights {
            pool: Vec::new(),
            means: BTreeMap::new(),
            recolour: true,
            phasers: true,
            ph: [4.5, 0.0, 24.0, 3.0, 2.0, 6.0, 70.0, 8.0],
            muzzle: [4.0, 0.5, 1.0],
            shots_seen: Vec::new(),
            muzzles: Vec::new(),
            said: 0,
            said_muzzle: 0,
            cl: false,
            points: Vec::new(),
            beam: Beam::default(),
        }
    }

    fn light(&mut self, n: usize) -> usize {
        while self.pool.len() <= n {
            let l = unsafe {
                let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(fn_new());
                let l = new(0x138);
                if l == 0 {
                    return 0;
                }
                let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) = core::mem::transmute(fn_point_light());
                ctor(l, 0, 0, b"ordnance\0".as_ptr());
                l
            };
            self.pool.push(l);
        }
        self.pool[n]
    }

    /// Registers a point light with the engine for this view: full to `start`, gone
    /// `range` further.
    fn register(&mut self, n: usize, pos: V3, col: V3, start: f32, range: f32) {
        let l = self.light(n);
        if l == 0 {
            return;
        }
        unsafe {
            core::ptr::write_unaligned((l + 0x100) as *mut f32, start);
            core::ptr::write_unaligned((l + 0x104) as *mut f32, range.max(0.01));
            let m = M34 { p: pos, ..M34::IDENTITY };
            let c = [col.x, col.y, col.z, 1.0f32];
            let f: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(fn_register_light());
            f(engine::storm3d(), l, c.as_ptr(), &m);
        }
    }

    /// A light of this module's for the view: with clustered lights (`scene::clusters`)
    /// a point of the scene's own list, kept here until the list is read again
    /// (`take_points`), which spares the engine a light object and its registry a
    /// record; else pool light `n` registered with the engine and styled, and `n`
    /// moves on.
    fn emit(&mut self, n: &mut usize, pos: V3, col: V3, start: f32, range: f32, style: r3d_core::scene::lights::Style) {
        if self.cl {
            self.points.push(r3d_core::scene::lights::Point { pos, colour: col, start, range: range.max(0.01), light: 0, style, medium: r3d_core::scene::lights::MediumId::PLAIN });
            return;
        }
        self.register(*n, pos, col, start, range);
        self.style(*n, style);
        *n += 1;
    }

    /// Switched off: the muzzles still lit and the shots remembered go (the engine's light
    /// objects of the pool are kept).
    pub fn drop_state(&mut self) {
        self.shots_seen.clear();
        self.muzzles.clear();
        self.points.clear();
    }

    /// The points `emit` kept for the scene's list (after the module read it again).
    pub fn take_points(&mut self) -> Vec<r3d_core::scene::lights::Point> {
        core::mem::take(&mut self.points)
    }

    /// How the lit draws take pool light `n` (render3d's hulls: as Lighting.asi took
    /// its phaser lights).
    fn style(&self, n: usize, s: r3d_core::scene::lights::Style) {
        if let Some(&l) = self.pool.get(n) {
            r3d_core::scene::lights::set_style(l, s);
        }
    }

    /// The mean colour (0..1) of a rectangle of a texture, from the TGA.
    fn mean(&mut self, tex: &str, r: [f32; 4]) -> Option<V3> {
        let key = (crate::rt::lower(tex), [(r[0] * 1000.0) as i32, (r[1] * 1000.0) as i32, (r[2] * 1000.0) as i32, (r[3] * 1000.0) as i32]);
        if let Some(m) = self.means.get(&key) {
            return *m;
        }
        let m = assets::image(tex).and_then(|img| {
            let (w, h) = (img.width as f32, img.height as f32);
            let x0 = ((r[0] * w + 0.5) as i32).max(0) as u32;
            let y0 = ((r[1] * h + 0.5) as i32).max(0) as u32;
            let x1 = (((r[0] + r[2]) * w + 0.5) as u32).min(img.width);
            let y1 = (((r[1] + r[3]) * h + 0.5) as u32).min(img.height);
            if x1 <= x0 || y1 <= y0 {
                return None;
            }
            let mut acc = [0f64; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = img.pixel(x, y);
                    acc[0] += p[2] as f64;
                    acc[1] += p[1] as f64;
                    acc[2] += p[0] as f64;
                }
            }
            let n = ((x1 - x0) * (y1 - y0)) as f64 * 255.0;
            Some(v3((acc[0] / n) as f32, (acc[1] / n) as f32, (acc[2] / n) as f32))
        });
        crate::log!("ordnance: light colour of {} {:?}: {:?}", tex, r, m);
        self.means.insert(key, m);
        m
    }

    /// A sprite's texture (its stem) and its first frame's rectangle.
    fn sprite_rect(sprite: usize) -> Option<(String, [f32; 4])> {
        let name = hooks::sprite_texture_name(sprite);
        if name.is_empty() {
            return None;
        }
        let base = name.rsplit(['\\', '/']).next().unwrap_or(name);
        let stem = base.split('.').next().unwrap_or(base);
        Some((String::from(stem), hooks::sprite_rect(sprite)))
    }

    /// A torpedo's or pulse's light colour: its class light's ODF colour (+0xf4), in
    /// its sprite's hue when recolouring (as `recolour_registry`).
    fn shot_colour(&mut self, l: usize, sprite: usize) -> V3 {
        let odf = hooks::light_colour(l);
        if !self.recolour {
            return odf;
        }
        let peak = odf.x.max(odf.y).max(odf.z);
        let Some((tex, r)) = Self::sprite_rect(sprite) else { return odf };
        let Some(m) = self.mean(&tex, r) else { return odf };
        let mp = m.x.max(m.y).max(m.z);
        if peak <= 0.0 || mp <= 0.0 {
            return odf;
        }
        m * (peak / mp)
    }

    /// Notes the torpedoes and pulses that are new since the last main view as muzzles,
    /// drops the muzzles that are out, and registers the rest; returns how many lights
    /// were used.
    fn muzzle_lights(&mut self, mut n: usize, now: f32, main: bool) -> usize {
        let (bright, time, reach) = (self.muzzle[0], self.muzzle[1].max(0.01), self.muzzle[2]);
        if main {
            let mut now_seen: Vec<usize> = Vec::new();
            let mut new: Vec<(usize, usize, usize)> = Vec::new();
            hooks::for_each_ordnance(|o| {
                if hooks::is_phaser(o) {
                    return;
                }
                let l = hooks::shot_point_light(o);
                if l == 0 {
                    return;
                }
                now_seen.push(o);
                new.push((o, l, hooks::shot_sprite(o)));
            });
            for (o, l, spr) in new {
                if self.shots_seen.contains(&o) || self.muzzles.len() >= 64 {
                    continue;
                }
                let h = hooks::shot_owner_handle(o);
                let owner = hooks::object_of(h);
                if owner == 0 {
                    continue;
                }
                let pos = hooks::transform(o).p;
                let op = hooks::obj_pos(owner);
                let rad = hooks::radius_of(owner);
                let col = self.shot_colour(l, spr);
                if self.said_muzzle < 16 {
                    self.said_muzzle += 1;
                    crate::log!(
                        "ordnance: muzzle of {} on {} at {:.0} from its centre (radius {:.0}), colour {:.2} {:.2} {:.2}",
                        hooks::ord_odf(o), hooks::odf_name(owner), (pos - op).len(), rad, col.x, col.y, col.z
                    );
                }
                self.muzzles.push(Muzzle { owner: h, off: pos - op, col, t0: now, reach: rad * reach });
            }
            self.shots_seen = now_seen;
            self.muzzles.retain(|m| now - m.t0 < time && now >= m.t0 - 1.0);
        }
        if bright <= 0.0 {
            return n;
        }
        let lit: Vec<(V3, V3, f32)> = self
            .muzzles
            .iter()
            .filter_map(|m| {
                let owner = hooks::object_of(m.owner);
                if owner == 0 {
                    return None;
                }
                let k = 1.0 - ((now - m.t0) / time).clamp(0.0, 1.0);
                if k <= 0.0 {
                    return None;
                }
                Some((hooks::obj_pos(owner) + m.off, m.col * (bright * k), m.reach))
            })
            .collect();
        for (p, c, r) in lit {
            if n >= 112 {
                break;
            }
            self.emit(&mut n, p, c, r * 0.1, r * 0.9, r3d_core::scene::lights::Style { hard: true, distinct: true, wrap: 0.4, power: 2.0 });
        }
        n
    }

    /// This view's torpedo and pulse lights in their sprites' colours.
    fn recolour_registry(&mut self) {
        // The ordnance classes' lights in play, with their sprites.
        let mut ls: Vec<(usize, usize)> = Vec::new();
        hooks::for_each_ordnance(|o| {
            let l = hooks::shot_point_light(o);
            if l != 0 && !ls.iter().any(|e| e.0 == l) {
                ls.push((l, hooks::shot_sprite(o)));
            }
        });
        if ls.is_empty() {
            return;
        }
        hooks::for_each_light_record(|rec, l| {
            let Some(&(_, sprite)) = ls.iter().find(|e| e.0 == l) else { return };
            let odf = hooks::light_colour(l);
            let peak = odf.x.max(odf.y).max(odf.z);
            let Some((tex, r)) = Self::sprite_rect(sprite) else { return };
            let Some(m) = self.mean(&tex, r) else { return };
            let mp = m.x.max(m.y).max(m.z);
            if peak <= 0.0 || mp <= 0.0 {
                return;
            }
            let c = m * (peak / mp);
            if self.said < 16 {
                self.said += 1;
                crate::log!("ordnance: light of {} recoloured {:.2} {:.2} {:.2} -> {:.2} {:.2} {:.2}", tex, odf.x, odf.y, odf.z, c.x, c.y, c.z);
            }
            hooks::set_record_colour(rec, c);
        });
    }

    /// The phasers' emitter and impact lights; returns how many lights were used.
    fn phaser_lights(&mut self, mut n: usize) -> usize {
        let mut shots: Vec<(V3, V3, V3, usize)> = Vec::new();
        hooks::for_each_ordnance(|o| {
            if !hooks::is_phaser(o) || !hooks::phaser_active(o) {
                return;
            }
            let cls = hooks::shot_class(o);
            if cls == 0 {
                return;
            }
            let (left, len) = (hooks::shot_life_left(o), hooks::class_life(cls));
            if left <= 0.0 || len <= 0.0 {
                return;
            }
            let age = len - left;
            let mut k = if age < 0.06 { age / 0.06 } else { 1.0 };
            if left < 0.25 {
                k *= left / 0.25;
            }
            if k <= 0.0 {
                return;
            }
            let (a, b, tint) = (hooks::shot_start(o), hooks::shot_end(o), hooks::shot_tint(o));
            let t = v3(tint.x.clamp(0.0, 1.0), tint.y.clamp(0.0, 1.0), tint.z.clamp(0.0, 1.0)) * k;
            shots.push((a, b, t, hooks::class_sprite(cls)));
        });
        for (a, b, t, spr) in shots {
            if n >= 96 {
                break;
            }
            let col = Self::sprite_rect(spr)
                .and_then(|(tex, _)| self.mean(&tex, [0.0, 0.0, 1.0, 1.0]))
                .map(|m| {
                    let p = m.x.max(m.y).max(m.z);
                    if p > 1e-3 { m * (1.0 / p) } else { v3(1.0, 1.0, 1.0) }
                })
                .unwrap_or(v3(1.0, 1.0, 1.0));
            let c = col.mul_comp(t);
            let d = b - a;
            let len = d.len();
            if len < 1e-3 {
                continue;
            }
            let ph = self.ph;
            if ph[0] > 0.0 {
                let p = a + d * (len.min(ph[3]) / len);
                self.emit(&mut n, p, c * ph[0], ph[1], ph[2] - ph[1], r3d_core::scene::lights::Style { hard: true, distinct: true, wrap: 0.6, power: 2.0 });
            }
            if ph[4] > 0.0 {
                let p = b - d * (len.min(ph[7]) / len);
                self.emit(&mut n, p, c * ph[4], ph[5], ph[6] - ph[5], r3d_core::scene::lights::Style { hard: true, distinct: true, wrap: 0.0, power: 1.0 });
            }
            if self.cl {
                // The beam itself, a line light, even along its length, lighting the
                // plating, rocks and gas it passes.
                r3d_core::scene::clusters::add_line(a, b, c, &self.beam.line);
            }
        }
        n
    }

    /// The start of a view: recolour, then register the new lights. True if the
    /// registry changed (the scene's list must be read again).
    pub fn begin_view(&mut self, fx: &[super::fx::Lit], now: f32, main: bool) -> bool {
        let lighting = r3d_core::plugin_loaded("Lighting.asi");
        self.cl = r3d_core::scene::clusters::on();
        self.points.clear();
        let mut changed = false;
        // The shots' own lights (each class's, riding with its shots) are bolts to the
        // gas (`[Nebulae] LitBolt=`): a corona about the shot, not a lit ball.
        if self.cl {
            hooks::for_each_ordnance(|o| {
                let l = hooks::shot_point_light(o);
                if l != 0 {
                    r3d_core::scene::lights::set_medium(l, BOLT.id());
                }
            });
        }
        if self.recolour && !lighting {
            self.recolour_registry();
            changed = true;
        }
        let mut n = 0;
        if self.phasers && !lighting {
            n = self.phaser_lights(n);
        }
        if self.muzzle[0] > 0.0 {
            n = self.muzzle_lights(n, now, main);
        }
        let spot = self.beam.cone_spot;
        for l in fx {
            if n >= 128 {
                break;
            }
            match (self.cl && spot, l.cone) {
                // A tractor's or an assimilation beam's wash: the cone's footprint.
                (true, Some(c)) => r3d_core::scene::clusters::add_spot(
                    c.apex,
                    c.axis,
                    l.col,
                    &r3d_core::scene::clusters::SpotParams { reach: c.reach, hw0: c.hw0, slope: c.slope, fade: c.reach * 0.15, wrap: 0.2 },
                ),
                _ => self.emit(&mut n, l.pos, l.col, l.start, l.range, r3d_core::scene::lights::Style::default()),
            }
        }
        changed || n > 0
    }
}
