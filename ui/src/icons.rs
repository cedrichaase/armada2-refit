//! The HUD's icons (STYLE.md, "Icons"): our own flat drawings of the game's command
//! and button pictures, baked by `ui/fontbake` (`icons.rs`, where each is drawn) into a
//! signed-distance atlas, and drawn as the text is, so they are sharp at any size and
//! take the theme's colours. Each is named by the game sprite it stands in for
//! (`b_stop`, `comm_button`); a name with no icon here keeps the game's art.
//!
//! The blob, little-endian:
//!   0 "A2IC"  4 atlas_w u32  8 atlas_h u32  12 design f32  16 pad f32  20 spread f32
//!   24 n u32  28 entries, 52 bytes each, by name (an icon's layers in drawing order):
//!        name [u8; 32] (NUL-padded), role u32, uv[4] f32 (the cell, its pad included)
//!   .. the atlas, atlas_w * atlas_h bytes, top row first
use crate::gfx::{fade, Color, Draw, Tex, R};

const ENTRY: usize = 52;

static BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons.bin"));

fn u32_at(o: usize) -> u32 {
    u32::from_le_bytes([BLOB[o], BLOB[o + 1], BLOB[o + 2], BLOB[o + 3]])
}
fn f32_at(o: usize) -> f32 {
    f32::from_bits(u32_at(o))
}

/// What a layer is coloured with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The button's content colour (its text's).
    Ink,
    /// The same, dimmer: depth, the unlit steps of a meter.
    Dim,
    Good,
    Warn,
    Bad,
    Accent,
    /// A resource, in its colour on the resource strip (`eResource`).
    Resource(u32),
}

fn role(v: u32) -> Role {
    match v {
        0 => Role::Ink,
        1 => Role::Dim,
        2 => Role::Good,
        3 => Role::Warn,
        4 => Role::Bad,
        5 => Role::Accent,
        6 => Role::Resource(2),
        7 => Role::Resource(4),
        8 => Role::Resource(3),
        _ => Role::Resource(0),
    }
}

fn n() -> usize {
    u32_at(24) as usize
}

fn name_at(i: usize) -> &'static [u8] {
    let o = 28 + i * ENTRY;
    let raw = &BLOB[o..o + 32];
    let end = raw.iter().position(|&b| b == 0).unwrap_or(32);
    &raw[..end]
}

/// The first entry of `name`'s layers, if there is an icon by that name.
fn first(name: &str) -> Option<usize> {
    let key = name.as_bytes();
    let (mut lo, mut hi) = (0, n());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if name_at(mid) < key { lo = mid + 1 } else { hi = mid }
    }
    (lo < n() && name_at(lo) == key).then_some(lo)
}

pub fn has(name: &str) -> bool {
    !name.is_empty() && first(name).is_some()
}

static mut TEX: Option<Tex> = None;

fn tex() -> Tex {
    unsafe {
        if TEX.is_none() {
            let (w, h) = (u32_at(4), u32_at(8));
            let at = 28 + n() * ENTRY;
            TEX = Some(crate::gfx::texture_l8(w, h, &BLOB[at..at + (w * h) as usize]));
        }
        TEX.unwrap()
    }
}

/// Draws the icon `name` filling the square `r`, each layer in the colour `colour`
/// gives its role, times `alpha`. False (nothing drawn) when there is no such icon.
pub fn draw(d: &mut Draw, name: &str, r: R, alpha: f32, colour: impl Fn(Role) -> Color) -> bool {
    let Some(mut i) = first(name) else { return false };
    let (design, pad, spread) = (f32_at(12), f32_at(16), f32_at(20));
    // The design square fills `r`; the cell carries `pad` more each side.
    let side = r.w.min(r.h);
    let k = side / design;
    let (cx, cy) = (r.x + (r.w - side) * 0.5, r.y + (r.h - side) * 0.5);
    let cell = R::new(cx - pad * k, cy - pad * k, (design + 2.0 * pad) * k, (design + 2.0 * pad) * k);
    // Screen pixels per unit of the atlas's distance: 2 * spread texels, a texel k px.
    let scale = 2.0 * spread * k;
    let t = tex();
    let key = name.as_bytes();
    while i < n() && name_at(i) == key {
        let o = 28 + i * ENTRY;
        let uv = [f32_at(o + 36), f32_at(o + 40), f32_at(o + 44), f32_at(o + 48)];
        let c = fade(colour(role(u32_at(o + 32))), alpha);
        d.sdf_image(t, cell, uv, c, scale);
        i += 1;
    }
    true
}

fn luma(c: Color) -> f32 {
    let (r, g, b) = ((c >> 16 & 0xff) as f32, (c >> 8 & 0xff) as f32, (c & 0xff) as f32);
    (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
}

/// The colours of an icon on a button of the theme: `ink` its content colour. On a
/// bright button (dark ink: LCARS's pills) the colours are taken most of the way to
/// the ink, so a green plus or a gold bar still stands out from the fill.
pub fn theme_colour(theme: &crate::widgets::Theme, ink: Color) -> impl Fn(Role) -> Color + '_ {
    let dark = luma(ink) < 0.35;
    move |r| {
        let c = match r {
            Role::Ink => return ink,
            Role::Dim => return fade(ink, 0.5),
            Role::Good => theme.good,
            Role::Warn => theme.warn,
            Role::Bad => theme.bad,
            Role::Accent => theme.accent,
            Role::Resource(res) => crate::engine::topbar::colour(res),
        };
        let c = if dark { crate::gfx::mix(c, ink | 0xff000000, 0.45) } else { c };
        (c & 0x00ffffff) | (ink & 0xff000000)
    }
}
