//! The engine glue of the effects module (README.md, "Hooks"): every address, call
//! site, vtable value, global and struct offset it uses, behind named accessors, and
//! its detours. Nothing else in the module names one
//! (`render3d/lint-layers.sh --strict src/mods/effects`).
//!
//! Hooks (installed at the module's first switch-on, through `core_hooks`; each part
//! installs its own when its `Enable` is on and no plugin of its own is loaded):
//!
//! | Symbol | Kind | What it is for |
//! |---|---|---|
//! | `Craft::Explode` (0x4c6bfc), its call of `ST3D_Instance::Fragment` | call (chained) | a craft the module can draw as debris is noted instead of broken up by the engine |
//! | the Evolver's four calls of `Fragment` on its cocoon (0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4) | call, one hook each (chained) | the cocoon bursting is noted the same way |
//! | `BuildingEffect::SafeDestroyConstruction` (0x46a0f1), its call of `Fragment` | call (chained) | a cancelled construction's faces built so far are noted |
//! | `Craft::RepairSystem` (0x4c90c0), its call of `NodeParticleEffect::AddParticle` | call (chained) | the new tail of the particle list is noted as a weld |
//! | `NodeParticleEffect::Render` (0x473767), its call of `ST3D_Sprite::DrawScaled3D` | call (chained) | the particle's node is noted (in `esi`) and a weld, or every particle while the sparks are ours, is skipped |
//! | the four calls of `CreateShieldHit` (0x58bb95, 0x58cad8, 0x58d502, 0x58dd65) | call, one hook each (chained) | records which weapon made a hit and its damage (a passive tap, see below) |
//!
//! Every detour shuts to what was there before when the module's gate is shut. The
//! shields tap is passive: it only records, and calls what was there before either way,
//! so it needs no gate (a hit made while the module is off is remembered, which does no
//! harm and lets a hit in flight at a switch-on keep its damage). Debris: while the
//! gate is shut, or the part is off, `Fragment` goes to the engine, which breaks the
//! craft up itself.
//!
//! What it reads: the explosion list (head at 0x7367c4) and each explosion's class,
//! life and radius; `ShieldEffect`'s hit tree and each hit's model, handle, transform
//! and shields; the particle list of `NodeParticleEffect`; the bee colony's tree and each
//! bee; `BuildingEffect`'s tree and each `Info`; an instance's transform, radius and
//! scale; an object's flags, key, shield transform and shield level.
use crate::core_hooks::{Gate, Hook, Hooks, Kind, Prev, Site};
use crate::engine::{self, field, field_f32, field_m34, field_u8, field_v3, hook, obj, rd};
use crate::math::{M34, V3};

static mut GATE: Option<Gate> = None;

/// Whether the module is switched on: every detour starts with it.
pub fn open() -> bool {
    unsafe { GATE.is_some_and(|g| g.open()) }
}

/// Keeps the module's gate for the detours (the first thing `hooks()` does).
pub fn keep_gate(h: &Hooks) {
    unsafe { GATE = Some(h.gate()) };
}

// ---- shared reads --------------------------------------------------------------

/// A live game object by handle, as the plugins took one: `Entity::Get`, a game
/// object (flags bit 2), with an instance and a class. 0 if not.
pub fn object_of(handle: i32) -> usize {
    const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), static, cdecl
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(FN_ENTITY_GET) };
    let o = unsafe { f(handle) };
    if o == 0 || field(o, 0x14) & 4 == 0 || field(o, 4) == 0 || field(o, 0x40) == 0 {
        return 0;
    }
    o
}

/// The in-order walk of an `std::map`/`std::multimap` red-black tree as the engine
/// builds it (`head` its header node, `nil` the tree's `_Nil`): `f(node)` for each node.
pub fn tree_walk(head: usize, nil: usize, limit: u32, mut f: impl FnMut(usize)) {
    if head == 0 || nil == 0 {
        return;
    }
    let mut node = field(head, 0); // the leftmost: begin()
    let mut guard = 0;
    while node != 0 && node != head && guard < limit {
        guard += 1;
        f(node);
        // the in-order successor
        let mut next = field(node, 8);
        if next != nil {
            while field(next, 0) != nil {
                next = field(next, 0);
            }
            node = next;
        } else {
            let mut p = field(node, 4);
            while node == field(p, 8) {
                node = p;
                p = field(p, 4);
            }
            if field(node, 8) != p {
                node = p;
            }
        }
    }
}

/// An instance's world transform (`+0x44`).
pub fn instance_world(inst: usize) -> M34 {
    field_m34(inst, 0x44)
}

/// An instance's transform as twelve floats: three rows and the origin.
pub fn instance_matrix12(inst: usize) -> [f32; 12] {
    unsafe { rd(inst + 0x44) }
}

/// An instance's bounding radius.
pub fn instance_radius(inst: usize) -> f32 {
    field_f32(inst, 0x40)
}

/// An instance's centre (`+0x68`).
pub fn instance_centre(inst: usize) -> V3 {
    field_v3(inst, 0x68)
}

/// An object's own radius through its instance (the shield effects' reach).
pub fn object_radius(o: usize) -> f32 {
    field_f32(field(o, 4), 0x40)
}

/// An object's key (`+0x28`), a number that stays with it.
pub fn object_key(o: usize) -> u32 {
    field(o, 0x28) as u32
}

