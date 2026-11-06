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
use super::{buttons, field as peek, rd};
use crate::syms_hud::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// Whether the boxes can be read (`hud.reads_popups`: an unknown build or a function not
/// stock reads nothing).
fn ok() -> bool {
    eng().ready(hook::HUD_READS_POPUPS).is_ok()
}

fn a(s: a2engine::Sym) -> usize {
    if ok() { eng().addr(s) } else { 0 }
}
use alloc::string::String;
use alloc::vec::Vec;

// ---- the message box ------------------------------------------------------------------

/// `g_messageDisplay` (the address of the instance). `+0x28`: the title (an `ObjectivesText`, the
/// message's first line), `+0x2c`: the body (the rest), `+0x34`: OK, a
/// `StandardButton` pressing `mUnpauseCallback` (hides the box; in a network game it
/// also clears `g_gameProcessObjectivesPause`). `CursorOver` is always true, so while
/// it shows the box has the mouse wherever the pointer is: it is modal.
pub fn message_box() -> usize {
    a(sym::POPUPS_MESSAGE)
}

pub struct Message {
    pub title: String,
    pub body: String,
    pub ok: usize,
}

pub fn message() -> Option<Message> {
    let m = message_box();
    if m == 0 || super::hud::hidden(m) {
        return None;
    }
    let ok = peek(m, field::POPUPS_MSG_OK());
    let ok = if buttons::press_fn(ok) == eng().addr(sym::POPUPS_MSG_UNPAUSE) { ok } else { 0 };
    Some(Message { title: buttons::text(peek(m, field::POPUPS_MSG_TITLE())), body: buttons::text(peek(m, field::POPUPS_MSG_BODY())), ok })
}

// ---- the objectives -------------------------------------------------------------------

/// `g_objectivesDisplay` (the address of the instance). Its texts, all `ObjectivesText`s: `+0x28` the
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
pub fn objectives_box() -> usize {
    a(sym::POPUPS_OBJECTIVES)
}

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
    let o = objectives_box();
    if o == 0 {
        return Objectives { title: String::new(), summary_header: String::new(), briefing: String::new(), header: String::new(), lines: Vec::new(), ok: 0 };
    }
    let (b, e) = (peek(o, field::POPUPS_OBJ_LINES()), peek(o, field::POPUPS_OBJ_LINES() + 4));
    let (db, de) = (peek(o, field::POPUPS_OBJ_DONE()), peek(o, field::POPUPS_OBJ_DONE() + 4));
    let mut lines = Vec::new();
    if b != 0 && e > b && (e - b) / 4 < 64 {
        for i in 0..(e - b) / 4 {
            let t = unsafe { rd::<usize>(b + 4 * i) };
            let done = db != 0 && db + i < de && unsafe { rd::<u8>(db + i) } != 0;
            lines.push((buttons::text(t), done));
        }
    }
    let ok = peek(o, field::POPUPS_OBJ_OK());
    let ok = if buttons::press_fn(ok) == eng().addr(sym::POPUPS_OBJ_UNPAUSE) { ok } else { 0 };
    Objectives {
        title: buttons::text(peek(o, field::POPUPS_OBJ_TITLE())),
        summary_header: buttons::text(peek(o, field::POPUPS_OBJ_SUMMARY_HEADER())),
        briefing: buttons::text(peek(o, field::POPUPS_OBJ_BRIEFING())),
        header: buttons::text(peek(o, field::POPUPS_OBJ_HEADER())),
        lines,
        ok,
    }
}

pub fn objectives_shown() -> bool {
    let o = objectives_box();
    o != 0 && !super::hud::hidden(o)
}

