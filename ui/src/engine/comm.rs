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
use super::{buttons, field, rd};
use alloc::string::String;
use alloc::vec::Vec;

// ---- the comm panel -------------------------------------------------------------------

/// `commDisplay`, an instance, and `CommDisplay`'s vtable.
pub const COMM: usize = 0x764a60;
const VT_COMM: usize = 0x6b5a5c;
/// Its labels (`StandardText`s made by `CreateLabel` from the GUI config's `comm*`
/// keys): the title, the player, message, team and alliance headings, the message and
/// give groups' captions, and the allied-victory and latency captions (these two NULL
/// when the game has no such toggle).
const C_TITLE: usize = 0x3c;
const C_HEAD_PLAYER: usize = 0x40;
const C_HEAD_MESSAGE: usize = 0x44;
const C_HEAD_TEAM: usize = 0x48;
const C_HEAD_ALLIANCE: usize = 0x4c;
const C_GROUP_MESSAGE: usize = 0x50;
const C_GROUP_GIVE: usize = 0x54;
const C_LABEL_VICTORY: usize = 0x58;
const C_LABEL_LATENCY: usize = 0x5c;
/// Its `StandardButton`s: message all, allies, enemies, the selected player (each
/// marks which bars' chat toggles are on, then `Transport::UpdateChatTargets`), give
/// the selected units to the selected player, OK (hides the panel).
const C_MSG: [usize; 4] = [0x60, 0x64, 0x68, 0x6c];
const C_GIVE_UNITS: usize = 0x70;
const C_OKAY: usize = 0x74;
/// One resource row per transferable resource, NULL for one the player's race does
/// not have (`Transfer*::UsedBy`): a 16-byte block of the up button, the down button,
/// the give button and the amount's text, in that order. Dilithium, metal, latinum,
/// crew, biomatter.
const C_ROWS: [usize; 5] = [0x78, 0x7c, 0x80, 0x84, 0x88];
/// The `GUI_CP_*_RES` key each row's resource is named by (`Transfer*::GetName`).
const ROW_KEYS: [&core::ffi::CStr; 5] = [c"GUI_CP_DILITHIUM_RES", c"GUI_CP_METAL_RES", c"GUI_CP_LATINUM_RES", c"GUI_CP_CREW_RES", c"GUI_CP_BIOMATTER_RES"];
/// A row's buttons hold their `TransferDetails` at `+0x60` (the stock callbacks read
/// it there): the amount to give at `+0x40`, its least and most at `+0x48`/`+0x4c`, the
/// step of the up and down buttons at `+0x50`.
const BTN_USER: usize = 0x60;
const TD_AMOUNT: usize = 0x40;
const TD_MIN: usize = 0x48;
const TD_MAX: usize = 0x4c;
/// The allied-victory and high-latency `ToggleButton`s (NULL when the game has none):
/// on when their state is 2.
const C_VICTORY: usize = 0x8c;
const C_LATENCY: usize = 0x90;
/// The bars: their count (`commBarNumberOfPlayers` in the GUI config) and the array of
/// `CommBar *`, one per slot in slot order.
const C_BAR_COUNT: usize = 0x94;
const C_BARS: usize = 0x98;
/// The setup's relationship type, as `PostLoad` reads it: teams fixed (the team column
/// shows), or alliances free (the alliance column shows).
const C_TEAMS: usize = 0x9c;
const C_ALLIANCES: usize = 0x9d;
/// The selected bar's index (`SetCurrentPlayer`): the player the give buttons and the
/// private message go to.
const C_SELECTED: usize = 0xa0;

