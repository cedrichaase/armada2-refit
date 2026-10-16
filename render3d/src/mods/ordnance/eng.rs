//! The engine calls and fields the ordnance module reads (README "Ordnance"). Reads
//! only, except the one write `Ordnance::RenderAll` itself makes (`mod.rs`).
use crate::engine::{field, field_f32, field_u8, rd, vtable};
use crate::math::{M34, V3};

const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(int handle), cdecl
const FN_OBJ_EVEN_IF_DESTROYED: usize = 0x42bfa0; // GameObject::GetObjEvenIfDestroyed(int), cdecl
const FN_CAM_POS: usize = 0x6194d0; // ST3D_Camera::GetPosition() const, thiscall: Vector3*
const FN_PRJ_ODF: usize = 0x6593a0; // cPrjID::GetOdfName() const, thiscall: char*
const FN_TRANSFORM: usize = 0x4cfd50; // Entity::GetTransform() const, thiscall: Matrix34*
const FN_ODF_NAME: usize = 0x4d5620; // GameObject::GetOdfName() const, thiscall

/// The live ordnance: `Ordnance::ordnanceList` (a `std::list` at 0x771fa8, its head
/// node's pointer at +4), each node's +8 the ordnance.
pub const ORDNANCE_HEAD: usize = 0x771fac;

/// Calls `f` with each live ordnance, re-reading the list's head as `RenderAll` does.
pub fn for_each_ordnance(mut f: impl FnMut(usize)) {
    let mut node = field(unsafe { rd::<usize>(ORDNANCE_HEAD) }, 0);
    let mut guard = 0;
    loop {
        let head = unsafe { rd::<usize>(ORDNANCE_HEAD) };
        if node == 0 || node == head || guard > 100_000 {
            break;
        }
        f(field(node, 8));
        // The next node is read after the call, as RenderAll reads it.
        node = field(node, 0);
        guard += 1;
    }
}

pub fn entity_get(handle: i32) -> usize {
    unsafe {
        let f: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(FN_ENTITY_GET);
        f(handle)
    }
}

pub fn obj_even_if_destroyed(handle: i32) -> usize {
    unsafe {
        let f: unsafe extern "C" fn(i32) -> usize = core::mem::transmute(FN_OBJ_EVEN_IF_DESTROYED);
        f(handle)
    }
}

/// A live game object by handle (Entity::Get, and flagged a game object, +0x14 bit 4,
/// with an instance and a class), or 0.
pub fn object_of(handle: i32) -> usize {
    let o = entity_get(handle);
    if o == 0 || field(o, 0x14) & 4 == 0 || field(o, 4) == 0 || field(o, 0x40) == 0 {
        return 0;
    }
    o
}

/// A game object's bounding radius (its instance's sphere, +0x34 +12).
pub fn radius_of(o: usize) -> f32 {
    let r = field_f32(field(o, 4), 0x40);
    if r > 0.01 { r } else { 50.0 }
}

/// A game object's position (+0xac) and velocity (+0xdc).
pub fn obj_pos(o: usize) -> V3 {
    unsafe { rd(o + 0xac) }
}
pub fn obj_vel(o: usize) -> V3 {
    unsafe { rd(o + 0xdc) }
}

pub fn camera_position(cam: usize) -> V3 {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(FN_CAM_POS);
        core::ptr::read_unaligned(f(cam))
    }
}

/// An entity's transform (`Entity::GetTransform`): position at +0x24.
pub fn transform(e: usize) -> M34 {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const M34 = core::mem::transmute(FN_TRANSFORM);
        let p = f(e);
        if p.is_null() { M34::IDENTITY } else { core::ptr::read_unaligned(p) }
    }
}

/// The ODF a cPrjID names (`tracbm.odf`), or "".
pub fn prj_odf(prj: usize) -> &'static str {
    if prj == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_PRJ_ODF);
        crate::rt::cstr_at(f(prj), 260)
    }
}

/// An ordnance's ODF: its class (+0x34) keeps its cPrjID at +0x150.
pub fn ord_odf(ord: usize) -> &'static str {
    prj_odf(field(field(ord, 0x34), 0x150))
}

pub fn odf_name(o: usize) -> &'static str {
    if o == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_ODF_NAME);
        crate::rt::cstr_at(f(o), 260)
    }
}

/// `CanUserSee` (vtable slot 2) of any entity.
pub fn can_see(o: usize) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), 8));
        f(o) != 0
    }
}

/// The stem of an ODF name: `tracbm.odf` -> `tracbm`, lower case.
pub fn stem(odf: &str) -> alloc::string::String {
    let s = odf.rsplit(['\\', '/']).next().unwrap_or(odf);
    crate::rt::lower(s.split('.').next().unwrap_or(s))
}

pub fn byte(o: usize, off: usize) -> u8 {
    field_u8(o, off)
}
