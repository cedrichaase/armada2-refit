//! The player's own settings of ours that a screen of ours changes (Game Settings'
//! "Command Bar"), never one of the game's (those stay in `ARMADA.PRF`, written by the
//! stock screens). They go to the player's overlay, `RefitPlayer.ini` in the game
//! directory (`rt::overlay`, tools/settings/README.md), as `[UI.HUD] BarHidden`: a
//! file installers never write, so installing the UI again keeps what the player
//! chose. Until ui 0.29.0 the key was in `UIPlayer.ini`; `migrate` carries it over once.
use crate::rt::{self, Ini, overlay, settings::*};

/// `[HUD] BarHidden` of UI.ini, through the overlay: the button bar is hidden (the
/// top-right palette button's toggle, which moved to Game Settings). Not in the
/// shipped UI.ini and not a row of the Refit screen. (P-UI's `ui/settings.toml` takes
/// this row over as `settings_table::HUD_BARHIDDEN`.)
static BAR_HIDDEN: Setting = Setting {
    size: ROW_SIZE,
    kind: KIND_BOOL,
    apply: APPLY_MISSION,
    scope: SCOPE_LOCAL,
    tab: TAB_INTERFACE,
    flags: FLAG_NOT_SHIPPED | FLAG_NO_ROW,
    file: c"UI.ini".as_ptr(),
    section: c"HUD".as_ptr(),
    key: c"BarHidden".as_ptr(),
    def: c"0".as_ptr(),
    values: core::ptr::null(),
    value_labels: core::ptr::null(),
    label: c"Command bar hidden".as_ptr(),
    help: c"".as_ptr(),
};

pub fn bar_hidden() -> bool {
    migrate();
    on(&ini(&BAR_HIDDEN), &BAR_HIDDEN)
}

pub fn set_bar_hidden(v: bool) {
    crate::settings::set_row(&BAR_HIDDEN, if v { "1" } else { "0" });
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
    let was = Ini::open_plain(&old).int("HUD", "BarHidden", 0);
    let osec = overlay::section(BAR_HIDDEN.file(), BAR_HIDDEN.section());
    let has = Ini::open_plain(&overlay::path()).string(&osec, BAR_HIDDEN.key()).is_some_and(|v| !v.is_empty());
    let what = if has {
        "the overlay already has BarHidden"
    } else if was == 0 {
        "BarHidden=0, the default: nothing to carry over"
    } else if overlay::set_row(&BAR_HIDDEN, "1") {
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
