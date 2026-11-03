//! What the frame's steps, the light gather and the pose code read of the engine: a
//! camera's rectangle and eye, the object and feature lists' fields, the ST3D device's
//! slots, the lighting registry, the instance clock and the LOD settings.
use super::{at, field, field_f32, field_u8, field_v3, rd, storm3d, vtable, DEV_CLEAR_DEPTH, DEV_FLUSH};
use crate::math::{M34, V3};
use crate::syms::sym;

// ---- the entity and its device ---------------------------------------------------

/// The handle the engine gives an `Entity` (`+0x28`, given out in order as objects are made).
pub fn entity_handle(e: usize) -> usize {
    field(e, 0x28)
}

/// The d3d8 device an ST3D device wraps (`+0x90`).
pub fn device_d3d8(st3d: usize) -> usize {
    field(st3d, 0x90)
}

/// Slot `n` of a vtable.
pub fn slot(vt: usize, n: usize) -> usize {
    field(vt, n * 4)
}

/// The ST3D device's `ClearDepthBuffer(ST3D_Rect*)`.
pub fn device_clear_depth_fn(st3d: usize) -> usize {
    field(vtable(st3d), DEV_CLEAR_DEPTH)
}

/// The ST3D device's `Flush()`.
pub fn device_flush_fn(st3d: usize) -> usize {
    field(vtable(st3d), DEV_FLUSH)
}

/// The `IDirect3DTexture8` of an ST3D texture on the current device (`+0x40 + index * 4`,
/// then `+4`); 0 when the device index is out of range.
pub fn d3d8_texture(st3d_tex: usize) -> usize {
    let idx = field(storm3d(), 0xc0);
    if idx > 8 {
        return 0;
    }
    let devtex = field(st3d_tex, 0x40 + idx * 4);
    field(devtex, 4)
}

// ---- cameras and the frame's globals --------------------------------------------

/// The camera's rectangle in back-buffer pixels (`+0x1b0`, four floats).
pub fn cam_viewport(cam: usize) -> [f32; 4] {
    unsafe { rd(cam + 0x1b0) }
}

/// One float of the camera's rectangle (`o` 0, 4, 8, 12).
pub fn cam_viewport_f(cam: usize, o: usize) -> f32 {
    field_f32(cam, 0x1b0 + o)
}

/// The camera's eye (`+0xe4`).
pub fn cam_eye(cam: usize) -> V3 {
    field_v3(cam, 0xe4)
}

/// The HUD's action camera: the camera of `g_cinematicView` (`+0x2c`).
pub fn cinematic_camera() -> usize {
    field(at(sym::CINEMATIC_VIEW), 0x2c)
}

/// The 3D credits are drawing (`g_credits_instance` is set).
pub fn credits_active() -> bool {
    unsafe { rd::<usize>(at(sym::G_CREDITS)) != 0 }
}

/// The engine's ambient colour, copied into Storm3D (`+0x28`) as the stock body does.
pub fn copy_ambient_to_storm3d(s3d: usize) {
    unsafe {
        let amb: [f32; 3] = rd(at(sym::G_AMBIENT));
        core::ptr::write_unaligned((s3d + 0x28) as *mut [f32; 3], amb);
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

/// A flag of the render state `rs` the body was given (non-zero), at `off`.
pub fn render_state_flag(rs: usize, off: usize) -> bool {
    field(rs, off) != 0
}

// ---- objects and features -------------------------------------------------------

/// Object flags (`+0x14`): a background object, drawn by `Background_Render`.
const OBJ_BACKGROUND: u32 = 0x10000;
/// Byte: distance-culled for this view (PreRenderAll).
const OBJ_CULLED: usize = 0x183;
/// `GameObject` vtable slot `Render(ST3D_Camera*)`.
const OBJ_RENDER: usize = 0x2c;

/// Whether the object body draws for this view: it exists, is not a background object and
/// was not culled.
pub fn object_drawn_by_body(o: usize) -> bool {
    !(o == 0 || field(o, 0x14) as u32 & OBJ_BACKGROUND != 0 || field_u8(o, OBJ_CULLED) != 0)
}

/// The object's own `Render(camera)`.
pub fn object_render_fn(o: usize) -> usize {
    field(vtable(o), OBJ_RENDER)
}

/// A game feature's `Render(cam, isMain)` (vtable `+0x20`) when its flags (`+0x20`) have
/// bit 4; 0 otherwise.
pub fn feature_render_fn(f: usize) -> usize {
    if f != 0 && field_u8(f, 0x20) & 4 != 0 { field(vtable(f), 0x20) } else { 0 }
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
    let head = field(s, 0x60);
    if head == 0 {
        return false;
    }
    let mut node = field(head, 0);
    let mut guard = 0;
    while node != head && node != 0 && guard < 4096 {
        guard += 1;
        let rec = field(node, 8);
        node = field(node, 0);
        let light = field(rec, 0);
        if rec == 0 || light == 0 {
            continue;
        }
        let colour: V3 = unsafe { rd(rec + 4) };
        let m: M34 = unsafe { rd(rec + 0x10) };
        let v = vtable(light);
        let kind = if v != 0 && v == at(sym::VT_DIRECTIONAL) {
            RegisteredKind::Directional
        } else if v != 0 && v == at(sym::VT_POINT) {
            RegisteredKind::Point
        } else {
            RegisteredKind::Other
        };
        let (start, range) = unsafe { (rd::<f32>(light + 0x100), rd::<f32>(light + 0x104)) };
        f(RegisteredLight { kind, light, colour, m, start, range });
    }
    true
}

// ---- the pose ------------------------------------------------------------------

/// An `ST3D_Instance`'s animation clock: flags (`+0xc`), whether it animates (`+0x76`)
/// and its start time (`+0x18`).
pub fn instance_clock(inst: usize) -> (u32, bool, f32) {
    let flags = field(inst, 0xc) as u32;
    let animating = field_u8(inst, 0x76) != 0;
    let start = unsafe { rd::<f32>(inst + 0x18) };
    (flags, animating, start)
}

/// The engine's LOD settings (Storm3D `+0x104`): detail level and the four distances;
/// None without the config.
pub fn lod_config() -> Option<(u32, [f32; 4])> {
    let cfg = field(storm3d(), 0x104);
    if cfg == 0 {
        return None;
    }
    unsafe { Some((rd::<u32>(cfg + 0x40), [rd(cfg + 0x30), rd(cfg + 0x34), rd(cfg + 0x38), rd(cfg + 0x3c)])) }
}
