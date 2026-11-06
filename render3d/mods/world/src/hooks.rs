//! The world module's engine glue (README.md, "World", "Hooks"): every address, call
//! site, vtable slot and field offset the planets, moons and asteroids, the nebulae and
//! their lights read or patch, and the three detours the module installs through
//! `hooks`. Each detour starts with the module's gate and hands the call to what
//! was there before (`Prev`) when the gate is shut, so a module switched off live puts
//! the engine's own code back in charge. The findings behind each site are in the
//! README ("World") beside the name of the hook that uses them.
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Site};
use r3d_core::engine::{self, field, field_f32, field_m34, field_u8, light, obj, rd};
use r3d_core::math::{M34, V3};
use crate::{log, rt};
use alloc::vec::Vec;
use crate::syms::{field as off, hook, sym, value, TABLE};
use r3d_core::engine::instance::{frustum_test, instance_database, set_on_screen};

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { TABLE.engine().addr(s) }

// ---- the nebulae's lights ---------------------------------------------------------

/// The call to `Nebula::Simulate_Nebula_Lights` inside `Nebula::Simulate`.
pub(crate) fn fn_entity_transform() -> usize { at(sym::FN_ENTITY_TRANSFORM) } // Entity::GetTransform() const, thiscall
/// `NebulaInstance`'s vtable; slot 3 is `ST3D_Instance::FrustumTest(cam)`.
pub(crate) fn vt_nebula_instance() -> usize { at(sym::VT_NEBULA_INSTANCE) }
const SLOT_FRUSTUM: usize = 3;

// ---- the nebulae --------------------------------------------------------------------

pub fn vt_nebula() -> usize { at(sym::VT_NEBULA) }
pub fn vt_latinum() -> usize { at(sym::VT_LATINUM) }
pub(crate) fn nebula_list() -> usize { at(sym::NEBULA_LIST) } // Nebula::nebulaList, std::vector<Nebula *> *
pub(crate) fn ghost_list() -> usize { at(sym::GHOST_LIST) } // Ghost::ghostList, std::list<Spirit *>
pub(crate) fn vt_spirit() -> usize { at(sym::VT_SPIRIT) }
const SLOT_SPIRIT_RENDER: usize = 3;
pub(crate) fn fn_entity_get() -> usize { at(sym::FN_ENTITY_GET) } // Entity::Get(int handle), cdecl

// ---- the planets ----------------------------------------------------------------------

pub fn vt_planet() -> usize { at(sym::VT_PLANET) }
pub(crate) fn vt_planet_db() -> usize { at(sym::VT_PLANET_DB) }
/// `PlanetInstance`: the instance a planet's fog-of-war spirit carries a copy of.
pub(crate) fn vt_planet_instance() -> usize { at(sym::VT_PLANET_INSTANCE) }
pub(crate) fn fn_distance_cull() -> usize { at(sym::FN_DISTANCE_CULL) } // GameObjectInstance::DistanceCull(eye, pos), thiscall
/// cfgPOP_HEAVY: the population a colony's city map is full at (RTS_CFG.h).
pub(crate) fn pop_heavy_addr() -> usize { at(sym::POP_HEAVY) }
/// The four population levels' maxima (`PlanetClass+0x490` indexes it, from 1).
pub(crate) fn pop_levels() -> usize { at(sym::POP_LEVELS) }

// ---- the rocks ------------------------------------------------------------------------

