//! Roots' Game Settings (`GameSettingsDlgProc`, adopted, opened from Options): Armada II's
//! settings form (`screens::form`: a window, a caption over each control, the sliders the
//! stock `ShellScrollBar`s, the choices the stock cycle buttons, Previous Menu at the foot).
//!
//! Roots keeps the stock controls (six sliders, the difficulty and the interface mouse
//! mode as cycle buttons) but its `TextLabel`s hold no text: Roots paints its captions
//! from its own language table, so the model has no words to pair with the controls, and
//! Armada II's `form::build` (which pairs a caption with the control under it) finds
//! nothing. The captions are words of ours, as Roots shows them, given to the controls in
//! the order Roots stacks them (decision 256, as decision 243 for the forms); a screen with
//! another number of sliders or buttons is left to its picture. Every change is a click on
//! the stock control (a slider's track at the point of the value, `click_scroll_with`; a
//! cycle button pressed), the pointer resting first.
use super::Screen;
use crate::engine::api::shellui::Kind;
use crate::gfx::{fade, Weight, R};
use crate::menus::generic::{shell_button_look, shell_slider_with};
use crate::menus::model::{Control, What};
use crate::widgets::{Align, Anchor, Ui};
use alloc::vec::Vec;

/// The sliders' captions top to bottom, and which have the end captions Roots paints
/// beside them (the AI handicaps: Hard at the low end, Easy at the high).
const SLIDERS: [(&str, bool); 6] = [
    ("Game Speed", false),
    ("Mouse Scroll Speed", false),
    ("Keyboard Scroll Speed", false),
    ("Mouse Hold Level", false),
    ("AI Build Cost Handicap", true),
    ("AI Build Time Handicap", true),
];
const ENDS: [&str; 2] = ["Hard", "Easy"];
/// The cycle buttons' captions top to bottom.
const CHOICES: [&str; 2] = ["Game Difficulty (Single Player Only)", "Interface Mouse Mode"];
const NOTE: &str = "(AI Handicaps for Single Player Only)";

// Armada II's form's measures (`screens::form`), in units.
const LABEL_H: f32 = 26.0;
const FIELD_H: f32 = 40.0;
const SLIDER_H: f32 = 28.0;
const CAP_H: f32 = 20.0;
const GAP: f32 = 16.0;

/// The button under the pointer last frame (0: none), to tell the stock screen.
static mut HOVER: (usize, usize) = (0, 0);

