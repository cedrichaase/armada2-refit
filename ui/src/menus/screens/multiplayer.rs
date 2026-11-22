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
    let (er, kind_rs, nav_rs) = connection_layout(ui, title, prompt, kinds.len(), nav.len());
    kit::edit(ui, s, edits[0], er);
    for (c, r) in kinds.iter().zip(kind_rs) {
        kit::button(ui, s, c, r, None);
    }
    for (c, r) in nav.iter().zip(nav_rs) {
        kit::button(ui, s, c, r, None);
    }
    true
}

/// The window of a connection screen: its title, the caption over the name field, the
/// rule under it, and where each part stands: the name field, one wide button for each
/// of `kinds` connection types, and `nav` buttons along the bottom. Shared with Roots'
/// Multiplayer Connection (`menus::roots`).
pub fn connection_layout(ui: &mut Ui, title: &str, prompt: &str, kinds: usize, nav: usize) -> (R, Vec<R>, Vec<R>) {
    let t = ui.theme;
    let screen = ui.screen();
    let bh = ui.u(t.button_h + 12.0);
    let gap = ui.u(t.gap * 1.5);
    let w = ui.u(720.0).min(screen.w - ui.u(32.0));
    let h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + ui.u(t.small_size + 8.0) + ui.u(t.row_h + 8.0) + ui.u(t.pad * 2.0) + kinds as f32 * (bh + gap) + ui.u(t.pad) + ui.u(t.button_h + t.pad);
    let r = Anchor::Centre.place(screen, w, h.min(screen.h - ui.u(32.0)), 0.0);
    let inner = ui.window(r, title);
    let mut y = inner.y;
    let pr = R::new(inner.x, y, inner.w, ui.u(t.small_size + 8.0));
    ui.text_in(pr, prompt, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    y = pr.bottom();
    let er = R::new(inner.x, y, inner.w, ui.u(t.row_h + 8.0));
    y = er.bottom() + ui.u(t.pad * 2.0);
    theme::separator(ui, R::new(inner.x, y - ui.u(t.pad), inner.w, ui.u(2.0)));
    let mut kind_rs = Vec::new();
    for _ in 0..kinds {
        kind_rs.push(R::new(inner.x, y, inner.w, bh));
        y += bh + gap;
    }
    let nb = ui.u(t.button_h);
    let mut nav_rs = Vec::new();
    for i in 0..nav {
        let bw = ui.u(240.0);
        nav_rs.push(R::new(inner.x + i as f32 * (bw + gap), inner.bottom() - nb, bw, nb));
    }
    (er, kind_rs, nav_rs)
}
