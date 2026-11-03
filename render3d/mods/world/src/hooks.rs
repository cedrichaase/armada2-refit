//! The world module's engine glue (README.md, "World", "Hooks"): every address, call
//! site, vtable slot and field offset the planets, moons and asteroids, the nebulae and
//! their lights read or patch, and the three detours the module installs through
//! `hooks`. Each detour starts with the module's gate and hands the call to what
//! was there before (`Prev`) when the gate is shut, so a module switched off live puts
//! the engine's own code back in charge. The findings behind each site are in the
//! README ("World") beside the name of the hook that uses them.
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Site};
use r3d_core::engine::{self, field, field_f32, field_m34, field_u8, light, obj, rd};
use r3d_core::math::{v3, M34, V3};
use crate::{log, rt};
use alloc::vec::Vec;

// ---- the nebulae's lights ---------------------------------------------------------

/// The call to `Nebula::Simulate_Nebula_Lights` inside `Nebula::Simulate`.
const SITE_NEBULA_LIGHTS: usize = 0x4a4a7a;
const FN_NEBULA_LIGHTS: usize = 0x4a5160; // Nebula::Simulate_Nebula_Lights(float), thiscall
const FN_ENTITY_TRANSFORM: usize = 0x4cfd50; // Entity::GetTransform() const, thiscall
/// `NebulaInstance`'s vtable; slot 3 is `ST3D_Instance::FrustumTest(cam)`.
const VT_NEBULA_INSTANCE: usize = 0x6b1448;
const SLOT_FRUSTUM: usize = 3;
const FN_FRUSTUM_TEST: usize = 0x62e990;

// ---- the nebulae --------------------------------------------------------------------

pub const VT_NEBULA: usize = 0x6b148c;
pub const VT_LATINUM: usize = 0x6b132c;
const NEBULA_LIST: usize = 0x73b1bc; // Nebula::nebulaList, std::vector<Nebula *> *
const GHOST_LIST: usize = 0x736c90; // Ghost::ghostList, std::list<Spirit *>
const VT_SPIRIT: usize = 0x6b1ca4;
const SLOT_SPIRIT_RENDER: usize = 3;
const FN_SPIRIT_RENDER: usize = 0x4d5f70;
const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), cdecl

// ---- the planets ----------------------------------------------------------------------

pub const VT_PLANET: usize = 0x6b2b3c;
const VT_PLANET_DB: usize = 0x6bab74;
/// `PlanetInstance`: the instance a planet's fog-of-war spirit carries a copy of.
const VT_PLANET_INSTANCE: usize = 0x6b2ce4;
const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
/// cfgPOP_HEAVY: the population a colony's city map is full at (RTS_CFG.h).
const POP_HEAVY: usize = 0x70213c;
/// The four population levels' maxima (`PlanetClass+0x490` indexes it, from 1).
const POP_LEVELS: usize = 0x702130;

// ---- the rocks ------------------------------------------------------------------------

pub const VT_SCRAP: usize = 0x6b15b4;
pub const VT_ASTEROID_FIELD: usize = 0x6b0cb4;
const ASTEROID_NAMES: usize = 0x6b07e8;
const CFG_CULL_DISTANCE: usize = 0x6fcac0; // cfgOBJECT_CULLING_DISTANCE
const FN_RES_VALUE: usize = 0x4d6820; // ResourceInterface::GetTotalValue, thiscall -> float
const FN_RES_MAX: usize = 0x4cf1f0; // ResourceInterfaceClass::GetTotalMaxValue, thiscall -> float

// ---- the detours' state -------------------------------------------------------------

static mut GATE: Option<Gate> = None;
static mut PREV_LIGHTS: usize = 0;
static mut PREV_FRUSTUM: usize = 0;
static mut PREV_SPIRIT: usize = 0;

/// The module's on flag: shut, every detour hands the call back.
#[inline(always)]
fn open() -> bool {
    unsafe { GATE.is_some_and(|g| g.open()) }
}

