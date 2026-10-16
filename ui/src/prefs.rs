//! The player's own settings of ours, kept between sessions in `UIPlayer.ini` in the
//! game directory: settings of the UI's that a screen of ours changes (Game Settings'
//! "Command Bar"), never one of the game's (those stay in `ARMADA.PRF`, written by the
//! stock screens). A file of its own, not `UI.ini`, so that installing the UI again
//! (which writes `UI.ini`) keeps what the player chose; and it is not shipped, so
//! `a2mod` and the installers never touch it.
use crate::rt::{self, Ini};

const FILE: &str = "UIPlayer.ini";

/// `[HUD] BarHidden`: the button bar is hidden (the top-right palette button's toggle,
/// which moved to Game Settings).
static mut BAR_HIDDEN: Option<bool> = None;

fn ini() -> Ini {
    Ini::open(&rt::game_path(FILE))
}

pub fn bar_hidden() -> bool {
    unsafe {
        if BAR_HIDDEN.is_none() {
            let v = ini().int("HUD", "BarHidden", 0) != 0;
            BAR_HIDDEN = Some(v);
        }
        BAR_HIDDEN.unwrap_or(false)
    }
}

pub fn set_bar_hidden(v: bool) {
    unsafe { BAR_HIDDEN = Some(v) };
    write("HUD", "BarHidden", if v { "1" } else { "0" });
    crate::log!("prefs: the button bar {}", if v { "hidden" } else { "shown" });
}

fn write(sec: &str, key: &str, val: &str) {
    let (s, k, v, f) = (rt::cstr(sec), rt::cstr(key), rt::cstr(val), rt::cstr(&rt::game_path(FILE)));
    let ok = unsafe { rt::win::WritePrivateProfileStringA(s.as_ptr(), k.as_ptr(), v.as_ptr(), f.as_ptr()) } != 0;
    if !ok {
        crate::log!("prefs: {} could not be written", FILE);
    }
}
