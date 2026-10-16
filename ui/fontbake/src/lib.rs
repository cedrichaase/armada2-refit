//! Bakes a TTF into the blob `ui/src/text` embeds: a single-channel signed-distance atlas
//! plus metrics and kerning (the blob's layout is documented in `ui/src/text/mod.rs`).
//!
//! The distance is exact: the outline is flattened into segments and every texel takes
//! the nearest one (point-segment distance), signed by the nonzero winding. Only
//! segments within the spread of a texel are looked at, which keeps a bake to seconds.

use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
    raw::{
        TableProvider,
        tables::gpos::{PairPos, PositionSubtables},
    },
};
use std::path::Path;

/// Atlas texels per em, and the distance (in texels) that maps to 0 / 255.
pub const EM_TEXELS: f32 = 48.0;
pub const SPREAD: f32 = 6.0;
const ATLAS_W: usize = 1024;
const GAP: usize = 1;

/// The charset: ASCII, Latin-1 and the typographic extras the UI uses.
pub fn charset() -> Vec<char> {
    let mut v: Vec<char> = (32u32..=126).chain(160..=255).filter_map(char::from_u32).collect();
    v.extend(['\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2013}', '\u{2014}', '\u{2026}', '\u{2022}', '\u{20AC}', '\u{00D7}']);
    v.sort();
    v.dedup();
    v
}

type Seg = [f32; 4]; // x0 y0 x1 y1, in atlas texels, y up

#[derive(Default)]
struct Flatten {
    segs: Vec<Seg>,
    cur: (f32, f32),
    start: (f32, f32),
    k: f32,
}

impl Flatten {
    fn line(&mut self, x: f32, y: f32) {
        let (x, y) = (x * self.k, y * self.k);
        self.segs.push([self.cur.0, self.cur.1, x, y]);
        self.cur = (x, y);
    }
}

impl OutlinePen for Flatten {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cur = (x * self.k, y * self.k);
        self.start = self.cur;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.line(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (p0, c, p1) = (self.cur, (cx * self.k, cy * self.k), (x * self.k, y * self.k));
        const N: usize = 10;
        for i in 1..N {
            let t = i as f32 / N as f32;
            let u = 1.0 - t;
            let px = u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0;
            let py = u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1;
            self.segs.push([self.cur.0, self.cur.1, px, py]);
            self.cur = (px, py);
        }
        self.segs.push([self.cur.0, self.cur.1, p1.0, p1.1]);
        self.cur = p1;
    }
    fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
        let (p0, a, b, p1) = (self.cur, (ax * self.k, ay * self.k), (bx * self.k, by * self.k), (x * self.k, y * self.k));
        const N: usize = 14;
        for i in 1..N {
            let t = i as f32 / N as f32;
            let u = 1.0 - t;
            let px = u * u * u * p0.0 + 3.0 * u * u * t * a.0 + 3.0 * u * t * t * b.0 + t * t * t * p1.0;
            let py = u * u * u * p0.1 + 3.0 * u * u * t * a.1 + 3.0 * u * t * t * b.1 + t * t * t * p1.1;
            self.segs.push([self.cur.0, self.cur.1, px, py]);
            self.cur = (px, py);
        }
        self.segs.push([self.cur.0, self.cur.1, p1.0, p1.1]);
        self.cur = p1;
    }
    fn close(&mut self) {
        if self.cur != self.start {
            self.segs.push([self.cur.0, self.cur.1, self.start.0, self.start.1]);
        }
        self.cur = self.start;
    }
}

