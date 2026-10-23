//! Specials: `Step::Specials`, the special-object effects (`SpecialObjectEffect::
//! RenderAll`, 0x60be90), drawn by us. See README.md, "specials".
//!
//! Stock walks `SpecialObjectEffect::mAllEffects` (a list whose head node pointer is at
//! 0x7a8c0c; each node's +8 is an effect) and calls each effect's `Render(cam)` (vtable
//! slot 0). Only the timed effects draw anything, and only those whose embedded
//! `ST3D_Instance` (+0x2c) was given a database (+0xac) and a visual target (+0xb0) by
//! the weapon that made them: an instance of the weapon's `particleArt` SOD, a single
//! cloud emitter, placed on the target and rendered with `ST3D_Instance::Render`, which
//! also simulates and draws its particle list. Here those emitters go to the particles
//! module (`particles::feed`), so their particles are ours, and each kind gets a look
//! of its own on top (`look.rs`).
pub mod look;

use super::{Module, Step};
use crate::engine::{self, field, field_u8};
use crate::engine::obj;
use crate::frame::steps::View;
use crate::log;
use crate::math::M34;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

/// The list's head node pointer (`mAllEffects` + 4).
const EFFECT_HEAD: usize = 0x7a8c0c;
const FN_GET_NAME: usize = 0x60be10; // SpecialObjectEffect::GetEffectName(), thiscall: char*
const FN_PRJ_ODF: usize = 0x6593a0; // cPrjID::GetOdfName() const, thiscall: char*
const FN_WORLD_TRANSFORM: usize = 0x4cff90; // Entity::GetWorldTransform(ST3D_Node*) const, thiscall, returns Matrix34 by hidden pointer
/// `Render` (slot 0) of the classes: the no-op base, the timed one (every `Timed*`,
/// `BoardingParty`, `LineOfSightEffect`) and `TimedChangeTeam`'s.
const FN_RENDER_BASE: usize = 0x49ea70;
const FN_RENDER_TIMED: usize = 0x60c460;
const FN_RENDER_CHANGE_TEAM: usize = 0x60e090;
pub const VT_CHANGE_TEAM: usize = 0x6bbe9c; // TimedChangeTeam (Computer Override)
pub const VT_CREW_DISRUPTION: usize = 0x6bbed4; // TimedCrewDisruption (Psionic Disruption)
pub const VT_DRAIN: usize = 0x6bbf64; // TimedDrain (Energy Dissipator, micro-organisms)

/// Offsets in a timed effect.
const E_OBJECT: usize = 0x8; // the object the effect is on
const E_INSTANCE: usize = 0x2c;
const E_DATABASE: usize = 0xac; // instance +0x80
const E_TARGET: usize = 0xb0;
const E_NODE: usize = 0xc8; // TimedChangeTeam: the target's node it sits on
const E_TIME_LEFT: usize = 0x28;
const E_DEAD: usize = 0xc;

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
        let ini = crate::config::ini();
        Specials {
            on: ini.int("Specials", "Enable", 1) != 0,
            survey: ini.int("Specials", "Survey", 1) != 0,
            reload: ini.int("Specials", "Reload", 1) != 0,
            hooked: false,
            sods: BTreeMap::new(),
            seen: BTreeSet::new(),
            born: BTreeMap::new(),
            frame: None,
            frames: 0,
            log_every: ini.int("Specials", "LogEvery", 600).max(0) as u32,
            fed: 0,
            fallback: 0,
            fx: Vec::new(),
            engine: Vec::new(),
            look: look::Looks::new(&ini),
        }
    }
}

/// Every effect in `mAllEffects`, in the list's order.
fn effects() -> Vec<usize> {
    let mut out = Vec::new();
    let head = field(EFFECT_HEAD, 0);
    if head == 0 {
        return out;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 100_000 {
        guard += 1;
        let e = field(n, 8);
        if e != 0 {
            out.push(e);
        }
        n = field(n, 0);
    }
    out
}

fn effect_name(e: usize) -> String {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_GET_NAME);
        let p = f(e);
        if p.is_null() {
            return String::new();
        }
        String::from(crate::rt::cstr_at(p, 64))
    }
}

/// The ODF stem of the ordnance that made an effect (`overrid`), or "".
fn ordnance_odf(e: usize) -> String {
    let wc = field(e, 4);
    let oc = if wc != 0 { field(wc, 0x18) } else { 0 };
    let prj = if oc != 0 { field(oc, 0x150) } else { 0 };
    if prj == 0 {
        return String::new();
    }
    let name = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_PRJ_ODF);
        let p = f(prj);
        if p.is_null() { "" } else { crate::rt::cstr_at(p, 260) }
    };
    let s = name.rsplit(['\\', '/']).next().unwrap_or(name);
    crate::rt::lower(s.split('.').next().unwrap_or(s))
}

/// `Entity::GetWorldTransform(node)`: the node's world matrix on the entity.
fn world_of_node(target: usize, node: usize) -> M34 {
    let mut m = M34::IDENTITY;
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *mut M34, usize) -> *mut M34 = core::mem::transmute(FN_WORLD_TRANSFORM);
        f(target, &mut m, node);
    }
    m
}

