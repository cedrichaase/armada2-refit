//! The multiplayer HUD boxes: the comm panel (`CommDisplay`, `commDisplay`, with a
//! `CommBar` per player slot), the dropped-player box (`DropPlayerBox`,
//! `dropPlayerBox`) and the box that waits for the other players' loads
//! (`LoadPercentageBox`, `loadPercentageBox`). Read by `hud::comm` (HUD.md, "Comms, the
//! drop box and the load wait").
//!
//! Each is a global instance, shown and hidden by the engine (`SetIsHidden`, the
//! component's `+0x16`): the comm panel by the top-right comm button
//! (`ButtonPanel::OpenComm`) and its own OK, the drop box and the load wait by
//! `Program::DisplayInputProcess` while the transport says so. Every field is read
//! only after the object's vtable is checked to be the class's own; every press goes
//! through the stock button's own press function (`buttons::queue`).
use super::{buttons, field as fld, rd};
use crate::syms_panels::{field, hook, sym, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

fn ok() -> bool {
    eng().ready(hook::PANELS_COMM).is_ok()
}

/// `commDisplay`, `dropPlayerBox` and `loadPercentageBox`, the instances (0 while the
/// hook is not ready).
pub fn comm_panel() -> usize {
    if ok() { addr(sym::PANELS_COMM) } else { 0 }
}

pub fn drop_panel() -> usize {
    if ok() { addr(sym::PANELS_DROP) } else { 0 }
}

pub fn load_panel() -> usize {
    if ok() { addr(sym::PANELS_LOAD) } else { 0 }
}

const C_MSG: [usize; 4] = [field::PANELS_C_MSG_0, field::PANELS_C_MSG_1, field::PANELS_C_MSG_2, field::PANELS_C_MSG_3];
const C_ROWS: [usize; 5] = [field::PANELS_C_ROW_0, field::PANELS_C_ROW_1, field::PANELS_C_ROW_2, field::PANELS_C_ROW_3, field::PANELS_C_ROW_4];
const B_ALLIANCE: [usize; 3] = [field::PANELS_B_ALLIANCE_0, field::PANELS_B_ALLIANCE_1, field::PANELS_B_ALLIANCE_2];
const CB_ALLIANCE: [a2engine::Sym; 3] = [sym::PANELS_CB_ENEMY, sym::PANELS_CB_NEUTRAL, sym::PANELS_CB_ALLY];
const CB_MSG: [a2engine::Sym; 4] = [sym::PANELS_CB_MSG_ALL, sym::PANELS_CB_MSG_ALLIES, sym::PANELS_CB_MSG_ENEMIES, sym::PANELS_CB_MSG_PRIVATE];

// ---- the comm panel -------------------------------------------------------------------

// `commDisplay`, an instance, and `CommDisplay`'s vtable.
// Its labels (`StandardText`s made by `CreateLabel` from the GUI config's `comm*`
// keys): the title, the player, message, team and alliance headings, the message and
// give groups' captions, and the allied-victory and latency captions (these two NULL
// when the game has no such toggle).
// Its `StandardButton`s: message all, allies, enemies, the selected player (each
// marks which bars' chat toggles are on, then `Transport::UpdateChatTargets`), give
// the selected units to the selected player, OK (hides the panel).
// One resource row per transferable resource, NULL for one the player's race does
// not have (`Transfer*::UsedBy`): a 16-byte block of the up button, the down button,
// the give button and the amount's text, in that order. Dilithium, metal, latinum,
// crew, biomatter.
/// The `GUI_CP_*_RES` key each row's resource is named by (`Transfer*::GetName`).
const ROW_KEYS: [&core::ffi::CStr; 5] = [c"GUI_CP_DILITHIUM_RES", c"GUI_CP_METAL_RES", c"GUI_CP_LATINUM_RES", c"GUI_CP_CREW_RES", c"GUI_CP_BIOMATTER_RES"];
// A row's buttons hold their `TransferDetails` at `+0x60` (the stock callbacks read
// it there): the amount to give at `+0x40`, its least and most at `+0x48`/`+0x4c`, the
// step of the up and down buttons at `+0x50`.
// The allied-victory and high-latency `ToggleButton`s (NULL when the game has none):
// on when their state is 2.
// The bars: their count (`commBarNumberOfPlayers` in the GUI config) and the array of
// `CommBar *`, one per slot in slot order.
// The setup's relationship type, as `PostLoad` reads it: teams fixed (the team column
// shows), or alliances free (the alliance column shows).
// The selected bar's index (`SetCurrentPlayer`): the player the give buttons and the
// private message go to.

// `CommBar`: `+0x28` it stands for a player (cleared for an empty slot), `+0x2c` its
// slot, `+0x34` the player's team (start location), `+0x38` the team shows, `+0x39`
// the alliance buttons work; the name button (`PlayerSelectCallback`) `+0x3c`, the
// name `+0x40` (a `PlayerNameText`, the name inline at `+0x84`), the chat
// `ToggleButton` `+0x44` (on: the player hears this player's chat), the team's text
// `+0x48` (a `TeamNameText`: its number, from 1, at `+0x84`), the enemy, neutral and
// ally `AllianceButton`s `+0x4c`, `+0x50`, `+0x54` (the relation in force has state
// 2; one proposed but not yet answered has `+0x7c` set), and the colour
// `ColorButton` `+0x58` (`PlayerSelectCallback` as well; the player's colour, three
// floats, at `+0x68`, which stayed white for a computer player: the team's colour is
// read instead).

// A `StandardText`'s GUI config label key (`StandardText::GetLabel`, `+0x54`), which
// its `Render` localises.
// `StandardText::GetInformation` (vtable `+0x24`): the colour and the text a text
// component shows this frame; its subclasses (`DropPlayerText`, `LoadPercentageText`)
// build the text there.

// The panel's own press functions, the stock buttons' `+0x5c`, by which `comm()`
// recognises each button before it is offered to the panel.

/// `GUI_POP_COMM_TEAM_PREFIX`, which `TeamNameText` writes before the team's number.
pub const TEAM_PREFIX: &core::ffi::CStr = c"GUI_POP_COMM_TEAM_PREFIX";

/// A button the panel may press: the button, if its press function is `cb`.
fn checked(b: usize, cb: a2engine::Sym) -> usize {
    if b != 0 && ok() && buttons::press_fn(b) == addr(cb) { b } else { 0 }
}

fn vt_is(o: usize, vt: a2engine::Sym) -> bool {
    o != 0 && ok() && fld(o, 0) == addr(vt)
}

/// A text component's label, localised as its `Render` does.
fn label_text(t: usize) -> String {
    let k = fld(t, field::PANELS_TEXT_LABEL_KEY);
    if t == 0 || k == 0 {
        return String::new();
    }
    let raw = buttons::latin1(k, 128);
    let mut key = alloc::vec::Vec::from(raw.as_bytes());
    key.push(0);
    match core::ffi::CStr::from_bytes_with_nul(&key) {
        Ok(k) => buttons::localize(k),
        Err(_) => raw,
    }
}

/// `GetInformation` of a text component: its colour and its text, as it would draw it.
fn information(t: usize) -> Option<([f32; 3], String)> {
    if t == 0 {
        return None;
    }
    let mut out = [0u32; 4];
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, *mut [u32; 4]) -> *mut [u32; 4] = core::mem::transmute(fld(fld(t, 0), field::PANELS_VT_GET_INFORMATION));
        f(t, &mut out);
    }
    let c = [f32::from_bits(out[0]), f32::from_bits(out[1]), f32::from_bits(out[2])];
    Some((c, buttons::clean(&buttons::latin1(out[3] as usize, 2048))))
}