pub fn vt_scrap() -> usize { at(sym::VT_SCRAP) }
pub fn vt_asteroid_field() -> usize { at(sym::VT_ASTEROID_FIELD) }
pub(crate) fn asteroid_names() -> usize { at(sym::ASTEROID_NAMES) }
pub(crate) fn cfg_cull_distance() -> usize { at(sym::CFG_CULL_DISTANCE) } // cfgOBJECT_CULLING_DISTANCE
pub(crate) fn fn_res_value() -> usize { at(sym::FN_RES_VALUE) } // ResourceInterface::GetTotalValue, thiscall -> float
pub(crate) fn fn_res_max() -> usize { at(sym::FN_RES_MAX) } // ResourceInterfaceClass::GetTotalMaxValue, thiscall -> float

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
    if !h.ready(&TABLE, hook::WORLD_READS) {
        h.stand_down("the engine's reads are not stock");
        return;
    }
    unsafe { GATE = Some(h.gate()) };
    let owner = h.owner();
    if lights && h.ready(&TABLE, hook::WORLD_LIGHTS) {
        let hook = Hook {
            owner,
            used_by: "nebula lights",
            kind: Kind::Call,
            site: Site::of(&TABLE, sym::SITE_NEBULA_LIGHTS),
            to: nebula_lights as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_LIGHTS = p.addr() };
        } else {
            log!("world: lights: Nebula::Simulate's light call is not stock's: the nebulae's lights stay as they are");
        }
    } else if lights {
        log!("world: lights: Nebula::Simulate's light call is not stock's: the nebulae's lights stay as they are");
    }
    if lights && h.ready(&TABLE, hook::WORLD_CULL) {
        let hook = Hook {
            owner,
            used_by: "nebula cull",
            kind: Kind::Slot { vt: vt_nebula_instance(), slot: SLOT_FRUSTUM },
            site: Site { addr: vt_nebula_instance(), ..Site::of(&TABLE, sym::SLOT_NEBULA_FRUSTUM) },
            to: nebula_frustum as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_FRUSTUM = p.addr() };
        } else {
            log!("world: lights: NebulaInstance's frustum test is not stock's: stock's nebula cull");
        }
    } else if lights {
        log!("world: lights: NebulaInstance's frustum test is not stock's: stock's nebula cull");
    }
    if spirit && h.ready(&TABLE, hook::WORLD_GHOSTS) {
        let hook = Hook {
            owner,
            used_by: "nebula ghosts",
            kind: Kind::Slot { vt: vt_spirit(), slot: SLOT_SPIRIT_RENDER },
            site: Site { addr: vt_spirit(), ..Site::of(&TABLE, sym::SLOT_SPIRIT_RENDER) },
            to: hook_spirit as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_SPIRIT = p.addr() };
            log!("world: nebulae: the ghosts' Render wrapped");
        } else {
            log!("world: nebulae: the ghosts' Render is not stock's: a fogged nebula keeps its billboards");
        }
    } else if spirit {
        log!("world: nebulae: the ghosts' Render is not stock's: a fogged nebula keeps its billboards");
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
        // The radius, the fourth float of the database node's sphere, or the instance's.
        let node = instance_database(inst);
        let r = ((if node != 0 { node + off::WD_NODE_SPHERE() } else { inst + off::WD_INST_SPHERE() }) + 3 * 4) as *mut f32;
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
            let get: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(fn_entity_get());
            let o = get(rd::<i32>(sp + off::WD_SPIRIT_HANDLE()));
            if o != 0 && (engine::vtable(o) == vt_nebula() || engine::vtable(o) == vt_latinum()) && super::nebulae::drawn_as_gas(&rt::lower(obj::odf_name(o))) {
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
    field(neb, off::WD_NEB_LIGHT())
}

/// The class's glow colour (`red_glow`..: `NebulaClass+0x22c`).
pub fn nebula_glow(cls: usize) -> [f32; 3] {
    unsafe { rd(cls + off::WD_NEB_GLOW()) }
}

/// The class's light falloff (`+0x244`, `+0x248`).
pub fn nebula_falloff(cls: usize) -> (f32, f32) {
    (field_f32(cls, off::WD_NEB_FALLOFF_START()), field_f32(cls, off::WD_NEB_FALLOFF_RANGE()))
}

/// Registers the nebula's light at its transform (`Entity::GetTransform`).
pub fn register_nebula_light(neb: usize, lt: usize, rgb: &[f32; 4]) {
    unsafe {
        let tf: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(fn_entity_transform());
        light::register(lt, rgb, &*tf(neb));
    }
}

/// The light's reach: full out to `start`, gone `range` further on.
pub fn set_light_reach(lt: usize, start: f32, range: f32) {
    light::set_reach(lt, start, range);
}

/// A planet instance's bounding sphere: centre (object space) `+0x34`, radius `+0x40`.
pub fn instance_sphere(inst: usize) -> [f32; 4] {
    unsafe { rd(inst + off::WD_INST_SPHERE()) }
}

/// The instance's placement (`+0x44`, a `Matrix34`).
pub fn instance_world(inst: usize) -> M34 {
    unsafe { rd(inst + off::WD_INST_MATRIX()) }
}

/// The instance's origin (`+0x68`).
pub fn instance_origin(inst: usize) -> V3 {
    unsafe { rd(inst + off::WD_INST_ORIGIN()) }
}

/// The instance's database (`+0x80`).
pub fn instance_db(inst: usize) -> usize {
    instance_database(inst)
}

// ---- nebulae.rs -----------------------------------------------------------------------

/// The class's radius (`+0x1e0`), when it is a plausible one.
pub fn nebula_class_radius(o: usize) -> f32 {
    field_f32(obj::class(o), off::WD_NEB_CLASS_RADIUS())
}

/// An object's handle (`+0x28`).
pub fn handle(o: usize) -> i32 {
    engine::entity_handle(o) as i32
}

/// A nebula's position (`+0xac`).
pub fn position(o: usize) -> [f32; 3] {
    unsafe { rd(o + off::WD_NEB_POSITION()) }
}

/// The handles the engine keeps ghosts of this frame (`Ghost::ghostList`).
pub fn ghost_handles(out: &mut Vec<i32>) {
    let handle = off::WD_SPIRIT_HANDLE();
    engine::for_each_in_list_at(ghost_list(), |sp| {
        if sp != 0 {
            out.push(unsafe { rd::<i32>(sp + handle) });
        }
    });
}

/// Every `Nebula` in the engine's list (`Nebula::nebulaList`).
pub fn each_nebula(mut f: impl FnMut(usize)) {
    engine::for_each_in_vector(nebula_list(), |n| f(n));
}

/// Marks a nebula on screen as `GameObject::Render` would (not hidden `+0x27`, seen by
/// the player, its instance in the frustum: slot 3 of the instance's vtable), for the
/// engine's nebula lights.
pub fn mark_nebula_on_screen(o: usize, cam: usize) {
    let inst = obj::instance(o);
    if obj::expired(o) || inst == 0 || cam == 0 || !obj::can_user_see(o) {
        return;
    }
    if frustum_test(inst, cam) {
        set_on_screen(o);
    }
}

// ---- planets.rs -----------------------------------------------------------------------

/// A planet class's ground and atmosphere texture names (`+0x4bc`, `+0x4ac`).
pub fn planet_textures(cls: usize) -> (&'static str, &'static str) {
    unsafe { (rt::cstr_at((cls + off::WD_PLANET_GROUND_TEX()) as *const u8, 16), rt::cstr_at((cls + off::WD_PLANET_ATMO_TEX()) as *const u8, 16)) }
}

/// A planet's colony: its race record (`+0x2c4`).
pub fn planet_race(o: usize) -> usize {
    field(o, off::WD_PLANET_RACE())
}

/// The colony's population (`+0x2c8`).
pub fn planet_population(o: usize) -> f32 {
    field_f32(o, off::WD_PLANET_POPULATION())
}

/// Whether the race's cities are drawn (`+0x452`, its city texture's name, is set).
pub fn race_has_people(race: usize) -> bool {
    field_u8(race, off::WD_RACE_NAME()) != 0
}

/// The race's city texture name (`+0x452`), which names its `PD_` development textures.
pub fn race_name(race: usize) -> &'static str {
    unsafe { rt::cstr_at((race + off::WD_RACE_NAME()) as *const u8, 32) }
}

