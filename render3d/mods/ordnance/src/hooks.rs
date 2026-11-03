//! The engine glue of the ordnance module (README.md, "Hooks"): every address, call
//! site, vtable slot and field it reads or patches, and its detours. Nothing else in the
//! module names one (`render3d/lint-layers.sh --strict src/mods/ordnance`). Reads only,
//! except the one write `Ordnance::RenderAll` itself makes (`shot_parties`) and the
//! colour of a torpedo's light in the engine's registry (`set_record_colour`).
//!
//! Hooks (installed through `hooks` at the module's first switch-on, or for the
//! per-class one, when a class is first met; every detour starts with the gate, and a
//! shut gate hands the call to what was there before):
//!
//! | Symbol | Kind | What it is for |
//! |---|---|---|
//! | `WarpSpriteEffect::Render` (0x59cc6b) | call | its `ST3D_Sprite::DrawScaled3D` of stock's warp sprite: noted for our flash instead, while one of our views runs and the flash is on |
//! | `Entity::Render` (slot 11 of an ordnance vtable) | slot, per class, lazily | the instance's draw of a shot whose class the engine submits for us (mines, pods, beams' models): ours while our ordnance step submits it, else the engine's |
//!
//! Not hooked, called: `RenderWarpSpriteEffects` (the body's call at 0x598423, which the
//! engine still runs to move and remove the effects), each class's `Submit` (vtable +0x5c)
//! for the shots we do not draw ourselves.
use super::Ordnance;
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Prev, Site};
use r3d_core::engine::{field, field_f32, field_u8, hook, rd, storm3d, vtable};
use r3d_core::frame::steps::View;
use r3d_core::math::{M34, V3};
use alloc::vec::Vec;

pub(crate) fn fn_entity_get() -> usize { at(sym::FN_ENTITY_GET) } // Entity::Get(int handle), cdecl
pub(crate) fn fn_obj_even_if_destroyed() -> usize { at(sym::FN_OBJ_EVEN_IF_DESTROYED) } // GameObject::GetObjEvenIfDestroyed(int), cdecl
pub(crate) fn fn_prj_odf() -> usize { at(sym::FN_PRJ_ODF) } // cPrjID::GetOdfName() const, thiscall: char*
pub(crate) fn fn_transform() -> usize { at(sym::FN_TRANSFORM) } // Entity::GetTransform() const, thiscall: Matrix34*
pub(crate) fn fn_odf_name() -> usize { at(sym::FN_ODF_NAME) } // GameObject::GetOdfName() const, thiscall
pub(crate) fn fn_shock_radius() -> usize { at(sym::FN_SHOCK_RADIUS) } // ShockwaveExplosion::GetCurrentRadius() const, thiscall

/// The live ordnance: `Ordnance::ordnanceList` (a `std::list` at 0x771fa8, its head
/// node's pointer at +4), each node's +8 the ordnance.
pub(crate) fn ordnance_head() -> usize { at(sym::ORDNANCE_HEAD) }
pub(crate) fn cull_distance() -> usize { at(sym::CULL_DISTANCE) } // cfgOBJECT_CULLING_DISTANCE, f32
const SLOT_SUBMIT: usize = 0x5c; // Ordnance vtable: Submit(ST3D_Camera*)

// Stock's Submit functions this module draws in their place (`StockSubmit`).
pub(crate) fn sub_phaser() -> usize { at(sym::SUB_PHASER) }
pub(crate) fn sub_pulse() -> usize { at(sym::SUB_PULSE) }
pub(crate) fn sub_photon() -> usize { at(sym::SUB_PHOTON) }
pub(crate) fn sub_plasma() -> usize { at(sym::SUB_PLASMA) }
pub(crate) fn sub_poleron() -> usize { at(sym::SUB_POLERON) }
pub(crate) fn sub_artillery() -> usize { at(sym::SUB_ARTILLERY) }
pub(crate) fn sub_srt() -> usize { at(sym::SUB_SRT) } // ShieldRemovingTorpedo
pub(crate) fn sub_chain() -> usize { at(sym::SUB_CHAIN) } // ChainPulsar: its sprite, and its emitter node's Render

