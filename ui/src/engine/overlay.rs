//! The selection overlay's engine side (HUD.md, "The selection overlay"): which units
//! `SelectionDisplay` marks in the 3D view, where they are on screen, and the gauges it
//! shows over them.
//!
//! `SelectionDisplay::Render` walks `GameObject::objectList`. It skips an object being
//! removed (`+0x113`), one not drawn this frame (`+0x25`) or that the player cannot
//! see (`CanUserSee`, vtable `+0x08`), and a station that is not under the pointer
//! (`CraftClass+0x20d`). The rest that are selected (`+0x10e`) or under the pointer
//! (`selectionDisplay+0x7c`, which its always-step sets from `DisplayInterface::MouseOn`
//! while the 3D view owns the mouse) get the group number (`+0x120`, 0..9); the
//! gauges show for the one under the pointer and for the units of its team
//! (`selectionDisplay+0x88`, the pointed unit's team `+0xec`, kept while the pointer
//! stays on that team; `+0x8c` fades them in, 0..1, drawn above 0.5).
//!
//! **Where.** `SelectionDisplay::ComputeContainerRect` (called here) projects the eight
//! corners of the object's bounding box (`Entity::GetBoundingBox`, vtable `+0x1c`, in
//! the object's transform) through `DisplayInterface::currentCamera` with
//! `ST3D_Camera::ProjectVirtualPoint` and returns the box around them, `{x, y, w, h}`,
//! in the virtual viewport (`DisplayInterface::s_pVirtualViewport`, `+0x8` / `+0xc` its
//! size), which is scaled to the back buffer per axis. `SelectionDisplay::Render`
//! points `currentCamera` at the camera in use (`ST3D_GraphicsEngine+0xfc`) first;
//! with its `Render` skipped we do the same around the call, and put back what was
//! there.
//!
//! **The gauges**, as `mDrawHullGauge`, `mDrawShieldGauge` and
//! `mDrawSpecialEnergyGauge` read them (for a craft, `GameObject+0x14` bit 3):
//! the hull `+0x158` (a fraction, shown while the maximum `+0x160` is above 0.1),
//! shields `Craft+0x1c8` of `+0x1cc`, special energy `+0x164` (a fraction, shown while
//! `+0x168` is above 0.1).
use super::{field, hook, rd};
use alloc::vec::Vec;

pub const SELECTION_DISPLAY: usize = 0x7645b0;
/// `g_cinematicView`: the unit view (an instance).
pub const CINEMATIC_VIEW: usize = 0x763758;
const SD_POINTED: usize = 0x7c;
const SD_TEAM: usize = 0x88;
const SD_FADE: usize = 0x8c;

const OBJECT_LIST: usize = 0x761084;
const OBJ_FLAGS: usize = 0x14;
const OBJ_DRAWN: usize = 0x25;
const OBJ_CLASS: usize = 0x40;
const OBJ_TEAM: usize = 0xec;
const OBJ_SELECTED: usize = 0x10e;
const OBJ_REMOVED: usize = 0x113;
const OBJ_GROUP: usize = 0x120;
const OBJ_HULL: usize = 0x158;
const OBJ_HULL_MAX: usize = 0x160;
const OBJ_SPECIAL: usize = 0x164;
const OBJ_SPECIAL_MAX: usize = 0x168;
const CRAFT_SHIELD: usize = 0x1c8;
const CRAFT_SHIELD_MAX: usize = 0x1cc;
const FLAG_CRAFT: u8 = 0x08;
const VT_CAN_USER_SEE: usize = 0x08;
const VT_CLASS_NAME: usize = 0x70;
const CLS_STATION: usize = 0x20d;

const RT_DYNAMIC_CAST: usize = 0x6731a0;
const TD_GAME_OBJECT_CLASS: usize = 0x6eee98;
const TD_CRAFT_CLASS: usize = 0x6eee08;

const CONTAINER_RECT: usize = 0x5081b0;
const CURRENT_CAMERA: usize = 0x765010;
const VIRTUAL_VIEWPORT: usize = 0x765004;
const S3D_CAMERA: usize = 0xfc;

/// A unit the overlay marks.
pub struct Unit {
    pub obj: usize,
    /// Its box on screen, in back-buffer pixels: x, y, w, h.
    pub rect: [f32; 4],
    pub selected: bool,
    pub pointed: bool,
    /// Stock would show its gauges now (pointed, or of the pointed unit's team).
    pub gauges: bool,
    pub team: i32,
    pub group: Option<u8>,
    pub craft: bool,
    pub station: bool,
    pub hull: Option<f32>,
    pub shields: Option<f32>,
    pub special: Option<f32>,
}

