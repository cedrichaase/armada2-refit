//! The briefing and objectives, ours (`[HUD] Messages=1`; HUD.md, "Messages and
//! objectives"): while the stock `ObjectivesDisplay` shows, a window over a dimmed
//! screen with the mission's title, the briefing's summary, and the objectives, each
//! with a check box ticked when the mission has marked it complete
//! (`SetObjectiveComplete`). OK presses the stock box's OK, which hides it; the keys
//! that close it are the stock box's own, run by `popups::objectives_keys`, and the
//! game's objectives key and button show it again (the stock box's hidden step and
//! `ButtonPanel::ShowObjectives`, both untouched).
use crate::engine::popups;
use crate::gfx::{Weight, R};
use crate::widgets::{theme, Ui};
use alloc::string::String;

static mut LOGGED: bool = false;

pub fn run(ui: &mut Ui) {
    if !popups::objectives_shown() {
        unsafe { LOGGED = false };
        return;
    }
    let o = popups::objectives();
    unsafe {
        if !LOGGED {
            let mut s = String::new();
            for (l, done) in &o.lines {
                s.push_str(&crate::sfmt!(" [{}] {}", if *done { "X" } else { " " }, l.chars().take(40).collect::<String>()));
            }
            crate::log!("objectives: \"{}\" / \"{}\" / \"{}\", {} objectives:{}; OK {}", o.title, o.summary_header, o.header, o.lines.len(), s,
                if o.ok != 0 { "found" } else { "NOT found" });
            LOGGED = true;
        }
    }
    ui.overlay(move |ui| draw(ui, o));
}

fn draw(ui: &mut Ui, o: popups::Objectives) {
    let t = ui.theme;
    let w_units = 900.0;
    let pad = ui.u(t.pad);
    let px = ui.u(t.text_size);
    let text_w = ui.u(w_units) - 4.0 * pad;
    let font = crate::text::font();
    let (_, brief_h) = crate::text::measure_wrapped(font, &o.briefing, px, text_w);
    let box_w = ui.u(22.0);
    // The check box's real width (a pill switch in LCARS), then a gap before the text.
    let lead = theme::check_w(ui, box_w) + pad;
    let mut lines_h = 0.0;
    let mut line_hs = alloc::vec::Vec::new();
    for (l, _) in &o.lines {
        let (_, h) = crate::text::measure_wrapped(font, l, px, text_w - lead);
        let h = h.max(box_w);
        line_hs.push(h);
        lines_h += h + ui.u(t.gap);
    }
    let head_h = ui.u(t.heading_size) * 1.6;
    let bh = ui.u(t.button_h);
    let content = head_h + brief_h + pad + head_h + lines_h + pad + bh;
    let h_units = (content / ui.scale + t.title_size + 6.0 * t.pad + 40.0).min(1000.0);
    let title = if o.title.is_empty() { " " } else { o.title.as_str() };
    let inner = super::panels::boxed(ui, title, w_units, h_units, 1.0);
    let x = inner.x + pad * 0.5;
    let w = inner.w - pad;
    let mut y = inner.y;
    theme::heading(ui, R::new(x, y, w, head_h), &o.summary_header);
    y += head_h;
    ui.draw.text_wrapped(&o.briefing, px, w, x, y, t.text, Weight::Regular);
    y += brief_h + pad;
    theme::heading(ui, R::new(x, y, w, head_h), &o.header);
    y += head_h;
    for (i, (l, done)) in o.lines.iter().enumerate() {
        let h = line_hs.get(i).copied().unwrap_or(box_w);
        let b = R::new(x, y + (ui.u(t.text_size) * 1.25 - box_w) * 0.5, box_w, box_w);
        theme::check(ui, b, *done, theme::State::fixed(0.0, false, *done, false));
        let col = if *done { t.text_dim } else { t.text };
        ui.draw.text_wrapped(l, px, w - lead, x + lead, y, col, Weight::Regular);
        y += h + ui.u(t.gap);
    }
    let bw = ui.u(180.0);
    let r = R::new(inner.right() - bw, inner.bottom() - bh, bw, bh);
    if o.ok != 0 {
        super::panels::game_text_button(ui, r, o.ok, "OK", &[]);
    }
}
