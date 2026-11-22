//! Roots' new-map name form (`TMapEditCreateForm`, opened by the map editor list's New Map)
//! as Armada II's small dialogs, like Manual IP: a window over a dimmed screen with the
//! file name as a field (Roots' own text field has the keyboard; Enter is its Create, which
//! stays disabled until a name is typed) and the two buttons at the foot. Roots paints this
//! form's title and prompt into its picture, so ours are words of ours.
use super::kit;
use crate::engine::api::forms::Form;
use crate::gfx::R;
use crate::widgets::{Align, Anchor, Ui};

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(name), Some(create), Some(cancel)) = (kit::shown(f, "editFilename"), kit::shown(f, "buttonJoinHost"), kit::shown(f, "buttonNo")) else {
        return false;
    };
    let t = ui.theme;
    let full = ui.screen();
    ui.draw.rect(full, t.scrim);
    let w = ui.u(640.0).min(full.w - ui.u(32.0));
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + ui.u(t.small_size + 8.0) + ui.u(t.row_h + 4.0) + ui.u(t.pad * 2.0) + ui.u(t.button_h + t.gap);
    let r = Anchor::Centre.place(full, w, h, 0.0);
    let inner = ui.window(r, "NEW MAP");
    let pr = R::new(inner.x, inner.y, inner.w, ui.u(t.small_size + 8.0));
    ui.text_in(pr, "File name of the new map", t.small_size, Align::Left, t.text_dim, crate::gfx::Weight::Regular);
    let er = R::new(inner.x, pr.bottom(), inner.w, ui.u(t.row_h + 4.0));
    kit::edit(ui, f, name, er);
    let (bh, bw, gap) = (ui.u(t.button_h), ui.u(180.0), ui.u(t.gap));
    let y = inner.bottom() - bh;
    let rc = R::new(inner.right() - 2.0 * bw - gap, y, bw, bh);
    // Roots' Create stays disabled until a name is typed (its action is updated when the
    // field changes), so ours is dimmed the same way.
    if create.enabled && !name.text.is_empty() {
        kit::button(ui, f, create, rc);
    } else {
        kit::dimmed(ui, rc, kit::caption(create));
    }
    kit::button(ui, f, cancel, R::new(inner.right() - bw, y, bw, bh));
    true
}
