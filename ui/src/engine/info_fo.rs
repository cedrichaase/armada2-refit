//! The info panel on Fleet Operations: Roots (HUD.md, "The info panel on Fleet Operations:
//! Roots"; PLAN-FLEETOPS.md, FO-6, decisions 440 to 459): FleetOpsHook's own `ShipDisplay`,
//! drawn by ours in Roots' layout.
//!
//! FleetOpsHook does part of the panel's gameplay work inside its drawing: the weapon icons
//! get their rectangles (from each weapon's `weapon<N>IconPos` in the ODF) in their `Render`,
//! the subsystem icons for the sensors and the weapons set the flag that draws a range ring
//! from theirs, and the buffs are worked out there. So the panel's `Render` keeps running,
//! every frame, as the engine calls it, but nothing it draws reaches the screen: while it runs
//! seven drawing slots of the engine's device (`ST3D_DeviceDirectX8`'s vtable, each checked
//! to hold its function: the triangle fans the sprites and the text are made of, the
//! rectangles, the line strips, and the clipped sprites) do nothing
//! (`render_unseen`). Every other call the panel makes (the state changes, the textures) runs as
//! it would. Ours then draws what the panel's own children say, in Roots' places.
//!
//! The pointer stays the panel's (`hud::set_info_area`) wherever ours sits where Roots' does, so
//! the engine's own `StandardComponent::Simulate` sets each child's mouse-over flag from the
//! child's own rectangle: the range rings, the tooltips, the clicks (the camera, a queue slot's
//! cancel, a group icon's selection) are the engine's and FleetOpsHook's. Nothing of
//! FleetOpsHook's memory is written: its members are read, and a stock child's rectangle (an
//! engine `StandardComponent` field the layout file filled) is moved only to follow ours, and put
//! back.
use super::info::{cast, child, f32_at, latin1, text_of, Text};
use super::{field, rd, readable};
use crate::syms_info::{field, hook, sym, value, TABLE};
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// Whether this build's info panel is FleetOpsHook's and its reads and the draw gate can be had.
pub fn available() -> bool {
    value::INFO_FO_PANEL() != 0 && eng().ready(hook::HUD_INFO_FO).is_ok() && eng().ready(hook::HUD_INFO).is_ok()
}

// ---- the draw gate ----------------------------------------------------------------------

static mut GATE: bool = false;
static mut ORIG: [usize; 7] = [0; 7];
static mut INSTALLED: bool = false;

macro_rules! gate {
    ($name:ident, $i:expr, $($a:ident),*) => {
        unsafe extern "thiscall" fn $name(this: usize, $($a: u32),*) {
            unsafe {
                if GATE {
                    return;
                }
                let f: unsafe extern "thiscall" fn(usize, $(gate!(@t $a)),*) = core::mem::transmute(ORIG[$i]);
                f(this, $($a),*)
            }
        }
    };
    (@t $a:ident) => { u32 };
}

// The slots' argument lists (ST3D_DeviceDirectX8's, from its symbols' signatures; each callee
// pops what it is given): DrawClippedRectangle (rect, z, colour, material), DrawRectangle
// (rect, colour), WriteText (text, x, y, colour, colour), the two DrawTriangleFan (vertices,
// count), DrawLineStrip (points, colour, count), RenderClippedSprite2D (sprite, position, size,
// a 2D vector by value, clip). Not RenderParticle: render3d's particles module holds that
// slot, and the panel draws no particles.
gate!(g_clipped_rect, 0, a, b, c, d);
gate!(g_rect, 1, a, b);
gate!(g_text, 2, a, b, c, d, e);
gate!(g_fan_plain, 3, a, b);
gate!(g_fan, 4, a, b);
gate!(g_line_strip, 5, a, b, c);
gate!(g_sprite_2d, 6, a, b, c, d, e, f, g);