// The warp flash: the call to ST3D_Sprite::DrawScaled3D in WarpSpriteEffect::Render.
pub(crate) fn site_warp_sprite() -> usize { at(sym::SITE_WARP_SPRITE) }
use r3d_core::engine::sprite::FN_DRAW_SCALED_3D as FN_DRAW_SCALED;
use crate::syms::sym;

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { crate::syms::TABLE.engine().addr(s) }
pub(crate) fn site_warp() -> usize { at(sym::SITE_WARP) } // RenderWarpSpriteEffects(cam), cdecl, in the body
pub(crate) fn fn_warp() -> usize { at(sym::FN_WARP) }

/// `Entity::Render(ST3D_Camera*)`, slot 11 of every ordnance vtable: the instance's
/// frustum test (instance slot 3), the on-screen byte +0x25, the instance's draw
/// (instance slot 5).
pub(crate) fn fn_entity_render() -> usize { at(sym::FN_ENTITY_RENDER) }
const SLOT_RENDER: usize = 11;

// An ordnance object (a shot): its class, instance, owner and target handles, the ends
// of a beam or a bolt's head and tail, the colours its `Submit` tints with.
const SHOT_INSTANCE: usize = 4;
const SHOT_CLASS: usize = 0x34;
const SHOT_LEFT: usize = 0xac;
const SHOT_OWNER: usize = 0x5c;
const SHOT_TARGET: usize = 0x60;
const SHOT_START: usize = 0xbc;
const SHOT_END: usize = 0xc8;
const SHOT_HUE: usize = 0xd0;
const SHOT_HUE_B: usize = 0xd4;
const SHOT_TINT: usize = 0xf0;
const SHOT_ON_SCREEN: usize = 0x25;
// An ordnance class: its light, life span, sprite, sizes and `cPrjID`.
const CLASS_LIGHT: usize = 0x10;
const CLASS_LIFE: usize = 0x1c;
const CLASS_SPRITE: usize = 0x12c;
const CLASS_LENGTH: usize = 0x144;
const CLASS_WIDTH: usize = 0x148;
const CLASS_PRJ: usize = 0x150;
// A sprite: its texture (name at +8), first frame's rectangle, colour.
const SPRITE_TEXTURE: usize = 0x58;
const SPRITE_RECT: usize = 0x38;
const SPRITE_COLOUR: usize = 0x24;
// A point light: its ODF colour; the registry (Storm3D+0x60): record +0 the light, +4 its colour.
const LIGHT_COLOUR: usize = 0xf4;
const REGISTRY: usize = 0x60;
pub(crate) fn vt_point() -> usize { at(sym::VT_POINT) }
pub(crate) fn vt_phaser() -> usize { at(sym::VT_PHASER) }
// A phaser: two state bytes (spent, firing).
const PHASER_SPENT: usize = 0x27;
const PHASER_FIRING: usize = 0x24;
// A game object: flags (bit 2: a game object), the bytes `RenderAll` tests.
const ENT_FLAGS: usize = 0x14;
const OBJ_GONE: usize = 0x113;
const OBJ_DEAD: usize = 0x112;
const OBJ_FORGET: usize = 0x10f;
const OBJ_HIDDEN: usize = 0x183;
const OBJ_INSTANCE: usize = 4;
const OBJ_CLASS: usize = 0x40;
const OBJ_POS: usize = 0xac;
const OBJ_VEL: usize = 0xdc;
const INST_DATABASE: usize = 0x80;
const INST_SPHERE: usize = 0x40;
// A warp effect: its handle (+0), position (+4), distance (+0x10).
const EFFECT_DISTANCE: usize = 0x10;
// A shockwave explosion: its class (+0x30, with `cPrjID` +0x24 and the fade factor +0xe0)
// and the radius it ends at (+0x50).
const WAVE_CLASS: usize = 0x30;
const WAVE_CLASS_PRJ: usize = 0x24;
const WAVE_CLASS_FADE: usize = 0xe0;
const WAVE_END: usize = 0x50;

