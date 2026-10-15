//! Text: the UI's font, baked at build time into a signed-distance atlas, and laying
//! a string out into glyph quads.
//!
//! Units: a `size` is the font's em in back-buffer pixels. Quads come out in
//! back-buffer pixels, y down, with their atlas UVs (0..1).

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
}

impl Font {
    pub fn glyph(&self, _ch: char) -> Option<Glyph> {
        None
    }
    /// Kerning between a pair, in ems (added to the first's advance).
    pub fn kern(&self, _a: char, _b: char) -> f32 {
        0.0
    }
}

static EMPTY: Font = Font { atlas_w: 1, atlas_h: 1, pixels: &[0], em_texels: 1.0, spread: 1.0, ascent: 0.8, descent: 0.2, line_height: 1.2 };

/// The UI's font.
pub fn font() -> &'static Font {
    &EMPTY
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphQuad {
    /// x0, y0, x1, y1 in back-buffer pixels.
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

/// Lays `text` out on one line with the top of its line box at `y`, starting at `x`;
/// calls `out` with each glyph's quad and returns the advance width.
pub fn layout(_text: &str, _size: f32, _x: f32, _y: f32, _out: &mut dyn FnMut(GlyphQuad)) -> f32 {
    0.0
}

/// The width and line height of `text` at `size`.
pub fn measure(text: &str, size: f32) -> (f32, f32) {
    (layout(text, size, 0.0, 0.0, &mut |_| {}), font().line_height * size)
}
