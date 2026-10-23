//! The in-mission HUD drawn by the UI, a panel at a time (HUD.md). Each panel of ours
//! replaces one stock `DisplayInterface` component, behind its `[HUD]` key: the stock
//! component keeps simulating, so its state stays current, and the panel switch
//! (`engine::hud`) stops it drawing and taking the mouse. Ours reads that state and
//! acts through the component's own functions.
pub mod bar;
pub mod info;
pub mod layout;
pub mod leftovers;
pub mod minimap;
pub mod overlay;
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

use crate::engine::{self, hud};
use crate::widgets::Ui;

static mut BAR: bool = false;
static mut INFO: bool = false;
static mut MINIMAP: bool = false;
static mut SELECTION: bool = false;
static mut UNIT_VIEW: bool = false;
static mut TOOLTIPS: bool = false;

/// Patches what the panels switched on in UI.ini need; each stands down on its own.
pub fn install() -> bool {
    let cfg = &crate::config::get().hud;
    if !cfg.enable {
        crate::log!("hud: Enable=0, the HUD is stock");
        return false;
    }
    if !cfg.bar && !cfg.info && !cfg.minimap && !cfg.selection && !cfg.unit_view && !cfg.tooltips && !panels::any(cfg) && !comm::any(cfg) {
        crate::log!("hud: no panel switched on, the HUD is stock");
        return false;
    }
    if !hud::functions_stock() || !engine::sound::functions_stock() || !engine::msvc::available() {
        crate::log!("hud: an engine function is not stock, the HUD stays stock");
        return false;
    }
    if !hud::install() {
        return false;
    }
    map_panels(cfg);
    if cfg.bar {
        let grid = unsafe { crate::rt::win::GetModuleHandleA(c"GridLayout.asi".as_ptr() as *const u8) };
        if !grid.is_null() {
            crate::log!("bar: GridLayout.asi is loaded and owns the bar: ours stands down");
        } else if bar::install() {
            hud::set_taken(hud::PALETTE_PTR, true, true);
            unsafe { BAR = true };
            crate::log!("bar: ours, keys {}", if cfg.bar_keys { "QWERT/ASDFG/ZXCVB by position" } else { "stock" });
        } else {
            crate::log!("bar: stays stock");
        }
    }
    if cfg.info {
        if engine::info::functions_stock() {
            hud::set_taken(hud::SHIP_DISPLAY, false, true);
            unsafe { INFO = true };
            crate::log!("info: ours");
        } else {
            crate::log!("info: an engine function is not stock, the info panel stays stock");
        }
    }
    panels::install(cfg);
    comm::install(cfg);
    if cfg.tooltips {
        hud::take_tooltips(true);
        unsafe { TOOLTIPS = true };
        crate::log!("tooltips: ours");
    }
    true
}

/// The minimap, the selection overlay and the unit view's frame (minimap.rs,
/// overlay.rs, unitview.rs).
fn map_panels(cfg: &crate::config::Hud) {
    if cfg.minimap {
        if engine::radar::functions_stock() && minimap::install() {
            hud::set_taken(engine::radar::MAP_RADAR, false, true);
            unsafe { MINIMAP = true };
            crate::log!("minimap: ours");
        } else {
            crate::log!("minimap: stays stock");
        }
    }
    if cfg.selection {
        if engine::overlay::functions_stock() && engine::radar::functions_stock() {
            hud::set_taken(engine::overlay::SELECTION_DISPLAY, false, true);
            unsafe { SELECTION = true };
            crate::log!("selection: ours, gauges {}", if cfg.selection_gauges { "on the selection" } else { "as stock" });
            if !cfg.selection_bubbles {
                let off = engine::overlay::hide_bubbles();
                crate::log!("selection: the 3D bubbles {}", if off { "off" } else { "left on: RenderSelectionEffects is not stock" });
            }
        } else {
            crate::log!("selection: stays stock");
        }
    }
    if cfg.unit_view {
        unsafe { UNIT_VIEW = true };
        if engine::unitview::functions_stock() {
            engine::unitview::enable();
            crate::log!("unit view: placed by us, in our frame");
        } else {
            crate::log!("unit view: its Render is not stock: our frame around the stock place");
        }
    }
}

/// The minimap's map square, when the minimap is ours and shows: a move ordered there
/// is the radar's (`cursor::draw` shows the warp or move cursor over it).
pub fn minimap_square(ui: &Ui) -> Option<crate::gfx::R> {
    if unsafe { !MINIMAP } || engine::ingame::hud_hidden() || hud::hidden(engine::radar::MAP_RADAR) {
        return None;
    }
    Some(minimap::place(ui).2)
}

/// The minimap's rectangle in pixels while it shows: ours, else the stock radar's (on
/// the stock canvas, stretched without HUD.asi).
pub fn minimap_rect(ui: &Ui) -> Option<[f32; 4]> {
    if hud::hidden(hud::MAP_RADAR) {
        None
    } else if unsafe { MINIMAP } {
        let (o, _, _) = minimap::place(ui);
        Some([o.x, o.y, o.right(), o.bottom()])
    } else {
        hud::panel_rect(hud::MAP_RADAR, ui.w, ui.h)
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
        if !BAR_PENDING || engine::ingame::hud_hidden() {
            return;
        }
        let p = engine::palette::palette();
        let b = engine::topbar::button(engine::topbar::Which::Palette);
        if p == 0 || b == 0 || !hud::in_view(p) {
            return;
        }
        BAR_PENDING = false;
        let want = crate::prefs::bar_hidden();
        let mode = engine::palette::mode(p);
        let press = (mode == 0) != want;
        crate::log!(
            "bar: {} as the player set it (palette mode {}, hidden flag {}){}",
            if want { "hidden" } else { "shown" },
            mode,
            hud::flag_hidden(p) as u8,
            if press { ", toggled" } else { "" }
        );
        if press {
            engine::buttons::queue(b);
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
        // The info panel first: the bar is placed against it.
        if INFO {
            layout::at("info");
            info::run(ui);
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
