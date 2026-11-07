//! Roots' map editor file picker (`TMapSaveForm`, PLAN-FLEETOPS.md, decisions 620 to 629):
//! the form FleetOpsHook shows for the editor's Load Map... and Save Map... in place of the
//! executable's file dialog, as a dialog of ours over the editor: the map files as our list
//! (the rows read from the grid's memory, as Map Select's), the file name as our field over
//! Roots' own text field (Save; Load hides the field and takes the chosen row), and Roots'
//! two buttons at the foot. Every action is Roots': a row chosen is the grid's row clicked
//! (Roots' handler puts its file name into the field), Save or Open and Cancel are the
//! form's own windowed buttons pressed at their place, and a double click on a row is the
//! confirm button, as on Roots' grid. The keyboard stays on Roots' form: Enter is its
//! default button (confirm), Escape its cancel button. Roots asks before Save overwrites a
//! map, through the VCL's message box (`TMessageForm`), drawn by `msgdlg`.
use super::{kit, mapselect};
use crate::engine::api::forms::Form;
use crate::gfx::{Weight, R};
use crate::widgets::{Align, Anchor, ListEvent, Ui};

/// A double click waiting to confirm: (form, frames left).
static mut CONFIRM: (usize, u32) = (0, 0);

/// The grid's two columns as Roots gives them, with each one's share of the list's width.
const COLUMNS: [(&str, f32); 2] = [("Filename", 0.72), ("Location", 0.28)];

/// The title, by what the confirm button says: Roots' one form serves both commands.
fn title(confirm: &str) -> &'static str {
    let c = confirm.trim_start_matches('&').to_ascii_lowercase();
    if c.starts_with("save") {
        "SAVE MAP"
    } else if c.starts_with("load") || c.starts_with("open") {
        "LOAD MAP"
    } else {
        "MAP FILE"
    }
}

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(g), Some(confirm), Some(close)) = (kit::shown(f, "grdMaps"), kit::shown(f, "btnConfirm"), kit::shown(f, "btnClose")) else {
        return false;
    };
    let Some(grid) = g.grid.as_ref() else { return false };
    // Save shows the file name field; Load (its confirm reads "Open") hides it and takes
    // the grid's selected row.
    let name = kit::shown(f, "edtSaveName");
    let t = ui.theme;
    let full = ui.screen();
    ui.draw.rect(full, t.scrim);
    let w = ui.u(900.0).min(full.w - ui.u(32.0));
    let list_h = ui.u(440.0).min(full.h * 0.55);
    let field_h = ui.u(t.small_size + 8.0) + ui.u(t.row_h + 4.0);
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + list_h + ui.u(t.gap) + field_h + ui.u(t.pad * 2.0) + ui.u(t.button_h + t.gap);
    let r = Anchor::Centre.place(full, w, h.min(full.h - ui.u(32.0)), 0.0);
    let inner = ui.window(r, title(kit::caption(confirm)));

    // The map files: a click is the grid's row chosen, a double click the confirm.
    let list_r = R::new(inner.x, inner.y, inner.w, list_h);
    let ev = mapselect::list(ui, f, g, grid, list_r, &COLUMNS, None, false);

    // The file name: Roots' own field under ours (Save), or the chosen row (Load).
    let cr = R::new(inner.x, list_r.bottom() + ui.u(t.gap), inner.w, ui.u(t.small_size + 8.0));
    let label = kit::text(f, "lbl1").trim_end_matches(':');
    ui.text_in(cr, if label.is_empty() { "Filename" } else { label }, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    let fr = R::new(inner.x, cr.bottom(), inner.w, ui.u(t.row_h + 4.0));
    let chosen = usize::try_from(grid.selected).ok().and_then(|i| grid.rows.get(i)).and_then(|row| row.first());
    let ready = match name {
        Some(name) => {
            kit::edit(ui, f, name, fr);
            !name.text.trim().is_empty()
        }
        None => {
            let pad = ui.u(t.pad * 0.7);
            ui.text_in(R::new(fr.x + pad, fr.y, fr.w - 2.0 * pad, fr.h), chosen.map_or("", |c| c.as_str()), t.text_size, Align::Left, t.text, Weight::Regular);
            chosen.is_some()
        }
    } && confirm.enabled;

    // Roots' confirm (dimmed while there is no name, as Roots' action is) and Cancel. A
    // double click on a row of our list confirms, as on Roots' grid. Enter and Escape need
    // nothing of ours: the keyboard stays on Roots' form, whose confirm is its default button
    // and Cancel its cancel button.
    let (bh, bw, gap) = (ui.u(t.button_h), ui.u(180.0), ui.u(t.gap));
    let y = inner.bottom() - bh;
    let rc = R::new(inner.right() - 2.0 * bw - gap, y, bw, bh);
    // The double click's first click chose the row, and the grid takes it a few frames
    // later (Load's confirm waits for a chosen row): the confirm waits for it as long.
    unsafe {
        let p = &mut *core::ptr::addr_of_mut!(CONFIRM);
        if f.top && ev == ListEvent::Activated {
            *p = (f.hwnd, 30);
        }
        if p.0 == f.hwnd && p.1 > 0 {
            p.1 -= 1;
            if ready && f.top {
                p.1 = 0;
                kit::press(f, confirm);
            }
        }
    }
    if ready {
        kit::button(ui, f, confirm, rc);
    } else {
        kit::dimmed(ui, rc, kit::caption(confirm));
    }
    kit::button(ui, f, close, R::new(inner.right() - bw, y, bw, bh));
    true
}
