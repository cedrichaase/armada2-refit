//! The 3D scene behind the main menu and the campaign selection (`[Menus] Scene=1`): a
//! fleet in orbit over a planet, drawn by render3d (`Render3D_MenuScene`, render3d's
//! README, "The menu scene") into the back buffer at the start of the menus' frame,
//! before anything of ours. The screen specs then draw over it (`shown`) in place of
//! the stock screen's art.
//!
//! Without Render3D.asi, or when it draws nothing (no Direct3D 9 device, its scene
//! switched off), `shown` is false and the specs draw what they drew before.
use crate::rt::win::{GetModuleHandleA, GetProcAddress};

type SceneFn = unsafe extern "C" fn(*const u8, u32, u32, u32, u32) -> u32;

/// render3d's function, looked up once (the ASI loader loads it before the first menu).
static mut FN: Option<Option<SceneFn>> = None;
/// The scene drew under this frame.
static mut SHOWN: bool = false;

fn scene_fn() -> Option<SceneFn> {
    unsafe {
        if let Some(f) = FN {
            return f;
        }
        let m = GetModuleHandleA(c"Render3D.asi".as_ptr() as *const u8);
        let p = if m.is_null() { 0 } else { GetProcAddress(m, c"Render3D_MenuScene".as_ptr() as *const u8) };
        let f = if p == 0 { None } else { Some(core::mem::transmute::<usize, SceneFn>(p)) };
        crate::log!("menus: the menu scene {}", if f.is_some() { "is render3d's" } else { "is not there (no Render3D.asi): the stock art" });
        FN = Some(f);
        f
    }
}

/// The scene a screen stands over, by its name in `engine::dialogs`, or None.
fn scene_for(screen: &str) -> Option<&'static core::ffi::CStr> {
    match screen {
        "main menu" => Some(c"main"),
        "single player" => Some(c"single"),
        _ => None,
    }
}

/// The scene this frame stands over and render3d's function, if any.
fn wanted() -> Option<(&'static core::ffi::CStr, SceneFn)> {
    if !crate::config::get().menus_scene || super::backdrop::over_mission() {
        return None;
    }
    let all = crate::engine::dialogs::stack();
    let full = all.iter().rposition(|s| s.w() >= crate::engine::dialogs::DESIGN_W && s.h() >= crate::engine::dialogs::DESIGN_H)?;
    let name = scene_for(crate::engine::dialogs::screen_name(all[full].proc_addr))?;
    Some((name, scene_fn()?))
}

/// At the start of the menus' frame, inside the scene, the back buffer `w` x `h` bound
/// and cleared: the scene of the full-screen dialog at the bottom of what shows.
pub fn draw(w: u32, h: u32) {
    unsafe { SHOWN = false };
    let Some((name, f)) = wanted() else {
        note("");
        return;
    };
    let drew = unsafe { f(name.as_ptr() as *const u8, 0, 0, w, h) } != 0;
    unsafe { SHOWN = drew };
    note(if drew { name.to_str().unwrap_or("") } else { "" });
}

/// The scene last logged ("" for none). The bench reads these lines: over the scene
/// the stock art is gone, so it clicks the main menu's entries by their labels.
static mut LOGGED: Option<&'static str> = None;

fn note(name: &'static str) {
    unsafe {
        if LOGGED != Some(name) {
            LOGGED = Some(name);
            if name.is_empty() {
                crate::log!("menus: scene off");
            } else {
                crate::log!("menus: scene {} on", name);
            }
        }
    }
}

/// Whether the scene is under this frame: the spec draws over it, not the stock art.
pub fn shown() -> bool {
    unsafe { SHOWN }
}
