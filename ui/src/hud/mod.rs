//! The in-mission HUD drawn by the UI, a panel at a time (HUD.md). Each panel of ours
//! replaces one stock `DisplayInterface` component, behind its `[HUD]` key: the stock
//! component keeps simulating, so its state stays current, and the panel switch
//! (`engine::hud`) stops it drawing and taking the mouse. Ours reads that state and
//! acts through the component's own functions.
pub mod bar;

use crate::engine::{self, hud};
use crate::widgets::Ui;

static mut BAR: bool = false;

/// Patches what the panels switched on in UI.ini need; each stands down on its own.
pub fn install() -> bool {
    let cfg = &crate::config::get().hud;
    if !cfg.enable {
        crate::log!("hud: Enable=0, the HUD is stock");
        return false;
    }
    if !cfg.bar {
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
        if BAR {
            bar::run(ui);
        }
    }
}
