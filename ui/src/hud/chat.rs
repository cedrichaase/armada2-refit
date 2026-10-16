//! The chat lines and the text entry, ours (`[HUD] Chat=1`; HUD.md, "Chat and the text
//! entry"): the engine's chat lines (`ChatDisplay`, each in its own colour) down the
//! left of the screen under the top bar, and while a text-entry line is shown, a field
//! under them with its prompt (whom the line goes to) in the accent, what has been
//! typed, and a caret where the engine's caret is. The keys stay the engine's own
//! (`TextInput_ProcessInput`): we only draw. The stock drawing of both is skipped
//! (`engine::chat`).
use crate::engine::chat;
use crate::gfx::{fade, Weight, R};
use crate::widgets::{theme, Ui};
use alloc::string::String;

struct Seen {
    lines: usize,
    entry: String,
    caret_moved: f64,
    caret: usize,
}

static mut SEEN: Seen = Seen { lines: 0, entry: String::new(), caret_moved: 0.0, caret: 0 };

pub fn run(ui: &mut Ui) {
    let s = unsafe { &mut *core::ptr::addr_of_mut!(SEEN) };
    let lines = chat::lines();
    let entries = chat::entries();
    if lines.len() != s.lines {
        if lines.len() > s.lines {
            if let Some((l, _)) = lines.last() {
                crate::log!("chat: line \"{}\"", l);
            }
        }
        s.lines = lines.len();
    }
    let t = ui.theme;
    let x = ui.u(16.0);
    // Under the top bar (the resources' strip and the buttons, about 80 units).
    let mut y = ui.u(110.0);
    let px = ui.u(t.small_size);
    let lh = px * 1.3;
    let w = ui.u(640.0).min(ui.w * 0.5);
    if !lines.is_empty() {
        let h = lines.len() as f32 * lh + ui.u(12.0);
        // A faint backing so the lines read over bright space; no frame, no mouse.
        ui.draw.fill(R::new(x - ui.u(8.0), y - ui.u(6.0), w + ui.u(16.0), h), crate::gfx::Corners::round(ui.u(6.0)), fade(t.panel, 0.45));
        for (text, col) in &lines {
            let sh = ui.u(1.0).max(1.0);
            ui.draw.text_w(text, px, libm::roundf(x + sh), libm::roundf(y + sh), 0xc0000000, Weight::Regular);
            ui.draw.text_w(text, px, libm::roundf(x), libm::roundf(y), *col, Weight::Regular);
            y += lh;
        }
        y += ui.u(14.0);
    }
    let now = ui.time();
    for e in &entries {
        let key = crate::sfmt!("{}|{}", e.prompt, e.text);
        if key != s.entry {
            crate::log!("chat: entry {} \"{}\" \"{}\" caret {} at {},{}", if e.active { "active" } else { "shown" }, e.prompt, e.text, e.caret, e.x, e.y);
            s.entry = key;
        }
        if e.caret != s.caret {
            s.caret = e.caret;
            s.caret_moved = now;
        }
        let tpx = ui.u(t.text_size);
        let fh = ui.u(t.row_h + 4.0);
        let r = R::new(x - ui.u(8.0), y, w + ui.u(16.0), fh);
        let st = theme::State { focus: e.active, ..theme::State::default() };
        theme::field(ui, r, st);
        let pad = ui.u(t.pad * 0.75);
        let ty = r.y + (fh - tpx * 1.2) * 0.5;
        let mut tx = r.x + pad;
        if !e.prompt.is_empty() {
            tx += ui.draw.text_w(e.prompt.trim_end(), tpx, libm::roundf(tx), libm::roundf(ty), t.accent, Weight::Bold) + ui.u(8.0);
        }
        ui.draw.push_clip(R::new(tx, r.y, r.right() - pad - tx, r.h));
        ui.draw.text_w(&e.text, tpx, libm::roundf(tx), libm::roundf(ty), t.text, Weight::Regular);
        if e.active {
            let before: String = e.text.chars().take(e.caret).collect();
            let (cw, _) = crate::text::measure_with(crate::text::font(), &before, tpx);
            theme::caret(ui, tx + cw, r.y + fh * 0.2, fh * 0.6, (now - s.caret_moved) as f32);
        }
        ui.draw.pop_clip();
        y += fh + ui.u(6.0);
    }
    if entries.is_empty() && !s.entry.is_empty() {
        crate::log!("chat: entry closed");
        s.entry.clear();
    }
}
