//! Bakes the atlas and renders sample strings through the runtime (ui/src/text) the way
//! the shader will: bilinear sample, then a ramp one pixel wide around 128. Writes PGMs to
//! $CLAUDE_JOB_DIR/tmp (or the system temp dir) for a look; asserts the basics.
//!   cargo test --features preview --target x86_64-unknown-linux-gnu --release
#![cfg(feature = "preview")]
#[allow(dead_code)]
#[path = "../../src/text/mod.rs"]
mod text;

use text::{Font, GlyphQuad};

fn sample(f: &Font, u: f32, v: f32) -> f32 {
    let x = (u * f.atlas_w as f32 - 0.5).max(0.0);
    let y = (v * f.atlas_h as f32 - 0.5).max(0.0);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let p = |xx: usize, yy: usize| f.pixels[yy.min(f.atlas_h as usize - 1) * f.atlas_w as usize + xx.min(f.atlas_w as usize - 1)] as f32;
    let a = p(x0, y0) * (1.0 - fx) + p(x0 + 1, y0) * fx;
    let b = p(x0, y0 + 1) * (1.0 - fx) + p(x0 + 1, y0 + 1) * fx;
    a * (1.0 - fy) + b * fy
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<f32>,
}

impl Canvas {
    fn draw(&mut self, f: &Font, q: &GlyphQuad, size: f32) {
        // distance in screen pixels per sdf byte step
        let px_per_texel = size / f.em_texels;
        let per_byte = f.spread * px_per_texel / 127.0;
        let (x0, y0, x1, y1) = (q.rect[0].floor() as i32, q.rect[1].floor() as i32, q.rect[2].ceil() as i32, q.rect[3].ceil() as i32);
        for y in y0.max(0)..y1.min(self.h as i32) {
            for x in x0.max(0)..x1.min(self.w as i32) {
                let u = q.uv[0] + (x as f32 + 0.5 - q.rect[0]) / (q.rect[2] - q.rect[0]) * (q.uv[2] - q.uv[0]);
                let v = q.uv[1] + (y as f32 + 0.5 - q.rect[1]) / (q.rect[3] - q.rect[1]) * (q.uv[3] - q.uv[1]);
                let d = (sample(f, u, v) - 128.0) * per_byte;
                let a = (d + 0.5).clamp(0.0, 1.0);
                let p = &mut self.px[y as usize * self.w + x as usize];
                *p = *p * (1.0 - a);
            }
        }
    }
    fn write(&self, path: &std::path::Path) {
        let mut b = format!("P5\n{} {}\n255\n", self.w, self.h).into_bytes();
        b.extend(self.px.iter().map(|v| (v * 255.0).round() as u8));
        std::fs::write(path, b).unwrap();
    }
}

fn out_dir() -> std::path::PathBuf {
    let d = std::env::var_os("CLAUDE_JOB_DIR").map(|d| std::path::PathBuf::from(d).join("tmp")).unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn preview() {
    let dir = out_dir();
    let (reg, bold) = (text::font(), text::font_bold());
    for f in [reg, bold] {
        assert_eq!(f.pixels.len(), (f.atlas_w * f.atlas_h) as usize);
        assert!(f.glyph('A').is_some() && f.glyph('€').is_some() && f.glyph('é').is_some());
        assert!(f.glyph('\u{4E00}').is_none());
        // kerning is something real, and negative for the classic pair
        println!("atlas {}x{}  asc {:.3} desc {:.3} lh {:.3}  kern AV {:.4} To {:.4}", f.atlas_w, f.atlas_h, f.ascent, f.descent, f.line_height, f.kern('A', 'V'), f.kern('T', 'o'));
    }
    assert!(reg.kern('A', 'V') < 0.0, "no GPOS kerning found");

    // the atlas itself
    let a = Canvas { w: reg.atlas_w as usize, h: reg.atlas_h as usize, px: reg.pixels.iter().map(|&p| p as f32 / 255.0).collect() };
    a.write(&dir.join("atlas-regular.pgm"));

    let lines = ["The quick brown fox jumps over the lazy dog 0123456789", "AVATAR To Ty We Yo - Fleet 12/34  Shields 100%  \u{201C}Warp\u{201D} \u{20AC}5 \u{00D7}2 \u{2026}", "Crème brûlée, Ångström, Straße, ñandú, ÿ", "Missing glyph: \u{4E00}"];
    for (name, size) in [("14", 14.0f32), ("32", 32.0)] {
        let mut c = Canvas { w: 1100, h: 1000, px: vec![1.0; 1100 * 1000] };
        let mut y = 8.0;
        for (f, label) in [(reg, "regular"), (bold, "bold")] {
            for l in lines {
                let mut quads = Vec::new();
                text::layout_with(f, l, size, 8.0, y, &mut |q| quads.push(q));
                for q in &quads {
                    c.draw(f, q, size);
                }
                y += f.line_height * size + 2.0;
            }
            let h = text::layout_wrapped(f, "A wrapped paragraph: the shield grid is at half strength, so reroute power to the forward arrays first.\nSecond paragraph.", size, 330.0, 8.0, y, &mut |q| c.draw(f, &q, size));
            println!("{label} {name}px wrapped height {h}");
            y += h + 6.0;
        }
        c.write(&dir.join(format!("sample-{name}.pgm")));
        let (w, h) = text::measure("Hello", size);
        println!("measure Hello @{size}: {w:.1} x {h:.1}");
    }
}
