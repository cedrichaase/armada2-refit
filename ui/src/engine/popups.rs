//! The HUD's boxes: the mission message box (`MessageDisplay`, `g_messageDisplay`),
//! the briefing and objectives (`ObjectivesDisplay`, `g_objectivesDisplay`), the pause
//! box (`PauseBox`, `pauseBox`) and the replay controls (`ReplayPanel`,
//! `g_replayPanel`). Read by `hud::messages`, `hud::objectives`, `hud::pause` and
//! `hud::replay` (HUD.md, "Messages and objectives", "Pause and replay").
//!
//! Each is shown and hidden by the engine (`SetIsHidden`, the component's `+0x16`):
//! a script's `MessageDisplay_Set_Text_From_File`, the objectives key or button, a
//! network pause order. Ours draws a box while the component is not hidden, and
//! presses the component's own buttons, whose press functions hide it again (and, for
//! the pause box, send the unpause order).
use super::{buttons, field, rd};
use alloc::string::String;
use alloc::vec::Vec;

// ---- the message box ------------------------------------------------------------------

/// `g_messageDisplay`, an instance. `+0x28`: the title (an `ObjectivesText`, the
/// message's first line), `+0x2c`: the body (the rest), `+0x34`: OK, a
/// `StandardButton` pressing `mUnpauseCallback` (hides the box; in a network game it
/// also clears `g_gameProcessObjectivesPause`). `CursorOver` is always true, so while
/// it shows the box has the mouse wherever the pointer is: it is modal.
pub const MESSAGE: usize = 0x764d68;
const MSG_TITLE: usize = 0x28;
const MSG_BODY: usize = 0x2c;
const MSG_OK: usize = 0x34;
const MSG_UNPAUSE: usize = 0x518620;

pub struct Message {
    pub title: String,
    pub body: String,
    pub ok: usize,
}

pub fn message() -> Option<Message> {
    if super::hud::hidden(MESSAGE) {
        return None;
    }
    let ok = field(MESSAGE, MSG_OK);
    let ok = if buttons::press_fn(ok) == MSG_UNPAUSE { ok } else { 0 };
    Some(Message { title: buttons::text(field(MESSAGE, MSG_TITLE)), body: buttons::text(field(MESSAGE, MSG_BODY)), ok })
}

// ---- the objectives -------------------------------------------------------------------

/// `g_objectivesDisplay`, an instance. Its texts, all `ObjectivesText`s: `+0x28` the
/// summary's header (`GUI_OD_BRIEFING_SUMMARY`), `+0x2c` the briefing, `+0x30` the
/// mission's title, `+0x34` the objectives' header (`GUI_OD_OBJECTIVES`). The
/// objectives themselves are a `std::vector<ObjectivesText *>` at `+0x3c` (begin,
/// end at `+0x40`), their check marks ("[X]" or "[  ]") a parallel vector at `+0x4c`,
/// and whether each is complete a `std::vector<bool>` stored as bytes at `+0x5c` (end
/// `+0x60`), which `SetObjectiveComplete` writes. `+0x70`: OK, pressing
/// `mUnpauseCallback`, which hides it.
///
/// The texts are parsed from the mission's objectives file
/// (`ObjectivesDisplay_Set_Text_From_File`, `ParseObjectivesText`) when a script sets
/// it, and again in `PostLoad` from `s_lastFilename`.
pub const OBJECTIVES: usize = 0x7648a8;
const OBJ_SUMMARY_HEADER: usize = 0x28;
const OBJ_BRIEFING: usize = 0x30;
const OBJ_TITLE: usize = 0x2c;
const OBJ_HEADER: usize = 0x34;
const OBJ_LINES: usize = 0x3c;
const OBJ_DONE: usize = 0x5c;
const OBJ_OK: usize = 0x70;
const OBJ_UNPAUSE: usize = 0x50df20;

pub struct Objectives {
    pub title: String,
    pub summary_header: String,
    pub briefing: String,
    pub header: String,
    /// Each objective's text and whether it is complete.
    pub lines: Vec<(String, bool)>,
    pub ok: usize,
}

/// The objectives box's contents, whether or not it shows.
pub fn objectives() -> Objectives {
    let o = OBJECTIVES;
    let (b, e) = (field(o, OBJ_LINES), field(o, OBJ_LINES + 4));
    let (db, de) = (field(o, OBJ_DONE), field(o, OBJ_DONE + 4));
    let mut lines = Vec::new();
    if b != 0 && e > b && (e - b) / 4 < 64 {
        for i in 0..(e - b) / 4 {
            let t = unsafe { rd::<usize>(b + 4 * i) };
            let done = db != 0 && db + i < de && unsafe { rd::<u8>(db + i) } != 0;
            lines.push((buttons::text(t), done));
        }
    }
    let ok = field(o, OBJ_OK);
    let ok = if buttons::press_fn(ok) == OBJ_UNPAUSE { ok } else { 0 };
    Objectives {
        title: buttons::text(field(o, OBJ_TITLE)),
        summary_header: buttons::text(field(o, OBJ_SUMMARY_HEADER)),
        briefing: buttons::text(field(o, OBJ_BRIEFING)),
        header: buttons::text(field(o, OBJ_HEADER)),
        lines,
        ok,
    }
}

