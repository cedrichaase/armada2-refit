//! The minimap's engine side (HUD.md, "The minimap"): what `MapRadar` and its
//! `RadarComponent` show, read as the stock panel reads it, and the camera move and the
//! order a click on it gives, made through the engine's own functions.
//!
//! - **The square.** The radar maps the world's x and z into a unit square
//!   (`RadarComponent::ConvertWorldToSquareNormal` / `ConvertSquareNormalToWorld`,
//!   called here): the map's rectangle (`g_terrain`) centred in a square `+0x5c` world
//!   units across, v running against z. Everything the radar draws is placed in it.
//! - **The picture.** `m_mapSprite`'s texture, which `CacheMap` fills with the map's
//!   features (nebulae, asteroids, ...) whenever `g_cachedMapNeedsUpdate` asks, from the
//!   panel's always-step, so it stays current with the stock `Render` skipped. We draw
//!   that texture (its Display8 mirror, as render3d reads the fog's) over our square.
//! - **Fog and shroud.** `Scanner::m_fog` / `m_shroud`: a grid of one `u32` per scan
//!   cell (`+0x4`), `+0xc` wide and `+0x10` tall, each a mask of the teams that see (or
//!   have seen) the cell. A cell is clear for the player when it shares a bit with
//!   `Entity::GetTeamsVisibleToUser`. Stock copies a pattern into `m_fogSprite` /
//!   `m_shroudSprite` for every other cell, the grid centred in the texture; we read the
//!   grid ourselves and draw our own mask, so it is never the engine's texture that is
//!   resized (CLAUDE.md, `UImid`).
//! - **Blips.** `RenderMapObjects`: every object of `GameObject::objectList` that is not
//!   being removed (`+0x113`) and that the player can see (`CanUserSee`, vtable `+0x08`),
//!   then the last-known enemy objects of `Ghost::ghostList`, each through
//!   `RenderMapObject`'s tests, kept here: the class (`+0x40`) is not hidden from the
//!   radar (`+0x18c`), it is inside the map, and a craft class (`IsCraftClass`) that is
//!   not a planet passes the ship or station filter (`sg_shipFilterState`,
//!   `sg_stationFilterState`: 0 all, 1 allies, 2 enemies, 3 neutral, by the user team's
//!   `IsUnconfirmedAlly` / `Enemy` / `Neutral`). A class with a radar sprite (`+0x190`,
//!   the planets and moons) shows the sprite; the others a dot of the team's colour
//!   (`Team::GetTeam(team)+0x14`), as big as `ComputeDot` makes it: `MapSize` (vtable
//!   `+0x24`) when the class says it has one (`+0x184`), else 2 for a ship, 3 for a
//!   station (`CraftClass+0x20d`), 1 for anything else, in texels of a 128-texel map.
//! - **Marks.** `RadarComponent+0x4c`: the pings (`AddMessageMark`), a list of
//!   `MessageMark`s: a position, a ring's radius in pixels (`+0x1c`, counted down by the
//!   always-step) and a colour (`+0x20`).
//! - **The camera.** In the RTS view stock outlines what the camera sees: the 3D view's
//!   corners cast on the map's middle height (`ST3D_Camera::ProjectScreenPointToWorldRay`,
//!   the distance kept between the camera's near and far planes).
//!
//! **Clicks** are what `RadarComponent::Simulate` does with the stock panel's: the
//! point under the pointer in the world (the map's middle height); the command button
//! (the right, or the left with `UserProfile+0x24`'s swapped buttons) with something
//! selected asks `ActionMode::GetAction` what clicking there means, turns a plain move
//! (`& 0xfff == 4`, without Alt) into a move to that point at the cursor's height
//! (`0x2b`, keeping the queue bit `0x4000`), names it to the overview
//! (`SetCursorAiAction`) and hands it to the bar's `BroadcastCommand`, as a click in the
//! 3D view orders; the other button, held (or either with nothing selected), moves the
//! camera there (`gCameraManager`'s camera, vtable `+0x54`). In the tactical view a
//! click moves the camera and snaps it to the nearest visible object
//! (`CameraManager::SnapToNearestVisible`). We make both from the panel's always-step
//! (below), where stock makes them from its focus-step, so they act in the game's
//! simulate and not in a frame being drawn.
//!
//! **The always-step hook.** `MapRadar::AlwaysGameSimulate` asks
//! `Entity::GetTeamsVisibleToUser` once a game frame (0x4eb3b2); that call is
//! re-pointed at `teams_visible_step`, which calls it and then makes the clicks queued
//! by the UI frame.
use super::{field, hook as patch, rd};
use crate::syms_radar::{field, hook, sym, value, TABLE};
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An engine address of the profile: 0 (and nothing to read) unless the hook that
/// uses it is ready.
fn at(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

/// The pointer a global holds; 0 while its hook is not ready.
fn ptr_of(s: a2engine::Sym) -> usize {
    let a = at(s);
    if a == 0 { 0 } else { rd_ptr(a) }
}

/// Entries the hooks check in memory and the code never reads: named here so that the
/// profile's entries all have a reader.
#[allow(dead_code)]
const CHECKED: [a2engine::Sym; 5] =
    [sym::RADAR_SLOT_SET_CURSOR_ACTION, sym::RADAR_SLOT_WHERE_CURSOR, sym::RADAR_SLOT_BROADCAST, sym::RADAR_CURSOR_TABLE_A, sym::RADAR_CURSOR_TABLE_B];

/// `mapRadar` (an instance): its `RadarComponent` is at `+0x28`, the ship and station
/// filter buttons (`StandardButton`s) at `+0x2c` / `+0x30`. Its address, or 0 while the
/// minimap's hook is not ready.
pub fn map_radar() -> usize {
    at(sym::RADAR_MAP_RADAR)
}

pub fn radar() -> usize {
    field(map_radar(), field::RADAR_MR_RADAR)
}

/// The world's (x, z) to the radar's unit square (u right, v down).
pub fn to_square(r: usize, p: [f32; 3]) -> (f32, f32) {
    let a = at(sym::RADAR_TO_SQUARE);
    if a == 0 {
        return (0.0, 0.0);
    }
    let mut out = [0f32; 2];
    let f: unsafe extern "thiscall" fn(usize, *mut f32, *const f32) -> *mut f32 = unsafe { core::mem::transmute(a) };
    unsafe { f(r, out.as_mut_ptr(), p.as_ptr()) };
    (out[0], out[1])
}

/// A point of the unit square in the world, at the map's middle height.
pub fn to_world(r: usize, u: f32, v: f32) -> [f32; 3] {
    let a = at(sym::RADAR_TO_WORLD);
    let mut out = [0f32; 3];
    if a == 0 {
        return out;
    }
    let i = [u, v];
    let f: unsafe extern "thiscall" fn(usize, *mut f32, *const f32) -> *mut f32 = unsafe { core::mem::transmute(a) };
    unsafe { f(r, out.as_mut_ptr(), i.as_ptr()) };
    out
}

/// The map's box: x0 y0 z0 x1 y1 z1.
pub fn terrain() -> [f32; 6] {
    let a = at(sym::RADAR_TERRAIN);
    if a == 0 {
        return [0.0; 6];
    }
    unsafe { rd(a) }
}

fn inside(p: [f32; 3]) -> bool {
    if at(sym::RADAR_TERRAIN) == 0 {
        return false;
    }
    let t = terrain();
    p[0] >= t[0] && p[0] <= t[3] && p[2] >= t[2] && p[2] <= t[5]
}

/// The map's picture: the Direct3D 8 texture `CacheMap` draws into, and the warp
/// suppression layer over it.
pub fn map_texture() -> usize {
    field(ptr_of(sym::RADAR_MAP_SPRITE), field::RADAR_SPRITE_TEXTURE)
}
pub fn warp_texture() -> usize {
    field(ptr_of(sym::RADAR_WARP_SPRITE), field::RADAR_SPRITE_TEXTURE)
}
pub fn background_sprite(r: usize) -> usize {
    field(r, field::RADAR_RC_BACKGROUND)
}

fn rd_ptr(a: usize) -> usize {
    unsafe { rd::<usize>(a) }
}

/// The fog layer's texture size, in which the scan grid is centred.
pub fn fog_texture_size() -> (u32, u32) {
    let t = field(ptr_of(sym::RADAR_FOG_SPRITE), field::RADAR_SPRITE_TEXTURE);
    if t == 0 {
        return (0, 0);
    }
    unsafe { (rd::<u32>(t + field::RADAR_TEX_W), rd::<u32>(t + field::RADAR_TEX_H)) }
}

/// A scan grid: its size and cells (one team mask each, row by row).
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: &'static [u32],
}

