//! The menu scene: a map of our own behind the front end's menus (a Federation fleet in
//! orbit over a planet, a starbase beside it, Klingon ships before a Metreon nebula, a
//! Borg fusion cube in a Mutara nebula), drawn on request by `UI.asi` through
//! `Render3D_MenuScene` from one of its views, with the modules' own looks (the sky's
//! recipe, the world's planet and gas, the hulls' lighting, shadows, running lights and
//! cloak) and the frame's bloom and tone map. README.md, "The menu scene".
//!
//! Nothing of a mission is read: in the front end there is no game object, no camera
//! and no registered light. The map is a recipe (`render3d/menus/map.ini`, built in; a
//! `Menu\map.ini` in the game directory takes precedence and is read again when it
//! changes), posed on a clock of its own, and the view is ours. Every module that draws
//! a part of it does so in `Module::menu`, given the posed `Shot`.
//!
//! Every scene is on the one map, under one sun: a view names the camera, and asking
//! for another view moves the camera over to it (`Blend=`). The planet turns about its
//! axis and the sky about +x (the fleet's orbit).
use crate::d3d::Device;
use crate::frame::post::PRect;
use crate::frame::steps::View;
use crate::math::{self, M34, M4, V3, v3};
use crate::mods;
use crate::scene::lights::{Directional, Lights};
use crate::{assets, log, rt};
use alloc::string::String;
use alloc::vec::Vec;

/// The map built into the plugin.
static BUILT_IN: &str = include_str!("../../menus/map.ini");

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
    /// The gas, over the hulls by their depth (before the cloak, which bends what is
    /// behind a hull).
    Gas,
    /// What blends over the hulls: additive meshes, decloaking hulls, running lights.
    Translucent,
    /// A departure's warp flashes and its tunnel, over everything (`Render3D_MenuWarp`).
    Warp,
}

/// A ship of the scene, posed for this frame.
pub struct ShipAt {
    /// Its SOD (the ODF's `baseName`).
    pub sod: String,
    /// Its placement, the ODF's `ScaleSOD` and the recipe's scale folded in.
    pub world: M34,
    pub team: Option<V3>,
    /// Decloaking: how much of the hull is back, 0 (not there) to 1; None when whole.
    pub cloak: Option<f32>,
    /// Going to warp: the hull's warp stretch (its ramp 0..1, the stretch S, the way it
    /// goes, map space); None when not.
    pub warp: Option<(f32, f32, V3)>,
    /// Gone to warp `age` seconds ago: not drawn, but its flash is (`Shot::flash`).
    pub gone: Option<f32>,
}

