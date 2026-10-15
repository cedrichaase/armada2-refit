//! What the UI shows in each context. For now the runtime's own demonstration
//! overlays, switched on per context in UI.ini's `[Demo]`; nothing is drawn otherwise.
use crate::surface::Context;
use crate::widgets::Ui;

pub fn run(_ctx: Context, _ui: &mut Ui) {}
