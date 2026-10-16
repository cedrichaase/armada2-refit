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
    /// `[HUD]`: the in-mission panels of ours that replace stock ones (HUD.md).
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
    pub hud: Hud,
}

/// `[HUD]` in UI.ini: which stock panels ours replace. Every panel is off by default,
/// and `Enable=0` leaves the whole HUD stock whatever the panel keys say.
#[derive(Clone, Debug, Default)]
pub struct Hud {
    pub enable: bool,
    /// The button bar (`PopupPaletteImp`), with the grid keys.
    pub bar: bool,
    /// The bar's keys are the grid's (QWERT/ASDFG/ZXCVB by position); 0 keeps stock's.
    pub bar_keys: bool,
    /// The key in each button's corner.
    pub bar_labels: bool,
    /// The panels of hud-panels (HUD.md): the resources, the top-right buttons, the
    /// message and objectives boxes, the pause box, the replay controls, the overlay
    /// messages, the chat lines and the text entry.
    pub resources: bool,
    pub buttons: bool,
    pub messages: bool,
    pub pause: bool,
    pub replay: bool,
    pub overlay: bool,
    pub chat: bool,
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
        hud: Hud {
            enable: ini.int("HUD", "Enable", 1) != 0,
            bar: ini.int("HUD", "Bar", 0) != 0,
            bar_keys: ini.int("HUD", "BarKeys", 1) != 0,
            bar_labels: ini.int("HUD", "BarLabels", 1) != 0,
            resources: ini.int("HUD", "Resources", 0) != 0,
            buttons: ini.int("HUD", "Buttons", 0) != 0,
            messages: ini.int("HUD", "Messages", 0) != 0,
            pause: ini.int("HUD", "Pause", 0) != 0,
            replay: ini.int("HUD", "Replay", 0) != 0,
            overlay: ini.int("HUD", "Overlay", 0) != 0,
            chat: ini.int("HUD", "Chat", 0) != 0,
        },
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
