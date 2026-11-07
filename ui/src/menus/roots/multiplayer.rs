//! Roots' Multiplayer Connection (`TMultiplayerConnectionForm`) as Armada II's
//! (`screens::multiplayer`): the player's name as a field, one wide button for each way to
//! connect (Roots' captions, so a Tunngle build says Tunngle and the others GameSpy), and
//! Previous Menu. Roots binds Internet's action when the form opens, so the buttons are
//! pressed at their own rectangles and do whatever Roots bound.
use super::kit;
use crate::engine::api::forms::Form;
use crate::menus::screens::multiplayer::connection_layout;
use crate::widgets::Ui;

const KINDS: &[&str] = &["buttonInternetConnect", "buttonManualIPConnect", "buttonLANTCPConnect"];

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(name), Some(back)) = (kit::shown(f, "editGameUsername"), kit::shown(f, "buttonBack")) else { return false };
    let kinds: alloc::vec::Vec<_> = KINDS.iter().filter_map(|n| kit::shown(f, n)).collect();
    if kinds.is_empty() {
        return false;
    }
    let (er, kind_rs, nav_rs) = connection_layout(ui, "MULTIPLAYER", "Name", kinds.len(), 1);
    kit::edit(ui, f, name, er);
    for (c, r) in kinds.into_iter().zip(kind_rs) {
        kit::button(ui, f, c, r);
    }
    kit::button(ui, f, back, nav_rs[0]);
    true
}
