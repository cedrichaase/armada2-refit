//! The game being loaded, for the loading screen (README.md, "The loading screen"):
//! its map and players as the shell set them up, read through the engine's own
//! `GameSetup` getters, and how far each network player's load has got, from the
//! transport. Every function is checked by its first bytes before the first call; with
//! one not stock, nothing here is called and every read answers None.
use super::{field, msvc, rd};
use alloc::string::String;
use alloc::vec::Vec;

use crate::syms_shell::{field, hook, sym, value, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

// The engine functions (`Transport::getGameSetup` and the `GameSetup` getters, all
// thiscall; `GameSetup::getPlayerName` returns a `std::string` through a pointer it is
// given: the player's name, or the computer's "Computer AI:Easy"; `GameSetup::RaceRandom`
// is static: the race index that stands for "random" until the load deals one,
// `Program::GameOpenPreLoad`; `Race::FindInstantActionSlot(int)` is cdecl: the `Race`
// for an instant-action race index, or null, its name `federation`, `borg`, ... in the
// object) and `g_transport` (`Transport*`: the game's connection, local or network) are
// in `syms_shell`, by name: `SETUP_*`.
/// Slots in a game setup.
pub const SLOTS: usize = value::SETUP_SLOTS as usize;

/// Whether the engine lets the getters run: every function is checked by its first
/// bytes before the first call; with one not stock, nothing here is called and every
/// read answers None.
fn ok() -> bool {
    eng().ready(hook::SHELL_READS_SETUP).is_ok()
}

fn transport() -> usize {
    let a = eng().addr(sym::SETUP_TRANSPORT);
    if a == 0 { 0 } else { unsafe { rd::<usize>(a) } }
}

unsafe fn this0(f: usize, this: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> usize>(f)(this) }
}
unsafe fn this1(f: usize, this: usize, a: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize) -> usize>(f)(this, a) }
}

/// A `Transport` virtual that answers a bool.
fn transport_flag(slot: usize) -> Option<bool> {
    let t = transport();
    if t == 0 || !ok() {
        return None;
    }
    Some(unsafe { this0(field(field(t, 0), slot), t) } as u8 != 0)
}

fn game_setup() -> usize {
    let t = transport();
    if t == 0 || !ok() {
        return 0;
    }
    unsafe { this0(eng().addr(sym::SETUP_GET_GAME_SETUP), t) }
}

/// A Latin-1 C string the engine owns, copied.
fn cstring(p: usize) -> Option<String> {
    if p == 0 {
        return None;
    }
    let mut s = String::new();
    for i in 0..260 {
        let b = unsafe { rd::<u8>(p + i) };
        if b == 0 {
            break;
        }
        s.push(b as char);
    }
    if s.is_empty() { None } else { Some(s) }
}

/// The map's file name, without a directory or extension (`a2_fed01`).
pub fn map_name() -> Option<String> {
    let gs = game_setup();
    if gs == 0 {
        return None;
    }
    let s = cstring(unsafe { this0(eng().addr(sym::SETUP_GET_MAP_NAME), gs) })?;
    let s = s.rsplit(['\\', '/']).next().unwrap_or(&s);
    let s = match s.rfind('.') {
        Some(i) if crate::rt::lower(&s[i..]) == ".bzn" => &s[..i],
        _ => s,
    };
    if s.is_empty() { None } else { Some(String::from(s)) }
}

/// The map's title, as the setup screens show it.
pub fn map_title() -> Option<String> {
    let gs = game_setup();
    if gs == 0 {
        return None;
    }
    cstring(unsafe { this0(eng().addr(sym::SETUP_GET_MAP_TITLE), gs) }).map(|s| String::from(s.trim()))
}

/// Instant Action (against the computer, from the main menu).
pub fn is_instant_action() -> bool {
    transport_flag(field::SETUP_VT_IS_INSTANT_ACTION).unwrap_or(false)
}

/// A campaign mission (or a replay). Not Instant Action: `TransportLocalInstantAction`
/// answers false, as a network transport does; ask `is_instant_action` first.
pub fn is_single_player() -> bool {
    transport_flag(field::SETUP_VT_IS_SINGLE_PLAYER).unwrap_or(true)
}

