//! The player's selection and team (HUD.md: for the info panel, the selection panel
//! and anything that shows what is selected).
//!
//! The selection is the overview's (`g_pOverView`, a `cOverViewImp`): its
//! `GetSelectList` (vtable `+0x98`) returns the selected objects' handles and
//! `+0x94` their count, which is how the bar's `Update` reads them. With `QOL.asi`'s
//! `MaxSelection` above 16 the list is QOL's own array; through these two calls it is
//! read the same way (qol/README.md, QOL-3), never at `cOverViewImp+0xbc`. A handle is
//! turned into the object with `Entity::Get`.
use super::rd;
use crate::syms_panels::{field, hook, sym, value, TABLE};
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

fn ok(h: a2engine::HookId) -> bool {
    eng().ready(h).is_ok()
}

pub fn overview() -> usize {
    if !ok(hook::PANELS_READS_SELECTION) {
        return 0;
    }
    unsafe { rd::<usize>(addr(sym::PANELS_OVERVIEW_PTR)) }
}

/// The handles of the selected objects, in selection order.
pub fn handles() -> Vec<i32> {
    let o = overview();
    let mut v = Vec::new();
    if o == 0 {
        return v;
    }
    unsafe {
        let vt = rd::<usize>(o);
        let count: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(rd::<usize>(vt + field::PANELS_VT_SELECT_COUNT()));
        let list: unsafe extern "thiscall" fn(usize) -> *const i32 = core::mem::transmute(rd::<usize>(vt + field::PANELS_VT_SELECT_LIST()));
        let n = count(o).clamp(0, 4096) as usize;
        let p = list(o);
        if !p.is_null() {
            v.extend_from_slice(core::slice::from_raw_parts(p, n));
        }
    }
    v
}

/// The object a handle names (0 when it is gone).
pub fn object(handle: i32) -> usize {
    if !ok(hook::PANELS_READS_SELECTION) {
        return 0;
    }
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(addr(sym::PANELS_ENTITY_GET)) };
    unsafe { f(handle) }
}

/// The local player's team.
pub fn user_team() -> i32 {
    if !ok(hook::PANELS_READS_SELECTION) {
        return 0;
    }
    let f: unsafe extern "C" fn() -> i32 = unsafe { core::mem::transmute(addr(sym::PANELS_USER_TEAM)) };
    unsafe { f() }
}

// ---- whether the selection can warp (the minimap's cursor) ---------------------------

/// `GameObject+0x14` (a byte, 4: a game object), `+0x40` its class, `+0xec` its team.
/// `Craft+0x1b0`: its `cCraftControl`, whose `+0x08` is the impulse speed and `+0x0c`
/// the warp speed (`SetWarpSpeed`), floats. `g_allowWarp` (the mission scripts'
/// `AllowWarp`), `g_transport`, `Transport::getGameSetup` and
/// `GameSetup::isAllowWarpSpeed` (the setup screen).

/// Warp is allowed in this game: by the mission and by the setup screen, as
/// `cCraftControl::TryWarp` asks (qol/README.md, QOL-8).
fn warp_allowed() -> bool {
    unsafe {
        if rd::<i32>(addr(sym::PANELS_ALLOW_WARP)) == 0 {
            return false;
        }
        let tr = rd::<usize>(addr(sym::PANELS_TRANSPORT));
        if tr == 0 {
            return true;
        }
        let get: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(addr(sym::PANELS_GET_SETUP));
        let setup = get(tr);
        if setup == 0 {
            return true;
        }
        let allow: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(addr(sym::PANELS_ALLOW_WARP_SPEED));
        allow(setup) != 0
    }
}

/// The cursor a move ordered on the minimap shows: the minimap's order is `GO_WARP`,
/// which a ship that can warp carries out at warp and one that cannot at impulse. Some
/// `"c_warp"` when one of this player's selected ships can warp (its control's warp
/// speed above its speed, as `TryWarp` tests, and warp allowed), `"c_move"` when the
/// player's ships selected cannot, None when none of the player's ships is selected.
pub fn move_cursor() -> Option<&'static str> {
    if !ok(hook::PANELS_READS_SELECTION) || !ok(hook::PANELS_READS_WARP) {
        return None;
    }
    let team = user_team();
    let mut any = false;
    let allowed = warp_allowed();
    for h in handles() {
        let o = object(h);
        if o == 0 {
            continue;
        }
        unsafe {
            if rd::<u8>(o + field::PANELS_OBJ_FLAGS()) as i32 & value::PANELS_OBJ_IS_GAME_OBJECT() == 0 || rd::<i32>(o + field::PANELS_OBJ_TEAM()) != team {
                continue;
            }
            if super::overlay::cast_craft_class(rd::<usize>(o + field::PANELS_OBJ_CLASS())) == 0 {
                continue;
            }
            any = true;
            let ctl = rd::<usize>(o + field::PANELS_CRAFT_CONTROL());
            if allowed && ctl != 0 && rd::<f32>(ctl + field::PANELS_CTL_WARP()) > rd::<f32>(ctl + field::PANELS_CTL_SPEED()) {
                return Some("c_warp");
            }
        }
    }
    if any { Some("c_move") } else { None }
}

pub fn functions_stock() -> bool {
    ok(hook::PANELS_READS_SELECTION)
}

/// `move_cursor`'s two calls are the stock accessors (each a field read and `ret`).
pub fn warp_functions_stock() -> bool {
    ok(hook::PANELS_READS_SELECTION) && ok(hook::PANELS_READS_WARP)
}