// -- State of the detours --------------------------------------------------------------

static mut GATE: Option<Gate> = None;
static mut HOOKS: Option<Hooks> = None;
static mut PREV_WARP_SPRITE: Prev = Prev(0);
static mut WARP_HOOKED: bool = false;
/// The warp flash is ours this view (`Warp.On`): else the engine's sprite is drawn.
static mut WARP_ON: bool = false;
/// Flashes noted by the warp hook in this view: (effect, position).
static mut NOTED: Vec<(usize, V3)> = Vec::new();
/// Per vtable whose slot 11 is ours: what was there.
static mut CLASS_SLOTS: Vec<(usize, Prev)> = Vec::new();
/// Set by the ordnance step around the engine's `Submit` of a shot it hands over.
static mut IN_SUBMIT: bool = false;
static mut MODULE: *mut Ordnance = core::ptr::null_mut();
static mut VIEW: *const View = core::ptr::null();

fn open() -> bool {
    unsafe { GATE.is_some_and(|g| g.open()) }
}

/// The module and the view being drawn, while one is (for the detours).
pub fn current() -> Option<(&'static mut Ordnance, &'static View)> {
    unsafe {
        if MODULE.is_null() || VIEW.is_null() {
            None
        } else {
            Some((&mut *MODULE, &*VIEW))
        }
    }
}

/// The module, for the one thing the others ask of it (`draw_wave`): None while it is
/// not set up.
pub fn module() -> Option<&'static mut Ordnance> {
    unsafe { if MODULE.is_null() { None } else { Some(&mut *MODULE) } }
}

pub fn set_module(m: *mut Ordnance) {
    unsafe { MODULE = m };
}

pub fn set_view(v: Option<&View>) {
    unsafe { VIEW = v.map_or(core::ptr::null(), |v| v as *const View) };
}

pub fn set_in_submit(on: bool) {
    unsafe { IN_SUBMIT = on };
}

pub fn set_warp_on(on: bool) {
    unsafe { WARP_ON = on };
}

// -- The hooks -------------------------------------------------------------------------

/// In `WarpSpriteEffect::Render`, in place of its `DrawScaled3D` of stock's warp
/// sprite: notes the flash (at `pos`, the effect's +4) for our warp step.
unsafe extern "thiscall" fn warp_sprite(sprite: usize, cam: usize, pos: *const V3, size: f32) {
    unsafe {
        if open() && WARP_ON && !VIEW.is_null() && !pos.is_null() && NOTED.len() < 64 {
            let eff = pos as usize - 4;
            if object_of(rd::<i32>(eff)) != 0 {
                NOTED.push((eff, core::ptr::read_unaligned(pos)));
                return;
            }
        }
        let f: unsafe extern "thiscall" fn(usize, usize, *const V3, f32) = core::mem::transmute(PREV_WARP_SPRITE.addr());
        f(sprite, cam, pos, size);
    }
}

/// Slot 11 of the vtables `take_class_slot` took: `Entity::Render(ord, cam)`, drawn by
/// us while our ordnance step submits, else what was there.
unsafe extern "thiscall" fn ord_render(ord: usize, cam: usize) {
    unsafe {
        if open() && IN_SUBMIT {
            if let Some((m, v)) = current() {
                if m.models_draw(v, ord, cam) {
                    return;
                }
            }
        }
        let vt = vtable(ord);
        let prev = CLASS_SLOTS.iter().find(|(t, _)| *t == vt).map_or(fn_entity_render(), |(_, p)| p.addr());
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(prev);
        f(ord, cam);
    }
}