/// A departure's tunnel: the warp's star streaks rushing at the eye (`Pass::Warp`).
#[derive(Clone, Copy)]
pub struct Tunnel {
    /// How far it has come in, 0..1.
    pub k: f32,
    /// Seconds since the departure began (the streaks' clock).
    pub time: f32,
    /// How much the scene under it is darkened when it is whole.
    pub dim: f32,
    /// How fast the streaks come.
    pub speed: f32,
    pub colour: V3,
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

/// A gas field of the map: nebulae of one class, and what the map sets of its recipe
/// for the menu (the menu's classes are its own, so a mission's are untouched).
#[derive(Clone)]
pub struct Gas {
    pub odf: String,
    pub radius: f32,
    /// The nebulae's centres (map).
    pub centres: Vec<[f32; 3]>,
    /// `KeyShare=`, `Scatter=`, `Opacity=`, `Brightness=` over the recipe's.
    pub key_share: Option<f32>,
    pub scatter: Option<f32>,
    pub opacity: Option<f32>,
    pub brightness: Option<f32>,
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
    pub gas: Vec<Gas>,
    /// Towards the Key (the sun), which lights the gas.
    pub key: Option<V3>,
    /// A departure's flashes: half size (times the ship's radius) and how long one
    /// lasts; the hulls module, which knows each ship's radius, adds them (`add_flash`).
    pub flash: (f32, f32),
    pub tunnel: Option<Tunnel>,
}

/// Warp flashes of this frame (centre, half size, brightness), put here by the hulls
/// module as it poses the departing ships and drawn by the sky at `Pass::Warp`.
static mut FLASHES: Vec<(V3, f32, f32)> = Vec::new();

pub fn add_flash(at: V3, half: f32, bright: f32) {
    unsafe { (*core::ptr::addr_of_mut!(FLASHES)).push((at, half, bright)) }
}

/// This frame's flashes, emptied.
pub fn take_flashes() -> Vec<(V3, f32, f32)> {
    unsafe { core::mem::take(&mut *core::ptr::addr_of_mut!(FLASHES)) }
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

/// The map's text: the game directory's `Menu\map.ini`, else the built-in one.
fn text_of() -> (String, &'static str) {
    if let Some(d) = rt::read_file(&rt::game_path("Menu\\map.ini")) {
        return (String::from_utf8_lossy(&d).into_owned(), "Menu\\map.ini in the game directory");
    }
    (String::from(BUILT_IN), "built in")
}

/// A group of the map: a place the scenes' ships, gas and cameras are given from.
#[derive(Clone)]
struct Group {
    name: String,
    at: V3,
    yaw: f32,
    /// The view on whose arrival the group's ships decloak (`CloakView=`): cloaked
    /// until then, after `delay` seconds, each ship `stagger` after the last, over `time`.
    cloak_view: Option<String>,
    cloak_delay: f32,
    cloak_time: f32,
    cloak_stagger: f32,
    /// The tunnel's tint when this group departs (`WarpColour=`).
    warp_colour: V3,
}

impl Group {
    /// A point given in the group's frame, in the map's.
    fn place(&self, p: V3) -> V3 {
        let (s, c) = (math::sinf(self.yaw), math::cosf(self.yaw));
        self.at + v3(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
    }
}

struct ShipDef {
    odf: String,
    sod: String,
    group: usize,
    /// Which of its group's ships it is, for the cloak's stagger.
    nth: usize,
    scale: f32,
    at: V3,
    yaw: f32,
    bob: f32,
    phase: f32,
    team: Option<V3>,
    /// Goes to warp when its group departs (`Warp=0`: a station stays).
    warp: bool,
}

impl ShipDef {
    /// Its nose, map space: the ship's +z turned by its yaw and its group's.
    fn nose(&self, g: &Group) -> V3 {
        let yw = self.yaw + g.yaw;
        v3(math::sinf(yw), 0.0, math::cosf(yw))
    }
}

/// A view: a camera in its group's frame.
struct ViewDef {
    name: String,
    group: usize,
    eye: V3,
    target: V3,
    fov: f32,
    sway: V3,
    sway_period: f32,
    /// The camera a departure from this view swings to (`WarpEye=`, `WarpTarget=`);
    /// None: behind the departing ships, looking the way they go.
    warp_eye: Option<V3>,
    warp_target: Option<V3>,
}

/// How a departure goes (`[Warp]`): seconds, unless said.
struct WarpDef {
    /// Before the first ship goes; between one ship and the next; how long a ship
    /// takes from impulse to gone.
    delay: f32,
    stagger: f32,
    ramp: f32,
    /// The stretch at the end of the ramp, and the impulse's acceleration (units/s²).
    stretch: f32,
    speed: f32,
    /// How long the camera takes to swing behind the ships, and how hard it follows
    /// them (units/s²).
    turn: f32,
    follow: f32,
    /// The tunnel: after the last ship has gone, when it starts and how long it takes
    /// to come in, how much it darkens the scene, how fast its streaks come; the field
    /// of view it widens by (degrees).
    tunnel_at: f32,
    tunnel_in: f32,
    dim: f32,
    tunnel_speed: f32,
    widen: f32,
    /// A flash's half size (times the ship's radius) and how long it lasts.
    flash: f32,
    flash_time: f32,
}

/// The map, read.
struct Map {
    /// What it was read from, to notice an edit.
    text: String,
    sky: String,
    sky_rate: f32,
    stars: Vec<(V3, f32, f32)>,
    sun: Option<(String, V3, f32)>,
    key: Directional,
    fill: Directional,
    /// Ground, clouds, radius, spin, cloud turn, axis, centre.
    planet: Option<(String, String, f32, f32, f32, V3, V3)>,
    orbit_rate: f32,
    blend: f32,
    groups: Vec<Group>,
    ships: Vec<ShipDef>,
    gas: Vec<Gas>,
    views: Vec<ViewDef>,
    warp: WarpDef,
}

/// A departure at one moment: the camera, how far each ship has got, the tunnel.
struct Departure {
    /// The departing ships' way (map space).
    dir: V3,
    /// For each ship of the map, seconds into its own departure (None: not going yet,
    /// or not at all).
    tau: Vec<Option<f32>>,
    /// The departure camera (eye, target), before the follow.
    eye: V3,
    target: V3,
    /// How far the swing to it has got, 0..1, and how far the camera has followed.
    swing: f32,
    follow: f32,
    tunnel: Option<Tunnel>,
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

/// The part of a section name after `<kind>.` (`[View.main]` -> `main`).
fn named<'a>(sec: &'a str, kind: &str) -> Option<&'a str> {
    sec.strip_prefix(kind)?.strip_prefix('.').filter(|n| !n.is_empty())
}

impl Map {
    fn read() -> Map {
        let (text, src) = text_of();
        let t = Text::parse(&text);
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
        let sun = t.get(s, "sun").filter(|x| !x.is_empty()).map(|o| (String::from(o), -key_dir, t.f(s, "sundistance", 30000.0)));
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
                t.v("planet", "at", V3::ZERO),
            )
        });
        let blend = t.f(s, "blend", 2.0).max(0.0);
        // The groups; ships, gas and views name theirs (none: the map's origin).
        let mut groups = alloc::vec![Group {
            name: String::new(),
            at: V3::ZERO,
            yaw: 0.0,
            cloak_view: None,
            cloak_delay: 0.0,
            cloak_time: 1.0,
            cloak_stagger: 0.0,
            warp_colour: v3(0.45, 0.62, 1.0),
        }];
        for (sec, _) in &t.sections {
            let Some(name) = named(sec, "group") else { continue };
            groups.push(Group {
                name: String::from(name),
                at: t.v(sec, "at", V3::ZERO),
                yaw: t.f(sec, "yaw", 0.0) * DEG,
                cloak_view: t.get(sec, "cloakview").map(rt::lower),
                cloak_delay: t.f(sec, "cloakdelay", blend * 0.7).max(0.0),
                cloak_time: t.f(sec, "cloaktime", 2.5).max(0.05),
                cloak_stagger: t.f(sec, "cloakstagger", 0.5).max(0.0),
                warp_colour: t.v(sec, "warpcolour", v3(0.45, 0.62, 1.0)),
            });
        }
        let group_of = |sec: &str| -> usize {
            let g = t.get(sec, "group").map(rt::lower).unwrap_or_default();
            groups.iter().position(|x| x.name == g).unwrap_or(0)
        };
        let mut ships: Vec<ShipDef> = Vec::new();
        for (sec, _) in &t.sections {
            if named(sec, "ship").is_none() {
                continue;
            }
            let Some(odf) = t.get(sec, "odf") else { continue };
            let sod = assets::sod_of(odf);
            let scale_sod = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|x| *x > 0.0).unwrap_or(1.0);
            let team = t.get(sec, "team").map(|_| t.v(sec, "team", v3(1.0, 1.0, 1.0)));
            let group = group_of(sec);
            let nth = ships.iter().filter(|s| s.group == group).count();
            ships.push(ShipDef {
                odf: String::from(odf),
                sod,
                group,
                nth,
                scale: scale_sod * t.f(sec, "scale", 1.0),
                at: t.v(sec, "at", V3::ZERO),
                yaw: t.f(sec, "yaw", 0.0) * DEG,
                bob: t.f(sec, "bob", 1.0),
                phase: t.f(sec, "phase", ships.len() as f32 * 1.7),
                team,
                warp: t.f(sec, "warp", 1.0) != 0.0,
            });
        }
        let mut gas = Vec::new();
        for (sec, _) in &t.sections {
            if named(sec, "nebula").is_none() {
                continue;
            }
            let Some(odf) = t.get(sec, "odf") else { continue };
            let g = &groups[group_of(sec)];
            // The class's radius: the recipe's, else the ODF's `effectRadius`.
            let radius = t.get(sec, "radius").and_then(parse_f).or_else(|| assets::odf(odf).and_then(|o| o.get_f32("effectRadius"))).filter(|r| *r > 10.0).unwrap_or(300.0);
            // Centres on a jittered grid over an ellipse of half extents `Size` (x, z)
            // in the group's frame, `Spacing` apart, as a map's nebula is laid out.
            let at = t.v(sec, "at", V3::ZERO);
            let size = t.v(sec, "size", v3(1500.0, 0.0, 1500.0));
            let step = t.f(sec, "spacing", radius * 1.2).max(radius * 0.3);
            let jitter = t.f(sec, "jitter", 0.35);
            let mut rng = math::Lcg(t.f(sec, "seed", 11.0) as u32 | 1);
            let mut centres = Vec::new();
            let (nx, nz) = ((size.x / step) as i32, (size.z / step) as i32);
            for iz in -nz..=nz {
                for ix in -nx..=nx {
                    let (x, z) = (ix as f32 * step + rng.signed() * step * jitter, iz as f32 * step + rng.signed() * step * jitter);
                    let e = (x / size.x.max(1.0)) * (x / size.x.max(1.0)) + (z / size.z.max(1.0)) * (z / size.z.max(1.0));
                    if e > 1.0 {
                        continue;
                    }
                    let p = g.place(at + v3(x, 0.0, z));
                    centres.push([p.x, p.y, p.z]);
                }
            }
            let over = |k: &str| t.get(sec, k).and_then(parse_f);
            gas.push(Gas {
                odf: String::from(odf),
                radius,
                centres,
                key_share: over("keyshare"),
                scatter: over("scatter"),
                opacity: over("opacity"),
                brightness: over("brightness"),
            });
        }
        let mut views = Vec::new();
        for (sec, _) in &t.sections {
            let Some(name) = named(sec, "view") else { continue };
            let gi = group_of(sec);
            let g = &groups[gi];
            let at = |k: &str| t.get(sec, k).map(|_| g.place(t.v(sec, k, V3::ZERO)));
            views.push(ViewDef {
                name: String::from(name),
                group: gi,
                eye: g.place(t.v(sec, "eye", v3(0.0, 50.0, 400.0))),
                target: g.place(t.v(sec, "target", V3::ZERO)),
                fov: t.f(sec, "fov", 40.0).clamp(5.0, 120.0),
                sway: t.v(sec, "sway", v3(8.0, 4.0, 6.0)),
                sway_period: t.f(sec, "swayperiod", 40.0).max(1.0),
                warp_eye: at("warpeye"),
                warp_target: at("warptarget"),
            });
        }
        let m = Map {
            text: text.clone(),
            sky: t.str(s, "sky", "mbgblue"),
            sky_rate: t.f(s, "skyturn", 0.004),
            stars,
            sun,
            key: Directional { dir: key_dir, colour: t.v(s, "keycolour", v3(1.0, 0.95, 0.86)) },
            fill: Directional { dir: fill_dir, colour: t.v(s, "fillcolour", v3(0.10, 0.12, 0.18)) },
            planet,
            orbit_rate: t.f("planet", "orbit", 0.004),
            blend,
            groups,
            ships,
            gas,
            views,
            warp: WarpDef {
                delay: t.f("warp", "delay", 0.9).max(0.0),
                stagger: t.f("warp", "stagger", 0.18).max(0.0),
                ramp: t.f("warp", "ramp", 0.75).max(0.05),
                stretch: t.f("warp", "stretch", 30.0).max(0.0),
                speed: t.f("warp", "speed", 120.0),
                turn: t.f("warp", "turn", 1.3).max(0.05),
                follow: t.f("warp", "follow", 260.0),
                tunnel_at: t.f("warp", "tunnelat", 0.1),
                tunnel_in: t.f("warp", "tunnelin", 0.9).max(0.05),
                dim: t.f("warp", "dim", 0.85).clamp(0.0, 1.0),
                tunnel_speed: t.f("warp", "tunnelspeed", 1.0),
                widen: t.f("warp", "widen", 18.0),
                flash: t.f("warp", "flash", 2.2).max(0.0),
                flash_time: t.f("warp", "flashtime", 0.45).max(0.05),
            },
        };
        log!(
            "menu: map ({}): sky {}, planet {}, {} groups, {} ships, {} gas fields, {} views, {} stars",
            src,
            m.sky,
            m.planet.as_ref().map_or("none", |p| p.0.as_str()),
            m.groups.len() - 1,
            m.ships.len(),
            m.gas.len(),
            m.views.len(),
            m.stars.len()
        );
        for g in m.groups.iter().skip(1) {
            log!("menu:   group {} at {:.0},{:.0},{:.0}{}", g.name, g.at.x, g.at.y, g.at.z, if g.cloak_view.is_some() { ", decloaks" } else { "" });
        }
        for sh in &m.ships {
            log!("menu:   ship {} ({}) in {} at {:.0},{:.0},{:.0} scale {:.2}", sh.odf, sh.sod, m.groups[sh.group].name, sh.at.x, sh.at.y, sh.at.z, sh.scale);
        }
        for g in &m.gas {
            log!("menu:   gas {} radius {:.0}, {} nebulae", g.odf, g.radius, g.centres.len());
        }
        for v in &m.views {
            log!("menu:   view {}", v.name);
        }
        m
    }

    fn view(&self, name: &str) -> Option<&ViewDef> {
        self.views.iter().find(|v| v.name == name)
    }

    /// A view's eye and target at `t`, with its sway.
    fn camera(v: &ViewDef, t: f32) -> (V3, V3) {
        let w = 6.283_185_5 / v.sway_period;
        let s = v.sway;
        let d = v3(s.x * math::sinf(w * t), s.y * math::sinf(w * t * 0.73 + 1.1), s.z * math::sinf(w * t * 0.51 + 2.3));
        (v.eye + d, v.target + d * 0.3)
    }

    /// The scene posed at `t` seconds; `cloak(group, nth)` says how far a ship of a
    /// decloaking group is back (None: whole).
    fn shot(&self, t: f32, cloak: impl Fn(&Group, usize) -> Option<f32>, dep: Option<&Departure>) -> Shot {
        let mut ships = Vec::new();
        for (si, s) in self.ships.iter().enumerate() {
            let g = &self.groups[s.group];
            // Each ship bobs and rolls a little on a period of its own.
            let ph = s.phase + t * 0.21;
            let bob = v3(0.0, math::sinf(ph) * 1.5, math::sinf(ph * 0.7) * 0.8) * s.bob;
            let roll = math::sinf(ph * 0.8 + 0.5) * 0.02 * s.bob;
            let pitch = math::sinf(ph * 0.6 + 1.3) * 0.012 * s.bob;
            let yw = s.yaw + g.yaw;
            let (sy, cy) = (math::sinf(yw), math::cosf(yw));
            let yaw = M34 { r: [v3(cy, 0.0, -sy), v3(0.0, 1.0, 0.0), v3(sy, 0.0, cy)], p: V3::ZERO };
            let m = rotation(v3(0.0, 0.0, 1.0), roll).then(&rotation(v3(1.0, 0.0, 0.0), pitch)).then(&yaw);
            let mut w = m;
            w.r[0] = w.r[0] * s.scale;
            w.r[1] = w.r[1] * s.scale;
            w.r[2] = w.r[2] * s.scale;
            w.p = g.place(s.at) + bob;
            // Departing: impulse along the way out, the warp stretch over the ramp,
            // then gone (its flash).
            let (mut warp, mut gone) = (None, None);
            if let Some((d, tau)) = dep.and_then(|d| d.tau[si].map(|tau| (d, tau))) {
                let wd = &self.warp;
                let tr = tau.min(wd.ramp);
                w.p = w.p + d.dir * (0.5 * wd.speed * tr * tr);
                if tau >= wd.ramp {
                    gone = Some(tau - wd.ramp);
                } else {
                    let b = smooth(tau / wd.ramp);
                    warp = Some((b, 1.0 + wd.stretch * b * b, d.dir));
                }
            }
            ships.push(ShipAt { sod: s.sod.clone(), world: w, team: s.team, cloak: cloak(g, s.nth), warp, gone });
        }
        // The orbit: the planet turns about its axis, and the sky with the fleet's orbit.
        let orbit = -self.orbit_rate * t;
        let planet = self.planet.as_ref().map(|(g, a, radius, spin, clouds, axis, at)| {
            // The model's pole (+y) along the axis, then the ground's turn.
            let up = *axis;
            let side = if up.y.abs() < 0.9 { v3(0.0, 1.0, 0.0).cross(up).norm() } else { v3(1.0, 0.0, 0.0) };
            let fwd = side.cross(up).norm();
            let frame = M34 { r: [side, up, fwd], p: V3::ZERO };
            let turn = rotation(v3(0.0, 1.0, 0.0), orbit * (1.0 + spin));
            let mut w = turn.then(&frame);
            w.p = *at;
            PlanetAt { ground: g.clone(), atmo: a.clone(), world: w, radius: *radius, cloud_yaw: clouds * t }
        });
        Shot {
            sky: self.sky.clone(),
            sky_turn: rotation(v3(1.0, 0.0, 0.0), self.sky_rate * t),
            stars: self.stars.clone(),
            sun: self.sun.clone(),
            planet,
            ships,
            gas: self.gas.clone(),
            key: Some(-self.key.dir),
            flash: (self.warp.flash, self.warp.flash_time),
            tunnel: dep.and_then(|d| d.tunnel),
        }
    }

    /// A departure from the view `vd`, `tw` seconds in: its group's ships that go to
    /// warp leave front first, the camera swings behind them and follows, and the
    /// tunnel comes in once they have gone.
    fn departure(&self, vd: &ViewDef, tw: f32) -> Departure {
        let g = &self.groups[vd.group];
        let wd = &self.warp;
        let going: Vec<usize> = (0..self.ships.len()).filter(|&i| self.ships[i].group == vd.group && self.ships[i].warp).collect();
        let mut dir = V3::ZERO;
        let mut centre = V3::ZERO;
        for &i in &going {
            dir = dir + self.ships[i].nose(g);
            centre = centre + g.place(self.ships[i].at);
        }
        let mut tau = alloc::vec![None; self.ships.len()];
        let (eye, target, last);
        if going.is_empty() {
            // Nothing leaves (a station alone): the camera goes on along its own line.
            dir = (vd.target - vd.eye).norm();
            eye = vd.eye;
            target = vd.target;
            last = wd.delay;
        } else {
            dir = if dir.len() > 1e-3 { dir.norm() } else { v3(0.0, 0.0, 1.0) };
            centre = centre * (1.0 / going.len() as f32);
            let spread = going.iter().map(|&i| (g.place(self.ships[i].at) - centre).len()).fold(0.0f32, f32::max);
            // Front first: in order along the way out.
            let mut order = going.clone();
            order.sort_by(|&a, &b| {
                let (pa, pb) = (g.place(self.ships[a].at).dot(dir), g.place(self.ships[b].at).dot(dir));
                pb.partial_cmp(&pa).unwrap_or(core::cmp::Ordering::Equal)
            });
            for (k, &i) in order.iter().enumerate() {
                let t0 = wd.delay + k as f32 * wd.stagger;
                if tw >= t0 {
                    tau[i] = Some(tw - t0);
                }
            }
            last = wd.delay + (order.len() - 1) as f32 * wd.stagger + wd.ramp;
            eye = vd.warp_eye.unwrap_or(centre - dir * (spread + 520.0) + v3(0.0, spread * 0.25 + 90.0, 0.0));
            target = vd.warp_target.unwrap_or(centre + dir * (spread + 900.0));
        }
        let swing = smooth(tw / wd.turn);
        let tf = (tw - wd.delay - wd.ramp * 0.3).max(0.0);
        let follow = 0.5 * wd.follow * tf * tf;
        let k = smooth((tw - last - wd.tunnel_at) / wd.tunnel_in);
        let tunnel = (k > 0.0).then_some(Tunnel { k, time: tw, dim: wd.dim, speed: wd.tunnel_speed, colour: g.warp_colour });
        Departure { dir, tau, eye, target, swing, follow, tunnel }
    }
}