/// `ObjectivesDisplay::FocusSimulate`: unless a video plays, simulates its OK and
/// background, and hides the box when either of two of the game's commands is set
/// (`s_completeCount+4`, `+8`: pointers to command flags its `PostLoad` takes; the
/// tutorial's "press Enter" is one). Stock runs it only while the box owns the mouse,
/// which, the box covering the screen, is whenever it shows; with the box ours it
/// never owns the mouse, so `objectives_keys` runs it at each simulate pass while it
/// shows, and the keys close it as in stock, by the engine's own bindings.
pub fn objectives_keys() {
    if !objectives_shown() {
        return;
    }
    let (o, focus) = (objectives_box(), eng().addr(sym::POPUPS_OBJ_FOCUS_SIMULATE));
    let vt = peek(o, 0);
    if focus == 0 || peek(vt, field::POPUPS_VT_FOCUS_SIMULATE()) != focus {
        return;
    }
    let f: unsafe extern "thiscall" fn(usize, f32) = unsafe { core::mem::transmute(focus) };
    unsafe { f(o, 0.0) };
}

// ---- the pause box --------------------------------------------------------------------

/// `pauseBox` (the address of the instance), shown and hidden by the pause order
/// (`NetOrderPause::Execute`). `+0x2c`: its text (a `PauseText`, which the GUI config
/// labels `GUI_POP_PAUSE_TEXT`), `+0x30`: the unpause button (`UnpauseCallback`, which
/// puts the unpause order in the round buffer and hides the box; labelled
/// `GUI_POP_PAUSE_CONTINUE`). Its `AlwaysSimulate` makes it the mouse's owner while the
/// transport is paused in view 2.
pub fn pause_box() -> usize {
    a(sym::POPUPS_PAUSE)
}

pub fn pause_shown() -> bool {
    let p = pause_box();
    p != 0 && !super::hud::hidden(p)
}

pub fn pause_button() -> usize {
    let p = pause_box();
    if p == 0 {
        return 0;
    }
    let b = peek(p, field::POPUPS_PAUSE_BUTTON());
    if buttons::press_fn(b) == eng().addr(sym::POPUPS_PAUSE_UNPAUSE) { b } else { 0 }
}

/// `IsGamePausedAtAll`: a pause order, a script's, or the objectives' in a network
/// game.
pub fn game_paused() -> bool {
    let p = a(sym::POPUPS_IS_GAME_PAUSED);
    if p == 0 {
        return false;
    }
    let f: unsafe extern "C" fn() -> bool = unsafe { core::mem::transmute(p) };
    unsafe { f() }
}

// ---- the replay controls --------------------------------------------------------------

/// `g_replayPanel` (the address of the instance): five `StandardButton`s at `+0x2c..+0x3c` (restart,
/// pause, slow, normal, fast), each pressing a function that finds the transport as a
/// `TransportReplay` (an RTTI cast; nothing outside a replay) and sets its speed
/// (`+0x84`: 0 paused, 1 slow, 2 normal, 3 fast) or goes to the first chapter.
pub fn replay_box() -> usize {
    a(sym::POPUPS_REPLAY)
}
pub const REPLAY_NAMES: [&str; 5] = ["restart", "pause", "slow", "normal", "fast"];
/// The `GUI_REPLAY_*` keys of their tooltips.
pub const REPLAY_KEYS: [&core::ffi::CStr; 5] = [c"GUI_REPLAY_RESTART", c"GUI_REPLAY_PAUSE", c"GUI_REPLAY_SLOW", c"GUI_REPLAY_NORMAL", c"GUI_REPLAY_FAST"];

pub fn replay_shown() -> bool {
    let r = replay_box();
    r != 0 && !super::hud::hidden(r)
}

pub fn replay_button(i: usize) -> usize {
    let r = replay_box();
    if i >= 5 || r == 0 {
        return 0;
    }
    let press = [
        sym::POPUPS_REPLAY_PRESS_0,
        sym::POPUPS_REPLAY_PRESS_1,
        sym::POPUPS_REPLAY_PRESS_2,
        sym::POPUPS_REPLAY_PRESS_3,
        sym::POPUPS_REPLAY_PRESS_4,
    ];
    let b = peek(r, field::POPUPS_REPLAY_BUTTONS() + 4 * i);
    if buttons::press_fn(b) == eng().addr(press[i]) { b } else { 0 }
}

/// The boxes' functions are stock (the profile's `hud.reads_popups` checks them).
pub fn functions_stock() -> bool {
    ok() && buttons::functions_stock()
}
