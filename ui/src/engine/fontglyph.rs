//! A picture out of the game's bitmap font (FO-6, the resources panel's icons): Fleet
//! Operations: Roots draws a resource's icon as a glyph of its fonts, `Sprites\FontFinal4_*.spr`,
//! not as a sprite. The `.spr` holds two animation strips, one entry per code from 0x20: the
//! `_uv` strip each glyph's top left corner and the `_w` strip its width, in the file's reference
//! size (`@referenceWidth`, `@referenceHeight`); the `page` line names the texture and the
//! height of every glyph. The content set names the font file (`[Files] IconFont`), the picture
//! is cut out of the texture at run time from the player's install, and nothing of it is copied.
use crate::gfx::Tex;
use alloc::string::String;
use alloc::vec::Vec;

struct Font {
    texture: String,
    reference: (f32, f32),
    height: f32,
    /// The top left corner of each code from 0x20, and its width.
    uv: Vec<(f32, f32)>,
    width: Vec<f32>,
}

static mut FONT: Option<Option<Font>> = None;

/// The numbers of an animation strip: its entries, one pair or one value a line, up to the
/// first blank line.
fn strip<'a>(lines: &[&'a str], suffix: &str) -> Vec<Vec<f32>> {
    let Some(at) = lines.iter().position(|l| l.starts_with("@animation") && l.split_whitespace().nth(1).is_some_and(|n| n.ends_with(suffix))) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut started = false;
    for l in &lines[at + 1..] {
        let l = l.split('#').next().unwrap_or("").trim();
        if !started {
            started = l == "@keyframes";
            continue;
        }
        if l.is_empty() || l.starts_with('@') {
            break;
        }
        out.push(l.split_whitespace().filter_map(|n| n.parse::<f32>().ok()).collect());
    }
    out
}

fn parse(text: &str) -> Option<Font> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let num = |key: &str| lines.iter().find_map(|l| l.strip_prefix(key)).and_then(|v| v.trim().parse::<f32>().ok());
    let reference = (num("@referenceWidth=")?, num("@referenceHeight=")?);
    let page = lines.iter().find(|l| l.starts_with("page"))?;
    let f: Vec<&str> = page.split('\t').collect();
    let texture = String::from(f.get(1)?.trim());
    let height = f.get(6)?.trim().parse::<f32>().ok()?;
    let uv: Vec<(f32, f32)> = strip(&lines, "_uv").iter().filter(|e| e.len() >= 2).map(|e| (e[0], e[1])).collect();
    let width: Vec<f32> = strip(&lines, "_w").iter().filter_map(|e| e.first().copied()).collect();
    if texture.is_empty() || height <= 0.0 || uv.is_empty() || uv.len() != width.len() {
        return None;
    }
    Some(Font { texture, reference, height, uv, width })
}

/// The glyph for `code` as a texture and its rectangle (u0, v0, u1, v1), with its width over
/// its height; None when the content set names no font, the file or its texture is missing,
/// or the font has no such code.
#[allow(static_mut_refs)]
pub fn glyph(code: u8) -> Option<(Tex, [f32; 4], f32)> {
    let font = unsafe { &mut FONT };
    if font.is_none() {
        let file = crate::content::get().files.icon_font;
        let f = if file.is_empty() {
            None
        } else {
            let f = crate::files::open(&crate::sfmt!("Sprites\\{}", file)).and_then(|d| parse(&String::from_utf8_lossy(&d)));
            crate::log!("fontglyph: {} {}", file, if f.is_some() { "read" } else { "NOT FOUND or unreadable" });
            f
        };
        *font = Some(f);
    }
    let f = font.as_ref()?.as_ref()?;
    let i = (code as usize).checked_sub(32)?;
    let ((x, y), w) = (*f.uv.get(i)?, *f.width.get(i)?);
    if w <= 0.0 {
        return None;
    }
    let tex = super::sprite::texture(&f.texture)?;
    Some((tex, [x / f.reference.0, y / f.reference.1, (x + w) / f.reference.0, (y + f.height) / f.reference.1], w / f.height))
}