/// An object is hidden from the player (`+0x113`).
pub fn object_hidden(o: usize) -> bool {
    field_u8(o, 0x113) != 0
}

/// The engine's own `Render` of an instance (vtable slot 5), as stock makes it.
pub fn engine_render(inst: usize, cam: usize) {
    unsafe {
        let ri: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(inst), 0x14));
        ri(inst, cam);
    }
}

/// Whether the player sees everything: the map editor is on (the user profile's flag),
/// or the view is the one that shows everything.
pub fn sees_all() -> bool {
    const USER_PROFILE: usize = 0x76b5ac; // +0x84: the map editor is on
    const VIEW_RECORD: usize = 0x76b610;
    unsafe { field_u8(rd::<usize>(USER_PROFILE), 0x84) != 0 || rd::<u32>(VIEW_RECORD) == 2 }
}

/// A colour handed to a `Fragment` call: white for none.
fn colour_arg(colour: *const f32) -> V3 {
    if colour.is_null() { V3 { x: 1.0, y: 1.0, z: 1.0 } } else { unsafe { V3 { x: *colour, y: *colour.add(1), z: *colour.add(2) } } }
}

/// What a game feature is, by its vtable.
#[derive(Clone, Copy, PartialEq)]
pub enum Feature {
    Explosion,
    Shield,
    /// `NodeParticleEffect`: the engine's `Render` draws it.
    NodeParticle,
    Building,
    WorkerBee,
    DroneColony,
    Other,
}

pub fn feature_kind(f: usize) -> Feature {
    const VT_EXPLOSION_FEATURE: usize = 0x6afb30;
    const VT_SHIELD_EFFECT: usize = 0x6b0134;
    const VT_NODE_PARTICLE: usize = 0x6b001c;
    const VT_BUILDING: usize = 0x6afe4c;
    const VT_WORKER_BEE: usize = 0x6b017c;
    const VT_DRONE_COLONY: usize = 0x6afe98;
    match engine::vtable(f) {
        VT_EXPLOSION_FEATURE => Feature::Explosion,
        VT_SHIELD_EFFECT => Feature::Shield,
        VT_NODE_PARTICLE => Feature::NodeParticle,
        VT_BUILDING => Feature::Building,
        VT_WORKER_BEE => Feature::WorkerBee,
        VT_DRONE_COLONY => Feature::DroneColony,
        _ => Feature::Other,
    }
}

/// The engine's own `RenderChunks`, through its call site (a plugin chained there
/// runs too).
pub fn engine_chunks() {
    const SITE_CHUNKS: usize = 0x598446; // ST3D_GraphicsEngine::RenderChunks(), thiscall
    const FN_CHUNKS: usize = 0x62d470;
    let f = hook::call_target(SITE_CHUNKS).unwrap_or(FN_CHUNKS);
    unsafe {
        let r: unsafe extern "thiscall" fn(usize) = core::mem::transmute(f);
        r(engine::storm3d());
    }
}

// ---- explosions ----------------------------------------------------------------

pub mod explosions {
    use super::*;

    const EXPLOSION_HEAD: usize = 0x7367c4;
    const VT_FIREBALL: usize = 0x6afbd4;
    const VT_SHOCKWAVE: usize = 0x6afdac;
    const FN_SHOCK_RADIUS: usize = 0x467730;
    const SLOT_VISIBLE: usize = 2;
    const SLOT_SUBMIT: usize = 15;

    /// Calls `f` with every live explosion, in the list's order.
    pub fn each(mut f: impl FnMut(usize)) {
        let head = unsafe { rd::<usize>(EXPLOSION_HEAD) };
        if head == 0 {
            return;
        }
        let mut n = field(head, 0);
        let mut guard = 0;
        while n != 0 && n != head && guard < 100_000 {
            let next = field(n, 0);
            f(field(n, 8));
            n = next;
            guard += 1;
        }
    }

    pub fn is_fireball(e: usize) -> bool {
        engine::vtable(e) == VT_FIREBALL
    }

    pub fn is_shockwave(e: usize) -> bool {
        engine::vtable(e) == VT_SHOCKWAVE
    }

    /// The explosion class's `IsVisible` (slot 2).
    pub fn visible(e: usize) -> bool {
        unsafe {
            let vis: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(engine::vtable(e), SLOT_VISIBLE * 4));
            vis(e) != 0
        }
    }

    /// The engine's own submit of an explosion (slot 15), as stock makes it.
    pub fn submit(e: usize, cam: usize) {
        unsafe {
            let submit: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(e), SLOT_SUBMIT * 4));
            submit(e, cam);
        }
    }

    /// A fireball's light: its class's length and the time left, None when it has no
    /// class or is out of time.
    pub fn light_life(e: usize) -> Option<(f32, f32)> {
        let cls = field(e, 0x30);
        let len = engine::ff(cls, 0x4c);
        let left = engine::ff(e, 0xa8);
        if cls == 0 || !(len > 0.0) || !(left > 0.0) {
            return None;
        }
        Some((len, left))
    }

    /// The radius of the body an explosion is on (its entity's, `+0x40`), 0 if none.
    pub fn body_radius(e: usize) -> f32 {
        let ent = field(e, 4);
        if ent != 0 { engine::ff(ent, 0x40) } else { 0.0 }
    }

    /// A fireball as drawn: its radius, its class's length (2.5 with no class) and the
    /// time left.
    pub fn fireball(e: usize) -> (f32, f32, f32) {
        let cls = field(e, 0x30);
        let r0 = engine::ff(e, 0x34);
        let life = if cls != 0 { engine::ff(cls, 0x4c) } else { 2.5 };
        (r0, life, engine::ff(e, 0xa8))
    }

    /// A shockwave as drawn: whether it has a class, its radius now, its largest, its
    /// age and the fraction of the radius at which it starts to fade.
    pub struct Shock {
        pub has_class: bool,
        pub radius: f32,
        pub max: f32,
        pub age: f32,
        pub fade: f32,
    }

    pub fn shockwave(e: usize) -> Shock {
        let cls = field(e, 0x30);
        let radius: f32 = unsafe {
            let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_SHOCK_RADIUS);
            f(e)
        };
        Shock {
            has_class: cls != 0,
            radius,
            max: engine::ff(e, 0x50),
            age: engine::ff(e, 0x4c),
            fade: if cls != 0 { engine::ff(cls, 0xe0) } else { 0.3 },
        }
    }

    /// A shockwave's colour (its class's, `+0xd0`).
    pub fn shock_colour(e: usize) -> V3 {
        engine::rv3(field(e, 0x30) + 0xd0)
    }
}

