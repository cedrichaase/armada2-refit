//! The bottom row on a game whose info panel is its own (Fleet Operations: Roots; HUD.md, "The
//! bottom row on Fleet Operations: Roots", PLAN-FLEETOPS.md, FO-6, decisions 440 to 459): where the
//! info panel and the bar sit, from `[HUD] BottomRow`.
//!
//! - `BottomRow=minimap, info, bar` (any order of the three): along the bottom of the screen, left
//!   to right, the bar as high as the info panel (its three rows of cells fitted to it, or
//!   `BarCellSize`), the info panel in the room left (at most its own width, or `InfoWidth`).
//! - `BottomRow=auto`: that row where the info panel keeps its own width beside the bar (16:9 and
//!   wider), else the bar above the minimap.
//! - `BottomRow=stacked` (or empty): the bar above the minimap, the info panel in its own place.
//!
//! The info panel's own place is Roots': the panel's rectangle as the engine has it (learnt from
//! the panel while it shows, kept for the resolution), else its layout's place estimated from
//! the screen's height. Where the info panel keeps that left edge, the engine's pointer, hover
//! and FleetOpsHook's range rings stay whole (`info_fo`).
use crate::gfx::R;
use crate::widgets::{theme, Ui};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Minimap,
    Info,
    Bar,
}

/// `BottomRow=`: None for stacked (or empty), Some(empty) for auto, else the order.
pub fn parse(s: &str) -> Option<Vec<Part>> {
    let s = s.trim().to_ascii_lowercase();
    match s.as_str() {
        "" | "stacked" | "above" | "0" => return None,
        "auto" => return Some(Vec::new()),
        _ => {}
    }
    let mut out = Vec::new();
    for w in s.split(',') {
        let p = match w.trim() {
            "minimap" | "map" | "radar" => Part::Minimap,
            "info" | "infopanel" | "panel" => Part::Info,
            "bar" | "grid" | "buttons" => Part::Bar,
            _ => continue,
        };
        if !out.contains(&p) {
            out.push(p);
        }
    }
    // A row needs the info panel and the bar in it; the minimap stays where it is.
    if !out.contains(&Part::Info) || !out.contains(&Part::Bar) {
        return None;
    }
    Some(out)
}

/// This frame's plan: the info panel's rectangle, and the bar's (with its cell) when the bar is
/// in the row.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plan {
    pub info: R,
    /// The info panel's own (Roots') rectangle: the left edge the engine's children are local to.
    pub native: R,
    pub bar: Option<(R, f32)>,
}

static mut PLAN: Option<Plan> = None;
/// The info panel's own rectangle in pixels, as the panel last had it, and the screen it was for.
static mut NATIVE: Option<(R, (f32, f32))> = None;

/// The info panel's own rectangle (Roots' layout) learnt from the panel this frame.
pub fn learn_native(ui: &Ui, r: R) {
    unsafe { NATIVE = Some((r, (ui.w, ui.h))) };
}

/// The plan of this frame, made by `plan`.
pub fn current() -> Option<Plan> {
    unsafe { PLAN }
}

pub fn clear() {
    unsafe { PLAN = None };
}

/// The info panel's own rectangle: as the panel had it on this screen, else estimated from the
/// layout table (its units are square on the layouts Roots picks by aspect, as high as the HUD canvas).
fn native(ui: &Ui) -> Option<R> {
    if let Some((r, s)) = unsafe { NATIVE } {
        if s == (ui.w, ui.h) {
            return Some(r);
        }
    }
    let l = &crate::content::get().info;
    if l.is_empty() {
        return None;
    }
    let k = ui.h / crate::engine::api::CANVAS.1;
    let place = l.numbers("Place");
    let (x, y) = if place.len() >= 2 { (place[0], place[1]) } else { (352.0, 1038.0) };
    Some(R::new(libm::roundf(x * k), libm::roundf(y * k), libm::roundf(l.size[0] * k), ui.h - libm::roundf(y * k)))
}

/// The bar's outer size for a cell `cs` (the grid's own frame and padding round 5 x 3 cells).
fn bar_size(ui: &Ui, cs: f32) -> (f32, f32) {
    super::bargrid::size_for(ui, cs)
}

