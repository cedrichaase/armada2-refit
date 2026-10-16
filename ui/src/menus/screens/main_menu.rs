//! The main menu (`do_mainMenu`), laid out our way: the stock screen's art faded behind
//! a column of labelled buttons. The example spec (MENUS.md, "Writing a screen spec").
//!
//! The stock main menu is a picture: its buttons are `ShellButton`s with no labels,
//! the emblems and panels of the background art, and the words ("SINGLE PLAYER") are
//! painted into the background bitmap. So the model has the buttons but not their
//! names; each entry here names one by a point inside it, in design coordinates
//! (measured on the bench: testbench/ui.json has the same points). A click on an entry
//! is a click on that button, which runs the stock screen's own handler (its sound,
//! the next `g_shellMode`, the confirmation the Exit button asks for).
use crate::gfx::{fade, Weight, R};
use crate::menus::generic::shell_button_look;
use crate::menus::{actions, Screen};
use crate::rt::win::{HWND, WM_MOUSEMOVE};
use crate::widgets::{Align, Ui};

/// The stock buttons, by a design point inside each, in the order shown.
const ENTRIES: &[(&str, i32, i32)] = &[
    ("SINGLE PLAYER", 141, 111),
    ("INSTANT ACTION", 641, 89),
    ("MULTIPLAYER", 161, 478),
    ("OPTIONS", 522, 433),
    ("REPLAY INTRO", 528, 533),
    ("CREDITS", 725, 533),
    ("EXIT", 725, 439),
];

/// The entry under the pointer last frame, to tell the stock screen (its hover sound
/// and animation) only when it changes.
static mut HOVER: (HWND, i32) = (core::ptr::null_mut(), -1);

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let screen = ui.screen();
    let t = ui.theme;
    // Behind: the stock screen, faded, so its art and animations still show.
    ui.draw.rect_vgrad(screen, 0xff03060c, 0xff09121f);
    s.mirror_part(ui, [0, 0, 800, 600], s.fit.area(), 0x40ffffff);
    ui.draw.rect_hgrad(R::new(0.0, 0.0, screen.w * 0.5, screen.h), 0xe0020408, 0x00020408);

    // The column.
    let (bw, bh, gap) = (ui.u(380.0), ui.u(54.0), ui.u(12.0));
    let n = ENTRIES.len() as f32;
    let title_h = ui.u(90.0);
    let total = title_h + n * bh + (n - 1.0) * gap;
    let x = libm::roundf(screen.w * 0.08).max(ui.u(40.0));
    let mut y = libm::roundf((screen.h - total) * 0.5);
    ui.text_in(R::new(x, y, bw, title_h * 0.6), "STAR TREK", 30.0, Align::Left, fade(t.text, 0.8), Weight::Bold);
    ui.text_in(R::new(x, y + title_h * 0.45, bw, title_h * 0.5), "ARMADA II", 44.0, Align::Left, t.accent, Weight::Bold);
    y += title_h;
    let mut hover = -1;
    for (i, &(label, dx, dy)) in ENTRIES.iter().enumerate() {
        let r = R::new(x, y, bw, bh);
        y += bh + gap;
        // Options, Exit, Replay Intro and Credits are `ShellButtonSelBox`es; the three
        // big pictures are hit areas of the screen's procedure itself, not widgets. A
        // click on the point is right for both.
        let stock = s.model.button_at(dx, dy);
        let enabled = s.top && stock.is_none_or(|c| c.enabled);
        if !enabled {
            shell_button_look(ui, r, label, false, false, false);
            continue;
        }
        let id = ui.id(label);
        let (hot, held, clicked) = ui.interact(id, r);
        shell_button_look(ui, r, label, hot, held, true);
        if hot {
            hover = i as i32;
        }
        if clicked {
            s.click(dx, dy);
        }
    }
    // Off the column, the faded stock screen still works as itself: the pointer goes
    // to the dialog (its own hover pictures, and a click on an emblem), so the stock
    // screen's targets keep working (the bench's `ui.json` points, its Exit panel).
    let column = R::new(x, libm::roundf((screen.h - total) * 0.5), bw, total);
    let over_column = ui.input.mouse.is_some_and(|p| column.contains(p));
    if s.top && !over_column {
        unsafe { HOVER = (s.slot.hwnd, -2) };
        crate::menus::generic::forward_input(ui, s);
    }
    // The stock screen hears of the hover, so its own hover sound and picture play.
    unsafe {
        let h = s.slot.hwnd;
        if s.top && over_column && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 {
                let e = ENTRIES[hover as usize];
                (e.1, e.2)
            } else {
                (2, 2)
            };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
        }
    }
    // The screen's own text (the version), where the game puts it.
    for c in s.model.controls.iter() {
        if matches!(c.what, crate::menus::model::What::Shell(crate::engine::shellui::Kind::Label)) && c.visible && !c.text.is_empty() {
            let r = s.fit.rect(c.rect);
            ui.text_in(r, &c.text, 16.0, Align::Right, t.text_dim, Weight::Regular);
        }
    }
    true
}