pub fn comm_shown() -> bool {
    vt_is(comm_panel(), sym::PANELS_VT_COMM) && !super::hud::hidden(comm_panel())
}

/// A player's bar.
pub struct Bar {
    pub slot: usize,
    pub name: String,
    pub team: Option<i32>,
    /// The player's colour, 0xAARRGGBB.
    pub colour: u32,
    pub selected: bool,
    /// The colour button (selects the player), the name button (the same).
    pub select: usize,
    /// The chat toggle, 0 when the player cannot be chatted to (a computer, oneself);
    /// whether it is on.
    pub chat: usize,
    pub chat_on: bool,
    /// Enemy, neutral, ally: each button (0 when the alliances cannot be changed),
    /// whether it is the relation in force, whether it is proposed.
    pub alliance: [(usize, bool, bool); 3],
}

/// A resource the player can give.
pub struct Row {
    pub name: String,
    pub amount: i32,
    pub min: i32,
    pub max: i32,
    pub up: usize,
    pub down: usize,
    pub give: usize,
}

/// The comm panel as it stands.
pub struct Comm {
    pub title: String,
    pub head_player: String,
    pub head_message: String,
    pub head_team: String,
    pub head_alliance: String,
    pub group_message: String,
    pub group_give: String,
    pub label_victory: String,
    pub label_latency: String,
    pub teams: bool,
    pub alliances: bool,
    pub bars: Vec<Bar>,
    /// Message all, allies, enemies, the selected player.
    pub msg: [usize; 4],
    pub give_units: usize,
    pub okay: usize,
    pub rows: Vec<Row>,
    /// The allied-victory and latency toggles (0 when absent) and whether each is on.
    pub victory: (usize, bool),
    pub latency: (usize, bool),
}

