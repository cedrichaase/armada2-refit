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
//!   that texture (its Direct3D 9 texture, as render3d reads the fog's) over our square.
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
use super::{field, hook, rd};
use alloc::vec::Vec;

/// `mapRadar` (an instance): `+0x28` its `RadarComponent`, `+0x2c` / `+0x30` the ship
/// and station filter buttons (`StandardButton`s).
pub const MAP_RADAR: usize = 0x763810;
const MR_RADAR: usize = 0x28;
const MR_SHIP_BUTTON: usize = 0x2c;
const MR_STATION_BUTTON: usize = 0x30;
/// `StandardButton+0x2c`: its image sprite.
const BTN_SPRITE: usize = 0x2c;

/// `RadarComponent+0x3c`: the sprite under the map (its frame's backing).
const RC_BACKGROUND: usize = 0x3c;
/// `RadarComponent+0x4c`: the `std::list<MessageMark>` (its head node).
const RC_MARKS: usize = 0x4c;

const MAP_SPRITE: usize = 0x76387c;
const WARP_SPRITE: usize = 0x763880;
const FOG_SPRITE: usize = 0x763874;
/// `ST3D_Sprite+0x58`: its texture; `ST3D_Texture+0x1c` / `+0x20`: its size.
const SPRITE_TEXTURE: usize = 0x58;
const TEX_W: usize = 0x1c;
const TEX_H: usize = 0x20;

const SHIP_FILTER: usize = 0x76386c;
const STATION_FILTER: usize = 0x763870;

const TO_SQUARE: usize = 0x4e8670;
const TO_WORLD: usize = 0x4e86e0;
const NEXT_SHIP_FILTER: usize = 0x4eb7c0;
const NEXT_STATION_FILTER: usize = 0x4eb930;

/// `g_terrain`: the map's box, x0 y0 z0 x1 y1 z1.
const TERRAIN: usize = 0x739590;
/// `Scanner::m_fog`, `Scanner::m_shroud`: `ScanGrid *`.
const SCAN_FOG: usize = 0x738a54;
const SCAN_SHROUD: usize = 0x738a58;
const GRID_CELLS: usize = 0x04;
const GRID_W: usize = 0x0c;
const GRID_H: usize = 0x10;
const TEAMS_VISIBLE: usize = 0x4d0070;
/// The call of it in `MapRadar::AlwaysGameSimulate`.
const SITE_TEAMS_VISIBLE: usize = 0x4eb3b2;

/// `UserProfile *`: `+0x24` the mouse buttons swapped, `+0x84` the map editor.
const USER_PROFILE: usize = 0x76b5ac;
/// `g_transport`, `Transport::getGameSetup`, `GameSetup::isFog` / `isShroud`.
const TRANSPORT: usize = 0x76b8d4;
const GET_GAME_SETUP: usize = 0x557940;
const IS_FOG: usize = 0x5462f0;
const IS_SHROUD: usize = 0x546300;
/// `View_Record`: 0 the RTS view, 1 tactical, 2 cinematic.
const VIEW_RECORD: usize = 0x76b610;

/// `GameObject::objectList` (`std::list<GameObject *> *`), `Ghost::ghostList`
/// (`std::list<Spirit *>`, its head node at `+4`).
const OBJECT_LIST: usize = 0x761084;
const GHOST_HEAD: usize = 0x736c94;
const OBJ_REMOVED: usize = 0x113;
const OBJ_CLASS: usize = 0x40;
const OBJ_POS: usize = 0xac;
const OBJ_TEAM: usize = 0xec;
/// `GameObject+0x10e`: selected by the player (`SelectionDisplay::Render` reads it).
const OBJ_SELECTED: usize = 0x10e;
const VT_CAN_USER_SEE: usize = 0x08;
const CLS_OFF_RADAR: usize = 0x18c;
const CLS_RADAR_SPRITE: usize = 0x190;
const CLS_HAS_MAP_SIZE: usize = 0x184;
const CLS_STATION: usize = 0x20d;
const VT_MAP_SIZE: usize = 0x24;
const VT_IS_CRAFT_CLASS: usize = 0x28;
/// `Spirit`: `+0x4` the class, `+0xc` the team, vtable `+0x10` the position.
const SPIRIT_CLASS: usize = 0x04;
const SPIRIT_TEAM: usize = 0x0c;
const VT_SPIRIT_POS: usize = 0x10;

