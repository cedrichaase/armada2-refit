//! The HUD's button sounds, played the way `StandardButton::Simulate` plays them: a
//! fire-and-forget `AudioSound2D` made with the game's `operator new`, named by the
//! sound the GUI config gave the buttons (`StandardButton::s_buttonClickSound`,
//! `s_buttonRolloverSound`, empty when it gave none).
use super::rd;
use crate::syms_panels::{field, hook, sym, value, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

/// `AudioSound2D::AudioSound2D(name, priority, volume, ...)` is thiscall; the object is
/// `value::PANELS_SOUND2D_BYTES()` made with the game's `operator new`.
fn play(name: a2engine::Sym) {
    if !functions_stock() {
        return;
    }
    play_as(addr(name), 4)
}

/// An `AudioSound2D` of the sound file `name` (a C string), at priority 8: the HUD's
/// buttons pass 4 as the third argument, the shell's 0.
fn play_as(name: usize, third: i32) {
    if name == 0 || !functions_stock() {
        return;
    }
    unsafe {
        if rd::<u8>(name) == 0 {
            return;
        }
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(addr(sym::PANELS_OPERATOR_NEW));
        let ctor: unsafe extern "thiscall" fn(usize, usize, i32, i32, i32, usize) -> usize = core::mem::transmute(addr(sym::PANELS_SOUND2D));
        let snd = new(value::PANELS_SOUND2D_BYTES() as usize);
        if snd != 0 {
            // As stock: priority 8, volume 0, then 4 (HUD) or 0 (shell), no callback.
            ctor(snd, name, 8, 0, third, 0);
        }
    }
}

pub fn click() {
    play(sym::PANELS_CLICK_SOUND)
}

pub fn rollover() {
    if !super::hud::video_playing() {
        play(sym::PANELS_ROLLOVER_SOUND)
    }
}

/// `ShellButton+0xec`: the button has sounds of its own (`SetSoundFile`), its hover
/// file at `+0xf0`; else `SetButtonMode` plays the shell's default, `cornron.wav`.

/// The shell's hover sound, as `ShellButton::SetButtonMode` plays it when a button
/// comes under the pointer: `like`'s own (a stock `ShellButton` of the same screen) or
/// the default. For a menu entry of ours whose stock target is a hit area of the
/// screen's procedure, not a button, and so plays none.
pub fn shell_hover(like: Option<usize>) {
    if !functions_stock() {
        return;
    }
    let own = like.filter(|&b| b != 0 && unsafe { rd::<u32>(b + field::PANELS_SB_HAS_SOUND()) } != 0);
    match own {
        Some(b) => play_as(b + field::PANELS_SB_HOVER_SOUND(), 0),
        // lint-allow(content): the default SetButtonMode itself plays, the engine's, not a set's.
        None => play_as(c"cornron.wav".as_ptr() as usize, 0),
    }
}

/// The shell's click (`SetButtonMode`'s for a press without a sound of its own), for
/// a control of ours that has no stock button to click.
pub fn shell_click() {
    if functions_stock() {
        // lint-allow(content): the click SetButtonMode itself plays, the engine's, not a set's.
        play_as(c"cornrclk.wav".as_ptr() as usize, 0);
    }
}

pub fn functions_stock() -> bool {
    eng().ready(hook::PANELS_SOUND).is_ok()
}
