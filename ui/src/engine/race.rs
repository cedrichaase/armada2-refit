//! The player's race, for the HUD's theme (STYLE.md, "Faction identity").
//!
//! `Race::s_missionRace` is the race whose interface the HUD is loaded with:
//! `DisplayInterface::PostLoadAll` sets it at the start of every mission, from the
//! user's team's race in the game setup (Federation when that is the random race),
//! before it reads that race's GUI config and sprites. So it is the race the stock HUD
//! is drawn in. A race's name (`Race+0x14`, from its `name` key: `federation`, `borg`,
//! ...) also names its emblem sprite (`<name>_icon`).
use super::rd;
use crate::syms_panels::{field, hook, sym, TABLE};
use crate::widgets::Race;


/// `Race::s_missionRace`, or 0 while its hook is not ready.
fn mission_race_ptr() -> usize {
    let e = TABLE.engine();
    if e.ready(hook::PANELS_READS_RACE).is_err() {
        return 0;
    }
    unsafe { rd::<usize>(e.addr(sym::PANELS_MISSION_RACE)) }
}

/// The mission's race's name, empty outside a mission.
pub fn mission_race_name() -> &'static str {
    let r = mission_race_ptr();
    if r == 0 {
        return "";
    }
    unsafe { super::hud::cstr_static((r + field::PANELS_RACE_NAME) as *const u8) }
}

static mut LAST: usize = 0;

/// The mission's race, as a theme knows it; logged once per change.
pub fn mission_race() -> Option<Race> {
    let r = mission_race_ptr();
    if r == 0 {
        return None;
    }
    let name = mission_race_name();
    let race = Race::from_name(name);
    unsafe {
        if LAST != r {
            LAST = r;
            crate::log!("race: the HUD's race is \"{}\" ({})", name, race.map_or("no theme of its own", |r| r.name()));
        }
    }
    race
}
