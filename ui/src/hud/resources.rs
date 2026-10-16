//! The resources, ours (`[HUD] Resources=1`; HUD.md, "The resources"): a strip at the
//! top of the screen with one cell per resource the player's race has, in the stock
//! panel's order (crew, officers, dilithium, latinum, metal, biomatter, those the
//! race's GUI config keeps), each its name, its amount as stock writes it (officers
//! as "have/most"), a mark in the stock panel's colour for it, and the game's tooltip.
use crate::engine::topbar;
use crate::gfx::{fade, Corners, Weight, R};
use crate::widgets::{theme, Align, Anchor, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// Each resource's name (its tooltip's first line, the game's localised word), kept
/// per component; looked up again after a second.
struct Names {
    at: f64,
    names: Vec<(usize, String, String)>,
}

static mut NAMES: Names = Names { at: -10.0, names: Vec::new() };

fn names(now: f64, shown: &[(u32, usize)]) -> &'static [(usize, String, String)] {
    let n = unsafe { &mut *core::ptr::addr_of_mut!(NAMES) };
    let same = n.names.len() == shown.len() && n.names.iter().zip(shown).all(|(a, b)| a.0 == b.1);
    if !same || now - n.at > 1.0 {
        n.names.clear();
        for &(r, c) in shown {
            let tip = topbar::tooltip(c, false).unwrap_or_default();
            let first = String::from(tip.lines().next().unwrap_or("").trim().trim_end_matches(':'));
            let name = if first.is_empty() || first.len() > 24 { String::from(topbar::NAMES[r as usize]) } else { first };
            n.names.push((c, name, tip));
        }
        if !same {
            let mut line = String::new();
            for (i, &(r, _)) in shown.iter().enumerate() {
                line.push_str(&crate::sfmt!(" {}={}({})", topbar::NAMES[r as usize], topbar::amount_text(r), n.names[i].1));
            }
            crate::log!("resources:{}", line);
        }
        n.at = now;
    }
    &n.names
}

pub fn run(ui: &mut Ui) {
    let shown = topbar::shown();
    if shown.is_empty() || crate::engine::hud::hidden(topbar::RESOURCE_PANEL) {
        return;
    }
    let now = ui.time();
    let names = names(now, &shown);
    let t = ui.theme;
    let (cell_w, h, gap) = (ui.u(150.0), ui.u(46.0), ui.u(6.0));
    let w = shown.len() as f32 * cell_w + (shown.len() - 1) as f32 * gap;
    let (outer, inner) = super::panels::fit(ui, Anchor::Top, w, h, ui.u(6.0));
    ui.panel(outer);
    for (i, &(r, _)) in shown.iter().enumerate() {
        let cell = R::new(inner.x + i as f32 * (cell_w + gap), inner.y, cell_w, inner.h);
        // The mark: a short bar in the stock panel's colour for this resource.
        let mark = R::new(cell.x, cell.y + cell.h * 0.18, ui.u(4.0), cell.h * 0.64);
        ui.draw.fill(mark, Corners::round(ui.u(2.0)), topbar::colour(r));
        let text_x = mark.right() + ui.u(8.0);
        let label = R::new(text_x, cell.y, cell.right() - text_x, cell.h * 0.42);
        let name = names.get(i).map_or("", |n| n.1.as_str());
        ui.text_in(label, &theme::caps(name), t.micro_size, Align::Left, t.text_dim, Weight::Regular);
        let value = R::new(text_x, cell.y + cell.h * 0.38, cell.right() - text_x, cell.h * 0.62);
        ui.text_in(value, &topbar::amount_text(r), t.heading_size, Align::Left, t.text, Weight::Bold);
        if i + 1 < shown.len() {
            let sep = R::new(cell.right() + gap * 0.5 - 0.5, cell.y + cell.h * 0.15, 1.0f32.max(ui.u(1.0)), cell.h * 0.7);
            ui.draw.rect(sep, fade(t.panel_edge, 0.6));
        }
        if let Some((_, _, tip)) = names.get(i) {
            if !tip.is_empty() {
                let tip = tip.clone();
                ui.tooltip(cell, &tip);
            }
        }
    }
}
