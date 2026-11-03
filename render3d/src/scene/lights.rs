//! The lights the engine registered for this view (`ST3D_GraphicsEngine::RegisterLight`
//! from `GameObject_PreRenderAll` and `Ordnance::PreRenderAll`; `Storm3D+0x60`, a
//! `std::list` of records: `+0` the light, `+4` its colour, `+0x10` its transform).
//! Plugins add theirs the same way (shields, repairs, venting), so every light source
//! in the game is in this one list. Layouts as `Lighting.asi` established them.
use crate::engine::{self, field, rd, storm3d, vtable};
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
    /// How the module that registered it wants it taken (`set_style`), else default.
    pub style: Style,
    /// What it is to the nebulae's gas (`set_medium`; a point added by hand,
    /// `scene::add_point`'s).
    pub medium: MediumId,
}

/// How a light a module registers is to be taken by the lit draws (as `Lighting.asi`
/// took its own phaser lights, a phaser's two lights): placed as a hard light
/// whatever its falloff, never merged with others of its colour (several banks fire at
/// once), its falloff raised to `power` and wrapped `wrap` round the plating it
/// grazes. Default: none of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub hard: bool,
    pub distinct: bool,
    pub wrap: f32,
    pub power: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style { hard: false, distinct: false, wrap: 0.0, power: 1.0 }
    }
}

static mut STYLES: alloc::collections::BTreeMap<usize, Style> = alloc::collections::BTreeMap::new();

/// Marks a light object (an `ST3D_Point_Light` a module made and registers) to be taken
/// so; it stays so for as long as the object lives (a module's pool).
#[allow(static_mut_refs)]
pub fn set_style(light: usize, s: Style) {
    unsafe {
        if s == Style::default() {
            STYLES.remove(&light);
        } else {
            STYLES.insert(light, s);
        }
    }
}

#[allow(static_mut_refs)]
fn style_of(light: usize) -> Style {
    unsafe { STYLES.get(&light).copied().unwrap_or_default() }
}

pub use super::media::{FLAG_SHARE, MediumId, MediumParams, Response, Share};

static mut FLAGS: alloc::collections::BTreeMap<usize, u32> = alloc::collections::BTreeMap::new();

/// Gives a light object a flag (`FLAG_SHARE`); it stays so for as long as the object
/// lives (a module's pool).
#[allow(static_mut_refs)]
pub fn set_flag(light: usize, flag: u32) {
    unsafe { *FLAGS.entry(light).or_insert(0) |= flag };
}

#[allow(static_mut_refs)]
pub fn has_flag(light: usize, flag: u32) -> bool {
    unsafe { FLAGS.get(&light).is_some_and(|f| f & flag != 0) }
}

/// The media modules have named, by slot (`media::MEDIA`: the slot is the response's).
static mut MEDIUM_NAMES: [Option<&'static str>; super::media::SLOTS] = [Some("plain"), None, None, None, None, None, None, None];

/// Names the medium of these parameters (`media.rs`): its id, the same for the same
/// parameters whoever registers it first. Parameters the gas has no response for are
/// taken as plain, logged.
#[allow(static_mut_refs)]
pub fn register_medium(name: &'static str, p: MediumParams) -> MediumId {
    let Some(id) = super::media::slot_of(p) else {
        crate::log!("lights: medium {}: the gas has no response for {:?}; taken as plain", name, p);
        return MediumId::PLAIN;
    };
    unsafe {
        let n = &mut MEDIUM_NAMES[id.0 as usize];
        if n.is_none() {
            *n = Some(name);
        }
    }
    id
}

/// The name a slot was registered under first, if any.
#[allow(static_mut_refs)]
pub fn medium_name(id: MediumId) -> Option<&'static str> {
    unsafe { MEDIUM_NAMES.get(id.0 as usize).copied().flatten() }
}

/// A module's medium, registered the first time it is asked for (a `static` of the
/// module's: `static SPARK: Medium = Medium::new("spark", media::SPARK)`).
pub struct Medium {
    name: &'static str,
    params: MediumParams,
    id: core::sync::atomic::AtomicU8,
}

impl Medium {
    pub const fn new(name: &'static str, params: MediumParams) -> Medium {
        Medium { name, params, id: core::sync::atomic::AtomicU8::new(u8::MAX) }
    }
    pub fn id(&self) -> MediumId {
        use core::sync::atomic::Ordering::Relaxed;
        let v = self.id.load(Relaxed);
        if v != u8::MAX {
            return MediumId(v);
        }
        let id = register_medium(self.name, self.params);
        self.id.store(id.0, Relaxed);
        id
    }
}

static mut MEDIUM: alloc::collections::BTreeMap<usize, MediumId> = alloc::collections::BTreeMap::new();

