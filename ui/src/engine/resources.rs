//! The player's resources (HUD.md: for the resource panel, `ResourcePanel`).
//!
//! `ResourcePanel` draws one `ResourceComponent` per resource, each asking the user's
//! team (`Team::GetTeam(Entity::GetUserTeam())`) for its amount with
//! `Team::GetResource(eResource)`, which answers a float (0 for a resource the team
//! has no store of). We ask the same function. The `eResource` values are the game's
//! own (0..5); which is which is read from the panel's components when the resource
//! panel is taken over (HUD.md, "Not done").
const TEAM_GET: usize = 0x496340;
const TEAM_GET_RESOURCE: usize = 0x498b00;

/// The user's team object.
pub fn user_team_object() -> usize {
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(TEAM_GET) };
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
    let f: unsafe extern "thiscall" fn(usize, u32) -> f32 = unsafe { core::mem::transmute(TEAM_GET_RESOURCE) };
    unsafe { f(t, r) }
}

pub fn functions_stock() -> bool {
    super::starts(TEAM_GET, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08]) && super::starts(TEAM_GET_RESOURCE, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08])
}
