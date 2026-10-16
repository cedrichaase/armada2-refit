//! The HUD's icons, drawn here in code and baked into a signed-distance atlas the way
//! the font is (`ui/src/icons.rs` reads the blob; STYLE.md, "Icons").
//!
//! Each icon is one or more **layers**, each a shape and a colour *role* the runtime
//! fills from the theme (the ink, a dimmer ink, a status colour, a resource's colour),
//! so an icon is flat, two-tone at most, sharp at any size and in every theme. The
//! shapes are built on a 64-unit grid (y down) from a few primitives with exact
//! distances (circle, rounded box, capsule, polygon, arc) combined as union, cut and
//! intersection, which keeps a distance good enough for an antialiased edge.
//!
//! Each keeps the metaphor of the game's own button art (the captain's chair for
//! orders, the steel beam for building, the brain for the AI menu, ...): only the
//! drawing is ours. Icons are named by the sprite they stand in for (`b_stop`,
//! `comm_button`), which the HUD finds by name in the game's sprite database.

/// Atlas texels per design unit, the design grid, and the distance spread in texels.
pub const DESIGN: f32 = 64.0;
pub const SPREAD: f32 = 6.0;
const PAD: usize = 6;
const CELL: usize = DESIGN as usize + 2 * PAD;
const ATLAS_W: usize = 1024;

/// Colour roles, as the runtime numbers them (`ui/src/icons.rs`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Role {
    Ink = 0,
    Dim = 1,
    Good = 2,
    Warn = 3,
    Bad = 4,
    Accent = 5,
    Dilithium = 6,
    Metal = 7,
    Latinum = 8,
    Crew = 9,
}

#[derive(Clone)]
pub enum S {
    Circle(f32, f32, f32),
    /// x0, y0, x1, y1, corner radius.
    Rect(f32, f32, f32, f32, f32),
    /// A capsule: a segment and its width.
    Seg(f32, f32, f32, f32, f32),
    Poly(Vec<(f32, f32)>),
    /// Centre, radius, from and to (degrees, clockwise from +x on screen), width.
    Arc(f32, f32, f32, f32, f32, f32),
    Union(Vec<S>),
    Sub(Box<S>, Box<S>),
    Inter(Box<S>, Box<S>),
    /// The outline of a shape, `w` wide, centred on its edge.
    Shell(Box<S>, f32),
    /// A shape grown by `r` (negative: shrunk), its corners rounded.
    Grow(Box<S>, f32),
    /// An affine map: a b c d (the 2x2, x' = a x + b y + tx) and tx ty.
    Xf(Box<S>, [f32; 6]),
}

