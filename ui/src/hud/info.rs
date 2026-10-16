//! The info panel, ours (HUD.md, "The info panel"): what the selection is, drawn in
//! the player's theme at the bottom centre of the screen, laid out by its content.
//!
//! The stock `ShipDisplay` keeps running; ours asks it which face the selection needs
//! (`engine::info::face`) and reads each part through the stock panel's own child
//! components, so the words, numbers and colours are stock's. Its faces:
//!
//! - **one object** (`Single`): the wireframe (the game's art, each system's part
//!   coloured by its damage, yellow blinking while it is repaired), the name, the class
//!   and the race emblem; for a craft the hull bar and the shields, crew, officers and
//!   special energy, cargo or construction, and its five systems with their repair
//!   state; for a planet its population, owner and metal; for a moon or a nebula its
//!   resource. A click on it takes the camera there, as stock's does.
//! - **a builder** (`Builder`): the same, and its queue of ten, the first with its
//!   progress; a click on a queued item cancels it.
//! - **a group** (`Group`): an icon per selected unit with its hull; a click selects
//!   that one alone, Shift-click takes it out of the selection.
//!
//! The craft under the pointer in the 3D view gets a small card above the panel, as
//! stock's hover strip shows it. Every part has stock's tooltip.
use crate::engine::info::{self, BarKind, Face, Single, Text};
use crate::engine::{closeup, hud, sprite};
use crate::gfx::{fade, mix, Color, Corners, Weight, R};
use crate::widgets::{theme, Align, State, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// The panel's rectangle in the last frame it drew, and when (the bar is placed
/// against it).
static mut RECT: Option<(R, f64)> = None;

/// The rectangle the panel drew in, while it is drawing.
pub fn rect() -> Option<R> {
    unsafe {
        match RECT {
            Some((r, at)) if crate::rt::now() - at < 0.5 => Some(r),
            _ => None,
        }
    }
}

struct Hover {
    /// What the pointer is over: a component of stock's whose tooltip to show, or a
    /// unit (for a group icon) whose name to show.
    key: usize,
    since: f64,
    tip: Option<(bool, f64, String)>,
}

static mut HOVER: Hover = Hover { key: 0, since: 0.0, tip: None };
static mut LAST_LOG: String = String::new();

fn hv() -> &'static mut Hover {
    unsafe { &mut *core::ptr::addr_of_mut!(HOVER) }
}

/// What a frame's layout found under the pointer, for the tooltip.
#[derive(Clone, Copy)]
enum Tip {
    None,
    /// A stock component (its tooltip).
    Comp(usize, R),
    /// A unit in the group: its name and class.
    Unit(usize, R),
}

pub fn run(ui: &mut Ui) {
    // The whole panel at `[HUD] InfoScale`: its close-up, words, bars and systems alike.
    let scale = ui.scale;
    ui.scale *= crate::config::get().hud.info_scale;
    run_scaled(ui);
    ui.scale = scale;
}

fn run_scaled(ui: &mut Ui) {
    let hover_craft = info::take_hover();
    if hud::hidden(hud::SHIP_DISPLAY) || crate::engine::ingame::hud_hidden() {
        unsafe { RECT = None };
        return;
    }
    let face = info::face();
    let obj = info::current();
    let mut tip = Tip::None;
    let r = match face {
        Face::Single | Face::Builder if obj != 0 => {
            let s = info::single(obj);
            log_once(face, &s);
            Some(single(ui, &s, face == Face::Builder, &mut tip))
        }
        Face::Group => group(ui, &mut tip),
        _ => None,
    };
    unsafe { RECT = r.map(|r| (r, crate::rt::now())) };
    if hover_craft != 0 && hover_craft != obj {
        hover_card(ui, hover_craft, r);
    }
    tooltip(ui, tip, r);
}

