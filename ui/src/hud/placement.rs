//! The placement markers, drawn by the UI (`engine::leftovers`, "The placement
//! markers"; HUD.md, "The placement markers"): while a station is being placed, its
//! footprint under the ghost, and the cells of the station placement grid that other
//! objects take.
//!
//! What is drawn is the engine's decision, never ours: the cells are the ones
//! `RenderFootprints` found not clear, and the footprint is clear or blocked as the
//! engine chose its sprite. In the theme (`[HUD] Placement=1`):
//!
//! - the footprint, in `good` where the place is clear and `bad` where it is not: a
//!   faint fill, a soft light inside its edge, an outline with a glow outside it; clear,
//!   corner brackets over the outline; blocked, hatched across. Shape and colour both
//!   tell the two apart, so they read without telling red from green;
//! - each cell that is not clear, an inset tile in `warn` (the amber of stock's grid),
//!   strongest about the footprint and fading with distance from it, to a floor that
//!   still shows where the map is taken further off;
//! - a cell under the footprint (in the world, its centre inside the footprint's
//!   corners) in `bad`, crossed: what is in the way.
//!
//! With `Placement=0` the engine's sprites are drawn as the engine drew them.
use crate::engine::leftovers::{Art, Placement, Projector};
use crate::gfx::{fade, Color, R};
use crate::widgets::Ui;

type P2 = (f32, f32);
type P3 = [f32; 3];

/// Lines logged so far (the footprint's state changing), at most `MAX_LOG`.
static mut LOGGED: u32 = 0;
const MAX_LOG: u32 = 48;
/// The last state logged: (cells, cells under the footprint, footprint: 0 none, 1
/// clear, 2 blocked).
static mut LAST: (usize, usize, u8) = (usize::MAX, 0, 0);

/// A frame without a placement: the next one logs its state afresh.
pub fn none() {
    unsafe { LAST = (usize::MAX, 0, 0) };
}

pub fn run(ui: &mut Ui, p: &Placement, ours: bool) {
    let clip = R::new(p.view.vp[0], p.view.vp[1], p.view.vp[2], p.view.vp[3]);
    ui.draw.push_clip(clip);
    let under = if ours { themed(ui, p, clip) } else { engine_art(ui, p, clip) };
    ui.draw.pop_clip();
    log(p, under, ours);
}

fn log(p: &Placement, under: usize, ours: bool) {
    let state = (p.cells.len(), under, p.foot.map_or(0, |f| if f.clear { 1 } else { 2 }));
    unsafe {
        let last = LAST;
        // A change of the footprint's state is a line (with what is under it then);
        // the pointer moving over the map is not.
        if last.2 == state.2 && last.0 != usize::MAX {
            return;
        }
        LAST = state;
        if LOGGED >= MAX_LOG {
            return;
        }
        LOGGED += 1;
    }
    let foot = match state.2 {
        1 => "clear",
        2 => "blocked",
        _ => "none",
    };
    crate::log!(
        "placement: footprint {}, {} cells not clear, {} under the footprint, drawn {}",
        foot,
        state.0,
        state.1,
        if ours { "in the theme" } else { "with the engine's sprites" }
    );
}

// ---- geometry -----------------------------------------------------------------------

fn lerp3(a: P3, b: P3, t: f32) -> P3 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn centre(c: &[P3; 4]) -> P3 {
    let mut o = [0.0; 3];
    for p in c {
        for k in 0..3 {
            o[k] += p[k] * 0.25;
        }
    }
    o
}

/// Distance on the map's plane (x, z).
fn dist(a: P3, b: P3) -> f32 {
    let (dx, dz) = (a[0] - b[0], a[2] - b[2]);
    libm::sqrtf(dx * dx + dz * dz)
}

