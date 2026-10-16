//! The HUD's plain buttons and texts (`StandardButton`, `StandardText`), as the panels
//! of `hud::` read and press them (HUD.md, "The top-right buttons", "Messages and
//! objectives").
//!
//! A `StandardButton` (a `StandardComponent`) holds its panel at `+0x04`, its
//! rectangle relative to the panel at `+0x08`, its border and picture sprites at
//! `+0x28` / `+0x2c` (`SetImages`), held at `+0x30`, its state at `+0x34` (0 disabled,
//! 2 active, else normal), its label inline at `+0x39` (`SetLabel`) and its press
//! function at `+0x5c`, a static `void (StandardButton *)` that
//! `mButtonPressFunction` (vtable `+0x20`) calls. `StandardButton::Simulate` presses it
//! on a click's release when the state is not 0, after the click sound unless a video
//! plays; `press` does the same. Its tooltips are `GetTooltip` / `GetVerboseTooltip`
//! (vtable `+0x00` / `+0x04`), which write the text `SetTooltipText` gave it into a
//! `std::stringstream`.
//!
//! The texts the message and objectives boxes show are `ObjectivesText`s, a
//! `StandardText` with its colour at `+0x70` (three floats) and its string inline at
//! `+0x84` (`ObjectivesText::SetText` copies it there, at most 0x800 bytes).
use super::{field, rd};
use alloc::string::String;

const BTN_BORDER: usize = 0x28;
const BTN_SPRITE: usize = 0x2c;
const BTN_STATE: usize = 0x34;
const BTN_LABEL: usize = 0x39;
const BTN_PRESS_FN: usize = 0x5c;
const VT_TOOLTIP: usize = 0x00;
const VT_VERBOSE: usize = 0x04;
const VT_PRESS: usize = 0x20;

/// `StandardButton::mButtonPressFunction`: what the vtable slot must be for `press` to
/// call it (a subclass with its own press is not pressed this way).
const STANDARD_BUTTON_PRESS: usize = 0x50bad0;

const TEXT_COLOUR: usize = 0x70;
const TEXT_STRING: usize = 0x84;
const TEXT_MAX: usize = 0x800;

/// The button's state: 0 disabled, 2 active, else normal. 0 for no button.
pub fn state(b: usize) -> u32 {
    if b == 0 { 0 } else { unsafe { rd(b + BTN_STATE) } }
}

pub fn enabled(b: usize) -> bool {
    state(b) != 0
}

/// The button's picture (`ST3D_Sprite *`), and the border drawn over it on hover.
pub fn sprite(b: usize) -> usize {
    field(b, BTN_SPRITE)
}

pub fn border(b: usize) -> usize {
    field(b, BTN_BORDER)
}

/// The button's label (`SetLabel`), as the game's localised string.
pub fn label(b: usize) -> String {
    if b == 0 {
        return String::new();
    }
    clean(&latin1(b + BTN_LABEL, 64))
}

/// The press function (`+0x5c`), to name a button by what it does.
pub fn press_fn(b: usize) -> usize {
    field(b, BTN_PRESS_FN)
}

/// The button's tooltip, short or verbose, as the game writes it.
pub fn tooltip(b: usize, verbose: bool) -> Option<String> {
    if b == 0 || !super::msvc::available() {
        return None;
    }
    let slot = if verbose { VT_VERBOSE } else { VT_TOOLTIP };
    super::msvc::with_stringstream(|ss| unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(rd::<usize>(rd::<usize>(b) + slot));
        f(b, ss)
    })
    .map(|s| clean(&s))
    .filter(|s| !s.is_empty())
}

/// Presses the button as releasing a click on it does: nothing if it is disabled, else
/// the click sound (unless a video plays) and its press function.
pub fn press(b: usize) -> bool {
    if b == 0 || !enabled(b) {
        return false;
    }
    let vt_press = unsafe { rd::<usize>(rd::<usize>(b) + VT_PRESS) };
    if vt_press != STANDARD_BUTTON_PRESS {
        crate::log!("buttons: {:#x} is not a StandardButton's press, not pressed", vt_press);
        return false;
    }
    if !super::hud::video_playing() {
        super::sound::click();
    }
    let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(vt_press) };
    unsafe { f(b) };
    true
}

// ---- presses at the game's simulate -----------------------------------------------------