fn grid(s: a2engine::Sym) -> Option<Grid> {
    let g = ptr_of(s);
    if g == 0 {
        return None;
    }
    unsafe {
        let (w, h) = (rd::<i32>(g + field::RADAR_GRID_W), rd::<i32>(g + field::RADAR_GRID_H));
        let p = rd::<usize>(g + field::RADAR_GRID_CELLS);
        if p == 0 || !(1..=4096).contains(&w) || !(1..=4096).contains(&h) {
            return None;
        }
        Some(Grid { w: w as usize, h: h as usize, cells: core::slice::from_raw_parts(p as *const u32, (w * h) as usize) })
    }
}

pub fn fog_grid() -> Option<Grid> {
    grid(sym::RADAR_SCAN_FOG)
}
pub fn shroud_grid() -> Option<Grid> {
    grid(sym::RADAR_SCAN_SHROUD)
}

pub fn teams_visible() -> u32 {
    let a = at(sym::RADAR_TEAMS_VISIBLE);
    if a == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn() -> u32 = unsafe { core::mem::transmute(a) };
    unsafe { f() }
}

fn game_setup() -> usize {
    let t = ptr_of(sym::RADAR_TRANSPORT);
    let a = at(sym::RADAR_GET_GAME_SETUP);
    if t == 0 || a == 0 {
        return 0;
    }
    let f: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(a) };
    unsafe { f(t) }
}