/// `__RTDynamicCast` and the type descriptors `RenderMapObject` and `ComputeDot` cast
/// with.
const RT_DYNAMIC_CAST: usize = 0x6731a0;
const TD_GAME_OBJECT_CLASS: usize = 0x6eee98;
const TD_CRAFT_CLASS: usize = 0x6eee08;
const TD_PLANET_CLASS: usize = 0x700c08;

const TEAM_GET: usize = 0x496340;
const TEAM_COLOUR: usize = 0x14;
const IS_ALLY: usize = 0x4971f0;
const IS_NEUTRAL: usize = 0x497210;
const IS_ENEMY: usize = 0x497230;

/// `ActionMode::GetAction(target, position, bool *)`, static: the order a click there
/// gives.
const GET_ACTION: usize = 0x44ab70;
const OVERVIEW_PTR: usize = 0x768e3c;
const VT_SELECT_NUM: usize = 0x94;
const VT_WHERE_CURSOR: usize = 0xbc;
const VT_SET_CURSOR_ACTION: usize = 0xc8;
const PALETTE_PTR: usize = 0x763c00;
const VT_BROADCAST: usize = 0xa4;
/// `g_pCommandAlt`: the engine's Alt state (an int array, [0]).
const COMMAND_ALT: usize = 0x761344;
/// `gCameraManager`: its first field the camera in use, whose vtable `+0x54` moves it
/// to a point.
const CAMERA_MANAGER: usize = 0x763370;
const VT_CAMERA_MOVE: usize = 0x54;
const SNAP_TO_NEAREST: usize = 0x4de460;
const TACTICAL_SNAP: usize = 0x6fcb48;

/// `ST3D_GraphicsEngine+0xfc`: the camera in use (the main view's outside
/// `CinematicView::Render`, which puts it back).
const S3D_CAMERA: usize = 0xfc;
/// `ST3D_Camera`: `+0x1a0` near, `+0x1a4` far, `+0x1b0` its viewport x y w h, `+0xe4`
/// its position.
const CAM_NEAR: usize = 0x1a0;
const CAM_FAR: usize = 0x1a4;
const CAM_VIEWPORT: usize = 0x1b0;
const CAM_POS: usize = 0xe4;
const SCREEN_RAY: usize = 0x619140;

pub fn radar() -> usize {
    field(MAP_RADAR, MR_RADAR)
}

/// The world's (x, z) to the radar's unit square (u right, v down).
pub fn to_square(r: usize, p: [f32; 3]) -> (f32, f32) {
    let mut out = [0f32; 2];
    let f: unsafe extern "thiscall" fn(usize, *mut f32, *const f32) -> *mut f32 = unsafe { core::mem::transmute(TO_SQUARE) };
    unsafe { f(r, out.as_mut_ptr(), p.as_ptr()) };
    (out[0], out[1])
}

/// A point of the unit square in the world, at the map's middle height.
pub fn to_world(r: usize, u: f32, v: f32) -> [f32; 3] {
    let mut out = [0f32; 3];
    let i = [u, v];
    let f: unsafe extern "thiscall" fn(usize, *mut f32, *const f32) -> *mut f32 = unsafe { core::mem::transmute(TO_WORLD) };
    unsafe { f(r, out.as_mut_ptr(), i.as_ptr()) };
    out
}

/// The map's box: x0 y0 z0 x1 y1 z1.
pub fn terrain() -> [f32; 6] {
    unsafe { rd(TERRAIN) }
}

fn inside(p: [f32; 3]) -> bool {
    let t = terrain();
    p[0] >= t[0] && p[0] <= t[3] && p[2] >= t[2] && p[2] <= t[5]
}

/// The map's picture: the Direct3D 8 texture `CacheMap` draws into, and the warp
/// suppression layer over it.
pub fn map_texture() -> usize {
    field(rd_ptr(MAP_SPRITE), SPRITE_TEXTURE)
}
pub fn warp_texture() -> usize {
    field(rd_ptr(WARP_SPRITE), SPRITE_TEXTURE)
}
pub fn background_sprite(r: usize) -> usize {
    field(r, RC_BACKGROUND)
}

fn rd_ptr(a: usize) -> usize {
    unsafe { rd::<usize>(a) }
}