/// What persists between frames.
struct State {
    map: Option<Map>,
    /// When the clock started (`rt::now`), and the last frame's time.
    start: f64,
    last: f64,
    /// When the map was last checked for an edit.
    checked: f64,
    /// The view shown, the one before it, and when it changed.
    view: String,
    prev: String,
    since: f64,
    /// The camera the last frame had, and the one blended from after a change of view.
    cam: Option<(V3, V3, f32)>,
    from: Option<(V3, V3, f32)>,
    said: u32,
    /// A departure: when it began, and the camera it swings from.
    warp: Option<(f64, Option<(V3, V3, f32)>)>,
}

static mut STATE: State = State {
    map: None,
    start: 0.0,
    last: 0.0,
    checked: 0.0,
    view: String::new(),
    prev: String::new(),
    since: 0.0,
    cam: None,
    from: None,
    said: 0,
    warp: None,
};

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

/// How far the ship `nth` of a decloaking group is back, the view `view` shown since
/// `since` (`now` the clock), the one before it `prev`; None when whole.
fn cloak_of(g: &Group, nth: usize, view: &str, prev: &str, since: f64, now: f64, blend: f32) -> Option<f32> {
    let cv = g.cloak_view.as_deref()?;
    let gone = (now - since) as f32;
    if cv == view {
        let k = (gone - g.cloak_delay - nth as f32 * g.cloak_stagger) / g.cloak_time;
        return if k >= 1.0 { None } else { Some(k.max(0.0)) };
    }
    // Leaving its view, the group stays as it was until the camera has gone; then it
    // cloaks again, to decloak on the next arrival.
    if cv == prev && gone < blend {
        return None;
    }
    Some(0.0)
}

