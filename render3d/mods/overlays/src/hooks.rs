//! The overlays module's engine glue (README.md, "overlays", "Hooks"): every address,
//! call and field offset the terrain's overlays (grid, fog and shroud), the selection
//! bubbles, the ghosts, the build footprints, the height posts and the pivot marker read
//! of the engine, and the one hook the module installs through `hooks`: the height
//! post's five call sites. Its detour starts with the module's gate and hands the call to
//! what was there before when the gate is shut. The step, feature and object bodies
//! (`Terrain_Geometry::Render`, `RenderSelectionEffects`, the ghost and placeholder
//! features, `TacticalCamera::Render`) are reached through the frame's dispatch, not
//! patched: while the module is off they run as stock. Findings are in the README beside
//! the hook that uses them.
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Site};
use r3d_core::engine::{self, field, field_f32, field_u8, rd, storm3d};
use r3d_core::math::{M34, V3};
use crate::rt;

// ---- the terrain step ---------------------------------------------------------------

/// `g_terrain_geometry`: the `Terrain_Geometry` `Terrain_Geometry::Render` draws.
pub const G_TERRAIN: usize = 0x772140;
/// `Terrain_Geometry::m_initialized`: false until its first `Initialize`.
const TERRAIN_INITIALIZED: usize = 0x772180;
/// `View_Record` (`PresetView`): 0 the RTS camera, 1 the tactical one, 2 cineractive.
const VIEW_RECORD: usize = 0x76b610;
/// `UserProfile` (`GameConfiguration*`): `+0x84` the map editor is on, `+0x45` the
/// tactical shroud option, `+0x48` the selection bubbles' brightness.
const USER_PROFILE: usize = 0x76b5ac;
const G_TRANSPORT: usize = 0x76b8d4;
const FN_GET_GAME_SETUP: usize = 0x557940; // Transport::getGameSetup, thiscall
const FN_IS_FOG: usize = 0x5462f0; // GameSetup::isFog, thiscall
const FN_IS_SHROUD: usize = 0x546300; // GameSetup::isShroud, thiscall
const FN_GRID_VISIBLE: usize = 0x51e180; // GridVisible(), cdecl
const FN_DRAW_GRID: usize = 0x593480; // Terrain_Geometry::DrawGrid(cam), thiscall
const PLACEHOLDER: usize = 0x736d50; // placeholderFeature
const FN_FOOTPRINTS: usize = 0x473930; // PlaceholderFeature::RenderFootprints(cam, float), thiscall
pub const VT_GHOST: usize = 0x6aff8c;
pub const VT_PLACEHOLDER: usize = 0x6b005c;
/// The scanner's fog and shroud grids (`Scanner::m_fog`, `m_shroud`).
const SCAN_FOG: usize = 0x738a54;
const SCAN_SHROUD: usize = 0x738a58;
/// The tactical grid's cell size, and its colour (`ST3D_Colour_DarkOrange`).
const GRID_SIZE: usize = 0x71a78c;
const DARK_ORANGE: usize = 0x7a8ee8;

unsafe fn this0_ret(f: usize, this: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> usize>(f)(this) }
}

fn game_setup_flag(f: usize) -> bool {
    unsafe {
        let t = rd::<usize>(G_TRANSPORT);
        if t == 0 {
            return false;
        }
        let gs = this0_ret(FN_GET_GAME_SETUP, t);
        gs != 0 && (core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(f)(gs)) != 0
    }
}

/// The game's setup has the fog of war on.
pub fn is_fog() -> bool {
    game_setup_flag(FN_IS_FOG)
}

/// The game's setup has the shroud on.
pub fn is_shroud() -> bool {
    game_setup_flag(FN_IS_SHROUD)
}

/// The map editor is on (`UserProfile+0x84`).
pub fn editor() -> bool {
    field_u8(unsafe { rd::<usize>(USER_PROFILE) }, 0x84) != 0
}

pub fn view_record() -> u32 {
    unsafe { rd::<u32>(VIEW_RECORD) }
}

