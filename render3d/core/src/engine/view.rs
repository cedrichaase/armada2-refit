//! What the frame's steps, the light gather and the pose code read of the engine: a
//! camera's rectangle and eye, the object and feature lists' fields, the ST3D device's
//! slots, the lighting registry, the instance clock and the LOD settings.
use super::{at, dev_clear_depth, dev_flush, field, field_f32, field_u8, field_v3, rd, storm3d, vtable};
use crate::math::{M34, V3};
use crate::syms::{field as off, sym};

// ---- the entity and its device ---------------------------------------------------

/// The handle the engine gives an `Entity` (`+0x28`, given out in order as objects are made).
pub fn entity_handle(e: usize) -> usize {
    field(e, off::ENTITY_HANDLE())
}

/// The d3d8 device an ST3D device wraps (`+0x90`).
pub fn device_d3d8(st3d: usize) -> usize {
    field(st3d, off::DEV_D3D8())
}

/// Slot `n` of a vtable.
pub fn slot(vt: usize, n: usize) -> usize {
    field(vt, n * 4)
}

/// The ST3D device's `ClearDepthBuffer(ST3D_Rect*)`.
pub fn device_clear_depth_fn(st3d: usize) -> usize {
    field(vtable(st3d), dev_clear_depth())
}

/// The ST3D device's `Flush()`.
pub fn device_flush_fn(st3d: usize) -> usize {
    field(vtable(st3d), dev_flush())
}

/// The `IDirect3DTexture8` of an ST3D texture on the current device (`+0x40 + index * 4`,
/// then `+4`); 0 when the device index is out of range.
pub fn d3d8_texture(st3d_tex: usize) -> usize {
    let idx = field(storm3d(), off::S3D_DEVICE_INDEX());
    if idx > 8 {
        return 0;
    }
    let devtex = field(st3d_tex, off::TEX_DEVICE_TEXTURES() + idx * 4);
    field(devtex, off::DEVTEX_D3D8())
}

// ---- cameras and the frame's globals --------------------------------------------

/// The camera's rectangle in back-buffer pixels (`+0x1b0`, four floats).
pub fn cam_viewport(cam: usize) -> [f32; 4] {
    unsafe { rd(cam + off::CAM_VIEWPORT()) }
}

/// One float of the camera's rectangle (`o` 0, 4, 8, 12).
pub fn cam_viewport_f(cam: usize, o: usize) -> f32 {
    field_f32(cam, off::CAM_VIEWPORT() + o)
}

/// The camera's eye (`+0xe4`).
pub fn cam_eye(cam: usize) -> V3 {
    field_v3(cam, off::CAM_EYE())
}

/// The HUD's action camera: the camera of `g_cinematicView` (`+0x2c`).
pub fn cinematic_camera() -> usize {
    field(at(sym::CINEMATIC_VIEW), off::CINEMATIC_CAMERA())
}

/// The 3D credits are drawing (`g_credits_instance` is set).
pub fn credits_active() -> bool {
    unsafe { rd::<usize>(at(sym::G_CREDITS)) != 0 }
}

/// The engine's ambient colour, copied into Storm3D (`+0x28`) as the stock body does.
pub fn copy_ambient_to_storm3d(s3d: usize) {
    unsafe {
        let amb: [f32; 3] = rd(at(sym::G_AMBIENT));
        core::ptr::write_unaligned((s3d + off::S3D_AMBIENT()) as *mut [f32; 3], amb);
    }
}

/// `cfgFADE_OUT_ENABLED`, the value the near fade is given.
pub fn fade_out_enabled() -> usize {
    unsafe { rd::<i32>(at(sym::CFG_FADE_OUT)) as usize }
}

/// The view mode word (`DisplayInterface::PreRenderAll`'s argument).
pub fn view_mode_word() -> usize {
    unsafe { rd::<usize>(at(sym::VIEW_MODE)) }
}

/// The view mode as a number (2 hides the selection circles).
pub fn view_mode_value() -> u32 {
    unsafe { rd::<u32>(at(sym::VIEW_MODE)) }
}

/// The render state the body was given (`Armada_RenderAllOurStuff`'s second argument):
/// whether terrain and ordnance draw, and the background's two arguments.
pub struct RenderState {
    pub terrain: bool,
    pub background: (bool, bool),
    pub ordnance: bool,
}

pub fn render_state(rs: usize) -> RenderState {
    let flag = |at: usize| field(rs, at) != 0;
    RenderState {
        terrain: flag(off::RS_TERRAIN()),
        background: (flag(off::RS_BACKGROUND_A()), flag(off::RS_BACKGROUND_B())),
        ordnance: flag(off::RS_ORDNANCE()),
    }
}

/// Storm3D's global particle list (`+0x68`, the chain pulsar's and Manheim trails),
/// what `RenderParticleList` is handed.
pub fn particle_list(s3d: usize) -> usize {
    s3d + off::S3D_PARTICLE_LIST()
}

// ---- objects and features -------------------------------------------------------

/// Object flags (`+0x14`): a background object, drawn by `Background_Render`.
const OBJ_BACKGROUND: u32 = 0x10000;

