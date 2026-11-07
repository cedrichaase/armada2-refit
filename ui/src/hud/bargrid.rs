//! The button bar of ours on Fleet Operations: Roots as the grid (PLAN-FLEETOPS.md, FO-6,
//! decisions 330 to 349): the stock buttons in `bar`'s 5x3 plan of position keys, and the
//! buttons the plan has no room for (Roots' bar holds up to 64) in columns of three beside it,
//! without keys.
//!
//! FleetOpsHook's range ring, the tooltip and the click all follow the stock button under the
//! pointer, which `StandardComponent::Simulate` finds by the button's own rectangle while the bar
//! owns the pointer. So the grid moves the stock buttons onto its cells rather than drawing cells
//! of its own over a bar that stays elsewhere: right after each update of the bar has laid the
//! buttons out (`after_layout`, from the bar's call of `IsGamePausedAtAll`, `hud.bar_grid`), each
//! shown button's rectangle becomes its cell's, and the bar is given the pointer over the grid's
//! frame (`hud::set_bar_area`) and nowhere else. The stock bar keeps simulating, owns the pointer,
//! shows its tooltip, plays its sounds and presses; its drawing is skipped (`set_bar_skin`). The
//! cells are drawn from the buttons' own bytes (hover `+0x38`, pressed `+0x30`, state `+0x34`), as
//! the skin draws them, and nothing is claimed: no `interact`, no claimed panel. The keys are
//! QOL's (`UI_BarCell`, `QOL_BarLabel`), as on Armada II; a cell without a key on this build
//! (Roots: W) shows no label.
use super::bar::{self, CELLS, COLS, LABEL, ROWS};
use crate::engine::api::{hud, palette, sprite, CANVAS};
use crate::gfx::{fade, Weight, R};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// The grid's place as the last UI frame drew it, in back-buffer pixels: the first cell's top
/// left, the cell's size and the gap, and canvas units per pixel. What `after_layout` turns the
/// cells into the buttons' rectangles with.
#[derive(Clone, Copy)]
struct Geom {
    at: f64,
    gx: f32,
    gy: f32,
    cs: f32,
    gap: f32,
    kx: f32,
    ky: f32,
}

struct State {
    on: bool,
    geom: Option<Geom>,
    /// The layout of the bar's last update (`after_layout`), what is drawn while it is fresh.
    cells: [usize; CELLS],
    over: Vec<usize>,
    built_at: f64,
    /// The overflow columns the last frame drew (the bar's size for the bottom row).
    over_cols: usize,
    last_log: String,
    last_rect: [i32; 4],
}

static mut STATE: State = State { on: false, geom: None, cells: [0; CELLS], over: Vec::new(), built_at: -1.0, over_cols: 0, last_log: String::new(), last_rect: [0; 4] };

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

/// Puts the grid's hook in (`hud.bar_grid`). False: the skin is drawn instead.
pub fn install() -> bool {
    let ok = palette::install_grid(after_layout);
    st().on = ok;
    ok
}

/// Whether the bar of ours is the grid (else the skin).
pub fn on() -> bool {
    st().on
}

/// Whether the pointer is over the grid and its position keys are on (the tooltips drop the
/// bar's own hotkey then).
pub fn keys_under_pointer() -> bool {
    let s = st();
    s.on && s.geom.is_some() && bar::qol_label(0).is_some() && hud::pointer_over_bar()
}

/// The cell's column and row: the fifteen keyed cells (`bar`'s plan), then the overflow, `k`
/// after the fifteenth, down columns of three to the right.
fn col_row(cell: usize) -> (usize, usize) {
    if cell < CELLS { (cell % COLS, cell / COLS) } else { (COLS + (cell - CELLS) / ROWS, (cell - CELLS) % ROWS) }
}

/// A cell's rectangle in pixels. The overflow columns are set off by a gap's width more.
fn cell_rect(g: &Geom, cell: usize) -> R {
    let (c, r) = col_row(cell);
    let sep = if c >= COLS { g.gap * 2.0 } else { 0.0 };
    R::new(g.gx + c as f32 * (g.cs + g.gap) + sep, g.gy + r as f32 * (g.cs + g.gap), g.cs, g.cs)
}