// ---- the particle list of NodeParticleEffect (repair and sparks) -----------------

pub mod nodes {
    use super::*;

    const LIST: usize = 0x736cfc;
    const COUNT: usize = 0x736d00;
    const FN_WORLD_POS: usize = 0x4cff70; // Entity::GetWorldPosition(ST3D_Node*), thiscall
    const P_HANDLE: usize = 0x08;
    const P_NODE: usize = 0x0c;
    const P_LOCAL: usize = 0x10;
    const P_SPRITE: usize = 0x1c;
    const P_COLOUR: usize = 0x20;
    const P_SIZE: usize = 0x2c;
    const P_AGE: usize = 0x34;
    const P_LIFE: usize = 0x38;

    /// Calls `f` with every particle node, in the list's order.
    pub fn each(mut f: impl FnMut(usize)) {
        let head = unsafe { rd::<usize>(LIST) };
        if head == 0 {
            return;
        }
        let mut n = field(head, 0);
        let mut guard = 0;
        while n != 0 && n != head && guard < 1024 {
            guard += 1;
            let node = n;
            n = field(n, 0);
            f(node);
        }
    }

    pub fn count() -> u32 {
        unsafe { rd::<u32>(COUNT) }
    }

    /// Whether an address is the list's head (a signature check of the code that reads
    /// it).
    pub fn is_list(a: usize) -> bool {
        a == LIST
    }

    /// The list's newest node (the new tail), if there is one.
    pub fn tail() -> Option<usize> {
        let head = unsafe { rd::<usize>(LIST) };
        if head == 0 {
            return None;
        }
        let node = field(head, 4);
        if node == 0 || node == head { None } else { Some(node) }
    }

    pub fn handle(node: usize) -> i32 {
        field(node, P_HANDLE) as i32
    }
    /// The hardpoint node it sits at (0: a point in the object's frame).
    pub fn hardpoint(node: usize) -> usize {
        field(node, P_NODE)
    }
    pub fn local(node: usize) -> V3 {
        engine::rv3(node + P_LOCAL)
    }
    pub fn sprite(node: usize) -> usize {
        field(node, P_SPRITE)
    }
    pub fn colour(node: usize) -> [f32; 3] {
        unsafe { rd(node + P_COLOUR) }
    }
    pub fn size(node: usize) -> f32 {
        field_f32(node, P_SIZE)
    }
    pub fn age(node: usize) -> f32 {
        field_f32(node, P_AGE)
    }
    pub fn life(node: usize) -> f32 {
        field_f32(node, P_LIFE)
    }

    /// `Entity::GetWorldPosition(ST3D_Node*)` of a hardpoint.
    pub fn world_pos(o: usize, hp: usize) -> V3 {
        let mut out = V3::ZERO;
        unsafe {
            let f: unsafe extern "thiscall" fn(usize, *mut V3, usize) -> *mut V3 = core::mem::transmute(FN_WORLD_POS);
            f(o, &mut out, hp);
        }
        out
    }
}

// ---- repair: the welds ---------------------------------------------------------

pub mod repair {
    use super::*;

    const SITE_ADD: usize = 0x4c90c0;
    const FN_ADD: usize = 0x4733c0;
    const SITE_SPRITE: usize = 0x473767;
    const SITE_LIST_CHECK: usize = 0x473677; // NodeParticleEffect::Render's read of the list head
    use crate::engine::sprite::FN_DRAW_SCALED_3D as FN_SPRITE;

    static mut PREV_ADD: Prev = Prev(0);
    static mut PREV_SPRITE: Prev = Prev(0);
    /// `NodeParticleEffect::Render` holds the particle's node in `esi` at the call.
    static mut RP_NODE: usize = 0;
    /// Both hooks are in: the welds can be ours, and the sparks can take the rest.
    static mut HOOKED: bool = false;

    /// Whether the engine's draw of `NodeParticleEffect`'s sprites goes through our
    /// thunk (the sparks can only take them over then).
    pub fn hooked() -> bool {
        unsafe { HOOKED }
    }

