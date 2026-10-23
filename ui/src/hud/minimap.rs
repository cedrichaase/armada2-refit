//! The minimap, ours (`[HUD] Minimap=1`; HUD.md, "The minimap"): the stock radar's
//! state (`engine::radar`) drawn at the screen's resolution in our layout, bottom left.
//!
//! - the map's picture is the engine's own texture (`CacheMap` keeps it current), drawn
//!   over our square;
//! - the blips, the pings and the camera's outline are drawn by us from the radar's
//!   reads, sharp at any size: ships as dots, stations as squares, in their team's
//!   colour, planets and moons as their radar sprites, last-known positions faint;
//! - the fog of war and the shroud are masks of our own built from the scan grids, one
//!   texel per scan cell, rebuilt only when what the player sees changes;
//! - the stock filter buttons are two buttons of ours that press the stock ones'
//!   functions; their tooltips are the game's words.
//!
//! The left button (held) moves the camera, the right orders the selection to the point
//! (the buttons swap with the player's option), both through the radar's own engine
//! calls, made in the panel's always-step.
use crate::engine::radar::{self, Kind};
use crate::engine::{hud, sprite};
use crate::gfx::{fade, rgba, Color, Corners, Tex, Weight, R};
use crate::widgets::{Align, Ui};
use alloc::vec::Vec;

/// The shroud: the slate the 3D view's shroud is, so the two read as one.
const SHROUD: Color = rgba(44, 49, 62, 255);

struct Layer {
    tex: Option<Tex>,
    w: usize,
    h: usize,
    /// What the mask was built from last: the cells' visibility, packed.
    last: Vec<u32>,
    teams: u32,
    /// Whether the first cells seen have been logged.
    told: bool,
}

impl Layer {
    const fn new() -> Layer {
        Layer { tex: None, w: 0, h: 0, last: Vec::new(), teams: 0, told: false }
    }
}

struct State {
    map: Option<Tex>,
    warp: Option<Tex>,
    fog: Layer,
    shroud: Layer,
    /// A camera drag in progress, by button.
    drag: Option<usize>,
    logged: bool,
    tip_since: f64,
    tip_on: u8,
}

static mut STATE: State = State {
    map: None,
    warp: None,
    fog: Layer::new(),
    shroud: Layer::new(),
    drag: None,
    logged: false,
    tip_since: 0.0,
    tip_on: 0,
};

fn st() -> &'static mut State {
    unsafe { &mut *core::ptr::addr_of_mut!(STATE) }
}

/// Patches the radar's always-step for the clicks. False: the minimap stays stock.
pub fn install() -> bool {
    radar::install()
}

/// The whole panel: the map square and the filter strip above it.
pub fn place(ui: &Ui) -> (R, R, R) {
    let cfg = &crate::config::get().hud;
    let m = ui.u(10.0);
    let pad = ui.u(8.0);
    let side = ui.u(cfg.minimap_size).min(ui.h * 0.45).max(ui.u(120.0));
    let strip = ui.u(30.0);
    // Inside the frame (an LCARS frame's elbows and bars), padded.
    let (fl, ft, fr, fb) = crate::widgets::theme::frame_margins(ui);
    let w = side + 2.0 * pad + fl + fr;
    let h = side + 2.0 * pad + strip + ui.u(6.0) + ft + fb;
    let outer = R::new(m, ui.h - m - h, w, h);
    let strip_r = R::new(outer.x + fl + pad, outer.y + ft + pad, side, strip);
    let square = R::new(outer.x + fl + pad, outer.bottom() - fb - pad - side, side, side);
    (outer, strip_r, square)
}

fn colour3(c: [f32; 3], a: f32) -> Color {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    rgba(b(c[0]), b(c[1]), b(c[2]), b(a))
}

/// Rebuilds a layer's mask when what the player sees of it changed: a white texel,
/// opaque where the cell is hidden, clear where it is seen.
fn update_layer(l: &mut Layer, name: &str, g: &radar::Grid, teams: u32) {
    if l.tex.is_none() || l.w != g.w || l.h != g.h {
        l.tex = Some(crate::gfx::texture_dynamic_argb(g.w as u32, g.h as u32));
        l.w = g.w;
        l.h = g.h;
        l.last.clear();
    }
    let n = g.w * g.h;
    let words = n.div_ceil(32);
    let mut bits = alloc::vec![0u32; words];
    for (i, &c) in g.cells.iter().enumerate() {
        if c & teams != 0 {
            bits[i / 32] |= 1 << (i % 32);
        }
    }
    if bits == l.last && teams == l.teams {
        return;
    }
    let seen: u32 = bits.iter().map(|b| b.count_ones()).sum();
    if !l.told && seen > 0 {
        // Once: what the scenarios check the clearing by (the base's is a few percent
        // of a big map's cells, as stock's minimap shows it).
        crate::log!("minimap: {} {} of {} cells clear for teams {:#x}", name, seen, n, teams);
        l.told = true;
    }
    let mut px = alloc::vec![0u8; n * 4];
    for i in 0..n {
        let seen = bits[i / 32] & (1 << (i % 32)) != 0;
        px[i * 4..i * 4 + 4].copy_from_slice(&[255, 255, 255, if seen { 0 } else { 255 }]);
    }
    if let Some(t) = l.tex {
        crate::gfx::tex_update(t, px.as_ptr(), g.w * 4, g.w as u32, g.h as u32);
    }
    l.last = bits;
    l.teams = teams;
}

