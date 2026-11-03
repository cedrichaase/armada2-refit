//! Specials: `Step::Specials`, the special-object effects (`SpecialObjectEffect::
//! RenderAll`, 0x60be90), drawn by us. See README.md, "specials".
//!
//! Stock walks `SpecialObjectEffect::mAllEffects` and calls each effect's `Render(cam)`
//! (`hooks.rs` has the addresses). Only the timed effects draw anything, and only those whose embedded
//! `ST3D_Instance` (+0x2c) was given a database (+0xac) and a visual target (+0xb0) by
//! the weapon that made them: an instance of the weapon's `particleArt` SOD, a single
//! cloud emitter, placed on the target and rendered with `ST3D_Instance::Render`, which
//! also simulates and draws its particle list. Here those emitters go to the particles
//! module (`particles::feed`), so their particles are ours, and each kind gets a look
//! of its own on top (`look.rs`).
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The runtime and its macros, as `crate::rt`, `crate::log` and `crate::sfmt` (r3d-core's).
#[allow(unused_imports)]
use r3d_core::{dump, log, rt, sfmt};

#[rustfmt::skip]
pub mod shaders;
pub mod content;
pub mod hooks;
mod syms;
pub mod look;

use r3d_core::module::{Module, Step};
use r3d_core::engine::obj;
use r3d_core::frame::steps::View;
use r3d_core::math::M34;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use hooks::{effect_name, RenderKind};
pub use hooks::{vt_change_team, vt_crew_disruption, vt_drain};

/// One effect drawn this frame, for the looks.
#[derive(Clone, Copy)]
pub struct Fx {
    pub effect: usize,
    pub vt: usize,
    pub target: usize,
    /// The object the weapon took hold of (`+0x8`), if it is still in the object list;
    /// the layer is drawn over its hull.
    pub victim: usize,
    pub world: M34,
    /// Seconds left (`+0x28`).
    pub left: f32,
    /// Seconds since we first saw it.
    pub age: f32,
}

pub struct Specials {
    on: bool,
    survey: bool,
    reload: bool,
    hooked: bool,
    /// Per database: the SOD its emitters are read from (the weapon ODF's
    /// `particleArt`), or None if it cannot be told (the effect is then the engine's).
    sods: BTreeMap<usize, Option<String>>,
    /// Survey: (vtable, name, has database) seen.
    seen: BTreeSet<(usize, String, bool)>,
    /// When each effect was first seen (sim time), by address and vtable.
    born: BTreeMap<(usize, usize), f32>,
    frame: Option<u32>,
    frames: u32,
    log_every: u32,
    fed: u32,
    fallback: u32,
    fx: Vec<Fx>,
    /// Effects the engine draws this frame (a class or a SOD we do not know).
    engine: Vec<usize>,
    look: look::Looks,
}

impl Default for Specials {
    fn default() -> Specials {
        Specials {
            on: true,
            survey: true,
            reload: true,
            hooked: false,
            sods: BTreeMap::new(),
            seen: BTreeSet::new(),
            born: BTreeMap::new(),
            frame: None,
            frames: 0,
            log_every: 600,
            fed: 0,
            fallback: 0,
            fx: Vec::new(),
            engine: Vec::new(),
            look: look::Looks::new(),
        }
    }
}

impl Specials {
    /// The SOD of a database: the effect's weapon class (`+0x4`, a `WeaponClass`)
    /// keeps its ordnance class at `+0x18` (`OrdnanceClass::Find` of its `ordName`),
    /// whose cPrjID (`+0x150`) names the ordnance ODF; its `particleArt` is the SOD
    /// the weapon gave the effect's instance.
    fn sod_of(&mut self, e: usize, db: usize) -> Option<String> {
        if let Some(s) = self.sods.get(&db) {
            return s.clone();
        }
        let ord = hooks::ordnance_odf(e);
        let art = if ord.is_empty() { None } else { r3d_core::assets::odf(&ord).and_then(|o| o.get_str("particleArt").map(String::from)) };
        let sod = art.clone().filter(|a| r3d_core::assets::sod(a).is_some());
        log!("specials: database {:#x} of '{}' (ordnance {}): {}", db, effect_name(e), if ord.is_empty() { "?" } else { ord.as_str() }, match (&sod, &art) {
            (Some(s), _) => s.as_str(),
            (None, Some(_)) => "its particleArt SOD does not load; the engine draws it",
            (None, None) => "no particleArt; the engine draws it",
        });
        self.sods.insert(db, sod.clone());
        sod
    }

    /// What stock's `Render` would place, and where: None when it draws nothing.
    fn placement(&self, e: usize, render: RenderKind) -> Option<(usize, M34)> {
        let db = hooks::database(e);
        let target = hooks::target(e);
        if db == 0 || target == 0 {
            return None;
        }
        match render {
            RenderKind::Timed => Some((target, obj::transform(target))),
            RenderKind::ChangeTeam => {
                let node = hooks::node(e);
                if node == 0 || !hooks::node_target_ok(target) {
                    return None;
                }
                Some((target, hooks::world_of_node(target, node)))
            }
            _ => None,
        }
    }

    /// Once a frame, before the scene reads the lights (the particles module has
    /// simulated by then): the effects, their emitters fed to the particles module.
    fn gather(&mut self, v: &View) {
        let list = hooks::effects();
        if self.survey {
            for &e in &list {
                self.survey(e);
            }
        }
        self.fx.clear();
        self.engine.clear();
        if !self.on || !r3d_core::particles::live() {
            return;
        }
        let mut alive: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut objects: BTreeSet<usize> = BTreeSet::new();
        if !list.is_empty() {
            r3d_core::engine::for_each_in_list(r3d_core::engine::object_list(), |o| {
                if o != 0 {
                    objects.insert(o);
                }
            });
        }
        for &e in &list {
            let vt = r3d_core::engine::vtable(e);
            let render = hooks::render_kind(e);
            match render {
                RenderKind::Base => continue,
                RenderKind::Timed | RenderKind::ChangeTeam => {}
                RenderKind::Other => {
                    // A class we do not know: as stock.
                    self.engine.push(e);
                    continue;
                }
            }
            let placed = self.placement(e, render);
            if let Some((_, world)) = placed {
                let db = hooks::database(e);
                let Some(sod) = self.sod_of(e, db) else {
                    self.engine.push(e);
                    continue;
                };
                // The instance's emitters at the placement (the weapons never scale it).
                r3d_core::particles::feed(hooks::instance_key(e), &sod, &world, v);
                self.fed += 1;
            }
            // The look: for a kind we have one for, while the effect lasts, whether or
            // not stock shows its cloud this frame (between the shot and the hit the
            // override's cloud has no node to sit on).
            if look::kind(vt).is_none() || hooks::dead(e) {
                continue;
            }
            let o = hooks::object(e);
            let victim = if objects.contains(&o) && !obj::expired(o) && obj::can_user_see(o) { o } else { 0 };
            let target = placed.map(|p| p.0).unwrap_or(0);
            let world = match placed {
                Some((_, w)) => w,
                None if victim != 0 => obj::transform(victim),
                None => continue,
            };
            let k = (e, vt);
            alive.insert(k);
            let born = match self.born.get(&k) {
                Some(b) => *b,
                None => {
                    log!(
                        "specials: {} begins on {} (cloud on {}), {:.1} s left",
                        effect_name(e), if victim != 0 { obj::odf_name(victim) } else { "-" },
                        if target != 0 { obj::odf_name(target) } else { "-" }, hooks::time_left(e)
                    );
                    self.born.insert(k, v.time);
                    v.time
                }
            };
            self.fx.push(Fx { effect: e, vt, target: if target != 0 { target } else { victim }, victim, world, left: hooks::time_left(e), age: (v.time - born).max(0.0) });
        }
        self.fallback += self.engine.len() as u32;
        self.born.retain(|k, _| alive.contains(k));
        if self.log_every > 0 && self.frames % self.log_every == 1 {
            log!("specials: {} effect(s) listed, {} fed to particles, {} drawn by the engine (frames since the last line)", list.len(), self.fed, self.fallback);
            self.fed = 0;
            self.fallback = 0;
            for f in &self.fx {
                log!(
                    "specials:   {} on {} (cloud on {}), {:.1} s left, {:.1} s old",
                    effect_name(f.effect), if f.victim != 0 { obj::odf_name(f.victim) } else { "-" }, obj::odf_name(f.target), f.left, f.age
                );
            }
        }
    }

    fn survey(&mut self, e: usize) {
        let vt = r3d_core::engine::vtable(e);
        let kind = hooks::render_kind(e);
        let base = kind == RenderKind::Base;
        let name = effect_name(e);
        let db = hooks::database(e);
        let key = (vt, name.clone(), db != 0 && matches!(kind, RenderKind::Timed | RenderKind::ChangeTeam));
        if self.seen.contains(&key) {
            return;
        }
        let target = if base { 0 } else { hooks::target(e) };
        log!(
            "specials: survey: effect vtable {:#x} (Render {:#x}) '{}', database {:#x}, target {:#x} ({}), left {:.1} s",
            vt, hooks::render_fn(e), name, if base { 0 } else { db }, target,
            if target != 0 { obj::odf_name(target) } else { "-" },
            if base { 0.0 } else { hooks::time_left(e) }
        );
        self.seen.insert(key);
    }
}

impl Module for Specials {
    fn name(&self) -> &'static str {
        "specials"
    }
    fn priority(&self) -> i32 {
        60
    }
    fn claims(&self) -> u32 {
        gpu::display::CLAIM_SPECIALS
    }
    fn rows(&self) -> &'static [crate::rt::settings::Setting] {
        &[]
    }
    /// No hook of its own: only the gate on what it reads. Not stock there, the module is
    /// not switched on and the engine draws its effects.
    fn hooks(&mut self, h: &mut r3d_core::hooks::Hooks) {
        if !h.ready(&syms::TABLE, syms::hook::SPECIALS_READS) {
            h.stand_down("the engine's reads are not stock");
        }
    }
    /// `[Specials]`: the module's own keys (`Enable`, `Survey`, `Reload`, `LogEvery`) and
    /// each kind's look. Read at load and at every settings change; `Reload=1` reads
    /// them again every 120 frames (the bench).
    fn configure(&mut self, ini: &crate::rt::Ini, _said: &mut r3d_core::module::Changes) {
        self.on = ini.int("Specials", "Enable", 1) != 0;
        self.survey = ini.int("Specials", "Survey", 1) != 0;
        self.reload = ini.int("Specials", "Reload", 1) != 0;
        self.log_every = ini.int("Specials", "LogEvery", 600).max(0) as u32;
        self.look.reload(ini);
    }
    fn device(&mut self) {
        if !self.hooked {
            self.hooked = true;
            log!("specials: {} (Render3D.ini [Specials] Enable)", if self.on { "on" } else { "off: the engine draws the special-object effects" });
        }
    }
    fn reset(&mut self) {
        self.look.reset();
    }
    /// Switched off live: the effects' ages and the frame's lists go. What the engine
    /// draws now is the engine's; a module switched on again counts ages from when it
    /// first sees an effect.
    fn off(&mut self) {
        self.born.clear();
        self.fx.clear();
        self.engine.clear();
    }
    fn lights(&mut self, v: &View) {
        if self.frame != Some(v.frame) {
            self.frame = Some(v.frame);
            self.frames += 1;
            if self.reload && self.frames % 120 == 0 {
                let ini = crate::rt::Ini::open(&crate::rt::game_path("Render3D.ini"));
                self.look.reload(&ini);
                let on = ini.int("Specials", "Enable", 1) != 0;
                if on != self.on {
                    log!("specials: Enable={} (Render3D.ini read again)", on as u32);
                    self.on = on;
                }
            }
            self.gather(v);
        }
        if self.on && r3d_core::particles::live() {
            self.look.lights(v, &self.fx);
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        // Without our particles nothing would draw the emitters: the engine's step.
        if !self.on || !r3d_core::particles::live() {
            return false;
        }
        match s {
            Step::Specials => {
                for &e in &self.engine {
                    hooks::engine_render(e, v.cam);
                }
                true
            }
            Step::Translucent => {
                self.look.draw(v, &self.fx);
                false
            }
            _ => false,
        }
    }

}
