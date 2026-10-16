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
use alloc::string::String;
use alloc::vec::Vec;

const SD: usize = super::hud::SHIP_DISPLAY;

// ---- the stock panel's children (members of `shipDisplay`) --------------------------

/// The hover strip (`SDInfoBar`): its own shield and hull texts, read for one unit.
const SD_INFOBAR: usize = 0x70;
const IB_SHIELD_TEXT: usize = 0x0c;
const IB_HULL_TEXT: usize = 0x10;
/// The five system texts (`SystemText`, with their icons) and their repair icons.
const SYS_TEXT: usize = 0x74;
const SYS_REPAIR: usize = 0xc4;
pub const N_SYSTEMS: usize = 5;
const CREW_TEXT: usize = 0x88;
const ENERGY_TEXT: usize = 0x8c;
const CLASS_TEXT: usize = 0x90;
const NAME_TEXT: usize = 0x94;
const OFFICER_TEXT: usize = 0x98;
const HULL_BAR: usize = 0xa0;
const CARGO_BAR: usize = 0xa4;
const METAL_BAR: usize = 0xa8;
const WIREFRAME: usize = 0xac;
const CONSTRUCTION_BAR: usize = 0xb0;
const RESEARCH_TEXT: usize = 0xb4;
const CARGO_TEXT: usize = 0xb8;
const POPULATION_TEXT: usize = 0xbc;
const PLANET_OWNER_TEXT: usize = 0xc0;
/// The builder's queue: ten `BuildWireframe`s (the queued class at +0x3c, the builder
/// at +0x28, the slot at +0x40).
const QUEUE: usize = 0x120;
pub const QUEUE_LEN: usize = 10;
const BW_BUILDER: usize = 0x28;
const BW_CLASS: usize = 0x3c;
/// The object the panel describes (set by `DetermineSelectedDisplayType`), and the
/// craft the pointer is over in the 3D view (set by `ShipDisplay::AlwaysSimulate`,
/// cleared by its `Render` once drawn).
const CURRENT: usize = 0x1e8;
const HOVER: usize = 0x1ec;

/// `ShipDisplay::DetermineSelectedDisplayType`, thiscall: which of the panel's faces
/// the selection needs, and with it the object (`+0x1e8`) and the panel's height
/// (`+0x1f4`), as stock's `Render` and `FocusSimulate` ask it first thing.
const DETERMINE_TYPE: usize = 0x4f2ee0;
/// `Entity::NO_ENTITY`, the handle of nothing.
const NO_ENTITY: usize = 0x6b41d8;

// ---- component vtable slots ---------------------------------------------------------

const VT_TOOLTIP: usize = 0x00;
const VT_VERBOSE: usize = 0x04;
const VT_SET_OBJECT: usize = 0x20;
const VT_INFORMATION: usize = 0x24;
const VT_LABEL: usize = 0x28;
const VT_CURRENT_COLOR: usize = 0x2c;
const VT_BAR_COLOR: usize = 0x20;
const VT_BAR_VALUE: usize = 0x24;
const TS_SPRITE: usize = 0x104;
const BAR_OBJECT: usize = 0x428;
/// `RepairIcon`: the craft (+0x28), its system (+0x2c), the system's id in the
/// craft's repair list (+0x30), its icon (+0x34), and the flash it blinks with (+0x40,
/// toggled every 0.4 s by `RepairIcon::Flash` in the panel's always-step).
const RI_SYSTEM: usize = 0x2c;
const RI_REPAIR_ID: usize = 0x30;
const RI_SPRITE: usize = 0x34;
const RI_FLASH: usize = 0x40;
/// `WireframeIcon`: its flash (+0x30), toggled as the repair icons'.
const WI_FLASH: usize = 0x30;

// ---- objects --------------------------------------------------------------------------

