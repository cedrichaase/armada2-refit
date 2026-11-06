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
use crate::syms::{field as off, hook, sym, value, TABLE};
use r3d_core::engine::instance::{instance_database, raw_scale, team_rgb};

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { TABLE.engine().addr(s) }

// ---- the terrain step ---------------------------------------------------------------

/// `g_terrain_geometry`: the `Terrain_Geometry` `Terrain_Geometry::Render` draws.
pub fn g_terrain() -> usize { at(sym::G_TERRAIN) }
/// `Terrain_Geometry::m_initialized`: false until its first `Initialize`.
pub(crate) fn terrain_initialized_addr() -> usize { at(sym::TERRAIN_INITIALIZED) }
/// `View_Record` (`PresetView`): 0 the RTS camera, 1 the tactical one, 2 cineractive.
pub(crate) fn view_record_addr() -> usize { at(sym::VIEW_RECORD) }
/// `UserProfile` (`GameConfiguration*`): `+0x84` the map editor is on, `+0x45` the
/// tactical shroud option, `+0x48` the selection bubbles' brightness.
pub(crate) fn user_profile() -> usize { at(sym::USER_PROFILE) }
pub(crate) fn g_transport() -> usize { at(sym::G_TRANSPORT) }
pub(crate) fn fn_get_game_setup() -> usize { at(sym::FN_GET_GAME_SETUP) } // Transport::getGameSetup, thiscall
pub(crate) fn fn_is_fog() -> usize { at(sym::FN_IS_FOG) } // GameSetup::isFog, thiscall
pub(crate) fn fn_is_shroud() -> usize { at(sym::FN_IS_SHROUD) } // GameSetup::isShroud, thiscall
pub(crate) fn fn_grid_visible() -> usize { at(sym::FN_GRID_VISIBLE) } // GridVisible(), cdecl
pub(crate) fn fn_draw_grid() -> usize { at(sym::FN_DRAW_GRID) } // Terrain_Geometry::DrawGrid(cam), thiscall
pub(crate) fn placeholder_addr() -> usize { at(sym::PLACEHOLDER) } // placeholderFeature
pub(crate) fn fn_footprints() -> usize { at(sym::FN_FOOTPRINTS) } // PlaceholderFeature::RenderFootprints(cam, float), thiscall
pub fn vt_ghost() -> usize { at(sym::VT_GHOST) }
pub fn vt_placeholder() -> usize { at(sym::VT_PLACEHOLDER) }
/// The scanner's fog and shroud grids (`Scanner::m_fog`, `m_shroud`).
pub(crate) fn scan_fog() -> usize { at(sym::SCAN_FOG) }
pub(crate) fn scan_shroud() -> usize { at(sym::SCAN_SHROUD) }
/// The tactical grid's cell size, and its colour (`ST3D_Colour_DarkOrange`).
pub(crate) fn grid_size() -> usize { at(sym::GRID_SIZE) }
pub(crate) fn dark_orange() -> usize { at(sym::DARK_ORANGE) }

unsafe fn this0_ret(f: usize, this: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> usize>(f)(this) }
}

fn game_setup_flag(f: usize) -> bool {
    unsafe {
        let t = rd::<usize>(g_transport());
        if t == 0 {
            return false;
        }
        let gs = this0_ret(fn_get_game_setup(), t);
        gs != 0 && (core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(f)(gs)) != 0
    }
}

/// The game's setup has the fog of war on.
pub fn is_fog() -> bool {
    game_setup_flag(fn_is_fog())
}

/// The game's setup has the shroud on.
pub fn is_shroud() -> bool {
    game_setup_flag(fn_is_shroud())
}

/// The map editor is on (`UserProfile+0x84`).
pub fn editor() -> bool {
    field_u8(unsafe { rd::<usize>(user_profile()) }, off::OV_EDIT_MODE()) != 0
}

pub fn view_record() -> u32 {
    unsafe { rd::<u32>(view_record_addr()) }
}

/// Whether the engine has made the terrain's meshes and found its textures yet.
pub fn terrain_initialized() -> bool {
    (unsafe { rd::<u8>(terrain_initialized_addr()) }) != 0
}

/// The profile's tactical shroud option (`+0x45`).
pub fn tactical_shroud_option() -> u8 {
    field_u8(unsafe { rd::<usize>(user_profile()) }, off::OV_TACTICAL_SHROUD())
}