fn log_once(face: Face, s: &Single) {
    let t = |x: &Option<Text>| x.as_ref().map_or(String::new(), |t| crate::sfmt!("{}{}", t.label, t.text));
    let mut line = crate::sfmt!("info: {:?} {} | {}", face, t(&s.name), t(&s.class));
    for st in &s.stats {
        line.push_str(&crate::sfmt!(" | {}{}", st.label, st.text));
    }
    if let Some(h) = &s.hull {
        line.push_str(&crate::sfmt!(" | hull {:.0}/{:.0}", h.value, h.max));
    }
    if let Some(t) = &s.shield_text {
        line.push_str(&crate::sfmt!(" | shield {}{}", t.label, t.text));
    }
    for (k, b) in &s.bars {
        line.push_str(&crate::sfmt!(" | {:?} {:.0}/{:.0}", k, b.value, b.max));
    }
    for sy in &s.systems {
        line.push_str(&crate::sfmt!(" | {}{}{}", sy.text.label, sy.text.text, if sy.state.repair_left > 0.01 { "(damaged)" } else { "" }));
    }
    #[allow(static_mut_refs)]
    let last = unsafe { &mut LAST_LOG };
    // Numbers change all the time: log a change of object or of parts only.
    let key = crate::sfmt!("{:?}{:#x}{}{}{}", face, s.obj, s.stats.len(), s.bars.len(), s.systems.len());
    if *last != key {
        crate::log!("{}", line);
        *last = key;
    }
}

// ---- drawing helpers ---------------------------------------------------------------------

/// A game sprite fitted into `r`, its aspect kept, tinted.
fn sprite_in(ui: &mut Ui, sp: usize, r: R, tint: Color) -> bool {
    let Some((tex, uv)) = sprite::art(sp) else { return false };
    let (tw, th) = crate::gfx::tex_size(tex);
    let (pw, ph) = ((uv[2] - uv[0]).abs() * tw as f32, (uv[3] - uv[1]).abs() * th as f32);
    if pw <= 0.0 || ph <= 0.0 {
        return false;
    }
    let s = (r.w / pw).min(r.h / ph);
    let (w, h) = (pw * s, ph * s);
    ui.draw.image_uv(tex, R::new(r.x + (r.w - w) * 0.5, r.y + (r.h - h) * 0.5, w, h), uv, tint);
    true
}

/// A stock colour that says something (red crew, a yellow system) kept; the plain
/// white and grey stock prints everything else in become the theme's text colour.
fn status_colour(ui: &Ui, c: [f32; 3]) -> Color {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    if max <= 0.0 || (max - min) / max < 0.3 {
        ui.theme.text
    } else {
        mix(info::colour(c), 0xffffffff, 0.15)
    }
}

fn text_w(ui: &Ui, s: &str, size: f32, w: Weight) -> f32 {
    let font = match w {
        Weight::Regular => crate::text::font(),
        Weight::Bold => crate::text::font_bold(),
    };
    crate::text::measure_with(font, s, ui.u(size)).0
}

/// One line of text in `r`, clipped to it.
fn line(ui: &mut Ui, r: R, s: &str, size: f32, c: Color, w: Weight, align: Align) {
    ui.draw.push_clip(r);
    ui.text_in(r, s, size, align, c, w);
    ui.draw.pop_clip();
}

/// The hull's colour: the theme's status colours by what is left.
fn hull_colour(ui: &Ui, f: f32) -> Color {
    let t = &ui.theme;
    if f > 0.6 {
        mix(t.warn, t.good, (f - 0.6) / 0.4)
    } else if f > 0.3 {
        mix(t.bad, t.warn, (f - 0.3) / 0.3)
    } else {
        t.bad
    }
}

/// A system's wireframe part or icon colour: the theme's text when intact, toward
/// `bad` with the damage, `warn` on the flash while it is repaired (stock: white, red,
/// yellow).
fn system_colour(ui: &Ui, st: &info::SystemState, flash: bool) -> Color {
    let t = &ui.theme;
    if st.repairing && flash {
        t.warn
    } else if st.repair_left > 0.01 {
        mix(t.text, t.bad, 0.45 + 0.55 * st.repair_left)
    } else {
        t.text
    }
}

/// The wireframe of a unit in `r`: each system's part in its colour, or the whole one.
fn wireframe(ui: &mut Ui, obj: usize, r: R) {
    let Some(w) = info::wire(obj) else { return };
    let flash = info::flash();
    if w.whole {
        let c = if info::whole_damaged(obj) { ui.theme.bad } else { ui.theme.text };
        sprite_in(ui, w.sprites[0], r, c);
        return;
    }
    for (i, &sp) in w.sprites.iter().enumerate() {
        if sp == 0 {
            continue;
        }
        let c = match info::system_state(obj, i) {
            Some(st) => system_colour(ui, &st, flash),
            None => ui.theme.text,
        };
        sprite_in(ui, sp, r, c);
    }
}

