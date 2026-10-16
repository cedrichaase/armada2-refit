//! The game's 2D sprites (`ST3D_Sprite`), drawn with our renderer: a HUD icon is the
//! game's own art, loaded from the same TGA the engine uses (HUD.md, "Icons").
//!
//! A sprite (a Storm3D database element, its `.spr` name at `+0x8`) draws a rectangle
//! of one texture: `ST3D_Sprite::DrawScaled2D` hands the device the texture at `+0x58`
//! and the texture rectangle at `+0x38` (u, v, then its width and height, in texture
//! coordinates, 0..1). A texture is a database element too, named (`+0x8`) after its
//! file in `Textures/RGB/`.
use super::{field, rd};
use crate::gfx::Tex;
use alloc::string::String;
use alloc::vec::Vec;

const ELEMENT_NAME: usize = 0x08;
const SPRITE_UV: usize = 0x38;
const SPRITE_TEXTURE: usize = 0x58;

/// The sprite's name, from its `.spr`.
pub fn name(sprite: usize) -> &'static str {
    unsafe { super::hud::cstr_static(field(sprite, ELEMENT_NAME) as *const u8) }
}

/// The name of the texture the sprite draws from.
pub fn texture_name(sprite: usize) -> &'static str {
    let t = field(sprite, SPRITE_TEXTURE);
    unsafe { super::hud::cstr_static(field(t, ELEMENT_NAME) as *const u8) }
}

/// The sprite's texture rectangle: u0, v0, u1, v1.
pub fn uv(sprite: usize) -> [f32; 4] {
    let [u, v, du, dv]: [f32; 4] = unsafe { rd(sprite + SPRITE_UV) };
    [u, v, u + du, v + dv]
}

struct Loaded {
    name: String,
    tex: Option<Tex>,
}

static mut CACHE: Vec<Loaded> = Vec::new();

/// The texture and rectangle to draw `sprite` with, loading its TGA the first time.
#[allow(static_mut_refs)]
pub fn art(sprite: usize) -> Option<(Tex, [f32; 4])> {
    if sprite == 0 || field(sprite, SPRITE_TEXTURE) == 0 {
        return None;
    }
    let tn = texture_name(sprite);
    if tn.is_empty() {
        return None;
    }
    let cache = unsafe { &mut CACHE };
    let i = match cache.iter().position(|l| l.name == tn) {
        Some(i) => i,
        None => {
            let tex = load(tn);
            crate::log!("sprite: {} -> texture {} {} uv {:?}", name(sprite), tn,
                match tex { Some(t) => { let (w, h) = crate::gfx::tex_size(t); crate::sfmt!("{}x{}", w, h) } None => crate::sfmt!("NOT FOUND") },
                uv(sprite));
            cache.push(Loaded { name: String::from(tn), tex });
            cache.len() - 1
        }
    };
    cache[i].tex.map(|t| (t, uv(sprite)))
}

/// `Textures/RGB/<name>`, either case of the extension, or the name as given.
fn load(name: &str) -> Option<Tex> {
    let lower = crate::rt::lower(name);
    if lower.ends_with(".tga") {
        return crate::gfx::load_tga(&crate::sfmt!("Textures\\RGB\\{}", name));
    }
    for ext in [".tga", ".TGA"] {
        if let Some(t) = crate::gfx::load_tga(&crate::sfmt!("Textures\\RGB\\{}{}", name, ext)) {
            return Some(t);
        }
    }
    None
}
