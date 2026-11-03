//! The shots that draw a model (mines, pods, the commando team, the myotron, the black
//! hole and flux wave weapons, the stasis field, the transwarp drive): their instance
//! drawn by us through the scene layer, with a look of their own from `[Model.<odf>]`
//! (a pulsing glow at the heart, its light), and the emitters on their SODs (rocket
//! trails, contrails, rift clouds) fed to the particles module. README "Ordnance",
//! "The models".
//!
//! Each of these classes' `Submit` sets the instance's scale as it likes (a beam's
//! length, a target's size) and then calls `Entity::Render` through slot 11 of its
//! vtable. That slot, in the vtables of the classes the engine submits for us, is
//! pointed at `ord_render`, which draws the instance while our ordnance step has
//! submitted it, and calls `Entity::Render` otherwise. So the class's own scaling runs
//! as it is, and only the draw is ours.
use super::{content, hooks};
use super::fx::Fx;
use super::ini::Settings;
use r3d_core::engine;
use r3d_core::frame::steps::View;
use r3d_core::math::{sinf, v3, M34, V3};
use r3d_core::engine::instance as game;
use r3d_core::scene::{self, pose, Params};
use r3d_core::assets;
use crate::log;
use a2fmt::sod::NodeKind;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// `[Model.<odf>]`: how a shot's model looks.
#[derive(Clone, Debug, PartialEq)]
struct Look {
    /// The glow at the model's heart: brightness (0: none), colour, half size (of the
    /// model's radius), the pulse's share and rate (radians a second), the star's share.
    glow: f32,
    col: V3,
    size: f32,
    pulse: f32,
    rate: f32,
    star: f32,
    min_px: f32,
    /// Its light on the hulls about it (0: none), full to `reach` world units.
    light: f32,
    reach: f32,
    /// The model's own lighting: ambient, self-illuminated texels.
    ambient: f32,
    self_illum: f32,
}

impl Look {
    fn load(s: &Settings, sec: &str) -> Look {
        let c = s.rgb(sec, "GlowColour", [255.0, 200.0, 140.0]);
        Look {
            glow: s.f(sec, "Glow", 0.0),
            col: v3(c[0], c[1], c[2]),
            size: s.f(sec, "GlowSize", 0.8),
            pulse: s.f(sec, "Pulse", 0.0),
            rate: s.f(sec, "PulseRate", 4.0),
            star: s.f(sec, "Star", 0.3),
            min_px: s.f(sec, "MinPixels", 6.0),
            light: s.f(sec, "Light", 0.0),
            reach: s.f(sec, "LightReach", 120.0),
            ambient: s.f(sec, "Ambient", 0.18),
            self_illum: s.f(sec, "SelfIllumination", 1.0),
        }
    }
}

/// What a shot's ODF draws: its SOD (`shotGeometry`), whether it has emitters, its look.
struct Kind {
    sod: String,
    emitters: bool,
    look: Look,
}

pub struct Models {
    pub on: bool,
    looks: BTreeMap<String, Look>,
    plain: Look,
    /// Per ODF stem: what it draws, or None (no SOD: the engine's).
    kinds: BTreeMap<String, Option<Kind>>,
    said: Vec<String>,
    pub drawn: u32,
}

impl Models {
    fn load_looks(s: &Settings) -> BTreeMap<String, Look> {
        let mut looks = BTreeMap::new();
        for sec in s.sections() {
            if let Some(n) = sec.strip_prefix("Model.") {
                looks.insert(crate::rt::lower(n), Look::load(s, &sec));
            }
        }
        looks
    }

    pub fn new(s: &Settings) -> Models {
        Models {
            on: s.i("Ordnance", "Models", 1) != 0,
            looks: Self::load_looks(s),
            plain: Look::load(s, "Model"),
            kinds: BTreeMap::new(),
            said: Vec::new(),
            drawn: 0,
        }
    }

    /// The settings read again: the looks the shots were given are dropped if any changed.
    pub fn configure(&mut self, s: &Settings) {
        self.on = s.i("Ordnance", "Models", 1) != 0;
        let (looks, plain) = (Self::load_looks(s), Look::load(s, "Model"));
        if looks != self.looks || plain != self.plain {
            self.looks = looks;
            self.plain = plain;
            self.kinds.clear();
        }
    }

    /// For a class the engine submits: points slot 11 of its vtable at `ord_render`,
    /// once, if it is still `Entity::Render`. False if it is not ours to draw.
    pub fn take_class(&mut self, ord: usize) -> bool {
        self.on && hooks::take_class_slot(ord)
    }

