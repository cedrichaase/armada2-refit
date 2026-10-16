//! Text: the UI's font, baked at build time into a signed-distance atlas, and laying
//! a string out into glyph quads.
//!
//! Units: a `size` is the font's em in back-buffer pixels. Quads come out in
//! back-buffer pixels, y down, with their atlas UVs (0..1).
//!
//! The font is Noto Sans Condensed (platform/vendor/noto-sans-condensed-2.015), baked by
//! `ui/fontbake` from build.rs into one blob per weight, little-endian:
//!   0  "A2SD"   4 atlas_w u32   8 atlas_h u32   12 em_texels f32   16 spread f32
//!   20 ascent f32   24 descent f32   28 line_height f32   32 nglyphs u32   36 nkern u32
//!   40 glyphs, 40 bytes each, sorted by codepoint:
//!        cp u32, advance f32, bounds[4] f32, uv[4] f32
//!   .. kerning pairs, 12 bytes each, sorted by (a, b): a u32, b u32, em f32
//!   .. the atlas, atlas_w * atlas_h bytes, top row first
//! The blob is read in place (no allocation, no parse step), so a `Font` is a static.

/// One glyph's metrics, in ems (y down, relative to the pen on the baseline), and
/// where it sits in the atlas.
#[derive(Clone, Copy, Debug, Default)]
pub struct Glyph {
    pub advance: f32,
    /// x0, y0, x1, y1 of the atlas cell (padding included), in ems from the pen.
    pub bounds: [f32; 4],
    /// u0, v0, u1, v1 in the atlas.
    pub uv: [f32; 4],
}

pub struct Font {
    pub atlas_w: u32,
    pub atlas_h: u32,
    /// One byte per texel: 128 on the outline, more inside.
    pub pixels: &'static [u8],
    /// Atlas texels per em.
    pub em_texels: f32,
    /// Distance in atlas texels from the outline to 0 or 255.
    pub spread: f32,
    /// In ems, ascent positive above the baseline, descent positive below.
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
    glyphs: &'static [u8],
    kerns: &'static [u8],
}

const GLYPH_BYTES: usize = 40;
const KERN_BYTES: usize = 12;

const fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
const fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}

impl Font {
    const fn parse(b: &'static [u8]) -> Font {
        assert!(b[0] == b'A' && b[1] == b'2' && b[2] == b'S' && b[3] == b'D');
        let (w, h) = (u32_at(b, 4), u32_at(b, 8));
        let (ng, nk) = (u32_at(b, 32) as usize, u32_at(b, 36) as usize);
        let (_, rest) = b.split_at(40);
        let (glyphs, rest) = rest.split_at(ng * GLYPH_BYTES);
        let (kerns, rest) = rest.split_at(nk * KERN_BYTES);
        let (pixels, _) = rest.split_at((w * h) as usize);
        Font {
            atlas_w: w,
            atlas_h: h,
            pixels,
            em_texels: f32_at(b, 12),
            spread: f32_at(b, 16),
            ascent: f32_at(b, 20),
            descent: f32_at(b, 24),
            line_height: f32_at(b, 28),
            glyphs,
            kerns,
        }
    }

    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        let (mut lo, mut hi) = (0, self.glyphs.len() / GLYPH_BYTES);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let o = mid * GLYPH_BYTES;
            let cp = u32_at(self.glyphs, o);
            if cp == ch as u32 {
                let f = |i: usize| f32_at(self.glyphs, o + 4 + 4 * i);
                return Some(Glyph { advance: f(0), bounds: [f(1), f(2), f(3), f(4)], uv: [f(5), f(6), f(7), f(8)] });
            }
            if cp < ch as u32 { lo = mid + 1 } else { hi = mid }
        }
        None
    }

    /// Kerning between a pair, in ems (added to the first's advance).
    pub fn kern(&self, a: char, b: char) -> f32 {
        let key = (a as u32, b as u32);
        let (mut lo, mut hi) = (0, self.kerns.len() / KERN_BYTES);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let o = mid * KERN_BYTES;
            let k = (u32_at(self.kerns, o), u32_at(self.kerns, o + 4));
            if k == key {
                return f32_at(self.kerns, o + 8);
            }
            if k < key { lo = mid + 1 } else { hi = mid }
        }
        0.0
    }

    /// The glyph drawn for `ch`: its own, else '?'.
    fn glyph_or_fallback(&self, ch: char) -> (char, Option<Glyph>) {
        match self.glyph(ch) {
            Some(g) => (ch, Some(g)),
            None => ('?', self.glyph('?')),
        }
    }
}

