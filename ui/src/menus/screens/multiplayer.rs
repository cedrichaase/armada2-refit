//! The multiplayer screens that are a column of choices.
//!
//! **Multiplayer Connection** (`do_multiplayerConnection`): the player's name (the
//! stock edit, as our field) and one button per connection type, in the stock order,
//! labelled as the game labels them: *Internet - GameSpy*, *Internet - Manual IP*,
//! *Local Area Network (TCP/IP)*, and, with `Online.asi`, *Internet - Online* in the IPX
//! button's place (online/README.md, "The menu entry": the stock button relabelled, so
//! it is in the model like the others, and a click on it is the stock click, which
//! `Online.asi` turns into the online flow). Then Previous Menu.
use super::setup_kit as kit;
use crate::gfx::{Weight, R};
use crate::menus::model::Control;
use crate::menus::Screen;
use crate::widgets::{theme, Align, Anchor, Ui};
use alloc::vec::Vec;

pub fn connection(ui: &mut Ui, s: &Screen) -> bool {
    let edits = kit::visible(s, kit::is_edit);
    let mut buttons: Vec<&Control> = kit::visible(s, |c| kit::is_button(c) && !c.text.is_empty());
    if edits.is_empty() || buttons.len() < 2 {
        return false;
    }
    buttons.sort_by_key(|c| c.rect[1]);
    // The connection types are the wide buttons; the rest (Previous Menu) go along the
    // bottom.
    let (kinds, nav): (Vec<&Control>, Vec<&Control>) = buttons.iter().partition(|c| c.rect[2] >= 300);
    let labels: Vec<&Control> = kit::visible(s, |c| kit::is_label(c) && !c.text.trim().is_empty());
    let title = labels.iter().find(|c| c.rect[2] >= 400).map_or("Multiplayer", |c| c.text.trim());
    let prompt = labels.iter().find(|c| c.rect[2] < 400).map_or("Name", |c| c.text.trim());
    kit::backdrop(ui);
    let t = ui.theme;
    let screen = ui.screen();
    let bh = ui.u(t.button_h + 12.0);
    let gap = ui.u(t.gap * 1.5);
    let w = ui.u(720.0).min(screen.w - ui.u(32.0));
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + ui.u(t.small_size + 8.0) + ui.u(t.row_h + 8.0) + ui.u(t.pad * 2.0) + kinds.len() as f32 * (bh + gap) + ui.u(t.pad) + ui.u(t.button_h + t.pad);
    let r = Anchor::Centre.place(screen, w, h.min(screen.h - ui.u(32.0)), 0.0);
    let inner = ui.window(r, title);
    let mut y = inner.y;
    let pr = R::new(inner.x, y, inner.w, ui.u(t.small_size + 8.0));
    ui.text_in(pr, prompt, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    y = pr.bottom();
    let er = R::new(inner.x, y, inner.w, ui.u(t.row_h + 8.0));
    kit::edit(ui, s, edits[0], er);
    y = er.bottom() + ui.u(t.pad * 2.0);
    theme::separator(ui, R::new(inner.x, y - ui.u(t.pad), inner.w, ui.u(2.0)));
    for c in kinds.iter() {
        kit::button(ui, s, c, R::new(inner.x, y, inner.w, bh), None);
        y += bh + gap;
    }
    let nb = ui.u(t.button_h);
    for (i, c) in nav.iter().enumerate() {
        let bw = ui.u(240.0);
        kit::button(ui, s, c, R::new(inner.x + i as f32 * (bw + gap), inner.bottom() - nb, bw, nb), None);
    }
    true
}
