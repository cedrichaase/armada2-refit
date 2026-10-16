//! The menu scene: a 3D scene of our own behind the front end's menus (a fleet in orbit
//! over a planet), drawn on request by `UI.asi` through `Render3D_MenuScene`, with the
//! modules' own looks (the sky's recipe, the world's planet, the hulls' lighting,
//! shadows and running lights) and the frame's bloom and tone map. README.md, "The menu
//! scene".
//!
//! Nothing of a mission is read: in the front end there is no game object, no camera
//! and no registered light. The scene is a recipe (`render3d/menus/<name>.ini`, built
//! in; a `Menu\<name>.ini` in the game directory takes precedence and is read again
//! when it changes), posed on a clock of its own, and the view is ours. Every module
//! that draws a part of it does so in `Module::menu`, given the posed `Shot`.
//!
//! The scene lives in the fleet's frame: the fleet holds still with the camera and the
//! sun, and the orbit shows as the planet turning under it and the sky turning about
//! the orbit's axis (+x). Planet centre at the origin; the fleet travels along +z (the
//! way the hulls' noses point), over the planet on +y or where `[Planet] At` says.
use crate::d3d::Device;
use crate::frame::post::PRect;
use crate::frame::steps::View;
use crate::math::{self, M34, M4, V3, v3};
use crate::mods;
use crate::scene::lights::{Directional, Lights};
use crate::{assets, log, rt};
use alloc::string::String;
use alloc::vec::Vec;

/// The recipes built into the plugin, by name.
static BUILT_IN: &[(&str, &str)] = &[
    ("main", include_str!("../../menus/main.ini")),
    ("single", include_str!("../../menus/single.ini")),
];

/// The passes of a menu frame, in order; the depth is cleared before `Ships`, which
/// draw on a projection of their own (the planet is far beyond them).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pass {
    /// The sky, its stars and the sun, no depth.
    Sky,
    /// The planet.
    Planet,
    /// The hulls, opaque.
    Ships,
    /// What blends over the hulls: additive meshes, running lights.
    Translucent,
}

/// A ship of the scene, posed for this frame.
pub struct ShipAt {
    /// Its SOD (the ODF's `baseName`).
    pub sod: String,
    /// Its placement, the ODF's `ScaleSOD` and the recipe's scale folded in.
    pub world: M34,
    pub team: Option<V3>,
}

/// The planet, posed for this frame.
pub struct PlanetAt {
    pub ground: String,
    pub atmo: String,
    /// Unit scale: the radius is apart.
    pub world: M34,
    pub radius: f32,
    /// How far the clouds have turned about the axis beyond the ground.
    pub cloud_yaw: f32,
}

/// The scene at one moment: what the modules draw.
pub struct Shot {
    pub sky: String,
    /// The sky's turn (the orbit): directions are taken through it into the sky's frame.
    pub sky_turn: M34,
    /// The stars (unit directions in the sky's frame, half size in radians, brightness).
    pub stars: Vec<(V3, f32, f32)>,
    /// The sun's ODF (a background object's SOD), the way towards it, its distance.
    pub sun: Option<(String, V3, f32)>,
    pub planet: Option<PlanetAt>,
    pub ships: Vec<ShipAt>,
}

/// One `key=value` text recipe, sections lower-cased.
struct Text {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Text {
    fn parse(t: &str) -> Text {
        let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for line in t.lines() {
            let l = line.split(';').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            if let Some(s) = l.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                sections.push((rt::lower(s.trim()), Vec::new()));
                continue;
            }
            let (Some((k, v)), Some(sec)) = (l.split_once('='), sections.last_mut()) else { continue };
            sec.1.push((rt::lower(k.trim()), String::from(v.trim().trim_matches('"'))));
        }
        Text { sections }
    }
    /// `other`'s keys over these (a recipe over its `Base`).
    fn over(mut self, other: Text) -> Text {
        for (name, keys) in other.sections {
            match self.sections.iter_mut().find(|s| s.0 == name) {
                Some(s) => {
                    for (k, v) in keys {
                        match s.1.iter_mut().find(|e| e.0 == k) {
                            Some(e) => e.1 = v,
                            None => s.1.push((k, v)),
                        }
                    }
                }
                None => self.sections.push((name, keys)),
            }
        }
        self
    }
    fn get(&self, sec: &str, key: &str) -> Option<&str> {
        let s = self.sections.iter().find(|s| s.0 == sec)?;
        s.1.iter().find(|e| e.0 == key).map(|e| e.1.as_str())
    }
    fn str(&self, sec: &str, key: &str, def: &str) -> String {
        String::from(self.get(sec, key).unwrap_or(def))
    }
    fn f(&self, sec: &str, key: &str, def: f32) -> f32 {
        self.get(sec, key).and_then(parse_f).unwrap_or(def)
    }
    fn v(&self, sec: &str, key: &str, def: V3) -> V3 {
        let Some(s) = self.get(sec, key) else { return def };
        let mut it = s.split(',').map(|x| parse_f(x.trim()));
        match (it.next().flatten(), it.next().flatten(), it.next().flatten()) {
            (Some(x), Some(y), Some(z)) => v3(x, y, z),
            _ => def,
        }
    }
}