/// The grid's place in the square: centred in the layer texture, which covers the
/// square (`UpdateScanGridSprite`).
fn grid_rect(sq: R, g: &radar::Grid) -> R {
    let (tw, th) = radar::fog_texture_size();
    if tw == 0 || th == 0 || (tw as usize) < g.w || (th as usize) < g.h {
        return sq;
    }
    let (x0, y0) = ((tw as usize - g.w) / 2, (th as usize - g.h) / 2);
    let (cx, cy) = (sq.w / tw as f32, sq.h / th as f32);
    R::new(sq.x + x0 as f32 * cx, sq.y + y0 as f32 * cy, g.w as f32 * cx, g.h as f32 * cy)
}

fn ext_texture(slot: &mut Option<Tex>, st3d: usize) -> Option<Tex> {
    let t = *slot.get_or_insert_with(crate::gfx::texture_external);
    match radar::engine_texture(st3d) {
        Some((c, w, h)) => {
            crate::gfx::tex_set_external(t, Some(c), w, h);
            Some(t)
        }
        None => {
            crate::gfx::tex_set_external(t, None, 1, 1);
            None
        }
    }
}

pub fn run(ui: &mut Ui) {
    let s = st();
    let r = radar::radar();
    if r == 0 || hud::hidden(radar::MAP_RADAR) || crate::engine::ingame::hud_hidden() {
        s.drag = None;
        return;
    }
    let t = ui.theme;
    let (outer, strip, sq) = place(ui);
    ui.panel(outer);

    // The map: space, the engine's picture of the map's features, warp suppression.
    ui.draw.fill(sq, Corners::SQUARE, rgba(4, 6, 12, 255));
    ui.draw.push_clip(sq);
    if let Some(m) = ext_texture(&mut s.map, radar::map_texture()) {
        ui.draw.image(m, sq, 0xffffffff);
    }
    if let Some(w) = ext_texture(&mut s.warp, radar::warp_texture()) {
        ui.draw.image(w, sq, 0xffffffff);
    }
    let at = |u: f32, v: f32| (sq.x + u * sq.w, sq.y + v * sq.h);
    // A texel of the stock radar's 128-texel map, at our size.
    let tx = sq.w / 128.0;

    // Blips.
    let blips = radar::blips(r);
    for b in &blips {
        let (x, y) = at(b.u, b.v);
        if b.sprite != 0 {
            if let Some((tex, uv)) = sprite::art(b.sprite) {
                let sz = (b.size.max(4.0) * tx).max(ui.u(6.0));
                ui.draw.image_uv(tex, R::new(x - sz * 0.5, y - sz * 0.5, sz, sz), uv, if b.ghost { 0x80ffffff } else { 0xffffffff });
                continue;
            }
        }
        let a = if b.ghost { 0.45 } else { 1.0 };
        // A team with no map colour (a scripted map, the bench's scenes): its relation.
        let col = if b.colour.iter().sum::<f32>() < 0.1 {
            fade(match radar::relation(b.team) { 0 => t.good, 1 => t.accent2, 2 => t.bad, _ => t.warn }, a)
        } else {
            colour3(b.colour, a)
        };
        let dark = fade(0xff000000, 0.8 * a);
        match b.kind {
            Kind::Station => {
                let sz = (b.size * tx).max(ui.u(10.0));
                let rr = R::new(x - sz * 0.5, y - sz * 0.5, sz, sz);
                ui.draw.fill(rr.inset(-1.0), Corners::SQUARE, dark);
                ui.draw.fill(rr, Corners::SQUARE, col);
            }
            Kind::Ship => {
                let sz = (b.size * tx).max(ui.u(7.0));
                let rr = R::new(x - sz * 0.5, y - sz * 0.5, sz, sz);
                ui.draw.fill(rr.inset(-1.0), Corners::round(sz * 0.5), dark);
                ui.draw.fill(rr, Corners::round(sz * 0.5), col);
            }
            Kind::Other => {
                let sz = (b.size * tx).max(ui.u(3.0));
                ui.draw.fill(R::new(x - sz * 0.5, y - sz * 0.5, sz, sz), Corners::SQUARE, fade(col, 0.8));
            }
        }
        if b.selected {
            let sz = (b.size * tx).max(if b.kind == Kind::Station { ui.u(10.0) } else { ui.u(7.0) }) + ui.u(6.0);
            ui.draw.stroke(R::new(x - sz * 0.5, y - sz * 0.5, sz, sz), Corners::round(sz * 0.5), ui.u(1.2).max(1.0), 0xffffffff);
        }
    }

    // Fog of war and shroud, over the blips (fog dims, shroud hides).
    let (fog_on, shroud_on) = radar::fog_on();
    let teams = radar::teams_visible();
    if fog_on {
        if let Some(g) = radar::fog_grid() {
            update_layer(&mut s.fog, "fog", &g, teams);
            if let Some(tex) = s.fog.tex {
                ui.draw.image(tex, grid_rect(sq, &g), fade(SHROUD, 0.6));
            }
        }
    }
    if shroud_on {
        if let Some(g) = radar::shroud_grid() {
            update_layer(&mut s.shroud, "shroud", &g, teams);
            if let Some(tex) = s.shroud.tex {
                ui.draw.image(tex, grid_rect(sq, &g), SHROUD);
            }
        }
    }

    // Pings: shrinking rings.
    let stock_w = hud::panel_rect(radar::MAP_RADAR, ui.w, ui.h).map_or(sq.w, |p| (p[2] - p[0]).max(1.0));
    for m in radar::marks(r) {
        let (x, y) = at(m.u, m.v);
        let rad = (m.radius * sq.w / stock_w).max(ui.u(3.0));
        let rr = R::new(x - rad, y - rad, 2.0 * rad, 2.0 * rad);
        ui.draw.stroke(rr, Corners::round(rad), ui.u(2.0).max(1.0), colour3(m.colour, 1.0));
    }

    // What the camera sees.
    if let Some(q) = radar::view_outline(r) {
        let w = ui.u(1.5).max(1.0);
        for i in 0..4 {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            ui.draw.line(at(a.0, a.1), at(b.0, b.1), w, fade(0xffffffff, 0.85));
        }
    }
    // The tactical view: the point the camera turns about, a ring through the eye.
    if let Some((c, e)) = radar::tactical_camera(r) {
        let (cx, cy) = at(c.0, c.1);
        let (ex, ey) = at(e.0, e.1);
        let rad = libm::sqrtf((ex - cx) * (ex - cx) + (ey - cy) * (ey - cy)).max(ui.u(4.0));
        let w = ui.u(1.5).max(1.0);
        ui.draw.stroke(R::new(cx - rad, cy - rad, 2.0 * rad, 2.0 * rad), Corners::round(rad), w, fade(t.warn, 0.9));
        ui.draw.line((cx, cy), (ex, ey), w, fade(t.warn, 0.9));
    }
    ui.draw.pop_clip();
    ui.draw.stroke(sq, Corners::SQUARE, ui.u(1.0).max(1.0), fade(t.panel_edge, 0.8));

    filters(ui, strip);
    mouse(ui, sq);

    if !s.logged && !blips.is_empty() {
        s.logged = true;
        let (tw, th) = radar::fog_texture_size();
        let g = radar::fog_grid();
        crate::log!("minimap: ours at {:.0},{:.0} {:.0}px; map texture {}; fog grid {}x{} in a {}x{} texture; fog {} shroud {}; {} blips; clicks {}",
            sq.x, sq.y, sq.w,
            match radar::engine_texture(radar::map_texture()) { Some((_, w, h)) => crate::sfmt!("{}x{}", w, h), None => crate::sfmt!("none") },
            g.as_ref().map_or(0, |g| g.w), g.as_ref().map_or(0, |g| g.h), tw, th, fog_on, shroud_on, blips.len(),
            if radar::hooked() { "through the radar's always-step" } else { "off" });
    }
}