/// The module's hooks, once (its first switch-on): the warp flash's sprite call. The
/// per-class slots are taken when a class is first met (`take_class_slot`), through the
/// same registry handle, which is kept here.
pub fn install(h: &Hooks) {
    unsafe {
        GATE = Some(h.gate());
        HOOKS = Some(Hooks::for_module(h.owner(), "ordnance", h.gate()));
    }
    let owner = h.owner();
    #[allow(static_mut_refs)]
    let Some(hooks) = (unsafe { HOOKS.as_mut() }) else { return };
    let warp = Hook {
        owner,
        used_by: "the warp flash",
        kind: Kind::Call,
        site: Site { symbol: "WarpSpriteEffect::Render", addr: site_warp_sprite(), expect: FN_DRAW_SCALED },
        to: warp_sprite as *const () as usize,
        chain: false,
    };
    match hooks.install(warp) {
        Some(p) => unsafe {
            PREV_WARP_SPRITE = p;
            WARP_HOOKED = true;
        },
        None => crate::log!("ordnance: the warp flash's site is not stock's, the engine draws it"),
    }
}

pub fn warp_hooked() -> bool {
    unsafe { WARP_HOOKED }
}

/// For a class the engine submits: points slot 11 of its vtable at `ord_render`,
/// once, if it is still `Entity::Render`. False if it is not ours to draw.
#[allow(static_mut_refs)]
pub fn take_class_slot(ord: usize) -> bool {
    let vt = vtable(ord);
    let slots = unsafe { &mut CLASS_SLOTS };
    if slots.iter().any(|(t, _)| *t == vt) {
        return true;
    }
    let Some(hooks) = (unsafe { HOOKS.as_mut() }) else { return false };
    let hook = Hook {
        owner: hooks.owner(),
        used_by: "the shots' models",
        kind: Kind::Slot { vt, slot: SLOT_RENDER },
        site: Site { symbol: "Entity::Render", addr: vt, expect: fn_entity_render() },
        to: ord_render as *const () as usize,
        chain: false,
    };
    match hooks.install(hook) {
        Some(prev) => {
            slots.push((vt, prev));
            crate::log!("ordnance: slot 11 (Entity::Render) of vtable {:#x} ({}) is ours", vt, ord_odf(ord));
            true
        }
        None => false,
    }
}

/// `RenderWarpSpriteEffects(cam)` as the body's call has it (the engine's, or another
/// plugin's chained onto it); the effects it moves and removes are noted by the hook.
pub fn run_warp(cam: usize) {
    unsafe {
        let f: unsafe extern "C" fn(usize) = core::mem::transmute(hook::call_target(site_warp()).unwrap_or(fn_warp()));
        f(cam);
    }
}

/// Starts a warp step: forget the flashes noted before.
#[allow(static_mut_refs)]
pub fn clear_noted() {
    unsafe { NOTED.clear() };
}

/// The flashes the hook noted in the step just run, as (effect, position).
#[allow(static_mut_refs)]
pub fn noted() -> Vec<(usize, V3)> {
    unsafe { NOTED.clone() }
}

// -- The engine's calls ----------------------------------------------------------------

/// Calls `f` with each live ordnance, re-reading the list's head as `RenderAll` does.
pub fn for_each_ordnance(mut f: impl FnMut(usize)) {
    let mut node = field(unsafe { rd::<usize>(ordnance_head()) }, 0);
    let mut guard = 0;
    loop {
        let head = unsafe { rd::<usize>(ordnance_head()) };
        if node == 0 || node == head || guard > 100_000 {
            break;
        }
        f(field(node, 8));
        // The next node is read after the call, as RenderAll reads it.
        node = field(node, 0);
        guard += 1;
    }
}

pub fn entity_get(handle: i32) -> usize {
    unsafe {
        let f: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(fn_entity_get());
        f(handle)
    }
}

pub fn obj_even_if_destroyed(handle: i32) -> usize {
    unsafe {
        let f: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(fn_obj_even_if_destroyed());
        f(handle)
    }
}

/// A live game object by handle (Entity::Get, and flagged a game object, +0x14 bit 4,
/// with an instance and a class), or 0.
pub fn object_of(handle: i32) -> usize {
    let o = entity_get(handle);
    if o == 0 || field(o, ENT_FLAGS) & 4 == 0 || field(o, OBJ_INSTANCE) == 0 || field(o, OBJ_CLASS) == 0 {
        return 0;
    }
    o
}