    fn kind(&mut self, odf: &str) -> Option<&Kind> {
        let stem = hooks::stem(odf);
        if !self.kinds.contains_key(&stem) {
            let k = assets::odf(&stem).and_then(|o| o.get_str("shotGeometry").map(String::from)).and_then(|g| {
                let sod = crate::rt::lower(g.trim_end_matches(content::SOD_EXTS[0]).trim_end_matches(content::SOD_EXTS[1]));
                let s = assets::sod(&sod)?;
                let emitters = s.nodes.iter().any(|n| matches!(n.kind, NodeKind::Emitter { .. }));
                let look = self.looks.get(&stem).cloned().unwrap_or_else(|| self.plain.clone());
                Some(Kind { sod, emitters, look })
            });
            match &k {
                Some(k) => log!("ordnance: model of {}: {} (emitters: {}, glow {})", stem, k.sod, k.emitters, k.look.glow),
                None => log!("ordnance: model of {}: none, the engine draws it", stem),
            }
            self.kinds.insert(stem.clone(), k);
        }
        self.kinds.get(&stem).and_then(|k| k.as_ref())
    }

    /// A bolt drawn by a recipe: the emitters on its SOD (the micro-organisms'
    /// contrails), which the engine's `Submit` would have kept going, fed to the
    /// particles module at the shot's transform.
    pub fn feed_shot(&mut self, v: &View, ord: usize) {
        if !self.on || !r3d_core::particles::live() {
            return;
        }
        let odf = hooks::ord_odf(ord);
        let Some(k) = self.kind(odf) else { return };
        if !k.emitters {
            return;
        }
        let sod = k.sod.clone();
        let s = game::instance_scale(hooks::shot_instance(ord));
        let mut world: M34 = hooks::transform(ord);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let key = hooks::shot_key(ord);
        r3d_core::particles::feed(key, &sod, &world, v);
    }

    /// In place of `Entity::Render` for `ord`, while our ordnance step submits it. True
    /// if it was handled (drawn, or culled as stock culls it); false leaves it to the
    /// engine.
    pub fn draw(&mut self, v: &View, fx: &mut Fx, ord: usize, cam: usize) -> bool {
        let inst = hooks::shot_instance(ord);
        if inst == 0 {
            return false;
        }
        let db = hooks::instance_database(inst);
        if db == 0 || engine::vtable(db) != game::VT_DATABASE {
            return false;
        }
        let odf = hooks::ord_odf(ord);
        let Some(k) = self.kind(odf) else { return false };
        let (sod, emitters, look) = (k.sod.clone(), k.emitters, k.look.clone());
        // Without the particles module the engine's own draw keeps the emitters going.
        if emitters && !r3d_core::particles::live() {
            return false;
        }
        let Some(model) = scene::model(&sod) else { return false };
        // Entity::Render: the instance's frustum test (which takes the scale the class's
        // Submit has just set), and the on-screen byte, as stock writes it.
        if !game::frustum_test(inst, cam) {
            return true;
        }
        hooks::mark_on_screen(ord);
        let s = game::instance_scale(inst);
        let mut world: M34 = hooks::transform(ord);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let clock = pose::Clock::of_instance(inst, v.time);
        let p = pose::pose(&model.sod, &model.info, &world, clock, pose::auto(v.eye));
        let a = look.ambient;
        // The `team*` groups (the mines' whole hull, the pods') in the owner's team
        // colour, as stock shows them.
        let owner = hooks::shot_owner(ord);
        let team = if owner != 0 && game::team_colours_on() { Some(game::team_colour(owner)) } else { None };
        let params = Params { alpha: 1.0, team, ambient: v3(a, a, a * 1.1), self_illum: look.self_illum, lit: true };
        scene::draw(v, &model, &p, &params);
        if emitters {
            // Keyed by the shot and its class, so a new shot at a freed address starts
            // its own trails.
            let key = hooks::shot_key(ord);
            r3d_core::particles::feed(key, &sod, &world, v);
        }
        if look.glow > 0.0 {
            // The heart: the shown meshes' centre, the model's radius.
            let (mut c, mut n, mut r) = (V3::ZERO, 0.0f32, 0.0f32);
            for part in &model.parts {
                if !p.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                let w = &p.world[part.node];
                c = c + w.transform_point(part.centre);
                n += 1.0;
                r = r.max(part.radius * w.r[0].len().max(w.r[1].len()).max(w.r[2].len()));
            }
            if n > 0.0 {
                let c = c * (1.0 / n);
                let seed = (r3d_core::clock::entity_key(ord) as u32 >> 4).wrapping_mul(2654435761) >> 22;
                let ph = seed as f32 * 0.0061;
                let pulse = 1.0 + look.pulse * sinf(v.time * look.rate + ph);
                let h = look.size * r.max(1.0);
                fx.glow(v, c, h, look.min_px, look.col, look.glow * pulse, look.star, v.time * 0.4 + ph, r * 0.6, look.light * pulse, look.reach);
            }
        }
        self.drawn = self.drawn.wrapping_add(1);
        let first = !self.said.iter().any(|x| x == odf) && self.said.len() < 64;
        if first || (v.main && self.drawn % 600 == 0) {
            if first {
                self.said.push(String::from(odf));
            }
            log!(
                "ordnance: drawing the model of {} ({}, scale {:.2} {:.2} {:.2}) at {:.0} {:.0} {:.0}",
                odf, sod, s.x, s.y, s.z, world.p.x, world.p.y, world.p.z
            );
        }
        true
    }
}
