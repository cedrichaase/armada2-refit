//! The player's own settings of ours that a screen of ours changes (Game Settings'
//! "Command Bar"), never one of the game's (those stay in `ARMADA.PRF`, written by the
//! stock screens). They go to the player's overlay, `RefitPlayer.ini` in the game
//! directory (`rt::overlay`, tools/settings/README.md), as `[UI.HUD] BarHidden`: a
//! file installers never write, so installing the UI again keeps what the player
//! chose. Until ui 0.29.0 the key was in `UIPlayer.ini`; `migrate` carries it over once.
use crate::rt::{self, Ini, overlay, settings::*};
use crate::settings_table::HUD_BARHIDDEN;

pub fn bar_hidden() -> bool {
    migrate();
    on(&ini(HUD_BARHIDDEN), HUD_BARHIDDEN)
}

pub fn set_bar_hidden(v: bool) {
    crate::settings::set_row(HUD_BARHIDDEN, if v { "1" } else { "0" });
    crate::log!("prefs: the button bar {}", if v { "hidden" } else { "shown" });
}

static mut MIGRATED: bool = false;

/// `UIPlayer.ini` (ui 0.13 to 0.28) to the overlay, once: when the overlay has no
/// `[UI.HUD] BarHidden`, a non-zero `[HUD] BarHidden` is set there; then the old file is
/// renamed `UIPlayer.ini.migrated`. Left in place for the next start when the overlay
/// could not be written.
pub fn migrate() {
    unsafe {
        if MIGRATED {
            return;
        }
        MIGRATED = true;
    }
    let old = rt::game_path(overlay::OLD_FILE);
    if !rt::file_exists(&old) {
        return;
    }
    let was = Ini::open_plain(&old).int(HUD_BARHIDDEN.section(), HUD_BARHIDDEN.key(), 0);
    let osec = overlay::section(HUD_BARHIDDEN.file(), HUD_BARHIDDEN.section());
    let has = Ini::open_plain(&overlay::path()).string(&osec, HUD_BARHIDDEN.key()).is_some_and(|v| !v.is_empty());
    let what = if has {
        "the overlay already has BarHidden"
    } else if was == 0 {
        "BarHidden=0, the default: nothing to carry over"
    } else if overlay::set_row(HUD_BARHIDDEN, "1") {
        "BarHidden=1 carried over"
    } else {
        crate::log!("settings: {} not migrated: {} could not be written", overlay::OLD_FILE, overlay::FILE);
        return;
    };
    let to = rt::sfmt!("{}.migrated", old);
    let moved = rt::move_file(&old, &to);
    crate::log!(
        "settings: {} migrated to {} [{}] ({}); {}",
        overlay::OLD_FILE,
        overlay::FILE,
        osec,
        what,
        if moved { "renamed UIPlayer.ini.migrated" } else { "could not be renamed" }
    );
}