/// Wraps the device's drawing slots (once). False: the panel cannot be drawn unseen, and ours
/// stands down (Roots' own shows).
#[allow(static_mut_refs)]
pub fn install() -> bool {
    unsafe {
        if INSTALLED {
            return true;
        }
        if !available() {
            return false;
        }
        // The status line under the pointer is drawn inside the panel's Render, through the
        // hover hook's call (which lifts the gate around it): the hook goes in with the gate.
        if !super::hover::install_only() {
            crate::log!("info: the hover strip's call is not stock: Roots' status line under the pointer is not drawn while ours is on");
        }
        let slots = [
            (sym::INFO_SLOT_DRAW_CLIPPED_RECT, g_clipped_rect as *const () as usize),
            (sym::INFO_SLOT_DRAW_RECT, g_rect as *const () as usize),
            (sym::INFO_SLOT_WRITE_TEXT, g_text as *const () as usize),
            (sym::INFO_SLOT_FAN_PLAIN, g_fan_plain as *const () as usize),
            (sym::INFO_SLOT_FAN, g_fan as *const () as usize),
            (sym::INFO_SLOT_LINE_STRIP, g_line_strip as *const () as usize),
            (sym::INFO_SLOT_SPRITE_2D, g_sprite_2d as *const () as usize),
        ];
        // Every slot's address first: nothing is written unless all can be.
        if slots.iter().any(|&(s, _)| eng().addr(s) == 0) {
            return false;
        }
        for (i, &(s, to)) in slots.iter().enumerate() {
            match super::hook::patch_slot(eng().addr(s), 0, to) {
                Some(old) => ORIG[i] = old,
                None => {
                    // Put back what was written.
                    for (j, &(s2, _)) in slots.iter().enumerate().take(i) {
                        super::hook::patch_slot(eng().addr(s2), 0, ORIG[j]);
                    }
                    return false;
                }
            }
        }
        INSTALLED = true;
        true
    }
}

/// Runs the panel's `Render` (`render`, thiscall on `c`) with the device's drawing dropped.
pub fn render_unseen(c: usize, render: unsafe extern "thiscall" fn(usize)) {
    unsafe {
        GATE = true;
        render(c);
        GATE = false;
    }
}

/// Runs `f` with the device's drawing let through, inside the panel's unseen `Render`: the
/// strip about the craft under the pointer (`SDInfoBar`, Roots' status line at the top of the
/// screen) is drawn by that `Render` and stays Roots' (or `hover`'s, `[HUD] Hover=1`).
pub fn ungated(f: impl FnOnce()) {
    unsafe {
        let g = GATE;
        GATE = false;
        f();
        GATE = g;
    }
}

// ---- the children's state ---------------------------------------------------------------

/// A component's rectangle, local to its owner (canvas units, inclusive), or None.
pub fn comp_rect(c: usize) -> Option<[i32; 4]> {
    if c == 0 || !readable(c, 0x20) {
        return None;
    }
    let r: [i32; 4] = unsafe { rd(c + field::INFO_COMP_RECT()) };
    (r[2] >= r[0] && r[3] >= r[1]).then_some(r)
}

/// Whether the engine has the pointer over the component this frame (its mouse-over flag).
pub fn comp_hover(c: usize) -> bool {
    c != 0 && unsafe { rd::<u8>(c + field::INFO_COMP_HOVER()) } != 0
}

/// One of FleetOpsHook's icons on the ship image (a weapon, or a buff): its rectangle, local to
/// the panel (canvas units), its sprite, and the component (for its tooltip and hover).
#[derive(Clone, Copy, Debug)]
pub struct Icon {
    pub rect: [i32; 4],
    pub sprite: usize,
    pub comp: usize,
}

/// Whether the craft's class gives weapon `i` an icon (FleetOpsHook's record of the class's
/// `weapon<N>IconPos`, read from the ODF): the panel's icons past the weapons the class has keep
/// the place and the sprite the last class gave them, so only these are drawn.
fn weapon_has_icon(obj: usize, i: usize) -> bool {
    let craft = cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CRAFT);
    let rec = field(craft, field::INFO_FO_CRAFT_RECORD());
    if rec == 0 || !readable(rec, 8) {
        return false;
    }
    let class = field(rec, field::INFO_FO_RECORD_CLASS());
    if class == 0 || !readable(class + field::INFO_FO_CLASS_ICON_POS(), 4) {
        return false;
    }
    let arr = unsafe { rd::<usize>(class + field::INFO_FO_CLASS_ICON_POS()) };
    if arr < 4 || !readable(arr - 4, 4) {
        return false;
    }
    let n = unsafe { rd::<u32>(arr - 4) } as usize;
    let stride = value::INFO_FO_ICON_POS_BYTES() as usize;
    let e = arr + i * stride;
    i < n && n <= 64 && readable(e, stride) && unsafe { rd::<u32>(e + field::INFO_FO_ICON_POS_PRESENT()) } != 0
}

