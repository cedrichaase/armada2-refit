//! The style's drawing primitives (ui/STYLE.md): shapes with any mix of rounded and
//! chamfered corners, strokes, soft edges, glows, inner glows, shadows, stripes and
//! LCARS elbows, all through the one shape kind of `shaders/ui.hlsl`, so each is one
//! quad whatever its size, sharp at any resolution, with no texture.
//!
//! Sizes here are back-buffer pixels: the theme (`widgets/theme.rs`) turns units into
//! pixels before it calls in.
use super::{Color, Draw, Vertex, KIND_SHAPE, R};

/// A shape's four corners, top left, top right, bottom right, bottom left, in pixels:
/// a positive one is rounded with that radius, a negative one cut at 45 degrees that
/// far along each edge, 0 square.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Corners(pub [f32; 4]);

impl Corners {
    pub const SQUARE: Corners = Corners([0.0; 4]);

    pub const fn round(r: f32) -> Corners {
        Corners([r; 4])
    }

    pub const fn cut(c: f32) -> Corners {
        Corners([-c; 4])
    }

    /// Fully rounded ends for a shape `h` pixels across its short side.
    pub fn pill(r: R) -> Corners {
        Corners::round(r.w.min(r.h) * 0.5)
    }

    /// Every corner times `s` (units to pixels).
    pub fn scaled(self, s: f32) -> Corners {
        Corners(self.0.map(|c| c * s))
    }

    /// The corners of the same shape grown outward by `d` pixels: a rounded (or square)
    /// corner's radius grows by `d`, a chamfer by `d * (2 - sqrt 2)`, so the outline
    /// stays parallel to the shape's own.
    pub fn grown(self, d: f32) -> Corners {
        Corners(self.0.map(|c| if c < 0.0 { (c - d * 0.585_786_4).min(0.0) } else { (c + d).max(0.0) }))
    }

    /// Only the corners whose flag is set; the others square.
    pub fn only(self, tl: bool, tr: bool, br: bool, bl: bool) -> Corners {
        let f = [tl, tr, br, bl];
        let mut c = self.0;
        for i in 0..4 {
            if !f[i] {
                c[i] = 0.0;
            }
        }
        Corners(c)
    }
}

/// Everything the shape kind can draw; `Default` is a filled, antialiased square box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub corners: Corners,
    /// The width of an outline drawn inside the edge; 0 fills the shape.
    pub stroke: f32,
    /// How many pixels the outer edge fades over: 1 is antialiased, more is soft.
    pub soft: f32,
    /// The same for the stroke's inner edge.
    pub soft_in: f32,
    /// A box cut out of the shape's bottom right corner: width, height and the radius
    /// of its own top-left corner (an LCARS elbow's inside). 0 wide: none.
    pub cut: [f32; 3],
    /// Stripes: period in pixels (0: none), the part of each period drawn (0..1), and
    /// their direction in radians (0: vertical stripes, repeating along x).
    pub stripes: [f32; 3],
    /// Mirrors the shape's own space, so the corners and the cut-out are mirrored too
    /// (an elbow at another corner).
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for Shape {
    fn default() -> Shape {
        Shape {
            corners: Corners::SQUARE,
            stroke: 0.0,
            soft: 1.0,
            soft_in: 1.0,
            cut: [0.0; 3],
            stripes: [0.0; 3],
            flip_x: false,
            flip_y: false,
        }
    }
}