/// `Entity+0x14`, what an object is: 0x4 a game object, 0x8 a craft (ships, stations,
/// planets), 0x20 it produces, 0x200 it builds (the builder's face), 0x80 and
/// 0x80000000 it carries cargo, 0x4000 a named object that is no craft, 0x40000 under
/// construction, 0x101000 drawn whole (no systems in its wireframe).
const OBJ_FLAGS: usize = 0x14;
pub const F_CRAFT: u32 = 0x8;
const F_PRODUCER: u32 = 0x20;
const F_CARGO: u32 = 0x80;
const F_CARGO2: u32 = 0x8000_0000;
const F_NAMED: u32 = 0x4000;
const F_CONSTRUCTION: u32 = 0x40000;
const F_WHOLE: u32 = 0x101000;
/// `GameObject+0x40`: its class (`GameObjectClass`), whose `+0x7c` names its
/// wireframe sprites and `+0x88` is its officer cost (`Get_Raw_Officer_Cost`).
const OBJ_CLASS: usize = 0x40;
const CLASS_WIREFRAME: usize = 0x7c;
const CLASS_OFFICERS: usize = 0x88;
/// `ConstructionObject+0x188`: the class being built, whose wireframe it shows.
const CONSTRUCTING_CLASS: usize = 0x188;
/// The thresholds stock shows a value at: crew (+0x1c4) and special energy (+0x168)
/// above 0.1; a cargo store (+0x17c) when it has one; planets' systems (+0xec).
const OBJ_CREW: usize = 0x1c4;
const OBJ_ENERGY: usize = 0x168;
const OBJ_STORE: usize = 0x17c;
const PLANET_SYSTEMS: usize = 0xec;
/// `GameObject+0x158`: health, 0..1 (a whole wireframe goes red below a half).
const OBJ_HEALTH: usize = 0x158;
/// `GameObject+0xfc`: its race, whose name (+0x14) names the race emblem
/// (`<name>_icon`, as `RaceIcon::Render` finds it).
const OBJ_RACE: usize = 0xfc;
const RACE_NAME: usize = 0x14;
/// `Craft+0x1c8` / `+0x1cc`: shields now and at most (`HealthBar::GetBarColor` reads
/// them for the colour it gives the hull bar).
const CRAFT_SHIELD: usize = 0x1c8;
const CRAFT_SHIELD_MAX: usize = 0x1cc;
/// `Craft+0x1e0`: its systems, 0x30 bytes each in `eSystem` order; a system's byte +1
/// is set while it is out, `+0x28` the repair under way. `Craft+0x30`: the list of
/// systems being repaired (`std::list`, each item's +0x10 the repair id).
const CRAFT_SYSTEMS: usize = 0x1e0;
const SYSTEM_BYTES: usize = 0x30;
const CRAFT_REPAIRS: usize = 0x30;
/// `IsPlanet`, vtable +0x0c of every entity.
const VT_IS_PLANET: usize = 0x0c;
/// `Craft::mComputeRepairLeft(const CraftSystem *)`, thiscall: how much of a system is
/// still to be repaired, 0..1.
const REPAIR_LEFT: usize = 0x4c8ca0;
/// `GameObject::GetSlowEffect` / `GetLineOfSightEffect`, thiscall: the gravity mine's
/// slowing, and a line of sight lent to another team (a Romulan spy's, when its
/// ordnance class (+0x4, +0x18) is a `RomulanSpyClass`). Stock shows each as an icon by
/// the wireframe (`gravmine_icon`, `romspy_icon`).
const SLOW_EFFECT: usize = 0x4d4ee0;
const LOS_EFFECT: usize = 0x4d4f60;
/// `__RTDynamicCast`, cdecl, and the type descriptors the panel casts with.
const RT_DYNAMIC_CAST: usize = 0x6731a0;
const TD_GAME_OBJECT: usize = 0x6f1280;
const TD_CRAFT: usize = 0x6f12a0;
const TD_COLONY: usize = 0x6f14b0;
const TD_CONSTRUCTION: usize = 0x6ffc60;
const TD_ORDNANCE_CLASS: usize = 0x70cb08;
const TD_ROMULAN_SPY_CLASS: usize = 0x70cb28;