/// Whether the engine has made the terrain's meshes and found its textures yet.
pub fn terrain_initialized() -> bool {
    (unsafe { rd::<u8>(TERRAIN_INITIALIZED) }) != 0
}

/// The profile's tactical shroud option (`+0x45`).
pub fn tactical_shroud_option() -> u8 {
    field_u8(unsafe { rd::<usize>(USER_PROFILE) }, 0x45)
}

/// Whether the terrain holds the shroud as a `TreeGrid` (`Terrain_Geometry+0x28`).
pub fn shroud_tree_grid(tg: usize) -> bool {
    field(tg, 0x28) != 0
}

/// The selection bubbles' brightness (`UserProfile+0x48`).
pub fn selection_brightness() -> f32 {
    field_f32(unsafe { rd::<usize>(USER_PROFILE) }, 0x48)
}

/// The user's team (`Transport::getUserTeam`), -1 without a transport.
pub fn user_team() -> i32 {
    unsafe {
        let t = rd::<usize>(G_TRANSPORT);
        if t == 0 { -1 } else { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> i32>(0x5578b0)(t) }
    }
}

/// `PlaceholderFeature::RenderFootprints`, the build footprints' sprites, at the grid's
/// height.
pub fn render_footprints(cam: usize, tg: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize, f32) = core::mem::transmute(FN_FOOTPRINTS);
        f(PLACEHOLDER, cam, field_f32(tg, 0x1c));
    }
}

/// `GridVisible()`.
pub fn grid_visible() -> bool {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(FN_GRID_VISIBLE)() != 0 }
}

/// The engine's own grid (`Terrain_Geometry::DrawGrid`).
pub fn draw_engine_grid(tg: usize, cam: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(FN_DRAW_GRID);
        f(tg, cam);
    }
}

/// The map's rectangle on the terrain: (x0, z0, x1, z1, grid height) (`+0x4`, `+0xc`,
/// `+0x10`, `+0x18`, `+0x1c`).
pub fn terrain_rect(tg: usize) -> (f32, f32, f32, f32, f32) {
    (field_f32(tg, 0x4), field_f32(tg, 0xc), field_f32(tg, 0x10), field_f32(tg, 0x18), field_f32(tg, 0x1c))
}

/// The grid's cell counts (`+0x20`, `+0x24`).
pub fn terrain_cells(tg: usize) -> (i32, i32) {
    (field(tg, 0x20) as i32, field(tg, 0x24) as i32)
}

pub fn grid_step() -> f32 {
    unsafe { rd::<f32>(GRID_SIZE) }
}

pub fn grid_colour() -> [f32; 3] {
    unsafe { rd(DARK_ORANGE) }
}

/// The engine's texture behind a layer: `Terrain_Geometry+0x34` the fog's, `+0x38` the
/// shroud's.
pub fn layer_texture(tg: usize, shroud: bool) -> usize {
    field(tg, if shroud { 0x38 } else { 0x34 })
}

/// The scanner grid under a layer: width `+0xc`, height `+0x10`.
pub fn scan_dims(shroud: bool) -> (i32, i32) {
    let grid = unsafe { rd::<usize>(if shroud { SCAN_SHROUD } else { SCAN_FOG }) };
    (field(grid, 0xc) as i32, field(grid, 0x10) as i32)
}

// ---- the height posts (the one hook) --------------------------------------------------

const FN_HEIGHT_POST: usize = 0x593840;
const SITES: [usize; 5] = [0x473bbc, 0x522080, 0x5222fd, 0x524c02, 0x524c2d];
const TERRAIN_Y: usize = 0x77215c;
const GREEN: usize = 0x7a8f68;
const RED: usize = 0x7a8f78;

static mut GATE: Option<Gate> = None;
static mut PREV_POST: usize = 0;

/// The plane's height posts are drawn to.
pub fn terrain_y() -> f32 {
    unsafe { rd::<f32>(TERRAIN_Y) }
}

