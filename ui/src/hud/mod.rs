//! The in-mission HUD drawn by the UI, a panel at a time (HUD.md). Each panel of ours
//! replaces one stock `DisplayInterface` component, behind its `[HUD]` key: the stock
//! component keeps simulating, so its state stays current, and the panel switch
//! (`engine::hud`) stops it drawing and taking the mouse. Ours reads that state and
//! acts through the component's own functions.
pub mod bar;
mod bargrid;
mod barskin;
pub mod info;
pub mod info_fo;
pub mod row;
pub mod layout;
pub mod leftovers;
pub mod minimap;
pub mod overlay;
pub mod placement;
mod tooltips;
pub mod unitview;
// hud-panels: the resources, the top-right buttons, the boxes, the overlay, the chat.
mod buttons;
mod chat;
mod messages;
mod objectives;
mod overlay_text;
pub mod panels;
mod pause;
mod replay;
mod trade;
mod resources;
// ui-multiplayer: the comm panel, the drop box, the load wait.
mod comm;

use crate::engine::api::{self, hud};
use crate::widgets::Ui;

static mut BAR: bool = false;
static mut INFO: bool = false;
static mut MINIMAP: bool = false;
static mut SELECTION: bool = false;
static mut UNIT_VIEW: bool = false;
static mut TOOLTIPS: bool = false;
/// What the start found possible, for the panels `apply_live` switches while playing:
/// the minimap's click hook is in, the info panel's engine functions are stock.
static mut MINIMAP_CAP: bool = false;
static mut INFO_CAP: bool = false;
/// The info panel is FleetOpsHook's (Roots): ours draws it in Roots' layout (`info_fo`).
static mut INFO_FO: bool = false;

/// Whether the selection overlay is ours (`[HUD] Selection=1`, its functions stock).
/// Whether the info panel of ours is Roots' layout over its panel's unseen Render.
pub fn info_fo_on() -> bool {
    unsafe { INFO && INFO_FO }
}

pub fn selection_ours() -> bool {
    unsafe { SELECTION }
}

/// Patches what the panels switched on in UI.ini need; each stands down on its own.
/// Why the HUD stood down at the start, for `UI_SettingsNote`: 0 it runs, 1 `Enable=0`,
/// 2 no panel on, 3 an engine function not stock, 4 the hook could not be installed.
pub static mut STOOD_DOWN: u8 = 0;

pub fn install() -> bool {
    let cfg = &crate::config::get().hud;
    if !cfg.enable {
        crate::log!("hud: Enable=0, the HUD is stock");
        unsafe { STOOD_DOWN = 1 };
        return false;
    }
    if !cfg.bar && !cfg.info && !cfg.minimap && !cfg.selection && !cfg.unit_view && !cfg.tooltips && !panels::any(cfg) && !comm::any(cfg) {
        crate::log!("hud: no panel switched on, the HUD is stock");
        unsafe { STOOD_DOWN = 2 };
        return false;
    }
    if !hud::functions_stock() || !api::msvc::available() {
        crate::log!("hud: an engine function is not stock, the HUD stays stock");
        unsafe { STOOD_DOWN = 3 };
        return false;
    }
    if !hud::install() {
        unsafe { STOOD_DOWN = 4 };
        return false;
    }
    map_panels(cfg);
    if cfg.bar {
        if bar::install() {
            hud::set_taken(hud::palette_ptr(), true, true);
            unsafe { BAR = true };
            if api::palette::slots_indirect() {
                hud::set_bar_skin(true);
                if bargrid::install() {
                    crate::log!("bar: ours as the grid, the stock buttons moved onto its cells (the stock bar keeps the pointer)");
                } else {
                    crate::log!("bar: ours as a skin over the stock buttons (the stock bar keeps the pointer)");
                }
            } else {
                crate::log!("bar: ours, its keys are QOL's");
            }
        } else {
            crate::log!("bar: stays stock");
        }
    }
    unsafe {
        INFO_CAP = api::info::functions_stock();
        INFO_FO = INFO_CAP && api::info_fo::available();
    }
    panels::install(cfg);
    comm::install(cfg);
    apply_live(cfg);
    true
}