fn parse_f(s: &str) -> Option<f32> {
    let s = s.trim().trim_end_matches('f');
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let mut v = 0.0f64;
    for c in int.bytes() {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10.0 + (c - b'0') as f64;
    }
    let mut k = 0.1f64;
    for c in frac.bytes() {
        if !c.is_ascii_digit() {
            return None;
        }
        v += (c - b'0') as f64 * k;
        k *= 0.1;
    }
    Some(if neg { -v as f32 } else { v as f32 })
}

/// The recipe's text: the game directory's `Menu\<name>.ini`, else the built-in one.
fn text_of(name: &str) -> Option<(String, &'static str)> {
    let mut path = String::from("Menu\\");
    path.push_str(name);
    path.push_str(".ini");
    if let Some(d) = rt::read_file(&rt::game_path(&path)) {
        return Some((String::from_utf8_lossy(&d).into_owned(), "Menu\\ in the game directory"));
    }
    BUILT_IN.iter().find(|(n, _)| *n == name).map(|(_, t)| (String::from(*t), "built in"))
}

struct ShipDef {
    odf: String,
    sod: String,
    scale: f32,
    at: V3,
    yaw: f32,
    bob: f32,
    phase: f32,
    team: Option<V3>,
}

/// A recipe, read.
struct Recipe {
    name: String,
    /// What it was read from (its own text and its base's), to notice an edit.
    text: String,
    sky: String,
    sky_rate: f32,
    stars: Vec<(V3, f32, f32)>,
    sun: Option<(String, V3, f32)>,
    key: Directional,
    fill: Directional,
    planet: Option<(String, String, f32, f32, f32, V3)>,
    altitude: f32,
    /// The planet's centre from the fleet (`[Planet] At`), when the recipe moves it
    /// from straight below.
    planet_at: Option<V3>,
    orbit_rate: f32,
    eye: V3,
    target: V3,
    fov: f32,
    sway: V3,
    sway_period: f32,
    blend: f32,
    ships: Vec<ShipDef>,
}

/// Turns about the unit axis `a` by `t` radians, as an M34 of rows.
fn rotation(a: V3, t: f32) -> M34 {
    let (s, c) = (math::sinf(t), math::cosf(t));
    let k = 1.0 - c;
    let (x, y, z) = (a.x, a.y, a.z);
    M34 {
        r: [
            v3(c + x * x * k, x * y * k + z * s, x * z * k - y * s),
            v3(y * x * k - z * s, c + y * y * k, y * z * k + x * s),
            v3(z * x * k + y * s, z * y * k - x * s, c + z * z * k),
        ],
        p: V3::ZERO,
    }
}

const DEG: f32 = 0.017_453_293;