fn len(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

fn seg_d(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (ex, ey) = (bx - ax, by - ay);
    let l2 = ex * ex + ey * ey;
    let t = if l2 > 0.0 { (((px - ax) * ex + (py - ay) * ey) / l2).clamp(0.0, 1.0) } else { 0.0 };
    len(px - ax - t * ex, py - ay - t * ey)
}

impl S {
    /// Signed distance, negative inside.
    pub fn d(&self, x: f32, y: f32) -> f32 {
        match self {
            S::Circle(cx, cy, r) => len(x - cx, y - cy) - r,
            S::Rect(x0, y0, x1, y1, r) => {
                let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                let (hx, hy) = ((x1 - x0) * 0.5 - r, (y1 - y0) * 0.5 - r);
                let (qx, qy) = ((x - cx).abs() - hx, (y - cy).abs() - hy);
                len(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
            }
            S::Seg(ax, ay, bx, by, w) => seg_d(x, y, *ax, *ay, *bx, *by) - w * 0.5,
            S::Poly(v) => {
                let n = v.len();
                let mut d = f32::MAX;
                let mut s = 1.0;
                let mut j = n - 1;
                for i in 0..n {
                    let (vi, vj) = (v[i], v[j]);
                    d = d.min(seg_d(x, y, vi.0, vi.1, vj.0, vj.1));
                    let c1 = y >= vi.1;
                    let c2 = y < vj.1;
                    let c3 = (vj.0 - vi.0) * (y - vi.1) > (vj.1 - vi.1) * (x - vi.0);
                    if (c1 && c2 && c3) || (!c1 && !c2 && !c3) {
                        s = -s;
                    }
                    j = i;
                }
                s * d
            }
            S::Arc(cx, cy, r, a0, a1, w) => {
                let (dx, dy) = (x - cx, y - cy);
                let mut a = dy.atan2(dx).to_degrees();
                let (lo, hi) = (*a0, *a1);
                while a < lo {
                    a += 360.0;
                }
                while a > lo + 360.0 {
                    a -= 360.0;
                }
                if a <= hi {
                    (len(dx, dy) - r).abs() - w * 0.5
                } else {
                    let p = |deg: f32| (cx + r * deg.to_radians().cos(), cy + r * deg.to_radians().sin());
                    let (e0, e1) = (p(lo), p(hi));
                    len(x - e0.0, y - e0.1).min(len(x - e1.0, y - e1.1)) - w * 0.5
                }
            }
            S::Union(v) => v.iter().map(|s| s.d(x, y)).fold(f32::MAX, f32::min),
            S::Sub(a, b) => a.d(x, y).max(-b.d(x, y)),
            S::Inter(a, b) => a.d(x, y).max(b.d(x, y)),
            S::Shell(a, w) => a.d(x, y).abs() - w * 0.5,
            S::Grow(a, r) => a.d(x, y) - r,
            S::Xf(a, m) => {
                let [a_, b, c, d_, tx, ty] = *m;
                let det = a_ * d_ - b * c;
                let (px, py) = (x - tx, y - ty);
                let (ix, iy) = ((d_ * px - b * py) / det, (-c * px + a_ * py) / det);
                a.d(ix, iy) * det.abs().sqrt()
            }
        }
    }
}

// ---- the vocabulary -------------------------------------------------------------------

fn c(x: f32, y: f32, r: f32) -> S {
    S::Circle(x, y, r)
}
fn rr(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> S {
    S::Rect(x0, y0, x1, y1, r)
}
fn seg(ax: f32, ay: f32, bx: f32, by: f32, w: f32) -> S {
    S::Seg(ax, ay, bx, by, w)
}
fn poly(v: &[(f32, f32)]) -> S {
    S::Poly(v.to_vec())
}
fn arc(cx: f32, cy: f32, r: f32, a0: f32, a1: f32, w: f32) -> S {
    S::Arc(cx, cy, r, a0, a1, w)
}
fn u(v: Vec<S>) -> S {
    S::Union(v)
}
fn sub(a: S, b: S) -> S {
    S::Sub(Box::new(a), Box::new(b))
}
fn inter(a: S, b: S) -> S {
    S::Inter(Box::new(a), Box::new(b))
}
fn shell(a: S, w: f32) -> S {
    S::Shell(Box::new(a), w)
}
fn grow(a: S, r: f32) -> S {
    S::Grow(Box::new(a), r)
}
/// A polyline, `w` wide, round joins and caps.
fn line(p: &[(f32, f32)], w: f32) -> S {
    u(p.windows(2).map(|q| seg(q[0].0, q[0].1, q[1].0, q[1].1, w)).collect())
}
fn xf(a: S, m: [f32; 6]) -> S {
    S::Xf(Box::new(a), m)
}
/// Scaled by `k` about (cx, cy).
fn scale(a: S, k: f32, cx: f32, cy: f32) -> S {
    xf(a, [k, 0.0, 0.0, k, cx - k * cx, cy - k * cy])
}
/// Turned by `deg` clockwise about (cx, cy).
fn rot(a: S, deg: f32, cx: f32, cy: f32) -> S {
    let (s, co) = deg.to_radians().sin_cos();
    xf(a, [co, -s, s, co, cx - co * cx + s * cy, cy - s * cx - co * cy])
}
/// A ring.
fn ring(x: f32, y: f32, r: f32, w: f32) -> S {
    shell(c(x, y, r), w)
}
/// A regular polygon of `n` corners, the first at `deg`.
fn ngon(x: f32, y: f32, r: f32, n: usize, deg: f32) -> S {
    let v: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let a = (deg + 360.0 * i as f32 / n as f32).to_radians();
            (x + r * a.cos(), y + r * a.sin())
        })
        .collect();
    S::Poly(v)
}
/// A star of `n` points, outer and inner radius.
fn star(x: f32, y: f32, ro: f32, ri: f32, n: usize, deg: f32) -> S {
    let v: Vec<(f32, f32)> = (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { ro } else { ri };
            let a = (deg + 180.0 * i as f32 / n as f32).to_radians();
            (x + r * a.cos(), y + r * a.sin())
        })
        .collect();
    S::Poly(v)
}

/// An arrow from a to b: a shaft `w` wide and a head `head` long.
fn arrow(ax: f32, ay: f32, bx: f32, by: f32, w: f32, head: f32) -> S {
    let (dx, dy) = (bx - ax, by - ay);
    let l = len(dx, dy);
    let (ux, uy) = (dx / l, dy / l);
    let (nx, ny) = (-uy, ux);
    let (hx, hy) = (bx - ux * head, by - uy * head);
    let hw = head * 0.62;
    u(vec![
        seg(ax, ay, hx + ux * 1.0, hy + uy * 1.0, w),
        grow(poly(&[(bx, by), (hx + nx * hw, hy + ny * hw), (hx - nx * hw, hy - ny * hw)]), 1.0),
    ])
}

/// A ship marker of the formations: a dart pointing up, centred at (x, y).
fn dart(x: f32, y: f32, s: f32) -> S {
    grow(poly(&[(x, y - s), (x + s * 0.8, y + s * 0.75), (x, y + s * 0.3), (x - s * 0.8, y + s * 0.75)]), 0.6)
}

/// A crew member: head and shoulders.
fn person(x: f32, y: f32, s: f32) -> S {
    u(vec![
        c(x, y - s * 0.55, s * 0.36),
        inter(rr(x - s * 0.62, y - s * 0.08, x + s * 0.62, y + s * 1.2, s * 0.5), rr(x - s, y - s, x + s, y + s * 0.85, 0.0)),
    ])
}

/// A dilithium crystal: a long gem, point up.
fn crystal(x: f32, y: f32, s: f32) -> S {
    grow(poly(&[(x, y - s), (x + s * 0.5, y - s * 0.35), (x + s * 0.5, y + s * 0.35), (x, y + s), (x - s * 0.5, y + s * 0.35), (x - s * 0.5, y - s * 0.35)]), 0.8)
}

