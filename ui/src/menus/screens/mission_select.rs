//! A campaign's mission list (`do_missionSelect`): the stock rows (`ShellButtonSelBox`es,
//! one per mission, labelled by the game) as our list in a window over the campaign
//! screen, and O.K. and Cancel. Picking a row is a click on the stock row, so the stock
//! screen keeps the choice; O.K. is a click on the stock O.K. (a double click on a row
//! does the same).
use crate::gfx::{Weight, R};
use crate::menus::screens::kit::{self, Memory};
use crate::menus::Screen;
use crate::rt::win::HWND;
use crate::widgets::{theme, Align, ListEvent, State, Ui};

#[derive(Default)]
struct Sel(Option<usize>);

static mut SEL: Memory<Sel> = Memory::new(Sel(None));

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let buttons = kit::buttons(s);
    if buttons.len() < 3 {
        return false;
    }
    // The two at the foot are O.K. and Cancel; the others are the rows, top to bottom.
    let foot_y = buttons.iter().map(|c| c.rect[1]).max().unwrap_or(0);
    let mut foot: alloc::vec::Vec<_> = buttons.iter().copied().filter(|c| c.rect[1] >= foot_y - 2).collect();
    foot.sort_by_key(|c| c.rect[0]);
    let mut rows: alloc::vec::Vec<_> = buttons.iter().copied().filter(|c| c.rect[1] < foot_y - 2).collect();
    rows.sort_by_key(|c| c.rect[1]);
    if foot.len() != 2 || rows.is_empty() {
        return false;
    }
    let title = kit::labels(s).into_iter().find(|c| c.visible && !c.text.is_empty()).map_or("Select Mission", |c| c.text.as_str());
    let names: alloc::vec::Vec<&str> = rows.iter().map(|c| c.text.as_str()).collect();
    let foot_entries = [(foot[0].text.as_str(), foot[0].enabled), (foot[1].text.as_str(), foot[1].enabled)];
    let out = mission_list(ui, s.slot.hwnd, title, &names, foot_entries, s.top);
    if let Some(i) = out.picked {
        s.click_control(rows[i]);
    }
    if let Some(i) = out.foot {
        s.click_control(foot[i]);
    }
    if out.activated && foot[0].enabled {
        s.click_control(foot[0]);
    }
    true
}

/// What the mission list was told this frame: a row picked (to be clicked on the stock
/// row), a double click or Enter on one (O.K.), a foot button clicked.
pub struct MissionOut {
    pub picked: Option<usize>,
    pub activated: bool,
    pub foot: Option<usize>,
}

/// The mission list, for any front end (Armada II's hosted one, and Roots' adopted one,
/// `menus::roots_dialogs`): a scrim, a window titled `title` with the rows (`dlg` keys
/// the choice kept between frames), and the foot's two buttons (label, enabled), which
/// act only on the top dialog (`top`). The stock rows do not show which one is chosen
/// (their state is only hover and held), so the choice is ours: it is what the clicks on
/// the stock rows made.
pub fn mission_list(ui: &mut Ui, dlg: HWND, title: &str, rows: &[&str], foot: [(&str, bool); 2], top: bool) -> MissionOut {
    let t = ui.theme;
    kit::scrim(ui);
    let row_h = ui.u(t.text_size + 16.0);
    let want = 150.0 + rows.len() as f32 * (t.text_size + 16.0) + t.button_h;
    let win = kit::centred(ui, 620.0, want);
    let inner = ui.window(win, title);

    let sel = unsafe { &mut *core::ptr::addr_of_mut!(SEL) }.of(dlg);
    let foot_h = ui.u(t.button_h);
    let list = R::new(inner.x, inner.y, inner.w, inner.h - foot_h - ui.u(t.gap));
    let mut chosen = sel.0;
    let mut ev = ListEvent::None;
    if top {
        ev = ui.list(list, "missions", rows.len(), row_h, &mut chosen, |ui, i, rr, _sel, _h| {
            ui.text_in(rr, rows[i], t.text_size, Align::Centre, t.text, Weight::Regular);
        });
    } else {
        let mut none = chosen;
        ui.list(list, "missions", rows.len(), row_h, &mut none, |ui, i, rr, _sel, _h| {
            ui.text_in(rr, rows[i], t.text_size, Align::Centre, t.text, Weight::Regular);
        });
    }
    let mut out = MissionOut { picked: None, activated: ev == ListEvent::Activated, foot: None };
    if let Some(i) = chosen {
        if ev != ListEvent::None && i < rows.len() {
            out.picked = Some(i);
        }
        sel.0 = Some(i);
    }
    let cells = {
        let mut st = crate::widgets::Stack::new(R::new(inner.x, inner.bottom() - foot_h, inner.w, foot_h), ui.u(t.gap));
        st.cells::<2>(foot_h)
    };
    for (i, &(label, on)) in foot.iter().enumerate() {
        // As `kit::button`: only the top dialog acts; a disabled control is drawn so.
        if top && on {
            if ui.button(cells[i], label) {
                out.foot = Some(i);
            }
        } else {
            theme::button(ui, cells[i], label, State { off: !on, ..State::default() });
        }
    }
    out
}
