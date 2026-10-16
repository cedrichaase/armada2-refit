//! The Technology Tree (`do_TechTreeWindow`), opened from the in-mission menu: a window
//! over the mission with the tree drawn by us, and OK.
//!
//! The stock dialog is one disabled multi-line `Edit` in Courier holding the tree as
//! ASCII art (`Starbase---*Pulse Cannon`, a `|`, `-` and `\` for the branches, `*` before a
//! buildable name, `(*Name)` for a requirement already shown above), scrolled by four
//! small buttons. The model reads the edit's text whole, so the tree is drawn from it
//! and the edit is never touched: the branch characters become lines on the text's
//! character grid, each name is set in the theme's font at its column, and our scroll
//! bar and the wheel scroll it. The text size is chosen so the widest line fits the
//! window: the tree is some 120 columns wide.
use crate::gfx::{fade, Weight, R};
use crate::menus::model::{Class, Control, What};
use crate::menus::screens::kit;
use crate::menus::Screen;
use crate::widgets::{Align, Ui};
use alloc::vec::Vec;

/// A column's width in em: a name's characters are narrower than this on average, so a
/// name never runs into the branch that follows it.
const PITCH: f32 = 0.56;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let Some(edit) = s.model.controls.iter().find(|c| matches!(c.what, What::Window(Class::Edit))) else { return false };
    let buttons = kit::buttons(s);
    let Some(ok) = buttons.iter().copied().find(|c| !c.text.is_empty()) else { return false };
    let title = kit::labels(s).into_iter().find(|c| !c.text.is_empty()).map_or("Technology Tree", |c| c.text.as_str());
    let t = ui.theme;

    kit::scrim(ui);
    let win = kit::centred(ui, 1500.0, 940.0);
    let inner = ui.window(win, title);
    let foot_h = ui.u(t.button_h);
    let gap = ui.u(t.gap);
    let view = R::new(inner.x, inner.y, inner.w, inner.h - foot_h - gap);
    ui.draw.rect_round(view, ui.u(6.0), fade(t.track, 0.9));

    draw_tree(ui, view.inset(ui.u(8.0)), edit);

    kit::button(ui, s, R::new(inner.x + (inner.w - ui.u(260.0)) * 0.5, inner.bottom() - foot_h, ui.u(260.0), foot_h), ok, &ok.text);
    true
}

fn draw_tree(ui: &mut Ui, r: R, edit: &Control) {
    let t = ui.theme;
    let lines: Vec<&str> = edit.text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1);
    // The size that fits the widest line in the view (less the scroll bar), at most the
    // body size and at least a legible one.
    let avail = r.w - ui.u(24.0);
    let size = (avail / (cols as f32 * PITCH * ui.scale)).clamp(12.0, t.text_size);
    let px = ui.u(size);
    let pitch = px * PITCH;
    let line_h = px * 1.3;
    let content_h = lines.len() as f32 * line_h + ui.u(8.0);
    let (top, bottom) = (r.y, r.bottom());
    let (dim, text, accent) = (t.text_dim, t.text, t.accent);
    let branch = fade(t.text_dim, 0.8);
    let lw = ui.u(1.5).max(1.0);
    ui.scroll_area(r, "tree", content_h, |ui, body| {
        for (n, line) in lines.iter().enumerate() {
            let y = body.y + n as f32 * line_h;
            if y + line_h < top || y > bottom {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            let (mut i, mut paren) = (0usize, false);
            while i < chars.len() {
                let x = body.x + i as f32 * pitch;
                let (cx, cy) = (x + pitch * 0.5, y + line_h * 0.5);
                match chars[i] {
                    '|' => ui.draw.line((cx, y), (cx, y + line_h), lw, branch),
                    '-' => ui.draw.line((x, cy), (x + pitch, cy), lw, branch),
                    '\\' => {
                        ui.draw.line((cx, y), (cx, cy), lw, branch);
                        ui.draw.line((cx, cy), (x + pitch, cy), lw, branch);
                    }
                    '*' => {
                        let d = px * 0.22;
                        ui.draw.rect_round(R::new(cx - d * 0.5, cy - d * 0.5, d, d), d * 0.5, if paren { dim } else { accent });
                    }
                    '(' => {
                        paren = true;
                        ui.draw.line((cx + pitch * 0.2, y + line_h * 0.2), (cx - pitch * 0.1, cy), lw, dim);
                        ui.draw.line((cx - pitch * 0.1, cy), (cx + pitch * 0.2, y + line_h * 0.8), lw, dim);
                    }
                    ')' => {
                        paren = false;
                        ui.draw.line((cx - pitch * 0.2, y + line_h * 0.2), (cx + pitch * 0.1, cy), lw, dim);
                        ui.draw.line((cx + pitch * 0.1, cy), (cx - pitch * 0.2, y + line_h * 0.8), lw, dim);
                    }
                    ' ' => {}
                    _ => {
                        // A name: up to the next branch character, set at its column.
                        let start = i;
                        while i < chars.len() && !matches!(chars[i], '|' | '-' | '\\' | '(' | ')' | '*') {
                            i += 1;
                        }
                        let word: alloc::string::String = chars[start..i].iter().collect();
                        let word = word.trim_end();
                        let col = if paren { dim } else if start == 0 { accent } else { text };
                        let w = if start == 0 { Weight::Bold } else { Weight::Regular };
                        ui.text_in(R::new(x, y, body.w, line_h), word, size, Align::Left, col, w);
                        continue;
                    }
                }
                i += 1;
            }
        }
    });
}