/// Whether the terrain holds the shroud as a `TreeGrid` (`Terrain_Geometry+0x28`).
pub fn shroud_tree_grid(tg: usize) -> bool {
    field(tg, off::OV_TG_SHROUD_TREE()) != 0
}

/// The selection bubbles' brightness (`UserProfile+0x48`).
pub fn selection_brightness() -> f32 {
    field_f32(unsafe { rd::<usize>(user_profile()) }, off::OV_SELECTION_BRIGHTNESS())
}

/// The user's team (`Transport::getUserTeam`), -1 without a transport.
pub fn user_team() -> i32 {
    unsafe {
        let t = rd::<usize>(g_transport());
        if t == 0 { -1 } else { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> i32>(at(sym::FN_USER_TEAM))(t) }
    }
}

/// `PlaceholderFeature::RenderFootprints`, the build footprints' sprites, at the grid's
/// height.
pub fn render_footprints(cam: usize, tg: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize, f32) = core::mem::transmute(fn_footprints());
        f(placeholder_addr(), cam, field_f32(tg, off::OV_TG_GRID_Y()));
    }
}

/// `GridVisible()`.
pub fn grid_visible() -> bool {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(fn_grid_visible())() != 0 }
}

/// The engine's own grid (`Terrain_Geometry::DrawGrid`).
pub fn draw_engine_grid(tg: usize, cam: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(fn_draw_grid());
        f(tg, cam);
    }
}

/// The map's rectangle on the terrain: (x0, z0, x1, z1, grid height) (`+0x4`, `+0xc`,
/// `+0x10`, `+0x18`, `+0x1c`).
pub fn terrain_rect(tg: usize) -> (f32, f32, f32, f32, f32) {
    (
        field_f32(tg, off::OV_TG_X0()),
        field_f32(tg, off::OV_TG_Z0()),
        field_f32(tg, off::OV_TG_X1()),
        field_f32(tg, off::OV_TG_Z1()),
        field_f32(tg, off::OV_TG_GRID_Y()),
    )
}

/// The grid's cell counts (`+0x20`, `+0x24`).
pub fn terrain_cells(tg: usize) -> (i32, i32) {
    (field(tg, off::OV_TG_CELLS_X()) as i32, field(tg, off::OV_TG_CELLS_Z()) as i32)
}

pub fn grid_step() -> f32 {
    unsafe { rd::<f32>(grid_size()) }
}

/// The grid's colour: what `Terrain_Geometry::DrawGrid`'s first line is drawn in, read
/// from the pointer its push holds now (`site_grid_colour`, the byte after the opcode):
/// `ST3D_Colour_Dark_Orange` on stock; on Roots its own colour, and FleetOpsHook's over
/// that. Stock's dark orange when the pointer is not one the process can read.
pub fn grid_colour() -> [f32; 3] {
    let site = at(sym::SITE_GRID_COLOUR);
    if site != 0 {
        let p = unsafe { core::ptr::read_unaligned((site + 1) as *const usize) };
        if p != 0 && readable(p, 12) {
            return unsafe { rd(p) };
        }
    }
    unsafe { rd(dark_orange()) }
}

