//! The mission scripts' team texts, ours (`[HUD] TeamTexts=1`; HUD.md, "The trade
//! status and the team message"): while a script shows the resources sent to another
//! team (the second Federation mission's "Sent to Team 2"), a small panel under the
//! top-right buttons with the team's name in its colour and a row per resource (a mark
//! in the resource's colour, its name, the amount); while it shows a team message, the
//! message in a strip under the resources. The stock printing is skipped
//! (`engine::trade`).
use crate::engine::{topbar, trade};
use crate::gfx::{Corners, Weight, R};
use crate::widgets::{Align, Anchor, Ui};
use alloc::string::String;

static mut LOGGED: Option<String> = None;

fn log_once(s: String) {
    #[allow(static_mut_refs)]
    let l = unsafe { &mut LOGGED };
    if l.as_ref() != Some(&s) {
        crate::log!("trade: {}", s);
        *l = Some(s);
    }
}

/// "Dilithium: 0" as its name and its amount.
fn split(line: &str) -> (&str, &str) {
    match line.rfind(':') {
        Some(i) => (line[..i].trim(), line[i + 1..].trim()),
        None => (line.trim(), ""),
    }
}

pub fn run(ui: &mut Ui) {
    let trades = trade::trades();
    let message = trade::message();
    match (&trades, &message) {
        (None, None) => log_once(String::new()),
        _ => log_once(crate::sfmt!(
            "{} | {}",
            trades.as_ref().map_or(String::new(), |t| crate::sfmt!("{} {} {} {}", t.title, t.lines[0].1, t.lines[1].1, t.lines[2].1)),
            message.as_deref().unwrap_or("")
        )),
    }
    if crate::engine::ingame::hud_hidden() {
        return;
    }
    let t = ui.theme;
    if let Some(tr) = trades {
        let (w, row) = (ui.u(190.0), ui.u(24.0));
        let h = row * 1.25 + 3.0 * row;
        // Under the top-right buttons (their panel's foot, or where it would be).
        let top = super::buttons::bottom().unwrap_or(ui.u(70.0)) + ui.u(6.0);
        let (outer, inner) = super::panels::fit(ui, Anchor::TopRight, w, h, ui.u(6.0));
        let (outer, inner) = (R::new(outer.x, top, outer.w, outer.h), R::new(inner.x, inner.y - outer.y + top, inner.w, inner.h));
        ui.panel(outer);
        let title = tr.title.trim_end_matches(':').trim();
        let head = R::new(inner.x, inner.y, inner.w, row * 1.25);
        let mark = R::new(head.x, head.y + head.h * 0.2, ui.u(4.0), head.h * 0.6);
        ui.draw.fill(mark, Corners::round(ui.u(2.0)), tr.colour);
        let tx = mark.right() + ui.u(8.0);
        ui.text_in(R::new(tx, head.y, head.right() - tx, head.h), &crate::widgets::theme::caps(title), t.small_size, Align::Left, t.title, Weight::Bold);
        for (i, (r, line)) in tr.lines.iter().enumerate() {
            let cell = R::new(inner.x, head.bottom() + i as f32 * row, inner.w, row);
            let (name, amount) = split(line);
            let mark = R::new(cell.x, cell.y + cell.h * 0.22, ui.u(4.0), cell.h * 0.56);
            ui.draw.fill(mark, Corners::round(ui.u(2.0)), topbar::colour(*r));
            let x = mark.right() + ui.u(8.0);
            ui.text_in(R::new(x, cell.y, cell.right() - x, cell.h), name, t.small_size, Align::Left, t.text_dim, Weight::Regular);
            ui.text_in(R::new(x, cell.y, cell.right() - x, cell.h), amount, t.small_size, Align::Right, t.text, Weight::Bold);
        }
    }
    if let Some(m) = message {
        // Under the resources' strip (about 70 units), centred: the script's status line.
        let px = ui.u(t.small_size);
        let (tw, _) = crate::text::measure_with(crate::text::font_bold(), &m, px);
        let (w, h) = (tw + ui.u(32.0), ui.u(30.0));
        let r = R::new(libm::roundf((ui.w - w) * 0.5), ui.u(84.0), w, h);
        ui.draw.fill(r, Corners::round(ui.u(6.0)), crate::gfx::fade(t.panel, 0.75));
        ui.text_in(r, &m, t.small_size, Align::Centre, t.title, Weight::Bold);
    }
}
