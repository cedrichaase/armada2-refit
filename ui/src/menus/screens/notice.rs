//! The small confirmations: a message with one button (`do_oneButton`: Save Successful,
//! an error), a yes-or-no question (`do_yesNo`: Exit, abort the mission) and the waiting
//! screen of a network game (`do_netWaiting`, with its Cancel). One window over a
//! dimming of whatever is under it: the first of the stock labels is the title when
//! there is a second, the rest is the text; the buttons are the stock buttons, in a
//! row in the order they stand. All of it is the stock screen's own text.
use super::common::*;
use crate::gfx::{Weight, R};
use crate::menus::model::Control;
use crate::menus::Screen;
use crate::widgets::{Anchor, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// The message's parts out of a model's labels and buttons: the title, the text, the
/// buttons left to right. None when there is nothing to show.
pub fn parts<'a>(labels: Vec<&'a Control>, btns: Vec<&'a Control>) -> Option<(&'a str, String, Vec<&'a Control>)> {
    let (mut labels, mut btns) = (labels, btns);
    if labels.is_empty() && btns.is_empty() {
        return None;
    }
    labels.sort_by_key(|c| (c.rect[1], c.rect[0]));
    btns.sort_by_key(|c| (c.rect[0], c.rect[1]));
    let (name, body): (&str, Vec<&Control>) = if labels.len() >= 2 { (labels[0].text.as_str(), labels[1..].to_vec()) } else { ("", labels) };
    let text = body.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");
    Some((name, text, btns))
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let labels: Vec<&Control> = s.model.controls.iter().filter(|c| is_label(c) && c.visible && !c.text.is_empty()).collect();
    let btns: Vec<&Control> = s.model.controls.iter().filter(|c| is_button(c) && c.visible && !c.text.is_empty()).collect();
    let Some((name, text, btns)) = parts(labels, btns) else { return false };
    backdrop_for(ui, s);
    let mut hot = None;
    notice_window(ui, name, &text, btns.len(), &mut |ui, i, r| {
        stock_button(ui, s, btns[i], r, &mut hot);
    });
    hover_stock(s, hot);
    true
}

/// The message's window, for any front end (Armada II's hosted one and Roots' adopted
/// one, `menus::roots_dialogs`): titled `name`, the text wrapped, and `n` buttons in a
/// row at its foot, each drawn by `button` in its rectangle.
pub fn notice_window(ui: &mut Ui, name: &str, text: &str, n: usize, button: &mut dyn FnMut(&mut Ui, usize, R)) {
    notice_window_icon(ui, name, text, None, 620.0, n, button);
}

/// The size of a notice's icon, in units.
pub const ICON: f32 = 44.0;

/// As `notice_window`, `w` units wide, with a picture (a `Tex` over the whole texture, drawn
/// `ICON` units square) beside the text, which is wrapped to what is left of the width and
/// centred on the picture's height when it is shorter.
pub fn notice_window_icon(ui: &mut Ui, name: &str, text: &str, icon: Option<crate::gfx::Tex>, w: f32, n: usize, button: &mut dyn FnMut(&mut Ui, usize, R)) {
    let t = ui.theme;
    let gap = if icon.is_some() { ICON + 16.0 } else { 0.0 };
    let inner_w = ui.u(w - 2.0 * t.pad - 40.0 - gap);
    let size = ui.u(t.text_size + 2.0);
    let (_, th) = crate::text::measure_wrapped(crate::text::font(), text, size, inner_w);
    let body_h = (th / ui.scale).max(if icon.is_some() { ICON } else { 0.0 });
    let h = (body_h + 70.0 + 2.0 * t.pad + 48.0 + 44.0).max(if icon.is_some() { 200.0 } else { 240.0 });
    let r = Anchor::Centre.place(ui.screen(), ui.u(w), ui.u(h), 0.0);
    let inner = ui.window(r, name);
    let (body_r, foot) = inner.take_top(inner.h - ui.u(44.0 + 12.0));
    let tx = body_r.inset(ui.u(20.0));
    let top = tx.y + ui.u(8.0);
    if let Some(tex) = icon {
        let ir = R::new(tx.x, top, ui.u(ICON), ui.u(ICON));
        ui.draw.image(tex, ir, 0xff_ff_ff_ff);
    }
    let ty = top + (ui.u(body_h) - th).max(0.0) * 0.5;
    ui.draw.text_wrapped(text, size, inner_w, tx.x + ui.u(gap), ty, t.text, Weight::Regular);

    let nf = n.max(1) as f32;
    let (bw, gap) = (ui.u(180.0), ui.u(24.0));
    let total = nf * bw + (nf - 1.0) * gap;
    let mut x = foot.x + (foot.w - total) * 0.5;
    for i in 0..n {
        button(ui, i, R::new(x, foot.bottom() - ui.u(44.0), bw, ui.u(44.0)));
        x += bw + gap;
    }
}
