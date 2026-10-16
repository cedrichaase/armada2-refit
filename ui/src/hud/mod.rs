//! The in-mission HUD drawn by the UI, a panel at a time (HUD.md). Each panel of ours
//! replaces one stock `DisplayInterface` component, behind its `[HUD]` key: the stock
//! component keeps simulating, so its state stays current, and the panel switch
//! (`engine::hud`) stops it drawing and taking the mouse. Ours reads that state and
//! acts through the component's own functions.
pub mod bar;
pub mod info;
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

static mut LOGGED: f64 = -10.0;

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
        }
        LOGGED = now;
        // A multiplayer box of ours covers the screen: nothing under it takes the mouse.
        if comm::blocking() {
            let screen = ui.screen();
            ui.claim(screen);
            ui.occlude(screen);
        }
        if SELECTION {
            let keep = if MINIMAP { let (o, _, _) = minimap::place(ui); alloc::vec![o] } else { alloc::vec::Vec::new() };
            overlay::run(ui, &keep);
        }
        if UNIT_VIEW {
            unitview::run(ui);
        }
        if MINIMAP {
            minimap::run(ui);
        }
        // The info panel first: the bar is placed against it.
        if INFO {
            info::run(ui);
        }
        if BAR {
            bar::run(ui);
        }
        panels::run(ui);
        comm::run(ui);
        if TOOLTIPS {
            tooltips::run(ui);
        }
    }
}
