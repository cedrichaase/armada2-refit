//! UI.ini, read at startup from the game directory.
use crate::rt::settings::{on, value};
use crate::rt::{self, Ini};
use crate::settings_table::*;
use core::sync::atomic::{AtomicPtr, Ordering};

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
    /// `[UI] Movies`: the cutscene proxy's full-screen movies are drawn by the UI
    /// (`movie`), not copied into the engine's locked back buffer.
    pub movies: bool,
    /// `[UI] PointerLog`: log the focus changes and the pointer's jumps between two
    /// frames (`input::diagnose`), for the cursor that jumps after a change of focus.
    pub pointer_log: bool,
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
    /// `[Menus] Scene`: the main menu and the campaign selection stand over render3d's
    /// 3D scene (`menus::scene`), when render3d draws it.
    pub menus_scene: bool,
    /// `[Menus] CampaignLocks`: the campaign selection shows which campaigns are not yet
    /// open (`menus::locks`).
    pub campaign_locks: bool,
    /// `[Menus] LockedSound`: the sound a click on a locked campaign plays (a file the
    /// game's sound system finds by name; empty: none).
    pub locked_sound: alloc::string::String,
    /// `[Menus] Credits`: the Credits are ours (`crate::credits`), over the menu scene.
    pub credits: bool,
    pub hud: Hud,
    /// `[Loading] Enable`: the loading screen of ours (`crate::loading`), in place of
    /// the engine's logo scene and bar.
    pub loading: bool,
    /// `[Loading] Hold`: seconds the load waits at its half-way point, the screen
    /// still drawn, for the test bench to look at it (0: never).
    pub loading_hold: f32,
}

/// `[HUD]` in UI.ini: which stock panels ours replace. Every panel is on by default,
/// and `Enable=0` leaves the whole HUD stock whatever the panel keys say.
#[derive(Clone, Debug, Default)]
pub struct Hud {
    pub enable: bool,
    /// The button bar (`PopupPaletteImp`), with the grid keys.
    pub bar: bool,
    /// The key in each button's corner.
    pub bar_labels: bool,
    /// Our own icons (`icons`) on the bar's and the top-right buttons in place of the
    /// game's button art; 0: the game's.
    pub icons: bool,
    /// The info panel (`ShipDisplay`).
    pub info: bool,
    /// The info panel shows a single unit live, from a camera of its own, in the place
    /// of its wireframe (`engine::closeup`; needs render3d).
    pub info_camera: bool,
    /// The info panel's size, a factor on the UI's scale.
    pub info_scale: f32,
    /// The minimap (`MapRadar`), ours, and its size in units.
    pub minimap: bool,
    pub minimap_size: f32,
    /// The selection overlay (`SelectionDisplay`) over the 3D view, ours; with
    /// `SelectionGauges=1` the selected units always show their gauges (0: as stock,
    /// while the pointer is on one of them).
    pub selection: bool,
    pub selection_gauges: bool,
    /// The engine's 3D selection bubbles (the ellipsoid about each selected unit);
    /// off by default while the overlay's brackets mark the selection.
    pub selection_bubbles: bool,
    /// The unit view (`CinematicView`) placed by us, in our frame; the engine still
    /// draws the view. Its side in units.
    pub unit_view: bool,
    pub unit_view_size: f32,
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
    /// The mission scripts' team texts (`engine::trade`): the trade status ("Sent to
    /// Team 2") and the team message.
    pub team_texts: bool,
    /// The multiplayer boxes (HUD.md, "Comms, the drop box and the load wait"): the comm
    /// panel, the dropped-player box, the wait for the other players' loads.
    pub comm: bool,
    pub drop_player: bool,
    pub load_wait: bool,
    /// The tooltips the engine shows at the pointer (`CursorInterface`: an object's
    /// name and description over the 3D view), drawn by us in the theme.
    pub tooltips: bool,
    /// The engine's last draws in a mission, drawn by us (`engine::leftovers`):
    /// `Overview`, the 3D view's drag box, height posts, order ring and placement markers;
    /// `Fade`, the scripts' fade
    /// to and from black; `UnitVideo`, the in-mission Bink video (the unit view's).
    pub overview: bool,
    /// `Placement`, how the placement markers (a placement's footprint and the cells
    /// that are not clear) are drawn under `Overview`: 1 in the theme (`hud::placement`),
    /// 0 with the engine's sprites.
    pub placement: bool,
    pub fade: bool,
    pub unit_video: bool,
    /// The leftovers of ui 0.25.0 (HUD.md, "The engine's last draws"): `SystemIcons`,
    /// the pointed unit's damaged systems over it (with `Selection`); `ShotScenes`,
    /// the shot director's scenes ringed on the minimap in the tactical view (with
    /// `Minimap`); `MinimapCursor`, the action cursor over the minimap (with
    /// `Minimap` and `[UI] Cursor`); `FrameRate`, what `FRAMERATE_DISPLAY` prints.
    pub system_icons: bool,
    pub shot_scenes: bool,
    pub minimap_cursor: bool,
    pub frame_rate: bool,
    /// `[HUD] LayoutLog`: for the test bench, every panel's rectangle written to UI.log
    /// when the layout changes (`hud::layout`). 0 in play.
    pub layout_log: bool,
}