fn editor() -> bool {
    let p = ptr_of(sym::RADAR_USER_PROFILE);
    p != 0 && unsafe { rd::<u8>(p + field::RADAR_UP_EDITOR) } != 0
}

/// Whether the game has fog of war and shroud, as `RadarComponent::Render` asks.
pub fn fog_on() -> (bool, bool) {
    let (fa, sa) = (at(sym::RADAR_IS_FOG), at(sym::RADAR_IS_SHROUD));
    let gs = game_setup();
    if gs == 0 || fa == 0 || sa == 0 || editor() {
        return (false, false);
    }
    unsafe {
        let fog: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(fa);
        let shroud: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(sa);
        (fog(gs), shroud(gs))
    }
}

/// `View_Record`: 0 the RTS view, 1 tactical, 2 cinematic; `u32::MAX` while the
/// minimap's hook is not ready.
pub fn view() -> u32 {
    let a = at(sym::RADAR_VIEW_RECORD);
    if a == 0 {
        return u32::MAX;
    }
    unsafe { rd(a) }
}

/// The ship and station filters: 0 all, 1 allies, 2 enemies, 3 neutral.
pub fn filters() -> (u32, u32) {
    let (s, st) = (at(sym::RADAR_SHIP_FILTER), at(sym::RADAR_STATION_FILTER));
    if s == 0 || st == 0 {
        return (0, 0);
    }
    unsafe { (rd(s), rd(st)) }
}

/// The filter buttons' art.
pub fn filter_sprites() -> (usize, usize) {
    let m = map_radar();
    (
        field(field(m, field::RADAR_MR_SHIP_BUTTON), field::RADAR_BTN_SPRITE),
        field(field(m, field::RADAR_MR_STATION_BUTTON), field::RADAR_BTN_SPRITE),
    )
}

// ---- the blips ----------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ship,
    Station,
    /// Anything else on the radar: mines, wrecks, ...
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct Blip {
    pub u: f32,
    pub v: f32,
    pub team: i32,
    pub kind: Kind,
    /// The dot's size in texels of a 128-texel map.
    pub size: f32,
    /// The class's radar sprite (planets, moons), or 0 for a dot.
    pub sprite: usize,
    /// A last-known position under the fog (`Ghost::ghostList`).
    pub ghost: bool,
    pub selected: bool,
    pub colour: [f32; 3],
}

/// `__RTDynamicCast` from a `GameObjectClass` to the type named by `to`; 0 when it is
/// not one (or the hook is not ready).
fn cast(p: usize, to: a2engine::Sym) -> usize {
    let (a, from, to) = (at(sym::RADAR_RT_DYNAMIC_CAST), at(sym::RADAR_TD_GAME_OBJECT_CLASS), at(to));
    if p == 0 || a == 0 || from == 0 || to == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(usize, i32, usize, usize, i32) -> usize = unsafe { core::mem::transmute(a) };
    unsafe { f(p, 0, from, to, 0) }
}

fn team_object(team: i32) -> usize {
    let a = at(sym::RADAR_TEAM_GET);
    if a == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(a) };
    unsafe { f(team) }
}

/// The team's colour (0..1), as the radar's dots and the 3D view's team shells use it.
pub fn team_colour(team: i32) -> [f32; 3] {
    let t = team_object(team);
    if t == 0 {
        return [0.7, 0.7, 0.7];
    }
    unsafe { rd(t + field::RADAR_TEAM_COLOUR) }
}

/// How a team stands to the player: 0 the player's own, 1 an ally, 2 an enemy, 3
/// neutral (the user team's `IsUnconfirmedAlly` / `Enemy`, as the radar's filters ask).
pub fn relation(team: i32) -> u8 {
    if team == super::selection::user_team() {
        return 0;
    }
    let user = team_object(super::selection::user_team());
    let (ia, ie) = (at(sym::RADAR_IS_ALLY), at(sym::RADAR_IS_ENEMY));
    if user == 0 || ia == 0 || ie == 0 {
        return 3;
    }
    unsafe {
        let ally: unsafe extern "thiscall" fn(usize, i32) -> bool = core::mem::transmute(ia);
        let enemy: unsafe extern "thiscall" fn(usize, i32) -> bool = core::mem::transmute(ie);
        if ally(user, team) {
            1
        } else if enemy(user, team) {
            2
        } else {
            3
        }
    }
}