fn toggle(t: usize, cb: a2engine::Sym) -> (usize, bool) {
    let b = checked(t, cb);
    (b, b != 0 && buttons::state(b) == 2)
}

pub fn comm() -> Option<Comm> {
    if !vt_is(comm_panel(), sym::PANELS_VT_COMM) {
        return None;
    }
    let c = comm_panel();
    let rd8 = |o: usize| unsafe { rd::<u8>(c + o) } != 0;
    let teams = rd8(field::PANELS_C_TEAMS);
    let alliances = rd8(field::PANELS_C_ALLIANCES);
    let selected = fld(c, field::PANELS_C_SELECTED);
    let mut bars = Vec::new();
    let (n, arr) = (fld(c, field::PANELS_C_BAR_COUNT), fld(c, field::PANELS_C_BARS));
    if arr != 0 && n <= 16 {
        for i in 0..n {
            let b = fld(arr, 4 * i);
            if !vt_is(b, sym::PANELS_VT_BAR) || unsafe { rd::<u8>(b + field::PANELS_B_PRESENT) } == 0 {
                continue;
            }
            let name = buttons::latin1(fld(b, field::PANELS_B_NAME) + field::PANELS_TEXT_INLINE, 64);
            let show_team = unsafe { rd::<u8>(b + field::PANELS_B_SHOW_TEAM) } != 0;
            let team = if show_team { Some(unsafe { rd::<i32>(fld(b, field::PANELS_B_TEAM_TEXT) + field::PANELS_TEXT_INLINE) }) } else { None };
            let cb = fld(b, field::PANELS_B_COLOUR);
            // The colour button's own colour stayed white for a computer player on the
            // bench; the player's team (its start location) has the colour the radar and
            // the setup screen show.
            let rgb = super::radar::team_colour(unsafe { rd::<i32>(b + field::PANELS_B_TEAM) });
            let select = checked(cb, sym::PANELS_CB_PLAYER_SELECT);
            let select = if select != 0 { select } else { checked(fld(b, field::PANELS_B_NAME_BUTTON), sym::PANELS_CB_PLAYER_SELECT) };
            let chat = checked(fld(b, field::PANELS_B_CHAT), sym::PANELS_CB_CHAT);
            let show_alliance = unsafe { rd::<u8>(b + field::PANELS_B_SHOW_ALLIANCE) } != 0;
            let mut alliance = [(0usize, false, false); 3];
            for (k, a) in alliance.iter_mut().enumerate() {
                let ab = fld(b, B_ALLIANCE[k]);
                if ab == 0 {
                    continue;
                }
                let st = buttons::state(ab);
                let pending = unsafe { rd::<u8>(ab + field::PANELS_ALLIANCE_PENDING) } != 0;
                *a = (if show_alliance { checked(ab, CB_ALLIANCE[k]) } else { 0 }, st == 2, pending);
            }
            bars.push(Bar {
                slot: fld(b, field::PANELS_B_SLOT),
                name,
                team,
                colour: buttons::colour(rgb),
                selected: i == selected,
                select,
                chat: if chat != 0 && buttons::enabled(chat) { chat } else { 0 },
                chat_on: chat != 0 && buttons::state(chat) == 2,
                alliance,
            });
        }
    }
    let mut rows = Vec::new();
    for (k, &o) in C_ROWS.iter().enumerate() {
        let r = fld(c, o);
        if r == 0 {
            continue;
        }
        let (up, down, give) = (fld(r, 0), fld(r, 4), fld(r, 8));
        let td = fld(up, field::PANELS_TD_USER);
        if td == 0 {
            continue;
        }
        let rdi = |off: usize| unsafe { rd::<i32>(td + off) };
        rows.push(Row {
            name: buttons::localize(ROW_KEYS[k]),
            amount: rdi(field::PANELS_TD_AMOUNT),
            min: rdi(field::PANELS_TD_MIN),
            max: rdi(field::PANELS_TD_MAX),
            up: checked(up, sym::PANELS_CB_UP),
            down: checked(down, sym::PANELS_CB_DOWN),
            give: checked(give, sym::PANELS_CB_GIVE),
        });
    }
    let mut msg = [0usize; 4];
    for (k, m) in msg.iter_mut().enumerate() {
        *m = checked(fld(c, C_MSG[k]), CB_MSG[k]);
    }
    Some(Comm {
        title: label_text(fld(c, field::PANELS_C_TITLE)),
        head_player: label_text(fld(c, field::PANELS_C_HEAD_PLAYER)),
        head_message: label_text(fld(c, field::PANELS_C_HEAD_MESSAGE)),
        head_team: label_text(fld(c, field::PANELS_C_HEAD_TEAM)),
        head_alliance: label_text(fld(c, field::PANELS_C_HEAD_ALLIANCE)),
        group_message: label_text(fld(c, field::PANELS_C_GROUP_MESSAGE)),
        group_give: label_text(fld(c, field::PANELS_C_GROUP_GIVE)),
        label_victory: label_text(fld(c, field::PANELS_C_LABEL_VICTORY)),
        label_latency: label_text(fld(c, field::PANELS_C_LABEL_LATENCY)),
        teams,
        alliances,
        bars,
        msg,
        give_units: checked(fld(c, field::PANELS_C_GIVE_UNITS), sym::PANELS_CB_GIVE_UNITS),
        okay: checked(fld(c, field::PANELS_C_OKAY), sym::PANELS_CB_OKAY),
        rows,
        victory: toggle(fld(c, field::PANELS_C_VICTORY), sym::PANELS_CB_VICTORY),
        latency: toggle(fld(c, field::PANELS_C_LATENCY), sym::PANELS_CB_LATENCY),
    })
}

