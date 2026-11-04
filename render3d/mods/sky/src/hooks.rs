//! The engine glue of the sky module (README.md, "Hooks"): every address, global,
//! function and struct offset it uses, behind named accessors. Nothing else in the
//! module names one (`render3d/lint-layers.sh --strict src/mods/sky`).
//!
//! The sky installs no hook: it takes `Step::Background` (the whole of
//! `Background_Render`) and the background objects (`Module::object`) from the seam the
//! core gives every module, and hands a background object it cannot draw back to the
//! engine's own `Render`. So there is nothing to gate; a module that is off is simply
//! not dispatched, and the engine draws its own background.
//!
//! What it reads: the map's background name (kept lower-cased by
//! `Starfield_Load_Background_Geometry`), `g_background_geometry_db` (stock draws the
//! sky only with one), `g_starfield` and its octree of stars, the two far planes
//! (`STAR_FAR_CLIP_PLANE`, `FAR_CLIPPING_PLANE`), the detail configuration byte that
//! turns the sky off, the user profile's edit-mode byte, the background instance (its
//! turn and scale), and a background object's class (`hidden`), instance and database.
use r3d_core::engine::{self, field, field_m34, field_u8, rd};
use r3d_core::math::{M34, V3};
use alloc::string::String;
use crate::syms::{sym, TABLE};

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { TABLE.engine().addr(s) }

/// The map's background name, lower-cased, as `Starfield_Load_Background_Geometry`
/// keeps it: `<name>.sod` (a cube model) or a bare prefix (six faces `<prefix>0..5`).
pub(crate) fn bg_name() -> usize { at(sym::BG_NAME) }
/// `g_background_geometry_db`: the sky's database; stock draws the sky only with one.
pub(crate) fn g_bg_db() -> usize { at(sym::G_BG_DB) }
/// `g_starfield`.
pub(crate) fn g_starfield() -> usize { at(sym::G_STARFIELD) }
/// `STAR_FAR_CLIP_PLANE` and `FAR_CLIPPING_PLANE` (`ART_CFG.h`; QOL may scale the
/// second in memory): the camera's far plane during the background, and after.
pub(crate) fn star_far_addr() -> usize { at(sym::STAR_FAR) }
pub(crate) fn far_plane_addr() -> usize { at(sym::FAR_PLANE) }
/// `Starfield_Load_Background_Geometry(char*)`, cdecl: a new background (bench only).
pub(crate) fn fn_load_bg() -> usize { at(sym::FN_LOAD_BG) }
pub(crate) fn fn_set_far() -> usize { at(sym::FN_SET_FAR) } // ST3D_Camera::SetFarPlane(float), thiscall
/// `UserProfile*`; its byte +0x84 is edit mode (`Program::Toggle_Edit_Mode`).
pub(crate) fn user_profile() -> usize { at(sym::USER_PROFILE) }
/// The class's `hidden` flag (`dlight` has it): not drawn outside edit mode.
const CLASS_HIDDEN: usize = 0x19c;
/// Object flags: a background object, drawn here and skipped by the object loop.
const OBJ_BACKGROUND: u32 = 0x10000;
pub(crate) fn fn_go_render() -> usize { at(sym::FN_GO_RENDER) } // GameObject::Render, slot 11 of the classes that keep it
const SLOT_RENDER: usize = 11;
pub(crate) fn vt_database() -> usize { at(sym::VT_DATABASE) } // a plain ST3D_Database
/// The background instance (`g_background_instance`), whose turn and scale are its own.
pub(crate) fn bg_instance() -> usize { at(sym::BG_INSTANCE) }
/// `ST3D_GraphicsEngine+0x104`: the detail configuration, whose byte +0x2c equal to 2
/// turns the sky off (`Background_Render`'s own test).
const S3D_DETAIL: usize = 0x104;
const DETAIL_SKY: usize = 0x2c;
/// The `RenderState` bytes `Background_Render` reads: the sky, and the stars.
const RS_SKY: usize = 4;
const RS_STARS: usize = 8;
/// The camera's clip rectangle (`+0x1b0`, four floats) the depth is cleared over.
const CAM_RECT: usize = 0x1b0;

unsafe fn this0(f: usize, this: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize)>(f)(this) }
}
unsafe fn this1(f: usize, this: usize, a: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize)>(f)(this, a) }
}

/// The map's background name.
pub fn background_name() -> String {
    unsafe { String::from(crate::rt::cstr_at(bg_name() as *const u8, 64)) }
}

/// Whether the map has a sky database (stock draws the sky only with one).
pub fn has_sky_db() -> bool {
    unsafe { rd::<usize>(g_bg_db()) != 0 }
}

/// `g_starfield` (0: the map has none).
pub fn starfield() -> usize {
    unsafe { rd::<usize>(g_starfield()) }
}