/// The two filters, ships and stations: each its name (the word, not the stock
/// buttons' 50 x 20 pictures, which do not read at this size) and the four states as
/// dots in the blips' colours (all, allies, enemies, neutral), the one in force lit. A
/// click on a dot picks that state (the stock button pressed as often as it takes); a
/// click on the name goes to the next, as the stock button does. The game's tooltips.
fn filters(ui: &mut Ui, strip: R) {
    let s = st();
    let t = ui.theme;
    let (ship_f, station_f) = radar::filters();
    let gap = ui.u(6.0);
    let gw = (strip.w - gap) * 0.5;
    // hovered: (filter 1 or 2, the state pointed at or 4 for the name)
    let mut hovered = (0u8, 0u8);
    let mut tip_rect = strip;
    for (i, state) in [ship_f, station_f].into_iter().enumerate() {
        let station = i == 1;
        let g = R::new(strip.x + i as f32 * (gw + gap), strip.y, gw, strip.h);
        crate::widgets::theme::field(ui, g, crate::widgets::theme::State::default());
        let d = ui.u(13.0).min(g.h - ui.u(8.0));
        let step = d + ui.u(4.0);
        let dots_w = 4.0 * step + ui.u(4.0);
        let name_r = R::new(g.x, g.y, g.w - dots_w, g.h);
        let id = ui.id(if station { "minimap.stations" } else { "minimap.ships" });
        let (hot, held, clicked) = ui.interact(id, name_r);
        let stt = crate::widgets::theme::State::of(ui, id, hot, held);
        let col = crate::gfx::mix(t.text_dim, t.text, stt.hot);
        let label = crate::widgets::theme::caps(radar::filter_name(station));
        ui.text_in(R::new(name_r.x + ui.u(7.0), name_r.y, name_r.w - ui.u(7.0), name_r.h), &label, t.micro_size, Align::Left, col, Weight::Bold);
        if clicked {
            radar::queue_filter(station, 1);
            crate::engine::sound::click();
        }
        if hot {
            hovered = (i as u8 + 1, 4);
            tip_rect = g;
        }
        for k in 0..4u32 {
            let cell = R::new(g.right() - ui.u(6.0) - (4 - k) as f32 * step + ui.u(4.0), g.y + (g.h - d) * 0.5, d, d);
            let id = ui.id(&crate::sfmt!("minimap.f{}.{}", i, k));
            let (hot, held, clicked) = ui.interact(id, cell);
            let on = state == k;
            let mark = match k {
                1 => t.good,
                2 => t.bad,
                3 => t.warn,
                _ => t.text,
            };
            if on {
                ui.draw.fill(cell.inset(-ui.u(2.0)), Corners::round(cell.w * 0.5 + ui.u(2.0)), fade(mark, 0.28));
            }
            let dot = if on { cell.inset(ui.u(1.0)) } else { cell.inset(ui.u(2.5)) };
            let a = if on { 1.0 } else if hot { 0.85 } else { 0.45 };
            if k == 0 {
                // all: a ring
                ui.draw.stroke(dot, Corners::round(dot.w * 0.5), ui.u(2.0).max(1.0), fade(mark, a));
            } else {
                ui.draw.fill(dot, Corners::round(dot.w * 0.5), fade(mark, a));
            }
            if clicked && !on && !held {
                radar::queue_filter(station, ((k + 4 - state) % 4) as u8);
                crate::engine::sound::click();
            }
            if hot {
                hovered = (i as u8 + 1, k as u8);
                tip_rect = cell;
            }
        }
    }
    let now = ui.time();
    let key = hovered.0 * 8 + hovered.1;
    if key != s.tip_on {
        s.tip_on = key;
        s.tip_since = now;
    }
    if hovered.0 != 0 && ui.waited(s.tip_since, 0.35) {
        let station = hovered.0 == 2;
        let current = if station { station_f } else { ship_f };
        let shown = if hovered.1 == 4 { current } else { hovered.1 as u32 };
        let (tip, state) = radar::filter_text(station, shown);
        let text = if hovered.1 == 4 { crate::sfmt!("{}\n{}", tip.trim(), state.trim()) } else { alloc::string::String::from(state.trim()) };
        let (w, h) = crate::widgets::theme::tooltip_size(ui, &text, ui.u(360.0));
        let r = R::new(tip_rect.x, (tip_rect.y - h - ui.u(6.0)).max(0.0), w, h);
        crate::widgets::theme::tooltip(ui, r, &text, 1.0);
    }
}