/// A craft live, close up, in `r` (`engine::closeup`, `[HUD] InfoCamera`): the picture
/// asked for this frame is drawn by the engine before the next, so the first frame of
/// a unit (or with the close-up unavailable) shows its wireframe. False when nothing
/// was drawn.
fn closeup(ui: &mut Ui, s: &Single, r: R) -> bool {
    if !closeup_on(s) {
        return false;
    }
    let px = [libm::roundf(r.x) as i32, libm::roundf(r.y) as i32, libm::roundf(r.w) as i32, libm::roundf(r.h) as i32];
    let Some(tex) = closeup::request(s.obj, px) else { return false };
    let at = R::new(px[0] as f32, px[1] as f32, px[2] as f32, px[3] as f32);
    let t = ui.theme;
    ui.draw.image(tex, at, 0xffffffff);
    // A soft vignette into the panel at the edges, and the panel's edge around it.
    let v = ui.u(10.0);
    let edge = fade(t.panel, 0.0);
    ui.draw.rect_vgrad(R::new(at.x, at.y, at.w, v), fade(t.panel, 0.55), edge);
    ui.draw.rect_vgrad(R::new(at.x, at.bottom() - v, at.w, v), edge, fade(t.panel, 0.55));
    ui.draw.frame(at, ui.u(t.edge).max(1.0), t.panel_edge);
    true
}

/// Whether `s` gets the close-up in place of its wireframe.
fn closeup_on(s: &Single) -> bool {
    crate::config::get().hud.info_camera && s.craft && !s.planet && closeup::available()
}

/// A bar in a row: its icon or name at the left, the bar, its value at the right.
fn bar_with(ui: &mut Ui, r: R, f: f32, col: Color, left: &str, right: &str) {
    bar_icon(ui, r, f, col, 0, left, right)
}

fn bar_icon(ui: &mut Ui, r: R, f: f32, col: Color, icon: usize, left: &str, right: &str) {
    let t = ui.theme;
    let has_icon = icon != 0 && sprite::art(icon).is_some();
    let lw = if has_icon { r.h + ui.u(6.0) } else if left.is_empty() { 0.0 } else { ui.u(BAR_LABEL) };
    let vw = ui.u(BAR_VALUE);
    if has_icon {
        sprite_in(ui, icon, R::new(r.x, r.y, r.h, r.h), t.text);
    } else if !left.is_empty() {
        line(ui, R::new(r.x, r.y, lw - ui.u(4.0), r.h), left, t.small_size, t.text_dim, Weight::Regular, Align::Left);
    }
    let bh = ui.u(BAR_H * 0.6).max(4.0);
    let br = R::new(r.x + lw, r.y + (r.h - bh) * 0.5, r.w - lw - vw, bh);
    theme::bar(ui, br, f, Some(col));
    if !right.is_empty() {
        line(ui, R::new(r.right() - vw, r.y, vw, r.h), right, t.small_size, t.text, Weight::Bold, Align::Right);
    }
}

/// `label` + `text` as stock prints them, with the label's trailing colon trimmed.
fn label_of(t: &Text) -> String {
    String::from(t.label.trim().trim_end_matches(':').trim())
}

// ---- one object ------------------------------------------------------------------------

const W_SINGLE: f32 = 600.0;
const WIRE: f32 = 150.0;
const BAR_H: f32 = 20.0;
const BAR_LABEL: f32 = 70.0;
const BAR_VALUE: f32 = 78.0;
const STAT_H: f32 = 24.0;
const SYS_H: f32 = 40.0;
const QUEUE_CELL: f32 = 52.0;