/// Whether the game being played (or set up) is a network game: false for a campaign
/// mission and for Instant Action (whose transport also answers "not single player", so
/// it is asked first), true for a network transport. None when the transport or one of
/// the getters cannot be read (no `g_transport`, or a function not stock): the caller
/// then treats it as the stricter answer. Used by the Refit screen to lock the settings
/// every player must share (`ui/README.md`, "The Refit screen").
pub fn network_game() -> Option<bool> {
    if transport() == 0 || !ok() {
        return None;
    }
    if transport_flag(field::SETUP_VT_IS_INSTANT_ACTION)? {
        return Some(false);
    }
    Some(!transport_flag(field::SETUP_VT_IS_SINGLE_PLAYER)?)
}

/// An instant-action race index's race name (`federation`), None for "random" (not
/// dealt yet) or an index with no race.
fn race_name_of(index: usize) -> Option<&'static str> {
    unsafe {
        if index == core::mem::transmute::<usize, unsafe extern "C" fn() -> usize>(eng().addr(sym::SETUP_RACE_RANDOM))() {
            return None;
        }
        let r = core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(eng().addr(sym::SETUP_FIND_IA_RACE))(index);
        if r == 0 {
            return None;
        }
        Some(super::hud::cstr_static((r + field::SETUP_RACE_NAME) as *const u8))
    }
}

/// The race the local player's interface will load in, as `DisplayInterface::
/// PostLoadAll` decides it (HUD.md, "The player's race"): in a game on this machine the
/// race a campaign mission's details set (`Race::s_missionRace`) when there is one, else
/// the local slot's race in the setup. None while that is still "random".
pub fn local_race_name() -> Option<&'static str> {
    if !ok() {
        return None;
    }
    let mission = super::race::mission_race_name();
    if !mission.is_empty() && is_single_player() {
        return Some(mission);
    }
    let gs = game_setup();
    if gs == 0 {
        return None;
    }
    let slot = unsafe { this0(eng().addr(sym::SETUP_GET_LOCAL_SLOT), gs) };
    race_name_of(unsafe { this1(eng().addr(sym::SETUP_GET_RACE), gs, slot) })
}

/// A player in the game being loaded.
#[derive(Clone, Debug)]
pub struct Player {
    pub slot: usize,
    pub name: String,
    /// The race's name (`klingon`), None while it is "random".
    pub race: Option<&'static str>,
    pub team: i32,
    pub local: bool,
}

/// The players in the setup, in slot order.
pub fn players() -> Vec<Player> {
    let mut out = Vec::new();
    let gs = game_setup();
    if gs == 0 {
        return out;
    }
    let local = unsafe { this0(eng().addr(sym::SETUP_GET_LOCAL_SLOT), gs) };
    for slot in 0..SLOTS {
        unsafe {
            if this1(eng().addr(sym::SETUP_IS_VACANT), gs, slot) as u8 != 0 || this1(eng().addr(sym::SETUP_IS_GAME_PLAYER), gs, slot) as u8 == 0 {
                continue;
            }
        }
        let name = msvc::with_string(|sp| unsafe {
            core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize, usize) -> usize>(eng().addr(sym::SETUP_GET_PLAYER_NAME))(gs, sp, slot);
        })
        .unwrap_or_default();
        let race = race_name_of(unsafe { this1(eng().addr(sym::SETUP_GET_RACE), gs, slot) });
        let team = unsafe { this1(eng().addr(sym::SETUP_GET_TEAM), gs, slot) } as i32;
        out.push(Player { slot, name, race, team, local: slot == local });
    }
    out
}

/// In a network game whose players' progress is shown (`Transport::ShowLoadPercentages`,
/// what the stock `LoadPercentageBox` asks): how far each slot's load has got, 0..1,
/// for the slots the transport sends to (`IsSendWorthy`). None in a game on this
/// machine, and once every player has finished.
pub fn load_fractions() -> Option<[Option<f32>; SLOTS]> {
    let t = transport();
    if t == 0 || !ok() || is_single_player() {
        return None;
    }
    let vt = field(t, 0);
    unsafe {
        if this0(field(vt, field::SETUP_VT_SHOW_LOAD_PERCENTAGES), t) as u8 == 0 {
            return None;
        }
        let mut out = [None; SLOTS];
        let get: unsafe extern "thiscall" fn(usize, usize) -> f32 = core::mem::transmute(field(vt, field::SETUP_VT_GET_LOAD_PERCENTAGE));
        for (slot, o) in out.iter_mut().enumerate() {
            if this1(field(vt, field::SETUP_VT_IS_SEND_WORTHY), t, slot) as u8 != 0 {
                *o = Some(get(t, slot).clamp(0.0, 1.0));
            }
        }
        Some(out)
    }
}
