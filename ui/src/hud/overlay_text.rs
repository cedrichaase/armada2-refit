//! The timed messages in the middle of the screen, ours (`[HUD] Overlay=1`; HUD.md,
//! "Overlay messages"): each stock `SingleOverlayText` at its own point (the HUD canvas
//! (`api::CANVAS`) scaled to the screen, as stock places it), justified as it was added,
//! in its own colour, faded as the stock panel fades it, in our type with a shadow.
use crate::engine::api::{self, buttons, hud, overlay_text as overlay};
use crate::gfx::{fade, Weight, R};
use crate::widgets::Ui;

/// The overlay messages, a panel of the table (`panels::PANELS`).
pub struct Overlay;

impl super::panels::Panel for Overlay {
    fn name(&self) -> &'static str {
        "overlay"
    }
    fn scope(&self) -> &'static str {
        "hud.overlay"
    }
    fn key(&self, c: &crate::config::Hud) -> bool {
        c.overlay
    }
    fn component(&self) -> (usize, &'static str) {
        (overlay::overlay(), "OverlayText")
    }
    fn draw(&self, ui: &mut Ui) {
        run(ui)
    }
}

/// The canvas rectangle (left, top, right, bottom) the engine's status line spans at any
/// aspect on Roots: its name or class at 625..974 x 50..69 at 4:3 and 669..930 at 21:9,
/// its icons and figures down to 117 (PLAN-FLEETOPS.md, decisions 460-469), rounded out.
const HOVER_BAND: [f32; 4] = [600.0, 45.0, 1000.0, 125.0];

static mut LAST_KEY: i32 = i32::MIN;
/// The last message logged as moved clear of the panels of ours.
static mut MOVED_KEY: i32 = i32::MIN;

pub fn run(ui: &mut Ui) {
    if hud::hidden(overlay::overlay()) {
        return;
    }
    let lines = overlay::lines();
    let origin = hud::panel_rect(overlay::overlay(), ui.w, ui.h).map_or((0.0, 0.0), |r| (r[0], r[1]));
    let (sx, sy) = (ui.w / api::CANVAS.0, ui.h / api::CANVAS.1);
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
        let widths: alloc::vec::Vec<f32> = rows.iter().map(|r| crate::text::measure_with(crate::text::font_bold(), r, px).0).collect();
        let place = |x0: f32, y0: f32, justify: i32| -> alloc::vec::Vec<R> {
            rows.iter().enumerate().map(|(i, _)| {
                let tw = widths[i];
                let x = if justify & 2 != 0 { x0 - tw * 0.5 } else if justify & 4 != 0 { x0 - tw } else { x0 };
                R::new(x.clamp(0.0, (ui.w - tw).max(0.0)), y0 + i as f32 * lh, tw, th)
            }).collect()
        };
        // Stock places a message against stock's panels; the panels of ours sit elsewhere
        // (Roots' mission name at the top left lands on our resource strip). A message
        // that would touch a panel of ours is centred across the screen at the same
        // height, and if that still touches one, moved down below it (HUD.md, "Overlay
        // messages"; PLAN-FLEETOPS.md, decision 326).
        let mut taken = super::layout::occupied();
        // The status line under the pointer (ours with `Hover=1`) shows only while the
        // pointer is over a craft, at the top centre where a relocated message lands: its
        // band is kept clear at all times, so the message does not jump when it shows
        // (decision 470).
        if api::hover::ours() {
            taken.push(R::new(ui.w * HOVER_BAND[0] / api::CANVAS.0, ui.h * HOVER_BAND[1] / api::CANVAS.1, ui.w * (HOVER_BAND[2] - HOVER_BAND[0]) / api::CANVAS.0, ui.h * (HOVER_BAND[3] - HOVER_BAND[1]) / api::CANVAS.1));
        }
        let gap = ui.u(t.gap).max(4.0);
        let hits = |rs: &[R]| -> Option<f32> {
            let mut low: Option<f32> = None;
            for r in rs {
                for o in &taken {
                    let o = o.inset(-gap);
                    if r.x < o.right() && o.x < r.right() && r.y < o.bottom() && o.y < r.bottom() {
                        low = Some(low.map_or(o.bottom(), |b: f32| b.max(o.bottom())));
                    }
                }
            }
            low
        };
        let mut rects = place(x0, y0, l.justify);
        let mut moved = false;
        if hits(&rects).is_some() {
            moved = true;
            rects = place(ui.w * 0.5, y0, (l.justify & !4) | 2);
            if let Some(b) = hits(&rects) {
                rects = place(ui.w * 0.5, b, (l.justify & !4) | 2);
            }
        }
        unsafe {
            if moved && l.key != MOVED_KEY {
                MOVED_KEY = l.key;
                crate::log!("overlay: #{} moved clear of the panels of ours, to {},{}", l.key, libm::roundf(rects[0].x) as i32, libm::roundf(rects[0].y) as i32);
            }
        }
        let col = fade(buttons::colour(l.colour), l.fade);
        let sh = ui.u(2.0).max(1.0);
        for (i, row) in rows.iter().enumerate() {
            let (x, y) = (rects[i].x, rects[i].y);
            ui.draw.text_w(row, px, libm::roundf(x + sh), libm::roundf(y + sh), fade(0xff000000, 0.8 * l.fade), Weight::Bold);
            ui.draw.text_w(row, px, libm::roundf(x), libm::roundf(y), col, Weight::Bold);
        }
    }
}