impl Recipe {
    fn read(name: &str) -> Option<Recipe> {
        let (own, src) = text_of(name)?;
        let mut t = Text::parse(&own);
        let mut text = own.clone();
        // One level of `Base=`: another recipe under this one.
        if let Some(base) = t.get("scene", "base").map(rt::lower).filter(|b| *b != name) {
            if let Some((bt, _)) = text_of(&base) {
                text.push_str(&bt);
                t = Text::parse(&bt).over(t);
            }
        }
        let s = "scene";
        let key_dir = t.v(s, "keyaxis", v3(-0.6, -0.35, 0.72)).norm();
        let fill_dir = t.v(s, "fillaxis", v3(0.5, 0.4, -0.6)).norm();
        let mut stars = Vec::new();
        let n = t.f(s, "stars", 1400.0).clamp(0.0, 8000.0) as u32;
        let size = t.f(s, "starsize", 0.0012);
        let mut rng = math::Lcg(t.f(s, "starseed", 7.0) as u32 | 1);
        for _ in 0..n {
            // A uniform direction, a size and brightness weighted towards the faint.
            let z = rng.signed();
            let a = rng.unit() * 6.283_185_5;
            let r = math::sqrtf((1.0 - z * z).max(0.0));
            let d = v3(r * math::cosf(a), z, r * math::sinf(a));
            let u = rng.unit();
            let b = 0.25 + 0.75 * u * u * u;
            stars.push((d, size * (0.6 + 0.8 * b), b));
        }
        let sun = t.get(s, "sun").filter(|x| !x.is_empty()).map(|o| {
            (String::from(o), -key_dir, t.f(s, "sundistance", 30000.0))
        });
        let planet = t.get("planet", "class").map(|cls| {
            let odf = assets::odf(cls);
            let g = |k: &str, d: &str| -> String {
                t.get("planet", k).map(String::from).or_else(|| odf.as_ref().and_then(|o| o.get_str(k).map(String::from))).unwrap_or(String::from(d))
            };
            let ground = g("groundtexturename", cls);
            // Every class but those that name one draws the stock clouds.
            let atmo = g("atmospheretexturename", "PA_ECFR0");
            (
                ground,
                atmo,
                t.f("planet", "radius", 3000.0),
                t.f("planet", "spin", 0.0),
                t.f("planet", "clouds", 0.0),
                t.v("planet", "axis", v3(1.0, 0.0, 0.0)).norm(),
            )
        });
        let mut ships = Vec::new();
        for (sec, _) in &t.sections {
            if !sec.starts_with("ship") {
                continue;
            }
            let Some(odf) = t.get(sec, "odf") else { continue };
            let sod = assets::sod_of(odf);
            let scale_sod = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|x| *x > 0.0).unwrap_or(1.0);
            let team = t.get(sec, "team").map(|_| t.v(sec, "team", v3(1.0, 1.0, 1.0)));
            ships.push(ShipDef {
                odf: String::from(odf),
                sod,
                scale: scale_sod * t.f(sec, "scale", 1.0),
                at: t.v(sec, "at", V3::ZERO),
                yaw: t.f(sec, "yaw", 0.0) * DEG,
                bob: t.f(sec, "bob", 1.0),
                phase: t.f(sec, "phase", ships.len() as f32 * 1.7),
                team,
            });
        }
        let r = Recipe {
            name: String::from(name),
            text,
            sky: t.str(s, "sky", "mbgblue"),
            sky_rate: t.f(s, "skyturn", 0.004),
            stars,
            sun,
            key: Directional { dir: key_dir, colour: t.v(s, "keycolour", v3(1.0, 0.95, 0.86)) },
            fill: Directional { dir: fill_dir, colour: t.v(s, "fillcolour", v3(0.10, 0.12, 0.18)) },
            planet,
            altitude: t.f("orbit", "altitude", 600.0),
            planet_at: t.get("planet", "at").map(|_| t.v("planet", "at", V3::ZERO)),
            orbit_rate: t.f("orbit", "rate", 0.004),
            eye: t.v("camera", "eye", v3(-180.0, 60.0, -260.0)),
            target: t.v("camera", "target", v3(40.0, -20.0, 0.0)),
            fov: t.f("camera", "fov", 40.0).clamp(5.0, 120.0),
            sway: t.v("camera", "sway", v3(8.0, 4.0, 6.0)),
            sway_period: t.f("camera", "swayperiod", 40.0).max(1.0),
            blend: t.f("camera", "blend", 1.5).max(0.0),
            ships,
        };
        log!(
            "menu: scene \"{}\" ({}): sky {}, planet {}, {} ships, {} stars",
            name,
            src,
            r.sky,
            r.planet.as_ref().map_or("none", |p| p.0.as_str()),
            r.ships.len(),
            r.stars.len()
        );
        for sh in &r.ships {
            log!("menu:   ship {} ({}) at {:.0},{:.0},{:.0} scale {:.2}", sh.odf, sh.sod, sh.at.x, sh.at.y, sh.at.z, sh.scale);
        }
        Some(r)
    }

    /// Where the fleet is: `Altitude` over the planet's surface on +y, or opposite
    /// `[Planet] At` from its centre (the origin).
    fn fleet(&self) -> V3 {
        match self.planet_at {
            Some(a) => -a,
            None => v3(0.0, self.planet.as_ref().map_or(0.0, |p| p.2) + self.altitude, 0.0),
        }
    }

    /// The camera's eye and target in the fleet's frame at `t`, with its sway.
    fn camera(&self, t: f32) -> (V3, V3) {
        let w = 6.283_185_5 / self.sway_period;
        let s = self.sway;
        let d = v3(s.x * math::sinf(w * t), s.y * math::sinf(w * t * 0.73 + 1.1), s.z * math::sinf(w * t * 0.51 + 2.3));
        (self.eye + d, self.target + d * 0.3)
    }

    /// The scene posed at `t` seconds.
    fn shot(&self, t: f32) -> Shot {
        let fleet = self.fleet();
        let mut ships = Vec::new();
        for s in &self.ships {
            // Each ship bobs and rolls a little on a period of its own.
            let ph = s.phase + t * 0.21;
            let bob = v3(0.0, math::sinf(ph) * 1.5, math::sinf(ph * 0.7) * 0.8) * s.bob;
            let roll = math::sinf(ph * 0.8 + 0.5) * 0.02 * s.bob;
            let pitch = math::sinf(ph * 0.6 + 1.3) * 0.012 * s.bob;
            let (sy, cy) = (math::sinf(s.yaw), math::cosf(s.yaw));
            let yaw = M34 { r: [v3(cy, 0.0, -sy), v3(0.0, 1.0, 0.0), v3(sy, 0.0, cy)], p: V3::ZERO };
            let m = rotation(v3(0.0, 0.0, 1.0), roll).then(&rotation(v3(1.0, 0.0, 0.0), pitch)).then(&yaw);
            let mut w = m;
            w.r[0] = w.r[0] * s.scale;
            w.r[1] = w.r[1] * s.scale;
            w.r[2] = w.r[2] * s.scale;
            w.p = fleet + s.at + bob;
            ships.push(ShipAt { sod: s.sod.clone(), world: w, team: s.team });
        }
        // The orbit: the planet turns under the fleet about +x, and the sky with it.
        let orbit = -self.orbit_rate * t;
        let planet = self.planet.as_ref().map(|(g, a, radius, spin, clouds, axis)| {
            // The model's pole (+y) along the axis, then the ground's turn.
            let up = *axis;
            let side = if up.y.abs() < 0.9 { v3(0.0, 1.0, 0.0).cross(up).norm() } else { v3(1.0, 0.0, 0.0) };
            let fwd = side.cross(up).norm();
            let frame = M34 { r: [side, up, fwd], p: V3::ZERO };
            let turn = rotation(v3(0.0, 1.0, 0.0), orbit * (1.0 + spin));
            let mut w = turn.then(&frame);
            w.p = V3::ZERO;
            PlanetAt { ground: g.clone(), atmo: a.clone(), world: w, radius: *radius, cloud_yaw: clouds * t }
        });
        Shot {
            sky: self.sky.clone(),
            sky_turn: rotation(v3(1.0, 0.0, 0.0), self.sky_rate * t),
            stars: self.stars.clone(),
            sun: self.sun.clone(),
            planet,
            ships,
        }
    }
}

