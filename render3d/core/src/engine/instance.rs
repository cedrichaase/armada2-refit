//! What every module reads of a game object and its instance, besides `obj`: the
//! engine side of `scene::instance`. Moved out of the hulls module's `game.rs` when the
//! helpers other modules used were lifted into the core (README, "Module interface",
//! "Services"). Reads only, except for the on-screen byte `Entity::Render` writes and
//! what the engine's own frustum test writes, which the HUD and the sim go on reading
//! as before.
use super::{field, field_f32, field_u8, rd, storm3d};
use crate::math::{v3, V3};
use crate::engine::at;
use crate::syms::{field as off, sym, value};

/// A plain `ST3D_Database`'s vtable.
pub fn vt_database() -> usize { at(sym::VT_DATABASE) }
/// `Shipyard::Render` (a Starbase too), `MiningStation::Render` (a TradingStation too)
/// and `ConstructionRig::Render`: the classes whose `Render` shows or hides nodes.
pub fn fn_shipyard_render() -> usize { at(sym::FN_SHIPYARD_RENDER) }
pub fn fn_mining_render() -> usize { at(sym::FN_MINING_RENDER) }
pub fn fn_rig_render() -> usize { at(sym::FN_RIG_RENDER) }
/// The Evolver's cocoon (`CocoonInstance`, vtable 0x6b2630, of `8472_cocoon.sod`, whose
/// database the engine keeps at 0x73fccc).
pub fn vt_cocoon() -> usize { at(sym::VT_COCOON) }
pub fn cocoon_db() -> usize { at(sym::COCOON_DB) }

/// The slot of a class's `Render(ST3D_Camera*)` (11), for a hook of it.
pub fn render_slot() -> usize {
    off::VT_OBJ_RENDER() / 4
}

/// The class's `Render` (slot 11).
pub fn render_fn(o: usize) -> usize {
    field(super::vtable(o), off::VT_OBJ_RENDER())
}

/// A craft instance (`GameObjectInstance::IsCraftInstance`, instance slot 10).
pub fn is_craft_instance(inst: usize) -> bool {
    if inst == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(super::vtable(inst), off::VT_INST_IS_CRAFT()));
        f(inst) != 0
    }
}

/// The instance's frustum test (slot 3), as `Entity::Render` makes it. For a craft it
/// also applies the scale effects (strategic view, colonising) into the instance's
/// scale, which is why it is called rather than redone.
pub fn frustum_test(inst: usize, cam: usize) -> bool {
    if inst == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(super::vtable(inst), off::VT_INST_FRUSTUM()));
        f(inst, cam) != 0
    }
}

/// The instance's scale: `+0x28` while `+0x74` says one is set (`ApplyScaleEffects`).
pub fn instance_scale(inst: usize) -> V3 {
    if inst != 0 && field_u8(inst, off::INST_SCALE_SET()) != 0 {
        unsafe { rd::<V3>(inst + off::INST_SCALE()) }
    } else {
        v3(1.0, 1.0, 1.0)
    }
}

/// The on-screen byte `Entity::Render` sets when the frustum test passes.
pub fn set_on_screen(o: usize) {
    write_on_screen(o, 1);
}

/// Writes the on-screen byte (`+0x25`), as `GameObject::RenderInternal` writes the
/// answer of its frustum test or of a hologram's draw.
pub fn write_on_screen(o: usize, v: u8) {
    unsafe { core::ptr::write_volatile((o + off::OBJ_ON_SCREEN()) as *mut u8, v) };
}

/// The team colour `GameObject::RenderInternal` hands the engine (`Storm3D+0x34`):
/// the team's (`Team::GetTeam(+0xec) + 8`), or in a game of type 2 the player's own
/// (`[+0xfc] + 0x474`) when it has one.
pub fn team_colour(o: usize) -> V3 {
    unsafe {
        let setup = {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(at(sym::FN_GET_GAME_SETUP));
            f(rd::<usize>(at(sym::G_TRANSPORT)))
        };
        let gt = if setup != 0 {
            let f: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(at(sym::FN_GET_GAME_TYPE));
            f(setup)
        } else {
            0
        };
        let p = field(o, off::OBJ_PLAYER());
        if gt == 2 && p != 0 {
            return rd::<V3>(p + off::PLAYER_COLOUR());
        }
        // Team::GetTeam reads Team::s_pTeamList unchecked.
        let i = field(o, off::OBJ_TEAM()) as i32;
        let team = if (0..value::TEAM_LIST_LEN()).contains(&i) { rd::<usize>(at(sym::TEAM_LIST) + 4 * i as usize) } else { 0 };
        if team == 0 { v3(1.0, 1.0, 1.0) } else { rd::<V3>(team + off::TEAM_COLOUR()) }
    }
}

/// A `Team`'s colour (`+8`), black for no team.
pub fn team_rgb(team: usize) -> V3 {
    if team == 0 { v3(0.0, 0.0, 0.0) } else { unsafe { rd::<V3>(team + off::TEAM_COLOUR()) } }
}

/// Whether the engine shows team colours at all (`Storm3D+0x40`, which
/// `ST3D_Workspace::SetLightingMaterial` asks before it gives a `team*` material the
/// team's colour).
pub fn team_colours_on() -> bool {
    field_u8(storm3d(), off::S3D_TEAM_COLOURS()) != 0
}

/// The name of an engine database element (`+0x8`).
pub fn element_name(e: usize) -> Option<&'static str> {
    let p = field(e, off::ELEM_NAME());
    if p == 0 {
        return None;
    }
    let s = unsafe { crate::rt::cstr_at(p as *const u8, 64) };
    if s.is_empty() { None } else { Some(s) }
}

