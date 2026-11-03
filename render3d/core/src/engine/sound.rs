//! A sound of the game's own sound system, played once: a fire-and-forget
//! `AudioSound2D` made with the game's `operator new`, as the shell's buttons
//! (`ShellButton::SetButtonMode`) and the HUD's (`StandardButton::Simulate`) play
//! theirs. Used by the menu scene for its ships' decloak (`menu`).
use super::rd;

const OPERATOR_NEW: usize = 0x652710;
/// `AudioSound2D::AudioSound2D(name, priority, volume, ...)`, thiscall.
const SOUND2D: usize = 0x45fc50;
const SOUND2D_BYTES: usize = 0x1c;

static mut NAMES: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec::Vec::new();

fn functions_stock() -> bool {
    let starts = |a: usize, b: &[u8]| b.iter().enumerate().all(|(i, &x)| unsafe { rd::<u8>(a + i) } == x);
    starts(OPERATOR_NEW, &[0x55, 0x8b, 0xec, 0xa1]) && starts(SOUND2D, &[0x55, 0x8b, 0xec, 0x6a, 0xff])
}

/// Plays the sound file `name` (found by the game's sound system by name, as an ODF's
/// sounds are) at the shell buttons' priority and volume.
pub fn play_2d(name: &str) {
    if !functions_stock() || name.is_empty() || name.len() > 120 {
        return;
    }
    // The name stays for the program's life (the sound may keep the pointer), one copy
    // per name.
    #[allow(static_mut_refs)]
    let names = unsafe { &mut NAMES };
    let p = match names.iter().find(|n| &n[..n.len() - 1] == name.as_bytes()) {
        Some(n) => n.as_ptr(),
        None => {
            let mut n = alloc::vec::Vec::with_capacity(name.len() + 1);
            n.extend_from_slice(name.as_bytes());
            n.push(0);
            names.push(n);
            names.last().map_or(core::ptr::null(), |n| n.as_ptr())
        }
    };
    unsafe {
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(OPERATOR_NEW);
        let ctor: unsafe extern "thiscall" fn(usize, *const u8, i32, i32, i32, usize) -> usize = core::mem::transmute(SOUND2D);
        let snd = new(SOUND2D_BYTES);
        if snd != 0 {
            // As the shell's buttons: priority 8, volume 0, 0, no callback.
            ctor(snd, p, 8, 0, 0, 0);
        }
    }
}