/// Switches the panels that can change while playing (the minimap, the info panel, the
/// tooltips) to what `cfg` says, and the bar's key mode; logs each change. At the start
/// it is the install; `UI_SettingsChanged` calls it again.
#[allow(static_mut_refs)]
pub fn apply_live(cfg: &crate::config::Hud) {
    unsafe {
        let want = cfg.minimap && MINIMAP_CAP;
        if want != MINIMAP {
            hud::set_taken(api::radar::map_radar(), false, want);
            MINIMAP = want;
            crate::log!("minimap: {}", if want { "ours" } else { "stock" });
        } else if cfg.minimap && !MINIMAP_CAP && !MINIMAP_LOGGED {
            crate::log!("minimap: stays stock");
        }
        MINIMAP_LOGGED = true;
        let want = cfg.info && INFO_CAP;
        if want != INFO {
            if INFO_FO {
                // Roots: the panel's Render runs unseen under ours (info_fo), never taken.
                let ok = info_fo::set(want);
                INFO = want && ok;
                crate::log!("info: {}", if INFO { "ours, in Roots' layout over its panel's unseen Render" } else { "Roots' own" });
            } else {
                hud::set_taken(hud::INFO_PANEL_TAKEN, false, want);
                INFO = want;
                crate::log!("info: {}", if want { "ours" } else { "stock" });
            }
        } else if cfg.info && !INFO_CAP && !INFO_LOGGED {
            crate::log!("info: an engine function is not stock, the info panel stays stock");
        }
        INFO_LOGGED = true;
        if cfg.tooltips != TOOLTIPS {
            hud::take_tooltips(cfg.tooltips);
            TOOLTIPS = cfg.tooltips;
            crate::log!("tooltips: {}", if cfg.tooltips { "ours" } else { "stock" });
        }
    }
}
static mut MINIMAP_LOGGED: bool = false;
static mut INFO_LOGGED: bool = false;

/// The minimap, the selection overlay and the unit view's frame (minimap.rs,
/// overlay.rs, unitview.rs).
fn map_panels(cfg: &crate::config::Hud) {
    // The minimap is switched in `apply_live`; its click hook goes in whether or not the
    // key is on at the start, so the key can be turned on while playing.
    unsafe { MINIMAP_CAP = api::radar::functions_stock() && minimap::install() };
    if cfg.selection {
        if api::overlay::functions_stock() && api::radar::functions_stock() {
            hud::set_taken(api::overlay::selection_display(), false, true);
            unsafe { SELECTION = true };
            crate::log!("selection: ours, gauges {}", if cfg.selection_gauges { "on the selection" } else { "as stock" });
            if !cfg.selection_bubbles {
                // Registered with Display8 (contract 1.1), the UI claims the bubbles
                // (`CLAIM_SELECTION`) and render3d, asking, draws none. The engine's
                // own are kept from drawing either way: Display8 cannot tell their
                // draws apart.
                let off = api::overlay::hide_bubbles();
                crate::log!(
                    "selection: the 3D bubbles {}{}",
                    if off { "off" } else { "left on: RenderSelectionEffects is not stock" },
                    if crate::claims::registered() { ", claimed (render3d draws none)" } else { "" }
                );
            }
        } else {
            crate::log!("selection: stays stock");
        }
    }
    if cfg.unit_view {
        unsafe { UNIT_VIEW = true };
        if api::unitview::functions_stock() {
            api::unitview::enable();
            crate::log!("unit view: placed by us, in our frame");
        } else {
            crate::log!("unit view: its Render is not stock: our frame around the stock place");
        }
    }
}

/// The minimap's map square, when the minimap is ours and shows: a move ordered there
/// is the radar's (`cursor::draw` shows the warp or move cursor over it).
pub fn minimap_square(ui: &Ui) -> Option<crate::gfx::R> {
    if unsafe { !MINIMAP } || api::ingame::hud_hidden() || hud::hidden(api::radar::map_radar()) {
        return None;
    }
    Some(minimap::place(ui).2)
}

/// The minimap's rectangle in pixels while it shows: ours, else the stock radar's (on
/// the stock canvas, stretched without HUD.asi).
pub fn minimap_rect(ui: &Ui) -> Option<[f32; 4]> {
    if hud::hidden(hud::map_radar()) {
        None
    } else if unsafe { MINIMAP } {
        let (o, _, _) = minimap::place(ui);
        Some([o.x, o.y, o.right(), o.bottom()])
    } else {
        hud::panel_rect(hud::map_radar(), ui.w, ui.h)
    }
}

/// The bottom row between the minimap and the unit view (HUD.md, "The bottom row"):
/// its free span in pixels, and the bar's size while ours shows. The info panel takes
/// its place in it from this, and the bar its place beside the info panel.
pub struct Row {
    pub lo: f32,
    pub hi: f32,
    pub bar: Option<(f32, f32)>,
}

pub fn bottom_row(ui: &Ui) -> Row {
    let lo = minimap_rect(ui).map_or(0.0, |r| r[2]);
    let hi = if unsafe { UNIT_VIEW } { unitview::shown_rect(ui).map_or(ui.w, |r| r.x) } else { ui.w };
    let bar = if unsafe { BAR } && bar::shows() { Some(bar::outer_size(ui)) } else { None };
    Row { lo, hi: hi.max(lo), bar }
}

