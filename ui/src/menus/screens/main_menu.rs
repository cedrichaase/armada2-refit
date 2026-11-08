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
    // On a screen wider than 4:3, the widescreen plate of the same art (installed by
    // ui/backdrops/install-plates.sh) behind it at the same fade, so the art
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
        if matches!(c.what, crate::menus::model::What::Shell(crate::engine::api::shellui::Kind::Label)) && c.visible && !c.text.is_empty() {
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
                crate::engine::api::sound::shell_hover(like);
            }
        }
    }
}

/// Over render3d's scene (`menus::scene`): the stock art is not shown, so the column is
/// the screen's only target (a click on the scene does nothing), down the left over a
/// band darkened for the text; the entries are lines of text that light up, an accent
/// bar and a glow at their left, as the pointer comes over them.
fn over_scene(ui: &mut Ui, s: &Screen) -> bool {
    let entries: alloc::vec::Vec<(&str, bool)> = ENTRIES
        .iter()
        .map(|&(label, dx, dy)| (label, s.top && s.model.button_at(dx, dy).is_none_or(|c| c.enabled)))
        .collect();
    let (clicked, hover) = scene_column(ui, "STAR TREK", "ARMADA II", &entries);
    if let Some(i) = clicked {
        s.click(ENTRIES[i].1, ENTRIES[i].2);
    }
    hover_changed(s, hover);
    let m = ui.u(24.0);
    version(ui, s, Some(R::new(ui.screen().w - m - ui.u(400.0), ui.screen().h - m - ui.u(24.0), ui.u(400.0), ui.u(24.0))));
    true
}