fn passes_filter(cls: usize, team: i32, filters: (u32, u32), user: usize) -> bool {
    let vt = rd_ptr(cls);
    let is_craft: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(vt + field::RADAR_VT_IS_CRAFT_CLASS)) };
    if !unsafe { is_craft(cls) } || cast(cls, sym::RADAR_TD_PLANET_CLASS) != 0 {
        return true;
    }
    let station = unsafe { rd::<u8>(cls + field::RADAR_CLS_STATION) } != 0;
    let f = if station { filters.1 } else { filters.0 };
    let test = match f {
        1 => at(sym::RADAR_IS_ALLY),
        2 => at(sym::RADAR_IS_ENEMY),
        3 => at(sym::RADAR_IS_NEUTRAL),
        _ => return true,
    };
    if user == 0 || test == 0 {
        return true;
    }
    let g: unsafe extern "thiscall" fn(usize, i32) -> bool = unsafe { core::mem::transmute(test) };
    unsafe { g(user, team) }
}

fn blip(r: usize, cls: usize, pos: [f32; 3], team: i32, ghost: bool, selected: bool, filters: (u32, u32), user: usize) -> Option<Blip> {
    if cls == 0 || unsafe { rd::<u32>(cls + field::RADAR_CLS_OFF_RADAR) } != 0 || !inside(pos) {
        return None;
    }
    if !passes_filter(cls, team, filters, user) {
        return None;
    }
    let craft = cast(cls, sym::RADAR_TD_CRAFT_CLASS);
    let station = craft != 0 && unsafe { rd::<u8>(craft + field::RADAR_CLS_STATION) } != 0;
    let has_size = unsafe { rd::<u8>(cls + field::RADAR_CLS_HAS_MAP_SIZE) } != 0;
    let size = if has_size {
        let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(rd::<usize>(rd_ptr(cls) + field::RADAR_VT_MAP_SIZE)) };
        unsafe { f(cls) as f32 }
    } else if craft != 0 {
        if station { 3.0 } else { 2.0 }
    } else {
        1.0
    };
    let (u, v) = to_square(r, pos);
    Some(Blip {
        u,
        v,
        team,
        kind: if craft == 0 { Kind::Other } else if station { Kind::Station } else { Kind::Ship },
        size,
        sprite: field(cls, field::RADAR_CLS_RADAR_SPRITE),
        ghost,
        selected,
        colour: team_colour(team),
    })
}

/// Everything the radar shows, ghosts first (stock draws them first).
pub fn blips(r: usize) -> Vec<Blip> {
    let mut out = Vec::new();
    let list = ptr_of(sym::RADAR_OBJECT_LIST);
    if r == 0 || list == 0 {
        return out;
    }
    let filters = filters();
    let user = team_object(super::selection::user_team());
    unsafe {
        let head = ptr_of(sym::RADAR_GHOST_LIST);
        if head != 0 {
            let mut n = rd::<usize>(head);
            let mut guard = 0;
            while n != head && n != 0 && guard < 8192 {
                let s = field(n, 8);
                if s != 0 {
                    let pf: unsafe extern "thiscall" fn(usize) -> *const [f32; 3] = core::mem::transmute(rd::<usize>(rd_ptr(s) + field::RADAR_VT_SPIRIT_POS));
                    let p = pf(s);
                    if !p.is_null() {
                        if let Some(b) = blip(r, field(s, field::RADAR_SPIRIT_CLASS), *p, rd::<i32>(s + field::RADAR_SPIRIT_TEAM), true, false, filters, user) {
                            out.push(b);
                        }
                    }
                }
                n = rd::<usize>(n);
                guard += 1;
            }
        }
        let head = field(list, 4);
        if head == 0 {
            return out;
        }
        let mut n = rd::<usize>(head);
        let mut guard = 0;
        while n != head && n != 0 && guard < 16384 {
            let o = field(n, 8);
            n = rd::<usize>(n);
            guard += 1;
            if o == 0 || rd::<u8>(o + field::RADAR_OBJ_REMOVED) != 0 {
                continue;
            }
            let see: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd_ptr(o) + field::RADAR_VT_CAN_USER_SEE));
            if !see(o) {
                continue;
            }
            let pos: [f32; 3] = rd(o + field::RADAR_OBJ_POS);
            if let Some(b) = blip(
                r,
                field(o, field::RADAR_OBJ_CLASS),
                pos,
                rd::<i32>(o + field::RADAR_OBJ_TEAM),
                false,
                rd::<u8>(o + field::RADAR_OBJ_SELECTED) != 0,
                filters,
                user,
            ) {
                out.push(b);
            }
        }
    }
    out
}

// ---- marks and the camera ---------------------------------------------------------

/// A ping: where, the ring's radius in the stock radar's pixels, its colour.
pub struct Mark {
    pub u: f32,
    pub v: f32,
    pub radius: f32,
    pub colour: [f32; 3],
}