static mut LOGGED: f64 = -10.0;
/// The bar's shown or hidden state is still to be made the player's (`prefs`): at the
/// start of each mission, and after Game Settings changed it.
static mut BAR_PENDING: bool = true;

/// Game Settings changed the bar's setting: the next mission frame applies it.
pub fn bar_pref_changed() {
    unsafe { BAR_PENDING = true };
}

/// Makes the bar shown or hidden as the player set it, once the HUD shows (not under
/// a mission's opening cinematic), by pressing the stock palette toggle
/// (`ButtonPanel`'s `TogglePalette`), as the top-right button did before it moved to
/// Game Settings.
///
/// The toggle is `PopupPaletteImp::NextPaletteMode`: it steps the palette mode
/// (`+0x110`) between 1, the bar, and 0, the floating palette, which nothing of ours
/// draws; so "hidden" is mode 0, and the mode is what is compared. The component's own
/// hidden flag (`+0x16`) is not the toggle's: the engine sets it for other reasons
/// too, and comparing it could press the toggle with the bar already up and take it
/// away for the mission.
fn apply_bar_pref() {
    unsafe {
        if !BAR_PENDING || api::ingame::hud_hidden() {
            return;
        }
        let p = api::palette::palette();
        let b = api::topbar::button(api::topbar::Which::Palette);
        if p == 0 || b == 0 || !hud::in_view(p) {
            return;
        }
        BAR_PENDING = false;
        let want = crate::prefs::bar_hidden();
        let mode = api::palette::mode(p);
        let press = (mode == 0) != want;
        crate::log!(
            "bar: {} as the player set it (palette mode {}, hidden flag {}){}",
            if want { "hidden" } else { "shown" },
            mode,
            hud::flag_hidden(p) as u8,
            if press { ", toggled" } else { "" }
        );
        if press {
            api::buttons::queue(b);
        }
    }
}

static mut UNDER_BOX: bool = false;

/// Logs once each time the HUD of ours stands aside for the engine's box and comes back.
fn note_under_box(up: bool) {
    unsafe {
        if up != UNDER_BOX {
            UNDER_BOX = up;
            crate::log!("hud: {}", if up { "under the engine's box (objectives or message): the panels of ours stand aside" } else { "the engine's box closed: the panels of ours are back" });
        }
    }
}

/// A mission's frame: every panel of ours.
pub fn run(ui: &mut Ui) {
    if !hud::installed() {
        return;
    }
    // The components, once per mission (a gap in frames is a new one).
    let now = crate::rt::now();
    unsafe {
        if now - LOGGED > 2.0 {
            hud::log_components();
            BAR_PENDING = true;
        }
        LOGGED = now;
        apply_bar_pref();
        // A multiplayer box of ours covers the screen: nothing under it takes the mouse.
        if comm::blocking() {
            let screen = ui.screen();
            ui.claim(screen);
            ui.occlude(screen);
        }
        layout::begin();
        // The engine's briefing or message box is up (not ours): stock's HUD sits under
        // it and the box owns the mouse, so nothing of ours but the boxes and the
        // multiplayer boxes draws or takes input (HUD.md, "Under the engine's boxes").
        if panels::stock_box_up() {
            note_under_box(true);
            panels::run(ui);
            layout::at("comm");
            comm::run(ui);
            layout::end(ui.w, ui.h);
            return;
        }
        note_under_box(false);
        if SELECTION {
            let keep = if MINIMAP { let (o, _, _) = minimap::place(ui); alloc::vec![o] } else { alloc::vec::Vec::new() };
            overlay::run(ui, &keep);
        }
        if UNIT_VIEW {
            layout::at("unitview");
            unitview::run(ui);
        }
        if MINIMAP {
            layout::at("minimap");
            minimap::run(ui);
        }
        // The info panel first: the bar is placed against it. On Roots the bottom row is planned
        // first (`row`): where the info panel and the bar go this frame.
        if INFO && INFO_FO {
            row::plan(ui, BAR && bar::shows());
            layout::at("info");
            info_fo::run(ui);
        } else {
            row::clear();
            if INFO {
                layout::at("info");
                info::run(ui);
            }
        }
        if BAR {
            layout::at("bar");
            bar::run(ui);
        }
        panels::run(ui);
        layout::at("comm");
        comm::run(ui);
        layout::end(ui.w, ui.h);
        if TOOLTIPS {
            tooltips::run(ui);
        }
    }
}