fn single(ui: &mut Ui, s: &Single, builder: bool, tip: &mut Tip) -> R {
    let t = ui.theme;
    let pad = ui.u(t.pad);
    let gap = ui.u(t.gap);
    // Heights, top to bottom, in pixels.
    let head = ui.u(t.heading_size + 4.0) + if s.class.is_some() { ui.u(t.small_size + 2.0) } else { 0.0 };
    let n_bars = s.hull.is_some() as usize + (s.shields.is_some() || s.shield_text.is_some()) as usize + s.bars.len();
    let bars_h = n_bars as f32 * (ui.u(BAR_H) + ui.u(4.0));
    let stats_h = if s.stats.is_empty() { 0.0 } else { ui.u(STAT_H) + gap };
    let right_h = head + gap + bars_h + stats_h;
    let top_h = right_h.max(ui.u(WIRE * if s.craft { 1.0 } else { 0.8 }));
    let sys_h = if s.systems.is_empty() { 0.0 } else { ui.u(SYS_H) + gap };
    let queue = if builder { info::queue(s.obj) } else { [(0, 0); info::QUEUE_LEN] };
    let queue_h = if builder { ui.u(QUEUE_CELL) + gap + ui.u(t.micro_size + 4.0) } else { 0.0 };
    let w = ui.u(W_SINGLE).min(ui.w - ui.u(20.0));
    // The frame's own bars (an LCARS frame's top and bottom) come on top of the padding.
    let (_, ft, _, fb) = theme::frame_margins(ui);
    let h = pad * 2.0 + top_h + sys_h + queue_h + ft + fb;
    let outer = place(ui, w, h);
    ui.panel(outer);
    let inner = theme::panel_inner(ui, outer);

    // The wireframe, at the left.
    let wire_side = ui.u(WIRE).min(top_h);
    let wr = R::new(inner.x, inner.y + (top_h - wire_side) * 0.5, wire_side, wire_side);
    if !closeup(ui, s, wr) {
        wireframe(ui, s.obj, wr.inset(ui.u(4.0)));
    }
    let (slowed, spy) = info::effects(s.obj);
    if slowed || spy {
        let (gs, rs) = info::effect_sprites();
        let isz = ui.u(26.0);
        if slowed {
            sprite_in(ui, gs, R::new(wr.x, wr.y, isz, isz), 0xffffffff);
        }
        if spy {
            sprite_in(ui, rs, R::new(wr.right() - isz, wr.y, isz, isz), 0xffffffff);
        }
    }
    let wire_id = ui.id("info.wire");
    let (wire_hot, _, _) = ui.interact(wire_id, wr);
    if wire_hot {
        *tip = Tip::Comp(info_wire_comp(), wr);
    }

    // The words, at the right of it.
    let x = wr.right() + gap * 1.5;
    let cw = inner.right() - x;
    let mut y = inner.y;
    let emblem = ui.u(34.0);
    let name_w = if s.race_icon != 0 { cw - emblem - gap } else { cw };
    if s.race_icon != 0 {
        sprite_in(ui, s.race_icon, R::new(inner.right() - emblem, y, emblem, emblem), 0xffffffff);
    }
    let name = s.name.as_ref().map(|n| n.text.clone()).filter(|n| !n.is_empty());
    let class = s.class.as_ref().map(|c| c.text.clone()).unwrap_or_default();
    let (title, sub) = match name {
        Some(n) => (n, class),
        None => (class, String::new()),
    };
    let title = if t.caps_titles { theme::caps(&title) } else { title };
    let tr = R::new(x, y, name_w, ui.u(t.heading_size + 4.0));
    line(ui, tr, &title, t.heading_size, t.title, Weight::Bold, Align::Left);
    if let Some(n) = s.name.as_ref().or(s.class.as_ref()) {
        let id = ui.id("info.name");
        if ui.interact(id, tr).0 {
            *tip = Tip::Comp(n.comp, tr);
        }
    }
    y += tr.h;
    if !sub.is_empty() {
        let cr = R::new(x, y, name_w, ui.u(t.small_size + 2.0));
        line(ui, cr, &sub, t.small_size, t.text_dim, Weight::Regular, Align::Left);
        if let Some(c) = &s.class {
            let id = ui.id("info.class");
            if ui.interact(id, cr).0 {
                *tip = Tip::Comp(c.comp, cr);
            }
        }
        y += cr.h;
    } else if s.class.is_some() {
        y += ui.u(t.small_size + 2.0);
    }
    y += gap;

    // Bars.
    let bh = ui.u(BAR_H);
    if let Some(hb) = &s.hull {
        let br = R::new(x, y, cw, bh);
        let f = hb.fraction();
        let txt = s.hull_text.as_ref();
        let left = txt.map_or(String::new(), |t| label_of(t));
        let right = txt.map_or(crate::sfmt!("{:.0}/{:.0}", hb.value, hb.max), |t| t.text.clone());
        bar_icon(ui, br, f, hull_colour(ui, f), txt.map_or(0, |t| t.icon), &left, &right);
        let id = ui.id("info.hull");
        if ui.interact(id, br).0 {
            *tip = Tip::Comp(hb.comp, br);
        }
        y += bh + ui.u(4.0);
    }
    if s.shields.is_some() || s.shield_text.is_some() {
        let br = R::new(x, y, cw, bh);
        let (now, max) = s.shields.unwrap_or((0.0, 0.0));
        let f = if max > 0.0 { (now / max).clamp(0.0, 1.0) } else { 0.0 };
        let txt = s.shield_text.as_ref();
        let left = txt.map_or(String::new(), |t| label_of(t));
        let right = txt.map_or(crate::sfmt!("{:.0}/{:.0}", now, max), |t| t.text.clone());
        let col = if s.shields_down { t.bad } else { t.fill };
        bar_icon(ui, br, if s.shields_down { 0.0 } else { f }, col, txt.map_or(0, |t| t.icon), &left, &right);
        if let Some(st) = txt {
            let id = ui.id("info.shield");
            if ui.interact(id, br).0 {
                *tip = Tip::Comp(st.comp, br);
            }
        }
        y += bh + ui.u(4.0);
    }
    for (i, (kind, b)) in s.bars.iter().enumerate() {
        let br = R::new(x, y, cw, bh);
        let f = b.fraction();
        let stock = info::colour(b.colour);
        let bright = (stock >> 16 & 0xff).max(stock >> 8 & 0xff).max(stock & 0xff) > 0x60;
        let col = match kind {
            BarKind::Construction => t.accent,
            _ if bright => mix(stock, t.fill, 0.25),
            _ => t.fill,
        };
        let value = match kind {
            BarKind::Construction => crate::sfmt!("{:.0}%", f * 100.0),
            _ => crate::sfmt!("{:.0}", b.value),
        };
        bar_with(ui, br, f, col, "", &value);
        let id = ui.id(&crate::sfmt!("info.bar{}", i));
        if ui.interact(id, br).0 {
            *tip = Tip::Comp(b.comp, br);
        }
        y += bh + ui.u(4.0);
    }

    // Stats: icon and value, in a row.
    if !s.stats.is_empty() {
        let sh = ui.u(STAT_H);
        let mut sx = x;
        for (i, st) in s.stats.iter().enumerate() {
            let label = label_of(st);
            let value = st.text.trim();
            let isz = sh;
            let has_icon = st.icon != 0 && sprite::art(st.icon).is_some();
            let lead = if has_icon { isz + ui.u(4.0) } else if !label.is_empty() { text_w(ui, &label, t.small_size, Weight::Regular) + ui.u(5.0) } else { 0.0 };
            let vw = text_w(ui, value, t.text_size, Weight::Bold);
            let item = R::new(sx, y, lead + vw + ui.u(4.0), sh);
            if item.right() > inner.right() {
                break;
            }
            if has_icon {
                sprite_in(ui, st.icon, R::new(sx, y, isz, isz), mix(info::colour(st.icon_colour), 0xffffffff, 0.2));
            } else if !label.is_empty() {
                line(ui, R::new(sx, y, lead, sh), &label, t.small_size, t.text_dim, Weight::Regular, Align::Left);
            }
            line(ui, R::new(sx + lead, y, vw + ui.u(2.0), sh), value, t.text_size, status_colour(ui, st.colour), Weight::Bold, Align::Left);
            let id = ui.id(&crate::sfmt!("info.stat{}", i));
            if ui.interact(id, item).0 {
                *tip = Tip::Comp(st.comp, item);
            }
            sx = item.right() + gap * 2.0;
        }
    }

    // Systems: a chip each along the bottom.
    let mut by = inner.y + top_h + gap;
    if !s.systems.is_empty() {
        let n = s.systems.len() as f32;
        let cwid = (inner.w - gap * (n - 1.0)) / n;
        let sh = ui.u(SYS_H);
        for (i, sy) in s.systems.iter().enumerate() {
            let r = R::new(inner.x + i as f32 * (cwid + gap), by, cwid, sh);
            let id = ui.id(&crate::sfmt!("info.sys{}", i));
            let (hot, _, _) = ui.interact(id, r);
            let damaged = sy.state.repair_left > 0.01;
            let blink = sy.state.repairing && sy.flash;
            let mut st = State::of(ui, id, hot, false);
            st.focus = blink;
            theme::field(ui, r, st);
            let col = system_colour(ui, &sy.state, sy.flash);
            let isz = sh - ui.u(10.0);
            let icon = if sy.text.icon != 0 { sy.text.icon } else { sy.repair_icon };
            let ir = R::new(r.x + ui.u(6.0), r.y + ui.u(5.0), isz, isz);
            if !sprite_in(ui, icon, ir, col) {
                line(ui, ir, &label_of(&sy.text), t.micro_size, col, Weight::Regular, Align::Left);
            }
            let vr = R::new(ir.right() + ui.u(4.0), r.y, r.right() - ir.right() - ui.u(8.0), r.h);
            let value = sy.text.text.trim();
            line(ui, vr, value, t.small_size, if damaged { col } else { status_colour(ui, sy.text.colour) }, Weight::Bold, Align::Right);
            if damaged {
                // What is repaired so far, along the chip's foot.
                let fr = R::new(r.x + ui.u(4.0), r.bottom() - ui.u(5.0), r.w - ui.u(8.0), ui.u(3.0));
                ui.draw.fill(fr, Corners::round(fr.h * 0.5), fade(t.track, 0.9));
                let done = R::new(fr.x, fr.y, fr.w * (1.0 - sy.state.repair_left).clamp(0.0, 1.0), fr.h);
                ui.draw.fill(done, Corners::round(fr.h * 0.5), if sy.state.repairing { t.warn } else { t.bad });
            }
            if hot {
                *tip = Tip::Comp(sy.text.comp, r);
            }
        }
        by += sh + gap;
    }

    // The builder's queue.
    if builder {
        let cell = ui.u(QUEUE_CELL);
        let n = info::QUEUE_LEN as f32;
        let qgap = ((inner.w - cell * n) / (n - 1.0)).max(ui.u(2.0));
        let progress = info::build_progress(s.obj).map(|b| b.fraction());
        for (i, &(class, comp)) in queue.iter().enumerate() {
            let r = R::new(inner.x + i as f32 * (cell + qgap), by, cell, cell);
            if class == 0 {
                ui.draw.fill(r, theme::button_corners(ui, r), fade(t.track, 0.5));
                continue;
            }
            let id = ui.id(&crate::sfmt!("info.queue{}", i));
            let (hot, held, clicked) = ui.interact(id, r);
            let mut st = State::of(ui, id, hot, held);
            st.on = i == 0;
            let c = theme::button_frame(ui, r, st);
            if let Some(w) = info::class_wire(class, info::drawn_whole(s.obj)) {
                for &sp in w.sprites.iter().filter(|&&sp| sp != 0) {
                    sprite_in(ui, sp, r.inset(ui.u(5.0)), c);
                }
            }
            if i == 0 {
                if let Some(f) = progress {
                    let pr = R::new(r.x, r.bottom() + ui.u(3.0), r.w, ui.u(t.micro_size * 0.5));
                    theme::bar(ui, pr, f, Some(t.accent));
                }
            }
            if hot {
                *tip = Tip::Comp(comp, r);
            }
            if clicked {
                crate::engine::sound::click();
                info::cancel(s.obj, i);
            }
        }
    }

    // A click anywhere else on the panel takes the camera to the object.
    let id = ui.id("info.panel");
    let (_, _, clicked) = ui.interact(id, outer);
    if clicked {
        info::look_at(s.obj);
    }
    outer
}

