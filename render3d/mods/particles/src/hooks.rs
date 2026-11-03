//! The engine glue of the particles module (README.md, "Hooks"): every address, call
//! site, vtable slot and field it reads or patches, and the two detours. Nothing else in
//! the module names one (`render3d/lint-layers.sh --strict src/mods/particles`).
//!
//! Hooks (installed at the module's first switch-on, through `hooks`, behind its
//! gate; a shut gate hands every call to what was there before):
//!
//! | Symbol | Kind | What it is for |
//! |---|---|---|
//! | `ST3D_DeviceDirectX8::RenderParticle` (vtable 0x6bc6ac, slot 50, 0x6262f0) | slot | drops the engine's draw of a particle handed over while the engine's current instance (`Storm3D+0x100`) is an object whose emitters we own, or the global list (instance 0), in a view of ours |
//! | `GameObject::RenderInternal` (0x4d544c) | call | the call of `ST3D_Instance::RenderSpriteNodeEffect` (0x62ebd0): skipped while one of our views runs, since we draw every object's billboard effect ourselves |
//!
//! What it reads: `GameObject::objectList`'s objects (the instance's radius), a craft
//! instance's damage bytes (written by its `Update`, vtable slot 14),
//! `Ordnance::ordnanceList`'s chain pulsar and Manheim shots (the free emitter each
//! carries), a freighter's cargo, a nebula's hidden flag, intensity and fade, and a game
//! object's billboard effect (`GameObject+0x3c`).
use super::content;
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Prev, Site};
use r3d_core::engine::{self, field, field_f32, field_u8, storm3d};
use r3d_core::math::V3;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use r3d_core::engine::DX8_VTABLE as VT_DEVICE_DX8;
use crate::syms::sym;

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { crate::syms::TABLE.engine().addr(s) }
const SLOT_RENDER_PARTICLE: usize = 50;
pub(crate) fn fn_render_particle() -> usize { at(sym::FN_RENDER_PARTICLE) }
const ENG_CURRENT_INSTANCE: usize = 0x100;

pub(crate) fn site_billboard() -> usize { at(sym::SITE_BILLBOARD) } // GameObject::RenderInternal: call RenderSpriteNodeEffect
pub(crate) fn fn_effect() -> usize { at(sym::FN_EFFECT) } // ST3D_Instance::RenderSpriteNodeEffect(nd, size, pull), thiscall
pub(crate) fn fn_sphere() -> usize { at(sym::FN_SPHERE) } // Entity::GetBoundingSphere() const, thiscall: centre, radius
const OBJ_EFFECT: usize = 0x3c;

/// The damage systems' bytes in a craft instance, in `content::SYSTEMS`' order. A node is
/// hidden while its byte is non-zero (the system is online), except `borg`, shown while
/// it is (`CraftInstance::RenderInternal` 0x4cb0e0).
const SYSTEM_BYTES: [usize; 6] = [0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1];
const SYSTEM_BORG: usize = 5;
pub(crate) fn vt_craft_instance() -> usize { at(sym::VT_CRAFT_INSTANCE) }
/// `CraftInstance::Update(const GameObject*)` (0x4cb390): the systems into the bytes.
const SLOT_UPDATE: usize = 14;
pub(crate) fn ordnance_list() -> usize { at(sym::ORDNANCE_LIST) } // Ordnance::ordnanceList, a std::list (head node at +4)
pub(crate) fn vt_chain_pulsar() -> usize { at(sym::VT_CHAIN_PULSAR) }
pub(crate) fn vt_manheim() -> usize { at(sym::VT_MANHEIM) }
const CHAIN_EMITTER: usize = 0xdc; // ChainPulsar: the free emitter (`crpulsar`)
const MANHEIM_EMITTER: usize = 0xd4; // ManheimEffect: its free emitter (`gemini`)
/// `Freighter` (vtable 0x6b26a8): the resource carried (`+0x25c`, an `eResource`: 3 is
/// latinum, 6 none), the amount (`+0x258`) and the class's capacities, one float per
/// resource (`+0x254`, a pointer).
pub(crate) fn vt_freighter() -> usize { at(sym::VT_FREIGHTER) }
const FREIGHTER_CAPACITY: usize = 0x254;
const FREIGHTER_AMOUNT: usize = 0x258;
const FREIGHTER_KIND: usize = 0x25c;
const RESOURCE_LATINUM: usize = 3;

pub fn vt_nebula() -> usize { at(sym::VT_NEBULA) }
pub fn vt_latinum() -> usize { at(sym::VT_LATINUM) }
pub(crate) fn fn_fade_out() -> usize { at(sym::FN_FADE_OUT) } // GameObjectInstance::ComputeFadeOut(float&) const, thiscall
const NEB_HIDDEN: usize = 0x1b0;
const NEB_INTENSITY: usize = 0x1d0;

