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
use alloc::vec::Vec;

const OVERVIEW_PTR: usize = 0x768e3c;
const VT_SELECT_COUNT: usize = 0x94;
const VT_SELECT_LIST: usize = 0x98;
/// `Entity::Get(handle)`, cdecl: the object, or NULL when it is gone.
const ENTITY_GET: usize = 0x4cfff0;
/// `Entity::GetUserTeam()`, cdecl: the local player's team number.
const USER_TEAM: usize = 0x4d0060;

pub fn overview() -> usize {
    unsafe { rd::<usize>(OVERVIEW_PTR) }
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
        let count: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(rd::<usize>(vt + VT_SELECT_COUNT));
        let list: unsafe extern "thiscall" fn(usize) -> *const i32 = core::mem::transmute(rd::<usize>(vt + VT_SELECT_LIST));
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
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(ENTITY_GET) };
    unsafe { f(handle) }
}

/// The local player's team.
pub fn user_team() -> i32 {
    let f: unsafe extern "C" fn() -> i32 = unsafe { core::mem::transmute(USER_TEAM) };
    unsafe { f() }
}

// ---- whether the selection can warp (the minimap's cursor) ---------------------------

/// `GameObject+0x14` (a byte, 4: a game object), `+0x40` its class, `+0xec` its team.
const OBJ_FLAGS: usize = 0x14;
const OBJ_CLASS: usize = 0x40;
const OBJ_TEAM: usize = 0xec;
/// `Craft+0x1b0`: its `cCraftControl`, whose `+0x08` is the impulse speed and `+0x0c`
/// the warp speed (`SetWarpSpeed`), floats.
const CRAFT_CONTROL: usize = 0x1b0;
const CTL_SPEED: usize = 0x08;
const CTL_WARP: usize = 0x0c;
/// `g_allowWarp` (the mission scripts' `AllowWarp`), `g_pTransport`,
/// `Transport::getGameSetup` and `GameSetup::isAllowWarpSpeed` (the setup screen).
const G_ALLOW_WARP: usize = 0x6f4de0;
const G_TRANSPORT: usize = 0x76b8d4;
const GET_SETUP: usize = 0x557940;
const ALLOW_WARP_SPEED: usize = 0x546280;

/// Warp is allowed in this game: by the mission and by the setup screen, as
/// `cCraftControl::TryWarp` asks (qol/README.md, QOL-8).
fn warp_allowed() -> bool {
    unsafe {
        if rd::<i32>(G_ALLOW_WARP) == 0 {
            return false;
        }
        let tr = rd::<usize>(G_TRANSPORT);
        if tr == 0 {
            return true;
        }
        let get: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(GET_SETUP);
        let setup = get(tr);
        if setup == 0 {
            return true;
        }
        let allow: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(ALLOW_WARP_SPEED);
        allow(setup) != 0
    }
}

/// The cursor a move ordered on the minimap shows: the minimap's order is `GO_WARP`,
/// which a ship that can warp carries out at warp and one that cannot at impulse. Some
/// `"c_warp"` when one of this player's selected ships can warp (its control's warp
/// speed above its speed, as `TryWarp` tests, and warp allowed), `"c_move"` when the
/// player's ships selected cannot, None when none of the player's ships is selected.
pub fn move_cursor() -> Option<&'static str> {
    let team = user_team();
    let mut any = false;
    let allowed = warp_allowed();
    for h in handles() {
        let o = object(h);
        if o == 0 {
            continue;
        }
        unsafe {
            if rd::<u8>(o + OBJ_FLAGS) & 4 == 0 || rd::<i32>(o + OBJ_TEAM) != team {
                continue;
            }
            if super::overlay::cast_craft_class(rd::<usize>(o + OBJ_CLASS)) == 0 {
                continue;
            }
            any = true;
            let ctl = rd::<usize>(o + CRAFT_CONTROL);
            if allowed && ctl != 0 && rd::<f32>(ctl + CTL_WARP) > rd::<f32>(ctl + CTL_SPEED) {
                return Some("c_warp");
            }
        }
    }
    if any { Some("c_move") } else { None }
}

pub fn functions_stock() -> bool {
    super::starts(ENTITY_GET, &[0x55, 0x8b, 0xec, 0x8b, 0x4d, 0x08]) && super::starts(USER_TEAM, &[0x8b, 0x0d, 0xd4, 0xb8, 0x76, 0x00])
}

/// `move_cursor`'s two calls are the stock accessors (each a field read and `ret`).
pub fn warp_functions_stock() -> bool {
    super::starts(GET_SETUP, &[0x8b, 0x41, 0x30, 0xc3]) && super::starts(ALLOW_WARP_SPEED, &[0x8b, 0x41, 0x10, 0x8a, 0x80, 0xd1])
}