/// The wireframe component, for the wireframe's tooltip.
fn info_wire_comp() -> usize {
    // The stock wireframe's tooltip names the unit's state; it was handed the object
    // by `info::single`'s reads (the one-object face's `WireframeIcon`).
    crate::engine::info::wire_comp()
}

// ---- a group ---------------------------------------------------------------------------

const TILE_W: f32 = 70.0;
const TILE_H: f32 = 78.0;
const COLS: usize = 8;

fn group(ui: &mut Ui, tip: &mut Tip) -> Option<R> {
    let units = info::group();
    if units.is_empty() {
        return None;
    }
    let t = ui.theme;
    let pad = ui.u(t.pad);
    let gap = ui.u(5.0);
    let (tw, th) = (ui.u(TILE_W), ui.u(TILE_H));
    let max_cols = (((ui.w - ui.u(40.0) - 2.0 * pad + gap) / (tw + gap)) as usize).max(1);
    let cols = units.len().min(COLS.max(units.len().div_ceil(4))).min(max_cols).max(1);
    let rows = units.len().div_ceil(cols);
    // The tiles sit inside the frame (an LCARS frame's elbows and bars), padded.
    let (fl, ft, fr, fb) = theme::frame_margins(ui);
    let w = cols as f32 * tw + (cols - 1) as f32 * gap + 2.0 * pad + fl + fr;
    let h = rows as f32 * th + (rows - 1) as f32 * gap + 2.0 * pad + ft + fb;
    let outer = place(ui, w.max(ui.u(300.0)), h);
    ui.panel(outer);
    let inner = theme::panel_inner(ui, outer);
    let x0 = inner.x + (inner.w - (cols as f32 * tw + (cols - 1) as f32 * gap)) * 0.5;
    let shift = ui.input.shift || info::shift_held();
    for (i, &u) in units.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let tr = R::new(x0 + c as f32 * (tw + gap), inner.y + r as f32 * (th + gap), tw, th);
        let id = ui.id(&crate::sfmt!("info.unit{}", u));
        let (hot, held, clicked) = ui.interact(id, tr);
        let st = State::of(ui, id, hot, held);
        theme::field(ui, tr, st);
        let bar_h = ui.u(6.0);
        let wr = R::new(tr.x + ui.u(4.0), tr.y + ui.u(4.0), tr.w - ui.u(8.0), tr.h - bar_h - ui.u(12.0));
        wireframe(ui, u, wr);
        if let Some(b) = info::hull(u) {
            let br = R::new(tr.x + ui.u(5.0), tr.bottom() - bar_h - ui.u(5.0), tr.w - ui.u(10.0), bar_h);
            let f = b.fraction();
            theme::bar(ui, br, f, Some(hull_colour(ui, f)));
        }
        if hot {
            *tip = Tip::Unit(u, tr);
        }
        if clicked {
            crate::log!("info: group click, shift: ui {} game {}", ui.input.shift, info::shift_held());
            crate::engine::sound::click();
            info::select(u, shift);
        }
    }
    Some(outer)
}