static REGULAR: Font = Font::parse(include_bytes!(concat!(env!("OUT_DIR"), "/font-regular.bin")));
static BOLD: Font = Font::parse(include_bytes!(concat!(env!("OUT_DIR"), "/font-bold.bin")));

/// The UI's font (Noto Sans Condensed Medium).
pub fn font() -> &'static Font {
    &REGULAR
}

/// The bold weight (Noto Sans Condensed SemiBold).
pub fn font_bold() -> &'static Font {
    &BOLD
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphQuad {
    /// x0, y0, x1, y1 in back-buffer pixels.
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

/// Lays `text` out on one line with the top of its line box at `y`, starting at `x`;
/// calls `out` with each glyph's quad and returns the advance width. The baseline is
/// rounded to a whole pixel so stems sit on the pixel grid; '\n' is ignored (see
/// `layout_wrapped` for breaks), and a character the font lacks draws as '?'.
pub fn layout_with(font: &Font, text: &str, size: f32, x: f32, y: f32, out: &mut dyn FnMut(GlyphQuad)) -> f32 {
    let base = libm::roundf(y + font.ascent * size);
    let mut pen = x;
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' {
            continue;
        }
        let (c, g) = font.glyph_or_fallback(ch);
        let Some(g) = g else { continue };
        if let Some(p) = prev {
            pen += font.kern(p, c) * size;
        }
        if g.bounds[2] > g.bounds[0] {
            out(GlyphQuad {
                rect: [pen + g.bounds[0] * size, base + g.bounds[1] * size, pen + g.bounds[2] * size, base + g.bounds[3] * size],
                uv: g.uv,
            });
        }
        pen += g.advance * size;
        prev = Some(c);
    }
    pen - x
}

/// `layout_with` in the regular weight.
pub fn layout(text: &str, size: f32, x: f32, y: f32, out: &mut dyn FnMut(GlyphQuad)) -> f32 {
    layout_with(font(), text, size, x, y, out)
}

/// The width and line height of `text` at `size`, one line, regular weight.
pub fn measure(text: &str, size: f32) -> (f32, f32) {
    measure_with(font(), text, size)
}

pub fn measure_with(font: &Font, text: &str, size: f32) -> (f32, f32) {
    (layout_with(font, text, size, 0.0, 0.0, &mut |_| {}), font.line_height * size)
}

/// Calls `line` with each line of `text` broken to fit `max_width`: at '\n', and at
/// spaces when the next word would not fit. A word wider than `max_width` gets a line of
/// its own (it is not split).
fn wrap(font: &Font, text: &str, size: f32, max_width: f32, line: &mut dyn FnMut(&str)) {
    let width = |s: &str| layout_with(font, s, size, 0.0, 0.0, &mut |_| {});
    for para in text.split('\n') {
        let (mut start, mut end) = (0, 0); // the line so far is para[start..end]; empty if start == end
        let mut pos = 0;
        loop {
            let rest = &para[pos..];
            let ws = pos + (rest.len() - rest.trim_start_matches(' ').len());
            if ws >= para.len() {
                break;
            }
            let we = ws + para[ws..].find(' ').unwrap_or(para.len() - ws);
            if start == end {
                start = ws;
            } else if width(&para[start..we]) > max_width {
                line(&para[start..end]);
                start = ws;
            }
            end = we;
            pos = we;
        }
        line(&para[start..end]);
    }
}

/// Lays `text` out as wrapped lines (see `wrap`), the first line's box top at `y`,
/// lines one `line_height` apart; returns the total height.
pub fn layout_wrapped(font: &Font, text: &str, size: f32, max_width: f32, x: f32, y: f32, out: &mut dyn FnMut(GlyphQuad)) -> f32 {
    let step = font.line_height * size;
    let mut n = 0u32;
    wrap(font, text, size, max_width, &mut |l| {
        layout_with(font, l, size, x, y + n as f32 * step, out);
        n += 1;
    });
    n as f32 * step
}

/// The widest line and total height `layout_wrapped` would produce.
pub fn measure_wrapped(font: &Font, text: &str, size: f32, max_width: f32) -> (f32, f32) {
    let mut n = 0u32;
    let mut w = 0.0f32;
    wrap(font, text, size, max_width, &mut |l| {
        w = w.max(layout_with(font, l, size, 0.0, 0.0, &mut |_| {}));
        n += 1;
    });
    (w, n as f32 * font.line_height * size)
}
