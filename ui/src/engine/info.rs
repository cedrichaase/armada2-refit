//! The info panel's reads and presses (HUD.md, "The info panel"): what the stock
//! `ShipDisplay` (`shipDisplay`, `hud::SHIP_DISPLAY`) shows about the selection, read
//! through the panel's own child components, so ours shows what stock computes.
//!
//! The stock panel builds its children once, in `PostLoad`, and keeps them as members:
//! texts (`StandardText` and its subclasses: `NameText`, `ClassText`, `CrewNumText`,
//! `SystemText`, ...), bars (`StandardBar`: `HealthBar`, `CargoBar`,
//! `ConstructionBar`, ...) and icons (`WireframeIcon`, `RepairIcon`, `RaceIcon`,
//! `BuildWireframe`, `ShipIcon`). Every frame its `Render` hands each child the object
//! it is about and draws it. With that `Render` skipped, ours does the handing and asks
//! each child for its answer instead of its picture:
//!
//! - a text: `SetCurrentObject` (vtable +0x20), then `GetInformation` (+0x24), which
//!   answers the colour and the string stock would print, and `GetLabel` (+0x28), a
//!   key of the localised strings printed before it (`Localized_Strings::Localize`);
//!   a text with an icon (`TextAndSprite`) has the icon at +0x104, tinted by
//!   `GetCurrentColor` (+0x2c);
//! - a bar: the object at +0x428, then `GetBarValue` (+0x24, value and maximum) and
//!   `GetBarColor` (+0x20);
//! - any child: `GetTooltip` (+0x00) and `GetVerboseTooltip` (+0x04) write the words
//!   stock's rollover shows into a `std::stringstream`.
//!
//! Each of these returns its answer through a pointer it is given (a colour is three
//! floats, red, green, blue). The rest (the wireframe's sprites, the systems' repair
//! state, the build queue, the selection's objects) are reads of their own, below.
use super::{field, rd, selection};
use crate::syms_info::{field, hook, sym, value, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An engine address of the profile: 0 (and nothing to read) unless the hook that
/// uses it is ready.
fn at(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

/// The info panel (an instance, found in the render lists: `hud::ship_display`), or 0
/// while the info panel's hook is not ready. `shipDisplay` itself is only the profile's
/// gate here: Roots' panel is not that global (decision 64).
fn sd() -> usize {
    if at(sym::INFO_SHIP_DISPLAY) == 0 { 0 } else { super::hud::ship_display() }
}

// ---- the stock panel's children (members of `shipDisplay`) --------------------------

/// The hover strip (`SDInfoBar`): its own shield and hull texts, read for one unit.
#[inline(always)]
fn sd_infobar() -> usize { field::INFO_SD_INFOBAR() }
#[inline(always)]
fn ib_shield_text() -> usize { field::INFO_IB_SHIELD_TEXT() }
#[inline(always)]
fn ib_hull_text() -> usize { field::INFO_IB_HULL_TEXT() }
/// The five system texts (`SystemText`, with their icons) and their repair icons.
#[inline(always)]
fn sys_text() -> usize { field::INFO_SD_SYS_TEXT() }
#[inline(always)]
fn sys_repair() -> usize { field::INFO_SD_SYS_REPAIR() }
#[inline(always)]
pub fn n_systems() -> usize { value::INFO_N_SYSTEMS() as usize }
#[inline(always)]
fn crew_text() -> usize { field::INFO_SD_CREW_TEXT() }
#[inline(always)]
fn energy_text() -> usize { field::INFO_SD_ENERGY_TEXT() }
#[inline(always)]
fn class_text() -> usize { field::INFO_SD_CLASS_TEXT() }
#[inline(always)]
fn name_text() -> usize { field::INFO_SD_NAME_TEXT() }
#[inline(always)]
fn officer_text() -> usize { field::INFO_SD_OFFICER_TEXT() }
#[inline(always)]
fn hull_bar() -> usize { field::INFO_SD_HULL_BAR() }
#[inline(always)]
fn cargo_bar() -> usize { field::INFO_SD_CARGO_BAR() }
#[inline(always)]
fn metal_bar() -> usize { field::INFO_SD_METAL_BAR() }
#[inline(always)]
fn off_wireframe() -> usize { field::INFO_SD_WIREFRAME() }
#[inline(always)]
fn construction_bar() -> usize { field::INFO_SD_CONSTRUCTION_BAR() }
#[inline(always)]
fn research_text() -> usize { field::INFO_SD_RESEARCH_TEXT() }
#[inline(always)]
fn cargo_text() -> usize { field::INFO_SD_CARGO_TEXT() }
#[inline(always)]
fn population_text() -> usize { field::INFO_SD_POPULATION_TEXT() }
#[inline(always)]
fn planet_owner_text() -> usize { field::INFO_SD_PLANET_OWNER_TEXT() }
/// The builder's queue: ten `BuildWireframe`s (the queued class at +0x3c, the builder
/// at +0x28, the slot at +0x40).
#[inline(always)]
fn off_queue() -> usize { field::INFO_SD_QUEUE() }
#[inline(always)]
pub fn queue_len() -> usize { value::INFO_QUEUE_LEN() as usize }
#[inline(always)]
fn bw_builder() -> usize { field::INFO_BW_BUILDER() }
#[inline(always)]
fn bw_class() -> usize { field::INFO_BW_CLASS() }
/// The object the panel describes (set by `DetermineSelectedDisplayType`), and the
/// craft the pointer is over in the 3D view (set by `ShipDisplay::AlwaysSimulate`,
/// cleared by its `Render` once drawn).
#[inline(always)]
fn off_current() -> usize { field::INFO_SD_CURRENT() }
#[inline(always)]
fn off_hover() -> usize { field::INFO_SD_HOVER() }

/// `ShipDisplay::DetermineSelectedDisplayType`, thiscall: which of the panel's faces
/// the selection needs, and with it the object (`+0x1e8`) and the panel's height
/// (`+0x1f4`), as stock's `Render` and `FocusSimulate` ask it first thing.
/// `Entity::NO_ENTITY`, the handle of nothing, is a global of the profile.

// ---- component vtable slots ---------------------------------------------------------

#[inline(always)]
fn vt_tooltip() -> usize { field::INFO_VT_TOOLTIP() }
#[inline(always)]
fn vt_verbose() -> usize { field::INFO_VT_VERBOSE() }
#[inline(always)]
fn vt_set_object() -> usize { field::INFO_VT_SET_OBJECT() }
#[inline(always)]
fn vt_information() -> usize { field::INFO_VT_INFORMATION() }
#[inline(always)]
fn vt_label() -> usize { field::INFO_VT_LABEL() }
#[inline(always)]
fn vt_current_color() -> usize { field::INFO_VT_CURRENT_COLOR() }
#[inline(always)]
fn vt_bar_color() -> usize { field::INFO_VT_BAR_COLOR() }
#[inline(always)]
fn vt_bar_value() -> usize { field::INFO_VT_BAR_VALUE() }
#[inline(always)]
fn ts_sprite() -> usize { field::INFO_TS_SPRITE() }
#[inline(always)]
fn bar_object() -> usize { field::INFO_BAR_OBJECT() }
/// `RepairIcon`: the craft (+0x28), its system (+0x2c), the system's id in the
/// craft's repair list (+0x30), its icon (+0x34), and the flash it blinks with (+0x40,
/// toggled every 0.4 s by `RepairIcon::Flash` in the panel's always-step).
#[inline(always)]
fn ri_system() -> usize { field::INFO_RI_SYSTEM() }
#[inline(always)]
fn ri_repair_id() -> usize { field::INFO_RI_REPAIR_ID() }
#[inline(always)]
fn ri_sprite() -> usize { field::INFO_RI_SPRITE() }
#[inline(always)]
fn ri_flash() -> usize { field::INFO_RI_FLASH() }
/// `WireframeIcon`: its flash (+0x30), toggled as the repair icons'.
#[inline(always)]
fn wi_flash() -> usize { field::INFO_WI_FLASH() }

// ---- objects --------------------------------------------------------------------------

/// `Entity+0x14`, what an object is: 0x4 a game object, 0x8 a craft (ships, stations,
/// planets), 0x20 it produces, 0x200 it builds (the builder's face), 0x80 and
/// 0x80000000 it carries cargo, 0x4000 a named object that is no craft, 0x40000 under
/// construction, 0x101000 drawn whole (no systems in its wireframe).
#[inline(always)]
fn obj_flags() -> usize { field::INFO_OBJ_FLAGS() }
#[inline(always)]
pub fn f_craft() -> u32 { value::INFO_F_CRAFT() as u32 }
#[inline(always)]
fn f_producer() -> u32 { value::INFO_F_PRODUCER() as u32 }
#[inline(always)]
fn f_cargo() -> u32 { value::INFO_F_CARGO() as u32 }
#[inline(always)]
fn f_cargo2() -> u32 { value::INFO_F_CARGO2() as u32 }
#[inline(always)]
fn f_named() -> u32 { value::INFO_F_NAMED() as u32 }
#[inline(always)]
fn f_construction() -> u32 { value::INFO_F_CONSTRUCTION() as u32 }
#[inline(always)]
fn f_whole() -> u32 { value::INFO_F_WHOLE() as u32 }
/// `GameObject+0x40`: its class (`GameObjectClass`), whose `+0x7c` names its
/// wireframe sprites and `+0x88` is its officer cost (`Get_Raw_Officer_Cost`).
#[inline(always)]
fn obj_class() -> usize { field::INFO_OBJ_CLASS() }
#[inline(always)]
fn class_wireframe() -> usize { field::INFO_CLASS_WIREFRAME() }
#[inline(always)]
fn class_officers() -> usize { field::INFO_CLASS_OFFICERS() }
/// `ConstructionObject+0x188`: the class being built, whose wireframe it shows.
#[inline(always)]
fn constructing_class() -> usize { field::INFO_CONSTRUCTING_CLASS() }
/// The thresholds stock shows a value at: crew (+0x1c4) and special energy (+0x168)
/// above 0.1; a cargo store (+0x17c) when it has one; planets' systems (+0xec).
#[inline(always)]
fn obj_crew() -> usize { field::INFO_OBJ_CREW() }
#[inline(always)]
fn obj_energy() -> usize { field::INFO_OBJ_ENERGY() }
#[inline(always)]
fn obj_store() -> usize { field::INFO_OBJ_STORE() }
#[inline(always)]
fn planet_systems() -> usize { field::INFO_PLANET_SYSTEMS() }
/// `GameObject+0x158`: health, 0..1 (a whole wireframe goes red below a half).
#[inline(always)]
fn obj_health() -> usize { field::INFO_OBJ_HEALTH() }
/// `GameObject+0xfc`: its race, whose name (+0x14) names the race emblem
/// (`<name>_icon`, as `RaceIcon::Render` finds it).
#[inline(always)]
fn obj_race() -> usize { field::INFO_OBJ_RACE() }
#[inline(always)]
fn race_name() -> usize { field::INFO_RACE_NAME() }
/// `Craft+0x1c8` / `+0x1cc`: shields now and at most (`HealthBar::GetBarColor` reads
/// them for the colour it gives the hull bar).
#[inline(always)]
fn craft_shield() -> usize { field::INFO_CRAFT_SHIELD() }
#[inline(always)]
fn craft_shield_max() -> usize { field::INFO_CRAFT_SHIELD_MAX() }
/// `Craft+0x1e0`: its systems, 0x30 bytes each in `eSystem` order; a system's byte +1
/// is set while it is out, `+0x28` the repair under way. `Craft+0x30`: the list of
/// systems being repaired (`std::list`, each item's +0x10 the repair id).
#[inline(always)]
fn craft_systems() -> usize { field::INFO_CRAFT_SYSTEMS() }
#[inline(always)]
fn system_bytes() -> usize { value::INFO_SYSTEM_BYTES() as usize }
#[inline(always)]
fn craft_repairs() -> usize { field::INFO_CRAFT_REPAIRS() }
/// `IsPlanet`, vtable +0x0c of every entity.
#[inline(always)]
fn vt_is_planet() -> usize { field::INFO_VT_IS_PLANET() }
/// `Craft::mComputeRepairLeft(const CraftSystem *)`, thiscall: how much of a system is
/// still to be repaired, 0..1.
/// `GameObject::GetSlowEffect` / `GetLineOfSightEffect`, thiscall: the gravity mine's
/// slowing, and a line of sight lent to another team (a Romulan spy's, when its
/// ordnance class (+0x4, +0x18) is a `RomulanSpyClass`). Stock shows each as an icon by
/// the wireframe (`gravmine_icon`, `romspy_icon`).
/// `__RTDynamicCast`, cdecl, and the type descriptors the panel casts with.

/// `Producer::GetBuildQueue`, thiscall: the ten queued classes (0 for a free slot).
/// `Producer::RequestDeleteBuildQueueItem(int)`, vtable +0x140: what a click on a
/// queue slot does (`BuildWireframe::Simulate`).
#[inline(always)]
fn vt_delete_queue_item() -> usize { field::INFO_VT_DELETE_QUEUE_ITEM() }
/// `cOverViewImp::Select(GameObject *, eSelectionType, bool, bool)`, vtable +0x90:
/// what a click on a group icon does (`ShipIcon::Simulate`): `(obj, 0, true, true)`
/// selects it alone, with Shift `(obj, 1, false, true)` takes it out.
#[inline(always)]
fn vt_select() -> usize { field::INFO_VT_SELECT() }
/// `gCameraManager`'s camera (its first member, a `GameCamera *`) and its
/// `SetInterestEntity` (vtable +0x58): what a click on the one-unit panel does
/// (`ShipDisplay::SimulateSingleObject`), the camera goes to the unit.
#[inline(always)]
fn vt_set_interest_entity() -> usize { field::INFO_VT_SET_INTEREST_ENTITY() }
/// `Localized_Strings::Localize`, thiscall on `g_string`.

// ---- what the panel shows ---------------------------------------------------------------

/// The panel's face: `DetermineSelectedDisplayType`'s answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    None,
    Single,
    Builder,
    /// 2 to 8 selected, and more than 8 (stock's two group layouts).
    Group,
}

/// Asks the stock panel which face it would show, as its `Render` does first; with it
/// the panel's object (`current`).
pub fn face() -> Face {
    let (a, panel) = (at(sym::INFO_DETERMINE_TYPE), sd());
    if a == 0 || panel == 0 {
        return Face::None;
    }
    let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(a) };
    match unsafe { f(panel) } {
        1 => Face::Single,
        2 => Face::Builder,
        3 | 4 => Face::Group,
        _ => Face::None,
    }
}

