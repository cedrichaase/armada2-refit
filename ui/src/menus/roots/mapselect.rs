//! Roots' Map Select (`TMapSelectForm`) as Armada II's Map Select: a window with the
//! players filter over the list of maps, the preview beside it, and Roots' two buttons at
//! the foot.
//!
//! Roots' grid paints its rows through its own window's device context, which is clipped
//! to nothing while the form is off screen, so the rows are not in the form's picture:
//! they are read from the grid's cells (`engine::forms`, read only) and listed by our own
//! list. A click on a row is a click on the grid's row at its place (the grid's own
//! selection and Roots' preview handler run); a row beyond the ones the grid shows is
//! reached by the arrow keys sent to the grid's window. The preview is the form's picture
//! of its image, a crop.
use super::kit;
use crate::engine::api::forms::{self, Action, Control, Form, Grid};
use crate::gfx::{Weight, R};
use crate::menus::screens::form as sform;
use crate::widgets::{Align, ListEvent, Ui};
use alloc::vec::Vec;

/// A row clicked and not yet the grid's selection: (form, row, frames left).
static mut PENDING: (usize, usize, u32) = (0, 0, 0);

/// The columns' captions when the grid has the four Roots gives its map list, with each
/// one's share of the list's width.
const COLUMNS: [(&str, f32); 4] = [("Name", 0.42), ("Players", 0.14), ("Filename", 0.29), ("Size", 0.15)];

/// The grid's row `i` selected as a player would: a click at the row when the grid shows
/// it, the arrow keys from the selected row otherwise.
fn choose(f: &Form, g: &Control, grid: &Grid, i: usize, from: i32, keys: bool) {
    let (rh, hh) = (if grid.row_h > 0 { grid.row_h } else { ROW_H }, if grid.head_h > 0 { grid.head_h } else { HEAD_H });
    let shown = ((g.rect[3] - hh) / rh).max(1);
    let at = i as i32 - grid.first;
    // The grid takes the keyboard with the click, so the arrow keys that follow move its
    // selection (Map Select's filter combo has it first, and would take them).
    if g.hwnd != 0 {
        forms::act(Action::Focus(g.hwnd));
    }
    if !keys && (0..shown).contains(&at) {
        let (x, y) = (g.rect[0] + 12, g.rect[1] + hh + at * rh + rh / 2);
        crate::log!("forms: map {} chosen (row {} at {},{})", i, i, x, y);
        forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
        forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, 0, x, y));
    } else if g.hwnd != 0 {
        let (vk, n) = if i as i32 > from { (VK_DOWN, i as i32 - from) } else { (VK_UP, from - i as i32) };
        crate::log!("forms: map {} chosen ({} keys from row {})", i, n, from);
        for _ in 0..n.min(64) {
            forms::act(Action::GridKey(g.hwnd, vk));
        }
    }
}

const VK_UP: u32 = 0x26;
const VK_DOWN: u32 = 0x28;
/// A NextSuite grid's default row and header heights, for a class that does not publish
/// them in a field.
const ROW_H: i32 = 19;
const HEAD_H: i32 = 19;

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(g), Some(accept), Some(cancel)) = (kit::shown(f, "mapSelectGrid"), kit::shown(f, "buttonAccept"), kit::shown(f, "buttonClose")) else {
        return false;
    };
    let Some(grid) = g.grid.as_ref() else { return false };
    let list_h = 520.0;
    let (body, foot) = sform::frame(ui, 1180.0, sform::LABEL_H + sform::FIELD_H + list_h + 2.0 * kit::GAP, "SELECT MAP");

    // The filter, in the settings screens' caption-over-field place.
    let fw = ui.u(360.0);
    if let Some(c) = kit::shown(f, "cbbPlayers") {
        let label = kit::text(f, "labelPlayers");
        kit::caption_over(ui, body.x, body.y, fw, if label.is_empty() { "Players" } else { label });
        kit::combo(ui, f, c, R::new(body.x, body.y + ui.u(sform::LABEL_H), fw, ui.u(sform::FIELD_H)));
    }

    // The list, and the preview beside it.
    let top = body.y + ui.u(sform::LABEL_H + sform::FIELD_H + kit::GAP);
    let pv = ui.u(300.0);
    let list_r = R::new(body.x, top, body.w - pv - ui.u(24.0), ui.u(list_h));
    list(ui, f, g, grid, list_r, &COLUMNS, Some(accept), false);

    // The preview: the form's picture of its image.
    if let Some(c) = kit::shown(f, "previewImage") {
        let side = pv.min(list_r.h);
        kit::crop(ui, f, c, R::new(list_r.right() + ui.u(24.0), top, side, side));
    }

    kit::back(ui, f, foot, "buttonClose");
    let (bw, bh) = (ui.u(220.0), ui.u(44.0));
    kit::button(ui, f, accept, R::new(foot.right() - bw, foot.bottom() - bh, bw, bh));
    let _ = cancel;
    true
}