    unsafe extern "C" fn add(name: *const u8, colour: *const f32, o: usize, hp: usize, size: f32, life: f32) {
        unsafe {
            let before = nodes::count();
            let f: unsafe extern "C" fn(*const u8, *const f32, usize, usize, f32, f32) = core::mem::transmute(PREV_ADD.addr());
            f(name, colour, o, hp, size, life);
            if !open() || !super::super::repair::taking() || nodes::count() != before.wrapping_add(1) {
                return;
            }
            if let Some(node) = nodes::tail() {
                super::super::repair::weld_added(node, o, hp, size, life);
            }
        }
    }

    unsafe extern "thiscall" fn sprite(sp: usize, cam: usize, pos: *const f32, size: f32) {
        unsafe {
            let node = RP_NODE;
            if open() && HOOKED && node != 0 && (super::super::sparks::takes() || super::super::repair::is_weld(node)) {
                return;
            }
            let f: unsafe extern "thiscall" fn(usize, usize, *const f32, f32) = core::mem::transmute(PREV_SPRITE.addr());
            f(sp, cam, pos, size);
        }
    }

    #[unsafe(naked)]
    unsafe extern "C" fn sprite_thunk() {
        core::arch::naked_asm!(
            "mov dword ptr [{node}], esi",
            "jmp {f}",
            node = sym RP_NODE,
            f = sym sprite,
        )
    }

    /// The `AddParticle` call in `Craft::RepairSystem` and the `DrawScaled3D` call in
    /// `NodeParticleEffect::Render`, chained. False (logged) if either is not as
    /// stock's.
    pub fn install(h: &mut Hooks) -> bool {
        let owner = h.owner();
        let ok_list = nodes::is_list(unsafe { rd::<usize>(SITE_LIST_CHECK) });
        if hook::call_target(SITE_ADD).is_none() || hook::call_target(SITE_SPRITE).is_none() {
            crate::log!("effects: repair: the call sites are not calls here: stock sparks");
            return false;
        }
        if !ok_list {
            crate::log!("effects: repair: NodeParticleEffect::Render differs: stock sparks");
            return false;
        }
        let add_hook = Hook {
            owner,
            used_by: "effects' repair welds",
            kind: Kind::Call,
            site: Site { symbol: "Craft::RepairSystem", addr: SITE_ADD, expect: FN_ADD },
            to: add as *const () as usize,
            chain: true,
        };
        let sprite_hook = Hook {
            owner,
            used_by: "effects' repair weld sprites",
            kind: Kind::Call,
            site: Site { symbol: "NodeParticleEffect::Render", addr: SITE_SPRITE, expect: FN_SPRITE },
            to: sprite_thunk as *const () as usize,
            chain: true,
        };
        unsafe {
            let a = h.install(add_hook);
            let s = h.install(sprite_hook);
            if let Some(p) = a {
                PREV_ADD = p;
            }
            if let Some(p) = s {
                PREV_SPRITE = p;
            }
            crate::log!(
                "effects: repair: AddParticle{} and DrawScaled3D{} hooked",
                if PREV_ADD.addr() != 0 && PREV_ADD.addr() != FN_ADD { " (chained)" } else { "" },
                if PREV_SPRITE.addr() != 0 && PREV_SPRITE.addr() != FN_SPRITE { " (chained)" } else { "" }
            );
            HOOKED = PREV_ADD.addr() != 0 && PREV_SPRITE.addr() != 0;
            HOOKED
        }
    }
}

// ---- shields -------------------------------------------------------------------

pub mod shields {
    use super::*;
    pub use super::{object_key, object_radius};

    const SHIELD_LIST: usize = 0x736de8;
    const SHIELD_NIL: usize = 0x736e38;
    const SHIELD_LOGICAL_DB: usize = 0x736ddc;
    const SHIELD_MATERIAL: usize = 0x736e2c; // what ShieldEffect::Render hands DrawShield
    const FN_DRAWSHIELD: usize = 0x474070;
    const FN_CREATE_HIT: usize = 0x4743b0;
    const SHIELD_TYPE_NAMES: usize = 0x6b00e0;
    const FN_GET_SHIELD_TYPE: usize = 0x474720;
    const HIT_SITES: [usize; 4] = [0x58bb95, 0x58cad8, 0x58d502, 0x58dd65];

    const HIT_MODEL: usize = 0x7c;
    const HIT_MODEL_RADIUS: usize = 0x40;
    const HIT_SCALE: usize = 0x28;
    const HIT_LEFT: usize = 0x84;
    const HIT_TIMED: usize = 0x88;
    const HIT_HANDLE: usize = 0x8c;
    const HIT_XFORM: usize = 0x90;
    const HIT_SHIELDS: usize = 0xc0;

    /// A hit the weapons' calls made: its id, the weapon's site (0 beam, 1 shot, 2 mine,
    /// 3 missile) and its damage.
    #[derive(Clone, Copy)]
    pub struct Made {
        pub id: i32,
        pub site: usize,
        pub damage: f32,
    }

    const NMADE: usize = 64;
    static mut MADE: [Made; NMADE] = [Made { id: -1, site: 0, damage: -1.0 }; NMADE];
    static mut MADE_NEXT: usize = 0;
    static mut PREV_CREATE: [Prev; 4] = [Prev(0); 4];
    static mut SH_ORD: usize = 0;
    static mut SH_SITE: usize = 0;

    /// What the weapons' calls noted of hit `id`, if it is still among the last ones.
    pub fn made_of(id: i32) -> Option<Made> {
        unsafe { (*core::ptr::addr_of!(MADE)).iter().copied().find(|m| m.id == id) }
    }

