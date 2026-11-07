//! The game's 2D sprites (`ST3D_Sprite`), drawn with our renderer: a HUD icon is the
//! game's own art, loaded from the same file the engine uses, TGA or DDS (HUD.md, "Icons").
//!
//! A sprite (a Storm3D database element, its `.spr` name at `+0x8`) draws a rectangle
//! of one texture: `ST3D_Sprite::DrawScaled2D` hands the device the texture at `+0x58`
//! and the texture rectangle at `+0x38` (u, v, then its width and height, in texture
//! coordinates, 0..1). A texture is a database element too, named (`+0x8`) after its
//! file in `Textures/RGB/`.
use super::{field, rd};
use crate::gfx::Tex;
use crate::syms_core::field as off;
use alloc::string::String;
use alloc::vec::Vec;

/// The sprite's name, from its `.spr`.
pub fn name(sprite: usize) -> &'static str {
    unsafe { super::hud::cstr_static(field(sprite, off::CORE_ELEMENT_NAME()) as *const u8) }
}

/// The name of the texture the sprite draws from.
pub fn texture_name(sprite: usize) -> &'static str {
    let t = field(sprite, off::CORE_SPRITE_TEXTURE());
    unsafe { super::hud::cstr_static(field(t, off::CORE_ELEMENT_NAME()) as *const u8) }
}

/// The sprite's texture rectangle: u0, v0, u1, v1.
pub fn uv(sprite: usize) -> [f32; 4] {
    let [u, v, du, dv]: [f32; 4] = unsafe { rd(sprite + off::CORE_SPRITE_UV()) };
    [u, v, u + du, v + dv]
}

struct Loaded {
    name: String,
    tex: Option<Tex>,
    /// The texture's alpha, kept for `own_tile` (one byte a texel, top row first), and its size.
    alpha: Vec<u8>,
    w: u32,
    h: u32,
}

static mut CACHE: Vec<Loaded> = Vec::new();

/// The texture and rectangle to draw `sprite` with, loading its file the first time.
///
/// A sprite the engine has not drawn yet may have no texture or no rectangle yet (both are
/// filled when its texture is first demand-loaded): seen on Roots, whose own HUD draws the
/// top-right buttons and the minimap's filters with art of FleetOpsHook's, so the stock
/// sprites behind them were never drawn. Then both come from the sprite's line in the game's
/// `.spr` files, by its name (`spr_entry`), read from the player's install (on a game whose
/// own icons go first, `files::Game::game_icons`; Armada II's draws are unchanged).
#[allow(static_mut_refs)]
pub fn art(sprite: usize) -> Option<(Tex, [f32; 4])> {
    if sprite == 0 {
        return None;
    }
    let has_tex = field(sprite, off::CORE_SPRITE_TEXTURE()) != 0;
    let engine_tn = if has_tex { texture_name(sprite) } else { "" };
    let mut rect = uv(sprite);
    let mut tn = String::from(engine_tn);
    // Only where the game's own icons go first (Roots): Armada II draws as before.
    if (tn.is_empty() || rect == [0.0; 4]) && crate::files::game().game_icons {
        let found = spr_entry(name(sprite), engine_tn);
        note_spr(name(sprite), engine_tn, found.as_ref());
        let (t, r) = found?;
        if tn.is_empty() {
            tn = t;
        }
        rect = r;
        // A line with no area (Roots blanks the minimap's filter buttons so) is no art.
        if rect[2] <= rect[0] || rect[3] <= rect[1] {
            return None;
        }
    }
    if tn.is_empty() {
        return None;
    }
    let cache = unsafe { &mut CACHE };
    let i = match cache.iter().position(|l| l.name == tn) {
        Some(i) => i,
        None => {
            let img = load(&tn);
            let (tex, alpha, w, h) = match img {
                Some((t, a, w, h)) => (Some(t), a, w, h),
                None => (None, Vec::new(), 0, 0),
            };
            crate::log!("sprite: {} -> texture {} {} uv {:?}{}", name(sprite), tn,
                match tex { Some(t) => { let (w, h) = crate::gfx::tex_size(t); crate::sfmt!("{}x{}", w, h) } None => crate::sfmt!("NOT FOUND") },
                rect, if has_tex && rect == uv(sprite) { "" } else { " (from its .spr line)" });
            cache.push(Loaded { name: tn, tex, alpha, w, h });
            cache.len() - 1
        }
    };
    cache[i].tex.map(|t| (t, rect))
}