const OBJ_KEY: usize = 0x28;
const INST_RADIUS: usize = 0x40;
const INST_DATABASE: usize = 0x80;
const INST_CLASS: usize = 0x98;
const CLASS_DATABASE: usize = 0x1dc;

static mut GATE: Option<Gate> = None;
static mut PREV_RP: Prev = Prev(0);
static mut PREV_EFFECT: Prev = Prev(0);
/// Our view is running: the engine's particles of owned objects are dropped.
static mut SUPPRESS: bool = false;
/// Skip the billboard effect's engine call while one of our views runs.
static mut SKIP: bool = false;
static mut BILLBOARDS: bool = false;
static mut OWNED: BTreeSet<usize> = BTreeSet::new();
static mut DROPPED: u32 = 0;

fn open() -> bool {
    unsafe { GATE.is_some_and(|g| g.open()) }
}

#[allow(static_mut_refs)]
unsafe extern "thiscall" fn render_particle(dev: usize, pt: usize) {
    unsafe {
        if open() && SUPPRESS {
            let inst = field(storm3d(), ENG_CURRENT_INSTANCE);
            // An owned object's list, or the global list (inst 0: the chain pulsar's and
            // Manheim trails, ours too).
            if inst == 0 || OWNED.contains(&inst) {
                DROPPED = DROPPED.wrapping_add(1);
                return;
            }
        }
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(PREV_RP.addr());
        f(dev, pt)
    }
}

unsafe extern "thiscall" fn billboard_stub(inst: usize, nd: usize, size: f32, pull: f32) {
    unsafe {
        if open() && SKIP {
            return;
        }
        let f: unsafe extern "thiscall" fn(usize, usize, f32, f32) = core::mem::transmute(PREV_EFFECT.addr());
        f(inst, nd, size, pull)
    }
}

/// Installs both hooks, once (the module's first switch-on). The particle hook is one
/// the module cannot draw without: the engine's draw of an owned object's particles
/// would double ours.
pub fn install(h: &mut Hooks) {
    unsafe { GATE = Some(h.gate()) };
    let owner = h.owner();
    let rp = Hook {
        owner,
        used_by: "particles' draw of an owned object's particles",
        kind: Kind::Slot { vt: VT_DEVICE_DX8, slot: SLOT_RENDER_PARTICLE },
        site: Site { symbol: "ST3D_DeviceDirectX8::RenderParticle", addr: VT_DEVICE_DX8, expect: fn_render_particle() },
        to: render_particle as *const () as usize,
        chain: true,
    };
    match h.install(rp) {
        Some(p) => unsafe { PREV_RP = p },
        None => h.stand_down("RenderParticle is not stock's"),
    }
    let bb = Hook {
        owner,
        used_by: "particles' billboard effects",
        kind: Kind::Call,
        site: Site { symbol: "GameObject::RenderInternal", addr: site_billboard(), expect: fn_effect() },
        to: billboard_stub as *const () as usize,
        chain: false,
    };
    if let Some(p) = h.install(bb) {
        unsafe {
            PREV_EFFECT = p;
            BILLBOARDS = true;
        }
    }
}

/// Whether the billboard effects' call is ours: only then does the module draw them
/// (otherwise the engine's call draws them).
pub fn billboards_hooked() -> bool {
    unsafe { BILLBOARDS }
}

/// One of our views is running: the engine's draws of owned objects' particles, and of
/// every object's billboard effect, are dropped.
pub fn suppress(on: bool) {
    unsafe {
        SUPPRESS = on;
        SKIP = on;
    }
}

/// The instances whose emitters we own this frame (their engine draws are dropped).
#[allow(static_mut_refs)]
pub fn set_owned(insts: impl Iterator<Item = usize>) {
    let owned = unsafe { &mut OWNED };
    owned.clear();
    owned.extend(insts);
}

/// Switched off: nothing owned, nothing counted.
#[allow(static_mut_refs)]
pub fn clear() {
    unsafe {
        SUPPRESS = false;
        SKIP = false;
        OWNED.clear();
        DROPPED = 0;
    }
}

pub fn dropped() -> u32 {
    unsafe { DROPPED }
}

// -- Game objects and their instances ----------------------------------------------

/// What names an object with its address in the simulation's table.
pub fn object_key(o: usize) -> (usize, u32) {
    (o, field(o, OBJ_KEY) as u32)
}

pub fn instance_radius(inst: usize) -> f32 {
    field_f32(inst, INST_RADIUS)
}

pub fn is_freighter(o: usize) -> bool {
    engine::vtable(o) == vt_freighter()
}

