//! UI.ini, read at startup from the game directory.
use crate::rt::{self, Ini};

#[derive(Clone, Debug)]
pub struct Config {
    pub enable: bool,
    pub log: bool,
    /// Multiplies the size the UI is laid out at (1.0: 1080 pixels tall).
    pub scale: f32,
    /// `[Demo]`: the runtime's demonstration overlay, per context.
    pub demo_mission: bool,
    pub demo_shell: bool,
    pub demo_loading: bool,
    /// `[Menus]`: the stock shell's dialogs hosted, drawn and driven by the UI
    /// (`engine::dialogs`, `menus`).
    pub menus: bool,
    /// `[Menus] Generic`: how a screen without a spec is drawn. 0: the dialog as the
    /// game paints it (mirrored), 1: its controls as our widgets.
    pub menus_widgets: bool,
    /// `[Menus] Specs`: screens with a layout of our own use it.
    pub menus_specs: bool,
    /// `[Menus] EscapeReturns`: Esc in the in-mission menu is Return to Game.
    pub menus_escape: bool,
    /// `[Menus] Trace`: log every hosted dialog's messages of note.
    pub menus_trace: bool,
    /// `[Menus] Host=0`: the dialogs run stock (user32's), only traced: a baseline for
    /// `Trace=1`.
    pub menus_host: bool,
}

pub fn load() -> Config {
    let ini = Ini::open(&rt::game_path("UI.ini"));
    Config {
        enable: ini.int("UI", "Enable", 1) != 0,
        log: ini.int("UI", "Log", 1) != 0,
        scale: ini.float("UI", "Scale", 1.0).clamp(0.25, 4.0),
        demo_mission: ini.int("Demo", "Mission", 0) != 0,
        demo_shell: ini.int("Demo", "Shell", 0) != 0,
        demo_loading: ini.int("Demo", "Loading", 0) != 0,
        menus: ini.int("Menus", "Enable", 0) != 0,
        menus_widgets: rt::lower(&ini.string("Menus", "Generic").unwrap_or_default()).trim() == "widgets",
        menus_specs: ini.int("Menus", "Specs", 1) != 0,
        menus_escape: ini.int("Menus", "EscapeReturns", 1) != 0,
        menus_trace: ini.int("Menus", "Trace", 0) != 0,
        menus_host: ini.int("Menus", "Host", 1) != 0,
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
