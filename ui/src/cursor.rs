//! Our own mouse cursor, drawn at the back buffer's resolution from the game's own cursor
//! art. In a mission it is the sprite the engine chose (`engine::cursor::shape`) and the
//! engine's is hidden for the frame; in a screen of ours and on the loading screen it is
//! the arrow. Off unless `[UI] Cursor=1` (README.md, "The cursor").
//!
//! The sprites come from `Sprites/cursor.spr`, read at run time: each line names a
//! texture, a rectangle in it (in a reference size, so as fractions of the texture),
//! how many frames of the strip it animates (`@anim=cursorNx1`) and its hotspot
//! (`@origin`, in texels of the 32x32 sprite). The art is the stock art: when the
//! texture has a `.a2neb-backup` beside it (an earlier run of `hud/cursor-aspect.py`
//! squashed it), that is read instead. Black is the colour key, as in the engine.
use crate::gfx::{self, Tex, R};
use crate::surface::Context;
use crate::widgets::Ui;
use alloc::string::String;
use alloc::vec::Vec;

struct Def {
    name: String,
    tex: Tex,
    /// u0 v0 u1 v1 of the first frame.
    uv: [f32; 4],
    /// How far the next frame is, in u.
    step_u: f32,
    frames: u32,
    /// Hotspot in texels of the sprite.
    origin: (f32, f32),
    /// The sprite's size in texels.
    size: (f32, f32),
}

static mut DEFS: Option<Vec<Def>> = None;

fn parse_pair(s: &str) -> Option<(f32, f32)> {
    let s = s.trim_start_matches('(').trim_end_matches(')');
    let (a, b) = s.split_once(',')?;
    Some((crate::rt::parse_f32(a.trim())?, crate::rt::parse_f32(b.trim())?))
}

/// A texture of the cursor art, black keyed to transparent.
fn load_texture(file: &str) -> Option<Tex> {
    let base = crate::sfmt!("Textures/RGB/{}.tga", file);
    let backup = crate::sfmt!("{}.a2neb-backup", base);
    let data = crate::rt::read_file(&crate::rt::game_path(&backup)).or_else(|| crate::rt::read_file(&crate::rt::game_path(&base)))?;
    let mut img = a2fmt::tga::decode(&data).ok()?;
    for px in img.pixels.chunks_exact_mut(4) {
        if px[0] == 0 && px[1] == 0 && px[2] == 0 {
            px[3] = 0;
        }
    }
    Some(gfx::texture_bgra(img.width, img.height, img.pixels))
}

fn load() -> Vec<Def> {
    let mut defs = Vec::new();
    let Some(data) = crate::rt::read_file(&crate::rt::game_path("Sprites/cursor.spr")) else {
        crate::log!("cursor: Sprites/cursor.spr not found");
        return defs;
    };
    let text = String::from_utf8_lossy(&data).into_owned();
    let (mut rw, mut rh) = (1.0f32, 1.0f32);
    let mut textures: Vec<(String, Option<Tex>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(v) = line.strip_prefix("@referenceWidth=") {
            rw = crate::rt::parse_f32(v.trim()).unwrap_or(rw);
            continue;
        }
        if let Some(v) = line.strip_prefix("@referenceHeight=") {
            rh = crate::rt::parse_f32(v.trim()).unwrap_or(rh);
            continue;
        }
        if line.starts_with('@') {
            continue;
        }
        let tok: Vec<&str> = line.split_whitespace().collect();
        if tok.len() < 6 {
            continue;
        }
        let num = |i: usize| crate::rt::parse_f32(tok[i]);
        let (Some(u), Some(v), Some(w), Some(h)) = (num(2), num(3), num(4), num(5)) else { continue };
        let (mut frames, mut origin) = (1u32, (0.0, 0.0));
        for x in &tok[6..] {
            if let Some(a) = x.strip_prefix("@anim=cursor") {
                frames = a.split('x').next().and_then(crate::rt::parse_i32).unwrap_or(1).max(1) as u32;
            } else if let Some(o) = x.strip_prefix("@origin=") {
                origin = parse_pair(o).unwrap_or(origin);
            }
        }
        let file = crate::rt::lower(tok[1]);
        let at = match textures.iter().position(|(f, _)| *f == file) {
            Some(i) => i,
            None => {
                textures.push((file.clone(), load_texture(&file)));
                textures.len() - 1
            }
        };
        let Some(tex) = textures[at].1 else { continue };
        defs.push(Def {
            name: crate::rt::lower(tok[0]),
            tex,
            uv: [u / rw, v / rh, (u + w) / rw, (v + h) / rh],
            step_u: w / rw,
            frames,
            origin,
            size: (w, h),
        });
    }
    crate::log!("cursor: {} sprites from cursor.spr", defs.len());
    defs
}