/// A grid's rows as a list of ours in `list_r`, under a header of the columns' captions
/// (when the grid has as many columns as `cols`): the player's click is the grid's row
/// chosen (by the arrow keys instead of a click when `keys`: a grid of slides, which has
/// no plain rows to click), a double click or Enter presses `activate`. Returns what the
/// list saw (an `Activated` with no `activate` is the caller's to act on).
pub(super) fn list(ui: &mut Ui, f: &Form, g: &Control, grid: &Grid, list_r: R, cols: &[(&str, f32)], activate: Option<&Control>, keys: bool) -> ListEvent {
    let t = ui.theme;
    let row_h = ui.u(t.text_size + 14.0);
    let head_h = ui.u(t.small_size + 12.0);
    let head = R::new(list_r.x, list_r.y, list_r.w, head_h);
    let rows_r = R::new(list_r.x, list_r.y + head_h, list_r.w, list_r.h - head_h);
    let named = grid.rows.first().map_or(cols.len(), |r| r.len()) == cols.len();
    let bar = ui.u(18.0);
    let mut x = head.x + ui.u(10.0);
    if named {
        for (name, share) in cols {
            ui.text_in(R::new(x, head.y, head.w * share, head.h), name, t.small_size, Align::Left, t.text_dim, Weight::Bold);
            x += (head.w - bar - ui.u(10.0)) * share;
        }
    }
    // Our selection follows the grid's, but a click waits for the grid to take it.
    let mut sel: Option<usize> = (grid.selected >= 0 && (grid.selected as usize) < grid.rows.len()).then_some(grid.selected as usize);
    unsafe {
        let p = &mut *core::ptr::addr_of_mut!(PENDING);
        if p.0 == f.hwnd && p.2 > 0 {
            p.2 -= 1;
            if sel == Some(p.1) {
                p.2 = 0;
            } else {
                sel = Some(p.1);
            }
        }
    }
    // A selection the grid made (a key, the filter, the form's own start) brings its row
    // into view in our list.
    static mut SEEN: (usize, i32) = (0, -2);
    unsafe {
        if SEEN != (f.hwnd, grid.selected) {
            SEEN = (f.hwnd, grid.selected);
            if let Some(s) = sel {
                let slot = ui.slot(ui.id("maps"));
                let (top, view) = (s as f32 * row_h, rows_r.h - ui.u(8.0));
                if top < slot[0] {
                    slot[0] = top;
                } else if top + row_h > slot[0] + view {
                    slot[0] = top + row_h - view;
                }
            }
        }
    }
    let rows: Vec<&Vec<alloc::string::String>> = grid.rows.iter().collect();
    let width = rows_r.w - bar - ui.u(10.0);
    let was = sel;
    let ev = ui.list(rows_r, "maps", rows.len(), row_h, &mut sel, |ui, i, rr, _sel, _hot| {
        let mut x = rr.x + ui.u(10.0);
        for (k, cell) in rows[i].iter().enumerate() {
            let share = if named { cols[k.min(cols.len() - 1)].1 } else { 1.0 / rows[i].len().max(1) as f32 };
            // A cell is cut at its column's edge, so a long file name stays out of Size.
            let cell_r = R::new(x, rr.y, width * share, rr.h);
            ui.draw.push_clip(R::new(cell_r.x, cell_r.y, (cell_r.w - ui.u(8.0)).max(0.0), cell_r.h));
            ui.text_in(cell_r, cell, t.text_size, Align::Left, t.text, Weight::Regular);
            ui.draw.pop_clip();
            x += width * share;
        }
    });
    if f.top && g.enabled {
        if let (ListEvent::Changed | ListEvent::Activated, Some(i)) = (ev, sel) {
            if grid.selected != i as i32 {
                choose(f, g, grid, i, was.map_or(grid.selected.max(0), |s| s as i32), keys);
                unsafe { PENDING = (f.hwnd, i, 8) };
            }
            if ev == ListEvent::Activated {
                if let Some(a) = activate {
                    kit::press(f, a);
                }
            }
        }
    }
    ev
}