fn is(c: &Control, k: Kind) -> bool {
    matches!(c.what, What::Shell(x) if x == k)
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let mut sliders: Vec<&Control> = s.model.controls.iter().filter(|c| c.visible && is(c, Kind::Scroll)).collect();
    sliders.sort_by_key(|c| c.rect[1]);
    let buttons = s.buttons();
    let mut foot: Vec<&Control> = buttons.iter().copied().filter(|c| c.rect[1] >= 800 && !c.text.trim().is_empty()).collect();
    foot.sort_by_key(|c| c.rect[0]);
    let mut choices: Vec<&Control> = buttons.iter().copied().filter(|c| c.rect[1] < 800 && !is(c, Kind::Scroll) && !c.text.trim().is_empty()).collect();
    choices.sort_by_key(|c| c.rect[1]);
    if sliders.len() != SLIDERS.len() || choices.len() != CHOICES.len() || foot.is_empty() {
        return false;
    }
    let t = ui.theme;
    let sc = ui.screen();
    crate::menus::screens::kit::backdrop(ui);
    let left_h: f32 = SLIDERS.iter().map(|&(_, ends)| LABEL_H + SLIDER_H + if ends { CAP_H } else { 0.0 } + GAP).sum::<f32>() + LABEL_H;
    let right_h = CHOICES.len() as f32 * (LABEL_H + FIELD_H + GAP);
    let body_h = left_h.max(right_h);
    let h = (body_h + 60.0 + 44.0 + 2.0 * t.pad + 54.0 + 36.0).min(1040.0);
    let r = Anchor::Centre.place(sc, ui.u(1180.0), ui.u(h), 0.0);
    let inner = ui.window(r, "Game Settings");
    let (body, foot_r) = inner.take_top(inner.h - ui.u(44.0 + 12.0));
    let gap = ui.u(40.0);
    let cw = (body.w - gap) * 0.5;
    let mut hot: usize = 0;
    let mut clicks: Vec<(i32, i32)> = Vec::new();

    // The sliders, left.
    let mut y = body.y + ui.u(6.0);
    for (i, c) in sliders.iter().enumerate() {
        let (caption, ends) = SLIDERS[i];
        ui.text_in(R::new(body.x, y, cw, ui.u(LABEL_H)), caption, t.text_size, Align::Left, t.text_dim, Weight::Regular);
        let sr = R::new(body.x, y + ui.u(LABEL_H), cw, ui.u(SLIDER_H));
        ui.scope(&crate::sfmt!("s{}", i), |ui| shell_slider_with(ui, c, sr, "bar", s.top, &mut |x, y| clicks.push((x, y))));
        let mut ih = LABEL_H + SLIDER_H + GAP;
        if ends {
            let cr = R::new(body.x, sr.bottom(), cw, ui.u(CAP_H));
            ui.text_in(cr, ENDS[0], t.small_size, Align::Left, t.text_dim, Weight::Regular);
            ui.text_in(cr, ENDS[1], t.small_size, Align::Right, t.text_dim, Weight::Regular);
            ih += CAP_H;
        }
        y += ui.u(ih);
    }
    ui.text_in(R::new(body.x, y, cw, ui.u(LABEL_H)), NOTE, t.small_size, Align::Left, t.text_dim, Weight::Regular);

    // The choices, right.
    let x = body.x + cw + gap;
    let mut y = body.y + ui.u(6.0);
    for (i, c) in choices.iter().enumerate() {
        ui.text_in(R::new(x, y, cw, ui.u(LABEL_H)), CHOICES[i], t.text_size, Align::Left, t.text_dim, Weight::Regular);
        let br = R::new(x, y + ui.u(LABEL_H), cw, ui.u(FIELD_H));
        if ui.scope(&crate::sfmt!("c{}", i), |ui| stock_button(ui, s, c, br, &mut hot)) {
            clicks.push((c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2));
        }
        y += ui.u(LABEL_H + FIELD_H + GAP);
    }
    ui.draw.rect(R::new(x - gap * 0.5, body.y, 1.0_f32.max(ui.u(1.0)), body.h - ui.u(12.0)), fade(t.panel_edge, 0.5));

    // The foot: Previous Menu, as Armada II's form's footer.
    let (w, bh, fgap) = (ui.u(220.0), ui.u(44.0), ui.u(12.0));
    let mut fx = foot_r.x;
    for c in foot {
        let br = R::new(fx, foot_r.bottom() - bh, w, bh);
        if ui.scope(&crate::sfmt!("f{:x}", c.key), |ui| stock_button(ui, s, c, br, &mut hot)) {
            clicks.push((c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2));
        }
        fx += w + fgap;
    }
    for (x, y) in clicks {
        s.click(x, y);
    }
    unsafe {
        if s.top && HOVER != (s.hwnd(), hot) {
            HOVER = (s.hwnd(), hot);
            let at = buttons.iter().find(|c| c.key == hot).map(|c| (c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2));
            s.hover(at);
        }
    }
    true
}

/// A stock button as our button, as Armada II's `common::stock_button` draws it: true on
/// its click; the pointer over it is noted in `hot`.
fn stock_button(ui: &mut Ui, s: &Screen, c: &Control, r: R, hot: &mut usize) -> bool {
    let label = c.text.trim();
    if !(s.top && c.enabled) {
        shell_button_look(ui, r, label, false, false, false);
        return false;
    }
    if ui.input.mouse.is_some_and(|p| r.contains(p)) {
        *hot = c.key;
    }
    ui.button(r, label)
}