/// Whether `n` bytes at `p` are committed, readable memory (a pointer another module put
/// in the engine's code).
fn readable(p: usize, n: usize) -> bool {
    use crate::rt::win::{MemInfo, VirtualQuery};
    let mut mi: MemInfo = unsafe { core::mem::zeroed() };
    if unsafe { VirtualQuery(p as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 {
        return false;
    }
    const MEM_COMMIT: u32 = 0x1000;
    const READ: u32 = 0x02 | 0x04 | 0x08 | 0x20 | 0x40 | 0x80; // not an offset: page protection bits
    mi.state == MEM_COMMIT && mi.protect & READ != 0 && mi.protect & 0x100 == 0 && p + n <= mi.base + mi.size // not an offset: PAGE_GUARD
}

/// The engine's texture behind a layer: `Terrain_Geometry+0x34` the fog's, `+0x38` the
/// shroud's.
pub fn layer_texture(tg: usize, shroud: bool) -> usize {
    field(tg, if shroud { off::OV_TG_SHROUD_TEX() } else { off::OV_TG_FOG_TEX() })
}

/// The scanner grid under a layer: width `+0xc`, height `+0x10`.
pub fn scan_dims(shroud: bool) -> (i32, i32) {
    let grid = unsafe { rd::<usize>(if shroud { scan_shroud() } else { scan_fog() }) };
    (field(grid, off::OV_SCAN_W()) as i32, field(grid, off::OV_SCAN_H()) as i32)
}

// ---- the height posts (the one hook) --------------------------------------------------

pub(crate) fn fn_height_post() -> usize { at(sym::FN_HEIGHT_POST) }
const SITES: [a2engine::Sym; 5] = [sym::SITE_POST0, sym::SITE_POST1, sym::SITE_POST2, sym::SITE_POST3, sym::SITE_POST4];
pub(crate) fn terrain_y_addr() -> usize { at(sym::TERRAIN_Y) }
pub(crate) fn green() -> usize { at(sym::GREEN) }
pub(crate) fn red() -> usize { at(sym::RED) }

static mut GATE: Option<Gate> = None;
static mut PREV_POST: usize = 0;

/// The plane's height posts are drawn to.
pub fn terrain_y() -> f32 {
    unsafe { rd::<f32>(terrain_y_addr()) }
}

/// The posts' colours: green above the plane, red below.
pub fn post_colours() -> ([f32; 3], [f32; 3]) {
    unsafe { (rd(green()), rd(red())) }
}

/// The engine's `DrawHeightPost`, for a post that is not ours.
pub fn draw_engine_post(pos: *const V3, cam: usize) {
    unsafe {
        let f: unsafe extern "C" fn(*const V3, usize) = core::mem::transmute(if PREV_POST != 0 { PREV_POST } else { fn_height_post() });
        f(pos, cam);
    }
}

/// Re-points the five calls of `DrawHeightPost` (once, at the first enable); each that
/// is not stock's is logged and left to the engine. Returns how many are ours.
pub fn install_posts(h: &mut Hooks) -> usize {
    unsafe { GATE = Some(h.gate()) };
    if !h.ready(&TABLE, hook::OVERLAYS_POSTS) {
        return 0;
    }
    let owner = h.owner();
    let mut n = 0;
    for s in SITES {
        let hook = Hook {
            owner,
            used_by: "height posts",
            kind: Kind::Call,
            site: Site::of(&TABLE, s),
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

// ---- the posts at DrawHeightPost's entry (a build whose companion owns its calls) ----

/// The return address of the latest call of `DrawHeightPost` that reached the entry
/// detour (the game draws on one thread).
static mut POST_CALLER: usize = 0;

/// `DrawHeightPost`'s entry: notes who called (`[esp]`) and goes on to `post_callee`
/// with the call as it was.
#[unsafe(naked)]
unsafe extern "C" fn post_entry() {
    core::arch::naked_asm!(
        "mov eax, dword ptr [esp]",
        "mov dword ptr [{caller}], eax",
        "jmp {f}",
        caller = sym POST_CALLER,
        f = sym post_callee,
    )
}

/// `DrawHeightPost` at its entry: a post noted for the view only while the build's
/// companion module calls it (Roots: FleetOpsHook's callbacks for the five calls, which
/// skip a post during its clean screenshot and under `DISABLE_HEIGHT_CHANGE`); any other
/// caller, render3d's own `draw_engine_post` included, gets the function as it was.
unsafe extern "C" fn post_callee(pos: *const V3, cam: usize) {
    let armed = engine::in_companion(unsafe { POST_CALLER });
    if armed && super::posts::in_view() && unsafe { GATE.is_some_and(|g| g.open()) } && !pos.is_null() && super::posts::note_raw(unsafe { core::ptr::read_unaligned(pos) }) {
        return;
    }
    draw_engine_post(pos, cam);
}

/// Whether this build takes the posts at `DrawHeightPost`'s entry (the profile's value).
pub fn posts_at_callee() -> bool {
    value::OV_POSTS_AT_CALLEE() != 0
}

/// Installs the callee hook (`overlays.posts_callee`; PLAN-FLEETOPS.md, "The rules of
/// (c)", rule 4), for a build whose profile asks for it. True when it is in.
pub fn install_posts_callee(h: &mut Hooks) -> bool {
    unsafe { GATE = Some(h.gate()) };
    if !posts_at_callee() || !h.ready(&TABLE, hook::OVERLAYS_POSTS_CALLEE) {
        return false;
    }
    let len = value::OV_POST_MOVED().max(0) as usize;
    let Some(prev) = h.install_entry("height posts (at the callee)", &TABLE, sym::FN_HEIGHT_POST, len, post_entry as *const () as usize) else {
        return false;
    };
    unsafe { PREV_POST = prev.addr() };
    true
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

pub(crate) fn buckets() -> usize { at(sym::BUCKETS) }
pub(crate) fn fn_render_selection() -> usize { at(sym::FN_RENDER_SELECTION) } // RenderSelectionEffects, cdecl
pub(crate) fn fn_entity_get() -> usize { at(sym::FN_ENTITY_GET) } // Entity::Get(handle), cdecl
pub(crate) fn fn_relation() -> usize { at(sym::FN_RELATION) } // GameObject::GetTeamRelation(team), thiscall
pub(crate) fn fn_is_scaled() -> usize { at(sym::FN_IS_SCALED) } // Entity::IsScaled, thiscall
pub(crate) fn fn_scale_factor() -> usize { at(sym::FN_SCALE_FACTOR) } // Entity::GetScaleFactor, thiscall: Vector3*

pub fn entity(handle: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(fn_entity_get())(handle) }
}

/// An object's team colour (`+0xf0`, the team, `+8`).
pub fn team_colour(o: usize) -> V3 {
    team_rgb(field(o, off::OV_OBJ_TEAM_PTR()))
}

/// Another UI has put a `ret` in `RenderSelectionEffects` (it draws the selection).
pub fn selection_effects_returns() -> bool {
    (unsafe { rd::<u8>(fn_render_selection()) }) == 0xc3
}

/// Every selection entry of the engine's sixteen buckets.
pub fn each_selection_entry(mut f: impl FnMut(usize)) {
    let next_at = off::OV_SEL_NEXT();
    for b in 0..value::OV_BUCKET_COUNT().max(0) as usize {
        let mut e = unsafe { rd::<usize>(buckets() + b * 4) };
        let mut guard = 0;
        while e != 0 && guard < 10000 {
            guard += 1;
            let next = field(e, next_at);
            f(e);
            e = next;
        }
    }
}

/// An entry's object handle (`+0x88`), flag bits (`+0x84`), pulse phase (`+0x8c`) and
/// scale (`+0x90`).
pub fn entry_handle(e: usize) -> usize {
    field(e, off::OV_SEL_HANDLE())
}
pub fn entry_bits(e: usize) -> u32 {
    field(e, off::OV_SEL_BITS()) as u32
}
pub fn entry_phase(e: usize) -> f32 {
    field_f32(e, off::OV_SEL_PHASE())
}
pub fn entry_scale(e: usize) -> V3 {
    unsafe { rd(e + off::OV_SEL_SCALE()) }
}

/// The craft's health measure the bubble pulses with (`+0x158`).
pub fn object_pulse(o: usize) -> f32 {
    field_f32(o, off::OV_OBJ_PULSE())
}

/// `GameObject::GetTeamRelation(team)`.
pub fn team_relation(o: usize, team: i32) -> i32 {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, i32) -> i32>(fn_relation())(o, team) }
}

/// The object's scale factor when the entity is scaled.
pub fn entity_scale_factor(o: usize) -> Option<V3> {
    unsafe {
        if core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(fn_is_scaled())(o) != 0 {
            let f = core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> *const V3>(fn_scale_factor())(o);
            if !f.is_null() {
                return Some(core::ptr::read_unaligned(f));
            }
        }
    }
    None
}

// ---- the ghosts and the build ghost ---------------------------------------------------

pub(crate) fn ghost_nodes() -> usize { at(sym::GHOST_NODES) }
pub(crate) fn fn_team_get() -> usize { at(sym::FN_TEAM_GET) } // Team::GetTeam(int), cdecl: Team&
pub(crate) fn fn_class_odf() -> usize { at(sym::FN_CLASS_ODF) } // GameObjectClass::GetOdfName, thiscall
pub(crate) fn fn_distance_cull() -> usize { at(sym::FN_DISTANCE_CULL) } // GameObjectInstance::DistanceCull(eye, pos), thiscall
pub(crate) fn vt_database() -> usize { at(sym::VT_DATABASE) }
// A spirit's `Render(ST3D_Camera*)` (vtable `+0xc`); Storm3D's lighting material in
// place of every mesh's (`+0xa8`), its additive scale (`+0x4`) and team tint (`+0x34`).

/// `Ghost+0x24`: the material every ghost mesh is lit with, and its ambient and
/// diffuse (`+0x18`, `+0x24`); a ghost without one is lit dark grey.
pub fn ghost_material(g: usize) -> (usize, V3, V3) {
    let mat = field(g, off::OV_GHOST_MATERIAL());
    let (amb, diff) = if mat == 0 {
        (r3d_core::math::v3(0.1, 0.1, 0.1), r3d_core::math::v3(0.1, 0.1, 0.1))
    } else {
        unsafe { (rd::<V3>(mat + off::OV_MAT_AMBIENT()), rd::<V3>(mat + off::OV_MAT_DIFFUSE())) }
    };
    (mat, amb, diff)
}

/// `Ghost+0x28`: the tint's factor.
pub fn ghost_factor(g: usize) -> f32 {
    field_f32(g, off::OV_GHOST_FACTOR())
}

/// Every spirit in `Ghost::ghostList`, with the team colour it is tinted with (times
/// `factor`); false if the list is not there.
pub fn each_spirit(mut f: impl FnMut(usize, V3)) -> bool {
    let head = unsafe { rd::<usize>(ghost_nodes()) };
    if head == 0 {
        return false;
    }
    let team_at = off::OV_SPIRIT_TEAM();
    engine::for_each_list_node(head, |n| {
        let sp = engine::list_node_value(n);
        if sp == 0 {
            return;
        }
        let team = unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(i32) -> usize>(fn_team_get())(field(sp, team_at) as i32) };
        f(sp, team_rgb(team));
    });
    true
}

/// A spirit the module does not draw: its own `Render`, under stock's state (the ghost
/// material, the additive scale and the team tint on `Storm3D`).
pub fn render_spirit(sp: usize, cam: usize, mat: usize, additive: f32, tint: V3) {
    let s3d = storm3d();
    unsafe {
        core::ptr::write_unaligned((s3d + off::OV_S3D_MATERIAL()) as *mut usize, mat);
        core::ptr::write_unaligned((s3d + off::OV_S3D_ALPHA()) as *mut f32, additive);
        core::ptr::write_unaligned((s3d + off::OV_S3D_TEAM()) as *mut V3, tint);
        let r: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(sp), off::OV_VT_SPIRIT_RENDER()));
        r(sp, cam);
    }
}