/// `Producer::GetBuildQueue`, thiscall: the ten queued classes (0 for a free slot).
const GET_BUILD_QUEUE: usize = 0x4b8070;
/// `Producer::RequestDeleteBuildQueueItem(int)`, vtable +0x140: what a click on a
/// queue slot does (`BuildWireframe::Simulate`).
const VT_DELETE_QUEUE_ITEM: usize = 0x140;
/// `cOverViewImp::Select(GameObject *, eSelectionType, bool, bool)`, vtable +0x90:
/// what a click on a group icon does (`ShipIcon::Simulate`): `(obj, 0, true, true)`
/// selects it alone, with Shift `(obj, 1, false, true)` takes it out.
const VT_SELECT: usize = 0x90;
/// `gCameraManager`'s camera (its first member, a `GameCamera *`) and its
/// `SetInterestEntity` (vtable +0x58): what a click on the one-unit panel does
/// (`ShipDisplay::SimulateSingleObject`), the camera goes to the unit.
const CAMERA_MANAGER: usize = 0x763370;
const VT_SET_INTEREST_ENTITY: usize = 0x58;
/// `Localized_Strings::Localize`, thiscall on `g_string`.
const LOCALIZE: usize = 0x481c90;
const G_STRING: usize = 0x7379fc;

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
    let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(DETERMINE_TYPE) };
    match unsafe { f(SD) } {
        1 => Face::Single,
        2 => Face::Builder,
        3 | 4 => Face::Group,
        _ => Face::None,
    }
}

/// The object the panel describes (after `face`).
pub fn current() -> usize {
    unsafe { rd::<usize>(SD + CURRENT) }
}

/// The craft under the pointer in the 3D view this frame, and clears it as stock's
/// `Render` does once it has drawn the hover strip.
pub fn take_hover() -> usize {
    unsafe {
        let none = rd::<i32>(NO_ENTITY);
        let h = rd::<i32>(SD + HOVER);
        *((SD + HOVER) as *mut i32) = none;
        if h == none {
            return 0;
        }
        let o = selection::object(h);
        if o == 0 || flags(o) & F_CRAFT == 0 { 0 } else { o }
    }
}

pub fn flags(obj: usize) -> u32 {
    if obj == 0 { 0 } else { unsafe { rd::<u32>(obj + OBJ_FLAGS) } }
}

fn f32_at(obj: usize, off: usize) -> f32 {
    unsafe { rd::<f32>(obj + off) }
}

fn is_planet(obj: usize) -> bool {
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(obj) + VT_IS_PLANET)) };
    unsafe { f(obj) }
}

fn cast(obj: usize, from: usize, to: usize) -> usize {
    if obj == 0 {
        return 0;
    }
    let f: unsafe extern "C" fn(usize, i32, usize, usize, i32) -> usize = unsafe { core::mem::transmute(RT_DYNAMIC_CAST) };
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
    let g = unsafe { rd::<usize>(G_STRING) };
    if key == 0 || g == 0 || unsafe { rd::<u8>(key) } == 0 {
        return String::new();
    }
    let f: unsafe extern "thiscall" fn(usize, usize) -> usize = unsafe { core::mem::transmute(LOCALIZE) };
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
        let set: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(vcall(c, VT_SET_OBJECT));
        set(c, obj);
        let mut out = [0u32; 4];
        let info: unsafe extern "thiscall" fn(usize, *mut u32) -> usize = core::mem::transmute(vcall(c, VT_INFORMATION));
        info(c, out.as_mut_ptr());
        let label: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(vcall(c, VT_LABEL));
        let mut t = Text {
            label: localize(label(c)),
            colour: [f32::from_bits(out[0]), f32::from_bits(out[1]), f32::from_bits(out[2])],
            text: latin1(out[3] as usize),
            comp: c,
            icon: 0,
            icon_colour: [1.0; 3],
        };
        if with_icon {
            t.icon = field(c, TS_SPRITE);
            if t.icon != 0 {
                let mut col = [0f32; 3];
                let cur: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, VT_CURRENT_COLOR));
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
        *((c + BAR_OBJECT) as *mut usize) = obj;
        let mut v = [0f32; 2];
        let val: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, VT_BAR_VALUE));
        val(c, v.as_mut_ptr());
        let mut col = [0f32; 3];
        let colf: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, VT_BAR_COLOR));
        colf(c, col.as_mut_ptr());
        Some(Bar { value: v[0], max: v[1], colour: col, comp: c })
    }
}

