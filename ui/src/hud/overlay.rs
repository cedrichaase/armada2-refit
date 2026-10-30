//! The selection overlay, ours (`[HUD] Selection=1`; HUD.md, "The selection overlay"):
//! what `SelectionDisplay` marks over the units in the 3D view, drawn by us at the
//! screen's resolution: thin corner brackets around each selected unit and the one under
//! the pointer, its hull, shield and special-energy gauges, its group number, and the
//! pointed unit's name. Where each unit is on screen is the engine's own answer
//! (`engine::overlay`, `ComputeContainerRect`), so the brackets sit where stock's
//! gauges sit, whatever camera or renderer draws the scene.
//!
//! Drawn first among the panels of ours, so they cover it, and not over a stock panel
//! that is still drawn (a unit whose middle is under one is left out).
use crate::engine::{hud, overlay, radar};
use crate::gfx::{fade, mix, Corners, Weight, R};
use crate::widgets::{Align, Ui};

/// The stock panels the overlay stays out of, while they are drawn.
const PANELS: [&str; 8] = ["ShipDisplay", "MapRadar", "PopupPalette", "ResourcePanel", "ButtonPanel", "CinematicView", "ObjectivesDisplay", "MessageDisplay"];

pub fn run(ui: &mut Ui, keep_out: &[R]) {
    if crate::engine::ingame::hud_hidden() || radar::view() == 2 {
        return;
    }
    let units = overlay::units(ui.w, ui.h);
    if units.is_empty() {
        return;
    }
    // The stock panels still drawn, in back-buffer pixels.
    let mut out: alloc::vec::Vec<R> = keep_out.into();
    for c in hud::components() {
        if hud::hidden(c) || hud::is_taken(c) || !PANELS.contains(&hud::name(c)) {
            continue;
        }
        if let Some(p) = hud::panel_rect(c, ui.w, ui.h) {
            out.push(R::new(p[0], p[1], p[2] - p[0], p[3] - p[1]));
        }
    }
    let gauges_on_selection = crate::config::get().hud.selection_gauges;
    let t = ui.theme;
    for u in &units {
        let mut r = R::new(u.rect[0], u.rect[1], u.rect[2], u.rect[3]);
        // At least a bracket's worth across, about its middle.
        let min = ui.u(26.0);
        if r.w < min {
            r = R::new(r.x - (min - r.w) * 0.5, r.y, min, r.h);
        }
        if r.h < min {
            r = R::new(r.x, r.y - (min - r.h) * 0.5, r.w, min);
        }
        let c = r.centre();
        if out.iter().any(|p| p.contains(c)) || c.0 < 0.0 || c.1 < 0.0 || c.0 > ui.w || c.1 > ui.h {
            continue;
        }
        let rel = radar::relation(u.team);
        let col = match rel {
            0 => t.accent,
            1 => t.good,
            2 => t.bad,
            _ => t.warn,
        };
        let col = if u.pointed { mix(col, 0xffffffff, 0.35) } else { col };
        brackets(ui, r, col, u.pointed);

        let show = u.gauges || u.pointed || (u.selected && gauges_on_selection);
        let gw = r.w.clamp(ui.u(44.0), ui.u(140.0));
        let mut y = r.y - ui.u(4.0);
        let x = c.0 - gw * 0.5;
        if show {
            let bh = ui.u(5.0).max(3.0);
            if let Some(f) = u.special {
                y -= ui.u(3.0).max(2.0) + ui.u(1.5);
                gauge(ui, R::new(x, y, gw, ui.u(3.0).max(2.0)), f, t.accent3);
            }
            if let Some(f) = u.shields {
                y -= ui.u(3.0).max(2.0) + ui.u(1.5);
                gauge(ui, R::new(x, y, gw, ui.u(3.0).max(2.0)), f, mix(t.accent2, 0xff5aa0ff, 0.5));
            }
            if let Some(f) = u.hull {
                y -= bh + ui.u(1.5);
                let hc = if f > 0.6 { t.good } else if f > 0.3 { t.warn } else { t.bad };
                gauge(ui, R::new(x, y, gw, bh), f, hc);
            }
        }
        // The damaged systems, under the brackets: where stock lays its row of system
        // icons out under the gauges, for every unit whose gauges show (with
        // `SelectionGauges=1` the selection's too) (`[HUD] SystemIcons`).
        if show && u.craft && crate::config::get().hud.system_icons {
            systems(ui, u.obj, (c.0, r.bottom() + ui.u(5.0)));
        }
        if let Some(g) = u.group {
            let d = [b'0' + g];
            let s = core::str::from_utf8(&d).unwrap_or("?");
            let sz = ui.u(16.0);
            let br = R::new(r.x - sz - ui.u(3.0), r.y, sz, sz);
            ui.draw.fill(br, Corners::round(ui.u(3.0)), fade(0xff000000, 0.75));
            ui.draw.stroke(br, Corners::round(ui.u(3.0)), ui.u(1.0).max(1.0), col);
            ui.text_in(br, s, t.micro_size, Align::Centre, t.text, Weight::Bold);
        }
        if u.pointed {
            let name = overlay::name(u.obj);
            if !name.is_empty() {
                let h = ui.u(18.0);
                let nr = R::new(c.0 - ui.u(150.0), y - h - ui.u(2.0), ui.u(300.0), h);
                ui.text_in(R::new(nr.x + 1.0, nr.y + 1.0, nr.w, nr.h), name, t.small_size, Align::Centre, 0xc0000000, Weight::Bold);
                ui.text_in(nr, name, t.small_size, Align::Centre, t.text, Weight::Bold);
            }
        }
    }
}

