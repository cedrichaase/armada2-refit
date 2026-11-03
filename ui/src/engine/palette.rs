//! The button bar (`PopupPaletteImp`, `g_pPopupPalette`): what it shows, pressing its
//! buttons, and its keys. Read by `hud::bar` (HUD.md, "The button bar").
//!
//! The bar's `Update` (in its `AlwaysGameSimulate`) works out the buttons for what is
//! selected and the menu open (`mUpdateButtonsAndPosition`), and keeps them in 21
//! `ControlButton`s at `+0x28`. A button is shown when its `IsValid` (vtable `+0x1c`)
//! says so, which is when `Render` draws it. Each holds a `CommandInfoClass` (`+0x88`)
//! or a `ModeInfo` (`+0x84`), its state (`+0x34`: 0 disabled, 2 active, else normal)
//! and, once drawn, its icon sprite (`+0x2c`, looked up by the name the command or
//! mode gives: `ControlButton::Render`). The rest is grid/README.md's: the names, the
//! hotkey tables, the menu toggles and the press.
use super::{field as peek, rd};
use crate::syms_hud::{field, hook, sym, value, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// The address of `s` when the bar can be read (`hud.reads_bar`), else 0.
fn a(s: a2engine::Sym) -> usize {
    if eng().ready(hook::HUD_READS_BAR).is_ok() { eng().addr(s) } else { 0 }
}
use alloc::string::String;

// `PopupPaletteImp+0x28` holds `ControlButton *[21]`; `+0x110` is the palette mode (1 the
// bar, 2 the build-locked bar, else floating) and `+0x124` the menu shown (`eMenuType`:
// 0 the top level, 1 orders, 2..4 build, evolve, research, 8 the top level again after a
// menu closed). The hotkey tables' counts (commands `+0xcc`, build items `+0xd8`,
// special weapons `+0xe4`) are what `mProcessKeyboardInput` walks. `Init` keeps six menu
// toggles' control pointers in a global (popup, build, orders, trade, AI, formations).
pub const N_SLOTS: usize = value::PALETTE_N_SLOTS as usize;
const N_TOGGLES: usize = value::PALETTE_N_TOGGLES as usize;
const KEY_COUNTS: [usize; 3] = [field::PALETTE_KEY_COMMANDS, field::PALETTE_KEY_BUILDS, field::PALETTE_KEY_WEAPONS];

// `ControlButton`: its state `+0x34` (0 disabled, 2 active, else normal), its icon sprite
// `+0x2c`, a `ModeInfo` `+0x84` or `CommandInfoClass` `+0x88`; vtable slots GetTooltip
// (+0), GetVerboseTooltip (+4), IsValid (+0x1c) and the press (+0x20).
//
// `CommandInfoClass`: its `buttonName` inline at `+0x20`, `preferredPosition` (column,
// row) at `+0x1b0`. `ModeInfo`: its kind at `+0x04` (1 a build item, its class at `+0x0c`
// with the ODF name at `+0x7c`; 2 a special weapon, the weapon class at `+0x10` with its
// `cPrjID` at `+0x208`; 3 a fixed button, `+0x14` indexing the names in a global table,
// 16 bytes apart).
//
// `CommandInfoClass::ButtonName(std::string &) const` and `ModeInfo::ButtonName` give the
// name of the button's sprite; `DisplayInterface::m_spriteDatabase` and
// `ST3D_Database::FindElement(name, class)` find it.

pub fn palette() -> usize {
    super::hud::palette()
}

pub fn mode(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + field::PALETTE_MODE) } }
}

pub fn menu(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + field::PALETTE_MENU) } }
}

/// The `i`-th of the bar's 21 buttons (0 if there is none).
pub fn slot(p: usize, i: usize) -> usize {
    if i >= N_SLOTS { 0 } else { peek(p, field::PALETTE_SLOTS + 4 * i) }
}

/// Whether the button is shown (`IsValid`).
pub fn valid(b: usize) -> bool {
    if b == 0 {
        return false;
    }
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + field::PALETTE_VT_IS_VALID)) };
    unsafe { f(b) }
}

/// 0: disabled (a click does nothing); 2: active (the alert level in force, ...).
pub fn state(b: usize) -> u32 {
    if b == 0 { 0 } else { unsafe { rd(b + field::PALETTE_BTN_STATE) } }
}

