//! The game's `RTS_CFG.h`, for the few keys a build's profile says render3d follows
//! (Roots' `SHIELD_EFFECT_RESCALE` and `SHIELD_EFFECT_DAMAGE_COLORED`, FO-5 step 5). The
//! file is C-like: `int NAME = value;` per line, `//` comments. It is read from the game
//! directory once, at the first ask; nothing is written. A key that is not there, or
//! does not read as an integer, is None.
use crate::rt;
use alloc::vec::Vec;

mod parse;
pub use parse::parse_int;

static mut DATA: Option<Vec<u8>> = None;
static mut READ: bool = false;

#[allow(static_mut_refs)]
fn data() -> &'static [u8] {
    unsafe {
        if !READ {
            READ = true;
            DATA = rt::read_file(&rt::game_path("RTS_CFG.h"));
            match &DATA {
                Some(d) => crate::log!("rtscfg: RTS_CFG.h read ({} bytes)", d.len()),
                None => crate::log!("rtscfg: no RTS_CFG.h in the game directory"),
            }
        }
        DATA.as_deref().unwrap_or(&[])
    }
}

/// The integer `name` is set to in the game's `RTS_CFG.h` (the last setting wins).
pub fn int(name: &str) -> Option<i32> {
    parse_int(data(), name)
}