/// The posts' colours: green above the plane, red below.
pub fn post_colours() -> ([f32; 3], [f32; 3]) {
    unsafe { (rd(GREEN), rd(RED)) }
}

/// The engine's `DrawHeightPost`, for a post that is not ours.
pub fn draw_engine_post(pos: *const V3, cam: usize) {
    unsafe {
        let f: unsafe extern "C" fn(*const V3, usize) = core::mem::transmute(if PREV_POST != 0 { PREV_POST } else { FN_HEIGHT_POST });
        f(pos, cam);
    }
}

/// Re-points the five calls of `DrawHeightPost` (once, at the first enable); each that
/// is not stock's is logged and left to the engine. Returns how many are ours.
pub fn install_posts(h: &mut Hooks) -> usize {
    unsafe { GATE = Some(h.gate()) };
    let owner = h.owner();
    let mut n = 0;
    for s in SITES {
        let hook = Hook {
            owner,
            used_by: "height posts",
            kind: Kind::Call,
            site: Site { symbol: "DrawHeightPost", addr: s, expect: FN_HEIGHT_POST },
            to: hook_post as *const () as usize,
            chain: false,
        };
        if let Some(p) = h.install(hook) {
            unsafe { PREV_POST = p.addr() };
            n += 1;
        }
    }
    n
}

/// `DrawHeightPost`, re-pointed: noted for the view while one of ours runs, else the
/// call that was there (always, while the gate is shut).
unsafe extern "C" fn hook_post(pos: *const V3, cam: usize) {
    if super::posts::in_view() && unsafe { GATE.is_some_and(|g| g.open()) } && !pos.is_null() && super::posts::note_raw(unsafe { core::ptr::read_unaligned(pos) }) {
        return;
    }
    draw_engine_post(pos, cam);
}

// ---- the selection bubbles ------------------------------------------------------------

const BUCKETS: usize = 0x773c8c;
const FN_RENDER_SELECTION: usize = 0x599900; // RenderSelectionEffects, cdecl
const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(handle), cdecl
const FN_RELATION: usize = 0x4d1080; // GameObject::GetTeamRelation(team), thiscall
const FN_IS_SCALED: usize = 0x4cff60; // Entity::IsScaled, thiscall
const FN_SCALE_FACTOR: usize = 0x4cff40; // Entity::GetScaleFactor, thiscall: Vector3*

pub fn entity(handle: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(FN_ENTITY_GET)(handle) }
}

/// An object's team colour (`+0xf0`, the team, `+8`).
pub fn team_colour(o: usize) -> V3 {
    let t = field(o, 0xf0);
    if t == 0 { V3::ZERO } else { unsafe { rd::<V3>(t + 8) } }
}

/// Another UI has put a `ret` in `RenderSelectionEffects` (it draws the selection).
pub fn selection_effects_returns() -> bool {
    (unsafe { rd::<u8>(FN_RENDER_SELECTION) }) == 0xc3
}

/// Every selection entry of the engine's sixteen buckets.
pub fn each_selection_entry(mut f: impl FnMut(usize)) {
    for b in 0..16 {
        let mut e = unsafe { rd::<usize>(BUCKETS + b * 4) };
        let mut guard = 0;
        while e != 0 && guard < 10000 {
            guard += 1;
            let next = field(e, 0x9c);
            f(e);
            e = next;
        }
    }
}

/// An entry's object handle (`+0x88`), flag bits (`+0x84`), pulse phase (`+0x8c`) and
/// scale (`+0x90`).
pub fn entry_handle(e: usize) -> usize {
    field(e, 0x88)
}
pub fn entry_bits(e: usize) -> u32 {
    field(e, 0x84) as u32
}
pub fn entry_phase(e: usize) -> f32 {
    field_f32(e, 0x8c)
}
pub fn entry_scale(e: usize) -> V3 {
    unsafe { rd(e + 0x90) }
}

/// The craft's health measure the bubble pulses with (`+0x158`).
pub fn object_pulse(o: usize) -> f32 {
    field_f32(o, 0x158)
}

