//! Game objects as the renderer reads them (`GameObject`, an `Entity`). Only reads:
//! the simulation owns every one of these (`sim-render-coupling.md`).
use super::{field, field_u8, rd, vtable};
use crate::math::M34;

pub(crate) fn fn_transform() -> usize { at(sym::FN_TRANSFORM) } // Entity::GetTransform() const, thiscall: Matrix34*
pub(crate) fn fn_odf_name() -> usize { at(sym::FN_ODF_NAME) } // GameObject::GetOdfName() const, thiscall: char*

/// `GameObject::objectList`, every object the object loop visits.
pub use super::object_list;
use crate::engine::at;
use crate::syms::{field as off, sym};

/// The object's world transform (`Entity::GetTransform`).
pub fn transform(o: usize) -> M34 {
    if o == 0 {
        return M34::IDENTITY;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(fn_transform());
        let p = f(o);
        if p.is_null() { M34::IDENTITY } else { core::ptr::read_unaligned(p) }
    }
}

/// The object's ODF name, without extension (`fgalaxy`).
pub fn odf_name(o: usize) -> &'static str {
    if o == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_odf_name());
        let s = crate::rt::cstr_at(f(o), 260);
        // The engine's string carries the extension.
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

/// Whether the player can see it now (fog of war, `hidden` classes):
/// `GameObject::CanUserSee`, the test stock's `Render` makes first.
pub fn can_user_see(o: usize) -> bool {
    if o == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), off::VT_OBJ_CAN_SEE()));
        f(o) != 0
    }
}

/// The object's instance (`GameObjectInstance`, an `ST3D_Instance`): `+0x4`.
pub fn instance(o: usize) -> usize {
    field(o, off::OBJ_INSTANCE())
}

/// The object's flags word (`+0x14`; 0x10000 = background object).
pub fn flags(o: usize) -> u32 {
    field(o, off::OBJ_FLAGS()) as u32
}

/// Expired or hidden (`+0x27`): stock's `Render` draws nothing.
pub fn expired(o: usize) -> bool {
    field_u8(o, off::OBJ_EXPIRED()) != 0
}

/// The team number (`+0xec`).
pub fn team(o: usize) -> i32 {
    field(o, off::OBJ_TEAM()) as i32
}

/// Distance-culled for this view (`+0x183`, written by `PreRenderAll`).
pub fn culled(o: usize) -> bool {
    field_u8(o, off::OBJ_CULLED()) != 0
}

/// The class object (`+0x40`).
pub fn class(o: usize) -> usize {
    field(o, off::OBJ_CLASS())
}

/// The vtable, which tells the C++ class apart (`body-map.md` lists the ones that
/// override `Render`).
pub fn class_vtable(o: usize) -> usize {
    vtable(o)
}

/// An instance's distances from this view's camera, written by `PreRenderAll`:
/// (weighted for culling, true).
pub fn instance_distances(inst: usize) -> (f32, f32) {
    if inst == 0 {
        return (0.0, 0.0);
    }
    unsafe { (rd::<f32>(inst + off::INST_CULL_DISTANCE()), rd::<f32>(inst + off::INST_DISTANCE())) }
}