/// Puts `Storm3D`'s ghost state back after `render_spirit`.
pub fn end_spirits() {
    let s3d = storm3d();
    unsafe {
        core::ptr::write_unaligned((s3d + off::OV_S3D_ALPHA()) as *mut f32, 1.0);
        core::ptr::write_unaligned((s3d + off::OV_S3D_MATERIAL()) as *mut usize, 0);
    }
}

/// A spirit that carries a plain SOD: (instance `+0x10`, class `+4`); None if its
/// instance has no database of `VT_DATABASE`'s kind.
pub fn spirit_instance(sp: usize) -> Option<(usize, usize)> {
    let inst = field(sp, off::OV_SPIRIT_INSTANCE());
    let db = instance_database(inst);
    if inst == 0 || db == 0 || engine::vtable(db) != vt_database() {
        return None;
    }
    Some((inst, field(sp, off::OV_SPIRIT_CLASS())))
}

/// A class's ODF name without its extension.
pub fn class_odf(class: usize) -> &'static str {
    if class == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(fn_class_odf());
        let s = rt::cstr_at(f(class), 260);
        match s.len().checked_sub(4) {
            // lint-allow(content): the engine names a class by its ODF file; the extension is the format's, not a set's.
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

/// The instance's origin (`+0x68`), scale (`+0x28`) and placement (`+0x44`).
pub fn instance_origin(inst: usize) -> V3 {
    unsafe { rd(inst + off::OV_INST_ORIGIN()) }
}
pub fn instance_scale(inst: usize) -> V3 {
    raw_scale(inst)
}
pub fn instance_world(inst: usize) -> M34 {
    unsafe { rd(inst + off::OV_INST_MATRIX()) }
}

/// `GameObjectInstance::DistanceCull(eye, pos)`, as `GameObjectSpirit::Render` asks it.
pub fn distance_culled(inst: usize, eye: &V3, pos: &V3) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(fn_distance_cull());
        f(inst, eye, pos) != 0
    }
}

