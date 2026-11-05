//! The mission scripts' team texts (HUD.md, "The trade status and the team message"):
//! `Team::RenderUserTeamData` (0x497df0), which the mission frame calls right after
//! `DisplayInterface::RenderAll` (at 0x5988d0), prints two things a script asks for in
//! the default font on the HUD canvas (`canvas::CANVAS`), so stretched on a wide screen:
//!
//! - **the trade status**, while `Team::s_showTradesToTeam` (0x6fea10, set by the
//!   script's `ShowTradesTo`) is a team from 2 to 8: `RenderResourcesSentToTeam` on the
//!   team at `s_pTeamList[1]` prints "Sent to Team %d:" in that team's colour (the
//!   team's `+0x08`, three floats) and the dilithium, latinum and metal it has sent,
//!   each in its resource's colour (`ResourceComponent::s_ResourceColor`), from three
//!   float arrays on the sender indexed by team (`+0x2c`, `+0x78`, `+0xc4`; the
//!   `Register*Trade` calls add to them), rounded half up. At (1350, 100) and below.
//! - **the team message**, while `Team::s_teamMessage` (0x738df8, a string the
//!   script's `SetTeamText` copies in) is not empty: localised, in red, at (50, 740).
//!
//! With them taken the call does nothing and `hud::trade` draws both.
use super::{buttons, field as fld, hook as patch, rd};
use crate::syms_panels::{field, hook, sym, TABLE};
use alloc::string::String;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

fn ok() -> bool {
    eng().ready(hook::PANELS_TRADE).is_ok()
}

// The call is in the mission frame (`call Team::RenderUserTeamData`, after `RenderAll`);
// `Team::s_pTeamList` is a `Team *[]`.

static mut PATCHED: bool = false;
static mut TAKEN: bool = false;

/// Patches the call in the mission frame. False (nothing patched) when it is not stock.
pub fn install() -> bool {
    if !ok() {
        return false;
    }
    let site = addr(sym::PANELS_TRADE_SITE);
    if !patch::patch_call(site, addr(sym::PANELS_RENDER_USER_TEAM_DATA), render_user_team_data as *const () as usize) {
        crate::log!("trade: the call at {:#x} is not to Team::RenderUserTeamData: the team texts stay stock", site);
        return false;
    }
    unsafe { PATCHED = true };
    true
}

pub fn set_taken(on: bool) {
    unsafe { TAKEN = on && PATCHED };
}

unsafe extern "C" fn render_user_team_data() {
    if unsafe { TAKEN } {
        return;
    }
    let f: unsafe extern "C" fn() = unsafe { core::mem::transmute(addr(sym::PANELS_RENDER_USER_TEAM_DATA)) };
    unsafe { f() }
}

/// What the trade status shows.
pub struct Trades {
    /// The team the resources went to, as the script numbers it.
    pub team: i32,
    /// "Sent to Team 2:", localised.
    pub title: String,
    /// The team's colour (0xAARRGGBB).
    pub colour: u32,
    /// (`eResource`, the localised line "Dilithium: 0") for dilithium, latinum, metal.
    pub lines: [(u32, String); 3],
}

fn team(i: usize) -> usize {
    fld(addr(sym::PANELS_TEAM_LIST), 4 * i)
}

/// The engine's `%d` filled in: the localised format has exactly one.
fn fill(key: &core::ffi::CStr, n: i64) -> String {
    let f = buttons::localize(key);
    match f.find("%d") {
        Some(i) => crate::sfmt!("{}{}{}", &f[..i], n, &f[i + 2..]),
        None => f,
    }
}

/// The trade status, while the script shows one.
pub fn trades() -> Option<Trades> {
    if !ok() {
        return None;
    }
    let to = unsafe { rd::<i32>(addr(sym::PANELS_SHOW_TRADES_TO)) };
    if !(2..=8).contains(&to) {
        return None;
    }
    let (from, dest) = (team(1), team(to as usize));
    if from == 0 {
        return None;
    }
    let sent = |base: usize| -> i64 { libm::floor(unsafe { rd::<f32>(from + base + 4 * to as usize) } as f64 + 0.5) as i64 };
    let colour = if dest == 0 { 0xffffffff } else { buttons::colour(unsafe { rd(dest + field::PANELS_TEAM_COLOUR) }) };
    Some(Trades {
        team: to,
        title: fill(c"TRADE_SENT_TO_TEAM", to as i64),
        colour,
        lines: [
            (2, fill(c"TRADE_DILITHIUM_AMOUNT", sent(field::PANELS_SENT_DILITHIUM))),
            (3, fill(c"TRADE_LATINUM_AMOUNT", sent(field::PANELS_SENT_LATINUM))),
            (4, fill(c"TRADE_METAL_AMOUNT", sent(field::PANELS_SENT_METAL))),
        ],
    })
}

/// The team message, localised, while the script shows one.
pub fn message() -> Option<String> {
    if !ok() {
        return None;
    }
    let message = addr(sym::PANELS_TEAM_MESSAGE);
    if unsafe { rd::<u8>(message) } == 0 {
        return None;
    }
    // The buffer as the engine hands it to `Localize`: a key, or the text itself.
    let c = unsafe { core::ffi::CStr::from_ptr(message as *const core::ffi::c_char) };
    let s = buttons::localize(c);
    if s.is_empty() { None } else { Some(s) }
}