/// A game object's bounding radius (its instance's sphere, +0x34 +12).
pub fn radius_of(o: usize) -> f32 {
    let r = field_f32(field(o, OBJ_INSTANCE), INST_SPHERE);
    if r > 0.01 { r } else { 50.0 }
}

/// A game object's position (+0xac) and velocity (+0xdc).
pub fn obj_pos(o: usize) -> V3 {
    unsafe { rd(o + OBJ_POS) }
}
pub fn obj_vel(o: usize) -> V3 {
    unsafe { rd(o + OBJ_VEL) }
}

pub fn camera_position(cam: usize) -> V3 {
    r3d_core::engine::sprite::camera_position(cam)
}

/// An entity's transform (`Entity::GetTransform`): position at +0x24.
pub fn transform(e: usize) -> M34 {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(fn_transform());
        let p = f(e);
        if p.is_null() { M34::IDENTITY } else { core::ptr::read_unaligned(p) }
    }
}

/// The ODF a cPrjID names (`tracbm.odf`), or "".
pub fn prj_odf(prj: usize) -> &'static str {
    if prj == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_prj_odf());
        crate::rt::cstr_at(f(prj), 260)
    }
}

/// An ordnance's ODF: its class keeps its cPrjID at +0x150.
pub fn ord_odf(ord: usize) -> &'static str {
    prj_odf(field(shot_class(ord), CLASS_PRJ))
}

pub fn odf_name(o: usize) -> &'static str {
    if o == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_odf_name());
        crate::rt::cstr_at(f(o), 260)
    }
}

/// `CanUserSee` (vtable slot 2) of any entity.
pub fn can_see(o: usize) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), 8));
        f(o) != 0
    }
}

/// The engine's `Submit` of a shot (vtable +0x5c).
pub fn submit(ord: usize, cam: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(vtable(ord), SLOT_SUBMIT));
        f(ord, cam);
    }
}

/// Where the class's `Submit` is, for the log and for `stock_submit`.
pub fn submit_addr(ord: usize) -> usize {
    field(vtable(ord), SLOT_SUBMIT)
}

/// The classes whose `Submit` is a sprite draw that `Who::Stock` has a copy of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockSubmit {
    Phaser,
    Pulse,
    Photon,
    Plasma,
    Poleron,
    Artillery,
    Srt,
    Chain,
}

pub fn stock_submit(ord: usize) -> Option<StockSubmit> {
    Some(match submit_addr(ord) {
        sub_phaser() => StockSubmit::Phaser,
        sub_pulse() => StockSubmit::Pulse,
        sub_photon() => StockSubmit::Photon,
        sub_plasma() => StockSubmit::Plasma,
        sub_poleron() => StockSubmit::Poleron,
        sub_artillery() => StockSubmit::Artillery,
        sub_srt() => StockSubmit::Srt,
        sub_chain() => StockSubmit::Chain,
        _ => return None,
    })
}

/// The colour a sprite-drawing `Submit` tints with: the shot's own for the torpedoes,
/// the artillery's for the artillery, the sprite's for the rest (the plasma cannon sets
/// none).
pub fn stock_colour(ord: usize, sub: StockSubmit, sprite: usize) -> V3 {
    match sub {
        StockSubmit::Photon | StockSubmit::Poleron | StockSubmit::Srt => unsafe { rd(ord + SHOT_HUE) },
        StockSubmit::Artillery => unsafe { rd(ord + SHOT_HUE_B) },
        _ => sprite_colour(sprite),
    }
}

// -- Shots -----------------------------------------------------------------------------

pub fn shot_class(ord: usize) -> usize {
    field(ord, SHOT_CLASS)
}

pub fn shot_instance(ord: usize) -> usize {
    field(ord, SHOT_INSTANCE)
}

/// What names a shot for the particles it feeds: its address and class, so a new shot
/// at a freed address starts its own trails.
pub fn shot_key(ord: usize) -> usize {
    ord ^ shot_class(ord).rotate_left(13)
}