/// The object the panel describes (after `face`).
pub fn current() -> usize {
    let panel = sd();
    if panel == 0 { 0 } else { unsafe { rd::<usize>(panel + off_current()) } }
}

/// Whether the panel still shows `obj` alone (the close-up checks before it draws).
pub fn still_single(obj: usize) -> bool {
    sd() != 0 && !super::hud::hidden(sd()) && !super::ingame::hud_hidden() && matches!(face(), Face::Single | Face::Builder) && current() == obj
}

/// The craft under the pointer in the 3D view this frame, and clears it as stock's
/// `Render` does once it has drawn the hover strip.
pub fn take_hover() -> usize {
    let (panel, none_at) = (sd(), at(sym::INFO_NO_ENTITY));
    if panel == 0 || none_at == 0 {
        return 0;
    }
    unsafe {
        let none = rd::<i32>(none_at);
        let h = rd::<i32>(panel + off_hover());
        *((panel + off_hover()) as *mut i32) = none;
        if h == none {
            return 0;
        }
        let o = selection::object(h);
        if o == 0 || flags(o) & f_craft() == 0 { 0 } else { o }
    }
}

pub fn flags(obj: usize) -> u32 {
    if obj == 0 { 0 } else { unsafe { rd::<u32>(obj + obj_flags()) } }
}