/// Whether the object's instance draws a plain `ST3D_Database` (a SOD as it is, not a
/// planet's or another special database).
pub fn draws_plain_sod(o: usize) -> bool {
    let db = field(super::obj::instance(o), off::INST_DATABASE());
    db != 0 && super::vtable(db) == vt_database()
}

/// The engine's graphics configuration (`ST3D_GraphicsEngine_Configuration`, Storm3D
/// `+0x104`).
pub fn graphics_config() -> usize {
    field(storm3d(), off::S3D_CONFIG())
}

/// An instance's scale (`+0x28`) as it is, with no test of the flag the scale effects
/// set (the background objects').
pub fn raw_scale(inst: usize) -> V3 {
    if inst == 0 { v3(1.0, 1.0, 1.0) } else { unsafe { rd::<V3>(inst + off::INST_SCALE()) } }
}

/// The engine's detail level (`ST3D_GraphicsEngine_Configuration+0x40`).
pub fn detail_level() -> u32 {
    field(field(storm3d(), off::S3D_CONFIG()), off::CFG_DETAIL()) as u32
}

/// The database the instance draws (`GameObjectInstance+0x80`).
pub fn instance_database(inst: usize) -> usize {
    field(inst, off::INST_DATABASE())
}

/// The instance's class (`GameObjectInstance+0x98`), for crafts the `CraftClass`.
pub fn instance_class(inst: usize) -> usize {
    field(inst, off::INST_CLASS())
}

/// Every damaged-system node of a craft's class with the instance byte that hides it
/// (`CraftClass::InitializeDamageNodes`; `CraftInstance::RenderInternal`): shields
/// `+0x40c`/`+0x9c`, engines `+0x418`/`+0x9d`, target `+0x410`/`+0x9e`, sensors
/// `+0x414`/`+0x9f`, life `+0x41c`/`+0xa0`. A byte that is not 0 hides the node's
/// subtree.
pub fn damage_nodes() -> [(usize, usize); 5] {
    [
        (off::CLASS_NODE_SHIELDS(), off::INST_DOWN_SHIELDS()),
        (off::CLASS_NODE_ENGINES(), off::INST_DOWN_ENGINES()),
        (off::CLASS_NODE_TARGET(), off::INST_DOWN_TARGET()),
        (off::CLASS_NODE_SENSORS(), off::INST_DOWN_SENSORS()),
        (off::CLASS_NODE_LIFE(), off::INST_DOWN_LIFE()),
    ]
}

/// The class's element at `at` (a `CraftClass` node field of `damage_nodes`).
pub fn class_node(class: usize, at: usize) -> usize {
    field(class, at)
}

/// The class's `crew` node (`+0x408`), whose children the crew fires show.
pub fn crew_node(class: usize) -> usize {
    field(class, off::CLASS_NODE_CREW())
}

/// The class's `borg` node (`+0x420`), shown only while the craft is assimilated.
pub fn borg_node(class: usize) -> usize {
    field(class, off::CLASS_NODE_BORG())
}

/// The instance's byte at `at` (a damage byte of `damage_nodes`).
pub fn instance_byte(inst: usize, at: usize) -> u8 {
    field_u8(inst, at)
}

/// A craft instance's six system bytes, in `damage_nodes`' order and then the
/// assimilated byte (`CraftInstance::Update` writes them).
pub fn system_bytes() -> [usize; 6] {
    let d = damage_nodes();
    [d[0].1, d[1].1, d[2].1, d[3].1, d[4].1, off::INST_ASSIMILATED()]
}

/// Whether the craft is assimilated (`+0xa1`).
pub fn assimilated(inst: usize) -> bool {
    field_u8(inst, off::INST_ASSIMILATED()) != 0
}

/// The crew-fire level (`CraftInstance+0xa4`).
pub fn crew_fire(inst: usize) -> f32 {
    field_f32(inst, off::INST_CREW_FIRE())
}

/// `+0x90`: the instance hides its `lights` node.
pub fn lights_hidden(inst: usize) -> bool {
    field_u8(inst, off::INST_LIGHTS_HIDDEN()) != 0
}

/// A node of the engine's own database, for the per-class nodes the stations show or
/// hide (`Shipyard::Render`: `class+0x4a4`, shown while a ship is being built, `+0x2b8`
/// not `NO_ENTITY`; `MiningStation::Render`: `class+0x4b0`, shown while a freighter is
/// docked).
pub fn station_node(o: usize) -> Option<(usize, bool)> {
    let class = field(o, off::OBJ_CLASS());
    match render_fn(o) {
        f if f != 0 && f == fn_shipyard_render() => {
            let n = field(class, off::SHIPYARD_BUILD_NODE());
            let none = unsafe { rd::<usize>(at(sym::NO_ENTITY)) };
            Some((n, field(o, off::SHIPYARD_BUILDING()) == none))
        }
        f if f != 0 && f == fn_mining_render() => {
            let n = field(class, off::MINING_DOCK_NODE());
            let docked = unsafe {
                let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(at(sym::FN_IS_SHIP_DOCKED));
                f(field(o, off::MINING_DOCKED())) != 0
            };
            Some((n, !docked))
        }
        _ => None,
    }
    .filter(|(n, _)| *n != 0)
}

/// The engine's `DrawHeightPost(top, camera)`: the post a build ghost stands on, when
/// the overlays' posts are not drawn.
pub fn draw_height_post(top: V3, cam: usize) {
    unsafe {
        let f: unsafe extern "C" fn(*const V3, usize) = core::mem::transmute(at(sym::FN_DRAW_HEIGHT_POST));
        f(&top, cam);
    }
}