/// A bar of gold-pressed latinum.
fn latinum(x: f32, y: f32, s: f32) -> S {
    sub(
        grow(poly(&[(x - s, y + s * 0.45), (x - s * 0.7, y - s * 0.45), (x + s * 0.7, y - s * 0.45), (x + s, y + s * 0.45)]), 1.5),
        rr(x - s * 0.45, y - s * 0.12, x + s * 0.45, y + s * 0.08, 1.0),
    )
}

/// A stack of metal plates (an ingot block, seen at three quarters).
fn metal(x: f32, y: f32, s: f32) -> S {
    let top = poly(&[(x, y - s), (x + s, y - s * 0.5), (x, y), (x - s, y - s * 0.5)]);
    let left = poly(&[(x - s, y - s * 0.5), (x, y), (x, y + s), (x - s, y + s * 0.5)]);
    let right = poly(&[(x + s, y - s * 0.5), (x, y), (x, y + s), (x + s, y + s * 0.5)]);
    let gap = u(vec![seg(x - s, y - s * 0.5, x, y, 2.4), seg(x, y, x + s, y - s * 0.5, 2.4), seg(x, y, x, y + s, 2.4)]);
    sub(grow(u(vec![top, left, right]), 1.0), gap)
}

/// An open hand from the side, palm up, fingers to the right: the trade icons' hand.
fn hand() -> S {
    let palm = u(vec![
        // the palm, and the fingers lying flat to the right, their tips curled up
        grow(poly(&[(14.0, 45.0), (40.0, 44.0), (54.0, 44.5), (58.0, 42.0), (60.0, 44.0), (57.0, 49.0), (40.0, 53.0), (14.0, 55.0)]), 1.2),
        // the thumb, raised over the palm
        seg(27.0, 46.0, 35.0, 37.5, 6.5),
    ]);
    // the cuff, set off from the hand by a gap
    u(vec![sub(palm, rr(8.0, 38.0, 15.5, 62.0, 0.0)), rr(4.0, 42.0, 12.5, 58.0, 1.5)])
}

/// Three steps of a level, along the foot: `n` lit in the role's colour (the rest dim).
fn meter_lit(n: usize) -> S {
    u((0..n).map(|i| rr(9.0 + i as f32 * 16.0, 50.0, 23.0 + i as f32 * 16.0, 59.0, 2.5)).collect())
}
fn meter_dim(n: usize) -> S {
    u((n..3).map(|i| rr(9.0 + i as f32 * 16.0, 50.0, 23.0 + i as f32 * 16.0, 59.0, 2.5)).collect())
}

/// A wrench, its head at the top right, the jaw open.
fn wrench() -> S {
    let head = sub(c(42.0, 22.0, 12.0), u(vec![rr(38.0, 4.0, 46.0, 24.0, 1.5), c(42.0, 22.0, 4.0)]));
    let head = rot(head, 45.0, 42.0, 22.0);
    u(vec![head, seg(14.0, 50.0, 36.0, 28.0, 8.0)])
}

/// The Federation-style ship: saucer ahead, engineering hull and nacelles behind, from
/// above, pointing up. TNG proportions: a wide elliptical saucer.
fn starship(x: f32, y: f32, s: f32) -> S {
    let k = s / 26.0;
    let at = |a: S| xf(a, [k, 0.0, 0.0, k, x - 32.0 * k, y - 32.0 * k]);
    // saucer: an ellipse 40 x 26
    let saucer = xf(c(0.0, 0.0, 1.0), [22.0, 0.0, 0.0, 13.0, 32.0, 16.0]);
    let neck = poly(&[(30.0, 26.0), (34.0, 26.0), (35.0, 34.0), (29.0, 34.0)]);
    let hull = rr(28.5, 31.0, 35.5, 50.0, 3.5);
    let pylons = u(vec![seg(30.0, 44.0, 12.5, 38.0, 2.6), seg(34.0, 44.0, 51.5, 38.0, 2.6)]);
    let nacelles = u(vec![rr(9.5, 27.0, 15.5, 58.0, 3.0), rr(48.5, 27.0, 54.5, 58.0, 3.0)]);
    at(u(vec![saucer, neck, hull, pylons, nacelles]))
}

/// The cut that separates a layer drawn over the ink from it (a knockout `gap` wide).
fn knock(a: &S, gap: f32) -> S {
    grow(a.clone(), gap)
}

pub struct Icon {
    pub name: &'static str,
    pub layers: Vec<(Role, S)>,
}

fn ink(name: &'static str, s: S) -> Icon {
    Icon { name, layers: vec![(Role::Ink, s)] }
}

/// The ink with a coloured part over it (the ink cut back around it).
fn two(name: &'static str, base: S, role: Role, top: S) -> Icon {
    let cut = knock(&top, 2.5);
    Icon { name, layers: vec![(Role::Ink, sub(base, cut)), (role, top)] }
}

/// A level icon: `glyph` in the ink above (and `light` in `role`), and a meter of
/// three with `n` lit in `role`.
fn level(name: &'static str, glyph: S, light: Option<S>, role: Role, n: usize) -> Icon {
    let mut layers = vec![(Role::Ink, glyph), (role, meter_lit(n))];
    if let Some(l) = light {
        layers.push((role, l));
    }
    if n < 3 {
        layers.push((Role::Dim, meter_dim(n)));
    }
    Icon { name, layers }
}