/// The engine's own `Render(cam)` of one effect.
fn engine_render(e: usize, cam: usize) {
    unsafe {
        let r: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(e), 0));
        r(e, cam);
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
        let ord = ordnance_odf(e);
        let art = if ord.is_empty() { None } else { crate::assets::odf(&ord).and_then(|o| o.get_str("particleArt").map(String::from)) };
        let sod = art.clone().filter(|a| crate::assets::sod(a).is_some());
        log!("specials: database {:#x} of '{}' (ordnance {}): {}", db, effect_name(e), if ord.is_empty() { "?" } else { ord.as_str() }, match (&sod, &art) {
            (Some(s), _) => s.as_str(),
            (None, Some(_)) => "its particleArt SOD does not load; the engine draws it",
            (None, None) => "no particleArt; the engine draws it",
        });
        self.sods.insert(db, sod.clone());
        sod
    }

    /// What stock's `Render` would place, and where: None when it draws nothing.
    fn placement(&self, e: usize, render: usize) -> Option<(usize, M34)> {
        let db = field(e, E_DATABASE);
        let target = field(e, E_TARGET);
        if db == 0 || target == 0 {
            return None;
        }
        match render {
            FN_RENDER_TIMED => Some((target, obj::transform(target))),
            FN_RENDER_CHANGE_TEAM => {
                let node = field(e, E_NODE);
                if node == 0 || field_u8(target, 0x27) != 0 || field(target, 0x14) & 8 == 0 {
                    return None;
                }
                Some((target, world_of_node(target, node)))
            }
            _ => None,
        }
    }

    /// Once a frame, before the scene reads the lights (the particles module has
    /// simulated by then): the effects, their emitters fed to the particles module.
    fn gather(&mut self, v: &View) {
        let list = effects();
        if self.survey {
            for &e in &list {
                let render = field(engine::vtable(e), 0);
                self.survey(e, render);
            }
        }
        self.fx.clear();
        self.engine.clear();
        if !self.on || !crate::mods::particles::live() {
            return;
        }
        let mut alive: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut objects: BTreeSet<usize> = BTreeSet::new();
        if !list.is_empty() {
            engine::for_each_in_list(engine::OBJECT_LIST, |o| {
                if o != 0 {
                    objects.insert(o);
                }
            });
        }
        for &e in &list {
            let vt = engine::vtable(e);
            let render = field(vt, 0);
            match render {
                FN_RENDER_BASE => continue,
                FN_RENDER_TIMED | FN_RENDER_CHANGE_TEAM => {}
                _ => {
                    // A class we do not know: as stock.
                    self.engine.push(e);
                    continue;
                }
            }
            let placed = self.placement(e, render);
            if let Some((_, world)) = placed {
                let db = field(e, E_DATABASE);
                let Some(sod) = self.sod_of(e, db) else {
                    self.engine.push(e);
                    continue;
                };
                // The instance's emitters at the placement (the weapons never scale it).
                crate::mods::particles::feed(e + E_INSTANCE, &sod, &world, v);
                self.fed += 1;
            }
            // The look: for a kind we have one for, while the effect lasts, whether or
            // not stock shows its cloud this frame (between the shot and the hit the
            // override's cloud has no node to sit on).
            if look::kind(vt).is_none() || field_u8(e, E_DEAD) != 0 {
                continue;
            }
            let o = field(e, E_OBJECT);
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
                        if target != 0 { obj::odf_name(target) } else { "-" }, engine::field_f32(e, E_TIME_LEFT)
                    );
                    self.born.insert(k, v.time);
                    v.time
                }
            };
            self.fx.push(Fx { effect: e, vt, target: if target != 0 { target } else { victim }, victim, world, left: engine::field_f32(e, E_TIME_LEFT), age: (v.time - born).max(0.0) });
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

    fn survey(&mut self, e: usize, render: usize) {
        let vt = engine::vtable(e);
        let name = effect_name(e);
        let db = field(e, E_DATABASE);
        let key = (vt, name.clone(), db != 0 && matches!(render, FN_RENDER_TIMED | FN_RENDER_CHANGE_TEAM));
        if self.seen.contains(&key) {
            return;
        }
        let target = if render == FN_RENDER_BASE { 0 } else { field(e, E_TARGET) };
        log!(
            "specials: survey: effect vtable {:#x} (Render {:#x}) '{}', database {:#x}, target {:#x} ({}), left {:.1} s",
            vt, render, name, if render == FN_RENDER_BASE { 0 } else { db }, target,
            if target != 0 { obj::odf_name(target) } else { "-" },
            if render == FN_RENDER_BASE { 0.0 } else { engine::field_f32(e, E_TIME_LEFT) }
        );
        self.seen.insert(key);
    }
}

impl Module for Specials {
    fn name(&self) -> &'static str {
        "specials"
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
    fn lights(&mut self, v: &View) {
        if self.frame != Some(v.frame) {
            self.frame = Some(v.frame);
            self.frames += 1;
            if self.reload && self.frames % 120 == 0 {
                let ini = crate::config::ini();
                self.look.reload(&ini);
                let on = ini.int("Specials", "Enable", 1) != 0;
                if on != self.on {
                    log!("specials: Enable={} (Render3D.ini read again)", on as u32);
                    self.on = on;
                }
            }
            self.gather(v);
        }
        if self.on && crate::mods::particles::live() {
            self.look.lights(v, &self.fx);
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        // Without our particles nothing would draw the emitters: the engine's step.
        if !self.on || !crate::mods::particles::live() {
            return false;
        }
        match s {
            Step::Specials => {
                for &e in &self.engine {
                    engine_render(e, v.cam);
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
