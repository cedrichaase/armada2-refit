//! Roots' map editor list (`TMapEditForm`, the shell's mode 300 behind the main menu's Map
//! Edit button) as Armada II's Map Select: a window with the list of maps (the rows read
//! from the grid's memory, as Map Select's), the selected map's preview and name beside it,
//! Roots' New Map, Delete and Edit buttons at the foot's right and Previous Menu at its
//! left. The buttons' actions are Roots': Edit opens the map in the editor and closes the
//! form, Delete asks Roots' Yes/No and removes the map's files, New Map opens the name
//! form (`mapeditcreate`). A double click on a row is Edit, as it is on Roots' grid.
use super::{kit, mapselect};
use crate::engine::api::forms::Form;
use crate::gfx::{Weight, R};
use crate::menus::screens::form as sform;
use crate::widgets::{Align, Ui};

/// The grid's columns as Roots gives its map list, with each one's share of the width.
const COLUMNS: [(&str, f32); 6] =
    [("Name", 0.25), ("Players", 0.08), ("Filename", 0.22), ("Size", 0.15), ("Type", 0.14), ("Author", 0.16)];

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let Some(g) = kit::shown(f, "mapSelectGrid") else { return false };
    let Some(grid) = g.grid.as_ref() else { return false };
    let t = ui.theme;
    let list_h = 610.0;
    let (body, foot) = sform::frame(ui, 1180.0, list_h - 60.0, "MAP EDITOR");
    let top = body.y + ui.u(kit::GAP * 0.5);
    let side = ui.u(260.0);
    let list_r = R::new(body.x, top, body.w - side - ui.u(24.0), ui.u(list_h));
    let edit = kit::shown(f, "buttonEditMap");
    let _ = mapselect::list(ui, f, g, grid, list_r, &COLUMNS, edit, false);

    // The selected map: its preview and the name Roots shows under it.
    let x = list_r.right() + ui.u(24.0);
    let pv = ui.u(240.0);
    if let Some(c) = kit::shown(f, "previewImage") {
        kit::crop(ui, f, c, R::new(x, top, pv, pv));
    }
    let name = kit::text(f, "labelSelectedMap");
    ui.text_in(R::new(x, top + pv + ui.u(12.0), side, ui.u(30.0)), name, t.text_size * 1.1, Align::Left, t.text, Weight::Bold);

    // The buttons stand in the foot, right of Previous Menu.
    let mut bx = foot.right();
    for name in ["buttonEditMap", "buttonDeleteMap", "btnCreateNewMap"] {
        let Some(c) = kit::shown(f, name) else { continue };
        let w = ui.u(200.0);
        bx -= w;
        let r = R::new(bx, foot.bottom() - ui.u(44.0), w, ui.u(44.0));
        if c.enabled {
            kit::button(ui, f, c, r);
        } else {
            kit::dimmed(ui, r, kit::caption(c));
        }
        bx -= ui.u(12.0);
    }
    kit::back(ui, f, foot, "buttonBack");
    true
}