fn child(off: usize) -> usize {
    unsafe { rd::<usize>(SD + off) }
}

/// A child's rollover text (`GetTooltip`, or `GetVerboseTooltip`), for the object it
/// was last handed.
pub fn tooltip(comp: usize, verbose: bool) -> Option<String> {
    if comp == 0 || !super::msvc::available() {
        return None;
    }
    let slot = if verbose { VT_VERBOSE } else { VT_TOOLTIP };
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
    pub sprites: [usize; N_SYSTEMS],
    pub whole: bool,
}

/// The wireframe of `class` (a `GameObjectClass`), whole or by system, falling back
/// to the other form when the art has only that one.
pub fn class_wire(class: usize, whole: bool) -> Option<Wire> {
    let name = latin1(field(class, CLASS_WIREFRAME));
    if name.is_empty() {
        return None;
    }
    let by_system = || {
        let mut s = [0usize; N_SYSTEMS];
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
        return Some(Wire { sprites: [w, 0, 0, 0, 0], whole: true });
    }
    let s = by_system();
    s.iter().any(|&x| x != 0).then_some(Wire { sprites: s, whole: false })
}

/// Whether `obj` is drawn whole rather than by system (`WireframeIcon::Render`).
pub fn drawn_whole(obj: usize) -> bool {
    flags(obj) & F_WHOLE != 0 || is_planet(obj)
}