/// The fog layer's texture size, in which the scan grid is centred.
pub fn fog_texture_size() -> (u32, u32) {
    let t = field(rd_ptr(FOG_SPRITE), SPRITE_TEXTURE);
    if t == 0 {
        return (0, 0);
    }
    unsafe { (rd::<u32>(t + TEX_W), rd::<u32>(t + TEX_H)) }
}

/// A scan grid: its size and cells (one team mask each, row by row).
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: &'static [u32],
}

fn grid(at: usize) -> Option<Grid> {
    let g = rd_ptr(at);
    if g == 0 {
        return None;
    }
    unsafe {
        let (w, h) = (rd::<i32>(g + GRID_W), rd::<i32>(g + GRID_H));
        let p = rd::<usize>(g + GRID_CELLS);
        if p == 0 || !(1..=4096).contains(&w) || !(1..=4096).contains(&h) {
            return None;
        }
        Some(Grid { w: w as usize, h: h as usize, cells: core::slice::from_raw_parts(p as *const u32, (w * h) as usize) })
    }
}

pub fn fog_grid() -> Option<Grid> {
    grid(SCAN_FOG)
}
pub fn shroud_grid() -> Option<Grid> {
    grid(SCAN_SHROUD)
}

pub fn teams_visible() -> u32 {
    let f: unsafe extern "C" fn() -> u32 = unsafe { core::mem::transmute(TEAMS_VISIBLE) };
    unsafe { f() }
}

fn game_setup() -> usize {
    let t = rd_ptr(TRANSPORT);
    if t == 0 {
        return 0;
    }
    let f: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(GET_GAME_SETUP) };
    unsafe { f(t) }
}

fn editor() -> bool {
    let p = rd_ptr(USER_PROFILE);
    p != 0 && unsafe { rd::<u8>(p + 0x84) } != 0
}

/// Whether the game has fog of war and shroud, as `RadarComponent::Render` asks.
pub fn fog_on() -> (bool, bool) {
    let gs = game_setup();
    if gs == 0 || editor() {
        return (false, false);
    }
    unsafe {
        let fog: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(IS_FOG);
        let shroud: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(IS_SHROUD);
        (fog(gs), shroud(gs))
    }
}

pub fn view() -> u32 {
    unsafe { rd(VIEW_RECORD) }
}

/// The ship and station filters: 0 all, 1 allies, 2 enemies, 3 neutral.
pub fn filters() -> (u32, u32) {
    unsafe { (rd(SHIP_FILTER), rd(STATION_FILTER)) }
}

/// The filter buttons' art.
pub fn filter_sprites() -> (usize, usize) {
    (field(field(MAP_RADAR, MR_SHIP_BUTTON), BTN_SPRITE), field(field(MAP_RADAR, MR_STATION_BUTTON), BTN_SPRITE))
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

fn cast(p: usize, to: usize) -> usize {
    if p == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(usize, i32, usize, usize, i32) -> usize = unsafe { core::mem::transmute(RT_DYNAMIC_CAST) };
    unsafe { f(p, 0, TD_GAME_OBJECT_CLASS, to, 0) }
}

fn team_object(team: i32) -> usize {
    let f: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(TEAM_GET) };
    unsafe { f(team) }
}

/// The team's colour (0..1), as the radar's dots and the 3D view's team shells use it.
pub fn team_colour(team: i32) -> [f32; 3] {
    let t = team_object(team);
    if t == 0 {
        return [0.7, 0.7, 0.7];
    }
    unsafe { rd(t + TEAM_COLOUR) }
}

/// How a team stands to the player: 0 the player's own, 1 an ally, 2 an enemy, 3
/// neutral (the user team's `IsUnconfirmedAlly` / `Enemy`, as the radar's filters ask).
pub fn relation(team: i32) -> u8 {
    if team == super::selection::user_team() {
        return 0;
    }
    let user = team_object(super::selection::user_team());
    if user == 0 {
        return 3;
    }
    unsafe {
        let ally: unsafe extern "thiscall" fn(usize, i32) -> bool = core::mem::transmute(IS_ALLY);
        let enemy: unsafe extern "thiscall" fn(usize, i32) -> bool = core::mem::transmute(IS_ENEMY);
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
    let is_craft: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(vt + VT_IS_CRAFT_CLASS)) };
    if !unsafe { is_craft(cls) } || cast(cls, TD_PLANET_CLASS) != 0 {
        return true;
    }
    let station = unsafe { rd::<u8>(cls + CLS_STATION) } != 0;
    let f = if station { filters.1 } else { filters.0 };
    let test = match f {
        1 => IS_ALLY,
        2 => IS_ENEMY,
        3 => IS_NEUTRAL,
        _ => return true,
    };
    if user == 0 {
        return true;
    }
    let g: unsafe extern "thiscall" fn(usize, i32) -> bool = unsafe { core::mem::transmute(test) };
    unsafe { g(user, team) }
}