/// The station being placed (`PlaceholderFeature`): its class (`+0x34`), placement
/// (`+0x38`), colour (`+0x68`) and whether the placement is blocked (`+0x74` clear).
pub fn placeholder(f: usize) -> (usize, M34, V3, bool) {
    (
        field(f, off::OV_PH_CLASS()),
        unsafe { rd(f + off::OV_PH_MATRIX()) },
        unsafe { rd(f + off::OV_PH_COLOUR()) },
        field_u8(f, off::OV_PH_PLACEABLE()) == 0,
    )
}

// ---- the pivot marker -----------------------------------------------------------------

/// `TacticalCamera`'s vtable and its `Render(ST3D_Camera*, bool)` (slot 8, `+0x20`).
pub fn vt_tactical_camera() -> usize { at(sym::VT_TACTICAL_CAMERA) }
pub(crate) fn fn_render() -> usize { at(sym::FN_RENDER) }
/// `TacticalCamera::Render` as stock has it, to its `ret 8`.
const RENDER_BYTES: [u8; 54] = [
    0x55, 0x8b, 0xec, 0x8a, 0x45, 0x0c, 0x84, 0xc0, 0x74, 0x28, 0x8b, 0x81, 0xe0, 0x00, 0x00, 0x00, 0x85, 0xc0,
    0x74, 0x1e, 0x8a, 0x91, 0xe4, 0x00, 0x00, 0x00, 0x84, 0xd2, 0x74, 0x14, 0x83, 0x3d, 0x10, 0xb6, 0x76, 0x00,
    0x01, 0x75, 0x0b, 0x8b, 0x4d, 0x08, 0x8b, 0x10, 0x51, 0x8b, 0xc8, 0xff, 0x52, 0x18, 0x5d, 0xc2, 0x08, 0x00,
];
/// `ST3D_Instance`'s vtable: the marker must be a plain instance.
pub(crate) fn vt_instance() -> usize { at(sym::VT_INSTANCE) }