/// `GameObject::GetTeamRelation(team)`.
pub fn team_relation(o: usize, team: i32) -> i32 {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, i32) -> i32>(FN_RELATION)(o, team) }
}

/// The object's scale factor when the entity is scaled.
pub fn entity_scale_factor(o: usize) -> Option<V3> {
    unsafe {
        if core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(FN_IS_SCALED)(o) != 0 {
            let f = core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> *const V3>(FN_SCALE_FACTOR)(o);
            if !f.is_null() {
                return Some(core::ptr::read_unaligned(f));
            }
        }
    }
    None
}

// ---- the ghosts and the build ghost ---------------------------------------------------

const GHOST_NODES: usize = 0x736c94;
const FN_TEAM_GET: usize = 0x496340; // Team::GetTeam(int), cdecl: Team&
const FN_CLASS_ODF: usize = 0x4ce370; // GameObjectClass::GetOdfName, thiscall
const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
const VT_DATABASE: usize = 0x6bc56c;
const SPIRIT_RENDER: usize = 0xc; // Spirit vtable: Render(ST3D_Camera*)
const S3D_MATERIAL: usize = 0xa8; // Storm3D: the lighting material in place of every mesh's
const S3D_ALPHA: usize = 0x4;
const S3D_TEAM: usize = 0x34;

/// `Ghost+0x24`: the material every ghost mesh is lit with, and its ambient and
/// diffuse (`+0x18`, `+0x24`); a ghost without one is lit dark grey.
pub fn ghost_material(g: usize) -> (usize, V3, V3) {
    let mat = field(g, 0x24);
    let (amb, diff) = if mat == 0 {
        (r3d_core::math::v3(0.1, 0.1, 0.1), r3d_core::math::v3(0.1, 0.1, 0.1))
    } else {
        unsafe { (rd::<V3>(mat + 0x18), rd::<V3>(mat + 0x24)) }
    };
    (mat, amb, diff)
}

/// `Ghost+0x28`: the tint's factor.
pub fn ghost_factor(g: usize) -> f32 {
    field_f32(g, 0x28)
}

/// Every spirit in `Ghost::ghostList`, with the team colour it is tinted with (times
/// `factor`); false if the list is not there.
pub fn each_spirit(mut f: impl FnMut(usize, V3)) -> bool {
    let head = unsafe { rd::<usize>(GHOST_NODES) };
    if head == 0 {
        return false;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 100_000 {
        guard += 1;
        let sp = field(n, 8);
        n = field(n, 0);
        if sp == 0 {
            continue;
        }
        let team = unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(i32) -> usize>(FN_TEAM_GET)(field(sp, 0xc) as i32) };
        f(sp, if team == 0 { V3::ZERO } else { unsafe { rd::<V3>(team + 8) } });
    }
    true
}

/// A spirit the module does not draw: its own `Render`, under stock's state (the ghost
/// material, the additive scale and the team tint on `Storm3D`).
pub fn render_spirit(sp: usize, cam: usize, mat: usize, additive: f32, tint: V3) {
    let s3d = storm3d();
    unsafe {
        core::ptr::write_unaligned((s3d + S3D_MATERIAL) as *mut usize, mat);
        core::ptr::write_unaligned((s3d + S3D_ALPHA) as *mut f32, additive);
        core::ptr::write_unaligned((s3d + S3D_TEAM) as *mut V3, tint);
        let r: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(sp), SPIRIT_RENDER));
        r(sp, cam);
    }
}

/// Puts `Storm3D`'s ghost state back after `render_spirit`.
pub fn end_spirits() {
    let s3d = storm3d();
    unsafe {
        core::ptr::write_unaligned((s3d + S3D_ALPHA) as *mut f32, 1.0);
        core::ptr::write_unaligned((s3d + S3D_MATERIAL) as *mut usize, 0);
    }
}