/// Installs the module's hooks (once, at its first enable). `lights`: the nebulae's
/// lights and cull are ours (no `Lighting.asi`); `spirit`: the ghosts' `Render` is
/// wrapped (no `Nebulae.asi`). A site that is not stock's is logged and left as it is.
pub fn install(h: &mut Hooks, lights: bool, spirit: bool) {
    unsafe { GATE = Some(h.gate()) };
    let owner = h.owner();
    if lights {
        let hook = Hook {
            owner,
            used_by: "nebula lights",
            kind: Kind::Call,
            site: Site { symbol: "Nebula::Simulate", addr: SITE_NEBULA_LIGHTS, expect: FN_NEBULA_LIGHTS },
            to: nebula_lights as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_LIGHTS = p.addr() };
        } else {
            log!("world: lights: Nebula::Simulate's light call is not stock's: the nebulae's lights stay as they are");
        }
        let hook = Hook {
            owner,
            used_by: "nebula cull",
            kind: Kind::Slot { vt: VT_NEBULA_INSTANCE, slot: SLOT_FRUSTUM },
            site: Site { symbol: "NebulaInstance::FrustumTest", addr: VT_NEBULA_INSTANCE, expect: FN_FRUSTUM_TEST },
            to: nebula_frustum as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_FRUSTUM = p.addr() };
        } else {
            log!("world: lights: NebulaInstance's frustum test is not stock's: stock's nebula cull");
        }
    }
    if spirit {
        let hook = Hook {
            owner,
            used_by: "nebula ghosts",
            kind: Kind::Slot { vt: VT_SPIRIT, slot: SLOT_SPIRIT_RENDER },
            site: Site { symbol: "GameObjectSpirit::Render", addr: VT_SPIRIT, expect: FN_SPIRIT_RENDER },
            to: hook_spirit as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_SPIRIT = p.addr() };
            log!("world: nebulae: the ghosts' Render wrapped");
        } else {
            log!("world: nebulae: the ghosts' Render is not stock's: a fogged nebula keeps its billboards");
        }
    }
}

/// `Nebula::Simulate_Nebula_Lights`, re-pointed: ours while the gate is open and the
/// lights are on; else the call that was there.
unsafe extern "thiscall" fn nebula_lights(neb: usize, dt: f32) {
    if !open() || !super::lights::nebula_lights_on() {
        unsafe {
            if PREV_LIGHTS != 0 {
                let f: unsafe extern "thiscall" fn(usize, f32) = core::mem::transmute(PREV_LIGHTS);
                f(neb, dt);
            }
        }
        return;
    }
    super::lights::nebula_light(neb);
}

/// `NebulaInstance` slot 3: the frustum test on the sphere times `NebulaCull`; the
/// scale is 1 (the previous target, untouched) while the gate is shut.
unsafe extern "thiscall" fn nebula_frustum(inst: usize, cam: usize) -> u32 {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u32 = core::mem::transmute(PREV_FRUSTUM);
        let scale = if open() { super::lights::cull_scale() } else { 1.0 };
        if !(scale > 1.0) {
            return f(inst, cam);
        }
        let node = field(inst, 0x80);
        let r = ((if node != 0 { node + 0x1c } else { inst + 0x34 }) + 0xc) as *mut f32;
        let keep = core::ptr::read_unaligned(r);
        core::ptr::write_unaligned(r, keep * scale);
        // Its answer is a bool in al: the rest of eax is not to be relied on.
        let inn = f(inst, cam) & 0xff;
        core::ptr::write_unaligned(r, keep);
        inn
    }
}

/// `GameObjectSpirit::Render`, wrapped: a ghost of a nebula whose class is drawn as gas
/// draws nothing (its gas is drawn with the class's).
unsafe extern "thiscall" fn hook_spirit(sp: usize, cam: usize) {
    unsafe {
        if open() && super::nebulae::spirit_active() && sp != 0 {
            let get: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(FN_ENTITY_GET);
            let o = get(rd::<i32>(sp + 8));
            if o != 0 && (engine::vtable(o) == VT_NEBULA || engine::vtable(o) == VT_LATINUM) && super::nebulae::drawn_as_gas(&rt::lower(obj::odf_name(o))) {
                return;
            }
        }
        if PREV_SPIRIT != 0 {
            let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(PREV_SPIRIT);
            f(sp, cam);
        }
    }
}