/// Whether `TacticalCamera::Render` is stock's (its slot and its bytes).
pub fn tactical_render_is_stock() -> bool {
    field(vt_tactical_camera(), off::OV_VT_TCAM_RENDER()) == fn_render() && unsafe { core::slice::from_raw_parts(fn_render() as *const u8, RENDER_BYTES.len()) } == RENDER_BYTES
}

/// The camera's marker instance (`+0xe0`).
pub fn camera_marker(f: usize) -> usize {
    field(f, off::OV_CAM_MARKER())
}

/// Whether the camera has no entity to follow (`+0xe4`).
pub fn camera_free(f: usize) -> bool {
    field_u8(f, off::OV_CAM_FREE()) != 0
}

/// Whether an instance is a plain `ST3D_Instance`.
pub fn is_plain_instance(inst: usize) -> bool {
    engine::vtable(inst) == vt_instance()
}

// ---- the move paths -------------------------------------------------------------------

use crate::syms::field;

/// A path's points (`AiPath`: `+0x0` the count, `+0x4` the `Vector3`s), at most 256.
pub fn path_points(path: usize, out: &mut alloc::vec::Vec<V3>) {
    out.clear();
    if path == 0 {
        return;
    }
    let n = field(path, field::PATH_COUNT()) as i32;
    let p = field(path, field::PATH_POINTS());
    if n <= 0 || n > 256 || p == 0 {
        return;
    }
    for i in 0..n as usize {
        out.push(unsafe { rd::<V3>(p + i * 12) });
    }
}

/// Whether the path is a patrol's loop (its type).
pub fn path_is_loop(path: usize) -> bool {
    path != 0 && field(path, field::PATH_TYPE()) as i32 == value::PATH_PATROL()
}

/// The overview (`cOverViewImp`), read through `g_pOverView`, which its constructor sets:
/// on stock the static object beside the pointer, on Roots the enlarged one FleetOpsHook
/// constructs in its place (the static one is never built there). 0 before it is made.
pub fn overview() -> usize {
    let p = at(sym::OVERVIEW_PTR);
    if p == 0 { 0 } else { unsafe { rd::<usize>(p) } }
}

/// The waypoint key's live state (`cOverViewImp+0x30c`, the `enter_waypoint` binding):
/// held or not; false before the overview has bound it.
pub fn waypoint_key() -> bool {
    let ov = overview();
    let key = if ov == 0 { 0 } else { field(ov, field::OV_WAYPOINT_KEY()) };
    key != 0 && unsafe { rd::<i32>(key) } != 0
}