/// `CommBar`: `+0x28` it stands for a player (cleared for an empty slot), `+0x2c` its
/// slot, `+0x34` the player's team (start location), `+0x38` the team shows, `+0x39`
/// the alliance buttons work; the name button (`PlayerSelectCallback`) `+0x3c`, the
/// name `+0x40` (a `PlayerNameText`, the name inline at `+0x84`), the chat
/// `ToggleButton` `+0x44` (on: the player hears this player's chat), the team's text
/// `+0x48` (a `TeamNameText`: its number, from 1, at `+0x84`), the enemy, neutral and
/// ally `AllianceButton`s `+0x4c`, `+0x50`, `+0x54` (the relation in force has state
/// 2; one proposed but not yet answered has `+0x7c` set), and the colour
/// `ColorButton` `+0x58` (`PlayerSelectCallback` as well; the player's colour, three
/// floats, at `+0x68`).
const VT_BAR: usize = 0x6b5870;
const B_PRESENT: usize = 0x28;
const B_SLOT: usize = 0x2c;
const B_SHOW_TEAM: usize = 0x38;
const B_SHOW_ALLIANCE: usize = 0x39;
const B_NAME_BUTTON: usize = 0x3c;
const B_NAME: usize = 0x40;
const B_CHAT: usize = 0x44;
const B_TEAM_TEXT: usize = 0x48;
const B_ALLIANCE: [usize; 3] = [0x4c, 0x50, 0x54];
const B_COLOUR: usize = 0x58;
const TEXT_INLINE: usize = 0x84;
const ALLIANCE_PENDING: usize = 0x7c;
const COLOUR_RGB: usize = 0x68;

/// A `StandardText`'s GUI config label key (`StandardText::GetLabel`, `+0x54`), which
/// its `Render` localises.
const TEXT_LABEL_KEY: usize = 0x54;
/// `StandardText::GetInformation` (vtable `+0x24`): the colour and the text a text
/// component shows this frame; its subclasses (`DropPlayerText`, `LoadPercentageText`)
/// build the text there.
const VT_GET_INFORMATION: usize = 0x24;

/// The panel's own press functions, the stock buttons' `+0x5c`, by which `comm()`
/// recognises each button before it is offered to the panel.
const CB_PLAYER_SELECT: usize = 0x510530;
const CB_CHAT: usize = 0x510610;
const CB_ALLIANCE: [usize; 3] = [0x510550, 0x510590, 0x5105d0];
const CB_MSG: [usize; 4] = [0x514f60, 0x514fa0, 0x515040, 0x5150e0];
const CB_GIVE_UNITS: usize = 0x515260;
const CB_OKAY: usize = 0x5155b0;
const CB_UP: usize = 0x5154f0;
const CB_DOWN: usize = 0x515520;
const CB_GIVE: usize = 0x515420;
const CB_VICTORY: usize = 0x515550;
const CB_LATENCY: usize = 0x515580;

/// `GUI_POP_COMM_TEAM_PREFIX`, which `TeamNameText` writes before the team's number.
pub const TEAM_PREFIX: &core::ffi::CStr = c"GUI_POP_COMM_TEAM_PREFIX";

/// A button the panel may press: the button, if its press function is `cb`.
fn checked(b: usize, cb: usize) -> usize {
    if b != 0 && buttons::press_fn(b) == cb { b } else { 0 }
}

fn vt_is(o: usize, vt: usize) -> bool {
    o != 0 && field(o, 0) == vt
}

/// A text component's label, localised as its `Render` does.
fn label_text(t: usize) -> String {
    let k = field(t, TEXT_LABEL_KEY);
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
        let f: unsafe extern "thiscall" fn(usize, *mut [u32; 4]) -> *mut [u32; 4] = core::mem::transmute(field(field(t, 0), VT_GET_INFORMATION));
        f(t, &mut out);
    }
    let c = [f32::from_bits(out[0]), f32::from_bits(out[1]), f32::from_bits(out[2])];
    Some((c, buttons::clean(&buttons::latin1(out[3] as usize, 2048))))
}