/// `PlanetInstance::RenderInternal` draws only while the planet's flags have bit 2 and
/// its byte `+0x113` is clear.
pub fn planet_hidden(o: usize) -> bool {
    obj::flags(o) & 4 == 0 || field_u8(o, off::WD_PLANET_HIDDEN()) != 0
}

/// Whether a database is a planet's.
pub fn is_planet_db(db: usize) -> bool {
    db != 0 && engine::vtable(db) == vt_planet_db()
}

/// A planet's fog-of-war spirit (`Ghost::ghostList`): the copy of its `PlanetInstance`
/// (`+0x10`) and its class (`+4`). None if it is not a planet's.
pub fn planet_spirit(sp: usize) -> Option<(usize, usize, usize)> {
    let inst = field(sp, off::WD_SPIRIT_INSTANCE());
    if inst == 0 || engine::vtable(inst) != vt_planet_instance() {
        return None;
    }
    let db = instance_db(inst);
    let cls = field(sp, off::WD_SPIRIT_CLASS());
    if cls == 0 || !is_planet_db(db) {
        return None;
    }
    Some((inst, cls, db))
}

/// `GameObjectInstance::DistanceCull(eye, pos)`.
pub fn distance_culled(inst: usize, eye: &V3, pos: &V3) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(fn_distance_cull());
        f(inst, eye, pos) != 0
    }
}

/// The race's city picture for hemisphere `h` (`+0x514 + 4h`): (array, width, height,
/// data).
pub fn city_array(race: usize, h: usize) -> (usize, u32, u32, usize) {
    let arr = field(race, off::WD_RACE_CITIES() + h * 4);
    (arr, field(arr, off::WD_CITY_WIDTH()) as u32, field(arr, off::WD_CITY_HEIGHT()) as u32, field(arr, off::WD_CITY_DATA()))
}

/// One value of a city array's data.
pub fn city_value(data: usize, index: usize) -> f32 {
    unsafe { rd(data + index * 4) }
}

/// The race's city gradient `k` (`+0x470`).
pub fn race_city_k(race: usize) -> f32 {
    field_f32(race, off::WD_RACE_CITY_K())
}

/// cfgPOP_HEAVY, as RTS_CFG.h set it at load.
pub fn pop_heavy() -> i32 {
    unsafe { rd::<i32>(pop_heavy_addr()) }
}