pub fn objectives_shown() -> bool {
    !super::hud::hidden(OBJECTIVES)
}

/// `ObjectivesDisplay::FocusSimulate`: unless a video plays, simulates its OK and
/// background, and hides the box when either of two of the game's commands is set
/// (`s_completeCount+4`, `+8`: pointers to command flags its `PostLoad` takes; the
/// tutorial's "press Enter" is one). Stock runs it only while the box owns the mouse,
/// which, the box covering the screen, is whenever it shows; with the box ours it
/// never owns the mouse, so `objectives_keys` runs it at each simulate pass while it
/// shows, and the keys close it as in stock, by the engine's own bindings.
const OBJ_FOCUS_SIMULATE: usize = 0x50d280;
const VT_FOCUS_SIMULATE: usize = 0x44;

pub fn objectives_keys() {
    if !objectives_shown() {
        return;
    }
    let vt = field(OBJECTIVES, 0);
    if field(vt, VT_FOCUS_SIMULATE) != OBJ_FOCUS_SIMULATE {
        return;
    }
    let f: unsafe extern "thiscall" fn(usize, f32) = unsafe { core::mem::transmute(OBJ_FOCUS_SIMULATE) };
    unsafe { f(OBJECTIVES, 0.0) };
}

// ---- the pause box --------------------------------------------------------------------

/// `pauseBox`, an instance, shown and hidden by the pause order
/// (`NetOrderPause::Execute`). `+0x2c`: its text (a `PauseText`, which the GUI config
/// labels `GUI_POP_PAUSE_TEXT`), `+0x30`: the unpause button (`UnpauseCallback`, which
/// puts the unpause order in the round buffer and hides the box; labelled
/// `GUI_POP_PAUSE_CONTINUE`). Its `AlwaysSimulate` makes it the mouse's owner while the
/// transport is paused in view 2.
pub const PAUSE: usize = 0x764d18;
const PAUSE_BUTTON: usize = 0x30;
const PAUSE_UNPAUSE: usize = 0x517990;

pub fn pause_shown() -> bool {
    !super::hud::hidden(PAUSE)
}

pub fn pause_button() -> usize {
    let b = field(PAUSE, PAUSE_BUTTON);
    if buttons::press_fn(b) == PAUSE_UNPAUSE { b } else { 0 }
}

/// `IsGamePausedAtAll`: a pause order, a script's, or the objectives' in a network
/// game.
const IS_GAME_PAUSED: usize = 0x489bf0;

pub fn game_paused() -> bool {
    let f: unsafe extern "C" fn() -> bool = unsafe { core::mem::transmute(IS_GAME_PAUSED) };
    unsafe { f() }
}

// ---- the replay controls --------------------------------------------------------------

/// `g_replayPanel`, an instance: five `StandardButton`s at `+0x2c..+0x3c` (restart,
/// pause, slow, normal, fast), each pressing a function that finds the transport as a
/// `TransportReplay` (an RTTI cast; nothing outside a replay) and sets its speed
/// (`+0x84`: 0 paused, 1 slow, 2 normal, 3 fast) or goes to the first chapter.
pub const REPLAY: usize = 0x763e30;
const REPLAY_BUTTONS: usize = 0x2c;
const REPLAY_PRESS: [usize; 5] = [0x5002b0, 0x5002e0, 0x500310, 0x500340, 0x500370];
pub const REPLAY_NAMES: [&str; 5] = ["restart", "pause", "slow", "normal", "fast"];
/// The `GUI_REPLAY_*` keys of their tooltips.
pub const REPLAY_KEYS: [&core::ffi::CStr; 5] = [c"GUI_REPLAY_RESTART", c"GUI_REPLAY_PAUSE", c"GUI_REPLAY_SLOW", c"GUI_REPLAY_NORMAL", c"GUI_REPLAY_FAST"];

pub fn replay_shown() -> bool {
    !super::hud::hidden(REPLAY)
}

pub fn replay_button(i: usize) -> usize {
    if i >= 5 {
        return 0;
    }
    let b = field(REPLAY, REPLAY_BUTTONS + 4 * i);
    if buttons::press_fn(b) == REPLAY_PRESS[i] { b } else { 0 }
}

pub fn functions_stock() -> bool {
    super::starts(IS_GAME_PAUSED, &[0xa0, 0xd9, 0x7b, 0x73, 0x00])
        && super::starts(OBJ_FOCUS_SIMULATE, &[0x56, 0x8b, 0xf1, 0xb9])
        && buttons::functions_stock()
}