    /// The tap on `CreateShieldHit`: a passive one (see the module's table).
    unsafe extern "C" fn shield_hit(o: usize, m: *const f32, ty: i32, dur: f32, flags: i32) -> i32 {
        unsafe {
            let (site, ord) = (SH_SITE, SH_ORD);
            if site > 3 || PREV_CREATE[site].addr() == 0 {
                return -1;
            }
            let f: unsafe extern "C" fn(usize, *const f32, i32, f32, i32) -> i32 = core::mem::transmute(PREV_CREATE[site].addr());
            let id = f(o, m, ty, dur, flags);
            if id >= 0 && ord != 0 {
                let cls = field(ord, 0x34);
                let mut dmg = -1.0f32;
                if cls != 0 {
                    dmg = rd::<f32>(cls + 0xa8 + 0x14);
                    if !(dmg >= 0.0 && dmg < 100000.0) {
                        dmg = -1.0;
                    }
                }
                MADE[MADE_NEXT] = Made { id, site, damage: dmg };
                MADE_NEXT = (MADE_NEXT + 1) % NMADE;
            }
            id
        }
    }

    macro_rules! thunk {
        ($name:ident, $n:literal) => {
            #[unsafe(naked)]
            unsafe extern "C" fn $name() {
                core::arch::naked_asm!(
                    "mov dword ptr [{ord}], ebx",
                    concat!("mov dword ptr [{site}], ", $n),
                    "jmp {f}",
                    ord = sym SH_ORD,
                    site = sym SH_SITE,
                    f = sym shield_hit,
                )
            }
        };
    }
    thunk!(thunk0, 0);
    thunk!(thunk1, 1);
    thunk!(thunk2, 2);
    thunk!(thunk3, 3);

    /// The four weapon calls of `CreateShieldHit`, chained through the thunks.
    pub fn install(h: &mut Hooks) {
        let thunks: [usize; 4] = [thunk0 as *const () as usize, thunk1 as *const () as usize, thunk2 as *const () as usize, thunk3 as *const () as usize];
        let owner = h.owner();
        let mut names = alloc::string::String::new();
        for i in 0..4 {
            if hook::call_target(HIT_SITES[i]).is_none() {
                continue;
            }
            let hk = Hook {
                owner,
                used_by: "effects' shield hits",
                kind: Kind::Call,
                site: Site { symbol: "CreateShieldHit", addr: HIT_SITES[i], expect: FN_CREATE_HIT },
                to: thunks[i],
                chain: true,
            };
            if let Some(prev) = h.install(hk) {
                unsafe { PREV_CREATE[i] = prev };
                names.push(' ');
                names.push_str(super::super::content::SITE_NAME[i]);
                if prev.addr() != FN_CREATE_HIT {
                    names.push_str("(chained)");
                }
            }
        }
        crate::log!("effects: shields: the weapons' hits noted at:{}", names);
    }

    /// Walks `m_shieldList` in order (as `ShieldEffect::Render` does): (id, ShieldHit).
    pub fn walk(mut f: impl FnMut(i32, usize)) {
        let head = unsafe { rd::<usize>(SHIELD_LIST + 4) };
        let nil = unsafe { rd::<usize>(SHIELD_NIL) };
        tree_walk(head, nil, 4096, |node| f(field(node, 0xc) as i32, field(node, 0x14)));
    }

    /// The `k`th model of the engine's shield database (0 if it is not loaded).
    pub fn db(k: usize) -> usize {
        let db = unsafe { rd::<usize>(SHIELD_LOGICAL_DB) };
        if db == 0 { 0 } else { field(db, k * 4) }
    }

    /// Whether the engine's table of shell names is the one this reads.
    pub fn names_ok() -> bool {
        unsafe { rd::<usize>(FN_GET_SHIELD_TYPE + 4) == SHIELD_TYPE_NAMES }
    }

    /// The name of shell type `k` in the engine's table: None at the table's end.
    pub fn type_name(k: usize) -> Option<&'static str> {
        let p = unsafe { rd::<usize>(SHIELD_TYPE_NAMES + k * 4) };
        if p == 0 {
            return None;
        }
        Some(unsafe { crate::rt::cstr_at(p as *const u8, 31) })
    }

    /// What `ShieldEffect::Render` hands `DrawShield`, and the engine's draw of a hit.
    pub fn draw_hit(sh: usize, cam: usize) {
        let mat = unsafe { rd::<usize>(SHIELD_MATERIAL) };
        unsafe {
            let f: unsafe extern "thiscall" fn(usize, usize, usize) = core::mem::transmute(FN_DRAWSHIELD);
            f(sh, cam, mat);
        }
    }

    pub fn hit_model(sh: usize) -> usize {
        field(sh, HIT_MODEL)
    }
    pub fn hit_handle(sh: usize) -> i32 {
        field(sh, HIT_HANDLE) as i32
    }
    pub fn hit_timed(sh: usize) -> bool {
        field_u8(sh, HIT_TIMED) != 0
    }
    pub fn hit_left(sh: usize) -> f32 {
        field_f32(sh, HIT_LEFT)
    }
    pub fn hit_shields(sh: usize) -> f32 {
        field_f32(sh, HIT_SHIELDS)
    }
    pub fn hit_model_radius(sh: usize) -> f32 {
        field_f32(sh, HIT_MODEL_RADIUS)
    }
    /// The hit's scale along its third axis.
    pub fn hit_scale_z(sh: usize) -> f32 {
        field_f32(sh, HIT_SCALE + 8)
    }
    /// The hit's frame (a 3x3).
    pub fn hit_xform(sh: usize) -> [f32; 9] {
        unsafe { rd(sh + HIT_XFORM) }
    }

    /// The object's shield transform: a 3x3 and the offset (twelve floats).
    pub fn shield_xform(o: usize) -> [f32; 12] {
        unsafe { rd(field(o, 0x40) + 0x38) }
    }

    /// The shields' level of a ship as (now, most): None when the object has none.
    pub fn level(o: usize) -> Option<(f32, f32)> {
        if field(o, 0x14) & 8 != 0 { Some((field_f32(o, 0x1c8), field_f32(o, 0x1cc))) } else { None }
    }

    /// A ship's instance scale per axis, as `ApplyScaleEffects` set it.
    pub fn instance_scale(inst: usize) -> [f32; 3] {
        let s = engine::instance::instance_scale(inst);
        [s.x, s.y, s.z]
    }

    pub fn object_instance(o: usize) -> usize {
        obj::instance(o)
    }
}