/// Whether `p` lies inside the convex quad `c` on the map's plane (x, z).
fn inside(c: &[P3; 4], p: P3) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (c[i], c[(i + 1) % 4]);
        let cross = (b[0] - a[0]) * (p[2] - a[2]) - (b[2] - a[2]) * (p[0] - a[0]);
        if cross.abs() < 1.0e-4 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// The quad's corners on the screen, if all are in front of the eye and it touches the
/// viewport.
fn screen(v: &Projector, c: &[P3; 4], clip: R) -> Option<[P2; 4]> {
    let mut s = [(0.0, 0.0); 4];
    for (o, p) in s.iter_mut().zip(c.iter()) {
        *o = v.project(*p)?;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &s {
        x0 = x0.min(p.0);
        y0 = y0.min(p.1);
        x1 = x1.max(p.0);
        y1 = y1.max(p.1);
    }
    if x1 < clip.x || y1 < clip.y || x0 > clip.right() || y0 > clip.bottom() {
        return None;
    }
    Some(s)
}

/// A point of the quad `c` at (u, v), 0..1 along its first and last edges.
fn at(c: &[P3; 4], u: f32, v: f32) -> P3 {
    lerp3(lerp3(c[0], c[1], u), lerp3(c[3], c[2], u), v)
}

// ---- in the theme ---------------------------------------------------------------------

/// Draws the markers in the theme; returns how many cells are under the footprint.
fn themed(ui: &mut Ui, p: &Placement, clip: R) -> usize {
    let t = &ui.theme;
    let (good, bad, warn) = (t.good, t.bad, t.warn);
    let line = ui.u(2.4).max(1.25);
    let thin = ui.u(1.4).max(1.0);
    let aa = 1.0;
    let v = &p.view;
    // The cells' size, from the first one; the footprint's centre and reach.
    let cell = p.cells.first().map(|c| dist(c[0], c[1])).filter(|s| *s > 0.0);
    let foot = p.foot.map(|f| (f, centre(&f.corners), f.corners.iter().map(|q| dist(*q, centre(&f.corners))).fold(0.0f32, f32::max)));
    let foot_s = foot.and_then(|(f, _, _)| screen(v, &f.corners, clip).map(|s| (f, s)));
    let glow = ui.u(12.0).max(4.0);
    // The footprint's body under the cells: its fill, the light inside its edge and,
    // blocked, the hatching.
    if let Some((f, s)) = foot_s {
        let col: Color = if f.clear { good } else { bad };
        ui.draw.poly(s, [fade(col, if f.clear { 0.08 } else { 0.10 }); 4]);
        ui.draw.band(&s, -glow * 1.5, -line * 0.5, fade(col, 0.0), fade(col, 0.30));
        if !f.clear {
            // One stripe a cell (from the footprint's own size without one).
            let edge = dist(f.corners[0], f.corners[1]).max(dist(f.corners[1], f.corners[2]));
            let n = match cell {
                Some(sz) => libm::roundf(edge / sz) as i32,
                None => 8,
            }
            .clamp(4, 16);
            let step = 1.0 / n as f32;
            for k in 1..2 * n {
                let d = k as f32 * step;
                let (u0, v0, u1, v1) = if d <= 1.0 { (d, 0.0, 0.0, d) } else { (1.0, d - 1.0, d - 1.0, 1.0) };
                let (Some(a), Some(b)) = (v.project(at(&f.corners, u0, v0)), v.project(at(&f.corners, u1, v1))) else { continue };
                ui.draw.line_soft(a, b, thin, aa, fade(col, 0.34));
            }
        }
    }
    let mut under = 0;
    for c in &p.cells {
        let mid = centre(c);
        let hit = foot.is_some_and(|(f, _, _)| inside(&f.corners, mid));
        if hit {
            under += 1;
        }
        // Strongest about the footprint, fading out over a few of its sizes; without a
        // placement (the map editor), all alike.
        let a = match foot {
            Some((_, fc, reach)) => {
                let s = cell.unwrap_or(reach * 0.25);
                let x = ((dist(mid, fc) - reach) / (reach * 2.0 + s * 4.0)).clamp(0.0, 1.0);
                let x = x * x * (3.0 - 2.0 * x);
                1.0 - x * 0.78
            }
            None => 0.55,
        };
        // The tile: the cell inset a little, so neighbours read as tiles, not a sheet.
        let inset = c.map(|q| lerp3(q, mid, 0.12));
        let Some(s) = screen(v, &inset, clip) else { continue };
        if hit {
            ui.draw.poly(s, [fade(bad, 0.2); 4]);
            ui.draw.band(&s, -thin * 0.5, thin * 0.5, fade(bad, 0.85), fade(bad, 0.85));
            ui.draw.band(&s, thin * 0.5, thin * 0.5 + aa, fade(bad, 0.85), fade(bad, 0.0));
            let x = c.map(|q| lerp3(q, mid, 0.34));
            if let Some(xs) = screen(v, &x, clip) {
                ui.draw.line_soft(xs[0], xs[2], thin, aa, fade(bad, 0.8));
                ui.draw.line_soft(xs[1], xs[3], thin, aa, fade(bad, 0.8));
            }
        } else {
            ui.draw.poly(s, [fade(warn, 0.09 * a); 4]);
            ui.draw.band(&s, -thin * 2.5, -thin * 0.5, fade(warn, 0.0), fade(warn, 0.16 * a));
            ui.draw.band(&s, -thin * 0.5, thin * 0.5, fade(warn, 0.6 * a), fade(warn, 0.6 * a));
            ui.draw.band(&s, thin * 0.5, thin * 0.5 + aa, fade(warn, 0.6 * a), fade(warn, 0.0));
        }
    }
    // The footprint's edge over the cells: the glow outside it, the outline and, clear,
    // corner brackets along the edges, a quarter of each.
    let Some((f, s)) = foot_s else { return under };
    let col: Color = if f.clear { good } else { bad };
    ui.draw.band(&s, line * 0.5, line * 0.5 + glow, fade(col, 0.32), fade(col, 0.0));
    ui.draw.band(&s, -line * 0.5, line * 0.5, fade(col, 0.85), fade(col, 0.85));
    ui.draw.band(&s, line * 0.5, line * 0.5 + aa, fade(col, 0.85), fade(col, 0.0));
    ui.draw.band(&s, -line * 0.5 - aa, -line * 0.5, fade(col, 0.0), fade(col, 0.85));
    if f.clear {
        for i in 0..4 {
            let (a, b, z) = (s[i], s[(i + 1) % 4], s[(i + 3) % 4]);
            let to = |q: P2, k: f32| (a.0 + (q.0 - a.0) * k, a.1 + (q.1 - a.1) * k);
            ui.draw.line_soft(a, to(b, 0.25), line * 2.2, aa, col);
            ui.draw.line_soft(a, to(z, 0.25), line * 2.2, aa, col);
        }
    }
    under
}

// ---- with the engine's sprites ----------------------------------------------------------

/// The engine's look: each cell and the footprint its sprite's rectangle of its texture,
/// added to the frame as the engine's z-sorted sprites are.
fn engine_art(ui: &mut Ui, p: &Placement, clip: R) -> usize {
    let under = p.foot.map_or(0, |f| p.cells.iter().filter(|c| inside(&f.corners, centre(c))).count());
    if let Some(art) = p.cell_art {
        for c in &p.cells {
            if let Some(s) = screen(&p.view, c, clip) {
                sprite(ui, s, &art);
            }
        }
    }
    if let Some(f) = p.foot {
        if let (Some(art), Some(s)) = (f.art, screen(&p.view, &f.corners, clip)) {
            sprite(ui, s, &art);
        }
    }
    under
}

fn sprite(ui: &mut Ui, s: [P2; 4], art: &Art) {
    super::leftovers::engine_sprite(ui, s, art.texture, art.uv, art.rgb);
}
