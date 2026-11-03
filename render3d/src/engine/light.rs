//! The engine's own point lights, as the modules make theirs: an ordinary
//! `ST3D_Point_Light` made once and registered each view after `GameObject_PreRenderAll`
//! (`scene::lights::Pool`).
use crate::math::M34;

pub const FN_NEW: usize = 0x652710; // operator new, cdecl
pub const FN_POINT_LIGHT: usize = 0x62f240; // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
pub const FN_REGISTER_LIGHT: usize = 0x62d250; // ST3D_GraphicsEngine::RegisterLight(light, colour&, Matrix34&), thiscall

/// A new point light named `name` (a NUL-terminated byte string); 0 if the engine could
/// not allocate one.
pub fn new_point_light(name: &'static [u8]) -> usize {
    unsafe {
        let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(FN_NEW);
        let l = new(0x138);
        if l == 0 {
            return 0;
        }
        let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(FN_POINT_LIGHT);
        ctor(l, 0, 0, name.as_ptr());
        l
    }
}

/// Full colour out to `start`, gone `range` further on (`+0x100`, `+0x104`).
pub fn set_reach(light: usize, start: f32, range: f32) {
    unsafe {
        core::ptr::write_unaligned((light + 0x100) as *mut f32, start);
        core::ptr::write_unaligned((light + 0x104) as *mut f32, range);
    }
}

/// `ST3D_GraphicsEngine::RegisterLight`.
pub fn register(light: usize, colour: &[f32; 4], at: &M34) {
    unsafe {
        let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
        reg(super::storm3d(), light, colour.as_ptr(), at);
    }
}

/// The map's background, as `Starfield_Load_Background_Geometry` keeps its name at
/// 0x738538: a prefix of six faces, or a cube SOD (`scene::lights`, the sky's light).
pub fn background_name() -> &'static str {
    unsafe { crate::rt::cstr_at(0x738538 as *const u8, 63) }
}
