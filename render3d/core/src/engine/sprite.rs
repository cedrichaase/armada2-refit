//! The engine's own sprites, as `scene::sprites` draws them: an `ST3D_Sprite` (the
//! ODF's `Sprite`, in the sprite table) either as a ribbon between two points
//! (`ST3D_Sprite::RenderAsPolygon` 0x63a810: phasers, pulses) or as a square facing the
//! camera (`ST3D_Sprite::DrawScaled3D` 0x63aa60: torpedoes, plasma, polerons,
//! artillery). Both end in `ST3D_Sprite::ClipAndRender3D` (0x63b4b0), which plays the
//! sprite's animation, takes its rectangle of the texture and draws one quad, its
//! texture times the colour, blended as the sprite's material says.
//!
//! What is read (README "Ordnance", the engine side):
//! - the sprite (the ordnance class +0x12c): its colour +0x24, its rectangle +0x38
//!   (u, v, w, h as fractions of the texture), its flags +0x50 (0x10: not animated;
//!   0x20: tiled along its length, `@tile_u`), its material +0x54 (blend factors
//!   +0x1c source, +0x20 destination, Direct3D's numbers), its texture +0x58 (named at
//!   +0x8, as every database element is);
//! - the animation: `ST3D_Sprite::Animate` (0x63a580) plays the sprite's channels and
//!   leaves the flipbook's offset in `ST3D_Sprite::m_texture_coordinate_offset`
//!   (0x7ad6d0). We call it where `ClipAndRender3D` would, so a flipbook shows the
//!   frame stock shows; it writes only the sprite's own drawing state.
//! - the brightness Storm3D +0x4 (times the colour) and the device's transparency +0x48
//!   (the vertex alpha).
use super::{field, field_f32, field_u8, rd, st3d_device, storm3d};
use crate::math::V3;
use alloc::string::String;

/// `ST3D_Sprite::DrawScaled3D`, the square facing the camera.
pub const FN_DRAW_SCALED_3D: usize = 0x63aa60;
const FN_ANIMATE: usize = 0x63a580; // ST3D_Sprite::Animate(), thiscall
const TEX_OFFSET: usize = 0x7ad6d0; // ST3D_Sprite::m_texture_coordinate_offset, Vector2
/// The time the engine's animation channels are evaluated at (`ST3D_AnimationChannel::
/// Evaluate` reads it; `SetStartTime` sets it from Storm3D's wall clock): under the bench
/// clock, the sim time while we call `Animate`, then put back (README "The animation clock").
const ANIM_TIME: usize = 0x7a8e74;
const FN_CAM_POS: usize = 0x6194d0; // ST3D_Camera::GetPosition() const, thiscall: Vector3*

/// `D3DBLEND_ONE`.
pub const BLEND_ONE: u32 = 2;

/// A sprite's look now.
pub struct Look {
    /// The texture's file name without a path or an extension.
    pub texture: String,
    /// The rectangle (after its animation): u, v, width, height.
    pub rect: [f32; 4],
    /// Direct3D's source and destination blend codes (`BLEND_ONE` for both when the
    /// material is missing or odd).
    pub src: u32,
    pub dst: u32,
    pub flags: u8,
}

/// The sprite's look now: texture, rectangle (after its animation), blend, flags.
pub fn look(sprite: usize) -> Option<Look> {
    let tex = field(sprite, 0x58);
    let name = unsafe { crate::rt::cstr_at(field(tex, 8) as *const u8, 260) };
    if name.is_empty() {
        return None;
    }
    // The file name without a path or an extension.
    let base = name.rsplit(['\\', '/']).next().unwrap_or(name);
    let stem = base.split('.').next().unwrap_or(base);
    let flags = field_u8(sprite, 0x50);
    if flags & 0x10 == 0 {
        unsafe {
            let f: unsafe extern "thiscall" fn(usize) = core::mem::transmute(FN_ANIMATE);
            if crate::clock::bench() {
                let t = ANIM_TIME as *mut f32;
                let saved = t.read_unaligned();
                t.write_unaligned(crate::clock::anim_now() as f32);
                f(sprite);
                t.write_unaligned(saved);
            } else {
                f(sprite);
            }
        }
    }
    let off: [f32; 2] = unsafe { rd(TEX_OFFSET) };
    let r: [f32; 4] = unsafe { rd(sprite + 0x38) };
    let mat = field(sprite, 0x54);
    let (mut src, mut dst) = (field(mat, 0x1c) as u32, field(mat, 0x20) as u32);
    if mat == 0 || !(1..=13).contains(&src) || !(1..=13).contains(&dst) {
        src = BLEND_ONE;
        dst = BLEND_ONE;
    }
    Some(Look { texture: String::from(stem), rect: [off[0] + r[0], off[1] + r[1], r[2], r[3]], src, dst, flags })
}

/// The width of one repeat of a tiled sprite (`@tile_u`, `+0x40`).
pub fn tile_width(sprite: usize) -> f32 {
    field_f32(sprite, 0x40)
}

/// The vertex colour stock gives a sprite: its colour times the engine's brightness,
/// the device's transparency as alpha.
pub fn colour(col: V3) -> [f32; 4] {
    let b = field_f32(storm3d(), 4);
    let a = field_f32(st3d_device(), 0x48);
    let c = |x: f32| (x * b).clamp(0.0, 1.0);
    [c(col.x), c(col.y), c(col.z), a.clamp(0.0, 1.0)]
}

/// The scale of the instance being drawn (Storm3D +0x100, its +0x28), when it is not 1.
pub fn draw_scale() -> Option<f32> {
    let inst = field(storm3d(), 0x100);
    let k = field_f32(inst, 0x28);
    (inst != 0 && k != 1.0).then_some(k)
}

/// A camera's position (`ST3D_Camera::GetPosition`).
pub fn camera_position(cam: usize) -> V3 {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(FN_CAM_POS);
        core::ptr::read_unaligned(f(cam))
    }
}