/// Whether the sprite's art carries a tile of its own (FO-6, decision 344): Roots' button
/// sheets draw each icon on a rounded translucent plate filling the sprite, so a plate of ours
/// behind it reads as a tile inside a tile. Measured on the art itself, per sprite: the band
/// a sixth in from each edge, between the corners, is covered (alpha over an eighth: Roots' plates are translucent, about 50 to 60 inside a brighter rim) along at
/// least 90% of its length on all four sides. A glyph on a clear ground covers little of it.
/// Kept per sprite name.
#[allow(static_mut_refs)]
pub fn own_tile(sprite: usize) -> bool {
    static mut SEEN: Vec<(String, bool)> = Vec::new();
    let n = name(sprite);
    if sprite == 0 || n.is_empty() {
        return false;
    }
    unsafe {
        if let Some((_, v)) = SEEN.iter().find(|(k, _)| k == n) {
            return *v;
        }
    }
    let Some((_, uv)) = art(sprite) else { return false };
    let tn = if field(sprite, off::CORE_SPRITE_TEXTURE()) != 0 { String::from(texture_name(sprite)) } else { String::new() };
    let cache = unsafe { &CACHE };
    // The texture `art` loaded for it: the engine's name, else the .spr line's.
    let l = cache.iter().find(|l| !tn.is_empty() && l.name == tn).or_else(|| spr_entry(n, "").and_then(|(t, _)| cache.iter().find(|l| l.name == t)));
    let v = match l {
        Some(l) if !l.alpha.is_empty() => tile_band(&l.alpha, l.w, l.h, uv),
        _ => false,
    };
    crate::log!("sprite: {} {}", n, if v { "carries its own tile" } else { "has no tile of its own" });
    unsafe { SEEN.push((String::from(n), v)) };
    v
}

fn tile_band(alpha: &[u8], w: u32, h: u32, uv: [f32; 4]) -> bool {
    let (x0, y0) = (uv[0] * w as f32, uv[1] * h as f32);
    let (sw, sh) = ((uv[2] - uv[0]) * w as f32, (uv[3] - uv[1]) * h as f32);
    if sw < 8.0 || sh < 8.0 {
        return false;
    }
    let at = |fx: f32, fy: f32| -> u8 {
        let x = ((x0 + fx * sw) as u32).min(w - 1);
        let y = ((y0 + fy * sh) as u32).min(h - 1);
        alpha[(y * w + x) as usize]
    };
    let (mut hit, mut all) = (0u32, 0u32);
    for k in 0..16 {
        let f = 0.25 + 0.5 * k as f32 / 15.0;
        for (fx, fy) in [(f, 1.0 / 6.0), (f, 5.0 / 6.0), (1.0 / 6.0, f), (5.0 / 6.0, f)] {
            all += 1;
            if at(fx, fy) > 32 {
                hit += 1;
            }
        }
    }
    hit * 10 >= all * 9
}

/// Logs once per sprite what its `.spr` line gave (Roots: the sprites the engine had not drawn).
#[allow(static_mut_refs)]
fn note_spr(sprite: &str, texture: &str, found: Option<&(String, [f32; 4])>) {
    static mut SEEN: Vec<String> = Vec::new();
    unsafe {
        if SEEN.len() >= 64 || SEEN.iter().any(|n| n == sprite) {
            return;
        }
        SEEN.push(String::from(sprite));
    }
    match found {
        Some((t, r)) => crate::log!("sprite: {} not drawn by the engine yet: its .spr line, {} uv {:?}", sprite, t, r),
        None => crate::log!("sprite: {} not drawn by the engine yet and no .spr line names it{}{}", sprite, if texture.is_empty() { "" } else { " on " }, texture),
    }
}