/// What persists between frames.
struct State {
    recipe: Option<Recipe>,
    /// When the clock started (`rt::now`), and the last frame's time.
    start: f64,
    last: f64,
    /// When the recipe was last checked for an edit.
    checked: f64,
    /// The camera the last frame had, and the one blended from after a change of scene.
    cam: Option<(V3, V3, f32)>,
    from: Option<(V3, V3, f32, f64)>,
    said: u32,
}

static mut STATE: State = State { recipe: None, start: 0.0, last: 0.0, checked: 0.0, cam: None, from: None, said: 0 };

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

/// A perspective projection (left-handed, row vectors), vertical field `fov` radians.
fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> M4 {
    let ys = 1.0 / math::tanf(fov * 0.5);
    let xs = ys / aspect.max(1e-3);
    let q = far / (far - near);
    M4([[xs, 0.0, 0.0, 0.0], [0.0, ys, 0.0, 0.0], [0.0, 0.0, q, 1.0], [0.0, 0.0, -near * q, 0.0]])
}

/// The view matrix of a camera at `eye` looking at `at`, +y up.
fn look_at(eye: V3, at: V3) -> M4 {
    let f = (at - eye).norm();
    let up = v3(0.0, 1.0, 0.0);
    let r = up.cross(f).norm();
    let u = f.cross(r);
    M34 { r: [r, u, f], p: eye }.inverse_affine().to_m4()
}