pub fn marks(r: usize) -> Vec<Mark> {
    let mut out = Vec::new();
    let head = field(r, field::RADAR_RC_MARKS);
    if head == 0 {
        return out;
    }
    unsafe {
        let mut n = rd::<usize>(head);
        let mut guard = 0;
        while n != head && n != 0 && guard < 256 {
            let m = n + 8;
            let p: [f32; 3] = rd(m);
            let (u, v) = to_square(r, p);
            out.push(Mark { u, v, radius: rd::<i32>(m + field::RADAR_MARK_RADIUS) as f32 + 1.0, colour: rd(m + field::RADAR_MARK_COLOUR) });
            n = rd::<usize>(n);
            guard += 1;
        }
    }
    out
}

pub fn camera() -> usize {
    field(super::storm3d(), field::RADAR_S3D_CAMERA)
}

/// What the main camera sees of the map, in the unit square: its viewport's corners
/// (top-left, top-right, bottom-right, bottom-left) on the map's middle height. None
/// outside the RTS view.
pub fn view_outline(r: usize) -> Option<[(f32, f32); 4]> {
    let cam = camera();
    let ray_at = at(sym::RADAR_SCREEN_RAY);
    if cam == 0 || ray_at == 0 || view() != 0 {
        return None;
    }
    let t = terrain();
    let y = (t[1] + t[4]) * 0.5;
    unsafe {
        let vp: [f32; 4] = rd(cam + field::RADAR_CAM_VIEWPORT);
        let (near, far) = (rd::<f32>(cam + field::RADAR_CAM_NEAR), rd::<f32>(cam + field::RADAR_CAM_FAR));
        let pos: [f32; 3] = rd(cam + field::RADAR_CAM_POS);
        if vp[2] <= 0.0 || vp[3] <= 0.0 {
            return None;
        }
        let ray: unsafe extern "thiscall" fn(usize, *mut f32, *const f32, f32) -> *mut f32 = core::mem::transmute(ray_at);
        let mut out = [(0.0, 0.0); 4];
        for (i, (sx, sy)) in [(0.0, 0.0), (vp[2], 0.0), (vp[2], vp[3]), (0.0, vp[3])].into_iter().enumerate() {
            let mut res = [0f32; 4];
            let pt = [sx, sy];
            ray(cam, res.as_mut_ptr(), pt.as_ptr(), y);
            let mut d = res[0];
            if !d.is_finite() || d < near || d > far {
                d = far;
            }
            let p = [pos[0] + res[1] * d, y, pos[2] + res[3] * d];
            out[i] = to_square(r, p);
        }
        Some(out)
    }
}

/// `gTacticalCamera`: in the tactical view stock marks, instead of the outline, the
/// point the camera turns about (x, z) with a ring through the eye. The two in the unit
/// square; None outside the tactical view.
pub fn tactical_camera(r: usize) -> Option<((f32, f32), (f32, f32))> {
    let tc = at(sym::RADAR_TACTICAL_CAMERA);
    if tc == 0 || view() != 1 {
        return None;
    }
    unsafe {
        let eye = [rd::<f32>(tc + field::RADAR_TC_EYE_X), 0.0, rd::<f32>(tc + field::RADAR_TC_EYE_Z)];
        let centre = [rd::<f32>(tc + field::RADAR_TC_CENTRE_X), 0.0, rd::<f32>(tc + field::RADAR_TC_CENTRE_Z)];
        if !eye.iter().chain(centre.iter()).all(|v| v.is_finite()) {
            return None;
        }
        Some((to_square(r, centre), to_square(r, eye)))
    }
}

/// `shotDirector`: the cinematic camera's director. Its scene list is a `std::list` of
/// `Scene`s (the head node; each node's `+0` the next, the scene at `+8`), `+0x2c` how
/// many. A scene is a place worth filming (a fight: `AddShot` gathers the shots the
/// game offers it within a radius of one another): its centre (x, y, z) and its radius,
/// in world units. In the tactical view stock's `RenderCameraViewFrustum` rings each
/// one in grey (`ST3D_Colour_Grey`), the radius scaled by the square's side
/// (`RadarComponent`'s side field).
///
/// The shot director's scenes in the tactical view, each its centre in the unit
/// square and its radius in the square's units; empty in any other view.
pub fn shot_scenes(r: usize) -> Vec<((f32, f32), f32)> {
    let mut out = Vec::new();
    let (sd, vt) = (at(sym::RADAR_SHOT_DIRECTOR), at(sym::RADAR_SHOT_DIRECTOR_VTABLE));
    if view() != 1 || r == 0 || sd == 0 || vt == 0 || unsafe { rd::<usize>(sd) } != vt {
        return out;
    }
    let side: f32 = unsafe { rd(r + field::RADAR_RC_SIDE) };
    if !(side.is_finite() && side > 0.0) {
        return out;
    }
    let head = field(sd, field::RADAR_SD_SCENES);
    if head == 0 {
        return out;
    }
    let mut n = field(head, 0);
    while n != head && n != 0 && out.len() < 64 {
        let s = n + 8;
        let c: [f32; 3] = unsafe { rd(s + field::RADAR_SCENE_CENTRE) };
        let rad: f32 = unsafe { rd(s + field::RADAR_SCENE_RADIUS) };
        if c.iter().all(|v| v.is_finite()) && rad.is_finite() && rad > 0.0 {
            out.push((to_square(r, c), rad / side));
        }
        n = field(n, 0);
    }
    out
}