fn blip(r: usize, cls: usize, pos: [f32; 3], team: i32, ghost: bool, selected: bool, filters: (u32, u32), user: usize) -> Option<Blip> {
    if cls == 0 || unsafe { rd::<u32>(cls + CLS_OFF_RADAR) } != 0 || !inside(pos) {
        return None;
    }
    if !passes_filter(cls, team, filters, user) {
        return None;
    }
    let craft = cast(cls, TD_CRAFT_CLASS);
    let station = craft != 0 && unsafe { rd::<u8>(craft + CLS_STATION) } != 0;
    let has_size = unsafe { rd::<u8>(cls + CLS_HAS_MAP_SIZE) } != 0;
    let size = if has_size {
        let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(rd::<usize>(rd_ptr(cls) + VT_MAP_SIZE)) };
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
        sprite: field(cls, CLS_RADAR_SPRITE),
        ghost,
        selected,
        colour: team_colour(team),
    })
}

/// Everything the radar shows, ghosts first (stock draws them first).
pub fn blips(r: usize) -> Vec<Blip> {
    let mut out = Vec::new();
    let list = rd_ptr(OBJECT_LIST);
    if r == 0 || list == 0 {
        return out;
    }
    let filters = filters();
    let user = team_object(super::selection::user_team());
    unsafe {
        let head = rd::<usize>(GHOST_HEAD);
        if head != 0 {
            let mut n = rd::<usize>(head);
            let mut guard = 0;
            while n != head && n != 0 && guard < 8192 {
                let s = field(n, 8);
                if s != 0 {
                    let pf: unsafe extern "thiscall" fn(usize) -> *const [f32; 3] = core::mem::transmute(rd::<usize>(rd_ptr(s) + VT_SPIRIT_POS));
                    let p = pf(s);
                    if !p.is_null() {
                        if let Some(b) = blip(r, field(s, SPIRIT_CLASS), *p, rd::<i32>(s + SPIRIT_TEAM), true, false, filters, user) {
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
            if o == 0 || rd::<u8>(o + OBJ_REMOVED) != 0 {
                continue;
            }
            let see: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd_ptr(o) + VT_CAN_USER_SEE));
            if !see(o) {
                continue;
            }
            let pos: [f32; 3] = rd(o + OBJ_POS);
            if let Some(b) = blip(r, field(o, OBJ_CLASS), pos, rd::<i32>(o + OBJ_TEAM), false, rd::<u8>(o + OBJ_SELECTED) != 0, filters, user) {
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
    let head = field(r, RC_MARKS);
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
            out.push(Mark { u, v, radius: rd::<i32>(m + 0x1c) as f32 + 1.0, colour: rd(m + 0x20) });
            n = rd::<usize>(n);
            guard += 1;
        }
    }
    out
}

pub fn camera() -> usize {
    field(super::storm3d(), S3D_CAMERA)
}

/// What the main camera sees of the map, in the unit square: its viewport's corners
/// (top-left, top-right, bottom-right, bottom-left) on the map's middle height. None
/// outside the RTS view.
pub fn view_outline(r: usize) -> Option<[(f32, f32); 4]> {
    let cam = camera();
    if cam == 0 || view() != 0 {
        return None;
    }
    let t = terrain();
    let y = (t[1] + t[4]) * 0.5;
    unsafe {
        let vp: [f32; 4] = rd(cam + CAM_VIEWPORT);
        let (near, far) = (rd::<f32>(cam + CAM_NEAR), rd::<f32>(cam + CAM_FAR));
        let pos: [f32; 3] = rd(cam + CAM_POS);
        if vp[2] <= 0.0 || vp[3] <= 0.0 {
            return None;
        }
        let ray: unsafe extern "thiscall" fn(usize, *mut f32, *const f32, f32) -> *mut f32 = core::mem::transmute(SCREEN_RAY);
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
static mut FILTER_PRESS: (bool, bool) = (false, false);

/// The command button is the right one, unless the player swapped the buttons.
pub fn buttons_swapped() -> bool {
    let p = rd_ptr(USER_PROFILE);
    p != 0 && unsafe { rd::<u8>(p + 0x24) } != 0
}

pub fn selection_count() -> i32 {
    let o = rd_ptr(OVERVIEW_PTR);
    if o == 0 {
        return 0;
    }
    let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(rd::<usize>(rd_ptr(o) + VT_SELECT_NUM)) };
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

pub fn queue_filter(station: bool) {
    unsafe {
        if station {
            FILTER_PRESS.1 = true
        } else {
            FILTER_PRESS.0 = true
        }
    }
}

/// Patches the always-step's call; false (nothing patched) when it is not stock.
pub fn install() -> bool {
    if unsafe { HOOKED } {
        return true;
    }
    if !functions_stock() {
        return false;
    }
    let ok = hook::patch_call(SITE_TEAMS_VISIBLE, TEAMS_VISIBLE, teams_visible_step as *const () as usize);
    unsafe { HOOKED = ok };
    if !ok {
        crate::log!("minimap: MapRadar::AlwaysGameSimulate's call at {:#x} is not stock: clicks on the minimap do nothing", SITE_TEAMS_VISIBLE);
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
        if s {
            let f: unsafe extern "C" fn(usize) = core::mem::transmute(NEXT_SHIP_FILTER);
            f(field(MAP_RADAR, MR_SHIP_BUTTON));
        }
        if st {
            let f: unsafe extern "C" fn(usize) = core::mem::transmute(NEXT_STATION_FILTER);
            f(field(MAP_RADAR, MR_STATION_BUTTON));
        }
    }
    teams
}

fn alt_held() -> bool {
    let p = rd_ptr(COMMAND_ALT);
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
            let cam = rd_ptr(CAMERA_MANAGER);
            if cam == 0 {
                return;
            }
            unsafe {
                let mv: unsafe extern "thiscall" fn(usize, *mut f32) = core::mem::transmute(rd::<usize>(rd_ptr(cam) + VT_CAMERA_MOVE));
                mv(cam, pos.as_mut_ptr());
                if snap && !alt_held() {
                    let s: unsafe extern "thiscall" fn(usize, *const f32, f32) = core::mem::transmute(SNAP_TO_NEAREST);
                    s(CAMERA_MANAGER, pos.as_ptr(), rd::<f32>(TACTICAL_SNAP));
                }
            }
        }
        Pending::Order(u, v) => {
            let mut pos = to_world(r, u, v);
            let o = rd_ptr(OVERVIEW_PTR);
            let pal = rd_ptr(PALETTE_PTR);
            if !inside(pos) || o == 0 || pal == 0 || selection_count() == 0 {
                return;
            }
            unsafe {
                let get: unsafe extern "C" fn(*mut u32, usize, *const f32, usize) -> *mut u32 = core::mem::transmute(GET_ACTION);
                let mut action = 0u32;
                get(&mut action, 0, pos.as_ptr(), 0);
                if action & 0xfff == 4 && !alt_held() {
                    action = (action & 0x4000) | 0x2b;
                    let mut at = [0f32; 3];
                    let wc: unsafe extern "thiscall" fn(usize, *mut f32) -> *mut f32 = core::mem::transmute(rd::<usize>(rd_ptr(o) + VT_WHERE_CURSOR));
                    wc(o, at.as_mut_ptr());
                    pos[1] = at[1];
                }
                let set: unsafe extern "thiscall" fn(usize, u32) = core::mem::transmute(rd::<usize>(rd_ptr(o) + VT_SET_CURSOR_ACTION));
                set(o, action);
                let bc: unsafe extern "thiscall" fn(usize, u32, *const f32, usize) -> bool = core::mem::transmute(rd::<usize>(rd_ptr(pal) + VT_BROADCAST));
                let ok = bc(pal, action, pos.as_ptr(), 0);
                crate::log!("minimap: order {:#x} at {:.0},{:.0}{}", action, pos[0], pos[2], if ok { "" } else { " (refused)" });
            }
        }
    }
}

/// Every function above that is called, checked by its first bytes.
pub fn functions_stock() -> bool {
    super::starts(TO_SQUARE, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x0c, 0xd9, 0x40, 0x08])
        && super::starts(TO_WORLD, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x0c, 0xd9, 0x05])
        && super::starts(TEAMS_VISIBLE, &[0x8b, 0x0d, 0xd4, 0xb8, 0x76, 0x00])
        && super::starts(GET_GAME_SETUP, &[0x8b, 0x41, 0x30, 0xc3])
        && super::starts(IS_FOG, &[0x8b, 0x41, 0x10, 0x8a, 0x80, 0xaf, 0x00, 0x00, 0x00, 0xc3])
        && super::starts(IS_SHROUD, &[0x8b, 0x41, 0x10, 0x8a, 0x80, 0xb0, 0x00, 0x00, 0x00, 0xc3])
        && super::starts(RT_DYNAMIC_CAST, &[0xff, 0x25])
        && super::starts(IS_ALLY, &[0x55, 0x8b, 0xec, 0x8b, 0x55, 0x08, 0x56, 0x33, 0xc0])
        && super::starts(IS_NEUTRAL, &[0x55, 0x8b, 0xec, 0x8b, 0x55, 0x08, 0x56, 0x33, 0xc0])
        && super::starts(IS_ENEMY, &[0x55, 0x8b, 0xec, 0x8b, 0x55, 0x08, 0x56, 0x33, 0xc0])
        && super::starts(TEAM_GET, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08])
        && super::starts(GET_ACTION, &[0x55, 0x8b, 0xec, 0x6a, 0xff, 0x68, 0xa8, 0xa8, 0x69, 0x00])
        && super::starts(SNAP_TO_NEAREST, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x3c])
        && super::starts(NEXT_SHIP_FILTER, &[0x55, 0x8b, 0xec, 0x6a, 0xff, 0x68, 0x93, 0xee, 0x69, 0x00])
        && super::starts(NEXT_STATION_FILTER, &[0x55, 0x8b, 0xec, 0x6a, 0xff, 0x68, 0xa6, 0xee, 0x69, 0x00])
        && super::starts(SCREEN_RAY, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x34, 0xd9, 0x81, 0xa8, 0x01])
        && unsafe { rd::<usize>(0x6b5ff4 + VT_SET_CURSOR_ACTION) } == 0x526b60
        && unsafe { rd::<usize>(0x6b5ff4 + VT_WHERE_CURSOR) } == 0x525150
        && unsafe { rd::<usize>(0x6b4f84 + VT_BROADCAST) } == 0x4fdca0
        && super::starts(LOCALIZE, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08, 0x8b, 0xc1])
}

