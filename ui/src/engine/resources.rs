//! The player's resources (HUD.md: for the resource panel, `ResourcePanel`).
//!
//! `ResourcePanel` draws one `ResourceComponent` per resource, each asking the user's
//! team (`Team::GetTeam(Entity::GetUserTeam())`) for its amount with
//! `Team::GetResource(eResource)`, which answers a float (0 for a resource the team
//! has no store of). We ask the same function. The `eResource` values are the game's
//! own (0..5); which is which is read from the panel's components when the resource
//! panel is taken over (HUD.md, "Not done").
use crate::syms_panels::{hook, sym, TABLE};

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

/// The user's team object.
pub fn user_team_object() -> usize {
    if !ok(hook::PANELS_RESOURCES) {
        return 0;
    }
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(addr(sym::PANELS_TEAM_GET)) };
    unsafe { f(super::selection::user_team()) }
}

/// The amount of resource `r` (`eResource`, 0..5) the user's team holds.
pub fn amount(r: u32) -> f32 {
    if r > 5 {
        return 0.0;
    }
    let t = user_team_object();
    if t == 0 {
        return 0.0;
    }
    let f: unsafe extern "thiscall" fn(usize, u32) -> f32 = unsafe { core::mem::transmute(addr(sym::PANELS_TEAM_GET_RESOURCE)) };
    unsafe { f(t, r) }
}

pub fn functions_stock() -> bool {
    ok(hook::PANELS_RESOURCES)
}
