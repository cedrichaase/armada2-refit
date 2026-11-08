//! Roots' Manual IP form (`TManualIPForm`) as Armada II's small dialogs (`kit::form`): a
//! window over a dimmed screen with the address prompt, the address as a field (Roots'
//! own text field has the keyboard; Enter is its Join) and the two buttons at the foot.
//! Roots paints this form's title and prompt into its picture, so ours are words of ours.
//! Reached through *Internet - Online* (`multiplayer.rs`), it says so: a join code or an
//! address joins, a blank field hosts (online/README.md, "On Fleet Operations: Roots").
use super::kit;
use crate::engine::api::forms::Form;
use crate::gfx::R;
use crate::widgets::{Align, Anchor, Ui};

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(addr), Some(join), Some(cancel)) = (kit::shown(f, "editHostaddress"), kit::shown(f, "buttonJoinHost"), kit::shown(f, "buttonNo")) else {
        return false;
    };
    let t = ui.theme;
    let full = ui.screen();
    ui.draw.rect(full, t.scrim);
    let w = ui.u(640.0).min(full.w - ui.u(32.0));
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + ui.u(t.small_size + 8.0) + ui.u(t.row_h + 4.0) + ui.u(t.pad * 2.0) + ui.u(t.button_h + t.gap);
    let r = Anchor::Centre.place(full, w, h, 0.0);
    let online = super::multiplayer::online::chosen();
    let inner = ui.window(r, if online { "INTERNET - ONLINE" } else { "MANUAL IP" });
    let pr = R::new(inner.x, inner.y, inner.w, ui.u(t.small_size + 8.0));
    ui.text_in(pr, if online { "Join code or host address (blank to host)" } else { "Host address" }, t.small_size, Align::Left, t.text_dim, crate::gfx::Weight::Regular);
    let er = R::new(inner.x, pr.bottom(), inner.w, ui.u(t.row_h + 4.0));
    kit::edit(ui, f, addr, er);
    let (bh, bw, gap) = (ui.u(t.button_h), ui.u(180.0), ui.u(t.gap));
    let y = inner.bottom() - bh;
    kit::button(ui, f, join, R::new(inner.right() - 2.0 * bw - gap, y, bw, bh));
    kit::button(ui, f, cancel, R::new(inner.right() - bw, y, bw, bh));
    true
}