/// The bar's update has laid the buttons out (the game's thread, once per update): each shown
/// button goes to its cell. Nothing while the grid is not drawn: the buttons keep the layout's
/// own rectangles, and `cursor_over` the bar's own answer.
pub fn after_layout(p: usize) {
    let s = st();
    let Some(g) = s.geom else { return };
    if crate::rt::now() - g.at > 0.5 || hud::hidden(p) || crate::engine::api::ingame::hud_hidden() {
        return;
    }
    let Some((ox, oy)) = palette::origin(p) else { return };
    let (cells, _, over) = bar::layout_all(p, palette::menu(p));
    let place = |b: usize, cell: usize| {
        let r = cell_rect(&g, cell);
        let l = libm::floorf(r.x * g.kx) as i32;
        let t = libm::floorf(r.y * g.ky) as i32;
        let rr = libm::floorf(r.right() * g.kx) as i32 - 1;
        let bb = libm::floorf(r.bottom() * g.ky) as i32 - 1;
        palette::set_rect(b, [l - ox, t - oy, rr.max(l) - ox, bb.max(t) - oy]);
    };
    for (c, &b) in cells.iter().enumerate() {
        if b != 0 {
            place(b, c);
        }
    }
    for (k, &b) in over.iter().enumerate() {
        place(b, CELLS + k);
    }
    // The first cell's rectangle as written, once per change: a bench run shows where the
    // buttons were moved to, beside the hover line's pointer (decision 348).
    let first = cell_rect(&g, 0);
    let sig = [libm::floorf(first.x * g.kx) as i32 - ox, libm::floorf(first.y * g.ky) as i32 - oy, ox, oy];
    if s.last_rect != sig {
        s.last_rect = sig;
        crate::log!("bar: grid cells moved: Q at [{},{}] local to the bar's origin {},{} (canvas)", sig[0], sig[1], ox, oy);
    }
    bar::set_cells(cells, false);
    s.cells = cells;
    s.over = over;
    s.built_at = crate::rt::now();
}

/// The grid's outer size in pixels for `n` overflow columns, and its cell size and gap (the
/// cells shrink when the overflow would run off the screen).
fn size(ui: &Ui, over_cols: usize) -> (f32, f32, f32, f32, f32) {
    // In the bottom row (`row`, Roots) the cell is the row's: three rows as high as the info panel.
    size_with(ui, over_cols, super::row::bar().map_or(ui.u(62.0), |b| b.1))
}

/// The grid's outer size with no overflow for a cell of `cs` pixels (the bottom row's plan).
pub fn size_for(ui: &Ui, cs: f32) -> (f32, f32) {
    let (w, h, ..) = size_with(ui, st().over_cols, cs);
    (w, h)
}

fn size_with(ui: &Ui, over_cols: usize, cs: f32) -> (f32, f32, f32, f32, f32) {
    let k = cs / ui.u(62.0);
    let (mut cs, mut gap, pad) = (cs, ui.u(5.0) * k.min(1.0), ui.u(8.0) * k.min(1.0));
    let (fl, ft, fr, fb) = theme::frame_margins(ui);
    let cols = (COLS + over_cols) as f32;
    let sep = if over_cols > 0 { 2.0 } else { 0.0 };
    let grid_w = |cs: f32, gap: f32| cols * cs + (cols - 1.0 + sep) * gap;
    let room = ui.w - 2.0 * ui.u(10.0) - 2.0 * pad - fl - fr;
    let w = grid_w(cs, gap);
    if w > room && room > 0.0 {
        let k = (room / w).max(0.4);
        cs *= k;
        gap *= k;
    }
    let gw = grid_w(cs, gap);
    let gh = ROWS as f32 * cs + (ROWS - 1) as f32 * gap;
    (gw + 2.0 * pad + fl + fr, gh + 2.0 * pad + ft + fb, cs, gap, pad)
}

/// The bar's outer size, as the last frame drew it (the bottom row's room for it).
pub fn outer_size(ui: &Ui) -> (f32, f32) {
    let (w, h, ..) = size(ui, st().over_cols);
    (w, h)
}

fn hide() {
    hud::set_bar_area(None);
    bar::set_cells([0; CELLS], false);
    st().geom = None;
}

