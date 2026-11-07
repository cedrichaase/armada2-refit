//! The status line about the craft under the pointer, ours (`[HUD] Hover=1`; HUD.md, "The
//! status line under the pointer"): what the engine's `SDInfoBar` would draw for
//! the ship or station the pointer is over in the 3D view (its name or class, shields,
//! hull, crew and special energy), drawn where the engine draws each part, in the colour
//! the engine gives it, with the engine's icons, and in our font at the engine's size.
//! Armada II draws the strip inside the info panel, which ours (`Info=1`) already
//! replaces; Fleet Operations: Roots draws it at the top centre of the screen.
//!
//! Nothing here is laid out by us: each part's rectangle, justification and colour are
//! the engine's answer for this frame (`engine::hover`), so the line follows Roots'
//! layout files at every aspect.
use crate::engine::api::{self, hover, sprite};
use crate::gfx::{fade, Color, Weight, R};
use crate::widgets::Ui;

/// Roots' figures and capitals stand 10 pixels tall in a text rectangle 19 canvas units
/// (15.2 pixels) tall at 1280x960, so a figure is 0.66 of the rectangle's height on screen
/// (decision 462); our font's figures are 0.714 of an em.
const FIGURE_OF_RECT: f32 = 0.66;
const FIGURE_EM: f32 = 0.714;

fn rgb(c: [f32; 3], a: f32) -> Color {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    (b(a) << 24) | (b(c[0]) << 16) | (b(c[1]) << 8) | b(c[2])
}

pub fn run(ui: &mut Ui) {
    let Some(r) = hover::take() else { return };
    ui.overlay(move |ui| draw(ui, &r));
}

/// A rectangle of the engine's (left, top, right, bottom, inclusive, on the HUD's canvas)
/// on the screen: the engine scales the canvas per axis.
fn screen(ui: &Ui, r: [i32; 4]) -> R {
    let (sx, sy) = (ui.w / api::CANVAS.0, ui.h / api::CANVAS.1);
    R::new(r[0] as f32 * sx, r[1] as f32 * sy, (r[2] - r[0] + 1) as f32 * sx, (r[3] - r[1] + 1) as f32 * sy)
}

fn draw(ui: &mut Ui, r: &hover::Readout) {
    let shadow = fade(0xff000000, 0.8);
    for p in &r.parts {
        if p.icon != 0 {
            // Square, as the icon's art is, in the middle of its rectangle (the canvas is
            // scaled per axis, so on a wide screen the engine's icon is stretched).
            let ir = screen(ui, p.icon_rect);
            let side = ir.w.min(ir.h);
            let sq = R::new(libm::roundf(ir.x + (ir.w - side) * 0.5), libm::roundf(ir.y + (ir.h - side) * 0.5), side, side);
            icon(ui, p.icon, sq, rgb(p.icon_rgb, 1.0));
        }
        let text = p.text.trim();
        if text.is_empty() {
            continue;
        }
        let tr = screen(ui, p.rect);
        let px = (tr.h * FIGURE_OF_RECT / FIGURE_EM).max(8.0);
        let font = crate::text::font_bold();
        let (tw, th) = crate::text::measure_with(font, text, px);
        // eJustify, as the strip's texts use it: bit 0 right, bit 1 centred, else left;
        // bit 3 centred in height (decision 462).
        let x = if p.justify & 1 != 0 {
            tr.right() - tw
        } else if p.justify & 2 != 0 {
            tr.x + (tr.w - tw) * 0.5
        } else {
            tr.x
        };
        let y = if p.justify & 8 != 0 { tr.y + (tr.h - th) * 0.5 } else { tr.y };
        let (x, y) = (libm::roundf(x), libm::roundf(y));
        ui.draw.text_w(text, px, x + 1.0, y + 1.0, shadow, Weight::Bold);
        ui.draw.text_w(text, px, x, y, rgb(p.rgb, 1.0), Weight::Bold);
    }
}

/// The engine's icon in its rectangle, tinted as the engine tints it.
fn icon(ui: &mut Ui, sp: usize, r: R, tint: Color) {
    let Some((tex, uv)) = sprite::art(sp) else { return };
    ui.draw.image_uv(tex, r, uv, tint);
}
