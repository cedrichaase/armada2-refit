//! Roots' Mods screen (`TModSettingsForm`): the list of installed mods (a grid Roots draws
//! itself, its rows read from the grid's memory as Map Select's and chosen by a click at the
//! row, so the selection is Roots' own), the selected mod's screenshot, title
//! and description, and the Launch Mod, Create Shortcut and Visit Website buttons as ours,
//! enabled as Roots enables them. Previous Menu is Roots' own.
use super::{kit, mapselect};
use crate::engine::api::forms::Form;
use crate::gfx::{Weight, R};
use crate::menus::screens::form as sform;
use crate::widgets::{Align, Ui};

/// The list's seven columns as Roots gives them (the fourth is a picture), by share of the
/// width.
const COLUMNS: [(&str, f32); 7] =
    [("Name", 0.28), ("Author", 0.20), ("Version", 0.10), ("", 0.02), ("Requires", 0.24), ("", 0.08), ("", 0.08)];

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let Some(grid) = kit::shown(f, "grdModSelect") else { return false };
    let (grid_h, info_h) = (330.0, 200.0);
    let (body, foot) = sform::frame(ui, 1180.0, grid_h + info_h + kit::GAP, "MODS");
    let t = ui.theme;
    let grid_r = R::new(body.x, body.y + ui.u(6.0), body.w, ui.u(grid_h));
    // The rows are read from the grid's memory (Roots' grid paints none of them while the
    // form is off screen); without them the form's picture is what there is to show.
    match grid.grid.as_ref() {
        Some(g) => {
            mapselect::list(ui, f, grid, g, grid_r, &COLUMNS, None, true);
        }
        None => kit::crop(ui, f, grid, grid_r),
    }

    let y = grid_r.bottom() + ui.u(kit::GAP);
    let shot = R::new(body.x, y, ui.u(260.0), ui.u(info_h - 20.0));
    if let Some(c) = kit::shown(f, "imgModScreenshot") {
        kit::crop(ui, f, c, shot);
    }
    let tx = shot.right() + ui.u(24.0);
    let tw = body.right() - tx;
    let title = kit::text(f, "lblModTitle");
    ui.text_in(R::new(tx, y, tw, ui.u(30.0)), title, t.text_size * 1.15, Align::Left, t.text, Weight::Bold);
    let px = ui.u(t.small_size * 1.1);
    ui.draw.text_wrapped(kit::text(f, "lblDescription"), px, tw, tx, y + ui.u(36.0), t.text_dim, Weight::Regular);

    // The buttons stand in the foot, right of Previous Menu.
    let mut x = foot.right();
    for name in ["btnDesktopShortcut2", "btnDesktopShortcut", "btnLaunchNow"] {
        let Some(c) = kit::shown(f, name) else { continue };
        let w = ui.u(200.0);
        x -= w;
        let r = R::new(x, foot.bottom() - ui.u(44.0), w, ui.u(44.0));
        if c.enabled {
            kit::button(ui, f, c, r);
        } else {
            crate::menus::generic::shell_button_look(ui, r, kit::caption(c), false, false, false);
        }
        x -= ui.u(12.0);
    }
    kit::back(ui, f, foot, "buttonBack");
    true
}
