//! The button bar (`PopupPaletteImp`, `g_pPopupPalette`): what it shows, pressing its
//! buttons. Read by `hud::bar` (HUD.md, "The button bar").
//!
//! The bar's `Update` (in its `AlwaysGameSimulate`) works out the buttons for what is
//! selected and the menu open (`mUpdateButtonsAndPosition`), and keeps them in
//! `ControlButton`s at `+0x28`: 21 inline on stock, behind a pointer on Roots (up to 64:
//! the profile's `palette_n_slots` and `palette_slots_indirect`). A button is shown when its `IsValid` (vtable `+0x1c`)
//! says so, which is when `Render` draws it. Each holds a `CommandInfoClass` (`+0x88`)
//! or a `ModeInfo` (`+0x84`), its state (`+0x34`: 0 disabled, 2 active, else normal)
//! and, once drawn, its icon sprite (`+0x2c`, looked up by the name the command or
//! mode gives: `ControlButton::Render`). Its keys are QOL's (`qol/bar.c`).
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

// `PopupPaletteImp+0x28` holds `ControlButton *[21]` (stock; Roots: a pointer to the array); `+0x110` is the palette mode (1 the
// bar, 2 the build-locked bar, else floating) and `+0x124` the menu shown (`eMenuType`:
// 0 the top level, 1 orders, 2..4 build, evolve, research, 8 the top level again after a
// menu closed). The hotkey tables' counts (commands `+0xcc`, build items `+0xd8`,
// special weapons `+0xe4`) and the six menu toggles are what the bar's keys walk, and are
// QOL's now (`qol/bar.c`, "QOL-1").
#[inline(always)]
pub fn n_slots() -> usize { value::PALETTE_N_SLOTS() as usize }

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

/// Whether the bar can be read (`hud.reads_bar`).
pub fn readable() -> bool {
    eng().ready(hook::HUD_READS_BAR).is_ok()
}

pub fn palette() -> usize {
    super::hud::palette()
}

pub fn mode(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + field::PALETTE_MODE()) } }
}

pub fn menu(p: usize) -> u32 {
    if p == 0 { 0 } else { unsafe { rd(p + field::PALETTE_MENU()) } }
}

/// Whether the button array sits behind a pointer held at `+0x28` (the profile's
/// `palette_slots_indirect`: Roots) rather than inline there (stock).
#[inline(always)]
pub fn slots_indirect() -> bool { value::PALETTE_SLOTS_INDIRECT() != 0 }

/// The `i`-th of the bar's buttons (0 if there is none): of `n_slots()`, 21 inline on
/// stock, 64 behind a pointer on Roots (both from the build's profile).
pub fn slot(p: usize, i: usize) -> usize {
    if i >= n_slots() {
        return 0;
    }
    if slots_indirect() {
        let base = peek(p, field::PALETTE_SLOTS());
        if base == 0 { 0 } else { peek(base, 4 * i) }
    } else {
        peek(p, field::PALETTE_SLOTS() + 4 * i)
    }
}

/// Whether the button is shown (`IsValid`).
pub fn valid(b: usize) -> bool {
    if b == 0 {
        return false;
    }
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + field::PALETTE_VT_IS_VALID())) };
    unsafe { f(b) }
}

/// 0: disabled (a click does nothing); 2: active (the alert level in force, ...).
pub fn state(b: usize) -> u32 {
    if b == 0 { 0 } else { unsafe { rd(b + field::PALETTE_BTN_STATE()) } }
}