// ---- the engine's textures and strings ----------------------------------------------

/// The Direct3D 9 texture behind an `ST3D_Texture`, and its size: its per-device
/// texture (`+0x40 + device index * 4`, the index at `Storm3D+0xc0`), whose `+4` is the
/// `IDirect3DTexture8`, which d3d8to9 answers for its `IDirect3DTexture9`
/// (render3d/README.md reads the fog's the same way).
pub fn d3d9_texture(st3d_tex: usize) -> Option<(crate::d3d::Com, u32, u32)> {
    if st3d_tex == 0 {
        return None;
    }
    let idx = field(super::storm3d(), 0xc0);
    if idx > 8 {
        return None;
    }
    let t8 = field(field(st3d_tex, 0x40 + idx * 4), 4);
    let c = crate::d3d::query(t8 as *mut core::ffi::c_void, &crate::d3d::IID_TEXTURE9)?;
    let (w, h) = unsafe { (rd::<u32>(st3d_tex + TEX_W), rd::<u32>(st3d_tex + TEX_H)) };
    Some((c, w, h))
}

/// `Localized_Strings::Localize` on `g_string`: the game's text for a key.
const LOCALIZE: usize = 0x481c90;
const G_STRING: usize = 0x7379fc;

pub fn localize(key: &core::ffi::CStr) -> &'static str {
    let s = rd_ptr(G_STRING);
    if s == 0 {
        return "";
    }
    let f: unsafe extern "thiscall" fn(usize, *const u8) -> *const u8 = unsafe { core::mem::transmute(LOCALIZE) };
    unsafe { super::hud::cstr_static(f(s, key.as_ptr() as *const u8)) }
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
