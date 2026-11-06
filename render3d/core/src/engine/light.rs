//! The engine's own point lights, as the modules make theirs: an ordinary
//! `ST3D_Point_Light` made once and registered each view after `GameObject_PreRenderAll`
//! (`scene::lights::Pool`).
use crate::math::M34;
use crate::engine::at;
use crate::syms::{field as off, sym, value};

pub fn fn_new() -> usize { at(sym::FN_NEW) } // operator new, cdecl
pub fn fn_point_light() -> usize { at(sym::FN_POINT_LIGHT) } // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
pub fn fn_register_light() -> usize { at(sym::FN_REGISTER_LIGHT) } // ST3D_GraphicsEngine::RegisterLight(light, colour&, Matrix34&), thiscall

/// A new point light named `name` (a NUL-terminated byte string); 0 if the engine could
/// not allocate one.
pub fn new_point_light(name: &'static [u8]) -> usize {
    unsafe {
        let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(fn_new());
        let l = new(value::POINT_LIGHT_BYTES() as u32);
        if l == 0 {
            return 0;
        }
        let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(fn_point_light());
        ctor(l, 0, 0, name.as_ptr());
        l
    }
}

/// Full colour out to `start`, gone `range` further on (`+0x100`, `+0x104`).
pub fn set_reach(light: usize, start: f32, range: f32) {
    unsafe {
        core::ptr::write_unaligned((light + off::LIGHT_START()) as *mut f32, start);
        core::ptr::write_unaligned((light + off::LIGHT_RANGE()) as *mut f32, range);
    }
}

/// `ST3D_GraphicsEngine::RegisterLight`.
pub fn register(light: usize, colour: &[f32; 4], at: &M34) {
    unsafe {
        let reg: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(fn_register_light());
        reg(super::storm3d(), light, colour.as_ptr(), at);
    }
}

/// The map's background, as `Starfield_Load_Background_Geometry` keeps its name at
/// `background_name`: a prefix of six faces, or a cube SOD (`scene::lights`, the sky's light).
pub fn background_name() -> &'static str {
    unsafe { crate::rt::cstr_at(super::at(crate::syms::sym::BACKGROUND_NAME) as *const u8, 63) }
}