// ---- clicks -----------------------------------------------------------------------

/// What the UI frame asks the panel's next always-step to do.
#[derive(Clone, Copy)]
enum Pending {
    None,
    /// Move the camera to a point of the square (`snap`: and snap, tactical view).
    Camera(f32, f32, bool),
    /// Order the selection there.
    Order(f32, f32),
}

static mut PENDING: [Pending; 2] = [Pending::None, Pending::None];
static mut PENDING_AT: f64 = 0.0;
static mut HOOKED: bool = false;
static mut FILTER_PRESS: (u8, u8) = (0, 0);

/// The command button is the right one, unless the player swapped the buttons.
pub fn buttons_swapped() -> bool {
    let p = ptr_of(sym::RADAR_USER_PROFILE);
    p != 0 && unsafe { rd::<u8>(p + field::RADAR_UP_SWAPPED) } != 0
}

pub fn selection_count() -> i32 {
    let o = ptr_of(sym::RADAR_OVERVIEW_PTR);
    if o == 0 {
        return 0;
    }
    let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(rd::<usize>(rd_ptr(o) + field::RADAR_VT_SELECT_NUM)) };
    unsafe { f(o) }
}

pub fn queue_camera(u: f32, v: f32, snap: bool) {
    unsafe {
        PENDING[0] = Pending::Camera(u, v, snap);
        PENDING_AT = crate::rt::now();
    }
}

pub fn queue_order(u: f32, v: f32) {
    unsafe {
        PENDING[1] = Pending::Order(u, v);
        PENDING_AT = crate::rt::now();
    }
}

/// Presses a filter button `n` times at the next always-step (each press is the
/// next state: all, allies, enemies, neutral).
pub fn queue_filter(station: bool, n: u8) {
    unsafe {
        if station {
            FILTER_PRESS.1 = n.min(3)
        } else {
            FILTER_PRESS.0 = n.min(3)
        }
    }
}

/// Patches the always-step's call; false (nothing patched) when the minimap's hook is
/// not ready (the engine logs why).
pub fn install() -> bool {
    if unsafe { HOOKED } {
        return true;
    }
    if eng().ready(hook::HUD_RADAR).is_err() {
        return false;
    }
    let site = at(sym::RADAR_SITE_TEAMS_VISIBLE);
    let ok = patch::patch_call(site, at(sym::RADAR_TEAMS_VISIBLE), teams_visible_step as *const () as usize);
    unsafe { HOOKED = ok };
    if !ok {
        crate::log!("minimap: MapRadar::AlwaysGameSimulate's call at {:#x} is not stock: clicks on the minimap do nothing", site);
    }
    ok
}

pub fn hooked() -> bool {
    unsafe { HOOKED }
}

extern "C" fn teams_visible_step() -> u32 {
    let teams = teams_visible();
    unsafe {
        // A click the game did not take within half a second (paused) is dropped.
        let stale = crate::rt::now() - PENDING_AT > 0.5;
        for i in 0..2 {
            let p = core::mem::replace(&mut PENDING[i], Pending::None);
            if !stale {
                act(p);
            }
        }
        let (s, st) = core::mem::take(&mut FILTER_PRESS);
        let m = map_radar();
        let (nf, ns) = (at(sym::RADAR_NEXT_SHIP_FILTER), at(sym::RADAR_NEXT_STATION_FILTER));
        for _ in 0..s {
            if nf != 0 {
                let f: unsafe extern "C" fn(usize) = core::mem::transmute(nf);
                f(field(m, field::RADAR_MR_SHIP_BUTTON));
            }
        }
        for _ in 0..st {
            if ns != 0 {
                let f: unsafe extern "C" fn(usize) = core::mem::transmute(ns);
                f(field(m, field::RADAR_MR_STATION_BUTTON));
            }
        }
    }
    teams
}

fn alt_held() -> bool {
    let p = ptr_of(sym::RADAR_COMMAND_ALT);
    p != 0 && unsafe { rd::<i32>(p) } != 0
}