pub fn run(ui: &mut Ui) {
    let s = st();
    let p = palette::palette();
    if p == 0 || hud::hidden(p) || crate::engine::api::ingame::hud_hidden() {
        return hide();
    }
    // What the bar's last update laid out, while its updates run; else (none yet) as it stands.
    let (cells, over) = if crate::rt::now() - s.built_at < 0.25 {
        (s.cells, s.over.clone())
    } else {
        let (c, _, o) = bar::layout_all(p, palette::menu(p));
        (c, o)
    };
    if cells.iter().all(|&b| b == 0) && over.is_empty() {
        return hide();
    }
    log_layout(p, &cells, &over);

    let over_cols = over.len().div_ceil(ROWS);
    s.over_cols = over_cols;
    let (ow, oh, cs, gap, pad) = size(ui, over_cols);
    let outer = bar::place(ui, ow, oh);
    // Not `ui.panel`: a claimed panel would take the pointer from the stock bar.
    theme::panel(ui, outer);
    super::layout::note(outer);
    let (fl, ft, _, _) = theme::frame_margins(ui);
    let g = Geom { at: crate::rt::now(), gx: outer.x + fl + pad, gy: outer.y + ft + pad, cs, gap, kx: CANVAS.0 / ui.w, ky: CANVAS.1 / ui.h };
    s.geom = Some(g);
    hud::set_bar_area(Some([outer.x, outer.y, outer.right(), outer.bottom()]));
    bar::set_cells(cells, true);

    let t = ui.theme;
    let gh = ROWS as f32 * cs + (ROWS - 1) as f32 * gap;
    // The fifth column, cancel/back, set off from the grid proper; the overflow after a line.
    let px = ui.u(1.0);
    let line = |x: f32| R::new(x, g.gy, px.max(1.0), gh);
    ui.draw.rect(line(g.gx + 4.0 * (cs + gap) - gap * 0.5 - px), fade(t.panel_edge, 0.5));
    if over_cols > 0 {
        ui.draw.rect(line(g.gx + COLS as f32 * (cs + gap) + gap * 0.5 - px), fade(t.panel_edge, 0.5));
    }
    let cfg = crate::config::get();
    for c in 0..CELLS + over.len() {
        let r = cell_rect(&g, c);
        let b = if c < CELLS { cells[c] } else { over[c - CELLS] };
        if b == 0 {
            ui.draw.fill(r, theme::button_corners(ui, r), fade(t.track, 0.55));
            continue;
        }
        let enabled = palette::state(b) != 0;
        let id = ui.id(&crate::sfmt!("bar.grid.{}", c));
        let mut st = theme::State::of(ui, id, palette::hovered(b) && enabled, palette::pressed(b) && enabled);
        st.on = palette::state(b) == 2;
        st.off = !enabled;
        let mut sp = palette::sprite(b);
        if sp == 0 {
            sp = palette::own_sprite(b);
        }
        // Roots' button art is a tile itself (decision 344): it fills the cell, no plate of ours.
        if crate::icons::whole_tile(sp) {
            crate::icons::tile_button(ui, r, sp, st);
            if let Some(label) = if c < CELLS && cfg.hud.bar_labels { bar::qol_label(c) } else { None } {
                let sz = ui.u(t.micro_size + 1.0);
                let lr = R::new(r.x + ui.u(3.0), r.y + ui.u(2.0), sz * 1.1, sz * 1.15);
                theme::badge(ui, lr, label, !enabled);
            }
            continue;
        }
        let content = theme::button_frame(ui, r, st);
        let icon = r.inset(ui.u(5.0));
        let ours = crate::icons::ours_for_sprite(cfg.hud.icons, sp, |s| sprite::art(s).is_some()) && {
            let ink = if enabled { content } else { fade(content, 0.45) };
            let ir = icon.inset(ui.u(0.5));
            crate::icons::draw(&mut ui.draw, sprite::name(sp), ir, 1.0, crate::icons::theme_colour(&t, ink))
        };
        if !ours {
            match sprite::art(sp) {
                Some((tex, uv)) => {
                    let tint = if enabled { 0xffffffff } else { fade(0xffffffff, 0.4) };
                    ui.draw.image_uv(tex, icon, uv, tint);
                }
                None => {
                    let d = palette::describe(b);
                    let name = d.name.strip_prefix("npc_").or_else(|| d.name.strip_prefix("all_")).unwrap_or(&d.name);
                    ui.text_in(icon, name, t.micro_size, Align::Centre, content, Weight::Regular);
                }
            }
        }
        if let Some(label) = if c < CELLS && cfg.hud.bar_labels { bar::qol_label(c) } else { None } {
            let sz = ui.u(t.micro_size + 1.0);
            let lr = R::new(r.x + ui.u(3.0), r.y + ui.u(2.0), sz * 1.1, sz * 1.15);
            theme::badge(ui, lr, label, !enabled);
        }
    }
}

/// `bar: grid menu=0 | Q=build ... | over: a b`, once per change.
fn log_layout(p: usize, cells: &[usize; CELLS], over: &[usize]) {
    let name = |b: usize| {
        let d = palette::describe(b);
        crate::sfmt!("{}{}", d.name, if palette::state(b) == 0 { "(off)" } else { "" })
    };
    let mut line = crate::sfmt!("bar: grid menu={} mode={} |", palette::menu(p), palette::mode(p));
    for (c, &b) in cells.iter().enumerate() {
        if b != 0 {
            line.push_str(&crate::sfmt!(" {}={}", LABEL[c] as char, name(b)));
        }
    }
    if !over.is_empty() {
        line.push_str(" | over:");
        for &b in over {
            line.push(' ');
            line.push_str(&name(b));
        }
    }
    let s = st();
    if s.last_log != line {
        crate::log!("{}", line);
        s.last_log = line;
    }
}
