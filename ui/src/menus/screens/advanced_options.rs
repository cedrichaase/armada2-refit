//! Game Setup's Advanced panel (`LaunchAdvancedOptionsWindow`), laid out our way: one
//! window of two columns, as the stock panel has them.
//!
//! - The **sliders** (Weapon Strength, Game Speed, AI Build Time, ...) are the stock
//!   `ShellScrollBar`s, which the model does not read: each is drawn from the mirror
//!   under its three labels (the name, and the two ends), and the pointer over it goes
//!   to the stock dialog as it would to the stock bar (a press, the drag, the release).
//!   The bar sits 16 design pixels under its labels, from the left end's label to the
//!   right end's.
//! - The **toggles** are our check boxes with their own labels (which say the state),
//!   the other settings buttons ours, labelled as the game labels them.
//! - **Close** and **Default** along the bottom.
use super::kit;
use crate::engine::shellui::Kind;
use crate::gfx::{fade, Weight, R};
use crate::menus::model::{Control, What};
use crate::menus::{generic, Screen};
use crate::widgets::{Align, Anchor, Ui};
use alloc::vec::Vec;

/// A slider: its name, its two ends, and the stock bar's design rectangle.
struct Slider<'a> {
    name: &'a str,
    lo: &'a str,
    hi: &'a str,
    bar: [i32; 4],
    x: i32,
}

/// The labels grouped by their top into sliders: a centred one names it, a left- and a
/// right-aligned one are its ends.
fn sliders<'a>(s: &'a Screen) -> Vec<Slider<'a>> {
    let labels: Vec<&Control> = kit::visible(s, |c| kit::is_label(c) && !c.text.trim().is_empty());
    let mut out: Vec<Slider> = Vec::new();
    for name in labels.iter().filter(|c| c.align == 0) {
        let near = |c: &&&Control| c.rect[1] == name.rect[1] && (c.rect[0] - name.rect[0]).abs() <= 20;
        let lo = labels.iter().filter(near).find(|c| c.align < 0);
        let hi = labels.iter().filter(near).find(|c| c.align > 0);
        let (Some(lo), Some(hi)) = (lo, hi) else { continue };
        let x0 = lo.rect[0];
        let x1 = hi.rect[0] + hi.rect[2];
        out.push(Slider { name: name.text.trim(), lo: lo.text.trim(), hi: hi.text.trim(), bar: [x0 - 2, name.rect[1] + 15, x1 - x0 + 4, 16], x: name.rect[0] });
    }
    out
}