/// A spirit that carries a plain SOD: (instance `+0x10`, class `+4`); None if its
/// instance has no database of `VT_DATABASE`'s kind.
pub fn spirit_instance(sp: usize) -> Option<(usize, usize)> {
    let inst = field(sp, 0x10);
    let db = field(inst, 0x80);
    if inst == 0 || db == 0 || engine::vtable(db) != VT_DATABASE {
        return None;
    }
    Some((inst, field(sp, 4)))
}

/// A class's ODF name without its extension.
pub fn class_odf(class: usize) -> &'static str {
    if class == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_CLASS_ODF);
        let s = rt::cstr_at(f(class), 260);
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

/// The instance's origin (`+0x68`), scale (`+0x28`) and placement (`+0x44`).
pub fn instance_origin(inst: usize) -> V3 {
    unsafe { rd(inst + 0x68) }
}
pub fn instance_scale(inst: usize) -> V3 {
    unsafe { rd::<V3>(inst + 0x28) }
}
pub fn instance_world(inst: usize) -> M34 {
    unsafe { rd(inst + 0x44) }
}

/// `GameObjectInstance::DistanceCull(eye, pos)`, as `GameObjectSpirit::Render` asks it.
pub fn distance_culled(inst: usize, eye: &V3, pos: &V3) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
        f(inst, eye, pos) != 0
    }
}

/// The station being placed (`PlaceholderFeature`): its class (`+0x34`), placement
/// (`+0x38`), colour (`+0x68`) and whether the placement is blocked (`+0x74` clear).
pub fn placeholder(f: usize) -> (usize, M34, V3, bool) {
    (field(f, 0x34), unsafe { rd(f + 0x38) }, unsafe { rd(f + 0x68) }, field_u8(f, 0x74) == 0)
}

// ---- the pivot marker -----------------------------------------------------------------

/// `TacticalCamera`'s vtable and its `Render(ST3D_Camera*, bool)` (slot 8, `+0x20`).
pub const VT_TACTICAL_CAMERA: usize = 0x6b46c0;
const FN_RENDER: usize = 0x4e5260;
/// `TacticalCamera::Render` as stock has it, to its `ret 8`.
const RENDER_BYTES: [u8; 54] = [
    0x55, 0x8b, 0xec, 0x8a, 0x45, 0x0c, 0x84, 0xc0, 0x74, 0x28, 0x8b, 0x81, 0xe0, 0x00, 0x00, 0x00, 0x85, 0xc0,
    0x74, 0x1e, 0x8a, 0x91, 0xe4, 0x00, 0x00, 0x00, 0x84, 0xd2, 0x74, 0x14, 0x83, 0x3d, 0x10, 0xb6, 0x76, 0x00,
    0x01, 0x75, 0x0b, 0x8b, 0x4d, 0x08, 0x8b, 0x10, 0x51, 0x8b, 0xc8, 0xff, 0x52, 0x18, 0x5d, 0xc2, 0x08, 0x00,
];
/// `ST3D_Instance`'s vtable: the marker must be a plain instance.
const VT_INSTANCE: usize = 0x6aff14;
const CAM_MARKER: usize = 0xe0;
const CAM_FREE: usize = 0xe4;

/// Whether `TacticalCamera::Render` is stock's (its slot and its bytes).
pub fn tactical_render_is_stock() -> bool {
    field(VT_TACTICAL_CAMERA, 0x20) == FN_RENDER && unsafe { core::slice::from_raw_parts(FN_RENDER as *const u8, RENDER_BYTES.len()) } == RENDER_BYTES
}

/// The camera's marker instance (`+0xe0`).
pub fn camera_marker(f: usize) -> usize {
    field(f, CAM_MARKER)
}

/// Whether the camera has no entity to follow (`+0xe4`).
pub fn camera_free(f: usize) -> bool {
    field_u8(f, CAM_FREE) != 0
}

/// Whether an instance is a plain `ST3D_Instance`.
pub fn is_plain_instance(inst: usize) -> bool {
    engine::vtable(inst) == VT_INSTANCE
}