pub fn icons() -> Vec<Icon> {
    let mut v = Vec::new();

    // ---- the menus ------------------------------------------------------------------
    // Orders: the captain's chair, from the side.
    let chair = u(vec![
        grow(poly(&[(17.0, 8.0), (25.0, 8.0), (28.0, 35.0), (21.0, 37.0)]), 2.0),
        rr(19.0, 33.0, 48.0, 40.0, 3.0),
        seg(27.0, 26.0, 44.0, 26.0, 5.0),
        seg(41.0, 26.0, 41.0, 34.0, 4.0),
        rr(29.0, 39.0, 36.0, 51.0, 1.5),
        rr(18.0, 50.0, 47.0, 56.0, 3.0),
    ]);
    v.push(ink("b_order", chair));
    // Build: the steel I-beam, at three quarters: its face in the ink, its length dim.
    let face = u(vec![rr(8.0, 22.0, 34.0, 28.0, 1.0), rr(17.0, 27.0, 25.0, 50.0, 0.0), rr(8.0, 49.0, 34.0, 55.0, 1.0)]);
    let length = u(vec![
        poly(&[(8.0, 22.0), (34.0, 22.0), (56.0, 9.0), (30.0, 9.0)]),
        poly(&[(25.0, 28.0), (25.0, 49.0), (47.0, 36.0), (47.0, 15.0)]),
        poly(&[(34.0, 49.0), (34.0, 55.0), (56.0, 42.0), (56.0, 36.0)]),
    ]);
    v.push(Icon { name: "b_build", layers: vec![(Role::Dim, sub(length, grow(face.clone(), 2.0))), (Role::Ink, face)] });
    // Research: a flask, its liquid in the accent.
    let flask = u(vec![rr(24.0, 6.0, 40.0, 11.0, 2.0), poly(&[(27.0, 10.0), (37.0, 10.0), (37.0, 24.0), (53.0, 52.0), (11.0, 52.0), (27.0, 24.0)])]);
    let flask = grow(flask, 2.0);
    let inside = grow(poly(&[(30.0, 13.0), (34.0, 13.0), (34.0, 25.0), (48.0, 49.0), (16.0, 49.0), (30.0, 25.0)]), 0.5);
    let liquid = inter(inside.clone(), rr(0.0, 36.0, 64.0, 64.0, 0.0));
    v.push(Icon { name: "b_research", layers: vec![(Role::Ink, sub(flask, grow(inside, 1.0))), (Role::Accent, liquid), (Role::Ink, u(vec![c(26.0, 43.0, 2.0), c(35.0, 40.0, 1.6)]))] });
    // Evolve (Species 8472): a double helix.
    let mut helix = Vec::new();
    let mut rungs = Vec::new();
    for i in 0..24 {
        let (t0, t1) = (i as f32 / 24.0, (i + 1) as f32 / 24.0);
        let y = |t: f32| 6.0 + t * 52.0;
        let x = |t: f32, ph: f32| 32.0 + 14.0 * (t * 2.0 * core::f32::consts::PI * 1.25 + ph).sin();
        helix.push(seg(x(t0, 0.0), y(t0), x(t1, 0.0), y(t1), 5.0));
        rungs.push(seg(x(t0, core::f32::consts::PI), y(t0), x(t1, core::f32::consts::PI), y(t1), 5.0));
    }
    let mut bars = Vec::new();
    for i in 1..8 {
        let t = i as f32 / 8.0;
        let y = 6.0 + t * 52.0;
        let a = (t * 2.0 * core::f32::consts::PI * 1.25).sin() * 14.0;
        bars.push(seg(32.0 - a, y, 32.0 + a, y, 2.5));
    }
    v.push(Icon { name: "b_evolve", layers: vec![(Role::Dim, u(bars)), (Role::Ink, u(helix)), (Role::Accent, u(rungs))] });
    // The AI menu: a brain from the side, drawn in line: its outline, the folds as
    // curves within it.
    let brain = u(vec![c(21.0, 30.0, 13.0), c(32.0, 22.0, 14.5), c(44.0, 28.0, 13.0), c(33.0, 34.0, 13.0)]);
    let inside = grow(brain.clone(), -2.2);
    let folds = inter(
        u(vec![
            arc(22.0, 30.0, 6.0, 180.0, 360.0, 3.8),
            arc(33.0, 20.0, 6.0, 0.0, 180.0, 3.8),
            arc(44.0, 30.0, 6.0, 180.0, 360.0, 3.8),
            arc(33.0, 46.0, 9.0, 220.0, 320.0, 3.8),
        ]),
        inside,
    );
    v.push(ink("b_aiMenu", u(vec![shell(grow(brain, -2.0), 4.4), folds])));
    // Formations: three darts in a wedge.
    v.push(ink("b_form", u(vec![dart(32.0, 18.0, 10.0), dart(17.0, 42.0, 9.0), dart(47.0, 42.0, 9.0)])));
    // Trade: the open hand holding a bar of latinum.
    v.push(two("b_trade", hand(), Role::Latinum, latinum(36.0, 28.0, 13.0)));
    // Back, cancel, close, stand down.
    v.push(ink("b_back", u(vec![arrow(52.0, 32.0, 10.0, 32.0, 7.0, 20.0)])));
    v.push(two("b_cancel", sub(ring(32.0, 32.0, 22.0, 7.0), c(0.0, 0.0, 0.0)), Role::Bad, rr(18.0, 28.5, 46.0, 35.5, 2.0)));
    v.push(ink("b_close", u(vec![seg(16.0, 16.0, 48.0, 48.0, 7.0), seg(48.0, 16.0, 16.0, 48.0, 7.0)])));
    v.push(Icon {
        name: "b_standdown",
        layers: vec![
            (Role::Bad, sub(grow(poly(&[(8.0, 12.0), (56.0, 12.0), (32.0, 54.0)]), 2.5), rr(17.0, 20.0, 47.0, 27.0, 2.0))),
        ],
    });
    // Upgrades (per race): a double chevron up.
    for n in ["b_frupgrade", "b_brupgrade", "b_krupgrade", "b_rrupgrade"] {
        v.push(two(n, line(&[(14.0, 52.0), (32.0, 36.0), (50.0, 52.0)], 7.0), Role::Accent, line(&[(14.0, 30.0), (32.0, 14.0), (50.0, 30.0)], 7.0)));
    }

    // ---- orders ---------------------------------------------------------------------
    // Move: a heading arrow.
    v.push(ink("b_move", grow(poly(&[(32.0, 6.0), (52.0, 56.0), (32.0, 44.0), (12.0, 56.0)]), 1.5)));
    // Attack: a reticle, its centre in red.
    let reticle = u(vec![
        ring(32.0, 32.0, 19.0, 4.5),
        seg(32.0, 4.0, 32.0, 18.0, 4.5),
        seg(32.0, 46.0, 32.0, 60.0, 4.5),
        seg(4.0, 32.0, 18.0, 32.0, 4.5),
        seg(46.0, 32.0, 60.0, 32.0, 4.5),
    ]);
    v.push(two("b_attack", reticle.clone(), Role::Bad, c(32.0, 32.0, 6.0)));
    // Stop: the octagon, a square within.
    v.push(Icon { name: "b_stop", layers: vec![(Role::Bad, shell(ngon(32.0, 32.0, 25.0, 8, 22.5), 6.0)), (Role::Ink, rr(23.0, 23.0, 41.0, 41.0, 2.0))] });
    // Transport: a crew member in the beam's sparkle.
    let sparkle = u(vec![
        star(13.0, 18.0, 6.0, 1.6, 4, -90.0),
        star(51.0, 26.0, 5.0, 1.4, 4, -90.0),
        star(15.0, 44.0, 4.0, 1.2, 4, -90.0),
        star(49.0, 50.0, 5.0, 1.4, 4, -90.0),
    ]);
    v.push(Icon { name: "b_transport", layers: vec![(Role::Ink, person(32.0, 32.0, 17.0)), (Role::Accent, sparkle)] });
    // Guard: a shield, its left half filled.
    let shield = grow(poly(&[(32.0, 6.0), (52.0, 13.0), (50.0, 36.0), (32.0, 58.0), (14.0, 36.0), (12.0, 13.0)]), 1.0);
    v.push(Icon { name: "b_guard", layers: vec![(Role::Ink, u(vec![shell(shield.clone(), 5.0), inter(shield, rr(0.0, 0.0, 32.0, 64.0, 0.0))]))] });
    // Patrol: there and back.
    v.push(ink("b_patrol", u(vec![arrow(8.0, 22.0, 56.0, 22.0, 6.0, 16.0), arrow(56.0, 42.0, 8.0, 42.0, 6.0, 16.0)])));
    // Explore (scout): binoculars.
    let bino = u(vec![c(19.0, 38.0, 13.0), c(45.0, 38.0, 13.0), rr(13.0, 18.0, 27.0, 36.0, 4.0), rr(37.0, 18.0, 51.0, 36.0, 4.0), rr(26.0, 26.0, 38.0, 38.0, 2.0)]);
    let lenses = u(vec![c(19.0, 39.0, 7.0), c(45.0, 39.0, 7.0)]);
    v.push(ink("b_scout", sub(bino, lenses)));
    // Search and destroy: a magnifying glass over a reticle.
    let glass = u(vec![ring(27.0, 27.0, 16.0, 5.5), seg(38.5, 38.5, 54.0, 54.0, 9.0)]);
    v.push(two("b_sandd", glass, Role::Bad, u(vec![seg(27.0, 17.0, 27.0, 37.0, 3.0), seg(17.0, 27.0, 37.0, 27.0, 3.0), c(27.0, 27.0, 3.0)])));
    // Decommission: a block taken apart.
    v.push(ink("b_decom", u(vec![rr(8.0, 8.0, 28.0, 28.0, 3.0), rr(36.0, 8.0, 56.0, 28.0, 3.0), rr(8.0, 36.0, 28.0, 56.0, 3.0), rot(rr(37.0, 37.0, 55.0, 55.0, 3.0), 18.0, 46.0, 46.0)])));
    // Board and capture: a boarding party and the way in.
    v.push(two("b_capture", u(vec![person(18.0, 30.0, 12.0), person(32.0, 36.0, 13.0)]), Role::Bad, arrow(38.0, 18.0, 58.0, 18.0, 5.0, 12.0)));
    // Colonise: a crew member standing on a world.
    let world = sub(c(32.0, 76.0, 36.0), c(32.0, 76.0, 0.0));
    v.push(two("b_colonize", person(32.0, 22.0, 14.0), Role::Good, inter(world, rr(0.0, 0.0, 64.0, 60.0, 0.0))));
    // Harvest: the pick.
    // the handle runs from the foot to (40, 18); the head is an arc about a point back
    // down the handle, so it crosses the handle's end square-on
    let pick_head = u(vec![arc(24.2, 41.1, 28.0, 262.0, 346.0, 7.0), c(40.0, 18.0, 5.5)]);
    v.push(two("b_harvest", seg(12.0, 58.0, 40.0, 18.0, 6.0), Role::Accent, pick_head));
    // Repair: the wrench; priority repair: with a plus; repair other: with an arrow out.
    v.push(ink("b_repair", wrench()));
    v.push(two("b_superRepair", wrench(), Role::Good, u(vec![rr(41.0, 36.0, 47.0, 58.0, 1.5), rr(33.0, 44.0, 55.0, 50.0, 1.5)])));
    v.push(two("b_repother", wrench(), Role::Accent, arrow(30.0, 52.0, 56.0, 52.0, 5.0, 12.0)));
    // Recrew: crew and a plus.
    v.push(two("b_recrew", u(vec![person(20.0, 34.0, 13.0), person(38.0, 38.0, 13.0)]), Role::Good, u(vec![rr(46.0, 6.0, 52.0, 28.0, 1.5), rr(38.0, 14.0, 60.0, 20.0, 1.5)])));
    // Rally point: four arrows to the centre.
    v.push(two("b_rally", u(vec![arrow(6.0, 6.0, 22.0, 22.0, 5.0, 12.0), arrow(58.0, 6.0, 42.0, 22.0, 5.0, 12.0), arrow(6.0, 58.0, 22.0, 42.0, 5.0, 12.0), arrow(58.0, 58.0, 42.0, 42.0, 5.0, 12.0)]), Role::Good, c(32.0, 32.0, 6.0)));
    // The flag (capture the flag).
    v.push(two("b_flag", seg(16.0, 8.0, 16.0, 58.0, 5.0), Role::Accent, grow(poly(&[(20.0, 10.0), (52.0, 14.0), (44.0, 22.0), (52.0, 32.0), (20.0, 30.0)]), 1.0)));

    // ---- alert levels and autonomy: a glyph and a meter -------------------------------
    // An alert: the status light in the level's colour, ringed.
    let alert = ring(32.0, 24.0, 19.0, 5.0);
    let light = c(32.0, 24.0, 11.0);
    v.push(level("b_galert", alert.clone(), Some(light.clone()), Role::Good, 1));
    v.push(level("b_yalert", alert.clone(), Some(light.clone()), Role::Warn, 2));
    v.push(level("b_ralert", alert, Some(light), Role::Bad, 3));
    let heading = scale(grow(poly(&[(32.0, 6.0), (52.0, 56.0), (32.0, 44.0), (12.0, 56.0)]), 1.5), 0.68, 32.0, 4.0);
    v.push(level("b_moveLow", heading.clone(), None, Role::Good, 1));
    v.push(level("b_moveMed", heading.clone(), None, Role::Warn, 2));
    v.push(level("b_moveHigh", heading, None, Role::Bad, 3));
    let burst = sub(star(32.0, 23.0, 20.0, 9.5, 8, -90.0), c(32.0, 23.0, 5.0));
    v.push(level("b_swNone", burst.clone(), None, Role::Good, 1));
    v.push(level("b_swMed", burst.clone(), None, Role::Warn, 2));
    v.push(level("b_swHigh", burst, None, Role::Bad, 3));

    // ---- formations: darts ----------------------------------------------------------
    let darts = |p: &[(f32, f32)], s: f32| u(p.iter().map(|&(x, y)| dart(x, y, s)).collect());
    v.push(ink("b_form1", darts(&[(32.0, 10.0), (32.0, 25.0), (32.0, 40.0), (32.0, 55.0)], 6.5))); // column
    v.push(ink("b_form2", darts(&[(14.0, 14.0), (32.0, 14.0), (50.0, 14.0), (14.0, 32.0), (32.0, 32.0), (50.0, 32.0), (14.0, 50.0), (32.0, 50.0), (50.0, 50.0)], 6.0))); // sheet
    v.push(ink("b_form3", darts(&[(32.0, 46.0), (21.0, 33.0), (43.0, 33.0), (10.0, 20.0), (54.0, 20.0)], 6.5))); // V
    v.push(ink("b_form4", darts(&[(32.0, 14.0), (21.0, 28.0), (43.0, 28.0), (10.0, 42.0), (54.0, 42.0)], 6.5))); // wedge
    v.push(ink("b_formx", darts(&[(12.0, 12.0), (52.0, 12.0), (32.0, 32.0), (12.0, 52.0), (52.0, 52.0)], 6.5))); // X
    v.push(ink("b_formwall", darts(&[(10.0, 20.0), (24.5, 20.0), (39.5, 20.0), (54.0, 20.0), (10.0, 42.0), (24.5, 42.0), (39.5, 42.0), (54.0, 42.0)], 6.0))); // assault wall
    // The three manoeuvres, in the accent: the arrow, the circle and the cloverleaf.
    v.push(two("b_farrow", c(0.0, 0.0, 0.0), Role::Accent, u(vec![arrow(22.0, 56.0, 22.0, 8.0, 6.0, 16.0), arrow(42.0, 8.0, 42.0, 56.0, 6.0, 16.0)])));
    let mut circ = vec![];
    for i in 0..4 {
        let a0 = i as f32 * 90.0 + 12.0;
        let a1 = a0 + 60.0;
        circ.push(arc(32.0, 32.0, 20.0, a0, a1, 5.0));
        let t = a1.to_radians();
        let (ex, ey) = (32.0 + 20.0 * t.cos(), 32.0 + 20.0 * t.sin());
        let (tx, ty) = (-t.sin(), t.cos());
        circ.push(grow(poly(&[(ex + tx * 9.0, ey + ty * 9.0), (ex - ty * 7.0 - tx * 1.0, ey + tx * 7.0 - ty * 1.0), (ex + ty * 7.0 - tx * 1.0, ey - tx * 7.0 - ty * 1.0)]), 0.8));
    }
    v.push(Icon { name: "b_fcircle", layers: vec![(Role::Accent, u(circ))] });
    let loop_ = |deg: f32| rot(shell(xf(c(0.0, 0.0, 1.0), [7.5, 0.0, 0.0, 13.0, 32.0, 17.0]), 5.0), deg, 32.0, 32.0);
    v.push(Icon { name: "b_fclover", layers: vec![(Role::Accent, u(vec![loop_(0.0), loop_(90.0), loop_(180.0), loop_(270.0)]))] });

    // ---- trade and resources --------------------------------------------------------
    // Start and end of a trade route: a waypoint and a flag on the route.
    let route = u(vec![seg(10.0, 50.0, 30.0, 30.0, 4.0), seg(30.0, 30.0, 54.0, 30.0, 4.0)]);
    v.push(two("b_strade", route.clone(), Role::Good, c(10.0, 50.0, 8.0)));
    v.push(two("b_etrade", route, Role::Bad, c(54.0, 30.0, 8.0)));
    // Load cargo: the hand and the resource.
    v.push(two("b_trad_dil", hand(), Role::Dilithium, crystal(34.0, 22.0, 15.0)));
    v.push(two("b_trad_lat", hand(), Role::Latinum, latinum(36.0, 28.0, 13.0)));
    v.push(two("b_trad_met", hand(), Role::Metal, metal(34.0, 22.0, 13.0)));
    v.push(two("b_tradcrew", hand(), Role::Crew, person(34.0, 24.0, 13.0)));
    v.push(two("b_trad_off", hand(), Role::Crew, u(vec![person(34.0, 24.0, 13.0), rr(44.0, 8.0, 54.0, 12.0, 1.5), rr(44.0, 15.0, 54.0, 19.0, 1.5)])));
    // Buy: the hand under the resource, receiving; sell: over it, letting go.
    // Buy: the resource and an arrow into the hand; sell: an arrow out of it.
    let trade = |name: &'static str, role: Role, item: S, buy: bool| {
        let a = if buy { arrow(52.0, 6.0, 52.0, 34.0, 4.5, 11.0) } else { arrow(52.0, 34.0, 52.0, 6.0, 4.5, 11.0) };
        let cut = grow(u(vec![item.clone(), a.clone()]), 2.5);
        Icon { name, layers: vec![(Role::Ink, sub(hand(), cut)), (role, item), (Role::Accent, a)] }
    };
    v.push(trade("b_buydil", Role::Dilithium, crystal(30.0, 22.0, 15.0), true));
    v.push(trade("b_buymet", Role::Metal, metal(30.0, 24.0, 13.0), true));
    v.push(trade("b_selldil", Role::Dilithium, crystal(30.0, 22.0, 15.0), false));
    v.push(trade("b_sellmet", Role::Metal, metal(30.0, 24.0, 13.0), false));
    // Transmute one into the other: two resources and an arrow between.
    let turn = arrow(27.0, 32.0, 41.0, 32.0, 4.5, 10.0);
    v.push(Icon {
        name: "b_recyclemet",
        layers: vec![(Role::Metal, metal(14.0, 32.0, 11.0)), (Role::Ink, turn.clone()), (Role::Dilithium, crystal(51.0, 32.0, 14.0))],
    });
    v.push(Icon {
        name: "b_recycledil",
        layers: vec![(Role::Dilithium, crystal(13.0, 32.0, 14.0)), (Role::Ink, turn), (Role::Metal, metal(50.0, 32.0, 11.0))],
    });

    // ---- the top-right buttons ------------------------------------------------------
    v.push(ink("menu_button", u(vec![rr(12.0, 14.0, 52.0, 20.0, 3.0), rr(12.0, 29.0, 52.0, 35.0, 3.0), rr(12.0, 44.0, 52.0, 50.0, 3.0)])));
    // Comm: a crew member and the signal.
    v.push(two("comm_button", person(32.0, 38.0, 13.0), Role::Accent, u(vec![arc(32.0, 30.0, 18.0, 200.0, 250.0, 4.0), arc(32.0, 30.0, 18.0, 290.0, 340.0, 4.0), arc(32.0, 30.0, 27.0, 205.0, 245.0, 4.0), arc(32.0, 30.0, 27.0, 295.0, 335.0, 4.0)])));
    // Idle unit: the starship, a sleeping "z" over it.
    let zz = |x: f32, y: f32, s: f32| line(&[(x, y), (x + s, y), (x, y + s), (x + s, y + s)], 3.0);
    v.push(two("idle_button", starship(28.0, 38.0, 19.0), Role::Accent, u(vec![zz(42.0, 6.0, 10.0), zz(52.0, 20.0, 7.0)])));
    // View: the eye.
    let eye = inter(c(32.0, 52.0, 30.0), c(32.0, 12.0, 30.0));
    v.push(ink("toggle_button", u(vec![sub(eye, grow(inter(c(32.0, 52.0, 30.0), c(32.0, 12.0, 30.0)), -4.5)), c(32.0, 32.0, 8.0)])));
    // Objectives: a list with the first ticked.
    v.push(two(
        "objectives_button",
        u(vec![rr(28.0, 14.0, 56.0, 20.0, 3.0), rr(28.0, 29.0, 56.0, 35.0, 3.0), rr(28.0, 44.0, 56.0, 50.0, 3.0), shell(rr(9.0, 26.0, 20.0, 37.0, 2.0), 3.0), shell(rr(9.0, 41.0, 20.0, 52.0, 2.0), 3.0)]),
        Role::Good,
        line(&[(8.0, 16.0), (13.0, 21.0), (22.0, 9.0)], 4.0),
    ));
    // Help: the question mark.
    v.push(ink("help_button", u(vec![arc(32.0, 22.0, 11.0, 180.0, 405.0, 6.5), seg(39.8, 29.8, 32.0, 36.0, 6.5), seg(32.0, 36.0, 32.0, 41.0, 6.5), c(32.0, 52.0, 4.2)])));
    // Palette (show or hide the button bar): the grid of buttons.
    let mut cells = vec![];
    for j in 0..3 {
        for i in 0..3 {
            cells.push(rr(10.0 + i as f32 * 16.0, 10.0 + j as f32 * 16.0, 22.0 + i as f32 * 16.0, 22.0 + j as f32 * 16.0, 3.0));
        }
    }
    v.push(ink("palette_button", u(cells)));
    // More: three dots.
    v.push(ink("more_button", u(vec![c(14.0, 32.0, 5.5), c(32.0, 32.0, 5.5), c(50.0, 32.0, 5.5)])));
    v
}

