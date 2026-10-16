//! The replay controls, ours (`[HUD] Replay=1`; HUD.md, "Pause and replay"): while
//! the stock `ReplayPanel` shows (in a replay), a row of five buttons at the top of
//! the screen, under the resources, each showing the stock button's picture and
//! pressing it: restart, pause, slow, normal, fast.
use crate::engine::popups;
use crate::widgets::{Anchor, Ui};

static mut LOGGED: bool = false;

pub fn run(ui: &mut Ui) {
    if !popups::replay_shown() {
        unsafe { LOGGED = false };
        return;
    }
    let found: [usize; 5] = core::array::from_fn(popups::replay_button);
    unsafe {
        if !LOGGED {
            crate::log!("replay: shown, buttons {:?}", found.map(|b| b != 0));
            LOGGED = true;
        }
    }
    let n = found.iter().filter(|&&b| b != 0).count();
    if n == 0 {
        return;
    }
    let (cs, gap) = (ui.u(44.0), ui.u(5.0));
    let w = n as f32 * cs + (n - 1) as f32 * gap;
    // Under the resource strip (its height, 46 units plus its frame and margin).
    let (outer, inner) = super::panels::fit(ui, Anchor::Top, w, cs, ui.u(6.0));
    let dy = ui.u(46.0 + 2.0 * ui.theme.pad + 16.0);
    let outer = crate::gfx::R::new(outer.x, outer.y + dy, outer.w, outer.h);
    let inner = crate::gfx::R::new(inner.x, inner.y + dy, inner.w, inner.h);
    ui.panel(outer);
    let mut i = 0;
    for (k, &b) in found.iter().enumerate() {
        if b == 0 {
            continue;
        }
        let r = crate::gfx::R::new(inner.x + i as f32 * (cs + gap), inner.y, cs, cs);
        super::panels::game_button(ui, r, b, popups::REPLAY_NAMES[k], false);
        i += 1;
    }
}