/// The button's icon sprite. `ControlButton::Render` looks it up each time it draws:
/// the command's or mode's `ButtonName` names it, and it is found by that name in the
/// HUD's sprite database (`DisplayInterface::m_spriteDatabase`, `FindElement`, as
/// `StandardButton::SetImages` does). The stock bar no longer draws, so we look it up
/// the same way, once per name. 0 when the button names no sprite.
pub fn sprite(b: usize) -> usize {
    let cmd = peek(b, field::PALETTE_BTN_COMMAND());
    let mode = peek(b, field::PALETTE_BTN_MODE());
    let (obj, f) = if cmd != 0 {
        (cmd, a(sym::PALETTE_CMD_BUTTON_NAME))
    } else if mode != 0 {
        (mode, a(sym::PALETTE_MODE_BUTTON_NAME))
    } else {
        return peek(b, field::PALETTE_BTN_SPRITE());
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
    let cmd = peek(b, field::PALETTE_BTN_COMMAND());
    let mode = peek(b, field::PALETTE_BTN_MODE());
    if eng().ready(hook::HUD_READS_BAR).is_err() {
        return d;
    }
    unsafe {
        if cmd != 0 {
            d.kind = Kind::Command;
            d.name.push_str(super::hud::cstr_static((cmd + field::PALETTE_CMD_NAME()) as *const u8));
            let p: [i32; 2] = rd(cmd + field::PALETTE_CMD_POS());
            d.pos = Some((p[0], p[1]));
            return d;
        }
        if mode == 0 {
            return d;
        }
        match rd::<u32>(mode + field::PALETTE_MODE_KIND()) {
            1 => {
                d.kind = Kind::Build;
                d.name.push_str(super::hud::cstr_static(peek(peek(mode, field::PALETTE_MODE_CLASS()), field::PALETTE_CLASS_ODF()) as *const u8));
            }
            2 => {
                d.kind = Kind::Weapon;
                let id = peek(peek(mode, field::PALETTE_MODE_WEAPON()), field::PALETTE_WEAPON_PRJID());
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
                let i = rd::<u32>(mode + field::PALETTE_MODE_INDEX()) as usize;
                let names = a(sym::PALETTE_MODE_NAMES);
                if names != 0 && i < value::PALETTE_NAMES_COUNT() as usize {
                    d.name.push_str(super::hud::cstr_static(rd::<usize>(names + value::PALETTE_NAMES_STRIDE() as usize * i) as *const u8));
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
    let slot = if verbose { field::PALETTE_VT_VERBOSE() } else { field::PALETTE_VT_TOOLTIP() };
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
    let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(b) + field::PALETTE_VT_PRESS())) };
    unsafe { f(b) };
    true
}

/// The icon sprite the button itself holds (`+0x2c`), whatever the name lookup finds: what a button
/// whose command or weapon names no sprite of the HUD's database may still carry (FO-6 step 6).
pub fn own_sprite(b: usize) -> usize {
    if b == 0 { 0 } else { peek(b, field::PALETTE_BTN_SPRITE()) }
}

/// Whether the button's hover byte is set: `StandardButton::Simulate` sets it from the pointer and the
/// button's rectangle whoever owns the pointer (FO-6 step 5), and FleetOpsHook's range ring follows it.
pub fn hovered(b: usize) -> bool {
    b != 0 && unsafe { rd::<u8>(b + field::PALETTE_BTN_HOVER()) } != 0
}

/// Whether the button's pressed byte is set (the mouse is held over an enabled button).
pub fn pressed(b: usize) -> bool {
    b != 0 && unsafe { rd::<u8>(b + field::PALETTE_BTN_PRESSED()) } != 0
}

/// The button's rectangle in canvas units (left, top, right, bottom, inclusive), local to its owner's
/// origin, which is the bar's own rectangle's top left (FO-6 step 6).
pub fn local_rect(b: usize) -> Option<[i32; 4]> {
    if b == 0 || !super::readable(b, 0x40) {
        return None;
    }
    let r: [i32; 4] = unsafe { rd(b + field::PALETTE_BTN_RECT()) };
    if r[2] < r[0] || r[3] < r[1] { None } else { Some(r) }
}

// ---- the grid of ours on Roots (PLAN-FLEETOPS.md, FO-6, decisions 330 to 349) -----------
//
// FleetOpsHook's range ring follows a weapon button's mouse-over flag, which the engine's
// `StandardComponent::Simulate` sets from the pointer and the button's own rectangle, and only
// for the bar while the bar owns the pointer. So the grid moves the stock buttons, not the
// pointer: right after the bar's `Update` has laid them out (its call of `IsGamePausedAtAll`,
// `hud.bar_grid`), each shown button's rectangle is set to its cell of ours, and
// `hud::cursor_over` gives the bar the pointer over the grid's frame. The buttons then simulate
// where they are drawn: hover, press, rollover, tooltip and ring are the engine's and
// FleetOpsHook's own. The layout writes the rectangles again at every update, so nothing of
// ours outlives a frame without the grid. Nothing of FleetOpsHook's memory is written.

static mut GRID_AFTER_LAYOUT: Option<fn(usize)> = None;

/// Wraps the bar's `Update` call of `IsGamePausedAtAll` (`hud.bar_grid`), on a build whose bar
/// is behind a pointer (Roots) only: `after_layout(palette)` runs right after the stock call,
/// once per update, on the game's thread. False: the grid cannot be had (the skin is drawn).
pub fn install_grid(after_layout: fn(usize)) -> bool {
    if !slots_indirect() || eng().ready(hook::HUD_BAR_GRID).is_err() || eng().ready(hook::HUD_READS_BAR).is_err() {
        return false;
    }
    unsafe { GRID_AFTER_LAYOUT = Some(after_layout) };
    let (site, paused) = (eng().addr(sym::PALETTE_UPDATE_PAUSED_CALL), eng().addr(sym::POPUPS_IS_GAME_PAUSED));
    super::hook::patch_call(site, paused, update_paused as *const () as usize)
}

extern "C" fn update_paused() -> bool {
    let f: extern "C" fn() -> bool = unsafe { core::mem::transmute(eng().addr(sym::POPUPS_IS_GAME_PAUSED)) };
    let paused = f();
    if let Some(cb) = unsafe { GRID_AFTER_LAYOUT } {
        let p = palette();
        if p != 0 {
            cb(p);
        }
    }
    paused
}

/// The bar's own origin in canvas units (its rectangle's top left), which a button's rectangle
/// is local to.
pub fn origin(p: usize) -> Option<(i32, i32)> {
    if p == 0 || !super::readable(p, 0x40) {
        return None;
    }
    let r: [i32; 4] = unsafe { rd(p + field::HUD_DI_RECT()) };
    Some((r[0], r[1]))
}

/// Sets a shown button's rectangle (canvas units, local to the bar's origin, inclusive), where
/// its `Simulate` looks for the pointer next.
pub fn set_rect(b: usize, r: [i32; 4]) {
    if b == 0 || !super::readable(b, 0x40) || eng().ready(hook::HUD_BAR_GRID).is_err() {
        return;
    }
    unsafe { core::ptr::write_unaligned((b + field::PALETTE_BTN_RECT()) as *mut [i32; 4], r) };
}

// ---- the bar measured (PLAN-FLEETOPS.md, FO-6 step 5) -----------------------------------
//
// Where the bar is not the stock one (`palette_slots_indirect`: Roots, whose FleetOpsHook
// owns the array, the slot plan and the range rings that follow the pointer), the UI draws
// nothing of it but logs what the engine's own objects say: the mode and its columns and
// rows, the buttons shown and what each is, and the button under the pointer with the
// states `StandardButton::Simulate` keeps (hover, pressed) and who owns the pointer. Each
// log line is once per change and the counts are capped; nothing is written to the engine.

static mut M_LAYOUT_SIG: u64 = 0;
static mut M_HOVER_SIG: u64 = 0;
static mut M_LAYOUT_LINES: u32 = 0;
static mut M_HOVER_LINES: u32 = 0;
const M_LAYOUT_MAX: u32 = 40;
const M_HOVER_MAX: u32 = 120;

fn fnv(h: &mut u64, v: u64) {
    *h = (*h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
}

fn sane(p: usize) -> bool {
    p >= 0x10000 && p < 0x8000_0000 && p % 4 == 0 && super::readable(p, 0x40)
}

/// What a button is, as a number that does not change when the bar rebuilds its `CommandInfoClass`
/// and `ModeInfo` objects (they are new each time): a command by the bytes of its name, a mode by
/// its kind and the class, weapon or fixed-button index it names.
fn identity(b: usize) -> u64 {
    let cmd = peek(b, field::PALETTE_BTN_COMMAND());
    let mode = peek(b, field::PALETTE_BTN_MODE());
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    if sane(cmd) {
        let name: [u8; 16] = unsafe { rd(cmd + field::PALETTE_CMD_NAME()) };
        for c in name {
            fnv(&mut h, c as u64);
        }
    } else if sane(mode) {
        let kind = unsafe { rd::<u32>(mode + field::PALETTE_MODE_KIND()) };
        fnv(&mut h, kind as u64);
        fnv(&mut h, match kind {
            1 => peek(mode, field::PALETTE_MODE_CLASS()),
            2 => { let w = peek(mode, field::PALETTE_MODE_WEAPON()); if sane(w) { peek(w, field::PALETTE_WEAPON_PRJID()) } else { 0 } }
            _ => unsafe { rd::<u32>(mode + field::PALETTE_MODE_INDEX()) as usize },
        } as u64);
    }
    h
}

/// Logs the bar when what it shows or what is under the pointer has changed.
#[allow(static_mut_refs)]
pub fn log_measure() {
    if !slots_indirect() || !readable() || unsafe { M_LAYOUT_LINES >= M_LAYOUT_MAX && M_HOVER_LINES >= M_HOVER_MAX } {
        return;
    }
    let p = palette();
    if p == 0 || !sane(p) || !super::readable(p, 0x130) {
        return;
    }
    let base = peek(p, field::PALETTE_SLOTS());
    if !sane(base) || !super::readable(base, 4 * n_slots()) {
        return;
    }
    // The buttons shown, by a cheap pass over raw values; names only when something changed.
    let mut shown: alloc::vec::Vec<(usize, usize)> = alloc::vec::Vec::new();
    let (mut lsig, mut hsig) = (0xcbf2_9ce4_8422_2325u64, 0xcbf2_9ce4_8422_2325u64);
    let mut hover = usize::MAX;
    let mut pressed = usize::MAX;
    for i in 0..n_slots() {
        let b = slot(p, i);
        if !sane(b) || !super::readable(b, 0x90) || !sane(peek(b, 0)) || !valid(b) {
            continue;
        }
        shown.push((i, b));
        fnv(&mut lsig, i as u64);
        fnv(&mut lsig, identity(b));
        fnv(&mut lsig, state(b) as u64);
        if unsafe { rd::<u8>(b + field::PALETTE_BTN_HOVER()) } != 0 && hover == usize::MAX {
            hover = i;
        }
        if unsafe { rd::<u8>(b + field::PALETTE_BTN_PRESSED()) } != 0 && pressed == usize::MAX {
            pressed = i;
        }
    }
    let (m, menu_now) = (mode(p), menu(p));
    let r = |o: usize| -> u32 { unsafe { rd::<u32>(p + o) } };
    let (uc, ur) = (r(field::PALETTE_USE_COLS()), r(field::PALETTE_USE_ROWS()));
    let hidden = unsafe { rd::<u8>(p + field::PALETTE_HIDDEN()) };
    for v in [m as u64, menu_now as u64, uc as u64, ur as u64, hidden as u64] {
        fnv(&mut lsig, v);
    }
    let owner = super::hud::cursor_owner();
    let on_bar = owner == p;
    let mp: [i32; 2] = { let a = eng().addr(sym::HUD_MOUSE_POS); if a == 0 { [0, 0] } else { unsafe { rd(a) } } };
    fnv(&mut hsig, hover as u64);
    fnv(&mut hsig, pressed as u64);
    fnv(&mut hsig, on_bar as u64);
    if hover != usize::MAX {
        fnv(&mut hsig, shown.iter().find(|(i, _)| *i == hover).map_or(0, |&(_, b)| state(b)) as u64);
    }
    unsafe {
        if lsig != M_LAYOUT_SIG && M_LAYOUT_LINES < M_LAYOUT_MAX {
            M_LAYOUT_SIG = lsig;
            M_LAYOUT_LINES += 1;
            let listed = super::hud::component_named("PopupPalette");
            crate::log!(
                "bar: measure mode={} menu={} use={}x{} floating={}x{} staticA={}x{} staticB={}x{} hidden={} shown={} of {} (palette {:#x}, render list {:#x})",
                m, menu_now, uc, ur,
                r(field::PALETTE_FLOAT_COLS()), r(field::PALETTE_FLOAT_ROWS()),
                r(field::PALETTE_STATIC_A_COLS()), r(field::PALETTE_STATIC_A_ROWS()),
                r(field::PALETTE_STATIC_B_COLS()), r(field::PALETTE_STATIC_B_ROWS()),
                hidden, shown.len(), n_slots(), p, listed
            );
            for chunk in shown.chunks(8) {
                let mut line = String::new();
                for &(i, b) in chunk {
                    let d = describe(b);
                    let pos = d.pos.map_or(String::new(), |(c, r)| crate::sfmt!("@{},{}", c, r));
                    line.push_str(&crate::sfmt!(" {}:{:?}:{}{}:s{}", i, d.kind, d.name, pos, state(b)));
                }
                crate::log!("bar:  slots{}", line);
            }
        }
        if hsig != M_HOVER_SIG && M_HOVER_LINES < M_HOVER_MAX {
            M_HOVER_SIG = hsig;
            M_HOVER_LINES += 1;
            let who = if owner == 0 { "none" } else if on_bar { "the bar" } else { super::hud::name(owner) };
            match shown.iter().find(|(i, _)| *i == hover) {
                Some(&(i, b)) => {
                    let d = describe(b);
                    let rc: [i32; 4] = rd(b + field::PALETTE_BTN_RECT());
                    crate::log!(
                        "bar: hover slot {} {:?} {} state={} pressed={} rect=[{},{},{},{}] pointer owner: {} (button owner {:#x}; the game's pointer at {},{} canvas)",
                        i, d.kind, d.name, state(b), pressed != usize::MAX, rc[0], rc[1], rc[2], rc[3], who, peek(b, field::PALETTE_BTN_OWNER()), mp[0], mp[1]
                    );
                }
                None => {
                    let ui = crate::input::state().mouse.map_or(String::from("outside"), |(x, y)| crate::sfmt!("{},{} px", x as i32, y as i32));
                    crate::log!("bar: hover none, pressed={} pointer owner: {} (the game's pointer at {},{} canvas; the window's at {})", pressed != usize::MAX, who, mp[0], mp[1], ui)
                }
            }
        }
    }
}
