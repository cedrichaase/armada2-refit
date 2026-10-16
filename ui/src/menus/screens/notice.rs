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
use crate::widgets::{Align, Anchor, Ui};
use alloc::vec::Vec;

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let mut labels: Vec<&Control> = s.model.controls.iter().filter(|c| is_label(c) && c.visible && !c.text.is_empty()).collect();
    let mut btns: Vec<&Control> = s.model.controls.iter().filter(|c| is_button(c) && c.visible && !c.text.is_empty()).collect();
    if labels.is_empty() && btns.is_empty() {
        return false;
    }
    labels.sort_by_key(|c| (c.rect[1], c.rect[0]));
    btns.sort_by_key(|c| (c.rect[0], c.rect[1]));
    let (name, body): (&str, Vec<&Control>) = if labels.len() >= 2 { (labels[0].text.as_str(), labels[1..].to_vec()) } else { ("", labels) };
    let text = body.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");

    backdrop_for(ui, s);
    let t = ui.theme;
    let w = 620.0;
    let inner_w = ui.u(w - 2.0 * t.pad - 40.0);
    let size = ui.u(t.text_size + 2.0);
    let (_, th) = crate::text::measure_wrapped(crate::text::font(), &text, size, inner_w);
    let h = (th / ui.scale + 70.0 + 2.0 * t.pad + 48.0 + 44.0).max(240.0);
    let r = Anchor::Centre.place(ui.screen(), ui.u(w), ui.u(h), 0.0);
    let inner = ui.window(r, name);
    let (body_r, foot) = inner.take_top(inner.h - ui.u(44.0 + 12.0));
    let tx = body_r.inset(ui.u(20.0));
    ui.draw.text_wrapped(&text, size, tx.w, tx.x, tx.y + ui.u(8.0), t.text, Weight::Regular);

    let mut hot = None;
    let n = btns.len().max(1) as f32;
    let (bw, gap) = (ui.u(180.0), ui.u(24.0));
    let total = n * bw + (n - 1.0) * gap;
    let mut x = foot.x + (foot.w - total) * 0.5;
    for c in btns {
        stock_button(ui, s, c, R::new(x, foot.bottom() - ui.u(44.0), bw, ui.u(44.0)), &mut hot);
        x += bw + gap;
    }
    hover_stock(s, hot);
    let _ = Align::Left;
    true
}