/// The class's population level (`+0x490`) and that level's maximum population.
pub fn class_max_population(cls: usize) -> f32 {
    let lvl = unsafe { rd::<i32>(cls + off::WD_PLANET_POP_LEVEL()) };
    match lvl {
        1..=4 => (unsafe { rd::<i32>(pop_levels() + (lvl as usize - 1) * 4) }) as f32,
        _ => 0.0,
    }
}

/// The race's colour (`+0x464`).
pub fn race_colour(race: usize) -> [f32; 3] {
    unsafe { rd(race + off::WD_RACE_COLOUR()) }
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
    let (h0, h1, shell) = (field(db, off::WD_PDB_GROUND()), field(db, off::WD_PDB_GROUND2()), field(db, off::WD_PDB_SHELL()));
    let radius = off::WD_HULL_RADIUS();
    let rad = |h: usize| if h != 0 { field_f32(h, radius) } else { 0.0 };
    PlanetParts {
        ground_r: rad(h0),
        ground_r2: rad(h1),
        shell_r: rad(shell),
        yaw: -field_f32(inst, off::WD_PLANET_YAW()) * field_f32(db, off::WD_PDB_YAW_RATE()),
        turb_t: field_f32(inst, off::WD_PLANET_TURB_TIME()),
        turb: field_f32(cls, off::WD_PLANET_TURBULENCE()) > 0.0,
    }
}

// ---- rocks.rs -------------------------------------------------------------------------

/// The scale an Entity sets on its instance (a moon shrinks as it is mined), applied
/// while the instance's flag `+0x74` is on; 1 otherwise.
pub fn instance_scale(inst: usize) -> V3 {
    r3d_core::engine::instance::instance_scale(inst)
}

/// The instance's clock as `GameObject::RenderInternal` plays its sprite nodes: the
/// time since it began (`+0x18`) and its flags (`+0xc`).
pub fn instance_clock(inst: usize, time: f32) -> (f32, u32) {
    if inst != 0 {
        let (flags, _, start) = engine::view::instance_clock(inst);
        ((time - start).max(0.0), flags)
    } else {
        (time, 3)
    }
}

/// An asteroid field's rocks: the array (`+0x98`) and its count (`+0x9c`).
pub fn field_rocks(inst: usize) -> (usize, usize) {
    (field(inst, off::WD_FIELD_ROCKS()), field(inst, off::WD_FIELD_ROCK_COUNT()))
}

/// The field class's eight databases (`+0x1e0`) and their count (`+0x1fc`), at most 16.
pub fn field_databases(cls: usize) -> (usize, usize) {
    (field(cls, off::WD_FIELD_DBS()), field(cls, off::WD_FIELD_DB_COUNT()).min(16))
}

/// The culling distance (cfgOBJECT_CULLING_DISTANCE).
pub fn cull_distance() -> f32 {
    unsafe { rd::<f32>(cfg_cull_distance()) }
}

/// One rock of a field: its `ST3D_Instance` (0x98 bytes each).
pub fn rock(arr: usize, i: usize) -> usize {
    arr + i * value::WD_ROCK_BYTES() as usize
}

/// A rock's model name: the table's name for the class database it uses, or None for
/// a database that is not the class's.
pub fn rock_model(rock: usize, dbs: usize, ndb: usize) -> Option<&'static str> {
    let db = instance_database(rock);
    let k = (0..ndb).find(|&k| field(dbs, k * 4) == db)?;
    Some(unsafe { rt::cstr_at(field(asteroid_names(), k * 4) as *const u8, 32) })
}

/// A rock's placement (`+0x44`).
pub fn rock_world(rock: usize) -> M34 {
    field_m34(rock, off::WD_INST_MATRIX())
}

/// Whether the rock is hidden (`+0x84`).
pub fn rock_hidden(rock: usize) -> bool {
    field_u8(rock, off::WD_ROCK_HIDDEN()) != 0
}

/// How much of the moon's dilithium is left, 0..1 (`Scrap::Render`: the object's
/// resource `+0x17c` over the class's maximum); 1 for a Scrap without dilithium.
pub fn moon_dilithium(o: usize) -> f32 {
    let res = field(o, off::WD_SCRAP_RESOURCE());
    if res == 0 || field_u8(res, off::WD_RESOURCE_EMPTY()) != 0 {
        return 1.0;
    }
    let rc = field(res, off::WD_RESOURCE_CLASS());
    if rc == 0 {
        return 1.0;
    }
    let (val, max) = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(fn_res_value());
        let g: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(fn_res_max());
        (f(res), g(rc))
    };
    if max > 0.0 { val / max } else { 1.0 }
}