fn icons_at(off: usize, n: usize, obj: usize, weapons: bool) -> Vec<Icon> {
    let mut out = Vec::new();
    let systems = child(field::INFO_FO_SYSTEMS());
    let Some(area) = comp_rect(systems) else { return out };
    for i in 0..n {
        let c = child(off + 4 * i);
        if c == 0 || !readable(c, 0x50) {
            continue;
        }
        let sprite = field(c, field::INFO_FO_ICON_SPRITE());
        let Some(r) = comp_rect(c) else { continue };
        // Until its first Render an icon spans the whole image (no place of its own yet).
        if sprite == 0 || r == area || r[2] - r[0] > (area[2] - area[0]) / 2 {
            continue;
        }
        // A weapon icon is drawn for the weapons the class places.
        if weapons && !weapon_has_icon(obj, i) {
            continue;
        }
        // A buff icon, while the object has one of the effects stock knows (slowed by a gravity
        // mine, a spy aboard): FleetOpsHook's weapon buffs are not told apart from a stale icon.
        if !weapons {
            let (slowed, spy) = super::info::effects(obj);
            if !slowed && !spy {
                continue;
            }
        }
        out.push(Icon { rect: r, sprite, comp: c });
    }
    out
}

/// The weapon icons FleetOpsHook placed on the ship image this frame.
pub fn weapon_icons(obj: usize) -> Vec<Icon> {
    icons_at(field::INFO_FO_WEAPON_ICONS(), value::INFO_FO_N_WEAPON_ICONS() as usize, obj, true)
}

/// The buff icons (a spy aboard, a gravity mine, a weapon's buff).
pub fn buff_icons(obj: usize) -> Vec<Icon> {
    icons_at(field::INFO_FO_BUFF_ICONS(), value::INFO_FO_N_BUFF_ICONS() as usize, obj, false)
}

/// The ship image's area (`infoSingleSystemsDisplay`), local to the panel.
pub fn image_rect() -> Option<[i32; 4]> {
    comp_rect(child(field::INFO_FO_SYSTEMS()))
}

/// The race icon child (`RaceIcon`), for its tooltip and hover.
pub fn race_comp() -> usize {
    child(field::INFO_SD_RACE_ICON())
}

/// The craft's rank in FleetOpsHook's record of it (`XPlevel`), 0 for none.
pub fn rank(obj: usize) -> u32 {
    let craft = cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CRAFT);
    let rec = field(craft, field::INFO_FO_CRAFT_RECORD());
    if rec == 0 || !readable(rec, 0x10) {
        return 0;
    }
    let class = field(rec, field::INFO_FO_RECORD_CLASS());
    if class == 0 || !readable(class + field::INFO_FO_CLASS_RANK(), 4) {
        return 0;
    }
    let r = unsafe { rd::<u32>(class + field::INFO_FO_CLASS_RANK()) };
    if r > 32 { 0 } else { r }
}

/// The craft's cloak: 0 none, 1 or 3 coming on or going off, 2 cloaked (Roots tints the ship
/// image by it).
pub fn cloak(obj: usize) -> u32 {
    let c = field(obj, field::INFO_OBJ_CLOAK());
    if c == 0 || !readable(c + field::INFO_CLOAK_STATE(), 4) {
        return 0;
    }
    unsafe { rd::<u32>(c + field::INFO_CLOAK_STATE()) }
}

/// A queue slot's component (`BuildWireframe`) and the plate sprite FleetOpsHook gave it.
pub fn queue_slot(i: usize) -> (usize, usize) {
    let c = child(field::INFO_SD_QUEUE() + 4 * i);
    (c, field(c, field::INFO_FO_BW_PLATE()))
}

/// The builder's queue, read only (the panel's `Render` hands its slots their classes).
pub fn queue_classes(obj: usize) -> Vec<usize> {
    super::info::queue_read(obj)
}

/// Whether `obj` is a craft (a ship or a station).
pub fn is_craft(obj: usize) -> bool {
    super::info::flags(obj) & super::info::f_craft() != 0
}

/// The hull as a fraction (`GameObject::health`) and the shields (now, at most).
pub fn hull_shields(obj: usize) -> (f32, Option<(f32, f32)>) {
    let h = f32_at(obj, field::INFO_OBJ_HEALTH()).clamp(0.0, 1.0);
    let craft = cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CRAFT);
    let s = (craft != 0).then(|| (f32_at(craft, field::INFO_CRAFT_SHIELD()), f32_at(craft, field::INFO_CRAFT_SHIELD_MAX())));
    (h, s)
}