/// The starfield's octree root.
pub fn starfield_root(sf: usize) -> usize {
    field(sf, 4)
}

/// An octree node's objects (a `std::list` head) and its children's.
pub fn node_objects(node: usize) -> usize {
    field(node, 0x30)
}
pub fn node_children(node: usize) -> usize {
    field(node, 0x3c)
}

/// One `Starfield` star (`mCreateStarSprite`): its `ST3D_Sprite*`, its four corners in
/// world space, and (of the sprite) its name and colour.
pub fn star_sprite(star: usize) -> usize {
    field(star, 0)
}
pub fn star_corners(star: usize) -> [V3; 4] {
    unsafe { rd(star + 0xc) }
}
pub fn sprite_name(sp: usize) -> &'static str {
    unsafe { crate::rt::cstr_at(field(sp, 8) as *const u8, 64) }
}
pub fn sprite_colour(sp: usize) -> [f32; 3] {
    unsafe { rd(sp + 0x24) }
}

/// Each element of a VC6 `std::list<auto_ptr<T>>` given its head node (the pointer
/// sits at node +0xc).
pub fn each_in(head: usize, mut f: impl FnMut(usize)) {
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 100_000 {
        let v = field(n, 0xc);
        if v != 0 {
            f(v);
        }
        n = field(n, 0);
        guard += 1;
    }
}

/// The far plane during the background, and the one after.
pub fn star_far() -> f32 {
    unsafe { rd(star_far_addr()) }
}
pub fn far_plane() -> f32 {
    unsafe { rd(far_plane_addr()) }
}

/// `ST3D_Camera::SetFarPlane`.
pub fn set_far(cam: usize, far: f32) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, f32)>(fn_set_far())(cam, far) }
}

/// A background to load in place of the map's (bench only).
pub fn load_background(name: &str) {
    let mut c = crate::rt::cstr(name);
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(*mut u8)>(fn_load_bg())(c.as_mut_ptr()) };
}

/// Whether the view's render state draws the sky (`Background_Render`'s own test: the
/// state's byte and the detail configuration).
pub fn sky_on(rs: usize) -> bool {
    let cfg3d = field(engine::storm3d(), S3D_DETAIL);
    field(rs, RS_SKY) != 0 && field(cfg3d, DETAIL_SKY) != 2
}

/// Whether the view's render state draws the stars.
pub fn stars_on(rs: usize) -> bool {
    field(rs, RS_STARS) != 0
}

/// Edit mode (the map editor), in which hidden background objects show.
pub fn edit_mode() -> bool {
    field_u8(unsafe { rd::<usize>(user_profile()) }, 0x84) != 0
}

/// The background instance's transform.
pub fn background_instance() -> M34 {
    field_m34(bg_instance(), 0x44)
}

/// Whether an object is a background object that shows (not hidden, unless the editor
/// is on).
pub fn background_object(o: usize, edit: bool) -> bool {
    o != 0 && engine::obj::flags(o) & OBJ_BACKGROUND != 0 && (edit || field_u8(engine::obj::class(o), CLASS_HIDDEN) == 0)
}

/// Whether the object's `Render` is the plain `GameObject::Render` (slot 11), the one
/// the module draws in its place.
pub fn plain_render(o: usize) -> bool {
    field(engine::vtable(o), SLOT_RENDER * 4) == fn_go_render()
}

/// The engine's own `Render` of a background object.
pub fn engine_render(o: usize, cam: usize) {
    unsafe { this1(field(engine::vtable(o), SLOT_RENDER * 4), o, cam) };
}

/// Whether an instance draws a plain `ST3D_Database` (the module reads its SOD itself).
pub fn plain_database(inst: usize) -> bool {
    let db = field(inst, 0x80);
    db != 0 && engine::vtable(db) == vt_database()
}

/// An instance's scale (`+0x28`) as the background objects use it: read as it is, with
/// no test of the flag the scale effects set.
pub fn instance_scale(inst: usize) -> V3 {
    if inst == 0 { V3 { x: 1.0, y: 1.0, z: 1.0 } } else { unsafe { rd::<V3>(inst + 0x28) } }
}

/// The end of `Background_Render`: the device's batch is flushed, the far plane put
/// back and the depth cleared over the camera's rectangle for the scene.
pub fn finish(st3d: usize, cam: usize) {
    unsafe {
        let dvt = engine::vtable(st3d);
        this0(field(dvt, engine::DEV_FLUSH), st3d);
        set_far(cam, far_plane());
        let r: [f32; 4] = rd(cam + CAM_RECT);
        this1(field(dvt, engine::DEV_CLEAR_DEPTH), st3d, r.as_ptr() as usize);
    }
}