fn smooth(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Draws the scene `name` over the back buffer's rectangle `rect`, inside the caller's
/// scene. False, with nothing drawn, when it cannot be (no recipe, no device, no
/// targets): the caller draws its own backdrop.
pub fn draw(name: &str, rect: PRect) -> bool {
    let s = st();
    let now = rt::now();
    if s.start == 0.0 || now - s.last > 600.0 {
        s.start = now;
    }
    s.last = now;
    let t = (now - s.start) as f32;
    let changed = s.recipe.as_ref().is_none_or(|r| r.name != name);
    if changed || now - s.checked > 1.0 {
        s.checked = now;
        let same_text = !changed && text_of(name).is_some_and(|(own, _)| s.recipe.as_ref().is_some_and(|r| r.text.starts_with(own.as_str())));
        if !same_text {
            let old = s.recipe.take();
            s.recipe = Recipe::read(name);
            if s.recipe.is_none() {
                if s.said & 1 == 0 {
                    s.said |= 1;
                    log!("menu: no scene \"{}\"", name);
                }
                s.recipe = old;
                return false;
            }
            if changed {
                if let Some((e, a, f)) = s.cam {
                    s.from = Some((e, a, f, now));
                }
            }
        }
    }
    let Some(r) = s.recipe.as_ref() else { return false };
    let (mut eye, mut at) = r.camera(t);
    let mut fov = r.fov;
    if let Some((e0, a0, f0, t0)) = s.from {
        let k = if r.blend > 0.0 { smooth(((now - t0) as f32) / r.blend) } else { 1.0 };
        eye = e0.lerp(eye, k);
        at = a0.lerp(at, k);
        fov = f0 + (fov - f0) * k;
        if k >= 1.0 {
            s.from = None;
        }
    }
    s.cam = Some((eye, at, fov));
    let shot = r.shot(t);
    let fleet = r.fleet();
    let (eye_w, at_w) = (fleet + eye, fleet + at);
    let lights = Lights { dirs: alloc::vec![r.key, r.fill], points: Vec::new() };
    let planet_r = r.planet.as_ref().map_or(0.0, |p| p.2);
    crate::frame::menu_frame(rect, move |dev: Device, frame: u32| {
        let view = look_at(eye_w, at_w);
        let aspect = rect.w as f32 / rect.h.max(1) as f32;
        let fovr = fov * DEG;
        let mk = |near: f32, far: f32| View {
            cam: 0,
            rs: 0,
            main: false,
            rect,
            dev,
            st3d: 0,
            view,
            proj: perspective(fovr, aspect, near, far),
            eye: eye_w,
            time: t,
            frame,
        };
        crate::scene::set_lights(lights);
        // The sky and the planet on a projection that reaches past the planet; the
        // ships on one of their own, after the depth is cleared.
        let reach = (eye_w.len() + planet_r * 1.2).max(1000.0);
        let far_v = mk((reach * 0.01).max(1.0), reach * 2.0);
        mods::menu(Pass::Sky, &shot, &far_v);
        mods::menu(Pass::Planet, &shot, &far_v);
        dev.clear(&[], crate::d3d::CLEAR_ZBUFFER, 0, 1.0, 0);
        let near_v = mk(1.0, (eye_w.len() * 0.5).max(4000.0));
        mods::menu_begin(&shot, &near_v);
        mods::menu(Pass::Ships, &shot, &near_v);
        crate::scene::flush(&near_v);
        mods::menu(Pass::Translucent, &shot, &near_v);
    })
}

/// `UI.asi`'s call: draws the scene `name` (a NUL-terminated name) over the rectangle
/// `x, y, w, h` of the back buffer, between the caller's `BeginScene` and `EndScene`
/// with the back buffer bound. 1 if it drew, 0 if the caller is to draw its own
/// backdrop. The device's state is put back as it was, the back buffer bound again.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_MenuScene(name: *const u8, x: u32, y: u32, w: u32, h: u32) -> u32 {
    if name.is_null() || w == 0 || h == 0 || crate::config::get().is_none() {
        return 0;
    }
    let n = rt::lower(unsafe { rt::cstr_at(name, 32) });
    draw(&n, PRect { x, y, w, h }) as u32
}