// ---- lights.rs ------------------------------------------------------------------------

/// A nebula's point light (made by `Nebula::InitializeGeometry`, `+0x1ac`); 0 if none.
pub fn nebula_light_object(neb: usize) -> usize {
    field(neb, 0x1ac)
}

/// The class's glow colour (`red_glow`..: `NebulaClass+0x22c`).
pub fn nebula_glow(cls: usize) -> [f32; 3] {
    unsafe { rd(cls + 0x22c) }
}

/// The class's light falloff (`+0x244`, `+0x248`).
pub fn nebula_falloff(cls: usize) -> (f32, f32) {
    (field_f32(cls, 0x244), field_f32(cls, 0x248))
}

/// Registers the nebula's light at its transform (`Entity::GetTransform`).
pub fn register_nebula_light(neb: usize, lt: usize, rgb: &[f32; 4]) {
    unsafe {
        let tf: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(FN_ENTITY_TRANSFORM);
        light::register(lt, rgb, &*tf(neb));
    }
}

/// The light's reach: full out to `start`, gone `range` further on.
pub fn set_light_reach(lt: usize, start: f32, range: f32) {
    light::set_reach(lt, start, range);
}

/// A planet instance's bounding sphere: centre (object space) `+0x34`, radius `+0x40`.
pub fn instance_sphere(inst: usize) -> [f32; 4] {
    unsafe { rd(inst + 0x34) }
}

/// The instance's placement (`+0x44`, a `Matrix34`).
pub fn instance_world(inst: usize) -> M34 {
    unsafe { rd(inst + 0x44) }
}

/// The instance's origin (`+0x68`).
pub fn instance_origin(inst: usize) -> V3 {
    unsafe { rd(inst + 0x68) }
}

/// The instance's database (`+0x80`).
pub fn instance_db(inst: usize) -> usize {
    field(inst, 0x80)
}

// ---- nebulae.rs -----------------------------------------------------------------------

/// The class's radius (`+0x1e0`), when it is a plausible one.
pub fn nebula_class_radius(o: usize) -> f32 {
    field_f32(obj::class(o), 0x1e0)
}

/// An object's handle (`+0x28`).
pub fn handle(o: usize) -> i32 {
    unsafe { rd::<i32>(o + 0x28) }
}

/// A nebula's position (`+0xac`).
pub fn position(o: usize) -> [f32; 3] {
    unsafe { rd(o + 0xac) }
}

/// The handles the engine keeps ghosts of this frame (`Ghost::ghostList`).
pub fn ghost_handles(out: &mut Vec<i32>) {
    let head = field(GHOST_LIST, 4);
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != 0 && n != head && guard < 100_000 {
        let sp = field(n, 8);
        if sp != 0 {
            out.push(unsafe { rd::<i32>(sp + 8) });
        }
        n = field(n, 0);
        guard += 1;
    }
}

/// Every `Nebula` in the engine's list (`Nebula::nebulaList`).
pub fn each_nebula(mut f: impl FnMut(usize)) {
    let vecp = unsafe { rd::<usize>(NEBULA_LIST) };
    if vecp != 0 {
        let (mut it, end) = (field(vecp, 4), field(vecp, 8));
        while it != 0 && it < end {
            f(field(it, 0));
            it += 4;
        }
    }
}

/// Marks a nebula on screen as `GameObject::Render` would (not hidden `+0x27`, seen by
/// the player, its instance in the frustum: slot 3 of the instance's vtable), for the
/// engine's nebula lights.
pub fn mark_nebula_on_screen(o: usize, cam: usize) {
    let inst = obj::instance(o);
    if field_u8(o, 0x27) != 0 || inst == 0 || cam == 0 || !obj::can_user_see(o) {
        return;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(engine::vtable(inst), SLOT_FRUSTUM * 4));
        if f(inst, cam) != 0 {
            core::ptr::write((o + 0x25) as *mut u8, 1);
        }
    }
}