pub fn load() -> Config {
    let ini = Ini::open(&rt::game_path("UI.ini"));
    Config {
        enable: ini.int("UI", "Enable", 1) != 0,
        log: ini.int("UI", "Log", 1) != 0,
        scale: ini.float("UI", "Scale", 1.0).clamp(0.25, 4.0),
        cursor: on(&ini, UI_CURSOR),
        cursor_size: ini.float("UI", "CursorSize", 32.0).clamp(16.0, 128.0),
        movies: ini.int("UI", "Movies", 1) != 0,
        pointer_log: ini.int("UI", "PointerLog", 0) != 0,
        theme: {
            let l = rt::lower(value(&ini, UI_THEME).trim());
            (!l.is_empty() && l != "auto").then_some(l)
        },
        demo_mission: ini.int("Demo", "Mission", 0) != 0,
        demo_shell: ini.int("Demo", "Shell", 0) != 0,
        demo_loading: ini.int("Demo", "Loading", 0) != 0,
        demo_style: ini.int("Demo", "Style", 0) != 0,
        hud: Hud {
            enable: on(&ini, HUD_ENABLE),
            bar: on(&ini, HUD_BAR),
            bar_labels: on(&ini, HUD_BARLABELS),
            icons: on(&ini, HUD_ICONS),
            info: on(&ini, HUD_INFO),
            info_camera: on(&ini, HUD_INFOCAMERA),
            info_scale: ini.float("HUD", "InfoScale", 1.3).clamp(0.6, 2.5),
            minimap: on(&ini, HUD_MINIMAP),
            minimap_size: ini.float("HUD", "MinimapSize", 300.0).clamp(120.0, 900.0),
            selection: on(&ini, HUD_SELECTION),
            selection_gauges: on(&ini, HUD_SELECTIONGAUGES),
            selection_bubbles: ini.int("HUD", "SelectionBubbles", 0) != 0,
            unit_view: on(&ini, HUD_UNITVIEW),
            unit_view_size: ini.float("HUD", "UnitViewSize", 260.0).clamp(120.0, 600.0),
            resources: ini.int("HUD", "Resources", 1) != 0,
            buttons: ini.int("HUD", "Buttons", 1) != 0,
            messages: ini.int("HUD", "Messages", 1) != 0,
            pause: ini.int("HUD", "Pause", 1) != 0,
            replay: ini.int("HUD", "Replay", 1) != 0,
            overlay: ini.int("HUD", "Overlay", 1) != 0,
            chat: ini.int("HUD", "Chat", 1) != 0,
            team_texts: ini.int("HUD", "TeamTexts", 1) != 0,
            comm: ini.int("HUD", "Comm", 1) != 0,
            drop_player: ini.int("HUD", "DropPlayer", 1) != 0,
            load_wait: ini.int("HUD", "LoadWait", 1) != 0,
            tooltips: on(&ini, HUD_TOOLTIPS),
            overview: ini.int("HUD", "Overview", 1) != 0,
            placement: ini.int("HUD", "Placement", 1) != 0,
            fade: ini.int("HUD", "Fade", 1) != 0,
            unit_video: ini.int("HUD", "UnitVideo", 1) != 0,
            system_icons: ini.int("HUD", "SystemIcons", 1) != 0,
            shot_scenes: ini.int("HUD", "ShotScenes", 1) != 0,
            minimap_cursor: ini.int("HUD", "MinimapCursor", 1) != 0,
            frame_rate: ini.int("HUD", "FrameRate", 1) != 0,
            layout_log: ini.int("HUD", "LayoutLog", 0) != 0,
        },
        menus: on(&ini, MENUS_ENABLE),
        menus_widgets: rt::lower(&ini.string("Menus", "Generic").unwrap_or_default()).trim() == "widgets",
        menus_specs: ini.int("Menus", "Specs", 1) != 0,
        menus_escape: ini.int("Menus", "EscapeReturns", 1) != 0,
        menus_trace: ini.int("Menus", "Trace", 0) != 0,
        menus_host: ini.int("Menus", "Host", 1) != 0,
        menus_scene: on(&ini, MENUS_SCENE),
        credits: on(&ini, MENUS_CREDITS),
        campaign_locks: ini.int("Menus", "CampaignLocks", 1) != 0,
        locked_sound: alloc::string::String::from(ini.string("Menus", "LockedSound").as_deref().unwrap_or("mnu_back.wav").trim()),
        loading: on(&ini, LOADING_ENABLE),
        loading_hold: ini.float("Loading", "Hold", 0.0).clamp(0.0, 600.0),
    }
}