// ---- the hover card and tooltips ------------------------------------------------------

/// The craft under the pointer in the 3D view: name, class, shields and hull, in a
/// small card beside the panel (stock's hover strip).
fn hover_card(ui: &mut Ui, obj: usize, panel: Option<R>) {
    let t = ui.theme;
    let (name, class) = info::name_and_class(obj);
    let (shield, hull) = info::hover_texts(obj);
    let mut lines: Vec<(String, f32, Color, Weight)> = Vec::new();
    if let Some(n) = name.filter(|n| !n.text.is_empty()) {
        lines.push((n.text, t.text_size, t.title, Weight::Bold));
    }
    if let Some(c) = class.filter(|c| !c.text.is_empty()) {
        lines.push((c.text, t.small_size, t.text_dim, Weight::Regular));
    }
    let hb = info::hull(obj);
    if lines.is_empty() && hb.is_none() {
        return;
    }
    // The content's size; the card is that plus the frame's own margins (an LCARS
    // frame's elbows take the left and the top and bottom bars), so no text runs
    // under the frame.
    let bar_h = ui.u(BAR_H);
    let n_bars = hb.is_some() as usize;
    let content_h = lines.iter().map(|l| ui.u(l.1 + 4.0)).sum::<f32>() + n_bars as f32 * (bar_h + ui.u(4.0));
    let probe = R::new(0.0, 0.0, ui.u(300.0), ui.u(300.0));
    let room = theme::panel_inner(ui, probe);
    let w = ui.u(300.0);
    let h = content_h + (probe.h - room.h);
    let (x, y) = match panel {
        // Beside the panel, at its right, when there is room; else above its right end
        // (the bar, when above the panel, sits at its left).
        Some(p) if ui.w - p.right() >= w + ui.u(16.0) => (p.right() + ui.u(8.0), p.bottom() - h),
        Some(p) => (p.right() - w, p.y - h - ui.u(8.0)),
        None => ((ui.w - w) * 0.5, ui.h - h - ui.u(10.0)),
    };
    let r = R::new(x.max(ui.u(4.0)), y.max(ui.u(4.0)), w, h);
    theme::panel(ui, r);
    let inner = theme::panel_inner(ui, r);
    let mut yy = inner.y;
    for (s, size, c, wt) in &lines {
        line(ui, R::new(inner.x, yy, inner.w, ui.u(size + 4.0)), s, *size, *c, *wt, Align::Left);
        yy += ui.u(size + 4.0);
    }
    if let Some(b) = hb {
        let f = b.fraction();
        let left = shield.map_or(String::new(), |s| crate::sfmt!("{}{}", s.label, s.text));
        let right = hull.map_or(crate::sfmt!("{:.0}/{:.0}", b.value, b.max), |h| crate::sfmt!("{}{}", h.label, h.text));
        bar_with(ui, R::new(inner.x, yy, inner.w, bar_h), f, hull_colour(ui, f), left.trim(), right.trim());
    }
}