#[allow(static_mut_refs)]
fn defs() -> &'static [Def] {
    unsafe {
        if DEFS.is_none() {
            DEFS = Some(load());
        }
        DEFS.as_deref().unwrap_or(&[])
    }
}

fn find(name: &str) -> Option<&'static Def> {
    let n = crate::rt::lower(name);
    defs().iter().find(|d| d.name == n)
}

/// A cursor sprite's first frame as a texture and its u0 v0 u1 v1 (for an image
/// button or a gallery), or None if the art is missing.
pub fn sprite(name: &str) -> Option<(Tex, [f32; 4])> {
    find(name).map(|d| (d.tex, d.uv))
}

static mut LAST: String = String::new();

/// UI.log says each change of the cursor drawn (the bench reads it).
#[allow(static_mut_refs)]
fn note_shape(ctx: Context, name: &str) {
    unsafe {
        if LAST != name {
            LAST = String::from(name);
            crate::log!("cursor: {:?} draws {}", ctx, name);
        }
    }
}

/// Draws the cursor on top of this frame's UI, and hides the one that would double it.
pub fn draw(ctx: Context, ui: &mut Ui) {
    if !crate::config::get().cursor {
        return;
    }
    let Some((mx, my)) = ui.input.mouse else { return };
    let name = match ctx {
        Context::Mission => {
            // Ours only where the engine's can be hidden, or there would be two.
            if !crate::engine::cursor::installed() || !crate::engine::cursor::synchronous() {
                return;
            }
            // None: the engine shows no cursor (a right-drag); neither do we.
            match crate::engine::cursor::shape() {
                Some(n) => n,
                None => return,
            }
        }
        _ => "c_arrow",
    };
    let Some(d) = find(name).or_else(|| find("c_arrow")) else { return };
    note_shape(ctx, name);
    // 32 texels at the configured size; near a whole multiple it is snapped to it, so
    // the art lands on the pixel grid.
    let want = ui.u(crate::config::get().cursor_size);
    let k = want / 32.0;
    let snapped = libm::roundf(k);
    let k = if snapped >= 1.0 && (k - snapped).abs() < 0.2 * snapped { snapped } else { k };
    let (w, h) = (d.size.0 * k, d.size.1 * k);
    let x = libm::floorf(mx) - libm::roundf(d.origin.0 * k);
    let y = libm::floorf(my) - libm::roundf(d.origin.1 * k);
    let mut uv = d.uv;
    if d.frames > 1 {
        // The engine steps its strips through their frames once a second.
        let f = (ui.time() * d.frames as f64) as u32 % d.frames;
        let du = d.step_u * f as f32;
        uv[0] += du;
        uv[2] += du;
    }
    ui.draw.image_uv(d.tex, R::new(x, y, w, h), uv, 0xffffffff);
    match ctx {
        Context::Mission => crate::engine::cursor::hide_this_frame(),
        Context::Shell => crate::engine::shell::hide_os_cursor(),
        Context::Loading => {}
    }
}
