//! The game's own display mode, raised to the desktop's (`[UI] Native`).
//!
//! `GameConfiguration::LoadProfile` reads the mode from `ARMADA.PRF` into the profile
//! (`UserProfile`, 0x76b5ac) at +0x50 (width), +0x54 (height) and +0x58 (depth), and
//! `GraphicsSettings::CommandLineOverride` writes a mode given on the command line into
//! the same fields. Without a profile they hold `GraphicsSettings`' defaults, 800x600x16;
//! GOG's `ARMADA.PRF` ships 1024x768x32. `WinMain` then calls
//! `GameConfiguration::InitializeGraphicsSystem` (the call at 0x484169), which opens the
//! device at that mode. That call is ours: when the profile still holds one of the two
//! stock modes, the desktop's mode goes in first, as a command-line resolution would.
//! The shell is raised to the desktop too (`dialogs::raise_shell_mode`), so the menus,
//! the loading screen and a mission share one mode and the device is never re-created
//! between them. Any other mode, chosen in Graphics Settings, is kept; the game saves
//! the mode in use into `ARMADA.PRF` on exit, as always.
use super::{eng, hook as patch, rd};
use crate::rt::win::*;
use crate::syms_core::{field, hook, sym, value};

// `GameConfiguration::InitializeGraphicsSystem()` is thiscall; `UserProfile` is the
// profile's global (`ui.core.toml`, hook `core.display`).
/// The modes a profile holds when the player never chose one.
const STOCK: [(u32, u32); 2] = [
    (value::CORE_DEFAULT_MODE_W as u32, value::CORE_DEFAULT_MODE_H as u32),
    (value::CORE_GOG_MODE_W as u32, value::CORE_GOG_MODE_H as u32),
];

pub fn install() -> bool {
    if eng().ready(hook::CORE_DISPLAY).is_err() {
        return false;
    }
    patch::patch_call(eng().addr(sym::CORE_SITE_INIT_GRAPHICS), eng().addr(sym::CORE_INIT_GRAPHICS), init as *const () as usize)
}

unsafe extern "thiscall" fn init(cfg: usize) -> u8 {
    let up = eng().addr(sym::CORE_USER_PROFILE);
    if cfg != 0 && up != 0 && cfg == unsafe { rd::<usize>(up) } {
        let (w, h) = unsafe { (rd::<u32>(cfg + field::CORE_PROFILE_MODE_W), rd::<u32>(cfg + field::CORE_PROFILE_MODE_H)) };
        let (dw, dh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        if dw <= 0 || dh <= 0 {
            crate::log!("display: the desktop's size is unknown: the game's mode stays {}x{}", w, h);
        } else if !STOCK.contains(&(w, h)) {
            crate::log!("display: the game's mode is {}x{}, chosen in Graphics Settings: kept", w, h);
        } else if (w, h) != (dw as u32, dh as u32) {
            unsafe {
                *((cfg + field::CORE_PROFILE_MODE_W) as *mut u32) = dw as u32;
                *((cfg + field::CORE_PROFILE_MODE_H) as *mut u32) = dh as u32;
                *((cfg + field::CORE_PROFILE_MODE_BPP) as *mut u32) = 32;
            }
            crate::log!("display: the game's mode {}x{} (the stock default) -> the desktop's, {}x{}x32", w, h, dw, dh);
        }
    }
    let f: unsafe extern "thiscall" fn(usize) -> u8 = unsafe { core::mem::transmute(eng().addr(sym::CORE_INIT_GRAPHICS)) };
    unsafe { f(cfg) }
}
