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
//!
//! Over render3d's menu scene (`menus::scene`) the stock art is not drawn at all and the
//! column stands alone (`over_scene`).
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
    if crate::menus::scene::shown() {
        return over_scene(ui, s);
    }
    let screen = ui.screen();
    let t = ui.theme;
    // Behind: the stock screen, faded, so its art and animations still show.
    ui.draw.rect_vgrad(screen, 0xff03060c, 0xff09121f);
    // On a screen wider than 4:3, the widescreen plate of the same art (Menus.asi's,
    // installed by menus/install-plates.sh) behind it at the same fade, so the art
    // fills the width: the plate is the stock picture in its middle, widened.
    // Only the sides: the middle is the mirror, which carries the animations.
    let area = s.fit.area();
    if let (Some(tex), true) = (crate::widgets::theme::plate(), area.x > 0.5) {
        let (tw, th) = crate::gfx::tex_size(tex);
        let w = area.h * tw as f32 / th.max(1) as f32;
        let full = R::new(area.x + area.w * 0.5 - w * 0.5, area.y, w, area.h);
        let u = |x: f32| (x - full.x) / full.w;
        let left = R::new(full.x.max(0.0), area.y, area.x - full.x.max(0.0), area.h);
        let right = R::new(area.right(), area.y, full.right().min(screen.w) - area.right(), area.h);
        for r in [left, right] {
            if r.w > 0.5 {
                ui.draw.image_uv(tex, r, [u(r.x), 0.0, u(r.right()), 1.0], 0x40ffffff);
            }
        }
    }
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
    if over_column {
        hover_changed(s, hover);
    }
    version(ui, s, None);
    true
}

/// The screen's own text (the version): where the game puts it, or at `at` (the
/// bottom-right corner) over the scene.
fn version(ui: &mut Ui, s: &Screen, at: Option<R>) {
    let t = ui.theme;
    for c in s.model.controls.iter() {
        if matches!(c.what, crate::menus::model::What::Shell(crate::engine::shellui::Kind::Label)) && c.visible && !c.text.is_empty() {
            let r = at.unwrap_or_else(|| s.fit.rect(c.rect));
            ui.text_in(r, &c.text, t.small_size, Align::Right, t.text_dim, Weight::Regular);
        }
    }
}

/// Tells the stock screen of a change of hover (its own sound plays).
fn hover_changed(s: &Screen, hover: i32) {
    unsafe {
        let h = s.slot.hwnd;
        if s.top && HOVER != (h, hover) {
            HOVER = (h, hover);
            let (x, y) = if hover >= 0 {
                let e = ENTRIES[hover as usize];
                (e.1, e.2)
            } else {
                (2, 2)
            };
            actions::push(actions::Action::Mouse { h, msg: WM_MOUSEMOVE, keys: 0, x: x - s.model.rect[0], y: y - s.model.rect[1] });
            // Single Player, Instant Action and Multiplayer are hit areas of the
            // procedure, which plays no sound for them (only the four buttons do): ours
            // plays the buttons' own.
            if hover >= 0 && s.model.button_at(x, y).is_none() {
                let like = ENTRIES.iter().find_map(|e| s.model.button_at(e.1, e.2)).map(|c| c.key);
                crate::engine::sound::shell_hover(like);
            }
        }
    }
}

/// Over render3d's scene (`menus::scene`): the stock art is not shown, so the column is
/// the screen's only target (a click on the scene does nothing), down the left over a
/// band darkened for the text; the entries are lines of text that light up, an accent
/// bar and a glow at their left, as the pointer comes over them.
fn over_scene(ui: &mut Ui, s: &Screen) -> bool {
    let screen = ui.screen();
    let t = ui.theme;
    // Shade for the text: the left, and a little at the top and the foot.
    ui.draw.rect_hgrad(R::new(0.0, 0.0, screen.w * 0.22, screen.h), 0xd8010205, 0xa0010205);
    ui.draw.rect_hgrad(R::new(screen.w * 0.22, 0.0, screen.w * 0.3, screen.h), 0xa0010205, 0x00010205);
    ui.draw.rect_vgrad(R::new(0.0, 0.0, screen.w, screen.h * 0.12), 0x80000000, 0x00000000);
    ui.draw.rect_vgrad(R::new(0.0, screen.h * 0.84, screen.w, screen.h * 0.16), 0x00000000, 0x90000000);

    // The column's geometry is the faded-art layout's (`draw`), entry for entry: the
    // bench's scenarios click its entries by position.
    let (w, eh, gap) = (ui.u(380.0), ui.u(54.0), ui.u(12.0));
    let n = ENTRIES.len() as f32;
    let title_h = ui.u(90.0);
    let total = title_h + n * eh + (n - 1.0) * gap;
    let x = libm::roundf(screen.w * 0.08).max(ui.u(40.0));
    let top = libm::roundf((screen.h - total) * 0.5);
    // The title, rising above the column's own title space.
    ui.text_in(R::new(x, top - ui.u(40.0), w, ui.u(40.0)), "STAR TREK", 28.0, Align::Left, fade(t.text, 0.85), Weight::Bold);
    ui.text_in(R::new(x - ui.u(3.0), top - ui.u(10.0), w * 1.5, ui.u(80.0)), "ARMADA II", 68.0, Align::Left, t.title, Weight::Bold);
    let rule = R::new(x, top + ui.u(74.0), ui.u(260.0), ui.u(2.0).max(1.0));
    ui.draw.rect_hgrad(rule, t.accent, fade(t.accent, 0.0));
    let mut y = top + title_h;
    let mut hover = -1;
    for (i, &(label, dx, dy)) in ENTRIES.iter().enumerate() {
        let r = R::new(x, y, w, eh);
        y += eh + gap;
        let stock = s.model.button_at(dx, dy);
        let enabled = s.top && stock.is_none_or(|c| c.enabled);
        let id = ui.id(label);
        let (hot, held, clicked) = if enabled { ui.interact(id, r) } else { (false, false, false) };
        let st = crate::widgets::State::of(ui, id, hot, held);
        let k = st.hot;
        if k > 0.01 {
            // A glow from the left fading out, the accent bar, the text nudged right.
            ui.draw.rect_hgrad(r, fade(t.accent, 0.28 * k), fade(t.accent, 0.0));
            ui.draw.rect(R::new(r.x, r.y + ui.u(8.0), ui.u(4.0).max(2.0), r.h - ui.u(16.0)), fade(t.accent, k));
        }
        let c = if !enabled {
            t.text_off
        } else if held {
            t.accent
        } else if k > 0.01 {
            crate::gfx::mix(t.text, 0xffffffff, k)
        } else {
            t.text
        };
        let tr = R::new(r.x + ui.u(18.0) + ui.u(10.0) * k, r.y, r.w - ui.u(24.0), r.h);
        ui.text_in(tr, label, 26.0, Align::Left, c, Weight::Bold);
        if hot {
            hover = i as i32;
        }
        if clicked {
            s.click(dx, dy);
        }
    }
    hover_changed(s, hover);
    let m = ui.u(24.0);
    version(ui, s, Some(R::new(screen.w - m - ui.u(400.0), screen.h - m - ui.u(24.0), ui.u(400.0), ui.u(24.0))));
    true
}
