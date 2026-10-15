//! Game objects as the renderer reads them (`GameObject`, an `Entity`). Only reads:
//! the simulation owns every one of these (`sim-render-coupling.md`).
use super::{field, field_u8, rd, vtable};
use crate::math::M34;

const FN_TRANSFORM: usize = 0x4cfd50; // Entity::GetTransform() const, thiscall: Matrix34*
const FN_ODF_NAME: usize = 0x4d5620; // GameObject::GetOdfName() const, thiscall: char*
const SLOT_CAN_SEE: usize = 2; // GameObject::CanUserSee() const

/// `GameObject::objectList`, every object the object loop visits.
pub use super::OBJECT_LIST;

/// The object's world transform (`Entity::GetTransform`).
pub fn transform(o: usize) -> M34 {
    if o == 0 {
        return M34::IDENTITY;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(FN_TRANSFORM);
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
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_ODF_NAME);
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
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), SLOT_CAN_SEE * 4));
        f(o) != 0
    }
}

/// The object's instance (`GameObjectInstance`, an `ST3D_Instance`): `+0x4`.
pub fn instance(o: usize) -> usize {
    field(o, 4)
}

/// The object's flags word (`+0x14`; 0x10000 = background object).
pub fn flags(o: usize) -> u32 {
    field(o, 0x14) as u32
}

/// Expired or hidden (`+0x27`): stock's `Render` draws nothing.
pub fn expired(o: usize) -> bool {
    field_u8(o, 0x27) != 0
}

/// The class object (`+0x40`).
pub fn class(o: usize) -> usize {
    field(o, 0x40)
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
    unsafe { (rd::<f32>(inst + 0x88), rd::<f32>(inst + 0x8c)) }
}