// ---- the dropped player ---------------------------------------------------------------

// `dropPlayerBox`, an instance, and `DropPlayerBox`'s vtable. `+0x2c`: its text, a
// `DropPlayerText` whose `GetInformation` writes the whole message (the wait and its
// seconds, which player dropped and why, the votes); `+0x30`: drop the player
// (`DropCallback`, a vote through the network transport), `+0x34`: let the computer
// take over (`AiTakeoverCallback`). Its `AlwaysGameSimulate` disables a button whose
// vote has been cast, and the AI one when the game type cannot have a computer player.
// Shown by `Program::DisplayInputProcess` while the transport waits on a player.

pub struct Drop {
    pub text: String,
    pub colour: u32,
    pub drop: usize,
    pub ai: usize,
}

pub fn drop_shown() -> bool {
    vt_is(drop_panel(), sym::PANELS_VT_DROP) && !super::hud::hidden(drop_panel())
}

pub fn drop_box() -> Option<Drop> {
    if !drop_shown() {
        return None;
    }
    let (c, text) = information(fld(drop_panel(), field::PANELS_D_TEXT))?;
    Some(Drop { text, colour: buttons::colour(c), drop: checked(fld(drop_panel(), field::PANELS_D_DROP), sym::PANELS_CB_DROP), ai: checked(fld(drop_panel(), field::PANELS_D_AI), sym::PANELS_CB_AI) })
}

// ---- the wait for the other players' loads --------------------------------------------

// `loadPercentageBox`, an instance, and `LoadPercentageBox`'s vtable. `+0x2c`: its
// text, a `LoadPercentageText` holding the milliseconds waited so far at `+0x284`
// (`UpdateTime`) and writing, in `GetInformation`, the wait, its seconds and each
// network player's progress (`Transport::GetLoadPercentage`). Shown by
// `Program::DisplayInputProcess` once this player's load is done while the transport
// still shows load percentages, and hidden when they are all in.
pub const LOAD_WAIT: &core::ffi::CStr = c"GUI_POP_LOAD_WAIT";
/// The word after the seconds ("seconds"), as the stock text writes it.
pub const LOAD_SECONDS: &core::ffi::CStr = c"GUI_POP_LOAD_SECONDS";

pub fn load_shown() -> bool {
    vt_is(load_panel(), sym::PANELS_VT_LOAD) && !super::hud::hidden(load_panel())
}

/// Seconds waited, and the stock box's text (for the log and as a fallback).
pub fn load_wait() -> Option<(f32, String)> {
    if !load_shown() {
        return None;
    }
    let t = fld(load_panel(), field::PANELS_L_TEXT);
    let ms = if t != 0 { unsafe { rd::<i32>(t + field::PANELS_L_MS) } } else { 0 };
    let text = information(t).map(|i| i.1).unwrap_or_default();
    Some((ms.max(0) as f32 / 1000.0, text))
}

/// The functions this module calls, as stock.
pub fn functions_stock() -> bool {
    if !ok() {
        return false;
    }
    // `StandardText::GetInformation` is called through each text's own vtable; the
    // three classes' entries must be theirs.
    let gi = |vt: a2engine::Sym, slot: a2engine::Sym, f: a2engine::Sym| fld(addr(vt), field::PANELS_VT_GET_INFORMATION) == addr(f) && fld(addr(slot), 0) == addr(f);
    gi(sym::PANELS_VT_DROP_TEXT, sym::PANELS_DROP_TEXT_SLOT, sym::PANELS_DROP_TEXT_INFO)
        && gi(sym::PANELS_VT_LOAD_TEXT, sym::PANELS_LOAD_TEXT_SLOT, sym::PANELS_LOAD_TEXT_INFO)
        // `ToggleButton::mButtonPressFunction`, which `buttons::press` calls for a toggle
        && eng().ready(hook::PANELS_TOGGLE_PRESS).is_ok()
        && buttons::functions_stock()
}