fn f32_at(obj: usize, off: usize) -> f32 {
    unsafe { rd::<f32>(obj + off) }
}

fn is_planet(obj: usize) -> bool {
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(obj) + vt_is_planet())) };
    unsafe { f(obj) }
}

fn cast(obj: usize, from: a2engine::Sym, to: a2engine::Sym) -> usize {
    let (a, from, to) = (at(sym::INFO_RT_DYNAMIC_CAST), at(from), at(to));
    if obj == 0 || a == 0 || from == 0 || to == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(usize, i32, usize, usize, i32) -> usize = unsafe { core::mem::transmute(a) };
    unsafe { f(obj, 0, from, to, 0) }
}

/// A NUL-terminated Latin-1 string of the game's.
fn latin1(p: usize) -> String {
    let mut s = String::new();
    if p == 0 {
        return s;
    }
    let mut i = 0;
    while i < 512 {
        let b = unsafe { rd::<u8>(p + i) };
        if b == 0 {
            break;
        }
        s.push(b as char);
        i += 1;
    }
    s
}

fn localize(key: usize) -> String {
    let (gs, a) = (at(sym::INFO_G_STRING), at(sym::INFO_LOCALIZE));
    let g = if gs == 0 { 0 } else { unsafe { rd::<usize>(gs) } };
    if key == 0 || g == 0 || a == 0 || unsafe { rd::<u8>(key) } == 0 {
        return String::new();
    }
    let f: unsafe extern "thiscall" fn(usize, usize) -> usize = unsafe { core::mem::transmute(a) };
    latin1(unsafe { f(g, key) })
}