/// The button's icon sprite. `ControlButton::Render` looks it up each time it draws:
/// the command's or mode's `ButtonName` names it, and it is found by that name in the
/// HUD's sprite database (`DisplayInterface::m_spriteDatabase`, `FindElement`, as
/// `StandardButton::SetImages` does). The stock bar no longer draws, so we look it up
/// the same way, once per name. 0 when the button names no sprite.
pub fn sprite(b: usize) -> usize {
    let cmd = peek(b, field::PALETTE_BTN_COMMAND);
    let mode = peek(b, field::PALETTE_BTN_MODE);
    let (obj, f) = if cmd != 0 {
        (cmd, a(sym::PALETTE_CMD_BUTTON_NAME))
    } else if mode != 0 {
        (mode, a(sym::PALETTE_MODE_BUTTON_NAME))
    } else {
        return peek(b, field::PALETTE_BTN_SPRITE);
    };
    if f == 0 {
        return 0;
    }
    let Some(name) = super::msvc::with_string(|s| unsafe {
        let g: unsafe extern "thiscall" fn(usize, usize) -> bool = core::mem::transmute(f);
        g(obj, s);
    }) else {
        return 0;
    };
    find_sprite(&name)
}

static mut SPRITES: alloc::vec::Vec<(String, usize)> = alloc::vec::Vec::new();
static mut SPRITES_DB: usize = 0;

/// A sprite of the HUD's sprite database by name, cached while the database is the same.
#[allow(static_mut_refs)]
pub fn find_sprite(name: &str) -> usize {
    unsafe {
        let (db_at, find_at) = (a(sym::PALETTE_SPRITE_DB), a(sym::PALETTE_FIND_ELEMENT));
        if db_at == 0 || find_at == 0 {
            return 0;
        }
        let db = rd::<usize>(db_at);
        if db == 0 {
            return 0;
        }
        let cache = &mut SPRITES;
        if SPRITES_DB != db {
            cache.clear();
            SPRITES_DB = db;
        }
        if let Some((_, sp)) = cache.iter().find(|(n, _)| n == name) {
            return *sp;
        }
        let c = crate::rt::cstr(name);
        let find: unsafe extern "thiscall" fn(usize, *const u8, u32) -> usize = core::mem::transmute(find_at);
        let sp = find(db, c.as_ptr(), 0);
        cache.push((String::from(name), sp));
        sp
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    None,
    Command,
    Build,
    Weapon,
    Menu,
}

/// What a button is: its kind and name ("stop", "bbase", "back", ...), and for a
/// command its preferred position in a three-column plan.
pub struct Desc {
    pub kind: Kind,
    pub name: String,
    pub pos: Option<(i32, i32)>,
}

pub fn describe(b: usize) -> Desc {
    let mut d = Desc { kind: Kind::None, name: String::new(), pos: None };
    let cmd = peek(b, field::PALETTE_BTN_COMMAND);
    let mode = peek(b, field::PALETTE_BTN_MODE);
    if eng().ready(hook::HUD_READS_BAR).is_err() {
        return d;
    }
    unsafe {
        if cmd != 0 {
            d.kind = Kind::Command;
            d.name.push_str(super::hud::cstr_static((cmd + field::PALETTE_CMD_NAME) as *const u8));
            let p: [i32; 2] = rd(cmd + field::PALETTE_CMD_POS);
            d.pos = Some((p[0], p[1]));
            return d;
        }
        if mode == 0 {
            return d;
        }
        match rd::<u32>(mode + field::PALETTE_MODE_KIND) {
            1 => {
                d.kind = Kind::Build;
                d.name.push_str(super::hud::cstr_static(peek(peek(mode, field::PALETTE_MODE_CLASS), field::PALETTE_CLASS_ODF) as *const u8));
            }
            2 => {
                d.kind = Kind::Weapon;
                let id = peek(peek(mode, field::PALETTE_MODE_WEAPON), field::PALETTE_WEAPON_PRJID);
                if id != 0 {
                    let mut buf = [0u8; 80];
                    let get_name = a(sym::PALETTE_PRJ_GET_NAME);
                    if get_name != 0 {
                        let f: unsafe extern "thiscall" fn(usize, *mut u8) = core::mem::transmute(get_name);
                        f(id, buf.as_mut_ptr());
                        d.name.push_str(super::hud::cstr_static(buf.as_ptr()));
                    }
                }
            }
            3 => {
                d.kind = Kind::Menu;
                let i = rd::<u32>(mode + field::PALETTE_MODE_INDEX) as usize;
                let names = a(sym::PALETTE_MODE_NAMES);
                if names != 0 && i < value::PALETTE_NAMES_COUNT as usize {
                    d.name.push_str(super::hud::cstr_static(rd::<usize>(names + value::PALETTE_NAMES_STRIDE as usize * i) as *const u8));
                }
            }
            _ => {}
        }
    }
    d
}

/// The button's tooltip as the game words it (`ControlButton::GetTooltip`, or the
/// longer `GetVerboseTooltip`): the command's name and hotkey, a build item's cost.
pub fn tooltip(b: usize, verbose: bool) -> Option<String> {
    if b == 0 || !super::msvc::available() || eng().ready(hook::HUD_READS_BAR).is_err() {
        return None;
    }
    let slot = if verbose { field::PALETTE_VT_VERBOSE } else { field::PALETTE_VT_TOOLTIP };
    super::msvc::with_stringstream(|ss| unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(rd::<usize>(rd::<usize>(b) + slot));
        f(b, ss)
    })
}