/// In `Program::DisplayInputProcess`: `call Entity::ResetVisibility`, made once per
/// pass after the input loop that runs `DisplayInterface::SimulateAll` (the HUD's
/// simulate, where a stock button presses itself) and before `Armada_RenderFrame`.
/// A press of ours is made there, not in the UI's frame: the UI draws inside the
/// engine's scene, and a press may open a dialog (the menu button's `do_escape`) or
/// change what the HUD shows; there it happens where stock's would.
const SITE_PRESSES: usize = 0x483869;
const RESET_VISIBILITY: usize = 0x4cfac0;

static mut QUEUE: [usize; 4] = [0; 4];
static mut QUEUED_AT: f64 = 0.0;
static mut HOOKED: bool = false;

/// Patches the site. False (presses are then made at once, in the UI's frame) when it
/// is not stock.
pub fn install_presses() -> bool {
    let ok = super::hook::patch_call(SITE_PRESSES, RESET_VISIBILITY, presses as *const () as usize);
    if !ok {
        crate::log!("buttons: the call at {:#x} is not to Entity::ResetVisibility: presses are made in the UI's frame", SITE_PRESSES);
    }
    unsafe { HOOKED = ok };
    ok
}

/// Presses `b` at the next simulate pass (`press`, there). A press not taken within
/// half a second is dropped.
#[allow(static_mut_refs)]
pub fn queue(b: usize) {
    if b == 0 {
        return;
    }
    unsafe {
        if !HOOKED {
            press(b);
            return;
        }
        if let Some(s) = QUEUE.iter_mut().find(|s| **s == 0 || **s == b) {
            *s = b;
        }
        QUEUED_AT = crate::rt::now();
    }
}

#[allow(static_mut_refs)]
unsafe extern "C" fn presses() {
    let f: unsafe extern "C" fn() = unsafe { core::mem::transmute(RESET_VISIBILITY) };
    unsafe { f() };
    let q = unsafe { &mut QUEUE };
    if q[0] == 0 {
        return;
    }
    let fresh = crate::rt::now() - unsafe { QUEUED_AT } < 0.5;
    for s in q.iter_mut() {
        let b = core::mem::take(s);
        if b != 0 && fresh {
            press(b);
        }
    }
}

/// An `ObjectivesText`'s string, '\r' dropped, or empty.
pub fn text(t: usize) -> String {
    if t == 0 {
        return String::new();
    }
    latin1(t + TEXT_STRING, TEXT_MAX)
}

/// An `ObjectivesText`'s colour, 0..1 per channel.
pub fn text_colour(t: usize) -> [f32; 3] {
    if t == 0 { [1.0; 3] } else { unsafe { rd(t + TEXT_COLOUR) } }
}

/// The game's strings mark a hotkey's letter with '^' and wrap lines with '\r\n'.
pub fn clean(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '^' | '\r' => {}
            _ => out.push(c),
        }
    }
    String::from(out.trim_matches(|c: char| c == '\n' || c == ' '))
}

/// The game's colour as ours: 0..1 floats to 0xAARRGGBB, opaque.
pub fn colour(c: [f32; 3]) -> u32 {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    0xff000000 | (b(c[0]) << 16) | (b(c[1]) << 8) | b(c[2])
}

/// `Localized_Strings::Localize(key)` on `g_string`: the game's text for a key such as
/// `GUI_POP_PAUSE_TEXT`, or the key itself when it has none.
const LOCALIZE: usize = 0x481c90;
const G_STRING: usize = 0x7379fc;

pub fn localize(key: &core::ffi::CStr) -> String {
    let s = unsafe { rd::<usize>(G_STRING) };
    if s == 0 {
        return String::from(key.to_str().unwrap_or(""));
    }
    let f: unsafe extern "thiscall" fn(usize, *const u8) -> *const u8 = unsafe { core::mem::transmute(LOCALIZE) };
    let p = unsafe { f(s, key.as_ptr() as *const u8) };
    clean(&latin1(p as usize, 1024))
}

/// A NUL-terminated string of the game's (Latin-1, as its localised strings are), at
/// most `max` bytes, '\r' dropped. Empty for NULL.
pub fn latin1(p: usize, max: usize) -> String {
    let mut s = String::new();
    if p == 0 {
        return s;
    }
    for i in 0..max {
        let c = unsafe { *((p + i) as *const u8) };
        if c == 0 {
            break;
        }
        if c != b'\r' {
            s.push(c as char);
        }
    }
    s
}

pub fn functions_stock() -> bool {
    super::starts(STANDARD_BUTTON_PRESS, &[0x8b, 0x41, 0x5c, 0x85, 0xc0]) && super::starts(LOCALIZE, &[0x55, 0x8b, 0xec])
}