/// The game's `.spr` files (`Sprites\*.spr`, each with what it includes), read once.
static mut SPR: Option<Vec<(String, a2fmt::spr::SpriteTable)>> = None;

#[allow(static_mut_refs)]
fn spr_tables() -> &'static [(String, a2fmt::spr::SpriteTable)] {
    unsafe {
        if SPR.is_none() {
            let mut v = Vec::new();
            for (f, dir) in crate::rt::list_entries(&crate::rt::game_path("Sprites\\*.spr")) {
                if dir {
                    continue;
                }
                let t = a2fmt::spr::SpriteTable::load(&f, |n| crate::files::open(&crate::sfmt!("Sprites\\{}", n)));
                v.push((crate::rt::lower(&f), t));
            }
            SPR = Some(v);
        }
        SPR.as_deref().unwrap_or(&[])
    }
}

/// A sprite's texture and rectangle (0..1) from its `.spr` line: the line whose texture is
/// `texture` when the engine named one, else the mission race's GUI sprites first
/// (`gui_<race>.spr`, as the engine loads them per race), then any file.
fn spr_entry(sprite_name: &str, texture: &str) -> Option<(String, [f32; 4])> {
    if sprite_name.is_empty() {
        return None;
    }
    let race = crate::sfmt!("gui_{}.spr", crate::rt::lower(super::race::mission_race_name()));
    let tables = spr_tables();
    let ok = |s: &&a2fmt::spr::Sprite| texture.is_empty() || s.texture.eq_ignore_ascii_case(texture);
    let pick = |want: &dyn Fn(&str) -> bool| {
        tables.iter().filter(|(f, _)| want(f)).find_map(|(_, t)| t.sprite(sprite_name).filter(ok).and_then(|s| s.uv_rect().map(|r| (s.texture.clone(), r))))
    };
    let shared = crate::rt::lower(crate::content::get().files.gui_sprites);
    let (t, r) = pick(&|f: &str| f == race).or_else(|| pick(&|f: &str| !shared.is_empty() && f == shared)).or_else(|| pick(&|_: &str| true))?;
    Some((t, [r[0], r[1], r[0] + r[2], r[1] + r[3]]))
}

/// `textures\<name>` (`files`), with the game's texture extensions in its order (`files::game`:
/// `.tga` on Armada II; `.tga`, then `.dds` on Fleet Operations: Roots, whose interface
/// buttons are mostly DDS, `all_interface_buttons03` to `11`), either case, or the name as
/// given when it has an extension. TGA or DDS is told by the bytes (`a2fmt::image`); the alpha is kept for `own_tile`.
fn load(name: &str) -> Option<(Tex, Vec<u8>, u32, u32)> {
    let lower = crate::rt::lower(name);
    let one = |rel: &str| -> Option<(Tex, Vec<u8>, u32, u32)> {
        let data = crate::files::open(rel)?;
        let img = a2fmt::image::decode(&data).ok()?;
        let alpha: Vec<u8> = img.pixels.chunks_exact(4).map(|p| p[3]).collect();
        let (w, h) = (img.width, img.height);
        Some((crate::gfx::texture_bgra(w, h, img.pixels), alpha, w, h))
    };
    if lower.ends_with(".tga") || lower.ends_with(".dds") {
        return one(&crate::sfmt!("textures\\{}", name));
    }
    for ext in crate::files::game().texture_exts {
        for e in [String::from(*ext), ext.to_ascii_uppercase()] {
            if let Some(t) = one(&crate::sfmt!("textures\\{}{}", name, e)) {
                return Some(t);
            }
        }
    }
    None
}