pub fn cast_craft_class(cls: usize) -> usize {
    if cls == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(usize, i32, usize, usize, i32) -> usize = unsafe { core::mem::transmute(RT_DYNAMIC_CAST) };
    unsafe { f(cls, 0, TD_GAME_OBJECT_CLASS, TD_CRAFT_CLASS, 0) }
}

/// The units to mark this frame, with their rectangles in a `w` x `h` back buffer.
pub fn units(w: f32, h: f32) -> Vec<Unit> {
    let mut out = Vec::new();
    let list = unsafe { rd::<usize>(OBJECT_LIST) };
    let cam = field(super::storm3d(), S3D_CAMERA);
    let vv = unsafe { rd::<usize>(VIRTUAL_VIEWPORT) };
    if list == 0 || cam == 0 || vv == 0 {
        return out;
    }
    let (vw, vh): (f32, f32) = unsafe { (rd(vv + 8), rd(vv + 12)) };
    if vw <= 0.0 || vh <= 0.0 {
        return out;
    }
    let (sx, sy) = (w / vw, h / vh);
    let sd = SELECTION_DISPLAY;
    let pointed = field(sd, SD_POINTED);
    let team_on = unsafe { rd::<i32>(sd + SD_TEAM) };
    let fade = unsafe { rd::<f32>(sd + SD_FADE) };
    unsafe {
        let saved = rd::<usize>(CURRENT_CAMERA);
        *(CURRENT_CAMERA as *mut usize) = cam;
        let head = field(list, 4);
        let mut n = if head != 0 { rd::<usize>(head) } else { 0 };
        let mut guard = 0;
        while head != 0 && n != head && n != 0 && guard < 16384 {
            let o = field(n, 8);
            n = rd::<usize>(n);
            guard += 1;
            if o == 0 || rd::<u8>(o + OBJ_REMOVED) != 0 || rd::<u8>(o + OBJ_DRAWN) == 0 {
                continue;
            }
            let selected = rd::<u8>(o + OBJ_SELECTED) != 0;
            let is_pointed = o == pointed;
            if !selected && !is_pointed {
                continue;
            }
            let see: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd::<usize>(o) + VT_CAN_USER_SEE));
            if !see(o) {
                continue;
            }
            let cc = cast_craft_class(field(o, OBJ_CLASS));
            let station = cc != 0 && rd::<u8>(cc + CLS_STATION) != 0;
            let mut r = [0f32; 4];
            let f: unsafe extern "thiscall" fn(usize, usize, *mut f32, f32) = core::mem::transmute(CONTAINER_RECT);
            f(sd, o, r.as_mut_ptr(), 1.0);
            if !r.iter().all(|v| v.is_finite()) || r[2] <= 0.0 || r[3] <= 0.0 {
                continue;
            }
            let team = rd::<i32>(o + OBJ_TEAM);
            let craft = rd::<u8>(o + OBJ_FLAGS) & FLAG_CRAFT != 0;
            let frac = |a: usize, b: usize| -> Option<f32> {
                let max = rd::<f32>(o + b);
                if max > 0.1 { Some(rd::<f32>(o + a).clamp(0.0, 1.0)) } else { None }
            };
            let group = rd::<i32>(o + OBJ_GROUP);
            out.push(Unit {
                obj: o,
                rect: [r[0] * sx, r[1] * sy, r[2] * sx, r[3] * sy],
                selected,
                pointed: is_pointed,
                gauges: fade > 0.5 && (is_pointed || (team_on != 0 && team_on == team)),
                team,
                group: if (0..10).contains(&group) { Some(group as u8) } else { None },
                craft,
                station,
                hull: if craft { frac(OBJ_HULL, OBJ_HULL_MAX) } else { None },
                shields: if craft {
                    let max = rd::<f32>(o + CRAFT_SHIELD_MAX);
                    if max > 0.1 { Some((rd::<f32>(o + CRAFT_SHIELD) / max).clamp(0.0, 1.0)) } else { None }
                } else {
                    None
                },
                special: if craft { frac(OBJ_SPECIAL, OBJ_SPECIAL_MAX) } else { None },
            });
        }
        *(CURRENT_CAMERA as *mut usize) = saved;
    }
    out
}

/// The unit's class as the game names it to the player (`GetClassDisplayName`,
/// vtable `+0x70`, e.g. "Assembler"). (`+0x48`, which `mDrawLabel` prints, is the
/// object's script label, e.g. "bconst1051": not for the player.)
pub fn name(obj: usize) -> &'static str {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(rd::<usize>(rd::<usize>(obj) + VT_CLASS_NAME));
        super::hud::cstr_static(f(obj))
    }
}

pub fn functions_stock() -> bool {
    super::starts(CONTAINER_RECT, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x74, 0x53, 0x8b, 0x5d, 0x08])
        && super::starts(RT_DYNAMIC_CAST, &[0xff, 0x25])
}