/// Draws the view `name` of the map over the back buffer's rectangle `rect`, inside
/// the caller's scene; the camera moves over from the view drawn before. False, with
/// nothing drawn, when it cannot be (no such view, no device, no targets): the caller
/// draws its own backdrop.
pub fn draw(name: &str, rect: PRect, depart: bool) -> bool {
    let s = st();
    let now = rt::now();
    if s.start == 0.0 || now - s.last > 600.0 {
        s.start = now;
        s.cam = None;
    }
    s.last = now;
    let t = (now - s.start) as f32;
    if s.map.is_none() || now - s.checked > 1.0 {
        s.checked = now;
        if s.map.as_ref().is_none_or(|m| m.text != text_of().0) {
            s.map = Some(Map::read());
        }
    }
    let Some(m) = s.map.as_ref() else { return false };
    let Some(vd) = m.view(name) else {
        if s.said & 1 == 0 {
            s.said |= 1;
            log!("menu: the map has no view \"{}\"", name);
        }
        return false;
    };
    if s.view != name {
        s.prev = core::mem::replace(&mut s.view, String::from(name));
        s.since = now;
        s.from = s.cam;
        log!("menu: view {}", name);
    }
    let (mut eye, mut at) = Map::camera(vd, t);
    let mut fov = vd.fov;
    let gone = (now - s.since) as f32;
    if let Some((e0, a0, f0)) = s.from {
        let k = if m.blend > 0.0 { smooth(gone / m.blend) } else { 1.0 };
        eye = e0.lerp(eye, k);
        at = a0.lerp(at, k);
        fov = f0 + (fov - f0) * k;
        if k >= 1.0 {
            s.from = None;
        }
    }
    // A departure: from the camera there was, swung behind the ships, following them.
    if !depart {
        s.warp = None;
    } else if s.warp.is_none() {
        s.warp = Some((now, s.cam));
        log!("menu: departure from {}", name);
    }
    let dep = s.warp.map(|(t0, from)| {
        let d = m.departure(vd, (now - t0) as f32);
        let (e0, a0, f0) = from.unwrap_or((eye, at, fov));
        // Over the top as it swings, so the camera does not pass through the ships.
        let lift = v3(0.0, math::sinf(d.swing * 3.141_592_7) * 320.0, 0.0);
        let off = d.dir * d.follow;
        let (we, wt) = (e0.lerp(d.eye, d.swing) + lift + off, a0.lerp(d.target, d.swing) + off);
        let wk = d.tunnel.map_or(0.0, |tn| tn.k);
        let wf = f0 + (vd.fov - f0) * d.swing + m.warp.widen * wk;
        (d, we, wt, wf)
    });
    if let Some((_, we, wt, wf)) = dep.as_ref() {
        eye = *we;
        at = *wt;
        fov = *wf;
    } else {
        s.cam = Some((eye, at, fov));
    }
    let (view, prev, since, blend) = (s.view.as_str(), s.prev.as_str(), s.since, m.blend);
    let shot = m.shot(t, |g, nth| cloak_of(g, nth, view, prev, since, now, blend), dep.as_ref().map(|d| &d.0));
    let lights = Lights { dirs: alloc::vec![m.key, m.fill], points: Vec::new() };
    let planet = m.planet.as_ref().map(|p| (p.6, p.2));
    crate::frame::menu_frame(rect, move |dev: Device, frame: u32| {
        let view = look_at(eye, at);
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
            eye,
            time: t,
            frame,
        };
        crate::scene::set_lights(lights);
        // The sky and the planet on a projection that reaches past the planet; the
        // ships and the gas on one of their own, after the depth is cleared.
        let (near_p, far_p) = match planet {
            Some((c, r)) => {
                let d = (c - eye).len();
                (((d - r * 1.05) * 0.5).max(10.0), d + r * 1.2)
            }
            None => (10.0, 1.0e5),
        };
        let far_v = mk(near_p, far_p.max(near_p * 2.0));
        mods::menu(Pass::Sky, &shot, &far_v);
        mods::menu(Pass::Planet, &shot, &far_v);
        dev.clear(&[], crate::d3d::CLEAR_ZBUFFER, 0, 1.0, 0);
        let near_v = mk(2.0, 16000.0);
        mods::menu_begin(&shot, &near_v);
        mods::menu(Pass::Ships, &shot, &near_v);
        crate::scene::flush(&near_v);
        mods::menu(Pass::Gas, &shot, &near_v);
        mods::menu(Pass::Translucent, &shot, &near_v);
        mods::menu(Pass::Warp, &shot, &near_v);
        take_flashes();
    })
}

