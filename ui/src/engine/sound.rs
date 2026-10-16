//! The HUD's button sounds, played the way `StandardButton::Simulate` plays them: a
//! fire-and-forget `AudioSound2D` made with the game's `operator new`, named by the
//! sound the GUI config gave the buttons (`StandardButton::s_buttonClickSound`,
//! `s_buttonRolloverSound`, empty when it gave none).
use super::rd;

const OPERATOR_NEW: usize = 0x652710;
/// `AudioSound2D::AudioSound2D(name, priority, volume, ...)`, thiscall.
const SOUND2D: usize = 0x45fc50;
const CLICK: usize = 0x764854;
const ROLLOVER: usize = 0x764814;
const SOUND2D_BYTES: usize = 0x1c;

fn play(name: usize) {
    play_as(name, 4)
}

/// An `AudioSound2D` of the sound file `name` (a C string), at priority 8: the HUD's
/// buttons pass 4 as the third argument, the shell's 0.
fn play_as(name: usize, third: i32) {
    unsafe {
        if rd::<u8>(name) == 0 {
            return;
        }
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(OPERATOR_NEW);
        let ctor: unsafe extern "thiscall" fn(usize, usize, i32, i32, i32, usize) -> usize = core::mem::transmute(SOUND2D);
        let snd = new(SOUND2D_BYTES);
        if snd != 0 {
            // As stock: priority 8, volume 0, then 4 (HUD) or 0 (shell), no callback.
            ctor(snd, name, 8, 0, third, 0);
        }
    }
}

pub fn click() {
    play(CLICK)
}

pub fn rollover() {
    if !super::hud::video_playing() {
        play(ROLLOVER)
    }
}

/// `ShellButton+0xec`: the button has sounds of its own (`SetSoundFile`), its hover
/// file at `+0xf0`; else `SetButtonMode` plays the shell's default, `cornron.wav`.
const SB_HAS_SOUND: usize = 0xec;
const SB_HOVER_SOUND: usize = 0xf0;

/// The shell's hover sound, as `ShellButton::SetButtonMode` plays it when a button
/// comes under the pointer: `like`'s own (a stock `ShellButton` of the same screen) or
/// the default. For a menu entry of ours whose stock target is a hit area of the
/// screen's procedure, not a button, and so plays none.
pub fn shell_hover(like: Option<usize>) {
    if !functions_stock() {
        return;
    }
    let own = like.filter(|&b| b != 0 && unsafe { rd::<u32>(b + SB_HAS_SOUND) } != 0);
    match own {
        Some(b) => play_as(b + SB_HOVER_SOUND, 0),
        None => play_as(c"cornron.wav".as_ptr() as usize, 0),
    }
}

/// The shell's click (`SetButtonMode`'s for a press without a sound of its own), for
/// a control of ours that has no stock button to click.
pub fn shell_click() {
    if functions_stock() {
        play_as(c"cornrclk.wav".as_ptr() as usize, 0);
    }
}

pub fn functions_stock() -> bool {
    super::starts(OPERATOR_NEW, &[0x55, 0x8b, 0xec, 0xa1]) && super::starts(SOUND2D, &[0x55, 0x8b, 0xec, 0x6a, 0xff])
}