/// Bakes the icons; blob layout in `ui/src/icons.rs`.
pub fn bake() -> Vec<u8> {
    let list = icons();
    let mut cells: Vec<(&str, Role, Vec<u8>)> = Vec::new();
    for ic in &list {
        assert!(ic.name.len() < 32, "{}: name too long", ic.name);
        for (role, s) in &ic.layers {
            let mut pix = vec![0u8; CELL * CELL];
            for r in 0..CELL {
                for col in 0..CELL {
                    let x = col as f32 + 0.5 - PAD as f32;
                    let y = r as f32 + 0.5 - PAD as f32;
                    let d = -s.d(x, y);
                    pix[r * CELL + col] = (128.0 + d / SPREAD * 127.0).round().clamp(0.0, 255.0) as u8;
                }
            }
            cells.push((ic.name, *role, pix));
        }
    }
    let per_row = ATLAS_W / CELL;
    let rows = cells.len().div_ceil(per_row);
    let atlas_h = (rows * CELL).next_power_of_two();
    let mut atlas = vec![0u8; ATLAS_W * atlas_h];
    let mut b = Vec::new();
    let u32le = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    let f32le = |b: &mut Vec<u8>, v: f32| b.extend_from_slice(&v.to_le_bytes());
    b.extend_from_slice(b"A2IC");
    u32le(&mut b, ATLAS_W as u32);
    u32le(&mut b, atlas_h as u32);
    f32le(&mut b, DESIGN);
    f32le(&mut b, PAD as f32);
    f32le(&mut b, SPREAD);
    u32le(&mut b, cells.len() as u32);
    // In name order (the layers of one icon in their order), for the binary search.
    let mut order: Vec<usize> = (0..cells.len()).collect();
    order.sort_by(|&a, &c| cells[a].0.cmp(cells[c].0).then(a.cmp(&c)));
    for (slot, &i) in order.iter().enumerate() {
        let (name, role, pix) = &cells[i];
        let (cx, cy) = ((slot % per_row) * CELL, (slot / per_row) * CELL);
        for r in 0..CELL {
            atlas[(cy + r) * ATLAS_W + cx..][..CELL].copy_from_slice(&pix[r * CELL..][..CELL]);
        }
        let mut nm = [0u8; 32];
        nm[..name.len()].copy_from_slice(name.as_bytes());
        b.extend_from_slice(&nm);
        u32le(&mut b, *role as u32);
        for v in [cx as f32 / ATLAS_W as f32, cy as f32 / atlas_h as f32, (cx + CELL) as f32 / ATLAS_W as f32, (cy + CELL) as f32 / atlas_h as f32] {
            f32le(&mut b, v);
        }
    }
    b.extend_from_slice(&atlas);
    b
}