pub fn comm_shown() -> bool {
    vt_is(COMM, VT_COMM) && !super::hud::hidden(COMM)
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

fn toggle(t: usize, cb: usize) -> (usize, bool) {
    let b = checked(t, cb);
    (b, b != 0 && buttons::state(b) == 2)
}

pub fn comm() -> Option<Comm> {
    if !vt_is(COMM, VT_COMM) {
        return None;
    }
    let c = COMM;
    let rd8 = |o: usize| unsafe { rd::<u8>(c + o) } != 0;
    let teams = rd8(C_TEAMS);
    let alliances = rd8(C_ALLIANCES);
    let selected = field(c, C_SELECTED);
    let mut bars = Vec::new();
    let (n, arr) = (field(c, C_BAR_COUNT), field(c, C_BARS));
    if arr != 0 && n <= 16 {
        for i in 0..n {
            let b = field(arr, 4 * i);
            if !vt_is(b, VT_BAR) || unsafe { rd::<u8>(b + B_PRESENT) } == 0 {
                continue;
            }
            let name = buttons::latin1(field(b, B_NAME) + TEXT_INLINE, 64);
            let show_team = unsafe { rd::<u8>(b + B_SHOW_TEAM) } != 0;
            let team = if show_team { Some(unsafe { rd::<i32>(field(b, B_TEAM_TEXT) + TEXT_INLINE) }) } else { None };
            let cb = field(b, B_COLOUR);
            let rgb: [f32; 3] = if cb != 0 { unsafe { rd(cb + COLOUR_RGB) } } else { [1.0; 3] };
            let select = checked(cb, CB_PLAYER_SELECT);
            let select = if select != 0 { select } else { checked(field(b, B_NAME_BUTTON), CB_PLAYER_SELECT) };
            let chat = checked(field(b, B_CHAT), CB_CHAT);
            let show_alliance = unsafe { rd::<u8>(b + B_SHOW_ALLIANCE) } != 0;
            let mut alliance = [(0usize, false, false); 3];
            for (k, a) in alliance.iter_mut().enumerate() {
                let ab = field(b, B_ALLIANCE[k]);
                if ab == 0 {
                    continue;
                }
                let st = buttons::state(ab);
                let pending = unsafe { rd::<u8>(ab + ALLIANCE_PENDING) } != 0;
                *a = (if show_alliance { checked(ab, CB_ALLIANCE[k]) } else { 0 }, st == 2, pending);
            }
            bars.push(Bar {
                slot: field(b, B_SLOT),
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
        let r = field(c, o);
        if r == 0 {
            continue;
        }
        let (up, down, give) = (field(r, 0), field(r, 4), field(r, 8));
        let td = field(up, BTN_USER);
        if td == 0 {
            continue;
        }
        let rdi = |off: usize| unsafe { rd::<i32>(td + off) };
        rows.push(Row {
            name: buttons::localize(ROW_KEYS[k]),
            amount: rdi(TD_AMOUNT),
            min: rdi(TD_MIN),
            max: rdi(TD_MAX),
            up: checked(up, CB_UP),
            down: checked(down, CB_DOWN),
            give: checked(give, CB_GIVE),
        });
    }
    let mut msg = [0usize; 4];
    for (k, m) in msg.iter_mut().enumerate() {
        *m = checked(field(c, C_MSG[k]), CB_MSG[k]);
    }
    Some(Comm {
        title: label_text(field(c, C_TITLE)),
        head_player: label_text(field(c, C_HEAD_PLAYER)),
        head_message: label_text(field(c, C_HEAD_MESSAGE)),
        head_team: label_text(field(c, C_HEAD_TEAM)),
        head_alliance: label_text(field(c, C_HEAD_ALLIANCE)),
        group_message: label_text(field(c, C_GROUP_MESSAGE)),
        group_give: label_text(field(c, C_GROUP_GIVE)),
        label_victory: label_text(field(c, C_LABEL_VICTORY)),
        label_latency: label_text(field(c, C_LABEL_LATENCY)),
        teams,
        alliances,
        bars,
        msg,
        give_units: checked(field(c, C_GIVE_UNITS), CB_GIVE_UNITS),
        okay: checked(field(c, C_OKAY), CB_OKAY),
        rows,
        victory: toggle(field(c, C_VICTORY), CB_VICTORY),
        latency: toggle(field(c, C_LATENCY), CB_LATENCY),
    })
}

// ---- the dropped player ---------------------------------------------------------------

/// `dropPlayerBox`, an instance, and `DropPlayerBox`'s vtable. `+0x2c`: its text, a
/// `DropPlayerText` whose `GetInformation` writes the whole message (the wait and its
/// seconds, which player dropped and why, the votes); `+0x30`: drop the player
/// (`DropCallback`, a vote through the network transport), `+0x34`: let the computer
/// take over (`AiTakeoverCallback`). Its `AlwaysGameSimulate` disables a button whose
/// vote has been cast, and the AI one when the game type cannot have a computer player.
/// Shown by `Program::DisplayInputProcess` while the transport waits on a player.
pub const DROP: usize = 0x764cd0;
const VT_DROP: usize = 0x6b5b68;
const D_TEXT: usize = 0x2c;
const D_DROP: usize = 0x30;
const D_AI: usize = 0x34;
const CB_DROP: usize = 0x516a30;
const CB_AI: usize = 0x516a60;

pub struct Drop {
    pub text: String,
    pub colour: u32,
    pub drop: usize,
    pub ai: usize,
}

pub fn drop_shown() -> bool {
    vt_is(DROP, VT_DROP) && !super::hud::hidden(DROP)
}

pub fn drop_box() -> Option<Drop> {
    if !drop_shown() {
        return None;
    }
    let (c, text) = information(field(DROP, D_TEXT))?;
    Some(Drop { text, colour: buttons::colour(c), drop: checked(field(DROP, D_DROP), CB_DROP), ai: checked(field(DROP, D_AI), CB_AI) })
}

// ---- the wait for the other players' loads --------------------------------------------

/// `loadPercentageBox`, an instance, and `LoadPercentageBox`'s vtable. `+0x2c`: its
/// text, a `LoadPercentageText` holding the milliseconds waited so far at `+0x284`
/// (`UpdateTime`) and writing, in `GetInformation`, the wait, its seconds and each
/// network player's progress (`Transport::GetLoadPercentage`). Shown by
/// `Program::DisplayInputProcess` once this player's load is done while the transport
/// still shows load percentages, and hidden when they are all in.
pub const LOAD: usize = 0x764de0;
const VT_LOAD: usize = 0x6b5d94;
const L_TEXT: usize = 0x2c;
const L_MS: usize = 0x284;
pub const LOAD_WAIT: &core::ffi::CStr = c"GUI_POP_LOAD_WAIT";

pub fn load_shown() -> bool {
    vt_is(LOAD, VT_LOAD) && !super::hud::hidden(LOAD)
}

/// Seconds waited, and the stock box's text (for the log and as a fallback).
pub fn load_wait() -> Option<(f32, String)> {
    if !load_shown() {
        return None;
    }
    let t = field(LOAD, L_TEXT);
    let ms = if t != 0 { unsafe { rd::<i32>(t + L_MS) } } else { 0 };
    let text = information(t).map(|i| i.1).unwrap_or_default();
    Some((ms.max(0) as f32 / 1000.0, text))
}

/// The functions this module calls, as stock.
pub fn functions_stock() -> bool {
    // `StandardText::GetInformation` is called through each text's own vtable; the
    // three classes' entries must be theirs.
    let gi = |vt: usize, f: usize| field(vt, VT_GET_INFORMATION) == f;
    gi(0x6b5bec, 0x516ae0)
        && gi(0x6b5e18, 0x518b70)
        // `ToggleButton::mButtonPressFunction`, which `buttons::press` calls for a toggle
        && super::starts(0x510990, &[0x8b, 0x41, 0x34, 0x85, 0xc0])
        && buttons::functions_stock()
}