// ---- the systems' icons ---------------------------------------------------------------

/// `SystemOrder`: the five systems (`eSystem`) in the order stock lays their icons out.
const SYSTEM_ORDER: usize = 0x70dce0;
/// `selectionDisplay+0x54`: a `RepairIcon` per system (by `eSystem`); its `+0x34` the
/// system's icon sprite, `+0x40` the flash it blinks with while the system is repaired.
const SD_REPAIR_ICONS: usize = 0x54;
const RI_SPRITE: usize = 0x34;
const RI_FLASH: usize = 0x40;
/// `Craft+0x1e0`: its systems, 0x30 bytes each: `+0x4` the system's size (0: the craft
/// has none), `+0x18` its strength now (a double), `+0x0` a flag that hides it unless
/// the craft's `+0x1bf` is set.
const CRAFT_SYSTEMS: usize = 0x1e0;
const CRAFT_SHOW_ALL: usize = 0x1bf;

/// One of a craft's systems as `SelectionDisplay` marks it under its gauges.
pub struct SystemIcon {
    /// `eSystem` (0..4).
    pub system: usize,
    /// The system's icon (an `ST3D_Sprite`), or 0.
    pub sprite: usize,
    /// Its strength now, of its full strength (0..1).
    pub strength: f32,
    pub state: super::info::SystemState,
    /// The repair icon's flash (on half the time while it is repaired).
    pub flash: bool,
}

/// The systems stock's overlay marks under a craft's gauges, in its order
/// (`mDrawSystemInformation`, inlined in `SelectionDisplay::Render`: every system the
/// craft has, `RepairIcon::Render` drawing each one's icon green by its strength, red
/// while it is out with what is left to repair, and yellow on the flash while it is
/// repaired). Empty for anything that is not a craft.
pub fn systems(obj: usize) -> Vec<SystemIcon> {
    let mut out = Vec::new();
    if obj == 0 || unsafe { rd::<u8>(obj + OBJ_FLAGS) } & FLAG_CRAFT == 0 {
        return out;
    }
    let base = field(obj, CRAFT_SYSTEMS);
    if base == 0 {
        return out;
    }
    let order: [u32; 5] = unsafe { rd(SYSTEM_ORDER) };
    let show_all = unsafe { rd::<u8>(obj + CRAFT_SHOW_ALL) } != 0;
    for &sys in &order {
        let sys = sys as usize;
        if sys >= super::info::N_SYSTEMS {
            continue;
        }
        let s = base + sys * 0x30;
        let size = unsafe { rd::<i32>(s + 4) };
        if size == 0 || (unsafe { rd::<u8>(s) } != 0 && !show_all) {
            continue;
        }
        let Some(state) = super::info::system_state(obj, sys) else { continue };
        let now = unsafe { rd::<f64>(s + 0x18) } as f32;
        let strength = if size > 0 && now.is_finite() { (now / size as f32).clamp(0.0, 1.0) } else { 1.0 };
        let icon = field(SELECTION_DISPLAY + SD_REPAIR_ICONS + sys * 4, 0);
        out.push(SystemIcon {
            system: sys,
            sprite: field(icon, RI_SPRITE),
            strength,
            state,
            flash: icon != 0 && unsafe { rd::<u8>(icon + RI_FLASH) } != 0,
        });
    }
    out
}

// ---- the selection bubbles ----------------------------------------------------------

/// `RenderSelectionEffects` (cdecl, the camera): the 3D selection bubbles, an additive
/// ellipsoid about each selected unit and the one under the pointer, drawn from
/// `SelectionEffect::sm_EffectHash` by the scene each frame (render3d draws them in its
/// place from the same hash: render3d/README.md, `Step::Selection`). Its stock start,
/// `push ebp; mov ebp,esp; mov eax,[Storm3D]`.
const RENDER_SELECTION: usize = 0x599900;
const RENDER_SELECTION_STOCK: [u8; 8] = [0x55, 0x8b, 0xec, 0xa1, 0x08, 0xd5, 0x7a, 0x00];

/// Switches the engine's bubbles off (`[HUD] SelectionBubbles=0`): the function
/// returns at once (`ret` in its first byte). render3d, which draws them in its place,
/// draws none while the UI claims them (`CLAIM_SELECTION`, `crate::claims`); only on a
/// `d3d8.dll` without claims (no Display8 of contract 1.1) does it read this `ret`. The
/// effects themselves (the hash, their pulse) are left as they are: nothing the sim
/// reads is changed.
pub fn hide_bubbles() -> bool {
    hook::patch_bytes(RENDER_SELECTION, &RENDER_SELECTION_STOCK, &[0xc3])
}