pub fn shot_owner_handle(ord: usize) -> i32 {
    unsafe { rd(ord + SHOT_OWNER) }
}

/// The shot's owner / target as a live game object, or 0.
pub fn shot_owner(ord: usize) -> usize {
    object_of(shot_owner_handle(ord))
}

pub fn shot_target(ord: usize) -> usize {
    object_of(unsafe { rd::<i32>(ord + SHOT_TARGET) })
}

/// A special weapon's field is about its owner or its target.
pub fn shot_party(ord: usize, on_owner: bool) -> usize {
    if on_owner { shot_owner(ord) } else { shot_target(ord) }
}

/// The ends of a beam, or a bolt's head and tail.
pub fn shot_start(ord: usize) -> V3 {
    unsafe { rd(ord + SHOT_START) }
}
pub fn shot_end(ord: usize) -> V3 {
    unsafe { rd(ord + SHOT_END) }
}

/// A phaser's tint, and the second colour the pulse's `Submit` uses.
pub fn shot_tint(ord: usize) -> V3 {
    unsafe { rd(ord + SHOT_TINT) }
}
pub fn shot_hue_b(ord: usize) -> V3 {
    unsafe { rd(ord + SHOT_HUE_B) }
}

/// Seconds of life left.
pub fn shot_life_left(ord: usize) -> f32 {
    field_f32(ord, SHOT_LEFT)
}

/// Stock's `Entity::Render` writes the on-screen byte of what it draws.
pub fn mark_on_screen(ord: usize) {
    unsafe { core::ptr::write_volatile((ord + SHOT_ON_SCREEN) as *mut u8, 1) };
}

/// A shot's class light, if it is an `ST3D_Point_Light`.
pub fn shot_point_light(ord: usize) -> usize {
    let cls = shot_class(ord);
    let l = if cls != 0 { field(cls, CLASS_LIGHT) } else { 0 };
    if l != 0 && vtable(l) == vt_point() { l } else { 0 }
}

pub fn class_sprite(cls: usize) -> usize {
    field(cls, CLASS_SPRITE)
}

pub fn shot_sprite(ord: usize) -> usize {
    class_sprite(shot_class(ord))
}

/// A phaser shot's class: its life span.
pub fn class_life(cls: usize) -> f32 {
    field_f32(cls, CLASS_LIFE)
}

pub fn class_length(cls: usize) -> f32 {
    field_f32(cls, CLASS_LENGTH)
}

pub fn class_width(cls: usize) -> f32 {
    field_f32(cls, CLASS_WIDTH)
}

pub fn is_phaser(o: usize) -> bool {
    vtable(o) == vt_phaser()
}

/// A phaser beam that is firing and not spent.
pub fn phaser_active(o: usize) -> bool {
    field_u8(o, PHASER_SPENT) == 0 && field_u8(o, PHASER_FIRING) != 0
}

/// `Ordnance::RenderAll`'s tests of a shot's parties, and its one write: a shot whose
/// target has +0x10f set forgets it, and so does one whose owner is not +0x112 while
/// its target is. Kept, so the game runs as it does without us. Returns (owner, target)
/// as game objects (0: none).
pub fn shot_parties(ord: usize) -> (usize, usize) {
    // The owner, if it is a game object and not +0x113 (gone).
    let mut owner = entity_get(shot_owner_handle(ord));
    if owner != 0 && (field(owner, ENT_FLAGS) & 4 == 0 || field_u8(owner, OBJ_GONE) != 0) {
        owner = 0;
    }
    let mut target = entity_get(unsafe { rd::<i32>(ord + SHOT_TARGET) });
    if target != 0 && field(target, ENT_FLAGS) & 4 == 0 {
        target = 0;
    } else if target != 0 && field_u8(target, OBJ_FORGET) != 0 {
        unsafe { core::ptr::write_unaligned((ord + SHOT_TARGET) as *mut i32, 0) };
        target = 0;
    } else if target != 0 {
        let h = shot_owner_handle(ord);
        let o2 = obj_even_if_destroyed(h);
        if o2 != 0 && field_u8(o2, OBJ_GONE) == 0 && field_u8(o2, OBJ_DEAD) == 0 && field_u8(target, OBJ_DEAD) != 0 {
            unsafe { core::ptr::write_unaligned((ord + SHOT_TARGET) as *mut i32, 0) };
            target = 0;
        }
    }
    (owner, target)
}

