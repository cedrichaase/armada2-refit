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

pub fn functions_stock() -> bool {
    super::starts(ENTITY_GET, &[0x55, 0x8b, 0xec, 0x8b, 0x4d, 0x08]) && super::starts(USER_TEAM, &[0x8b, 0x0d, 0xd4, 0xb8, 0x76, 0x00])
}