// ---- planets.rs -----------------------------------------------------------------------

/// A planet class's ground and atmosphere texture names (`+0x4bc`, `+0x4ac`).
pub fn planet_textures(cls: usize) -> (&'static str, &'static str) {
    unsafe { (rt::cstr_at((cls + 0x4bc) as *const u8, 16), rt::cstr_at((cls + 0x4ac) as *const u8, 16)) }
}

/// A planet's colony: its race record (`+0x2c4`).
pub fn planet_race(o: usize) -> usize {
    field(o, 0x2c4)
}

/// The colony's population (`+0x2c8`).
pub fn planet_population(o: usize) -> f32 {
    field_f32(o, 0x2c8)
}

/// Whether the race's cities are drawn (`+0x452`).
pub fn race_has_people(race: usize) -> bool {
    field_u8(race, 0x452) != 0
}

/// `PlanetInstance::RenderInternal` draws only while the planet's flags have bit 2 and
/// its byte `+0x113` is clear.
pub fn planet_hidden(o: usize) -> bool {
    obj::flags(o) & 4 == 0 || field_u8(o, 0x113) != 0
}

/// Whether a database is a planet's.
pub fn is_planet_db(db: usize) -> bool {
    db != 0 && engine::vtable(db) == VT_PLANET_DB
}

/// A planet's fog-of-war spirit (`Ghost::ghostList`): the copy of its `PlanetInstance`
/// (`+0x10`) and its class (`+4`). None if it is not a planet's.
pub fn planet_spirit(sp: usize) -> Option<(usize, usize, usize)> {
    let inst = field(sp, 0x10);
    if inst == 0 || engine::vtable(inst) != VT_PLANET_INSTANCE {
        return None;
    }
    let db = instance_db(inst);
    let cls = field(sp, 4);
    if cls == 0 || !is_planet_db(db) {
        return None;
    }
    Some((inst, cls, db))
}

/// `GameObjectInstance::DistanceCull(eye, pos)`.
pub fn distance_culled(inst: usize, eye: &V3, pos: &V3) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
        f(inst, eye, pos) != 0
    }
}

/// The race's city picture for hemisphere `h` (`+0x514 + 4h`): (array, width, height,
/// data).
pub fn city_array(race: usize, h: usize) -> (usize, u32, u32, usize) {
    let arr = field(race, 0x514 + h * 4);
    (arr, field(arr, 0) as u32, field(arr, 4) as u32, field(arr, 8))
}

/// One value of a city array's data.
pub fn city_value(data: usize, index: usize) -> f32 {
    unsafe { rd(data + index * 4) }
}

/// The race's city gradient `k` (`+0x470`).
pub fn race_city_k(race: usize) -> f32 {
    field_f32(race, 0x470)
}

/// cfgPOP_HEAVY, as RTS_CFG.h set it at load.
pub fn pop_heavy() -> i32 {
    unsafe { rd::<i32>(POP_HEAVY) }
}

/// The class's population level (`+0x490`) and that level's maximum population.
pub fn class_max_population(cls: usize) -> f32 {
    let lvl = unsafe { rd::<i32>(cls + 0x490) };
    match lvl {
        1..=4 => (unsafe { rd::<i32>(POP_LEVELS + (lvl as usize - 1) * 4) }) as f32,
        _ => 0.0,
    }
}

/// The race's colour (`+0x464`).
pub fn race_colour(race: usize) -> [f32; 3] {
    unsafe { rd(race + 0x464) }
}

/// What `SetAtmosphereParameters` and the database give a planet's draw: the hulls of
/// its ground (`+0xa8`, `+0xac`) and shell (`+0xb0`) and their radii (`+0x150`), the
/// atmosphere's yaw (minus the instance's `+0xb4` times the database's `+0xa4`), its
/// turbulence clock (instance `+0xb8`) and whether the class has turbulence (`+0x4cc`).
pub struct PlanetParts {
    pub ground_r: f32,
    pub ground_r2: f32,
    pub shell_r: f32,
    pub yaw: f32,
    pub turb_t: f32,
    pub turb: bool,
}