/// Clicks and drags on the map, queued for the radar's always-step.
fn mouse(ui: &mut Ui, sq: R) {
    let s = st();
    let inp = ui.input;
    let id = ui.id("minimap.map");
    let _ = ui.interact(id, sq);
    let (cam_b, cmd_b) = if radar::buttons_swapped() { (1, 0) } else { (0, 1) };
    let Some((mx, my)) = inp.mouse else {
        s.drag = None;
        return;
    };
    let over = sq.contains((mx, my));
    let uv = (((mx - sq.x) / sq.w).clamp(0.0, 1.0), ((my - sq.y) / sq.h).clamp(0.0, 1.0));
    let tactical = radar::view() == 1;
    if let Some(b) = s.drag {
        if !inp.down[b] {
            s.drag = None;
        } else if !tactical {
            radar::queue_camera(uv.0, uv.1, false);
        }
    }
    if !over {
        return;
    }
    if inp.pressed[cmd_b] {
        if radar::selection_count() > 0 {
            radar::queue_order(uv.0, uv.1);
        } else if !tactical {
            s.drag = Some(cmd_b);
            radar::queue_camera(uv.0, uv.1, false);
        }
    }
    if inp.pressed[cam_b] {
        radar::queue_camera(uv.0, uv.1, tactical);
        if !tactical {
            s.drag = Some(cam_b);
        }
    }
}