/// Which corner of its rectangle an LCARS elbow turns at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Elbow {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Draw {
    /// A shape in `r`, coloured per corner (tl, tr, br, bl) for gradients.
    pub fn shape(&mut self, r: R, sh: &Shape, cols: [Color; 4]) {
        if r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        let (hx, hy) = (r.w * 0.5, r.h * 0.5);
        let lim = hx.min(hy);
        let c = sh.corners.0.map(|c| c.clamp(-lim, lim));
        // Room for the soft edge, which fades out `soft / 2` past the shape.
        let m = libm::ceilf(sh.soft.max(1.0) * 0.5) + 1.0;
        let g = r.inset(-m);
        let (ux0, ux1) = if sh.flip_x { (hx + m, -hx - m) } else { (-hx - m, hx + m) };
        let (uy0, uy1) = if sh.flip_y { (hy + m, -hy - m) } else { (-hy - m, hy + m) };
        let k = [KIND_SHAPE, sh.stroke.max(0.0), hx, hy];
        let t = [sh.soft.max(1.0), sh.soft_in.max(1.0), sh.cut[0].max(0.0), sh.cut[1].max(0.0)];
        let p = [sh.stripes[0].max(0.0), sh.stripes[1].clamp(0.0, 1.0), sh.stripes[2], sh.cut[2].max(0.0)];
        let vx = |x, y, col, u, v| Vertex { x, y, col, u, v, k, s: c, t, p };
        let (x0, y0, x1, y1) = (g.x, g.y, g.right(), g.bottom());
        self.quad(None, [
            vx(x0, y0, cols[0], ux0, uy0),
            vx(x1, y0, cols[1], ux1, uy0),
            vx(x1, y1, cols[2], ux1, uy1),
            vx(x0, y1, cols[3], ux0, uy1),
        ]);
    }

    /// A filled shape.
    pub fn fill(&mut self, r: R, c: Corners, col: Color) {
        self.shape(r, &Shape { corners: c, ..Shape::default() }, [col; 4]);
    }

    /// A filled shape shaded from `top` to `bottom`.
    pub fn fill_v(&mut self, r: R, c: Corners, top: Color, bottom: Color) {
        self.shape(r, &Shape { corners: c, ..Shape::default() }, [top, top, bottom, bottom]);
    }

    /// A filled shape shaded from `left` to `right`.
    pub fn fill_h(&mut self, r: R, c: Corners, left: Color, right: Color) {
        self.shape(r, &Shape { corners: c, ..Shape::default() }, [left, right, right, left]);
    }

    /// An outline `w` pixels wide, inside the shape's edge.
    pub fn stroke(&mut self, r: R, c: Corners, w: f32, col: Color) {
        self.shape(r, &Shape { corners: c, stroke: w, ..Shape::default() }, [col; 4]);
    }

    /// An outline shaded from `top` to `bottom`: a bevel when `top` is light and
    /// `bottom` dark (or clear).
    pub fn stroke_v(&mut self, r: R, c: Corners, w: f32, top: Color, bottom: Color) {
        self.shape(r, &Shape { corners: c, stroke: w, ..Shape::default() }, [top, top, bottom, bottom]);
    }

    /// Light around the outside of a shape, fading to nothing `reach` pixels out. The
    /// shape itself is left clear, so it can go under a translucent panel.
    pub fn glow(&mut self, r: R, c: Corners, reach: f32, col: Color) {
        if reach <= 0.0 {
            return;
        }
        let g = reach * 0.5;
        let sh = Shape { corners: c.grown(g), stroke: g + 1.0, soft: reach, ..Shape::default() };
        self.shape(r.inset(-g), &sh, [col; 4]);
    }

    /// Light along the inside of a shape's edge, fading to nothing `reach` pixels in.
    pub fn inner_glow(&mut self, r: R, c: Corners, reach: f32, col: Color) {
        if reach <= 0.0 {
            return;
        }
        let sh = Shape { corners: c, stroke: reach * 0.5, soft_in: reach, ..Shape::default() };
        self.shape(r, &sh, [col; 4]);
    }

    /// A soft shadow of a shape, moved by `off` and blurred over `reach` pixels.
    pub fn shadow(&mut self, r: R, c: Corners, off: (f32, f32), reach: f32, col: Color) {
        let g = reach * 0.25;
        let sh = Shape { corners: c.grown(g), soft: reach.max(1.0), ..Shape::default() };
        let rr = R::new(r.x + off.0, r.y + off.1, r.w, r.h).inset(-g);
        self.shape(rr, &sh, [col; 4]);
    }

    /// The shape filled with stripes: `period` pixels apart, `duty` of each drawn, at
    /// `angle` radians (0: vertical stripes).
    pub fn stripes(&mut self, r: R, c: Corners, period: f32, duty: f32, angle: f32, col: Color) {
        let sh = Shape { corners: c, stripes: [period, duty, angle], ..Shape::default() };
        self.shape(r, &sh, [col; 4]);
    }

    /// A shape with fully rounded ends.
    pub fn pill(&mut self, r: R, col: Color) {
        self.fill(r, Corners::pill(r), col);
    }

    /// An LCARS elbow filling `r`: a bar `side` pixels wide down one side and a bar
    /// `bar` pixels tall along the top or bottom, joined at corner `at` by a curve of
    /// radius `outer` outside and `inner` inside. The far end of the horizontal bar is
    /// capped round. Coloured `top` to `bottom`.
    #[allow(clippy::too_many_arguments)]
    pub fn elbow(&mut self, r: R, at: Elbow, side: f32, bar: f32, outer: f32, inner: f32, top: Color, bottom: Color) {
        let (flip_x, flip_y) = match at {
            Elbow::TopLeft => (false, false),
            Elbow::TopRight => (true, false),
            Elbow::BottomLeft => (false, true),
            Elbow::BottomRight => (true, true),
        };
        let side = side.min(r.w);
        let bar = bar.min(r.h);
        // In the shape's own (unmirrored) space: the turn at the top left, the bar's
        // end at the top right.
        let cap = bar * 0.5;
        let sh = Shape {
            corners: Corners([outer, cap, 0.0, 0.0]),
            cut: [r.w - side, r.h - bar, inner],
            flip_x,
            flip_y,
            ..Shape::default()
        };
        self.shape(r, &sh, [top, top, bottom, bottom]);
    }
}