/// Whether `RenderAll` submits the shot: its owner or its target is seen, or it is
/// within the culling distance of the eye and seen.
pub fn shot_shown(ord: usize, owner: usize, target: usize, eye: V3) -> bool {
    let cull = unsafe { rd::<f32>(cull_distance()) };
    let seen = |o: usize| o != 0 && field_u8(o, OBJ_HIDDEN) == 0 && can_see(o);
    seen(owner) || seen(target) || ((eye - transform(ord).p).len() <= cull && can_see(ord))
}

// -- Sprites and the lights' registry --------------------------------------------------

/// A sprite's texture name, as the engine holds it.
pub fn sprite_texture_name(sprite: usize) -> &'static str {
    let tex = field(sprite, SPRITE_TEXTURE);
    unsafe { crate::rt::cstr_at(field(tex, 8) as *const u8, 260) }
}

/// A sprite's first frame (u, v, w, h).
pub fn sprite_rect(sprite: usize) -> [f32; 4] {
    unsafe { rd(sprite + SPRITE_RECT) }
}

pub fn sprite_colour(sprite: usize) -> V3 {
    unsafe { rd(sprite + SPRITE_COLOUR) }
}

/// A point light's ODF colour.
pub fn light_colour(l: usize) -> V3 {
    unsafe { rd(l + LIGHT_COLOUR) }
}

/// Calls `f(record, light)` for each record of the engine's light registry
/// (`Storm3D+0x60`) this view.
pub fn for_each_light_record(mut f: impl FnMut(usize, usize)) {
    let head = field(storm3d(), REGISTRY);
    if head == 0 {
        return;
    }
    let mut node = field(head, 0);
    let mut guard = 0;
    while node != head && node != 0 && guard < 4096 {
        guard += 1;
        let rec = field(node, 8);
        node = field(node, 0);
        f(rec, field(rec, 0));
    }
}

/// The colour of a registry record (+4): this view's, not the class's.
pub fn set_record_colour(rec: usize, c: V3) {
    unsafe { core::ptr::write_unaligned((rec + 4) as *mut V3, c) };
}

// -- Instances and waves ---------------------------------------------------------------

/// An instance's database (+0x80), checked against the engine's by the caller.
pub fn instance_database(inst: usize) -> usize {
    field(inst, INST_DATABASE)
}

/// A warp effect's handle (+0), and its distance.
pub fn effect_handle(eff: usize) -> i32 {
    unsafe { rd(eff) }
}
pub fn effect_distance(eff: usize) -> f32 {
    field_f32(eff, EFFECT_DISTANCE)
}

/// A `ShockwaveExplosion`'s ODF, by its class's `cPrjID`.
pub fn wave_odf(expl: usize) -> &'static str {
    prj_odf(field(field(expl, WAVE_CLASS), WAVE_CLASS_PRJ))
}

/// A `ShockwaveExplosion`'s radius now, the radius it ends at, and the share of the way
/// after which its class fades it (`fadeFactor`, 0.5 without a class).
pub fn wave_state(ex: usize) -> (f32, f32, f32) {
    let cls = field(ex, WAVE_CLASS);
    let rad = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(fn_shock_radius());
        f(ex)
    };
    let end = field_f32(ex, WAVE_END);
    let fade = if cls != 0 { field_f32(cls, WAVE_CLASS_FADE) } else { 0.5 };
    (rad, end, fade)
}

/// The stem of an ODF name: `tracbm.odf` -> `tracbm`, lower case.
pub fn stem(odf: &str) -> alloc::string::String {
    let s = odf.rsplit(['\\', '/']).next().unwrap_or(odf);
    crate::rt::lower(s.split('.').next().unwrap_or(s))
}
