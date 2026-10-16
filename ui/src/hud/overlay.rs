//! The timed messages in the middle of the screen, ours (`[HUD] Overlay=1`; HUD.md,
//! "Overlay messages"): each stock `SingleOverlayText` at its own point (the 1600 x
//! 1200 space scaled to the screen, as stock places it), justified as it was added,
//! in its own colour, faded as the stock panel fades it, in our type with a shadow.
use crate::engine::{buttons, hud, overlay};
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
        let (tw, th) = crate::text::measure_with(crate::text::font_bold(), &l.text, px);
        let x = origin.0 + l.x as f32 * sx;
        let y = origin.1 + l.y as f32 * sy;
        // eJustify is flags: 2 centres the text on the point across, 16 down (the
        // intro's lines are added with 18 at x 800, the mission's name with 0 at its
        // corner); 4 is taken as right-aligned.
        let x = if l.justify & 2 != 0 {
            x - tw * 0.5
        } else if l.justify & 4 != 0 {
            x - tw
        } else {
            x
        };
        let x = x.clamp(0.0, (ui.w - tw).max(0.0));
        let y = if l.justify & 16 != 0 { y - th * 0.5 } else { y };
        let col = fade(buttons::colour(l.colour), l.fade);
        let sh = ui.u(2.0).max(1.0);
        ui.draw.text_w(&l.text, px, libm::roundf(x + sh), libm::roundf(y + sh), fade(0xff000000, 0.8 * l.fade), Weight::Bold);
        ui.draw.text_w(&l.text, px, libm::roundf(x), libm::roundf(y), col, Weight::Bold);
    }
}
