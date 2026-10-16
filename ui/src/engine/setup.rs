//! The game being loaded, for the loading screen (README.md, "The loading screen"):
//! its map and players as the shell set them up, read through the engine's own
//! `GameSetup` getters, and how far each network player's load has got, from the
//! transport. Every function is checked by its first bytes before the first call; with
//! one not stock, nothing here is called and every read answers None.
use super::{field, msvc, rd, starts};
use alloc::string::String;
use alloc::vec::Vec;

/// `g_transport` (`Transport*`): the game's connection, local or network.
const TRANSPORT: usize = 0x76b8d4;
/// `Transport::getGameSetup()`, thiscall: the `GameSetup` the shell filled in.
const FN_GET_GAME_SETUP: usize = 0x557940;
/// `GameSetup::getMapName() const`, thiscall: the map's file name (`a2_fed01`, `mp08gorn`).
const FN_GET_MAP_NAME: usize = 0x5461f0;
/// `GameSetup::getMapTitle() const`, thiscall: the map's title as the setup screens show it.
const FN_GET_MAP_TITLE: usize = 0x546240;
/// `GameSetup::getLocalSlot() const`, thiscall: this player's slot.
const FN_GET_LOCAL_SLOT: usize = 0x547470;
/// `GameSetup::getRace(int slot) const`, thiscall: an instant-action race index.
const FN_GET_RACE: usize = 0x546650;
/// `GameSetup::getTeam(int slot) const`, thiscall.
const FN_GET_TEAM: usize = 0x546610;
/// `GameSetup::isVacant(int slot) const`, thiscall.
const FN_IS_VACANT: usize = 0x546af0;
/// `GameSetup::isGamePlayer(int slot) const`, thiscall: a slot that plays (not an
/// observer, not empty).
const FN_IS_GAME_PLAYER: usize = 0x546b90;
/// `GameSetup::getPlayerName(int slot) const`, thiscall, returns a `std::string` through
/// a pointer it is given: the player's name, or the computer's ("Computer AI:Easy").
const FN_GET_PLAYER_NAME: usize = 0x5466e0;
/// `GameSetup::RaceRandom()`, static: the race index that stands for "random" until the
/// load deals one (`Program::GameOpenPreLoad`).
const FN_RACE_RANDOM: usize = 0x547cd0;
/// `Race::FindInstantActionSlot(int)`, cdecl: the `Race` for an instant-action race
/// index, or null. Its name (`federation`, `borg`, ...) is at `+0x14`.
const FN_FIND_IA_RACE: usize = 0x48b180;
const RACE_NAME: usize = 0x14;
/// Slots in a game setup.
pub const SLOTS: usize = 8;

/// `Transport`'s virtual functions, by vtable offset (all thiscall).
const VT_IS_INSTANT_ACTION: usize = 0x10;
const VT_IS_SINGLE_PLAYER: usize = 0x18;
const VT_SHOW_LOAD_PERCENTAGES: usize = 0x40;
const VT_GET_LOAD_PERCENTAGE: usize = 0x44;
const VT_IS_SEND_WORTHY: usize = 0x5c;

const SIGS: &[(usize, &[u8])] = &[
    (FN_GET_GAME_SETUP, &[0x8b, 0x41, 0x30, 0xc3]),
    (FN_GET_MAP_NAME, &[0x56, 0x8b, 0xf1, 0x8b, 0x46, 0x10]),
    (FN_GET_MAP_TITLE, &[0x8b, 0x41, 0x10, 0x83, 0xc0, 0x6c, 0xc3]),
    (FN_GET_LOCAL_SLOT, &[0x8b, 0x01, 0xc3]),
    (FN_GET_RACE, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08]),
    (FN_GET_TEAM, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08]),
    (FN_IS_VACANT, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08]),
    (FN_IS_GAME_PLAYER, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08]),
    (FN_GET_PLAYER_NAME, &[0x55, 0x8b, 0xec, 0x6a, 0xff]),
    (FN_RACE_RANDOM, &[0xa1, 0xb4, 0x7c, 0x73, 0x00, 0xc3]),
    (FN_FIND_IA_RACE, &[0x55, 0x8b, 0xec, 0x53, 0x56]),
];

static mut CHECKED: Option<bool> = None;

fn ok() -> bool {
    unsafe {
        if CHECKED.is_none() {
            let good = SIGS.iter().all(|&(a, s)| starts(a, s));
            if !good {
                crate::log!("setup: a GameSetup getter is not stock; the loading screen shows no mission details");
            }
            CHECKED = Some(good);
        }
        CHECKED == Some(true)
    }
}

fn transport() -> usize {
    unsafe { rd::<usize>(TRANSPORT) }
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
    unsafe { this0(FN_GET_GAME_SETUP, t) }
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
    let s = cstring(unsafe { this0(FN_GET_MAP_NAME, gs) })?;
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
    cstring(unsafe { this0(FN_GET_MAP_TITLE, gs) }).map(|s| String::from(s.trim()))
}

/// Instant Action (against the computer, from the main menu).
pub fn is_instant_action() -> bool {
    transport_flag(VT_IS_INSTANT_ACTION).unwrap_or(false)
}

/// A game on this machine alone: a campaign mission or Instant Action.
pub fn is_single_player() -> bool {
    transport_flag(VT_IS_SINGLE_PLAYER).unwrap_or(true)
}

/// An instant-action race index's race name (`federation`), None for "random" (not
/// dealt yet) or an index with no race.
fn race_name_of(index: usize) -> Option<&'static str> {
    unsafe {
        if index == core::mem::transmute::<usize, unsafe extern "C" fn() -> usize>(FN_RACE_RANDOM)() {
            return None;
        }
        let r = core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(FN_FIND_IA_RACE)(index);
        if r == 0 {
            return None;
        }
        Some(super::hud::cstr_static((r + RACE_NAME) as *const u8))
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
    let slot = unsafe { this0(FN_GET_LOCAL_SLOT, gs) };
    race_name_of(unsafe { this1(FN_GET_RACE, gs, slot) })
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
    let local = unsafe { this0(FN_GET_LOCAL_SLOT, gs) };
    for slot in 0..SLOTS {
        unsafe {
            if this1(FN_IS_VACANT, gs, slot) as u8 != 0 || this1(FN_IS_GAME_PLAYER, gs, slot) as u8 == 0 {
                continue;
            }
        }
        let name = msvc::with_string(|sp| unsafe {
            core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize, usize) -> usize>(FN_GET_PLAYER_NAME)(gs, sp, slot);
        })
        .unwrap_or_default();
        let race = race_name_of(unsafe { this1(FN_GET_RACE, gs, slot) });
        let team = unsafe { this1(FN_GET_TEAM, gs, slot) } as i32;
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
        if this0(field(vt, VT_SHOW_LOAD_PERCENTAGES), t) as u8 == 0 {
            return None;
        }
        let mut out = [None; SLOTS];
        let get: unsafe extern "thiscall" fn(usize, usize) -> f32 = core::mem::transmute(field(vt, VT_GET_LOAD_PERCENTAGE));
        for (slot, o) in out.iter_mut().enumerate() {
            if this1(field(vt, VT_IS_SEND_WORTHY), t, slot) as u8 != 0 {
                *o = Some(get(t, slot).clamp(0.0, 1.0));
            }
        }
        Some(out)
    }
}