/// A stock colour (three floats, 0..1) as 0xAARRGGBB.
pub fn colour(c: [f32; 3]) -> u32 {
    let b = |v: f32| ((v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32).min(255);
    0xff00_0000 | b(c[0]) << 16 | b(c[1]) << 8 | b(c[2])
}

fn vcall(c: usize, slot: usize) -> usize {
    unsafe { rd::<usize>(rd::<usize>(c) + slot) }
}

/// A text child's answer for `obj`: its label (localised), colour and string.
pub struct Text {
    pub label: String,
    pub colour: [f32; 3],
    pub text: String,
    /// The child, for its tooltip.
    pub comp: usize,
    /// Its icon (a `TextAndSprite`'s) and the icon's tint, or 0.
    pub icon: usize,
    pub icon_colour: [f32; 3],
}

fn text_of(c: usize, obj: usize, with_icon: bool) -> Option<Text> {
    if c == 0 || obj == 0 {
        return None;
    }
    unsafe {
        let set: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(vcall(c, vt_set_object()));
        set(c, obj);
        let mut out = [0u32; 4];
        let info: unsafe extern "thiscall" fn(usize, *mut u32) -> usize = core::mem::transmute(vcall(c, vt_information()));
        info(c, out.as_mut_ptr());
        let label: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(vcall(c, vt_label()));
        let mut t = Text {
            label: localize(label(c)),
            colour: [f32::from_bits(out[0]), f32::from_bits(out[1]), f32::from_bits(out[2])],
            text: latin1(out[3] as usize),
            comp: c,
            icon: 0,
            icon_colour: [1.0; 3],
        };
        if with_icon {
            t.icon = field(c, ts_sprite());
            if t.icon != 0 {
                let mut col = [0f32; 3];
                let cur: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, vt_current_color()));
                cur(c, col.as_mut_ptr());
                t.icon_colour = col;
            }
        }
        Some(t)
    }
}