// ---- debris (chunks) -----------------------------------------------------------

pub mod chunks {
    use super::*;
    use crate::engine::instance::{COCOON_DB, VT_COCOON};

    const SITE_EXPLODE_FRAGMENT: usize = 0x4c6bfc; // in Craft::Explode: call ST3D_Instance::Fragment
    const FN_FRAGMENT: usize = 0x62eb20;
    const SITE_BUILD_FRAGMENT: usize = 0x46a0f1; // in BuildingEffect::SafeDestroyConstruction: the same call
    const FN_CLASS_ODF: usize = 0x4ce370; // GameObjectClass::GetOdfName(), thiscall
    /// The Evolver's four calls of `Fragment` on its cocoon.
    const SITES_COCOON: [usize; 4] = [0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4];

    static mut PREV_FRAGMENT: Prev = Prev(0);
    static mut PREV_BUILD_FRAGMENT: Prev = Prev(0);
    static mut PREV_COCOON: [Prev; 4] = [Prev(0); 4];

    type FragmentFn = unsafe extern "thiscall" fn(usize, *const f32, usize, u32, i32, usize);

    unsafe extern "thiscall" fn fragment(inst: usize, colour: *const f32, vel: usize, smoking: u32, faces: i32, attractor: usize) {
        unsafe {
            if open() && super::super::chunks::note_craft(inst, faces, colour_arg(colour)) {
                return;
            }
            let f: FragmentFn = core::mem::transmute(PREV_FRAGMENT.addr());
            f(inst, colour, vel, smoking, faces, attractor);
        }
    }

    /// The Evolver's cocoon breaking up (faces -1, the whole of it): one detour per
    /// site, each calling what was at its own site.
    unsafe extern "thiscall" fn cocoon<const I: usize>(inst: usize, colour: *const f32, vel: usize, smoking: u32, faces: i32, attractor: usize) {
        unsafe {
            if open() && super::super::chunks::note_cocoon(inst, faces, colour_arg(colour)) {
                return;
            }
            let f: FragmentFn = core::mem::transmute(PREV_COCOON[I].addr());
            f(inst, colour, vel, smoking, faces, attractor);
        }
    }

    /// A construction cancelled or destroyed (`BuildingEffect::SafeDestroyConstruction`):
    /// the instance is the one embedded in the construction's `Info` (`+0x20`), its class
    /// at the `Info`'s `+0xd0`, and `faces` the faces built so far, which are the ones
    /// that break.
    unsafe extern "thiscall" fn build_fragment(inst: usize, colour: *const f32, vel: usize, smoking: u32, faces: i32, attractor: usize) {
        unsafe {
            if open() && super::super::chunks::note_building(inst, faces, colour_arg(colour)) {
                return;
            }
            let f: FragmentFn = core::mem::transmute(PREV_BUILD_FRAGMENT.addr());
            f(inst, colour, vel, smoking, faces, attractor);
        }
    }

    /// Whether an instance is the Evolver's cocoon (its class and its database).
    pub fn is_cocoon(inst: usize) -> bool {
        let db = field(inst, 0x80);
        engine::vtable(inst) == VT_COCOON && db != 0 && db == unsafe { rd::<usize>(COCOON_DB) }
    }

