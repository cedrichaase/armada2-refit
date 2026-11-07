//! The resources, ours (`[HUD] Resources=1`; HUD.md, "The resources"): a strip at the
//! top of the screen with one cell per resource the player's race has (those the
//! race's GUI config keeps), in the order `ORDER` gives (the materials first, then the
//! people), each its name, its amount as stock writes it (officers
//! as "have/most"), a mark in the stock panel's colour for it, and the game's tooltip.
use crate::engine::api::topbar;
use crate::gfx::{fade, Corners, Weight, R};
use crate::widgets::{theme, Align, Anchor, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// The resources, a panel of the table (`panels::PANELS`).
pub struct Resources;

impl super::panels::Panel for Resources {
    fn name(&self) -> &'static str {
        "resources"
    }
    fn scope(&self) -> &'static str {
        "hud.resources"
    }
    fn key(&self, c: &crate::config::Hud) -> bool {
        c.resources
    }
    fn component(&self) -> (usize, &'static str) {
        (topbar::resource_panel(), "ResourcePanel")
    }
    fn with_hud(&self) -> bool {
        true
    }
    fn draw(&self, ui: &mut Ui) {
        run(ui)
    }
}

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
            // Roots' short tooltip is one icon glyph (the set's word for its picture, by
            // `text::game`, or a control character without it); its long one is
            // "Tritanium: <glyph>" and a sentence, in the game's own table and by race: the
            // name is read from it (decisions 153 and 160).
            let mut name = if crate::content::get().is_glyph(&tip) { None } else { word(&tip) };
            let mut text = tip.clone();
            if name.is_none() {
                let long = topbar::tooltip(c, true).unwrap_or_default();
                name = word(&long);
                if name.is_some() {
                    text = long.chars().filter(|c| !c.is_control() || *c == '\n').collect();
                }
            }
            let name = name.unwrap_or_else(|| String::from(crate::content::get().resource(r as usize)));
            let tip = text;
            n.names.push((c, name, tip));
        }
        if !same {
            let mut line = String::new();
            for (i, &(r, _)) in shown.iter().enumerate() {
                line.push_str(&crate::sfmt!(" {}={}({}) tip {:?}", crate::content::get().resource(r as usize), topbar::amount_text(r), n.names[i].1, n.names[i].2));
            }
            crate::log!("resources:{}", line);
        }
        n.at = now;
    }
    &n.names
}

/// A resource's name out of a tooltip: its first line up to the colon ("Crew: 1200/3250"),
/// when that is a word (not a control character, which draws as "?", nor a sentence).
fn word(tip: &str) -> Option<String> {
    let first = tip.lines().next().unwrap_or("");
    let first = first.split(':').next().unwrap_or("").trim();
    let ok = !first.is_empty() && first.len() <= 24 && !first.chars().any(|c| c.is_control()) && first.chars().any(|c| c.is_alphabetic());
    ok.then(|| String::from(first))
}

/// The cells' order, by `eResource`: dilithium, metal, latinum, biomatter, crew,
/// officers (stock's panel goes by `eResource`: crew first).
const ORDER: [u32; 6] = [2, 4, 3, 5, 0, 1];

pub fn run(ui: &mut Ui) {
    let mut shown = topbar::shown();
    shown.sort_by_key(|&(r, _)| ORDER.iter().position(|&o| o == r).unwrap_or(6));
    if shown.is_empty() || crate::engine::api::hud::hidden(topbar::resource_panel()) {
        return;
    }
    let names = names(crate::rt::now(), &shown);
    let t = ui.theme;
    let (cell_w, h, gap) = (ui.u(150.0), ui.u(46.0), ui.u(6.0));
    let w = shown.len() as f32 * cell_w + (shown.len() - 1) as f32 * gap;
    let (outer, inner) = place(ui, w, h);
    // Lamps in the frame (Starfleet): the user wants them on the resource strip.
    ui.panel_lit(outer);
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

/// The strip's rectangle and its content's. At the top centre of the screen, as the stock
/// panel is, unless the content set says the game's own resource panel has a place
/// (`[Resources] Placement = component`): then its top left corner, which is where that
/// game draws its resources, the top centre being its command and message area.
fn place(ui: &Ui, w: f32, h: f32) -> (R, R) {
    use crate::engine::api::hud;
    if crate::content::get().resources_at_component {
        if let Some(p) = hud::panel_rect(topbar::resource_panel(), ui.w, ui.h) {
            let (outer, inner) = super::panels::fit(ui, Anchor::TopLeft, w, h, 0.0);
            let (dx, dy) = (p[0] - outer.x, p[1] - outer.y);
            return (R::new(outer.x + dx, outer.y + dy, outer.w, outer.h), R::new(inner.x + dx, inner.y + dy, inner.w, inner.h));
        }
    }
    super::panels::fit(ui, Anchor::Top, w, h, ui.u(6.0))
}