/// The craft's damaged systems in a row centred on `top` (its middle and top): each
/// the system's icon on a dark chip, in `bad` while it is out, `warn` on the flash
/// while it is repaired, else by its strength as the hull gauge is coloured, with a bar
/// under it: what is repaired so far while it is out, else its strength. A system at
/// full strength, not out and not repaired is left out (stock draws it green).
fn systems(ui: &mut Ui, obj: usize, top: (f32, f32)) {
    let t = ui.theme;
    let shown: alloc::vec::Vec<_> = overlay::systems(obj)
        .into_iter()
        .filter(|s| s.strength < 0.995 || s.state.repair_left > 0.01 || s.state.repairing)
        .collect();
    log_systems(obj, &shown);
    if shown.is_empty() {
        return;
    }
    let (sz, gap) = (ui.u(18.0), ui.u(3.0));
    let w = shown.len() as f32 * sz + (shown.len() - 1) as f32 * gap;
    let mut x = top.0 - w * 0.5;
    for s in &shown {
        let out = s.state.repair_left > 0.01;
        let col = if s.state.repairing && s.flash {
            t.warn
        } else if out {
            t.bad
        } else if s.strength > 0.6 {
            t.good
        } else if s.strength > 0.3 {
            t.warn
        } else {
            t.bad
        };
        let chip = R::new(libm::floorf(x), libm::floorf(top.1), sz, sz);
        ui.draw.fill(chip, Corners::round(ui.u(3.0)), fade(0xff000000, 0.72));
        ui.draw.stroke(chip, Corners::round(ui.u(3.0)), ui.u(1.0).max(1.0), fade(col, 0.85));
        let ir = chip.inset(ui.u(3.0));
        if !sprite_in(ui, s.sprite, ir, col) {
            ui.draw.fill(ir.inset(ui.u(3.0)), Corners::round(ui.u(2.0)), col);
        }
        let f = if out { 1.0 - s.state.repair_left } else { s.strength };
        let bar = R::new(chip.x, chip.bottom() + ui.u(1.5), sz, ui.u(2.0).max(2.0));
        ui.draw.fill(bar, Corners::SQUARE, fade(t.track, 0.9));
        ui.draw.fill(R::new(bar.x, bar.y, bar.w * f.clamp(0.0, 1.0), bar.h), Corners::SQUARE, col);
        x += sz + gap;
    }
}

/// A game sprite fitted into `r`, its aspect kept, tinted; false without its art.
fn sprite_in(ui: &mut Ui, sp: usize, r: R, tint: u32) -> bool {
    let Some((tex, uv)) = crate::engine::sprite::art(sp) else { return false };
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

/// UI.log says which systems are shown when that changes ("overlay: systems of
/// Galaxy: 2 out 40%, 0 62%"): the bench reads it.
fn log_systems(obj: usize, shown: &[overlay::SystemIcon]) {
    // The last row logged, per unit (a few units at once show their rows).
    static mut LAST: [(usize, u64); 8] = [(0, 0); 8];
    let mut key: u64 = 1;
    for s in shown {
        key = key.wrapping_mul(31).wrapping_add(s.system as u64 * 8 + (s.state.repair_left > 0.01) as u64 * 4 + s.state.repairing as u64 + 1);
    }
    unsafe {
        let last = &mut *core::ptr::addr_of_mut!(LAST);
        let i = last.iter().position(|e| e.0 == obj).unwrap_or_else(|| {
            last.rotate_right(1);
            last[0] = (obj, 1);
            0
        });
        if last[i].1 == key {
            return;
        }
        last[i].1 = key;
    }
    if shown.is_empty() {
        return;
    }
    let mut line = alloc::string::String::new();
    for s in shown {
        line.push_str(&crate::sfmt!(" {}{}{} {:.0}%", s.system, if s.state.repair_left > 0.01 { " out" } else { "" }, if s.state.repairing { " repairing" } else { "" }, s.strength * 100.0));
    }
    crate::log!("overlay: systems of {}:{}", overlay::name(obj), line);
}

/// Four corner brackets around `r`.
fn brackets(ui: &mut Ui, r: R, col: u32, strong: bool) {
    let len = (r.w.min(r.h) * 0.28).clamp(ui.u(6.0), ui.u(20.0));
    let w = ui.u(if strong { 2.0 } else { 1.5 }).max(1.0);
    let shadow = fade(0xff000000, 0.45);
    for (x, y, sx, sy) in [(r.x, r.y, 1.0, 1.0), (r.right(), r.y, -1.0, 1.0), (r.x, r.bottom(), 1.0, -1.0), (r.right(), r.bottom(), -1.0, -1.0)] {
        let hx = if sx > 0.0 { x } else { x - len };
        let vy = if sy > 0.0 { y } else { y - len };
        let hy = if sy > 0.0 { y } else { y - w };
        let vx = if sx > 0.0 { x } else { x - w };
        for (o, c) in [(1.0, shadow), (0.0, col)] {
            ui.draw.rect(R::new(hx + o, hy + o, len, w), c);
            ui.draw.rect(R::new(vx + o, vy + o, w, len), c);
        }
    }
}

/// A thin gauge: a dark track with a hairline, filled to `f`.
fn gauge(ui: &mut Ui, r: R, f: f32, col: u32) {
    ui.draw.fill(r.inset(-1.0), Corners::SQUARE, fade(0xff000000, 0.7));
    ui.draw.fill(r, Corners::SQUARE, fade(ui.theme.track, 0.9));
    if f > 0.0 {
        ui.draw.fill_v(R::new(r.x, r.y, r.w * f.clamp(0.0, 1.0), r.h), Corners::SQUARE, mix(col, 0xffffffff, 0.25), col);
    }
}