/// A bar child's answer for `obj`: value, maximum, colour.
pub struct Bar {
    pub value: f32,
    pub max: f32,
    pub colour: [f32; 3],
    pub comp: usize,
}

impl Bar {
    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 { (self.value / self.max).clamp(0.0, 1.0) } else { 0.0 }
    }
}

fn bar_of(c: usize, obj: usize) -> Option<Bar> {
    if c == 0 || obj == 0 {
        return None;
    }
    unsafe {
        *((c + bar_object()) as *mut usize) = obj;
        let mut v = [0f32; 2];
        let val: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, vt_bar_value()));
        val(c, v.as_mut_ptr());
        let mut col = [0f32; 3];
        let colf: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, vt_bar_color()));
        colf(c, col.as_mut_ptr());
        Some(Bar { value: v[0], max: v[1], colour: col, comp: c })
    }
}

fn child(off: usize) -> usize {
    let panel = sd();
    if panel == 0 { 0 } else { unsafe { rd::<usize>(panel + off) } }
}

/// A child's rollover text (`GetTooltip`, or `GetVerboseTooltip`), for the object it
/// was last handed.
pub fn tooltip(comp: usize, verbose: bool) -> Option<String> {
    if comp == 0 || !super::msvc::available() {
        return None;
    }
    let slot = if verbose { vt_verbose() } else { vt_tooltip() };
    super::msvc::with_stringstream(|ss| unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(vcall(comp, slot));
        f(comp, ss)
    })
}

// ---- the wireframe ---------------------------------------------------------------------

/// A unit's wireframe: its class's sprites from the HUD's sprite database, named
/// `<wireframe>w1` .. `w5` (one per system, in `eSystem` order), or `<wireframe>_s`
/// for an object drawn whole (`WireframeIcon::GetWireframe`).
pub struct Wire {
    pub sprites: Vec<usize>,
    pub whole: bool,
}

/// The wireframe of `class` (a `GameObjectClass`), whole or by system, falling back
/// to the other form when the art has only that one.
pub fn class_wire(class: usize, whole: bool) -> Option<Wire> {
    let name = latin1(field(class, class_wireframe()));
    if name.is_empty() {
        return None;
    }
    let by_system = || {
        let mut s = alloc::vec![0usize; n_systems()];
        for (i, sp) in s.iter_mut().enumerate() {
            *sp = super::palette::find_sprite(&crate::sfmt!("{}w{}", name, i + 1));
        }
        s
    };
    let whole_one = || super::palette::find_sprite(&crate::sfmt!("{}_s", name));
    if !whole {
        let s = by_system();
        if s.iter().any(|&x| x != 0) {
            return Some(Wire { sprites: s, whole: false });
        }
    }
    let w = whole_one();
    if w != 0 {
        let mut sprites = alloc::vec![0usize; n_systems().max(1)];
        sprites[0] = w;
        return Some(Wire { sprites, whole: true });
    }
    let s = by_system();
    s.iter().any(|&x| x != 0).then_some(Wire { sprites: s, whole: false })
}

/// Whether `obj` is drawn whole rather than by system (`WireframeIcon::Render`).
pub fn drawn_whole(obj: usize) -> bool {
    flags(obj) & f_whole() != 0 || is_planet(obj)
}

/// The wireframe `obj` shows: its own class's, or for a station under construction
/// the class being built.
pub fn wire(obj: usize) -> Option<Wire> {
    if obj == 0 {
        return None;
    }
    let cons = cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CONSTRUCTION);
    let class = if cons != 0 { field(cons, constructing_class()) } else { field(obj, obj_class()) };
    class_wire(class, drawn_whole(obj))
}

/// A system's state on a craft, for its wireframe part and its repair icon.
#[derive(Clone, Copy, Default)]
pub struct SystemState {
    /// Still to be repaired, 0..1 (0: intact).
    pub repair_left: f32,
    /// Being repaired now (in the craft's repair list, or a repair under way).
    pub repairing: bool,
}

/// The repair id of each system (`eSystem` 0..4), as the craft's repair list names
/// them (`WireframeIcon::DrawSystem`).
fn repair_id(sys: usize) -> Option<u32> {
    Some(match sys {
        0 => value::INFO_REPAIR_ID_0(),
        1 => value::INFO_REPAIR_ID_1(),
        2 => value::INFO_REPAIR_ID_2(),
        3 => value::INFO_REPAIR_ID_3(),
        4 => value::INFO_REPAIR_ID_4(),
        _ => return None,
    } as u32)
}