/// The configuration in force: swapped whole by `reload`, so a reader never sees half of
/// it. A replaced one is leaked (a few hundred bytes, once per settings change), which
/// keeps `get()`'s `&'static` honest.
static CONFIG: AtomicPtr<Config> = AtomicPtr::new(core::ptr::null_mut());

pub fn set(c: Config) {
    let new = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(c));
    CONFIG.store(new, Ordering::Release);
}

/// The configuration, None before it is read or with `Enable=0`.
pub fn try_get() -> Option<&'static Config> {
    unsafe { CONFIG.load(Ordering::Acquire).as_ref() }
}

pub fn get() -> &'static Config {
    try_get().expect("config read at startup")
}

/// `UI_SettingsChanged`'s part: reads UI.ini and the overlay again and swaps in a
/// configuration that takes the keys read every frame from the new read and keeps the
/// rest as the start made it (a key a patch was installed for stays as installed: it is
/// a restart row).
pub fn reload() {
    let Some(old) = try_get() else { return };
    let new = load();
    let mut c = old.clone();
    c.scale = new.scale;
    c.cursor = new.cursor;
    c.cursor_size = new.cursor_size;
    c.theme = new.theme;
    c.menus_scene = new.menus_scene;
    let (h, n) = (&mut c.hud, &new.hud);
    h.minimap = n.minimap;
    h.minimap_size = n.minimap_size;
    h.info = n.info;
    h.info_camera = n.info_camera;
    h.info_scale = n.info_scale;
    h.icons = n.icons;
    h.bar_labels = n.bar_labels;
    h.selection_gauges = n.selection_gauges;
    h.tooltips = n.tooltips;
    h.unit_view_size = n.unit_view_size;
    h.system_icons = n.system_icons;
    h.shot_scenes = n.shot_scenes;
    h.minimap_cursor = n.minimap_cursor;
    h.placement = n.placement;
    h.layout_log = n.layout_log;
    set(c);
}

// ---- the settings hub (tools/settings/README.md, contract C5) ---------------------------

/// Each row's value as of the last read, to say what a change applied.
static mut LAST: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();

fn read_rows() -> alloc::vec::Vec<alloc::string::String> {
    SETTINGS.iter().map(|r| value(&crate::rt::settings::ini(r), r)).collect()
}

/// The values at the start, the baseline for the first `UI_SettingsChanged`.
#[allow(static_mut_refs)]
pub fn remember() {
    unsafe { LAST = read_rows() };
}

/// Called by the hub on the game thread after `RefitPlayer.ini` changed: reads the keys
/// again, swaps the configuration (`reload`) and switches the panels that can change
/// while playing (`hud::apply_live`). The rows read every frame take the new value at
/// the next frame; a restart row is not applied again.
#[unsafe(no_mangle)]
#[allow(static_mut_refs)]
pub extern "C" fn UI_SettingsChanged() {
    rt::ini_refresh_all();
    if try_get().is_none() {
        return;
    }
    let now = read_rows();
    let mut applied = alloc::string::String::new();
    let mut latched = alloc::string::String::new();
    let mut bar_hidden = false;
    for (i, r) in SETTINGS.iter().enumerate() {
        if unsafe { LAST.get(i) } == Some(&now[i]) {
            continue;
        }
        if core::ptr::eq(r, HUD_BARHIDDEN) {
            bar_hidden = true;
        }
        let list = if r.apply == crate::rt::settings::APPLY_LIVE { &mut applied } else { &mut latched };
        if !list.is_empty() {
            list.push_str(", ");
        }
        list.push_str(&rt::sfmt!("{}.{}={}", r.section(), r.key(), now[i]));
    }
    unsafe { LAST = now };
    reload();
    crate::hud::apply_live(&get().hud);
    if bar_hidden {
        crate::hud::bar_pref_changed();
    }
    if applied.is_empty() && latched.is_empty() {
        crate::log!("settings: re-read, nothing changed");
    } else {
        if !applied.is_empty() {
            crate::log!("settings: applied {}", applied);
        }
        if !latched.is_empty() {
            crate::log!("settings: {} (next mission or restart)", latched);
        }
    }
}
