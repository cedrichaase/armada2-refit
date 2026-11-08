//! Roots' Admiral's Log (`AdmiralsLogDlgProc`, adopted; from the in-mission menu and after
//! a mission): Armada II's log (`screens::admirals_log::log_frame`: a window, the tab strip,
//! the players down the left, the page, Done). The tabs, the players and Done are the
//! dialog's owner-drawn buttons (child windows, read by id and text as on Armada II); a
//! click on ours is a click on the stock button, and the dialog shows the tab's pane by
//! itself.
//!
//! On Roots the eight panes are pop-ups of FleetOpsHook's, each adopted on its own and
//! drawn above the log (decision 235), and their labels hold no text (Roots paints them),
//! so Armada II's Score table and typesetting have nothing to read: the page is the shown
//! pane's own picture, fitted into our page, the pointer over it posted to the pane (its
//! sort headers and scroll buttons keep working). Save to text is left out, as on Armada
//! II. The tabs' words are painted too (their windows have none): ours, as Roots shows them.
//! The panes are not drawn again by the frame while the log has its spec (decision
//! 259).
use super::Screen;
use crate::engine::api::forms::{self, Action, Dialog};
use crate::gfx::R;
use crate::menus::model::{Class, Control, What};
use crate::menus::screens::admirals_log::log_frame;
use crate::menus::screens::kit;
use crate::rt::win::HWND;
use crate::widgets::Ui;
use alloc::vec::Vec;

/// The tabs' words as Roots paints them, by the tab's id (decision 259).
const TABS: [&str; 8] = ["Score", "Military", "Economy", "Timeline", "Battles", "Ships", "Build", "Tally"];

/// The pane the mouse is held on (its window), and the last point sent to it.
static mut HELD: usize = 0;
static mut LAST: (usize, i32, i32) = (0, i32::MIN, i32::MIN);

fn owner_drawn(c: &Control) -> bool {
    matches!(c.what, What::Window(Class::OwnerDrawn))
}

/// The log's panes: the adopted pop-ups drawn just above it (`forms::Dialog::pane`).
pub fn panes_of(log: &Dialog) -> Vec<Dialog> {
    forms::dialogs().into_iter().filter(|d| d.pane && d.z == log.z + 1).collect()
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let mut tabs: Vec<&Control> = s.model.controls.iter().filter(|c| c.id < 8 && owner_drawn(c) && c.visible).collect();
    tabs.sort_by_key(|c| c.id);
    let mut players: Vec<&Control> = s.model.controls.iter().filter(|c| (2100..2300).contains(&c.id) && c.id != 2211 && c.id != 2212 && owner_drawn(c)).collect();
    players.sort_by_key(|c| c.id);
    let Some(done) = s.model.controls.iter().find(|c| c.id == 2212 && owner_drawn(c)) else { return false };
    if tabs.len() < 2 {
        return false;
    }
    kit::backdrop(ui);
    // Roots paints the tabs' words (their windows have no text): ours, by the tab's id.
    let names: Vec<&str> = tabs.iter().map(|c| if c.text.trim().is_empty() { TABS.get(c.id as usize).copied().unwrap_or("") } else { c.text.as_str() }).collect();
    let people: Vec<&str> = players.iter().map(|c| c.text.as_str()).collect();
    let ids: Vec<i32> = players.iter().map(|c| c.id).collect();
    let out = log_frame(ui, s.hwnd() as HWND, s.top, &names, None, &people, &ids, (done.text.trim(), done.enabled));
    if let Some(i) = out.tab {
        s.click_control(tabs[i]);
    }
    if let Some(i) = out.player {
        s.click_control(players[i]);
    }
    if out.done {
        s.click_control(done);
    }
    // The page: the shown pane's picture, fitted, and the pointer to the pane.
    let panes = panes_of(s.d);
    if let Some(p) = panes.iter().find(|p| p.visible) {
        let Some(size) = p.design.map(|r| [0, 0, r[2], r[3]]) else { return true };
        let dst = kit::contain(out.page.inset(ui.u(6.0)), size);
        if let Some(t) = crate::menus::roots::dialog_texture(p) {
            ui.draw.image(t, dst, 0xff_ff_ff_ff);
        }
        if p.top || s.top {
            forward(ui, p.hwnd, dst, size);
        }
    }
    true
}

/// The pointer over `dst` (where the pane of `size` is drawn) goes to the pane as a
/// player's would: every move, the press, the moves while held, the release.
fn forward(ui: &mut Ui, pane: usize, dst: R, size: [i32; 4]) {
    let Some(m) = ui.input.mouse else { return };
    let over = dst.contains(m);
    if over {
        ui.claim(dst);
    }
    let k = dst.w / size[2].max(1) as f32;
    let x = libm::floorf((m.0 - dst.x) / k) as i32;
    let y = libm::floorf((m.1 - dst.y) / k) as i32;
    unsafe {
        let moved = LAST != (pane, x, y);
        if HELD == pane {
            if ui.input.released[0] || !ui.input.down[0] {
                HELD = 0;
                forms::act(Action::Mouse(pane, forms::MOUSE_UP, 0, x, y));
            } else if moved {
                forms::act(Action::Mouse(pane, forms::MOUSE_MOVE, forms::MOUSE_HELD, x, y));
            }
        } else if over {
            if moved {
                forms::act(Action::Mouse(pane, forms::MOUSE_MOVE, 0, x, y));
            }
            if ui.input.pressed[0] && HELD == 0 {
                HELD = pane;
                forms::act(Action::Mouse(pane, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
            }
        }
        if over || HELD == pane {
            LAST = (pane, x, y);
        }
    }
}