/// `UI.asi`'s call: draws the view `name` (a NUL-terminated name) of the map over the
/// rectangle `x, y, w, h` of the back buffer, between the caller's `BeginScene` and
/// `EndScene` with the back buffer bound. 1 if it drew, 0 if the caller is to draw its
/// own backdrop. The device's state is put back as it was, the back buffer bound again.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_MenuScene(name: *const u8, x: u32, y: u32, w: u32, h: u32) -> u32 {
    if name.is_null() || w == 0 || h == 0 || crate::config::get().is_none() {
        return 0;
    }
    let n = rt::lower(unsafe { rt::cstr_at(name, 32) });
    draw(&n, PRect { x, y, w, h }, false) as u32
}

/// `UI.asi`'s call while a mission loads from the campaign selection: as
/// `Render3D_MenuScene`, but the view's ships go to warp, the camera follows them, and
/// the warp's tunnel comes in. The departure begins at the first call after one of
/// `Render3D_MenuScene` and goes on as long as this is called.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_MenuWarp(name: *const u8, x: u32, y: u32, w: u32, h: u32) -> u32 {
    if name.is_null() || w == 0 || h == 0 || crate::config::get().is_none() {
        return 0;
    }
    let n = rt::lower(unsafe { rt::cstr_at(name, 32) });
    draw(&n, PRect { x, y, w, h }, true) as u32
}