    /// The ODF name of the class a construction's `Info` (the instance at its `+0x20`)
    /// builds, without the extension; "" if it has none.
    pub fn building_odf(inst: usize) -> &'static str {
        if inst <= 0x20 {
            return "";
        }
        let class = field(inst - 0x20, 0xd0);
        if class == 0 {
            return "";
        }
        let s = unsafe {
            let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_CLASS_ODF);
            crate::rt::cstr_at(f(class), 260)
        };
        crate::assets::odf_stem(s)
    }

    /// Installs the hooks for the parts of the debris that are stock here. False when
    /// `Craft::Explode`'s own call is not stock's (the engine's chunks entirely).
    pub fn install(h: &mut Hooks) -> bool {
        let owner = h.owner();
        let explode = Hook {
            owner,
            used_by: "effects' debris of a dying craft",
            kind: Kind::Call,
            site: Site { symbol: "Craft::Explode", addr: SITE_EXPLODE_FRAGMENT, expect: FN_FRAGMENT },
            to: fragment as *const () as usize,
            chain: true,
        };
        if hook::call_target(SITE_EXPLODE_FRAGMENT).is_none() {
            crate::log!("effects: debris: Craft::Explode does not call Fragment here: the engine's chunks");
            return false;
        }
        let Some(prev) = h.install(explode) else { return false };
        unsafe { PREV_FRAGMENT = prev };
        crate::log!("effects: debris: Craft::Explode's Fragment{} hooked: our own chunks", if prev.addr() != FN_FRAGMENT { " (chained)" } else { "" });
        let detours: [usize; 4] = [cocoon::<0> as *const () as usize, cocoon::<1> as *const () as usize, cocoon::<2> as *const () as usize, cocoon::<3> as *const () as usize];
        let mut n = 0;
        for i in 0..4 {
            let hk = Hook {
                owner,
                used_by: "effects' debris of the Evolver's cocoon",
                kind: Kind::Call,
                site: Site { symbol: "Evolver::mDoRemoveConstructionEffect", addr: SITES_COCOON[i], expect: FN_FRAGMENT },
                to: detours[i],
                chain: true,
            };
            if let Some(p) = h.install(hk) {
                unsafe { PREV_COCOON[i] = p };
                n += 1;
            }
        }
        crate::log!("effects: debris: {} of the Evolver's {} cocoon Fragment calls ours", n, SITES_COCOON.len());
        if hook::call_target(SITE_BUILD_FRAGMENT).is_some() {
            let hk = Hook {
                owner,
                used_by: "effects' debris of a cancelled construction",
                kind: Kind::Call,
                site: Site { symbol: "BuildingEffect::SafeDestroyConstruction", addr: SITE_BUILD_FRAGMENT, expect: FN_FRAGMENT },
                to: build_fragment as *const () as usize,
                chain: true,
            };
            if let Some(p) = h.install(hk) {
                unsafe { PREV_BUILD_FRAGMENT = p };
                crate::log!("effects: debris: a cancelled construction's Fragment{} hooked: our own chunks", if p.addr() != FN_FRAGMENT { " (chained)" } else { "" });
            }
        } else {
            crate::log!("effects: debris: SafeDestroyConstruction does not call Fragment here: the engine's chunks for it");
        }
        true
    }
}

// ---- bees ----------------------------------------------------------------------

pub mod bees {
    use super::*;

    const BEE_NIL: usize = 0x736bc4; // the bees' multimap's _Nil
    const SLOT_BEE_LIST: usize = 0x40; // the colony's GetBeeList()

    const B_POS: usize = 0x68;
    const B_WIDTH: usize = 0x88;
    const B_STATE: usize = 0x8c;
    const B_SPOT: usize = 0xac;
    const B_OWNER: usize = 0xbc;
    const STATE_WELDING: u32 = 4;

    /// The bee's owner object (0 if none).
    pub fn owner(bee: usize) -> usize {
        object_of(field(bee, B_OWNER) as i32)
    }

    /// The colony's bees, in order.
    pub fn walk(colony: usize, mut f: impl FnMut(usize)) {
        let list = unsafe {
            let g: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(field(engine::vtable(colony), SLOT_BEE_LIST));
            g(colony)
        };
        let head = field(list, 4);
        let nil = unsafe { rd::<usize>(BEE_NIL) };
        tree_walk(head, nil, 1024, |node| f(node + 0x10));
    }

    pub fn welding(bee: usize) -> bool {
        field(bee, B_STATE) as u32 == STATE_WELDING
    }
    pub fn from(bee: usize) -> V3 {
        field_v3(bee, B_POS)
    }
    pub fn spot(bee: usize) -> V3 {
        field_v3(bee, B_SPOT)
    }
    pub fn width(bee: usize) -> f32 {
        field_f32(bee, B_WIDTH)
    }
}

// ---- construction ----------------------------------------------------------------

pub mod construction {
    use super::*;

    /// `ConstructionObject::Render`: draws nothing (the object loop's view of a station
    /// being built).
    const FN_CONSTRUCTION_OBJECT_RENDER: usize = 0x4a3790;
    const BE_HEAD: usize = 0x736bd4; // BuildingEffect::m_list: its head node
    const BE_NIL: usize = 0x736c1c; // that tree's _Nil

    const I_BUILDER: usize = 0x0; // the builder's handle
    const I_TEAMS: usize = 0x4; // teams that see it
    const I_COLOUR: usize = 0x8; // its colour at the start
    const I_INST: usize = 0x20; // the station's instance (a CraftInstance)
    const I_SCALE_HANDLE: usize = 0xb4; // the handle ApplyScaleEffects is asked for
    const I_FACES_TOTAL: usize = 0xc8; // faces of the finished station
    const I_FACES: usize = 0xcc; // faces built so far
    const I_CLASS: usize = 0xd0; // the station's class
    const I_SPEED: usize = 0xe0; // weldingSpeed: an edge's progress a second
    const I_RADIUS: usize = 0xe8; // weldingRadius: the spark sprite's half size
    const I_EDGES: usize = 0x100; // the welding edges' list head node
    // A welding edge's list node: start +0x14, end +0x20, progress +0x2c.
    const E_START: usize = 0x14;
    const E_END: usize = 0x20;
    const E_T: usize = 0x2c;