/// The latinum spray's rate: `cargo / capacity` while the hold carries latinum, else 0,
/// as `Freighter::RenderInternal` sets it. Reads only.
pub fn cargo_rate(o: usize) -> f32 {
    if o == 0 || engine::vtable(o) != vt_freighter() || field(o, FREIGHTER_KIND) != RESOURCE_LATINUM {
        return 0.0;
    }
    let caps = field(o, FREIGHTER_CAPACITY);
    if caps == 0 {
        return 0.0;
    }
    let cap = field_f32(caps, RESOURCE_LATINUM * 4);
    let amount = field_f32(o, FREIGHTER_AMOUNT);
    if cap > 0.0 && amount.is_finite() {
        (amount / cap).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The damage nodes a craft instance hides (bit per `content::SYSTEMS` entry), or `0xff`
/// when it shows another database than the class's main one (none of its nodes). 0 for
/// an instance that is not a craft's. The bytes are written by the instance's `Update`,
/// which `GameObject::Render` calls before drawing; an object a module draws instead
/// never gets it, so it is called here, through the vtable (the bench's Scene.asi wraps
/// it). It only reads the craft.
pub fn hidden_systems(o: usize, inst: usize) -> u8 {
    if engine::vtable(inst) != vt_craft_instance() {
        return 0;
    }
    unsafe {
        let up: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(inst), SLOT_UPDATE * 4));
        up(inst, o);
    }
    let class = field(inst, INST_CLASS);
    if field(inst, INST_DATABASE) != field(class, CLASS_DATABASE) {
        return 0xff;
    }
    let mut h = 0u8;
    for (s, off) in SYSTEM_BYTES.iter().enumerate() {
        let b = field_u8(inst, *off);
        let hide = if s == SYSTEM_BORG { b == 0 } else { b != 0 };
        if hide {
            h |= 1 << s;
        }
    }
    h
}

/// The shots that carry a free emitter into the global list: the chain pulsar's and
/// the Manheim effect's, with the emitter's name.
pub fn free_emitter_shots() -> Vec<(usize, &'static str)> {
    let mut ords = Vec::new();
    let head = field(ordnance_list(), 4);
    if head != 0 {
        let mut n = field(head, 0);
        let mut guard = 0;
        while n != head && n != 0 && guard < 100_000 {
            guard += 1;
            let o = field(n, 8);
            n = field(n, 0);
            match engine::vtable(o) {
                vt_chain_pulsar() if field(o, CHAIN_EMITTER) != 0 => ords.push((o, content::CHAIN_EMITTER)),
                vt_manheim() if field(o, MANHEIM_EMITTER) != 0 => ords.push((o, content::MANHEIM_EMITTER)),
                _ => {}
            }
        }
    }
    ords
}

// -- Billboard effects -------------------------------------------------------------

/// An object's billboard effect: the node data's sprite name, or None.
pub fn billboard_effect(o: usize) -> Option<&'static str> {
    if o == 0 {
        return None;
    }
    let nd = field(o, OBJ_EFFECT);
    if nd == 0 {
        return None;
    }
    Some(unsafe { crate::rt::cstr_at(field(nd, 8) as *const u8, 64) })
}

/// `Entity::GetBoundingSphere`: centre and radius.
pub fn bounding_sphere(o: usize) -> Option<(V3, f32)> {
    let s: [f32; 4] = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const [f32; 4] = core::mem::transmute(fn_sphere());
        let p = f(o);
        if p.is_null() {
            return None;
        }
        core::ptr::read_unaligned(p)
    };
    Some((V3 { x: s[0], y: s[1], z: s[2] }, s[3]))
}

// -- Nebulae -----------------------------------------------------------------------

/// Whether an object is a nebula or a latinum cloud, whose puffs we draw.
pub fn is_nebula_class(o: usize) -> bool {
    let vt = engine::vtable(o);
    vt == vt_nebula() || vt == vt_latinum()
}

pub fn is_nebula(o: usize) -> bool {
    engine::vtable(o) == vt_nebula()
}

/// `Nebula::Render`'s hidden flag.
pub fn nebula_hidden(o: usize) -> bool {
    field_u8(o, NEB_HIDDEN) != 0
}

pub fn nebula_intensity(o: usize) -> f32 {
    field_f32(o, NEB_INTENSITY)
}

/// `ComputeFadeOut` of a nebula's instance: Some(factor) while the eye is near enough.
pub fn nebula_fade(inst: usize) -> Option<f32> {
    let mut f = 1.0f32;
    let near: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = unsafe { core::mem::transmute(fn_fade_out()) };
    if unsafe { near(inst, &mut f) } != 0 { Some(f) } else { None }
}