/// Whether the object body draws for this view: it exists, is not a background object and
/// was not culled.
/// A background object (flag `0x10000`), which the body's loop never draws.
pub fn is_background_object(o: usize) -> bool {
    field(o, off::OBJ_FLAGS()) as u32 & OBJ_BACKGROUND != 0
}

pub fn object_drawn_by_body(o: usize) -> bool {
    !(o == 0 || field(o, off::OBJ_FLAGS()) as u32 & OBJ_BACKGROUND != 0 || field_u8(o, off::OBJ_CULLED()) != 0)
}

/// The object's own `Render(camera)`.
pub fn object_render_fn(o: usize) -> usize {
    field(vtable(o), off::VT_OBJ_RENDER())
}

/// A game feature's `Render(cam, isMain)` (vtable `+0x20`) when its flags (`+0x20`) have
/// bit 4; 0 otherwise.
pub fn feature_render_fn(f: usize) -> usize {
    if f != 0 && field_u8(f, off::FEATURE_FLAGS()) & 4 != 0 { field(vtable(f), off::VT_FEATURE_RENDER()) } else { 0 }
}

// ---- the lighting registry ------------------------------------------------------

/// One light of the engine's registry (Storm3D `+0x60`).
pub enum RegisteredKind {
    Directional,
    Point,
    /// A vtable that is neither (a plugin's own light), read as a point light.
    Other,
}

pub struct RegisteredLight {
    pub kind: RegisteredKind,
    pub light: usize,
    pub colour: V3,
    pub m: M34,
    /// The light's `start` and `range` (`+0x100`, `+0x104`).
    pub start: f32,
    pub range: f32,
}

/// Calls `f` with each registered light, in the registry's order; false when the registry
/// does not exist.
pub fn for_each_registered_light(mut f: impl FnMut(RegisteredLight)) -> bool {
    let s = storm3d();
    let head = field(s, off::S3D_LIGHTS());
    if head == 0 {
        return false;
    }
    let (next, value) = (off::LIST_NEXT(), off::LIST_VALUE());
    let (rec_light, rec_colour, rec_matrix) = (off::LIGHT_REC_LIGHT(), off::LIGHT_REC_COLOUR(), off::LIGHT_REC_MATRIX());
    let (start_at, range_at) = (off::LIGHT_START(), off::LIGHT_RANGE());
    let mut node = field(head, next);
    let mut guard = 0;
    while node != head && node != 0 && guard < 4096 {
        guard += 1;
        let rec = field(node, value);
        node = field(node, next);
        let light = field(rec, rec_light);
        if rec == 0 || light == 0 {
            continue;
        }
        let colour: V3 = unsafe { rd(rec + rec_colour) };
        let m: M34 = unsafe { rd(rec + rec_matrix) };
        let v = vtable(light);
        let kind = if v != 0 && v == at(sym::VT_DIRECTIONAL) {
            RegisteredKind::Directional
        } else if v != 0 && v == at(sym::VT_POINT) {
            RegisteredKind::Point
        } else {
            RegisteredKind::Other
        };
        let (start, range) = unsafe { (rd::<f32>(light + start_at), rd::<f32>(light + range_at)) };
        f(RegisteredLight { kind, light, colour, m, start, range });
    }
    true
}

/// Calls `f(record, light)` for each record of the engine's light registry (Storm3D
/// `+0x60`) this view.
pub fn for_each_light_record(mut f: impl FnMut(usize, usize)) {
    let head = field(storm3d(), off::S3D_LIGHTS());
    if head == 0 {
        return;
    }
    let (next, value, rec_light) = (off::LIST_NEXT(), off::LIST_VALUE(), off::LIGHT_REC_LIGHT());
    let mut node = field(head, next);
    let mut guard = 0;
    while node != head && node != 0 && guard < 4096 {
        guard += 1;
        let rec = field(node, value);
        node = field(node, next);
        f(rec, field(rec, rec_light));
    }
}

/// Sets a registry record's colour (`+4`): this view's, not the light's own.
pub fn set_light_record_colour(rec: usize, c: V3) {
    unsafe { core::ptr::write_unaligned((rec + off::LIGHT_REC_COLOUR()) as *mut V3, c) };
}

// ---- the pose ------------------------------------------------------------------

/// An `ST3D_Instance`'s animation clock: flags (`+0xc`), whether it animates (`+0x76`)
/// and its start time (`+0x18`).
pub fn instance_clock(inst: usize) -> (u32, bool, f32) {
    let flags = field(inst, off::INST_FLAGS()) as u32;
    let animating = field_u8(inst, off::INST_ANIMATING()) != 0;
    let start = unsafe { rd::<f32>(inst + off::INST_ANIM_START()) };
    (flags, animating, start)
}

/// The engine's LOD settings (Storm3D `+0x104`): detail level and the four distances;
/// None without the config.
pub fn lod_config() -> Option<(u32, [f32; 4])> {
    let cfg = field(storm3d(), off::S3D_CONFIG());
    if cfg == 0 {
        return None;
    }
    let d = cfg + off::CFG_LOD_DISTANCES();
    unsafe { Some((rd::<u32>(cfg + off::CFG_DETAIL()), [rd(d), rd(d + 4), rd(d + 8), rd(d + 12)])) }
}
