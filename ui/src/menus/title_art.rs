//! The main menu's title as the game's own art, on Fleet Operations: Roots (PLAN-FLEETOPS.md,
//! decisions 360 to 369): the lettering of `bitmaps\main\transparant.png` in the game's
//! Data folder, drawn in place of the column's text heading.
//!
//! The file is the player's own and is only ever read, at run time: it is decoded
//! (`a2fmt::png`), cropped to the lettering (`a2fmt::png::title_box`, found by alpha and
//! colour, so the faint panel and the frame line the picture carries are left out), and
//! uploaded as a texture of ours. Nothing of it is written anywhere. A picture the
//! player's install lacks, or that does not decode, gives `None` and the caller keeps
//! the text heading. Only that file is read, never the picture of the whole menu.
//!
//! Colour and alpha are kept apart where it matters: the picture is cut down by whole
//! factors in premultiplied space (a box, so a transparent texel's colour never leaks
//! in), un-premultiplied back to the straight alpha the UI's pipeline blends, and the
//! colour of the fully transparent texels beside an edge is filled in from their
//! neighbours so the sampler's bilinear step does not darken the edge.
use crate::gfx::{self, Tex};
use alloc::string::String;
use alloc::vec::Vec;

struct Art {
    /// The crop, BGRA straight alpha, rows top-down.
    w: u32,
    h: u32,
    px: Vec<u8>,
    /// The texture drawn now and the reduction it was made at.
    tex: Option<(Tex, u32)>,
}

static mut ART: Option<Option<Art>> = None;

/// The path to read: `[Menus] TitleImage=` (`none` for no picture), else the content
/// set's `Files.MenuTitle`; "" when neither names one.
fn path() -> String {
    let c = crate::config::get().menus_title_image.trim();
    if c.eq_ignore_ascii_case("none") {
        return String::new();
    }
    if !c.is_empty() {
        return String::from(c);
    }
    String::from(crate::content::get().files.menu_title)
}

fn load() -> Option<Art> {
    let p = path();
    if p.is_empty() {
        return None;
    }
    let Some(data) = crate::files::open(&p) else {
        crate::log!("menus: title picture {} not found; the column keeps its text", p);
        return None;
    };
    let img = match a2fmt::png::decode(&data) {
        Ok(i) => i,
        Err(e) => {
            crate::log!("menus: title picture {} does not decode ({:?}); the column keeps its text", p, e);
            return None;
        }
    };
    let Some([bx, by, bw, bh]) = a2fmt::png::title_box(&img) else {
        crate::log!("menus: title picture {} has no lettering; the column keeps its text", p);
        return None;
    };
    let mut px = Vec::with_capacity((bw * bh * 4) as usize);
    for y in by..by + bh {
        let o = ((y * img.width + bx) * 4) as usize;
        px.extend_from_slice(&img.pixels[o..o + (bw * 4) as usize]);
    }
    crate::log!("menus: title picture {} ({}x{}), lettering {}x{} at {},{}", p, img.width, img.height, bw, bh, bx, by);
    Some(Art { w: bw, h: bh, px, tex: None })
}

/// The crop cut down `n` times each way (premultiplied box), straight alpha again, the
/// colour of transparent texels borrowed from the ones beside them.
fn reduce(a: &Art, n: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = ((a.w / n).max(1), (a.h / n).max(1));
    let mut out = alloc::vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (mut sb, mut sg, mut sr, mut sa, mut cnt) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for dy in 0..n {
                for dx in 0..n {
                    let (sx, sy) = (x * n + dx, y * n + dy);
                    if sx >= a.w || sy >= a.h {
                        continue;
                    }
                    let q = &a.px[((sy * a.w + sx) * 4) as usize..][..4];
                    let al = q[3] as u32;
                    sb += q[0] as u32 * al;
                    sg += q[1] as u32 * al;
                    sr += q[2] as u32 * al;
                    sa += al;
                    cnt += 1;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            if sa > 0 {
                out[o] = ((sb + sa / 2) / sa) as u8;
                out[o + 1] = ((sg + sa / 2) / sa) as u8;
                out[o + 2] = ((sr + sa / 2) / sa) as u8;
                out[o + 3] = ((sa + cnt / 2) / cnt.max(1)) as u8;
            }
        }
    }
    // Colour for the transparent texels beside ink, two rings.
    for _ in 0..2 {
        let src = out.clone();
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let o = ((y as u32 * w + x as u32) * 4) as usize;
                if src[o + 3] != 0 || (src[o] | src[o + 1] | src[o + 2]) != 0 {
                    continue;
                }
                let (mut s, mut c) = ([0u32; 3], 0u32);
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let p = ((ny as u32 * w + nx as u32) * 4) as usize;
                    if (src[p] | src[p + 1] | src[p + 2]) != 0 {
                        for k in 0..3 {
                            s[k] += src[p + k] as u32;
                        }
                        c += 1;
                    }
                }
                if c > 0 {
                    for k in 0..3 {
                        out[o + k] = (s[k] / c) as u8;
                    }
                }
            }
        }
    }
    (w, h, out)
}

/// The title's texture and its aspect (width over height), made for a picture about
/// `want_w` screen pixels wide; None when there is no picture to draw.
#[allow(static_mut_refs)]
pub fn get(want_w: f32) -> Option<(Tex, f32)> {
    let art = unsafe { ART.get_or_insert_with(load) }.as_mut()?;
    // Whole-number reductions down to under twice the size drawn; the sampler does the rest.
    let n = ((art.w as f32 / want_w.max(1.0)) as u32).clamp(1, 8);
    let aspect = art.w as f32 / art.h as f32;
    if let Some((t, k)) = art.tex {
        if k == n {
            return Some((t, aspect));
        }
    }
    let (w, h, px) = if n == 1 { (art.w, art.h, reduce(art, 1).2) } else { let r = reduce(art, n); (r.0, r.1, r.2) };
    let t = gfx::texture_bgra(w, h, px);
    art.tex = Some((t, n));
    Some((t, aspect))
}
