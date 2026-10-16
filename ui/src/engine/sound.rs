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
    unsafe {
        if rd::<u8>(name) == 0 {
            return;
        }
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(OPERATOR_NEW);
        let ctor: unsafe extern "thiscall" fn(usize, usize, i32, i32, i32, usize) -> usize = core::mem::transmute(SOUND2D);
        let snd = new(SOUND2D_BYTES);
        if snd != 0 {
            // As stock: priority 8, volume 0, 4, no callback.
            ctor(snd, name, 8, 0, 4, 0);
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

pub fn functions_stock() -> bool {
    super::starts(OPERATOR_NEW, &[0x55, 0x8b, 0xec, 0xa1]) && super::starts(SOUND2D, &[0x55, 0x8b, 0xec, 0x6a, 0xff])
}