unsafe fn act(p: Pending) {
    let r = radar();
    if r == 0 {
        return;
    }
    match p {
        Pending::None => {}
        Pending::Camera(u, v, snap) => {
            let mut pos = to_world(r, u, v);
            if !inside(pos) {
                return;
            }
            let (cm, snap_fn) = (at(sym::RADAR_CAMERA_MANAGER), at(sym::RADAR_SNAP_TO_NEAREST));
            let cam = if cm == 0 { 0 } else { rd_ptr(cm) };
            if cam == 0 {
                return;
            }
            unsafe {
                let mv: unsafe extern "thiscall" fn(usize, *mut f32) = core::mem::transmute(rd::<usize>(rd_ptr(cam) + field::RADAR_VT_CAMERA_MOVE));
                mv(cam, pos.as_mut_ptr());
                let snap_at = at(sym::RADAR_TACTICAL_SNAP);
                if snap && !alt_held() && snap_fn != 0 && snap_at != 0 {
                    let s: unsafe extern "thiscall" fn(usize, *const f32, f32) = core::mem::transmute(snap_fn);
                    s(cm, pos.as_ptr(), rd::<f32>(snap_at));
                }
            }
        }
        Pending::Order(u, v) => {
            let mut pos = to_world(r, u, v);
            let o = ptr_of(sym::RADAR_OVERVIEW_PTR);
            let pal = ptr_of(sym::RADAR_PALETTE_PTR);
            let get_at = at(sym::RADAR_GET_ACTION);
            if !inside(pos) || o == 0 || pal == 0 || get_at == 0 || selection_count() == 0 {
                return;
            }
            unsafe {
                let get: unsafe extern "C" fn(*mut u32, usize, *const f32, usize) -> *mut u32 = core::mem::transmute(get_at);
                let mut action = 0u32;
                get(&mut action, 0, pos.as_ptr(), 0);
                if action & value::RADAR_ACTION_MASK as u32 == value::RADAR_ACTION_MOVE as u32 && !alt_held() {
                    action = (action & value::RADAR_ACTION_CANNOT as u32) | value::RADAR_ACTION_GO_WARP as u32;
                    let mut cur = [0f32; 3];
                    let wc: unsafe extern "thiscall" fn(usize, *mut f32) -> *mut f32 = core::mem::transmute(rd::<usize>(rd_ptr(o) + field::RADAR_VT_WHERE_CURSOR));
                    wc(o, cur.as_mut_ptr());
                    pos[1] = cur[1];
                }
                let set: unsafe extern "thiscall" fn(usize, u32) = core::mem::transmute(rd::<usize>(rd_ptr(o) + field::RADAR_VT_SET_CURSOR_ACTION));
                set(o, action);
                let bc: unsafe extern "thiscall" fn(usize, u32, *const f32, usize) -> bool = core::mem::transmute(rd::<usize>(rd_ptr(pal) + field::RADAR_VT_BROADCAST));
                let ok = bc(pal, action, pos.as_ptr(), 0);
                crate::log!("minimap: order {:#x} at {:.0},{:.0}{}", action, pos[0], pos[2], if ok { "" } else { " (refused)" });
            }
        }
    }
}

/// `cOverViewImp::SetCursorAiAction`'s table of the actions' cursors: an entry of 20
/// bytes per action (`& 0xfff`), its cursor sprite at `+0x0c` and its "cannot" cursor
/// at `+0x10` (the action's bit `0x4000`), the first entry's (`c_select`) for an action
/// with none. Loaded with the cursor art; the names before them are `cursor.spr`'s.
///
/// The cursor stock's minimap shows at a point of the square while the player has
/// something selected, by `cursor.spr` name: `RadarComponent::Simulate` (from the
/// panel's focus-step) asks `ActionMode::GetAction` (no target, the point) what a click
/// there means, makes a plain move without Alt a `GO_WARP` (0x2b, the "cannot" bit
/// kept), and names it to the overview (`SetCursorAiAction`, which picks the cursor
/// from its table into `iCursor+0x2c`). Returns the action too. None with nothing
/// selected or off the map, where stock shows the panel's own arrow
/// (`minimapDefaultCursor`, `standard_cursor`).
pub fn action_cursor(r: usize, u: f32, v: f32) -> Option<(u32, &'static str)> {
    let (table, get_at) = (at(sym::RADAR_ACTION_CURSORS), at(sym::RADAR_GET_ACTION));
    if r == 0 || table == 0 || get_at == 0 || selection_count() == 0 {
        return None;
    }
    let pos = to_world(r, u, v);
    if !inside(pos) {
        return None;
    }
    let mut action = 0u32;
    unsafe {
        let get: unsafe extern "C" fn(*mut u32, usize, *const f32, usize) -> *mut u32 = core::mem::transmute(get_at);
        get(&mut action, 0, pos.as_ptr(), 0);
    }
    if action & value::RADAR_ACTION_MASK as u32 == value::RADAR_ACTION_MOVE as u32 && !alt_held() {
        action = (action & value::RADAR_ACTION_CANNOT as u32) | value::RADAR_ACTION_GO_WARP as u32;
    }
    let i = action & value::RADAR_ACTION_MASK as u32;
    if i >= value::RADAR_ACTION_CURSORS_N as u32 {
        return None;
    }
    let entry = table + value::RADAR_ACTION_CURSOR_STRIDE as usize * i as usize;
    let mut sprite = field(entry, if action & value::RADAR_ACTION_CANNOT as u32 != 0 { field::RADAR_AC_CANNOT } else { field::RADAR_AC_CURSOR });
    if sprite == 0 {
        sprite = field(table, field::RADAR_AC_CURSOR);
    }
    let name = super::cursor::element_name(sprite)?;
    Some((action, name))
}

