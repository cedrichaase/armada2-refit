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
use super::{field, rd};
use alloc::string::String;

/// `PopupPaletteImp+0x28`: `ControlButton *[21]`.
const SLOTS: usize = 0x28;
pub const N_SLOTS: usize = 21;
/// `+0x110`: the palette mode (1 the bar, 2 the build-locked bar, else floating);
/// `+0x124`: the menu shown (`eMenuType`: 0 the top level, 1 orders, 2..4 build,
/// evolve, research, 8 the top level again after a menu closed).
const MODE: usize = 0x110;
const MENU: usize = 0x124;
/// The hotkey tables' counts (commands `+0xcc`, build items `+0xd8`, special weapons
/// `+0xe4`), which `mProcessKeyboardInput` walks.
const KEY_COUNTS: [usize; 3] = [0xcc, 0xd8, 0xe4];
/// `Init` keeps six menu toggles' control pointers here (popup, build, orders, trade,
/// AI, formations).
const TOGGLES: usize = 0x763d3c;
const N_TOGGLES: usize = 6;

/// `ControlButton`.
const BTN_STATE: usize = 0x34;
const BTN_SPRITE: usize = 0x2c;
const BTN_MODE: usize = 0x84;
const BTN_COMMAND: usize = 0x88;
const VT_TOOLTIP: usize = 0x00;
const VT_VERBOSE: usize = 0x04;
const VT_IS_VALID: usize = 0x1c;
const VT_PRESS: usize = 0x20;

/// `CommandInfoClass`: its `buttonName` inline at `+0x20`, `preferredPosition` (column,
/// row) at `+0x1b0`.
const CMD_NAME: usize = 0x20;
const CMD_POS: usize = 0x1b0;
/// `ModeInfo`: its kind at `+0x04` (1 a build item, its class at `+0x0c` with the ODF
/// name at `+0x7c`; 2 a special weapon, the weapon class at `+0x10` with its `cPrjID`
/// at `+0x208`; 3 a fixed button, `+0x14` indexing the names at `MODE_NAMES`).
const MODE_KIND: usize = 0x04;
const MODE_NAMES: usize = 0x709f68;
const PRJ_GET_NAME: usize = 0x65dce0;

/// `ControlButton::mButtonPressFunction` (ControlButton's vtable `+0x20`).
const CONTROL_BUTTON_PRESS: usize = 0x4e69e0;

pub fn palette() -> usize {
    super::hud::palette()
}

pub fn mode(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + MODE) } }
}

pub fn menu(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + MENU) } }
}

/// The `i`-th of the bar's 21 buttons (0 if there is none).
pub fn slot(p: usize, i: usize) -> usize {
    if i >= N_SLOTS { 0 } else { field(p, SLOTS + 4 * i) }
}

/// Whether the button is shown (`IsValid`).
pub fn valid(b: usize) -> bool {
    if b == 0 {
        return false;
    }
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + VT_IS_VALID)) };
    unsafe { f(b) }
}

/// 0: disabled (a click does nothing); 2: active (the alert level in force, ...).
pub fn state(b: usize) -> u32 {
    if b == 0 { 0 } else { unsafe { rd(b + BTN_STATE) } }
}

/// The icon sprite `ControlButton::Render` gave the button (0 until it is first drawn
/// there, or when the command names no sprite).
pub fn sprite(b: usize) -> usize {
    field(b, BTN_SPRITE)
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
    let cmd = field(b, BTN_COMMAND);
    let mode = field(b, BTN_MODE);
    unsafe {
        if cmd != 0 {
            d.kind = Kind::Command;
            d.name.push_str(super::hud::cstr_static((cmd + CMD_NAME) as *const u8));
            let p: [i32; 2] = rd(cmd + CMD_POS);
            d.pos = Some((p[0], p[1]));
            return d;
        }
        if mode == 0 {
            return d;
        }
        match rd::<u32>(mode + MODE_KIND) {
            1 => {
                d.kind = Kind::Build;
                d.name.push_str(super::hud::cstr_static(field(field(mode, 0x0c), 0x7c) as *const u8));
            }
            2 => {
                d.kind = Kind::Weapon;
                let id = field(field(mode, 0x10), 0x208);
                if id != 0 {
                    let mut buf = [0u8; 80];
                    let f: unsafe extern "thiscall" fn(usize, *mut u8) = core::mem::transmute(PRJ_GET_NAME);
                    f(id, buf.as_mut_ptr());
                    d.name.push_str(super::hud::cstr_static(buf.as_ptr()));
                }
            }
            3 => {
                d.kind = Kind::Menu;
                let i = rd::<u32>(mode + 0x14) as usize;
                if i < 9 {
                    d.name.push_str(super::hud::cstr_static(rd::<usize>(MODE_NAMES + 16 * i) as *const u8));
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
    if b == 0 || !super::msvc::available() {
        return None;
    }
    let slot = if verbose { VT_VERBOSE } else { VT_TOOLTIP };
    super::msvc::with_stringstream(|ss| unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(rd::<usize>(rd::<usize>(b) + slot));
        f(b, ss)
    })
}

/// Presses the button as releasing a click on it does (`StandardButton::Simulate`):
/// nothing if it is disabled, else the click sound, unless a video plays, and the
/// button's press function, which hands its command or mode to the bar.
pub fn press(b: usize) -> bool {
    if b == 0 || state(b) == 0 {
        return false;
    }
    if !super::hud::video_playing() {
        super::sound::click();
    }
    let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + VT_PRESS)) };
    unsafe { f(b) };
    true
}

// ---- the keys ----------------------------------------------------------------------

/// In `PopupPaletteImp::Update`: `call mProcessKeyboardInput`, made when something of
/// the player's is selected and the game is not paused.
const SITE_KEYS: usize = 0x4fb56a;
const PROCESS_KEYS: usize = 0x4fb5b0;

static mut STOCK_KEYS_OFF: bool = false;
static mut ON_KEYS: Option<fn(usize)> = None;
static ZERO: i32 = 0;

/// Wraps the bar's keyboard step: with `stock_keys_off`, stock's bar keys do nothing
/// (the three hotkey tables are emptied for the stock call and put back, and the six
/// menu toggles point at a zero), and `on_keys` runs after it, on the game's thread in
/// the bar's own update. False, with nothing written, when the site is not stock (it
/// is not while GridLayout.asi is loaded: it wraps the same call).
pub fn install_keys(stock_keys_off: bool, on_keys: fn(usize)) -> bool {
    if super::hook::call_target(SITE_KEYS) != Some(PROCESS_KEYS) {
        crate::log!("bar: the bar's keyboard call is not stock (GridLayout.asi?)");
        return false;
    }
    if !super::starts(CONTROL_BUTTON_PRESS, &[0x8b, 0xc1, 0x56, 0x8b, 0x90]) || !super::starts(PRJ_GET_NAME, &[0x55, 0x8b, 0xec, 0x56, 0x57]) {
        return false;
    }
    unsafe {
        STOCK_KEYS_OFF = stock_keys_off;
        ON_KEYS = Some(on_keys);
    }
    super::hook::patch_call(SITE_KEYS, PROCESS_KEYS, process_keys as *const () as usize)
}

unsafe extern "thiscall" fn process_keys(p: usize, crafts: usize) {
    let stock: unsafe extern "thiscall" fn(usize, usize) = unsafe { core::mem::transmute(PROCESS_KEYS) };
    unsafe {
        if STOCK_KEYS_OFF {
            for i in 0..N_TOGGLES {
                *((TOGGLES + 4 * i) as *mut *const i32) = &ZERO;
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
