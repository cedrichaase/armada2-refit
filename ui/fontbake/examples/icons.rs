//! Renders the baked icons to a contact sheet, as the UI's shader draws them:
//! `cargo run --example icons --target x86_64-unknown-linux-gnu -- out.ppm [px]`
//! (from ui/fontbake). Each icon at `px` (default 40, the bar's at 1080 lines) and at
//! twice that, on a dark button, the roles in stand-in colours. For looking only.
use fontbake::icons::{self, Role};

fn colour(r: Role) -> [f32; 3] {
    match r {
        Role::Ink => [0.92, 0.93, 0.98],
        Role::Dim => [0.55, 0.57, 0.66],
        Role::Good => [0.35, 0.85, 0.45],
        Role::Warn => [0.98, 0.78, 0.25],
        Role::Bad => [0.95, 0.30, 0.28],
        Role::Accent => [0.62, 0.62, 0.98],
        Role::Dilithium => [0.55, 0.85, 1.0],
        Role::Metal => [0.75, 0.75, 0.78],
        Role::Latinum => [0.95, 0.80, 0.30],
        Role::Crew => [0.45, 0.95, 0.45],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "icons.ppm".into());
    let px: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    let list = icons::icons();
    let (pad, cols) = (12usize, 8usize);
    let big = px * 2;
    let cw = px + big + 3 * pad;
    let ch = big + 2 * pad + 14;
    let rows = list.len().div_ceil(cols);
    let (w, h) = (cols * cw, rows * ch);
    let mut img = vec![[0.07f32, 0.07, 0.10]; w * h];
    // Each layer rasterised straight from its shape at the target size: the distance
    // in pixels, one pixel of antialiasing, as the shader's saturate(0.5 - d) does.
    for (n, ic) in list.iter().enumerate() {
        let (ox, oy) = ((n % cols) * cw, (n / cols) * ch);
        for (size, x0, y0) in [(px, ox + pad, oy + pad + (big - px) / 2), (big, ox + 2 * pad + px, oy + pad)] {
            let k = icons::DESIGN / size as f32;
            // the button behind
            for y in 0..size + 8 {
                for x in 0..size + 8 {
                    let (xx, yy) = (x0 + x - 4, y0 + y - 4);
                    img[yy * w + xx] = [0.16, 0.17, 0.24];
                }
            }
            for (role, s) in &ic.layers {
                let c = colour(*role);
                for y in 0..size {
                    for x in 0..size {
                        let d = s.d((x as f32 + 0.5) * k, (y as f32 + 0.5) * k) / k;
                        let a = (0.5 - d).clamp(0.0, 1.0);
                        let p = &mut img[(y0 + y) * w + x0 + x];
                        for i in 0..3 {
                            p[i] = p[i] * (1.0 - a) + c[i] * a;
                        }
                    }
                }
            }
        }
    }
    let mut b = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in &img {
        for v in p {
            b.push((v.clamp(0.0, 1.0).powf(1.0) * 255.0) as u8);
        }
    }
    std::fs::write(&out, b).unwrap();
    let names: Vec<&str> = list.iter().map(|i| i.name).collect();
    std::fs::write(format!("{out}.names"), names.join("\n")).unwrap();
    eprintln!("{} icons, {} layers, blob {} bytes", list.len(), list.iter().map(|i| i.layers.len()).sum::<usize>(), icons::bake().len());
}