fn tooltip(ui: &mut Ui, tip: Tip, panel: Option<R>) {
    let h = hv();
    let (key, at) = match tip {
        Tip::None => (0, R::default()),
        Tip::Comp(c, r) => (c, r),
        Tip::Unit(u, r) => (u, r),
    };
    let now = ui.time();
    if key != h.key {
        h.key = key;
        h.since = now;
        h.tip = None;
    }
    if key == 0 || now - h.since < 0.35 {
        return;
    }
    let verbose = now - h.since > 1.5;
    let stale = match &h.tip {
        Some((v, when, _)) => *v != verbose || now - when > 0.5,
        None => true,
    };
    if stale {
        let text = match tip {
            Tip::Comp(c, _) => info::tooltip(c, verbose).or_else(|| info::tooltip(c, false)).unwrap_or_default(),
            Tip::Unit(u, _) => {
                let (n, c) = info::name_and_class(u);
                let mut s = n.map_or(String::new(), |n| n.text);
                if let Some(c) = c.filter(|c| !c.text.is_empty()) {
                    if !s.is_empty() {
                        s.push('\n');
                    }
                    s.push_str(&c.text);
                }
                s
            }
            Tip::None => String::new(),
        };
        if h.tip.is_none() {
            crate::log!("info: tooltip {:#x}: {:?}", key, text);
        }
        h.tip = Some((verbose, now, clean(&text)));
    }
    let Some((_, _, text)) = &h.tip else { return };
    if text.is_empty() {
        return;
    }
    let text = text.clone();
    let maxw = ui.u(460.0);
    let (w, th) = theme::tooltip_size(ui, &text, maxw);
    let top = panel.map_or(at.y, |p| p.y.min(at.y));
    let x = at.x.min(ui.w - w - ui.u(4.0)).max(ui.u(4.0));
    let y = (top - th - ui.u(6.0)).max(ui.u(4.0));
    // Last of the frame, over the bar (which may sit above the panel).
    ui.overlay(move |ui| theme::tooltip(ui, R::new(x, y, w, th), &text, 1.0));
}

/// The game's tooltip strings mark a hotkey's letter with `^`.
fn clean(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if c != '^' && c != '\r' && c != '\0' {
            o.push(c);
        }
    }
    // Printf's escaped percent sign comes through as written.
    String::from(o.replace("%%", "%").trim())
}

// ---- placement --------------------------------------------------------------------------

/// The panel's place: bottom centre, above the screen's edge.
fn place(ui: &Ui, w: f32, h: f32) -> R {
    let m = ui.u(10.0);
    R::new(libm::roundf((ui.w - w) * 0.5), libm::roundf(ui.h - m - h), libm::roundf(w), libm::roundf(h))
}