/// Marks a light object as what it is to the gas; it stays so for as long as the object
/// lives (a module's pool, a class's light, a nebula's).
#[allow(static_mut_refs)]
pub fn set_medium(light: usize, m: MediumId) {
    unsafe {
        if m == MediumId::PLAIN {
            MEDIUM.remove(&light);
        } else {
            MEDIUM.insert(light, m);
        }
    }
}

#[allow(static_mut_refs)]
pub fn medium_of(light: usize) -> MediumId {
    if light == 0 {
        return MediumId::PLAIN;
    }
    unsafe { MEDIUM.get(&light).copied().unwrap_or(MediumId::PLAIN) }
}

#[derive(Default, Clone)]
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
                    l.points.push(Point { pos: m.p, colour, start, range, light, style: style_of(light), medium: medium_of(light) });
                }
                // Another kind (a plugin's own vtable for a light the CPU path must not
                // take, as Lighting.asi's planet glow): read as a point light.
                _ => {
                    let (start, range) = unsafe { (rd::<f32>(light + 0x100), rd::<f32>(light + 0x104)) };
                    if start.is_finite() && range.is_finite() && start >= 0.0 && range >= 0.0 && start + range < 1.0e6 {
                        l.points.push(Point { pos: m.p, colour, start, range, light, style: style_of(light), medium: medium_of(light) });
                    }
                }
            }
        }
        l.replace_scene_lights();
        l
    }

    /// `Lighting.asi`'s scene lights in place of the map's: every directional light the map registered is dropped,
    /// and a warm Key, a dim blue Fill opposite it and a faint light in the sky's own
    /// colour, from its brightest side, take their place. `[Lights]` in Render3D.ini;
    /// nothing is changed while `Lighting.asi` is loaded (it has replaced them in the
    /// engine's list already) or with `Replace=0`. `gather` applies it, so every
    /// reading of the list (a module's re-read after registering lights too) has them.
    pub fn replace_scene_lights(&mut self) {
        let Some(c) = crate::core_services::scene_lights().and_then(|p| p.scene_config()) else { return };
        if !c.replace || self.dirs.is_empty() {
            return;
        }
        self.dirs.clear();
        self.dirs.push(Directional { dir: c.key_axis, colour: c.key });
        self.dirs.push(Directional { dir: c.fill_axis, colour: c.fill });
        if let Some((col, dir)) = sky_light(&c) {
            self.dirs.push(Directional { dir, colour: col });
        }
    }

    /// How much of a light reaches `at`: 1, or for a light with `FLAG_SHARE` (a
    /// planet's day side) the share of it facing `at`, 0 over the far side
    /// (`Lighting.asi`'s `glow_phase`: half of one minus the cosine between the Key's
    /// direction and the way from the light to `at`).
    pub fn share(&self, p: &Point, at: V3) -> f32 {
        if !has_flag(p.light, FLAG_SHARE) {
            return 1.0;
        }
        let Some(k) = self.key() else { return 1.0 };
        let d = at - p.pos;
        let (dl, kl) = (d.len(), k.dir.len());
        if dl < 1e-3 || kl < 1e-6 {
            return 1.0;
        }
        0.5 - 0.5 * d.dot(k.dir) / (dl * kl)
    }

    /// The point lights that reach a sphere, strongest first, at most `max`; one with
    /// `FLAG_SHARE` with its colour times its `share` at the centre.
    pub fn points_near(&self, centre: V3, radius: f32, max: usize) -> Vec<Point> {
        let mut v: Vec<(f32, Point)> = self
            .points
            .iter()
            .filter_map(|p| {
                let d = ((p.pos - centre).len() - radius).max(0.0);
                if d >= p.start + p.range {
                    return None;
                }
                let mut p = *p;
                let sh = self.share(&p, centre);
                if sh <= 0.0 {
                    return None;
                }
                p.colour = p.colour * sh;
                let f = if d <= p.start { 1.0 } else { 1.0 - (d - p.start) / p.range.max(1e-3) };
                let peak = p.colour.x.max(p.colour.y).max(p.colour.z);
                Some((f * peak, p))
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

/// The scene lights (`Lighting.asi`'s `Lights=1` and its settled values): what replaces
/// the map's directional lights. Its keys are the hulls module's (`[Lights]`: `Replace`,
/// `KeyColour`, `KeyAxis`, `FillColour`, `FillAxis`, `SkyLight`); the core only applies
/// what `core_services::scene_lights()` gives, and changes nothing without it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneConfig {
    pub replace: bool,
    pub key: V3,
    pub key_axis: V3,
    pub fill: V3,
    pub fill_axis: V3,
    pub sky: f32,
}

/// The sky's own light: the map's background (`engine::light::background_name`: a
/// prefix of six faces, or a cube SOD), its texels'
/// mean weighted by their chroma, scaled to a peak of `SkyLight`, from the side where
/// that colour is (the faces of a prefix sky face +z, +x, -z, -x, +y, -y); a SOD sky's
/// faces carry no direction, so it falls along the Fill's axis. Read once per map.
#[allow(static_mut_refs)]
fn sky_light(c: &SceneConfig) -> Option<(V3, V3)> {
    static mut SKY: Option<(alloc::string::String, Option<(V3, V3)>)> = None;
    if !(c.sky > 0.0) {
        return None;
    }
    let name = engine::light::background_name();
    unsafe {
        if let Some((n, v)) = SKY.as_ref() {
            if n == name {
                return *v;
            }
        }
    }
    let lname = crate::rt::lower(name);
    let mut acc = [0.0f64; 4];
    let mut dir = [0.0f64; 3];
    let mut faces = 0;
    let scan = |tex: &str, acc: &mut [f64; 4]| -> Option<f64> {
        let img = crate::assets::image(tex)?;
        let before = acc[3];
        for px in img.pixels.chunks_exact(4) {
            let (b, g, r) = (px[0] as f64, px[1] as f64, px[2] as f64);
            let w = r.max(g).max(b) - r.min(g).min(b);
            acc[0] += r * w;
            acc[1] += g * w;
            acc[2] += b * w;
            acc[3] += w;
        }
        Some(acc[3] - before)
    };
    if let Some(sod) = lname.strip_suffix(".sod") {
        if let Some(s) = crate::assets::sod(sod) {
            for n in &s.nodes {
                if let Some(t) = n.mesh().and_then(|m| m.diffuse_texture()) {
                    scan(t, &mut acc);
                }
            }
        }
    } else if !lname.is_empty() {
        const FACE: [[f64; 3]; 6] =
            [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
        for (i, f) in FACE.iter().enumerate() {
            let mut t = lname.clone();
            t.push((b'0' + i as u8) as char);
            if let Some(w) = scan(&t, &mut acc) {
                faces += 1;
                for k in 0..3 {
                    dir[k] += f[k] * w;
                }
            }
        }
    }
    let mut out = None;
    if acc[3] >= 1.0 {
        let col = [acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3]];
        let peak = col[0].max(col[1]).max(col[2]);
        if peak > 0.0 {
            let s = c.sky as f64 / peak;
            let colour = crate::math::v3((col[0] * s) as f32, (col[1] * s) as f32, (col[2] * s) as f32);
            let d = crate::math::v3((-dir[0] / acc[3]) as f32, (-dir[1] / acc[3]) as f32, (-dir[2] / acc[3]) as f32);
            let axis = if faces == 6 && d.len() >= 0.05 { d.norm() } else { c.fill_axis };
            crate::log!("lights: sky {} colour {:?} along {:?}", lname, colour, axis);
            out = Some((colour, axis));
        }
    }
    unsafe { SKY = Some((alloc::string::String::from(name), out)) };
    out
}

/// Engine point lights of our own, made on first use and registered each view (the
/// plugins' way: an ordinary `ST3D_Point_Light` registered after `GameObject_PreRenderAll`).
pub struct Pool {
    name: &'static [u8],
    objs: Vec<usize>,
}

impl Pool {
    pub const fn new(name: &'static [u8]) -> Pool {
        Pool { name, objs: Vec::new() }
    }
    /// Light object `i` of the pool (made by `register`), 0 if none yet; for
    /// `scene::lights::set_style`.
    pub fn light(&self, i: usize) -> usize {
        self.objs.get(i).copied().unwrap_or(0)
    }
    /// Registers light `i` of the pool at `pos`: full colour to `start`, gone `range`
    /// further on. False if no light object could be made.
    pub fn register(&mut self, i: usize, pos: V3, colour: V3, start: f32, range: f32) -> bool {
        while self.objs.len() <= i {
            let l = engine::light::new_point_light(self.name);
            if l == 0 {
                return false;
            }
            self.objs.push(l);
        }
        let l = self.objs[i];
        engine::light::set_reach(l, start, range);
        let col = [colour.x, colour.y, colour.z, 1.0f32];
        let m = M34 { r: M34::IDENTITY.r, p: pos };
        engine::light::register(l, &col, &m);
        true
    }
    /// `register`, the light then of `medium` to the nebulae's gas (a module's spark,
    /// `media::SPARK`: a small corona about it, not a ball its reach wide).
    pub fn register_in(&mut self, i: usize, pos: V3, colour: V3, start: f32, range: f32, medium: MediumId) -> bool {
        if !self.register(i, pos, colour, start, range) {
            return false;
        }
        set_medium(self.objs[i], medium);
        true
    }
}
