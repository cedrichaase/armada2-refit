//! The timed messages in the middle of the screen, ours (`[HUD] Overlay=1`; HUD.md,
//! "Overlay messages"): each stock `SingleOverlayText` at its own point (the 1600 x
//! 1200 space scaled to the screen, as stock places it), justified as it was added,
//! in its own colour, faded as the stock panel fades it, in our type with a shadow.
use crate::engine::{buttons, hud, overlay_text as overlay};
use crate::gfx::{fade, Weight};
use crate::widgets::Ui;

static mut LAST_KEY: i32 = i32::MIN;

pub fn run(ui: &mut Ui) {
    if hud::hidden(overlay::OVERLAY) {
        return;
    }
    let lines = overlay::lines();
    let origin = hud::panel_rect(overlay::OVERLAY, ui.w, ui.h).map_or((0.0, 0.0), |r| (r[0], r[1]));
    let (sx, sy) = (ui.w / 1600.0, ui.h / 1200.0);
    let t = ui.theme;
    let px = ui.u(t.title_size);
    for l in &lines {
        unsafe {
            if l.key > LAST_KEY {
                crate::log!("overlay: #{} at {},{} justify {} \"{}\"", l.key, l.x, l.y, l.justify, l.text);
                LAST_KEY = l.key;
            }
        }
        if l.text.is_empty() || l.fade <= 0.01 {
            continue;
        }
        // A message may hold several lines (the intro's "USS Enterprise\nStardate ...\n
        // Location ..."): each is placed by the justification on its own, the block as
        // a whole centred down the point when it is.
        let rows: alloc::vec::Vec<&str> = l.text.split('\n').map(|r| r.trim_end_matches('\r')).collect();
        let (_, th) = crate::text::measure_with(crate::text::font_bold(), "Ag", px);
        let lh = libm::roundf(th * 1.15);
        let block = lh * (rows.len() as f32 - 1.0) + th;
        let x0 = origin.0 + l.x as f32 * sx;
        let y0 = origin.1 + l.y as f32 * sy;
        // eJustify is flags: 2 centres the text on the point across, 16 down (the
        // intro's lines are added with 18 at x 800, the mission's name with 0 at its
        // corner); 4 is taken as right-aligned.
        let y0 = if l.justify & 16 != 0 { y0 - block * 0.5 } else { y0 };
        let col = fade(buttons::colour(l.colour), l.fade);
        let sh = ui.u(2.0).max(1.0);
        for (i, row) in rows.iter().enumerate() {
            let (tw, _) = crate::text::measure_with(crate::text::font_bold(), row, px);
            let x = if l.justify & 2 != 0 {
                x0 - tw * 0.5
            } else if l.justify & 4 != 0 {
                x0 - tw
            } else {
                x0
            };
            let x = x.clamp(0.0, (ui.w - tw).max(0.0));
            let y = y0 + i as f32 * lh;
            ui.draw.text_w(row, px, libm::roundf(x + sh), libm::roundf(y + sh), fade(0xff000000, 0.8 * l.fade), Weight::Bold);
            ui.draw.text_w(row, px, libm::roundf(x), libm::roundf(y), col, Weight::Bold);
        }
    }
}