pub fn planet_parts(cls: usize, inst: usize, db: usize) -> PlanetParts {
    let (h0, h1, shell) = (field(db, 0xa8), field(db, 0xac), field(db, 0xb0));
    let rad = |h: usize| if h != 0 { field_f32(h, 0x150) } else { 0.0 };
    PlanetParts {
        ground_r: rad(h0),
        ground_r2: rad(h1),
        shell_r: rad(shell),
        yaw: -field_f32(inst, 0xb4) * field_f32(db, 0xa4),
        turb_t: field_f32(inst, 0xb8),
        turb: field_f32(cls, 0x4cc) > 0.0,
    }
}

// ---- rocks.rs -------------------------------------------------------------------------

/// The scale an Entity sets on its instance (a moon shrinks as it is mined), applied
/// while the instance's flag `+0x74` is on; 1 otherwise.
pub fn instance_scale(inst: usize) -> V3 {
    if inst != 0 && field_u8(inst, 0x74) != 0 { unsafe { rd::<V3>(inst + 0x28) } } else { v3(1.0, 1.0, 1.0) }
}

/// The instance's clock as `GameObject::RenderInternal` plays its sprite nodes: the
/// time since it began (`+0x18`) and its flags (`+0xc`).
pub fn instance_clock(inst: usize, time: f32) -> (f32, u32) {
    if inst != 0 { ((time - unsafe { rd::<f32>(inst + 0x18) }).max(0.0), field(inst, 0xc) as u32) } else { (time, 3) }
}

/// An asteroid field's rocks: the array (`+0x98`) and its count (`+0x9c`).
pub fn field_rocks(inst: usize) -> (usize, usize) {
    (field(inst, 0x98), field(inst, 0x9c))
}

/// The field class's eight databases (`+0x1e0`) and their count (`+0x1fc`), at most 16.
pub fn field_databases(cls: usize) -> (usize, usize) {
    (field(cls, 0x1e0), field(cls, 0x1fc).min(16))
}

/// The culling distance (cfgOBJECT_CULLING_DISTANCE).
pub fn cull_distance() -> f32 {
    unsafe { rd::<f32>(CFG_CULL_DISTANCE) }
}

/// One rock of a field: its `ST3D_Instance` (0x98 bytes each).
pub fn rock(arr: usize, i: usize) -> usize {
    arr + i * 0x98
}

/// A rock's model name: the table's name for the class database it uses, or None for
/// a database that is not the class's.
pub fn rock_model(rock: usize, dbs: usize, ndb: usize) -> Option<&'static str> {
    let db = field(rock, 0x80);
    let k = (0..ndb).find(|&k| field(dbs, k * 4) == db)?;
    Some(unsafe { rt::cstr_at(field(ASTEROID_NAMES, k * 4) as *const u8, 32) })
}

/// A rock's placement (`+0x44`).
pub fn rock_world(rock: usize) -> M34 {
    field_m34(rock, 0x44)
}

/// Whether the rock is hidden (`+0x84`).
pub fn rock_hidden(rock: usize) -> bool {
    field_u8(rock, 0x84) != 0
}

/// How much of the moon's dilithium is left, 0..1 (`Scrap::Render`: the object's
/// resource `+0x17c` over the class's maximum); 1 for a Scrap without dilithium.
pub fn moon_dilithium(o: usize) -> f32 {
    let res = field(o, 0x17c);
    if res == 0 || field_u8(res, 0x18) != 0 {
        return 1.0;
    }
    let rc = field(res, 4);
    if rc == 0 {
        return 1.0;
    }
    let (val, max) = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_RES_VALUE);
        let g: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_RES_MAX);
        (f(res), g(rc))
    };
    if max > 0.0 { val / max } else { 1.0 }
}