impl Draw {
    /// Text wrapped at `max_w` pixels (`text::layout_wrapped`), the first line's top
    /// at `y`; returns its height.
    #[allow(clippy::too_many_arguments)]
    pub fn text_wrapped(&mut self, s: &str, size: f32, max_w: f32, x: f32, y: f32, c: Color, w: super::Weight) -> f32 {
        let font = super::font_of(w);
        let Some(tex) = self.font_tex(w) else { return 0.0 };
        let scale = 2.0 * font.spread * size / font.em_texels;
        let k = [super::KIND_SDF, scale.max(1.0), 0.0, 0.0];
        let mut quads = alloc::vec::Vec::new();
        let h = crate::text::layout_wrapped(font, s, size, max_w, x, y, &mut |q| quads.push(q));
        for q in quads {
            let [x0, y0, x1, y1] = q.rect;
            self.aquad(Some(tex), R::new(x0, y0, x1 - x0, y1 - y0), [c; 4], q.uv, k);
        }
        h
    }
}

// ---- images for the style --------------------------------------------------------------

/// A BMP from the game's files (`Menus/mainbkgr.bmp`, the widescreen menu plates that
/// `menus/install-plates.sh` installs), 24- or 32-bit uncompressed, as a texture; None
/// if it is missing or another format. The caller keeps the `Tex`.
pub fn load_bmp(rel: &str) -> Option<super::Tex> {
    let b = crate::rt::read_file(&crate::rt::game_path(rel))?;
    let u16_at = |o: usize| b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
    let u32_at = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    if b.get(0..2) != Some(b"BM") {
        return None;
    }
    let off = u32_at(10)? as usize;
    let w = u32_at(18)? as i32;
    let h = u32_at(22)? as i32;
    let bpp = u16_at(28)? as usize;
    let comp = u32_at(30)?;
    if w <= 0 || h == 0 || comp != 0 || (bpp != 24 && bpp != 32) || w > 8192 || h.abs() > 8192 {
        return None;
    }
    let (w, top_down, h) = (w as usize, h < 0, h.unsigned_abs() as usize);
    let bytes = bpp / 8;
    let pitch = (w * bytes + 3) & !3;
    if b.len() < off + pitch * h {
        return None;
    }
    let mut px = alloc::vec![0u8; w * h * 4];
    for y in 0..h {
        let src_row = if top_down { y } else { h - 1 - y };
        let src = &b[off + src_row * pitch..];
        let dst = &mut px[y * w * 4..(y + 1) * w * 4];
        for x in 0..w {
            dst[x * 4] = src[x * bytes];
            dst[x * 4 + 1] = src[x * bytes + 1];
            dst[x * 4 + 2] = src[x * bytes + 2];
            dst[x * 4 + 3] = 255;
        }
    }
    Some(super::texture_bgra(w as u32, h as u32, px))
}
