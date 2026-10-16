//! A campaign's mission list (`do_missionSelect`): the stock rows (`ShellButtonSelBox`es,
//! one per mission, labelled by the game) as our list in a window over the campaign
//! screen, and O.K. and Cancel. Picking a row is a click on the stock row, so the stock
//! screen keeps the choice; O.K. is a click on the stock O.K. (a double click on a row
//! does the same).
use crate::gfx::{Weight, R};
use crate::menus::screens::kit::{self, Memory};
use crate::menus::Screen;
use crate::widgets::{Align, ListEvent, Ui};

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
    let t = ui.theme;
    let title = kit::labels(s).into_iter().find(|c| c.visible && !c.text.is_empty()).map_or("Select Mission", |c| c.text.as_str());

    kit::scrim(ui);
    let row_h = ui.u(t.text_size + 16.0);
    let want = 150.0 + rows.len() as f32 * (t.text_size + 16.0) + t.button_h;
    let win = kit::centred(ui, 620.0, want);
    let inner = ui.window(win, title);

    let sel = unsafe { &mut SEL }.of(s.slot.hwnd);
    // The stock rows do not show which one is chosen (their state is only hover and
    // held), so the choice is ours: it is what the clicks on the stock rows made.
    let foot_h = ui.u(t.button_h);
    let list = R::new(inner.x, inner.y, inner.w, inner.h - foot_h - ui.u(t.gap));
    let mut chosen = sel.0;
    let mut ev = ListEvent::None;
    if s.top {
        ev = ui.list(list, "missions", rows.len(), row_h, &mut chosen, |ui, i, rr, _sel, _h| {
            ui.text_in(rr, &rows[i].text, t.text_size, Align::Centre, t.text, Weight::Regular);
        });
    } else {
        let mut none = chosen;
        ui.list(list, "missions", rows.len(), row_h, &mut none, |ui, i, rr, _sel, _h| {
            ui.text_in(rr, &rows[i].text, t.text_size, Align::Centre, t.text, Weight::Regular);
        });
    }
    if let Some(i) = chosen {
        if ev != ListEvent::None && i < rows.len() {
            s.click_control(rows[i]);
        }
        sel.0 = Some(i);
    }
    let cells = {
        let mut st = crate::widgets::Stack::new(R::new(inner.x, inner.bottom() - foot_h, inner.w, foot_h), ui.u(t.gap));
        st.cells::<2>(foot_h)
    };
    kit::button(ui, s, cells[0], foot[0], &foot[0].text);
    kit::button(ui, s, cells[1], foot[1], &foot[1].text);
    if ev == ListEvent::Activated && foot[0].enabled {
        s.click_control(foot[0]);
    }
    true
}
