//! The game's `RTS_CFG.h`, for the few keys a build's profile says render3d follows
//! (Roots' `SHIELD_EFFECT_RESCALE` and `SHIELD_EFFECT_DAMAGE_COLORED`, FO-5 step 5). The
//! file is C-like: `int NAME = value;` per line, `//` comments. It is read once, at the
//! first ask, through the file system (`files`, `data\RTS_CFG.h`): the game directory's
//! own on Armada II, and on a game with mods the active mod's over its parents' over the
//! game's, the copy the game itself reads (FO-5 step 6). Nothing is written. A key that is
//! not there, or does not read as an integer, is None.
use crate::files;
use alloc::vec::Vec;

mod parse;
pub use parse::parse_int;

static mut DATA: Option<Vec<u8>> = None;
static mut READ: bool = false;

/// Where the file is, as the file system finds it.
pub const PATH: &str = "data\\RTS_CFG.h";

#[allow(static_mut_refs)]
fn data() -> &'static [u8] {
    unsafe {
        if !READ {
            READ = true;
            DATA = files::open(PATH);
            match &DATA {
                Some(d) => crate::log!("rtscfg: RTS_CFG.h read ({} bytes, {})", d.len(), files::served_by(PATH).unwrap_or_default()),
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
