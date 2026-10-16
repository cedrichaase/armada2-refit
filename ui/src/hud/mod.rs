//! The in-mission HUD drawn by the UI, a panel at a time (HUD.md). Each panel of ours
//! replaces one stock `DisplayInterface` component, behind its `[HUD]` key: the stock
//! component keeps simulating, so its state stays current, and the panel switch
//! (`engine::hud`) stops it drawing and taking the mouse. Ours reads that state and
//! acts through the component's own functions.
pub mod bar;
pub mod minimap;
pub mod overlay;
pub mod unitview;

use crate::engine::{self, hud};
use crate::widgets::Ui;

static mut BAR: bool = false;
static mut MINIMAP: bool = false;
static mut SELECTION: bool = false;
static mut UNIT_VIEW: bool = false;

/// Patches what the panels switched on in UI.ini need; each stands down on its own.
pub fn install() -> bool {
    let cfg = &crate::config::get().hud;
    if !cfg.enable {
        crate::log!("hud: Enable=0, the HUD is stock");
        return false;
    }
    if !cfg.bar && !cfg.minimap && !cfg.selection && !cfg.unit_view {
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
        } else {
            crate::log!("selection: stays stock");
        }
    }
    if cfg.unit_view {
        unsafe { UNIT_VIEW = true };
        crate::log!("unit view: our frame");
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
        if BAR {
            bar::run(ui);
        }
    }
}