/// `SetCursorAiAction` reads the table as `action_cursor` does, and `GetAction` is
/// stock (checked once).
pub fn action_cursors_stock() -> bool {
    static mut STOCK: Option<bool> = None;
    unsafe {
        *(*core::ptr::addr_of_mut!(STOCK)).get_or_insert_with(|| {
            let ok = functions_stock() && eng().ready(hook::HUD_RADAR_CURSOR).is_ok();
            crate::log!("minimap: the action cursor over it {}", if ok { "is stock's choice" } else { "stays the engine's (SetCursorAiAction is not stock)" });
            ok
        })
    }
}

/// Every function above that is called, and every table it reads, is stock: the
/// minimap's hook is ready.
pub fn functions_stock() -> bool {
    eng().ready(hook::HUD_RADAR).is_ok()
}

// ---- the engine's textures and strings ----------------------------------------------

/// The texture behind an `ST3D_Texture`, to sample, and its size: its per-device
/// texture (at the texture's device list, indexed by the device index of Storm3D), whose
/// Direct3D 8 texture is the `IDirect3DTexture8`, which `gpu` makes a texture of
/// (Display8's mirror of it;
/// render3d/README.md reads the fog's the same way).
pub fn engine_texture(st3d_tex: usize) -> Option<(gpu::Texture, u32, u32)> {
    if st3d_tex == 0 {
        return None;
    }
    let idx = field(super::storm3d(), field::RADAR_S3D_DEVICE_INDEX);
    if idx > 8 {
        return None;
    }
    let t8 = field(field(st3d_tex, field::RADAR_TEX_DEVICES + idx * 4), field::RADAR_DEVTEX_D3D8);
    let c = crate::gfx::gpu().engine_texture(t8 as *mut core::ffi::c_void)?;
    let (w, h) = unsafe { (rd::<u32>(st3d_tex + field::RADAR_TEX_W), rd::<u32>(st3d_tex + field::RADAR_TEX_H)) };
    Some((c, w, h))
}

/// `Localized_Strings::Localize` on `g_string`: the game's text for a key.
pub fn localize(key: &core::ffi::CStr) -> &'static str {
    let (s, a) = (ptr_of(sym::RADAR_G_STRING), at(sym::RADAR_LOCALIZE));
    if s == 0 || a == 0 {
        return "";
    }
    let f: unsafe extern "thiscall" fn(usize, *const u8) -> *const u8 = unsafe { core::mem::transmute(a) };
    unsafe { super::hud::cstr_static(f(s, key.as_ptr() as *const u8)) }
}

/// A filter's name: its tooltip ("Filter Ships") without the word "Filter", else
/// English.
pub fn filter_name(station: bool) -> &'static str {
    let tip = localize(if station { c"GUI_MM_STATIONS_TOOLTIP" } else { c"GUI_MM_SHIPS_TOOLTIP" }).trim();
    let rest = tip.strip_prefix("Filter ").map(str::trim);
    match rest {
        Some(r) if !r.is_empty() && r.len() <= 14 => r,
        _ if station => "Stations",
        _ => "Ships",
    }
}

/// A filter button's tooltip, and what its state shows, in the game's words.
pub fn filter_text(station: bool, state: u32) -> (&'static str, &'static str) {
    let keys: [&core::ffi::CStr; 4] = if station {
        [c"GUI_MM_ALL_STATIONS_VISIBLE", c"GUI_MM_ALLIED_STATIONS_VISIBLE", c"GUI_MM_ENEMY_STATIONS_VISIBLE", c"GUI_MM_NEUTRAL_STATIONS_VISIBLE"]
    } else {
        [c"GUI_MM_ALL_SHIPS_VISIBLE", c"GUI_MM_ALLIED_SHIPS_VISIBLE", c"GUI_MM_ENEMY_SHIPS_VISIBLE", c"GUI_MM_NEUTRAL_SHIPS_VISIBLE"]
    };
    let tip = localize(if station { c"GUI_MM_STATIONS_TOOLTIP" } else { c"GUI_MM_SHIPS_TOOLTIP" });
    (tip, localize(keys[(state as usize).min(3)]))
}
