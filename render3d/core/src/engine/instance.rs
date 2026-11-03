//! What every module reads of a game object and its instance, besides `obj`: the
//! engine side of `scene::instance`. Moved out of the hulls module's `game.rs` when the
//! helpers other modules used were lifted into the core (README, "Module interface",
//! "Services"). Reads only, except for the on-screen byte `Entity::Render` writes and
//! what the engine's own frustum test writes, which the HUD and the sim go on reading
//! as before.
use super::{field, field_f32, field_u8, rd, storm3d};
use crate::math::{v3, V3};
use crate::engine::at;
use crate::syms::sym;

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

/// The class's `Render` (slot 11).
pub fn render_fn(o: usize) -> usize {
    field(super::vtable(o), 11 * 4)
}

/// A craft instance (`GameObjectInstance::IsCraftInstance`, instance slot 10).
pub fn is_craft_instance(inst: usize) -> bool {
    if inst == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(super::vtable(inst), 10 * 4));
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
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(super::vtable(inst), 3 * 4));
        f(inst, cam) != 0
    }
}

/// The instance's scale: `+0x28` while `+0x74` says one is set (`ApplyScaleEffects`).
pub fn instance_scale(inst: usize) -> V3 {
    if inst != 0 && field_u8(inst, 0x74) != 0 {
        unsafe { rd::<V3>(inst + 0x28) }
    } else {
        v3(1.0, 1.0, 1.0)
    }
}

/// The on-screen byte `Entity::Render` sets when the frustum test passes.
pub fn set_on_screen(o: usize) {
    unsafe { core::ptr::write_volatile((o + 0x25) as *mut u8, 1) };
}

/// The team colour `GameObject::RenderInternal` hands the engine (`Storm3D+0x34`):
/// the team's (`Team::GetTeam(+0xec) + 8`), or in a game of type 2 the player's own
/// (`[+0xfc] + 0x474`) when it has one.
pub fn team_colour(o: usize) -> V3 {
    unsafe {
        let setup = {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(0x557940usize);
            f(rd::<usize>(0x76b8d4))
        };
        let gt = if setup != 0 {
            let f: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(0x5464c0usize);
            f(setup)
        } else {
            0
        };
        let p = field(o, 0xfc);
        if gt == 2 && p != 0 {
            return rd::<V3>(p + 0x474);
        }
        // Team::GetTeam reads Team::s_pTeamList (0x738db0) unchecked.
        let i = field(o, 0xec) as i32;
        let team = if (0..32).contains(&i) { rd::<usize>(0x738db0 + 4 * i as usize) } else { 0 };
        if team == 0 { v3(1.0, 1.0, 1.0) } else { rd::<V3>(team + 8) }
    }
}

/// Whether the engine shows team colours at all (`Storm3D+0x40`, which
/// `ST3D_Workspace::SetLightingMaterial` asks before it gives a `team*` material the
/// team's colour).
pub fn team_colours_on() -> bool {
    field_u8(storm3d(), 0x40) != 0
}

/// The name of an engine database element (`+0x8`).
pub fn element_name(e: usize) -> Option<&'static str> {
    let p = field(e, 8);
    if p == 0 {
        return None;
    }
    let s = unsafe { crate::rt::cstr_at(p as *const u8, 64) };
    if s.is_empty() { None } else { Some(s) }
}

/// Whether the object's instance draws a plain `ST3D_Database` (a SOD as it is, not a
/// planet's or another special database).
pub fn draws_plain_sod(o: usize) -> bool {
    let db = field(super::obj::instance(o), 0x80);
    db != 0 && super::vtable(db) == vt_database()
}

/// The engine's detail level (`ST3D_GraphicsEngine_Configuration+0x40`).
pub fn detail_level() -> u32 {
    field(field(storm3d(), 0x104), 0x40) as u32
}

/// The instance's class (`GameObjectInstance+0x98`), for crafts the `CraftClass`.
pub fn instance_class(inst: usize) -> usize {
    field(inst, 0x98)
}

/// Every damaged-system node of a craft's class with the instance byte that hides it
/// (`CraftClass::InitializeDamageNodes`; `CraftInstance::RenderInternal`): shields
/// `+0x40c`/`+0x9c`, engines `+0x418`/`+0x9d`, target `+0x410`/`+0x9e`, sensors
/// `+0x414`/`+0x9f`, life `+0x41c`/`+0xa0`. A byte that is not 0 hides the node's
/// subtree.
pub const DAMAGE_NODES: [(usize, usize); 5] = [(0x40c, 0x9c), (0x418, 0x9d), (0x410, 0x9e), (0x414, 0x9f), (0x41c, 0xa0)];

/// The class's element at `off` (a `CraftClass` node field: the damage nodes, the
/// `crew` node at `+0x408`, the `borg` node at `+0x420`).
pub fn class_node(class: usize, off: usize) -> usize {
    field(class, off)
}

/// The instance's byte at `off` (a damage byte of `DAMAGE_NODES`, `+0xa1` assimilated).
pub fn instance_byte(inst: usize, off: usize) -> u8 {
    field_u8(inst, off)
}

/// The crew-fire level (`CraftInstance+0xa4`).
pub fn crew_fire(inst: usize) -> f32 {
    field_f32(inst, 0xa4)
}

/// `+0x90`: the instance hides its `lights` node.
pub fn lights_hidden(inst: usize) -> bool {
    field_u8(inst, 0x90) != 0
}

/// A node of the engine's own database, for the per-class nodes the stations show or
/// hide (`Shipyard::Render`: `class+0x4a4`, shown while a ship is being built, `+0x2b8`
/// not `NO_ENTITY`; `MiningStation::Render`: `class+0x4b0`, shown while a freighter is
/// docked).
pub fn station_node(o: usize) -> Option<(usize, bool)> {
    let class = field(o, 0x40);
    match render_fn(o) {
        fn_shipyard_render() => {
            let n = field(class, 0x4a4);
            let none = unsafe { rd::<usize>(0x6b41d8) };
            Some((n, field(o, 0x2b8) == none))
        }
        fn_mining_render() => {
            let n = field(class, 0x4b0);
            let docked = unsafe {
                let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(0x4d7020usize);
                f(field(o, 0x2b0)) != 0
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
        let f: unsafe extern "C" fn(*const V3, usize) = core::mem::transmute(0x593840usize);
        f(&top, cam);
    }
}
