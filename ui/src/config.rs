//! UI.ini, read at startup from the game directory.
use crate::rt::{self, Ini};

#[derive(Clone, Debug)]
pub struct Config {
    pub enable: bool,
    pub log: bool,
    /// Multiplies the size the UI is laid out at (1.0: 1080 pixels tall).
    pub scale: f32,
    /// `[Demo]`: the runtime's demonstration overlay, per context.
    /// Our own mouse cursor (cursor.rs), and its size in units.
    pub cursor: bool,
    pub cursor_size: f32,
    /// `[UI] Theme`: a theme's name (`shell`, `federation`, `klingon`, ...) to use
    /// everywhere; None for `auto`, the player's race in a mission and the shell's
    /// theme elsewhere (widgets/theme.rs).
    pub theme: Option<alloc::string::String>,
    pub demo_mission: bool,
    pub demo_shell: bool,
    pub demo_loading: bool,
    /// `[Demo] Style`: the style gallery, every theme and widget state (app.rs), in
    /// a screen of ours in the front end (in the Credits' place) and over a mission.
    pub demo_style: bool,
}

pub fn load() -> Config {
    let ini = Ini::open(&rt::game_path("UI.ini"));
    Config {
        enable: ini.int("UI", "Enable", 1) != 0,
        log: ini.int("UI", "Log", 1) != 0,
        scale: ini.float("UI", "Scale", 1.0).clamp(0.25, 4.0),
        cursor: ini.int("UI", "Cursor", 0) != 0,
        cursor_size: ini.float("UI", "CursorSize", 32.0).clamp(16.0, 128.0),
        theme: ini.string("UI", "Theme").filter(|s| {
            let l = rt::lower(s.trim());
            !l.is_empty() && l != "auto"
        }),
        demo_mission: ini.int("Demo", "Mission", 0) != 0,
        demo_shell: ini.int("Demo", "Shell", 0) != 0,
        demo_loading: ini.int("Demo", "Loading", 0) != 0,
        demo_style: ini.int("Demo", "Style", 0) != 0,
    }
}

static mut CONFIG: Option<Config> = None;

pub fn set(c: Config) {
    unsafe { CONFIG = Some(c) };
}

#[allow(static_mut_refs)]
pub fn get() -> &'static Config {
    unsafe { CONFIG.as_ref() }.expect("config read at startup")
}