/// The path the player is sketching (`cOverViewImp+0x264`) while the waypoint key
/// (`+0x30c`, the live state of the `enter_waypoint` binding) is held; 0 otherwise.
pub fn sketch() -> usize {
    if !waypoint_key() {
        return 0;
    }
    field(overview(), field::OV_ADD_PATH())
}

/// The overview's selected objects (`GetSelectNum`, `GetSelectList`: the entity ids,
/// wherever QOL's `MaxSelection` or Roots' enlarged overview keeps the array), as objects.
pub fn selected_objects(out: &mut alloc::vec::Vec<usize>) {
    out.clear();
    let ov = overview();
    if ov == 0 {
        return;
    }
    let vt = engine::vtable(ov);
    if vt == 0 {
        return;
    }
    unsafe {
        let num: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(rd::<usize>(vt + field::OV_VT_SELECT_NUM()));
        let list: unsafe extern "thiscall" fn(usize) -> *const i32 = core::mem::transmute(rd::<usize>(vt + field::OV_VT_SELECT_LIST()));
        let n = num(ov);
        let ids = list(ov);
        if n <= 0 || n > 512 || ids.is_null() {
            return;
        }
        for i in 0..n as usize {
            let o = entity(core::ptr::read_unaligned(ids.add(i)) as usize);
            if o != 0 {
                out.push(o);
            }
        }
    }
}

/// A ship of the player's team (a craft, flags `0xc`): where it is.
pub fn ship_position(o: usize, team: i32) -> Option<V3> {
    if o == 0 || r3d_core::engine::obj::flags(o) & 0xc != 0xc || field(o, field::OBJ_TEAM()) as i32 != team {
        return None;
    }
    Some(unsafe { rd::<V3>(o + field::OBJ_POSITION()) })
}

/// A ship of the player's team on a path: (where it is, its path, the point it flies
/// to). The craft's process (`+0x44`) holds the path it follows (`+0x88`) and the
/// index of the point it is heading to (`+0x12c`) while its command (`+0x38`) is a move
/// or a patrol.
pub fn ship_path(o: usize, team: i32) -> Option<(V3, usize, i32)> {
    let pos = ship_position(o, team)?;
    let proc_ = field(o, field::OBJ_PROCESS());
    if proc_ == 0 {
        return None;
    }
    let cmd = field(proc_, field::PROC_COMMAND()) as i32;
    if cmd != value::CMD_GO() && cmd != value::CMD_PATROL() {
        return None;
    }
    let path = field(proc_, field::PROC_PATH());
    if path == 0 {
        return None;
    }
    Some((pos, path, field(proc_, field::PROC_POINT()) as i32))
}

static mut PATH_GATE: Option<Gate> = None;
static mut PREV_DRAW_PATH: usize = 0;
static mut PATHS_READY: bool = false;

/// Whether the move paths' entries are all resolved (nothing of them is read before).
pub fn paths_ready() -> bool {
    unsafe { PATHS_READY }
}

/// Re-points stock's draw of the sketch (`mNormalModeRender`'s call of `mDrawPath`),
/// once, at the first enable. False when the call is not stock's (the engine keeps
/// drawing its line while the key is held).
pub fn install_paths(h: &mut Hooks) -> bool {
    unsafe { PATH_GATE = Some(h.gate()) };
    if !h.ready(&TABLE, hook::OVERLAYS_PATHS) {
        return false;
    }
    unsafe { PATHS_READY = true };
    let hook = Hook {
        owner: h.owner(),
        used_by: "move paths",
        kind: Kind::Call,
        site: Site::of(&TABLE, sym::SITE_DRAW_PATH),
        to: hook_draw_path as *const () as usize,
        chain: false,
    };
    match h.install(hook) {
        Some(p) => {
            unsafe { PREV_DRAW_PATH = p.addr() };
            true
        }
        None => false,
    }
}

/// `cOverViewImp::mDrawPath(path, colour, n)`, re-pointed: nothing while ours are drawn
/// (`paths::ours`), else the call that was there.
unsafe extern "thiscall" fn hook_draw_path(ov: usize, path: usize, colour: usize, n: i32) {
    if super::paths::ours() && unsafe { PATH_GATE.is_some_and(|g| g.open()) } {
        return;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize, usize, i32) = core::mem::transmute(if PREV_DRAW_PATH != 0 { PREV_DRAW_PATH } else { at(sym::FN_DRAW_PATH) });
        f(ov, path, colour, n);
    }
}