    const FN_TEAMS_VISIBLE: usize = 0x4d0070; // Entity::GetTeamsVisibleToUser(), cdecl
    const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
    const FN_TEAM_COLOUR: usize = 0x4d5000; // GameObject::GetTeamColor(), thiscall
    const FN_SCALE_EFFECTS: usize = 0x599760; // ApplyScaleEffects(ST3D_Instance*, handle), cdecl
    const FN_FACE_LIMIT: usize = 0x62ea80; // ST3D_Instance::SetRenderFaceLimit(int), thiscall
    const FN_CLASS_ODF: usize = 0x4ce370; // GameObjectClass::GetOdfName(), thiscall
    const FN_GET_CO: usize = 0x4afe80; // ConstructionRig::GetConstructionObject(), thiscall
    const S3D_TEAM: usize = 0x34; // Storm3D: the team colour of the instance drawn

    /// The station under construction as an object: its `Render` draws nothing.
    pub fn is_construction_object(o: usize) -> bool {
        field(engine::vtable(o), 11 * 4) == FN_CONSTRUCTION_OBJECT_RENDER
    }

    /// Walks `BuildingEffect::m_list` in order: each `Info`.
    pub fn each_info(mut f: impl FnMut(usize)) {
        let head = unsafe { rd::<usize>(BE_HEAD) };
        let nil = unsafe { rd::<usize>(BE_NIL) };
        tree_walk(head, nil, 1024, |node| {
            let info = field(node, 0x14);
            if info != 0 {
                f(info);
            }
        });
    }

    /// The builder object (0 if none).
    pub fn builder(info: usize) -> usize {
        object_of(field(info, I_BUILDER) as i32)
    }
    /// Whether a team that sees it is one the player sees.
    pub fn teams_seen(info: usize) -> bool {
        let teams = unsafe {
            let f: unsafe extern "C" fn() -> u32 = core::mem::transmute(FN_TEAMS_VISIBLE);
            f()
        };
        teams & field(info, I_TEAMS) as u32 != 0
    }
    pub fn faces(info: usize) -> i32 {
        field(info, I_FACES) as i32
    }
    pub fn faces_total(info: usize) -> i32 {
        field(info, I_FACES_TOTAL) as i32
    }
    pub fn speed(info: usize) -> f32 {
        field_f32(info, I_SPEED)
    }
    pub fn welding_radius(info: usize) -> f32 {
        field_f32(info, I_RADIUS)
    }
    /// The station's instance (embedded in the `Info`).
    pub fn instance(info: usize) -> usize {
        info + I_INST
    }
    /// Its colour at the start.
    pub fn colour(info: usize) -> V3 {
        field_v3(info, I_COLOUR)
    }
    /// The head node of the welding edges' list (0: none).
    pub fn edges_head(info: usize) -> usize {
        field(info, I_EDGES)
    }
    /// The list node after `n`.
    pub fn edge_next(n: usize) -> usize {
        field(n, 0)
    }
    pub fn edge_start(n: usize) -> V3 {
        engine::rv3(n + E_START)
    }
    pub fn edge_end(n: usize) -> V3 {
        engine::rv3(n + E_END)
    }
    pub fn edge_t(n: usize) -> f32 {
        field_f32(n, E_T)
    }

    /// The ODF name of the class an `Info` builds, without the extension; "" if none.
    pub fn class_odf(info: usize) -> &'static str {
        let class = field(info, I_CLASS);
        if class == 0 {
            return "";
        }
        let s = unsafe {
            let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_CLASS_ODF);
            crate::rt::cstr_at(f(class), 260)
        };
        crate::assets::odf_stem(s)
    }

    /// `GameInstance::DistanceCull(eye, pos)`.
    pub fn distance_culled(inst: usize, eye: &V3, pos: &V3) -> bool {
        unsafe {
            let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
            f(inst, eye, pos) != 0
        }
    }

    /// The builder's team colour, else the `Info`'s own.
    pub fn team_colour(info: usize, builder: usize) -> V3 {
        if builder != 0 {
            unsafe {
                let f: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(FN_TEAM_COLOUR);
                let p = f(builder);
                if p.is_null() { colour(info) } else { core::ptr::read_unaligned(p) }
            }
        } else {
            colour(info)
        }
    }

    /// As stock: the scale effects into the instance, its face limit, then its
    /// frustum test. Each writes only the effect's own instance.
    pub fn prepare(inst: usize, info: usize, faces: i32, cam: usize) -> bool {
        unsafe {
            let se: unsafe extern "C" fn(usize, usize) = core::mem::transmute(FN_SCALE_EFFECTS);
            se(inst, field(info, I_SCALE_HANDLE));
            let fl: unsafe extern "thiscall" fn(usize, i32) = core::mem::transmute(FN_FACE_LIMIT);
            fl(inst, faces);
        }
        crate::scene::instance::frustum_test(inst, cam)
    }

    /// The engine's own draw of the instance, as stock makes it (the team colour set
    /// for it first).
    pub fn engine_draw(inst: usize, colour: V3, cam: usize) {
        unsafe { core::ptr::write_unaligned((engine::storm3d() + S3D_TEAM) as *mut V3, colour) };
        engine_render(inst, cam);
    }

    /// The construction object's on-screen byte, which the HUD reads.
    pub fn mark_on_screen(builder: usize, shown: bool) {
        if builder != 0 && crate::scene::instance::render_fn(builder) == crate::engine::instance::FN_RIG_RENDER {
            unsafe {
                let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(FN_GET_CO);
                let co = f(builder);
                if co != 0 {
                    core::ptr::write_volatile((co + 0x25) as *mut u8, shown as u8);
                }
            }
        }
    }
}