/// Presses the button as releasing a click on it does (`StandardButton::Simulate`):
/// nothing if it is disabled, else the click sound, unless a video plays, and the
/// button's press function, which hands its command or mode to the bar.
pub fn press(b: usize) -> bool {
    if b == 0 || state(b) == 0 || eng().ready(hook::HUD_READS_BAR).is_err() {
        return false;
    }
    if !super::hud::video_playing() {
        super::sound::click();
    }
    let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + field::PALETTE_VT_PRESS)) };
    unsafe { f(b) };
    true
}

// ---- the keys ----------------------------------------------------------------------

// In `PopupPaletteImp::Update`: `call mProcessKeyboardInput`, made when something of the
// player's is selected and the game is not paused (`hud.barkeys`).

static mut STOCK_KEYS_OFF: bool = false;
static mut ON_KEYS: Option<fn(usize)> = None;
static ZERO: i32 = 0;

/// The bar's keys are the grid's (`stock_keys_off`) or the engine's, from now on.
pub fn set_stock_keys_off(on: bool) {
    unsafe { STOCK_KEYS_OFF = on };
}

/// Wraps the bar's keyboard step: with `stock_keys_off`, stock's bar keys do nothing
/// (the three hotkey tables are emptied for the stock call and put back, and the six
/// menu toggles point at a zero), and `on_keys` runs after it, on the game's thread in
/// the bar's own update. False, with nothing written, when the site is not stock (it
/// is not while GridLayout.asi is loaded: it wraps the same call).
pub fn install_keys(stock_keys_off: bool, on_keys: fn(usize)) -> bool {
    // Not stock while GridLayout.asi is loaded: it wraps the same call.
    if eng().ready(hook::HUD_BARKEYS).is_err() || eng().ready(hook::HUD_READS_BAR).is_err() {
        return false;
    }
    // `ControlButton::mButtonPressFunction`, which a button's press slot leads to, was
    // checked with the rest of `hud.reads_bar`: the press below hands its command to the bar.
    let _ = eng().addr(sym::PALETTE_BUTTON_PRESS);
    unsafe {
        STOCK_KEYS_OFF = stock_keys_off;
        ON_KEYS = Some(on_keys);
    }
    super::hook::patch_call(eng().addr(sym::PALETTE_KEYS_CALL), eng().addr(sym::PALETTE_PROCESS_KEYS), process_keys as *const () as usize)
}

unsafe extern "thiscall" fn process_keys(p: usize, crafts: usize) {
    let stock: unsafe extern "thiscall" fn(usize, usize) = unsafe { core::mem::transmute(eng().addr(sym::PALETTE_PROCESS_KEYS)) };
    unsafe {
        if STOCK_KEYS_OFF {
            let toggles = eng().addr(sym::PALETTE_TOGGLES);
            for i in 0..N_TOGGLES {
                *((toggles + 4 * i) as *mut *const i32) = &ZERO;
            }
            let saved = KEY_COUNTS.map(|o| rd::<i32>(p + o));
            for o in KEY_COUNTS {
                *((p + o) as *mut i32) = 0;
            }
            stock(p, crafts);
            for (o, v) in KEY_COUNTS.iter().zip(saved) {
                *((p + o) as *mut i32) = v;
            }
        } else {
            stock(p, crafts);
        }
        if let Some(f) = ON_KEYS {
            f(p);
        }
    }
}