/// The wireframe `obj` shows: its own class's, or for a station under construction
/// the class being built.
pub fn wire(obj: usize) -> Option<Wire> {
    if obj == 0 {
        return None;
    }
    let cons = cast(obj, TD_GAME_OBJECT, TD_CONSTRUCTION);
    let class = if cons != 0 { field(cons, CONSTRUCTING_CLASS) } else { field(obj, OBJ_CLASS) };
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
const REPAIR_ID: [u32; N_SYSTEMS] = [3, 2, 1, 5, 6];

pub fn system_state(obj: usize, sys: usize) -> Option<SystemState> {
    if obj == 0 || sys >= N_SYSTEMS || cast(obj, TD_GAME_OBJECT, TD_CRAFT) == 0 {
        return None;
    }
    let systems = field(obj, CRAFT_SYSTEMS);
    if systems == 0 {
        return None;
    }
    let s = systems + sys * SYSTEM_BYTES;
    let f: unsafe extern "thiscall" fn(usize, usize) -> f32 = unsafe { core::mem::transmute(REPAIR_LEFT) };
    let left = unsafe { f(obj, s) };
    let mut repairing = false;
    unsafe {
        let head = rd::<usize>(obj + CRAFT_REPAIRS);
        if head != 0 {
            let mut n = rd::<usize>(head);
            let mut guard = 0;
            while n != head && n != 0 && guard < 32 {
                let item = rd::<usize>(n + 8);
                if item != 0 && rd::<u32>(item + 0x10) == REPAIR_ID[sys] {
                    repairing = true;
                    break;
                }
                n = rd::<usize>(n);
                guard += 1;
            }
        }
        if !repairing && rd::<u8>(s + 1) == 0 && rd::<f32>(s + 0x28) > 0.0 {
            repairing = true;
        }
    }
    Some(SystemState { repair_left: left, repairing })
}

/// The flash stock's wireframe and repair icons blink with (on half of every 0.8 s).
pub fn flash() -> bool {
    let w = child(WIREFRAME);
    w != 0 && unsafe { rd::<u8>(w + WI_FLASH) } != 0
}

/// The one-object face's wireframe component (handed its object by `single`), for
/// its tooltip.
pub fn wire_comp() -> usize {
    child(WIREFRAME)
}

/// Whether a whole object's wireframe goes red (health below a half).
pub fn whole_damaged(obj: usize) -> bool {
    obj != 0 && f32_at(obj, OBJ_HEALTH) < 0.5
}

/// The status icons stock draws by the wireframe: slowed by a gravity mine, a
/// Romulan spy aboard.
pub fn effects(obj: usize) -> (bool, bool) {
    let craft = cast(obj, TD_GAME_OBJECT, TD_CRAFT);
    if craft == 0 {
        return (false, false);
    }
    unsafe {
        let slow: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(SLOW_EFFECT);
        let los: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(LOS_EFFECT);
        let s = slow(craft) != 0;
        let l = los(craft);
        let spy = l != 0 && field(l, 4) != 0 && cast(field(field(l, 4), 0x18), TD_ORDNANCE_CLASS, TD_ROMULAN_SPY_CLASS) != 0;
        (s, spy)
    }
}

/// The sprites of those two icons.
pub fn effect_sprites() -> (usize, usize) {
    (super::palette::find_sprite("gravmine_icon"), super::palette::find_sprite("romspy_icon"))
}

/// The race emblem of `obj` (`<race>_icon`), or 0.
pub fn race_icon(obj: usize) -> usize {
    let race = field(obj, OBJ_RACE);
    if race == 0 {
        return 0;
    }
    let name = latin1(race + RACE_NAME);
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
    let w = child(WIREFRAME);
    if w != 0 {
        unsafe { *((w + 0x28) as *mut usize) = obj };
    }
    s.class = text_of(child(CLASS_TEXT), obj, false);
    if fl & F_CRAFT != 0 {
        s.craft = true;
        s.race_icon = race_icon(obj);
        s.name = text_of(child(NAME_TEXT), obj, false);
        if f32_at(obj, OBJ_CREW) > 0.1 {
            s.stats.extend(text_of(child(CREW_TEXT), obj, true));
        }
        if is_planet(obj) {
            s.planet = true;
            s.stats.extend(text_of(child(POPULATION_TEXT), obj, false));
            s.stats.extend(text_of(child(PLANET_OWNER_TEXT), obj, false));
            if let Some(b) = bar_of(child(METAL_BAR), obj) {
                s.bars.push((BarKind::Metal, b));
            }
            if unsafe { rd::<usize>(obj + PLANET_SYSTEMS) } != 0 {
                for i in 0..2 {
                    s.systems.extend(system(obj, i));
                }
                s.stats.extend(text_of(child(OFFICER_TEXT), obj, true));
                if let Some(b) = bar_of(child(CONSTRUCTION_BAR), obj) {
                    s.bars.push((BarKind::Construction, b));
                }
            }
            return s;
        }
        if field(field(obj, OBJ_CLASS), CLASS_OFFICERS) as i32 > 0 {
            s.stats.extend(text_of(child(OFFICER_TEXT), obj, true));
        }
        s.hull = bar_of(child(HULL_BAR), obj);
        if let Some(h) = &s.hull {
            s.shields_down = h.colour[0] == 0.0 && h.colour[1] == 0.0 && h.colour[2] >= 1.0;
        }
        let max = f32_at(obj, CRAFT_SHIELD_MAX);
        if max > 0.0 {
            s.shields = Some((f32_at(obj, CRAFT_SHIELD), max));
        }
        let ib = child(SD_INFOBAR);
        s.hull_text = text_of(field(ib, IB_HULL_TEXT), obj, true);
        s.shield_text = text_of(field(ib, IB_SHIELD_TEXT), obj, true);
        if fl & (F_CARGO | F_CARGO2) != 0 {
            s.stats.extend(text_of(child(CARGO_TEXT), obj, false));
            if let Some(b) = bar_of(child(CARGO_BAR), obj) {
                s.bars.push((BarKind::Cargo, b));
            }
        } else if fl & F_PRODUCER != 0 || cast(obj, TD_CRAFT, TD_COLONY) != 0 {
            if let Some(b) = bar_of(child(CONSTRUCTION_BAR), obj) {
                s.bars.push((BarKind::Construction, b));
            }
            s.stats.extend(text_of(child(RESEARCH_TEXT), obj, false));
        }
        if f32_at(obj, OBJ_ENERGY) > 0.1 && f32_at(obj, OBJ_ENERGY) as i32 > 0 {
            s.stats.extend(text_of(child(ENERGY_TEXT), obj, true));
        }
        for i in 0..N_SYSTEMS {
            s.systems.extend(system(obj, i));
        }
    } else {
        if unsafe { rd::<usize>(obj + OBJ_STORE) } != 0 {
            if let Some(b) = bar_of(child(CARGO_BAR), obj) {
                s.bars.push((BarKind::Cargo, b));
            }
        } else if fl & F_CONSTRUCTION != 0 {
            if let Some(b) = bar_of(child(CONSTRUCTION_BAR), obj) {
                s.bars.push((BarKind::Construction, b));
            }
        }
        if fl & F_NAMED != 0 {
            s.name = text_of(child(NAME_TEXT), obj, false);
        }
    }
    s
}

fn system(obj: usize, i: usize) -> Option<System> {
    let text = text_of(child(SYS_TEXT + 4 * i), obj, true)?;
    let ri = child(SYS_REPAIR + 4 * i);
    let (icon, sys, flash) = if ri != 0 {
        unsafe {
            *((ri + 0x28) as *mut usize) = obj;
            (field(ri, RI_SPRITE), rd::<u32>(ri + RI_SYSTEM) as usize, rd::<u8>(ri + RI_FLASH) != 0)
        }
    } else {
        (0, i, false)
    };
    let _ = RI_REPAIR_ID;
    Some(System { text, repair_icon: icon, repair_comp: ri, state: system_state(obj, sys).unwrap_or_default(), flash })
}

/// The hull bar of any object (for group icons): value, maximum, stock's colour.
pub fn hull(obj: usize) -> Option<Bar> {
    bar_of(child(HULL_BAR), obj)
}

/// A unit's name and class, as the panel words them.
pub fn name_and_class(obj: usize) -> (Option<Text>, Option<Text>) {
    let name = if flags(obj) & (F_CRAFT | F_NAMED) != 0 { text_of(child(NAME_TEXT), obj, false) } else { None };
    (name, text_of(child(CLASS_TEXT), obj, false))
}

/// The hover strip's shield and hull texts for a craft.
pub fn hover_texts(obj: usize) -> (Option<Text>, Option<Text>) {
    let ib = child(SD_INFOBAR);
    (text_of(field(ib, IB_SHIELD_TEXT), obj, false), text_of(field(ib, IB_HULL_TEXT), obj, false))
}

// ---- the builder's queue -------------------------------------------------------------

/// The builder's queue: each slot's class (0: free), and stock's slot component for
/// its tooltip (handed the builder and the class, as stock's `Render` does).
pub fn queue(obj: usize) -> [(usize, usize); QUEUE_LEN] {
    let mut out = [(0usize, 0usize); QUEUE_LEN];
    if obj == 0 || flags(obj) & F_PRODUCER == 0 {
        return out;
    }
    let f: unsafe extern "thiscall" fn(usize) -> *const usize = unsafe { core::mem::transmute(GET_BUILD_QUEUE) };
    let q = unsafe { f(obj) };
    if q.is_null() {
        return out;
    }
    for (i, o) in out.iter_mut().enumerate() {
        let class = unsafe { *q.add(i) };
        let comp = child(QUEUE + 4 * i);
        if comp != 0 {
            unsafe {
                *((comp + BW_BUILDER) as *mut usize) = obj;
                *((comp + BW_CLASS) as *mut usize) = class;
            }
        }
        *o = (class, comp);
    }
    out
}

/// The build progress of the queue's first item (the panel's construction bar).
pub fn build_progress(obj: usize) -> Option<Bar> {
    bar_of(child(CONSTRUCTION_BAR), obj)
}

/// Cancels the queue's slot `i`, as a click on it does.
pub fn cancel(obj: usize, i: usize) {
    if obj == 0 || flags(obj) & F_PRODUCER == 0 {
        return;
    }
    crate::log!("info: cancel queue slot {}", i);
    let f: unsafe extern "thiscall" fn(usize, i32) = unsafe { core::mem::transmute(vcall(obj, VT_DELETE_QUEUE_ITEM)) };
    unsafe { f(obj, i as i32) };
}

// ---- the group ------------------------------------------------------------------------

/// `GameObject+0x113`: set once it is destroyed (stock's panel leaves it out).
const OBJ_DESTROYED: usize = 0x113;

/// The selected objects, in selection order (gone ones left out, as stock's
/// `GetObjUnlessDestroyed`).
pub fn group() -> Vec<usize> {
    selection::handles()
        .into_iter()
        .map(selection::object)
        .filter(|&o| o != 0 && flags(o) & 0x4 != 0 && unsafe { rd::<u8>(o + OBJ_DESTROYED) } == 0)
        .collect()
}

/// `g_pCommandShift`: points to the game's own state of its Shift binding, which
/// `ShipIcon::Simulate` asks.
const COMMAND_SHIFT: usize = 0x761340;

/// Whether the game sees Shift held (its command binding, as stock's click asks).
pub fn shift_held() -> bool {
    let p = unsafe { rd::<usize>(COMMAND_SHIFT) };
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
    let f: unsafe extern "thiscall" fn(usize, usize, i32, i32, i32) = unsafe { core::mem::transmute(vcall(o, VT_SELECT)) };
    unsafe {
        if shift { f(o, obj, 1, 0, 1) } else { f(o, obj, 0, 1, 1) }
    }
}

/// The camera goes to `obj`, as a click on the one-unit panel does.
pub fn look_at(obj: usize) {
    let cam = unsafe { rd::<usize>(CAMERA_MANAGER) };
    if cam == 0 || obj == 0 {
        return;
    }
    crate::log!("info: camera to {:#x}", obj);
    let f: unsafe extern "thiscall" fn(usize, usize) = unsafe { core::mem::transmute(vcall(cam, VT_SET_INTEREST_ENTITY)) };
    unsafe { f(cam, obj) };
}

const SLOW_START: [u8; 6] = [0x56, 0x57, 0x8b, 0xf9, 0x8b, 0x47];
const LOS_START: [u8; 6] = [0x56, 0x57, 0x8b, 0xf9, 0x8b, 0x47];

/// Every function above that is called, checked by its first bytes.
pub fn functions_stock() -> bool {
    super::starts(DETERMINE_TYPE, &[0x56, 0x8b, 0xf1, 0xc7, 0x86, 0xe8, 0x01, 0x00, 0x00])
        && super::starts(REPAIR_LEFT, &[0x55, 0x8b, 0xec, 0x8b, 0x4d, 0x08, 0x80, 0x39])
        && super::starts(GET_BUILD_QUEUE, &[0x56, 0x8b, 0xf1, 0x8b, 0x06, 0xff, 0x90, 0x68])
        && super::starts(LOCALIZE, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08, 0x8b, 0xc1])
        && super::starts(RT_DYNAMIC_CAST, &[0xff, 0x25, 0x70, 0x80, 0x7b, 0x00])
        && super::starts(SLOW_EFFECT, &SLOW_START)
        && super::starts(LOS_EFFECT, &LOS_START)
        && super::selection::functions_stock()
}