/// The name, crew and special energy texts, as the panel's own children word them.
pub fn texts(obj: usize) -> (Option<Text>, Option<Text>, Option<Text>) {
    let f = super::info::flags(obj);
    let name = if f & (super::info::f_craft() | super::info::f_named()) != 0 { text_of(child(field::INFO_SD_NAME_TEXT()), obj, false) } else { None };
    let crew = if f32_at(obj, field::INFO_OBJ_CREW()) > 0.1 { text_of(child(field::INFO_SD_CREW_TEXT()), obj, true) } else { None };
    let energy = if f32_at(obj, field::INFO_OBJ_ENERGY()) > 0.1 { text_of(child(field::INFO_SD_ENERGY_TEXT()), obj, true) } else { None };
    (name, crew, energy)
}

/// The class's name in the HUD's sprite database (`GameObjectClass+0x7c`, as the wireframe
/// names), for `<name>_si` and `<name>w1`.
pub fn class_art_name(obj: usize) -> alloc::string::String {
    let class = field(obj, field::INFO_OBJ_CLASS());
    latin1(field(class, field::INFO_CLASS_WIREFRAME()))
}

/// The class's art name of a class (a queued item).
pub fn class_art_name_of(class: usize) -> alloc::string::String {
    latin1(field(class, field::INFO_CLASS_WIREFRAME()))
}

// ---- the rectangles of the stock children that follow ours -----------------------------

/// The rectangles moved, with what the layout file gave them (put back by `restore`).
static mut MOVED: Vec<(usize, [i32; 4])> = Vec::new();

/// Sets a stock child's rectangle (local to the panel, canvas units, inclusive) where ours
/// draws it, so the engine's hover and click follow; the first time, keeps what it was.
#[allow(static_mut_refs)]
pub fn move_rect(c: usize, r: [i32; 4]) {
    if c == 0 || !readable(c, 0x20) {
        return;
    }
    unsafe {
        let at = c + field::INFO_COMP_RECT();
        let now: [i32; 4] = rd(at);
        let m = &mut MOVED;
        if !m.iter().any(|&(x, _)| x == c) {
            m.push((c, now));
        }
        if now != r {
            core::ptr::write_unaligned(at as *mut [i32; 4], r);
        }
    }
}

/// A moved child's rectangle as the layout file gave it (or as it is, when never moved).
#[allow(static_mut_refs)]
pub fn original_rect(c: usize) -> Option<[i32; 4]> {
    unsafe { MOVED.iter().find(|&&(x, _)| x == c).map(|&(_, r)| r) }.or_else(|| comp_rect(c))
}

/// Puts every moved rectangle back (ours off, or a new mission's panel).
#[allow(static_mut_refs)]
pub fn restore() {
    unsafe {
        for &(c, r) in MOVED.iter() {
            if readable(c, 0x20) {
                core::ptr::write_unaligned((c + field::INFO_COMP_RECT()) as *mut [i32; 4], r);
            }
        }
        MOVED.clear();
    }
}

/// Forgets the moved rectangles without writing (the panel they belonged to is gone).
#[allow(static_mut_refs)]
pub fn forget() {
    unsafe { MOVED.clear() };
}

/// Logs, once per panel, the children's rectangles the layout table is checked against.
pub fn log_children(panel: usize) {
    static mut LOGGED: usize = 0;
    unsafe {
        if LOGGED == panel {
            return;
        }
        LOGGED = panel;
    }
    let r = |c: usize| comp_rect(c).map_or(alloc::string::String::from("-"), |r| crate::sfmt!("{},{},{},{}", r[0], r[1], r[2] - r[0] + 1, r[3] - r[1] + 1));
    let mut line = crate::sfmt!("info: Roots' panel {:#x}: name {} crew {} energy {} image {}", panel, r(child(field::INFO_SD_NAME_TEXT())), r(child(field::INFO_SD_CREW_TEXT())), r(child(field::INFO_SD_ENERGY_TEXT())), r(child(field::INFO_FO_SYSTEMS())));
    for i in 0..super::info::n_systems() {
        line.push_str(&crate::sfmt!(" sys{} {}", i, r(child(field::INFO_SD_SYS_REPAIR() + 4 * i))));
    }
    line.push_str(&crate::sfmt!(" race {}", r(race_comp())));
    for i in 0..2 {
        line.push_str(&crate::sfmt!(" queue{} {}", i, r(queue_slot(i).0)));
    }
    line.push_str(&crate::sfmt!(" progress {}", r(child(field::INFO_SD_CONSTRUCTION_BAR()))));
    crate::log!("{}", line);
}