/// Signed distance from (px, py) to the outline, positive inside.
fn sdf(segs: &[Seg], px: f32, py: f32) -> f32 {
    let mut best = SPREAD * SPREAD * 4.0;
    let mut winding = 0i32;
    for s in segs {
        let (x0, y0, x1, y1) = (s[0], s[1], s[2], s[3]);
        // nonzero winding: a ray towards +x crossing the segment
        if (y0 <= py) != (y1 <= py) {
            let t = (py - y0) / (y1 - y0);
            if x0 + t * (x1 - x0) > px {
                winding += if y1 > y0 { 1 } else { -1 };
            }
        }
        // cheap reject: the segment's box is farther than the best so far
        let dx = (x0.min(x1) - px).max(px - x0.max(x1)).max(0.0);
        let dy = (y0.min(y1) - py).max(py - y0.max(y1)).max(0.0);
        if dx * dx + dy * dy >= best {
            continue;
        }
        let (ex, ey) = (x1 - x0, y1 - y0);
        let l2 = ex * ex + ey * ey;
        let t = if l2 > 0.0 { (((px - x0) * ex + (py - y0) * ey) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let (qx, qy) = (x0 + t * ex - px, y0 + t * ey - py);
        best = best.min(qx * qx + qy * qy);
    }
    let d = best.sqrt();
    if winding != 0 { d } else { -d }
}

struct Cell {
    cp: char,
    advance: f32,
    // left edge and top (y up) of the cell relative to the pen, in texels; size in texels
    left: i32,
    top: i32,
    w: usize,
    h: usize,
    pix: Vec<u8>,
    x: usize,
    y: usize,
}

/// Pair adjustments (font units) for the charset from the GPOS `kern` feature: format 1
/// pair sets and format 2 class pairs. Lookups add; within a lookup the first subtable
/// that has the pair wins.
fn gpos_kerning(font: &FontRef, gids: &[(char, skrifa::GlyphId)]) -> Vec<(char, char, i16)> {
    let mut out = Vec::new();
    let Ok(gpos) = font.gpos() else { return out };
    let (Ok(features), Ok(lookups)) = (gpos.feature_list(), gpos.lookup_list()) else { return out };
    let mut used: Vec<u16> = Vec::new();
    for rec in features.feature_records() {
        if rec.feature_tag().to_string() != "kern" {
            continue;
        }
        if let Ok(f) = rec.feature(features.offset_data()) {
            for i in f.lookup_list_indices() {
                if !used.contains(&i.get()) {
                    used.push(i.get());
                }
            }
        }
    }
    let subs: Vec<Vec<PairPos>> = used
        .iter()
        .filter_map(|&i| lookups.lookups().get(i as usize).ok())
        .filter_map(|l| match l.subtables() {
            Ok(PositionSubtables::Pair(p)) => Some(p.iter().filter_map(|s| s.ok()).collect()),
            _ => None,
        })
        .collect();
    for &(a, ga) in gids {
        for &(b, gb) in gids {
            let mut sum = 0i32;
            for lookup in &subs {
                for st in lookup {
                    match st {
                        PairPos::Format1(f) => {
                            let Some(ci) = f.coverage().ok().and_then(|c| c.get(ga)) else { continue };
                            let Ok(set) = f.pair_sets().get(ci as usize) else { continue };
                            let hit = set.pair_value_records().iter().filter_map(|r| r.ok()).find(|r| r.second_glyph().to_u32() == gb.to_u32());
                            if let Some(r) = hit {
                                sum += r.value_record1().x_advance().unwrap_or(0) as i32;
                                break;
                            }
                        }
                        PairPos::Format2(f) => {
                            if f.coverage().ok().and_then(|c| c.get(ga)).is_none() {
                                continue;
                            }
                            let (Ok(d1), Ok(d2)) = (f.class_def1(), f.class_def2()) else { continue };
                            let rec = f.class1_records().get(d1.get(ga) as usize).ok().and_then(|c1| c1.class2_records().get(d2.get(gb) as usize).ok());
                            if let Some(r) = rec {
                                sum += r.value_record1().x_advance().unwrap_or(0) as i32;
                            }
                            break;
                        }
                    }
                }
            }
            if sum != 0 {
                out.push((a, b, sum as i16));
            }
        }
    }
    out
}

/// Bakes one font. Blob layout: see `ui/src/text/mod.rs`.
pub fn bake(ttf: &[u8]) -> Vec<u8> {
    let font = FontRef::new(ttf).expect("not a font");
    let loc = LocationRef::default();
    let m = font.metrics(Size::unscaled(), loc);
    let upm = m.units_per_em as f32;
    let k = EM_TEXELS / upm;
    let gm = font.glyph_metrics(Size::unscaled(), loc);
    let outlines = font.outline_glyphs();
    let cmap = font.charmap();
    let pad = SPREAD.ceil() as i32;

    let mut cells = Vec::new();
    let mut gids = Vec::new();
    for ch in charset() {
        let Some(gid) = cmap.map(ch) else { continue };
        gids.push((ch, gid));
        let advance = gm.advance_width(gid).unwrap_or(0.0) / upm;
        let mut fl = Flatten { k, ..Default::default() };
        if let Some(o) = outlines.get(gid) {
            o.draw(DrawSettings::unhinted(Size::unscaled(), loc), &mut fl).expect("outline");
        }
        if fl.segs.is_empty() {
            cells.push(Cell { cp: ch, advance, left: 0, top: 0, w: 0, h: 0, pix: vec![], x: 0, y: 0 });
            continue;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for s in &fl.segs {
            x0 = x0.min(s[0]).min(s[2]);
            x1 = x1.max(s[0]).max(s[2]);
            y0 = y0.min(s[1]).min(s[3]);
            y1 = y1.max(s[1]).max(s[3]);
        }
        let left = x0.floor() as i32 - pad;
        let top = y1.ceil() as i32 + pad;
        let w = (x1.ceil() as i32 + pad - left) as usize;
        let h = (top - (y0.floor() as i32 - pad)) as usize;
        let mut pix = vec![0u8; w * h];
        for r in 0..h {
            for c in 0..w {
                let d = sdf(&fl.segs, (left + c as i32) as f32 + 0.5, (top - r as i32) as f32 - 0.5);
                pix[r * w + c] = (128.0 + d / SPREAD * 127.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        cells.push(Cell { cp: ch, advance, left, top, w, h, pix, x: 0, y: 0 });
    }

    // Shelf packing, tallest first.
    let mut order: Vec<usize> = (0..cells.len()).filter(|&i| cells[i].w > 0).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(cells[i].h));
    let (mut sx, mut sy, mut shelf) = (GAP, GAP, 0);
    for &i in &order {
        if sx + cells[i].w + GAP > ATLAS_W {
            sx = GAP;
            sy += shelf + GAP;
            shelf = 0;
        }
        cells[i].x = sx;
        cells[i].y = sy;
        sx += cells[i].w + GAP;
        shelf = shelf.max(cells[i].h);
    }
    let atlas_h = (sy + shelf + GAP).next_power_of_two();
    // Black (outside) everywhere a cell is not.
    let mut atlas = vec![0u8; ATLAS_W * atlas_h];
    for c in &cells {
        for r in 0..c.h {
            atlas[(c.y + r) * ATLAS_W + c.x..][..c.w].copy_from_slice(&c.pix[r * c.w..][..c.w]);
        }
    }

    let ascent = m.ascent / upm;
    let descent = -m.descent / upm;
    let line_height = ascent + descent + m.leading / upm;
    let kern = gpos_kerning(&font, &gids);

    let mut b = Vec::new();
    let f32le = |b: &mut Vec<u8>, v: f32| b.extend_from_slice(&v.to_le_bytes());
    let u32le = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    b.extend_from_slice(b"A2SD");
    u32le(&mut b, ATLAS_W as u32);
    u32le(&mut b, atlas_h as u32);
    for v in [EM_TEXELS, SPREAD, ascent, descent, line_height] {
        f32le(&mut b, v);
    }
    u32le(&mut b, cells.len() as u32);
    u32le(&mut b, kern.len() as u32);
    for c in &cells {
        u32le(&mut b, c.cp as u32);
        f32le(&mut b, c.advance);
        if c.w == 0 {
            for _ in 0..8 {
                f32le(&mut b, 0.0);
            }
            continue;
        }
        // bounds: ems from the pen, y down from the baseline
        let bounds = [c.left as f32 / EM_TEXELS, -c.top as f32 / EM_TEXELS, (c.left + c.w as i32) as f32 / EM_TEXELS, (c.h as i32 - c.top) as f32 / EM_TEXELS];
        let uv = [c.x as f32 / ATLAS_W as f32, c.y as f32 / atlas_h as f32, (c.x + c.w) as f32 / ATLAS_W as f32, (c.y + c.h) as f32 / atlas_h as f32];
        for v in bounds.into_iter().chain(uv) {
            f32le(&mut b, v);
        }
    }
    // cells are in codepoint order (charset() is sorted), as the runtime's binary search needs
    for (a, c, v) in &kern {
        u32le(&mut b, *a as u32);
        u32le(&mut b, *c as u32);
        f32le(&mut b, *v as f32 / upm);
    }
    b.extend_from_slice(&atlas);
    b
}

/// Bakes the two vendored weights into `out`: Medium as `font-regular.bin`, SemiBold as
/// `font-bold.bin`.
pub fn bake_fonts(vendor: &Path, out: &Path) {
    let dir = vendor.join("noto-sans-condensed-2.015");
    for (src, dst) in [("NotoSans-CondensedMedium.ttf", "font-regular.bin"), ("NotoSans-CondensedSemiBold.ttf", "font-bold.bin")] {
        let p = dir.join(src);
        println!("cargo:rerun-if-changed={}", p.display());
        let ttf = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        std::fs::write(out.join(dst), bake(&ttf)).unwrap();
    }
}