pub fn system_state(obj: usize, sys: usize) -> Option<SystemState> {
    if obj == 0 || sys >= n_systems() || cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CRAFT) == 0 {
        return None;
    }
    let systems = field(obj, craft_systems());
    if systems == 0 {
        return None;
    }
    let s = systems + sys * system_bytes();
    let a = at(sym::INFO_REPAIR_LEFT);
    if a == 0 {
        return None;
    }
    let f: unsafe extern "thiscall" fn(usize, usize) -> f32 = unsafe { core::mem::transmute(a) };
    let left = unsafe { f(obj, s) };
    let mut repairing = false;
    unsafe {
        let head = rd::<usize>(obj + craft_repairs());
        if head != 0 {
            let mut n = rd::<usize>(head);
            let mut guard = 0;
            while n != head && n != 0 && guard < 32 {
                let item = rd::<usize>(n + 8);
                if item != 0 && Some(rd::<u32>(item + field::INFO_REPAIR_ITEM_ID())) == repair_id(sys) {
                    repairing = true;
                    break;
                }
                n = rd::<usize>(n);
                guard += 1;
            }
        }
        if !repairing && rd::<u8>(s + field::INFO_SYS_OUT()) == 0 && rd::<f32>(s + field::INFO_SYS_REPAIR()) > 0.0 {
            repairing = true;
        }
    }
    Some(SystemState { repair_left: left, repairing })
}

/// The flash stock's wireframe and repair icons blink with (on half of every 0.8 s).
pub fn flash() -> bool {
    let w = child(off_wireframe());
    w != 0 && unsafe { rd::<u8>(w + wi_flash()) } != 0
}

/// The one-object face's wireframe component (handed its object by `single`), for
/// its tooltip.
pub fn wire_comp() -> usize {
    child(off_wireframe())
}

/// Whether a whole object's wireframe goes red (health below a half).
pub fn whole_damaged(obj: usize) -> bool {
    obj != 0 && f32_at(obj, obj_health()) < 0.5
}

/// The status icons stock draws by the wireframe: slowed by a gravity mine, a
/// Romulan spy aboard.
pub fn effects(obj: usize) -> (bool, bool) {
    let craft = cast(obj, sym::INFO_TD_GAME_OBJECT, sym::INFO_TD_CRAFT);
    if craft == 0 {
        return (false, false);
    }
    let (sa, la) = (at(sym::INFO_SLOW_EFFECT), at(sym::INFO_LOS_EFFECT));
    if sa == 0 || la == 0 {
        return (false, false);
    }
    unsafe {
        let slow: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(sa);
        let los: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(la);
        let s = slow(craft) != 0;
        let l = los(craft);
        let spy = l != 0 && field(l, field::INFO_LOS_ORDNANCE()) != 0 && cast(field(field(l, field::INFO_LOS_ORDNANCE()), field::INFO_ORDNANCE_CLASS()), sym::INFO_TD_ORDNANCE_CLASS, sym::INFO_TD_ROMULAN_SPY_CLASS) != 0;
        (s, spy)
    }
}

/// The sprites of those two icons.
pub fn effect_sprites() -> (usize, usize) {
    (super::palette::find_sprite("gravmine_icon"), super::palette::find_sprite("romspy_icon"))
}

/// The race emblem of `obj` (`<race>_icon`), or 0.
pub fn race_icon(obj: usize) -> usize {
    let race = field(obj, obj_race());
    if race == 0 {
        return 0;
    }
    let name = latin1(race + race_name());
    if name.is_empty() { 0 } else { super::palette::find_sprite(&crate::sfmt!("{}_icon", name)) }
}

// ---- one object, as stock's single face shows it --------------------------------------

/// A system row: the system's text and icon, and its repair state.
pub struct System {
    pub text: Text,
    /// The repair icon's sprite, or 0.
    pub repair_icon: usize,
    pub repair_comp: usize,
    pub state: SystemState,
    pub flash: bool,
}

