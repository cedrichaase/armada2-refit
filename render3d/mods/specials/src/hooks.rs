//! The specials module's engine glue (README.md, "specials", "Hooks"). The module has no
//! hook to install: it reads `SpecialObjectEffect::mAllEffects` and its effects, and
//! takes `Step::Specials` from the frame's step table, which hands back to the engine's
//! own `RenderAll` whenever the module is off or its particles are not live. Reads only,
//! and the engine's own `Render(cam)` of an effect it leaves to the engine.
//!
//! Stock walks `mAllEffects` (a list whose head node pointer is at 0x7a8c0c; each
//! node's +8 is an effect) and calls each effect's `Render(cam)` (vtable slot 0).
use r3d_core::engine::{self, field, field_f32, field_u8};
use r3d_core::math::M34;
use alloc::string::String;
use alloc::vec::Vec;
use crate::syms::sym;

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { crate::syms::TABLE.engine().addr(s) }

/// The list's head node pointer (`mAllEffects` + 4).
pub(crate) fn effect_head() -> usize { at(sym::EFFECT_HEAD) }
pub(crate) fn fn_get_name() -> usize { at(sym::FN_GET_NAME) } // SpecialObjectEffect::GetEffectName(), thiscall: char*
pub(crate) fn fn_prj_odf() -> usize { at(sym::FN_PRJ_ODF) } // cPrjID::GetOdfName() const, thiscall: char*
pub(crate) fn fn_world_transform() -> usize { at(sym::FN_WORLD_TRANSFORM) } // Entity::GetWorldTransform(ST3D_Node*) const, thiscall, returns Matrix34 by hidden pointer
/// `Render` (slot 0) of the classes: the no-op base, the timed one (every `Timed*`,
/// `BoardingParty`, `LineOfSightEffect`) and `TimedChangeTeam`'s.
pub(crate) fn fn_render_base() -> usize { at(sym::FN_RENDER_BASE) }
pub(crate) fn fn_render_timed() -> usize { at(sym::FN_RENDER_TIMED) }
pub(crate) fn fn_render_change_team() -> usize { at(sym::FN_RENDER_CHANGE_TEAM) }
pub fn vt_change_team() -> usize { at(sym::VT_CHANGE_TEAM) } // TimedChangeTeam (Computer Override)
pub fn vt_crew_disruption() -> usize { at(sym::VT_CREW_DISRUPTION) } // TimedCrewDisruption (Psionic Disruption)
pub fn vt_drain() -> usize { at(sym::VT_DRAIN) } // TimedDrain (Energy Dissipator, micro-organisms)

/// Offsets in a timed effect.
const E_OBJECT: usize = 0x8; // the object the effect is on
const E_INSTANCE: usize = 0x2c;
const E_DATABASE: usize = 0xac; // instance +0x80
const E_TARGET: usize = 0xb0;
const E_NODE: usize = 0xc8; // TimedChangeTeam: the target's node it sits on
const E_TIME_LEFT: usize = 0x28;
const E_DEAD: usize = 0xc;

/// What an effect's `Render` is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderKind {
    /// The no-op base: nothing to draw.
    Base,
    /// A timed effect's: its instance placed on the target.
    Timed,
    /// `TimedChangeTeam`'s: placed on a node of the target.
    ChangeTeam,
    /// A class we do not know: the engine's.
    Other,
}

/// The class of an effect's `Render` (vtable slot 0).
pub fn render_kind(e: usize) -> RenderKind {
    match field(engine::vtable(e), 0) {
        fn_render_base() => RenderKind::Base,
        fn_render_timed() => RenderKind::Timed,
        fn_render_change_team() => RenderKind::ChangeTeam,
        _ => RenderKind::Other,
    }
}

/// The address of an effect's `Render`, for the survey's log.
pub fn render_fn(e: usize) -> usize {
    field(engine::vtable(e), 0)
}

/// Every effect in `mAllEffects`, in the list's order.
pub fn effects() -> Vec<usize> {
    let mut out = Vec::new();
    let head = field(effect_head(), 0);
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

pub fn effect_name(e: usize) -> String {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_get_name());
        let p = f(e);
        if p.is_null() {
            return String::new();
        }
        String::from(crate::rt::cstr_at(p, 64))
    }
}

/// The ODF stem of the ordnance that made an effect (`overrid`), or "": the effect's
/// weapon class (`+0x4`, a `WeaponClass`) keeps its ordnance class at `+0x18`
/// (`OrdnanceClass::Find` of its `ordName`), whose cPrjID (`+0x150`) names the ODF.
pub fn ordnance_odf(e: usize) -> String {
    let wc = field(e, 4);
    let oc = if wc != 0 { field(wc, 0x18) } else { 0 };
    let prj = if oc != 0 { field(oc, 0x150) } else { 0 };
    if prj == 0 {
        return String::new();
    }
    let name = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_prj_odf());
        let p = f(prj);
        if p.is_null() { "" } else { crate::rt::cstr_at(p, 260) }
    };
    let s = name.rsplit(['\\', '/']).next().unwrap_or(name);
    crate::rt::lower(s.split('.').next().unwrap_or(s))
}

/// `Entity::GetWorldTransform(node)`: the node's world matrix on the entity.
pub fn world_of_node(target: usize, node: usize) -> M34 {
    let mut m = M34::IDENTITY;
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *mut M34, usize) -> *mut M34 = core::mem::transmute(fn_world_transform());
        f(target, &mut m, node);
    }
    m
}

/// The engine's own `Render(cam)` of one effect.
pub fn engine_render(e: usize, cam: usize) {
    unsafe {
        let r: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(e), 0));
        r(e, cam);
    }
}

/// The effect's embedded `ST3D_Instance` (`+0x2c`), the key its emitters are fed under.
pub fn instance_key(e: usize) -> usize {
    e + E_INSTANCE
}

/// The database the weapon gave the instance (`+0xac`).
pub fn database(e: usize) -> usize {
    field(e, E_DATABASE)
}

/// The visual target the cloud is placed on (`+0xb0`).
pub fn target(e: usize) -> usize {
    field(e, E_TARGET)
}

/// The target's node a `TimedChangeTeam` sits on (`+0xc8`).
pub fn node(e: usize) -> usize {
    field(e, E_NODE)
}

/// The object the effect is on (`+0x8`).
pub fn object(e: usize) -> usize {
    field(e, E_OBJECT)
}

/// Whether the effect has ended (`+0xc`).
pub fn dead(e: usize) -> bool {
    field_u8(e, E_DEAD) != 0
}

/// Seconds left (`+0x28`).
pub fn time_left(e: usize) -> f32 {
    field_f32(e, E_TIME_LEFT)
}

/// Whether a target can carry a node's cloud: not flagged (`+0x27`), and shown (`+0x14`
/// bit 3), as `TimedChangeTeam::Render` tests.
pub fn node_target_ok(target: usize) -> bool {
    field_u8(target, 0x27) == 0 && field(target, 0x14) & 8 != 0
}

/// An instance's bounding radius (`+0x40`), 0 without an instance.
pub fn bounding_radius(inst: usize) -> f32 {
    if inst != 0 { field_f32(inst, 0x40) } else { 0.0 }
}