/// The main menu over the scene, for any front end that has one (Armada II's, and Roots'
/// forms, `menus::roots`): the shade, the title (`small` over `big`, or the layout's
/// words or picture), and a column of entries, each (label, enabled), that light up
/// under the pointer. Where and how it stands is the layout's (`menus::main_layout`,
/// `Menu\mainmenu.ini`); with none it is the faded-art layout's, entry for entry. Returns
/// the entry clicked and the one under the pointer (-1: none), as indices of `entries`
/// whether or not the layout hides some.
pub fn scene_column(ui: &mut Ui, small: &str, big: &str, entries: &[(&str, bool)]) -> (Option<usize>, i32) {
    use crate::menus::main_layout::{self as ml, Side};
    let lay = ml::get();
    let screen = ui.screen();
    let t = ui.theme;
    let mirror = lay.side == Side::Right;
    let al = match lay.side {
        Side::Left => Align::Left,
        Side::Right => Align::Right,
        Side::Centre => Align::Centre,
    };
    static mut LISTED: bool = false;
    if !unsafe { LISTED } {
        unsafe { LISTED = true };
        let keys: alloc::vec::Vec<alloc::string::String> = entries.iter().map(|e| ml::norm(e.0)).collect();
        crate::log!("menus: main menu entries (the keys of [Entries]): {}", keys.join(", "));
    }
    // The entries shown, under the words the layout gives them.
    let shown: alloc::vec::Vec<(usize, alloc::string::String, bool)> =
        entries.iter().enumerate().filter_map(|(i, &(l, on))| lay.entry(l).map(|w| (i, w, on))).collect();
    // Shade for the text: the column's side, and a little at the top and the foot.
    let k = lay.opacity;
    let sh = |c: u32| (c & 0x00ff_ffff) | (((c >> 24) as f32 * k) as u32).min(255) << 24;
    let (dark, mid) = (sh(0xd8010205), sh(0xa0010205));
    let clear = sh(0x00010205);
    match lay.side {
        Side::Left => {
            ui.draw.rect_hgrad(R::new(0.0, 0.0, screen.w * 0.22, screen.h), dark, mid);
            ui.draw.rect_hgrad(R::new(screen.w * 0.22, 0.0, screen.w * 0.3, screen.h), mid, clear);
        }
        Side::Right => {
            ui.draw.rect_hgrad(R::new(screen.w * 0.78, 0.0, screen.w * 0.22, screen.h), mid, dark);
            ui.draw.rect_hgrad(R::new(screen.w * 0.48, 0.0, screen.w * 0.3, screen.h), clear, mid);
        }
        Side::Centre => {
            ui.draw.rect_hgrad(R::new(screen.w * 0.24, 0.0, screen.w * 0.26, screen.h), clear, dark);
            ui.draw.rect_hgrad(R::new(screen.w * 0.5, 0.0, screen.w * 0.26, screen.h), dark, clear);
        }
    }
    ui.draw.rect_vgrad(R::new(0.0, 0.0, screen.w, screen.h * 0.12), sh(0x80000000), sh(0x00000000));
    ui.draw.rect_vgrad(R::new(0.0, screen.h * 0.84, screen.w, screen.h * 0.16), sh(0x00000000), sh(0x90000000));

    // The column's geometry is the faded-art layout's with the built-in layout, entry for
    // entry: the bench's scenarios click its entries by position.
    let (w, eh, gap) = (ui.u(lay.width), ui.u(lay.entry_height), ui.u(lay.entry_gap));
    let n = shown.len() as f32;
    let auto_x = libm::roundf(screen.w * 0.08).max(ui.u(40.0));
    let x = match lay.side {
        Side::Left => lay.x.map_or(auto_x, |v| ui.u(v)),
        Side::Right => screen.w - lay.x.map_or(auto_x, |v| ui.u(v)) - w,
        Side::Centre => (screen.w - w) * 0.5 + lay.x.map_or(0.0, |v| ui.u(v)),
    };
    // The title: a picture when the layout names one and it is there, else two lines.
    let lettering = if lay.image.is_empty() { None } else { crate::menus::title_art::get(&lay.image, lay.crop, ui.u(lay.image_width)) };
    // `rise`: how far the title reaches above the column's nominal top; `bottom`: where
    // it ends below it.
    let (rise, bottom) = match lettering {
        Some((_, aspect)) => (ui.u(14.0), ui.u(lay.image_width) / aspect - ui.u(14.0)),
        None => (ui.u(40.0), ui.u(76.0)),
    };
    let first = bottom + ui.u(lay.title_gap);
    let total = first + n * eh + (n - 1.0).max(0.0) * gap;
    let top = match lay.y {
        Some(y) => ui.u(y) + rise,
        None => libm::roundf((screen.h - total) * 0.5),
    };
    // A box `bw` wide in the column's width, placed on its side.
    let boxed = |bw: f32| match lay.side {
        Side::Left => x,
        Side::Right => x + w - bw,
        Side::Centre => x + (w - bw) * 0.5,
    };
    if let Some((tex, aspect)) = lettering {
        // The picture holds the rule and both lines, rising a little above the column's
        // title space.
        let iw = ui.u(lay.image_width);
        let ix = match lay.side {
            Side::Left => x - ui.u(8.0),
            Side::Right => x + w - iw + ui.u(8.0),
            Side::Centre => x + (w - iw) * 0.5,
        };
        ui.draw.image(tex, R::new(ix, top - ui.u(14.0), iw, iw / aspect), 0xff_ff_ff_ff);
    } else {
        let l1 = lay.line1.text.as_deref().unwrap_or(small);
        let l2 = lay.line2.text.as_deref().unwrap_or(big);
        let off = match lay.side {
            Side::Left => -ui.u(3.0),
            Side::Right => ui.u(3.0),
            Side::Centre => 0.0,
        };
        ui.text_in(R::new(boxed(w), top - ui.u(40.0), w, ui.u(40.0)), l1, lay.line1.size, al, lay.line1.colour.unwrap_or(fade(t.text, 0.85)), Weight::Bold);
        ui.text_in(R::new(boxed(w * 1.5) + off, top - ui.u(10.0), w * 1.5, ui.u(80.0)), l2, lay.line2.size, al, lay.line2.colour.unwrap_or(t.title), Weight::Bold);
        if lay.rule {
            let rw = ui.u(260.0);
            let rx = match lay.side {
                Side::Left => x,
                Side::Right => x + w - rw,
                Side::Centre => x + (w - rw) * 0.5,
            };
            let rc = lay.rule_colour.unwrap_or(t.accent);
            let rule = R::new(rx, top + ui.u(74.0), rw, ui.u(2.0).max(1.0));
            match lay.side {
                Side::Left => ui.draw.rect_hgrad(rule, rc, fade(rc, 0.0)),
                Side::Right => ui.draw.rect_hgrad(rule, fade(rc, 0.0), rc),
                Side::Centre => {
                    ui.draw.rect_hgrad(R::new(rule.x, rule.y, rw * 0.5, rule.h), fade(rc, 0.0), rc);
                    ui.draw.rect_hgrad(R::new(rule.x + rw * 0.5, rule.y, rw * 0.5, rule.h), rc, fade(rc, 0.0));
                }
            }
        }
    }
    let mut y = top + first;
    let mut hover = -1;
    let mut picked = None;
    for &(i, ref label, enabled) in shown.iter() {
        let r = R::new(x, y, w, eh);
        y += eh + gap;
        let id = ui.id(label);
        let (hot, held, clicked) = if enabled { ui.interact(id, r) } else { (false, false, false) };
        let st = crate::widgets::State::of(ui, id, hot, held);
        let k = st.hot;
        if k > 0.01 {
            // A glow from the column's edge fading out, the accent bar, the text nudged in.
            if mirror {
                ui.draw.rect_hgrad(r, fade(t.accent, 0.0), fade(t.accent, 0.28 * k));
                ui.draw.rect(R::new(r.x + r.w - ui.u(4.0).max(2.0), r.y + ui.u(8.0), ui.u(4.0).max(2.0), r.h - ui.u(16.0)), fade(t.accent, k));
            } else {
                ui.draw.rect_hgrad(r, fade(t.accent, 0.28 * k), fade(t.accent, 0.0));
                ui.draw.rect(R::new(r.x, r.y + ui.u(8.0), ui.u(4.0).max(2.0), r.h - ui.u(16.0)), fade(t.accent, k));
            }
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
        let tr = if mirror {
            R::new(r.x + ui.u(6.0) - ui.u(10.0) * k, r.y, r.w - ui.u(24.0), r.h)
        } else {
            R::new(r.x + ui.u(18.0) + ui.u(10.0) * k, r.y, r.w - ui.u(24.0), r.h)
        };
        ui.text_in(tr, label, lay.entry_size, al, c, Weight::Bold);
        if hot {
            hover = i as i32;
        }
        if clicked {
            picked = Some(i);
        }
    }
    (picked, hover)
}