/// Everything stock's one-object face shows, in its own words (`DisplaySingleObject`):
/// which parts an object gets is stock's choice, made here as stock makes it.
#[derive(Default)]
pub struct Single {
    pub obj: usize,
    pub craft: bool,
    pub planet: bool,
    pub name: Option<Text>,
    pub class: Option<Text>,
    /// Texts with icons: crew, officers, energy, cargo, population, research, owner.
    pub stats: Vec<Text>,
    /// The hull (a craft's `HealthBar`), and stock's shield and hull texts.
    pub hull: Option<Bar>,
    pub hull_text: Option<Text>,
    pub shield_text: Option<Text>,
    /// Shields now / at most, for a craft that has them.
    pub shields: Option<(f32, f32)>,
    /// Shields down: the hull bar's colour is pure blue then.
    pub shields_down: bool,
    /// Other bars: cargo, a planet's metal, construction (progress).
    pub bars: Vec<(BarKind, Bar)>,
    pub systems: Vec<System>,
    pub race_icon: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarKind {
    Cargo,
    Metal,
    Construction,
}

pub fn single(obj: usize) -> Single {
    let mut s = Single { obj, ..Default::default() };
    if obj == 0 {
        return s;
    }
    let fl = flags(obj);
    let w = child(off_wireframe());
    if w != 0 {
        unsafe { *((w + field::INFO_WI_OBJECT()) as *mut usize) = obj };
    }
    s.class = text_of(child(class_text()), obj, false);
    if fl & f_craft() != 0 {
        s.craft = true;
        s.race_icon = race_icon(obj);
        s.name = text_of(child(name_text()), obj, false);
        if f32_at(obj, obj_crew()) > 0.1 {
            s.stats.extend(text_of(child(crew_text()), obj, true));
        }
        if is_planet(obj) {
            s.planet = true;
            s.stats.extend(text_of(child(population_text()), obj, false));
            s.stats.extend(text_of(child(planet_owner_text()), obj, false));
            if let Some(b) = bar_of(child(metal_bar()), obj) {
                s.bars.push((BarKind::Metal, b));
            }
            if unsafe { rd::<usize>(obj + planet_systems()) } != 0 {
                for i in 0..2 {
                    s.systems.extend(system(obj, i));
                }
                s.stats.extend(text_of(child(officer_text()), obj, true));
                if let Some(b) = bar_of(child(construction_bar()), obj) {
                    s.bars.push((BarKind::Construction, b));
                }
            }
            return s;
        }
        if field(field(obj, obj_class()), class_officers()) as i32 > 0 {
            s.stats.extend(text_of(child(officer_text()), obj, true));
        }
        s.hull = bar_of(child(hull_bar()), obj);
        if let Some(h) = &s.hull {
            s.shields_down = h.colour[0] == 0.0 && h.colour[1] == 0.0 && h.colour[2] >= 1.0;
        }
        let max = f32_at(obj, craft_shield_max());
        if max > 0.0 {
            s.shields = Some((f32_at(obj, craft_shield()), max));
        }
        let ib = child(sd_infobar());
        s.hull_text = text_of(field(ib, ib_hull_text()), obj, true);
        s.shield_text = text_of(field(ib, ib_shield_text()), obj, true);
        if fl & (f_cargo() | f_cargo2()) != 0 {
            s.stats.extend(text_of(child(cargo_text()), obj, false));
            if let Some(b) = bar_of(child(cargo_bar()), obj) {
                s.bars.push((BarKind::Cargo, b));
            }
        } else if fl & f_producer() != 0 || cast(obj, sym::INFO_TD_CRAFT, sym::INFO_TD_COLONY) != 0 {
            if let Some(b) = bar_of(child(construction_bar()), obj) {
                s.bars.push((BarKind::Construction, b));
            }
            s.stats.extend(text_of(child(research_text()), obj, false));
        }
        if f32_at(obj, obj_energy()) > 0.1 && f32_at(obj, obj_energy()) as i32 > 0 {
            s.stats.extend(text_of(child(energy_text()), obj, true));
        }
        for i in 0..n_systems() {
            s.systems.extend(system(obj, i));
        }
    } else {
        if unsafe { rd::<usize>(obj + obj_store()) } != 0 {
            if let Some(b) = bar_of(child(cargo_bar()), obj) {
                s.bars.push((BarKind::Cargo, b));
            }
        } else if fl & f_construction() != 0 {
            if let Some(b) = bar_of(child(construction_bar()), obj) {
                s.bars.push((BarKind::Construction, b));
            }
        }
        if fl & f_named() != 0 {
            s.name = text_of(child(name_text()), obj, false);
        }
    }
    s
}

fn system(obj: usize, i: usize) -> Option<System> {
    let text = text_of(child(sys_text() + 4 * i), obj, true)?;
    let ri = child(sys_repair() + 4 * i);
    let (icon, sys, flash) = if ri != 0 {
        unsafe {
            *((ri + field::INFO_RI_OBJECT()) as *mut usize) = obj;
            (field(ri, ri_sprite()), rd::<u32>(ri + ri_system()) as usize, rd::<u8>(ri + ri_flash()) != 0)
        }
    } else {
        (0, i, false)
    };
    let _ = ri_repair_id();
    Some(System { text, repair_icon: icon, repair_comp: ri, state: system_state(obj, sys).unwrap_or_default(), flash })
}

/// The hull bar of any object (for group icons): value, maximum, stock's colour.
pub fn hull(obj: usize) -> Option<Bar> {
    bar_of(child(hull_bar()), obj)
}

/// A unit's name and class, as the panel words them.
pub fn name_and_class(obj: usize) -> (Option<Text>, Option<Text>) {
    let name = if flags(obj) & (f_craft() | f_named()) != 0 { text_of(child(name_text()), obj, false) } else { None };
    (name, text_of(child(class_text()), obj, false))
}

/// The hover strip's shield and hull texts for a craft.
pub fn hover_texts(obj: usize) -> (Option<Text>, Option<Text>) {
    let ib = child(sd_infobar());
    (text_of(field(ib, ib_shield_text()), obj, false), text_of(field(ib, ib_hull_text()), obj, false))
}

// ---- the builder's queue -------------------------------------------------------------

/// The builder's queue: each slot's class (0: free), and stock's slot component for
/// its tooltip (handed the builder and the class, as stock's `Render` does).
pub fn queue(obj: usize) -> Vec<(usize, usize)> {
    let mut out = alloc::vec![(0usize, 0usize); queue_len()];
    if obj == 0 || flags(obj) & f_producer() == 0 {
        return out;
    }
    let a = at(sym::INFO_GET_BUILD_QUEUE);
    if a == 0 {
        return out;
    }
    let f: unsafe extern "thiscall" fn(usize) -> *const usize = unsafe { core::mem::transmute(a) };
    let q = unsafe { f(obj) };
    if q.is_null() {
        return out;
    }
    for (i, o) in out.iter_mut().enumerate() {
        let class = unsafe { *q.add(i) };
        let comp = child(off_queue() + 4 * i);
        if comp != 0 {
            unsafe {
                *((comp + bw_builder()) as *mut usize) = obj;
                *((comp + bw_class()) as *mut usize) = class;
            }
        }
        *o = (class, comp);
    }
    out
}

/// The build progress of the queue's first item (the panel's construction bar).
pub fn build_progress(obj: usize) -> Option<Bar> {
    bar_of(child(construction_bar()), obj)
}

/// Cancels the queue's slot `i`, as a click on it does.
pub fn cancel(obj: usize, i: usize) {
    if obj == 0 || flags(obj) & f_producer() == 0 {
        return;
    }
    crate::log!("info: cancel queue slot {}", i);
    let f: unsafe extern "thiscall" fn(usize, i32) = unsafe { core::mem::transmute(vcall(obj, vt_delete_queue_item())) };
    unsafe { f(obj, i as i32) };
}

// ---- the group ------------------------------------------------------------------------

/// `GameObject+0x113`: set once it is destroyed (stock's panel leaves it out).
#[inline(always)]
fn obj_destroyed() -> usize { field::INFO_OBJ_DESTROYED() }

/// The selected objects, in selection order (gone ones left out, as stock's
/// `GetObjUnlessDestroyed`).
pub fn group() -> Vec<usize> {
    selection::handles()
        .into_iter()
        .map(selection::object)
        .filter(|&o| o != 0 && flags(o) & value::INFO_F_GAME_OBJECT() as u32 != 0 && unsafe { rd::<u8>(o + obj_destroyed()) } == 0)
        .collect()
}

/// `g_pCommandShift`: points to the game's own state of its Shift binding, which
/// `ShipIcon::Simulate` asks.

/// Whether the game sees Shift held (its command binding, as stock's click asks).
pub fn shift_held() -> bool {
    let a = at(sym::INFO_COMMAND_SHIFT);
    let p = if a == 0 { 0 } else { unsafe { rd::<usize>(a) } };
    p != 0 && unsafe { rd::<i32>(p) } != 0
}

/// A click on a group icon: selects that unit alone, or with Shift takes it out of
/// the selection.
pub fn select(obj: usize, shift: bool) {
    let o = selection::overview();
    if o == 0 || obj == 0 {
        return;
    }
    crate::log!("info: {} {:#x}", if shift { "deselect" } else { "select" }, obj);
    let f: unsafe extern "thiscall" fn(usize, usize, i32, i32, i32) = unsafe { core::mem::transmute(vcall(o, vt_select())) };
    unsafe {
        if shift { f(o, obj, 1, 0, 1) } else { f(o, obj, 0, 1, 1) }
    }
}

/// The camera goes to `obj`, as a click on the one-unit panel does.
pub fn look_at(obj: usize) {
    let cm = at(sym::INFO_CAMERA_MANAGER);
    let cam = if cm == 0 { 0 } else { unsafe { rd::<usize>(cm) } };
    if cam == 0 || obj == 0 {
        return;
    }
    crate::log!("info: camera to {:#x}", obj);
    let f: unsafe extern "thiscall" fn(usize, usize) = unsafe { core::mem::transmute(vcall(cam, vt_set_interest_entity())) };
    unsafe { f(cam, obj) };
}

/// Every function above that is called is stock (the info panel's hook is ready), and so
/// are the selection's.
pub fn functions_stock() -> bool {
    eng().ready(hook::HUD_INFO).is_ok() && super::selection::functions_stock()
}