/// Whether the pointer is dragging a stock slider (it keeps going to the dialog until
/// the button comes up, wherever the pointer is).
static mut DRAG: bool = false;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let buttons: Vec<&Control> = kit::visible(s, |c| kit::is_button(c) && !c.text.is_empty());
    let sl = sliders(s);
    if buttons.is_empty() || sl.is_empty() {
        return false;
    }
    let t = ui.theme;
    let screen = ui.screen();
    ui.draw.rect(screen, t.scrim);
    // The bottom row: the buttons below every setting.
    let low = buttons.iter().map(|c| c.rect[1]).max().unwrap_or(0);
    let (mut nav, set): (Vec<&Control>, Vec<&Control>) = buttons.iter().partition(|c| c.rect[1] >= low - 4);
    nav.sort_by_key(|c| c.rect[0]);
    // The two columns split at the stock panel's middle.
    let mid = s.model.rect[0] + s.model.rect[2] / 2;
    let w = ui.u(1100.0).min(screen.w - ui.u(32.0));
    let h = ui.u(800.0).min(screen.h - ui.u(32.0));
    let r = Anchor::Centre.place(screen, w, h, 0.0);
    let inner = ui.window(r, "Advanced Options");
    let bh = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let colw = (inner.w - ui.u(t.pad * 3.0)) * 0.5;
    let body_h = inner.h - bh - ui.u(t.pad);
    let mut over_bar = false;
    for col in 0..2 {
        let cx = inner.x + col as f32 * (colw + ui.u(t.pad * 3.0));
        let in_col = |x: i32| (x < mid) == (col == 0);
        // Sliders and settings in their stock order, top to bottom.
        let mut items: Vec<(i32, Option<&Slider>, Option<&Control>)> = Vec::new();
        for sd in sl.iter().filter(|sd| in_col(sd.x + 100)) {
            items.push((sd.bar[1] - 15, Some(sd), None));
        }
        for c in set.iter().filter(|c| in_col(c.rect[0] + c.rect[2] / 2)) {
            items.push((c.rect[1], None, Some(c)));
        }
        items.sort_by_key(|i| i.0);
        let n_sl = items.iter().filter(|i| i.1.is_some()).count() as f32;
        let n_set = items.len() as f32 - n_sl;
        let slider_h = ui.u(t.small_size + 34.0);
        let row = ((body_h - n_sl * (slider_h + gap)) / n_set.max(1.0)).min(bh + gap);
        let mut y = inner.y;
        for (_, sd, c) in items {
            if let Some(sd) = sd {
                let lr = R::new(cx, y, colw, ui.u(t.small_size + 6.0));
                ui.text_in(lr, sd.lo, t.small_size, Align::Left, t.text_dim, Weight::Regular);
                ui.text_in(lr, sd.name, t.small_size, Align::Centre, t.text, Weight::Bold);
                ui.text_in(lr, sd.hi, t.small_size, Align::Right, t.text_dim, Weight::Regular);
                let br = R::new(cx, lr.bottom() + ui.u(4.0), colw, slider_h - lr.h - ui.u(4.0));
                ui.draw.rect_round(br.inset(-ui.u(2.0)), ui.u(4.0), fade(t.panel_edge, 0.6));
                s.mirror_part(ui, sd.bar, br, 0xffffffff);
                if ui.input.mouse.is_some_and(|p| br.contains(p)) {
                    over_bar = true;
                    // Our pointer over the stock bar is the stock bar's: map it into
                    // the design place of the bar as `forward_input` does for the
                    // whole dialog.
                    unsafe { BAR = (br, sd.bar) };
                }
                y += slider_h + gap;
            } else if let Some(c) = c {
                let br = R::new(cx, y, colw, row - gap);
                if matches!(c.what, What::Shell(Kind::Toggle)) {
                    kit::check(ui, s, c, br);
                } else {
                    kit::cycle(ui, s, c, br);
                }
                y += row;
            }
        }
    }
    // Close (the first) at the left, the rest at the right.
    let by = inner.bottom() - bh;
    let bw = ui.u(220.0);
    for (i, c) in nav.iter().enumerate() {
        let x = if i == 0 { inner.x } else { inner.right() - (nav.len() - i) as f32 * (bw + gap) + gap };
        kit::button(ui, s, c, R::new(x, by, bw, bh), None);
    }
    // The pointer over a slider, or dragging one, goes to the stock dialog in the
    // bar's design place.
    let drag = unsafe { DRAG };
    if s.top && (over_bar || drag) {
        let (br, bar) = unsafe { BAR };
        let mut ui2 = ui.input;
        if let Some(p) = ui2.mouse {
            // Into the bar's design rectangle, then to where `forward_input` expects
            // the pointer: the screen place of that design point.
            let fx = ((p.0 - br.x) / br.w.max(1.0)).clamp(0.0, 1.0);
            let fy = ((p.1 - br.y) / br.h.max(1.0)).clamp(0.0, 1.0);
            let dx = bar[0] as f32 + fx * bar[2] as f32;
            let dy = bar[1] as f32 + fy * bar[3] as f32;
            ui2.mouse = Some((s.fit.x + dx * s.fit.s, s.fit.y + dy * s.fit.s));
        }
        let saved = ui.input;
        ui.input = ui2;
        generic::forward_input(ui, s);
        ui.input = saved;
        ui.claim(screen);
        unsafe {
            if ui.input.pressed[0] && over_bar {
                DRAG = true;
            }
            if !ui.input.down[0] {
                DRAG = false;
            }
        }
    }
    true
}

/// The slider under the pointer: our rectangle and the stock bar's design one.
static mut BAR: (R, [i32; 4]) = (R { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, [0; 4]);