/// The cell that fits three rows into `h` pixels of bar.
fn cell_for(ui: &Ui, h: f32) -> f32 {
    // The grid's padding and gaps shrink with a cell under its own 62 units (`bargrid::size_with`:
    // 2 x 8 and 2 x 5 of every 62), so three rows take the height less the frame.
    let (_, ft, _, fb) = theme::frame_margins(ui);
    let cs = (h - ft - fb) / (3.0 + 26.0 / 62.0);
    let cs = if cs > ui.u(62.0) { (h - ft - fb - ui.u(26.0)) / 3.0 } else { cs };
    cs.max(ui.u(16.0))
}

/// Lays the bottom row out for this frame (before the info panel and the bar draw).
pub fn plan(ui: &Ui, bar_shown: bool) {
    let cfg = &crate::config::get().hud;
    let Some(native) = native(ui) else {
        unsafe { PLAN = None };
        return;
    };
    let m = ui.u(10.0);
    let gap = ui.u(6.0);
    let own_w = if cfg.info_width > 0.0 { ui.u(cfg.info_width) } else { native.w };
    let stacked = Plan { info: R::new(native.x, native.y, own_w.min(ui.w - native.x), native.h), native, bar: None };
    let Some(order) = cfg.bottom_row.clone() else {
        unsafe { PLAN = Some(stacked) };
        return;
    };
    if !bar_shown {
        unsafe { PLAN = Some(stacked) };
        return;
    }
    let cs = if cfg.bar_cell_size > 0.0 { ui.u(cfg.bar_cell_size) } else { cell_for(ui, native.h) };
    let (bw, bh) = bar_size(ui, cs);
    let auto = order.is_empty();
    let order = if auto { alloc::vec![Part::Minimap, Part::Info, Part::Bar] } else { order };
    // The minimap's right edge (it keeps its own place); its left edge when it comes last.
    let mini = super::minimap_rect(ui);
    let mut x = 0.0f32;
    let mut info_x = native.x;
    let mut bar_x = 0.0;
    let ii = order.iter().position(|&p| p == Part::Info).unwrap_or(0);
    let bi = order.iter().position(|&p| p == Part::Bar).unwrap_or(0);
    let mi = order.iter().position(|&p| p == Part::Minimap);
    for (i, &p) in order.iter().enumerate() {
        match p {
            Part::Minimap => {
                if let Some(r) = mini {
                    x = if i == 0 { r[2] } else { x.max(r[2]) };
                }
            }
            Part::Info => {
                // Right after the minimap the info panel keeps its own left edge (Roots' place).
                info_x = if i > 0 && mi == Some(i - 1) { native.x.max(x) } else if i == 0 { native.x.min(m) } else { x + gap };
                // The room up to the next part (the bar after it, or the minimap at the end).
                let mut right = ui.w;
                if bi > ii {
                    right -= bw + gap;
                }
                if let (Some(j), Some(r)) = (mi, mini) {
                    if j > ii {
                        right = right.min(r[0] - gap);
                    }
                }
                let w = own_w.min(right - info_x).max(0.0);
                x = info_x + w;
            }
            Part::Bar => {
                bar_x = if i == 0 { m } else { x + gap };
                x = bar_x + bw;
            }
        }
    }
    let room_right = if bi > ii { ui.w - bw - gap } else { mini.filter(|_| mi.is_some_and(|j| j > ii)).map_or(ui.w, |r| r[0] - gap) };
    let info_w = own_w.min(room_right - info_x).max(0.0);
    // Auto: the row only where the info panel keeps its own width beside the bar.
    if auto && (info_w + 0.5 < own_w || bh > native.h + 1.0) {
        unsafe { PLAN = Some(stacked) };
        return;
    }
    // The bar after the info panel sits right after it.
    if bi > ii {
        bar_x = info_x + info_w + gap;
    }
    let bar = R::new(libm::roundf(bar_x), native.bottom() - bh, bw, bh);
    unsafe {
        PLAN = Some(Plan { info: R::new(libm::roundf(info_x), native.y, libm::roundf(info_w), native.h), native, bar: Some((bar, cs)) })
    };
}

/// The bar's place and cell when it is in the row this frame.
pub fn bar() -> Option<(R, f32)> {
    current().and_then(|p| p.bar)
}
