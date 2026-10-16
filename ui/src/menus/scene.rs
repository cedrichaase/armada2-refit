//! The 3D scene behind the front end's screens (`[Menus] Scene=1`):
//! render3d's map (`Render3D_MenuScene`, render3d's README, "The menu scene"), drawn
//! into the back buffer at the start of the menus' frame, before anything of ours. The
//! main menu shows its view `main`; the campaign selection the view of the campaign
//! last pointed at (`choose`), the Federation's at first, and render3d moves the camera
//! over. The screen specs then draw over it (`shown`) in place of the stock screen's art.
//!
//! Without Render3D.asi, or when it draws nothing (no Direct3D 9 device, its scene
//! switched off), `shown` is false and the specs draw what they drew before.
//!
//! **The departure.** A mission loaded while the scene is up (from the campaign
//! selection, or a save from a screen over it) keeps it under the loading screen: the
//! view that showed last is asked of render3d's `Render3D_MenuWarp`, whose ships go to
//! warp with the camera following them into the warp's tunnel (`begin_departure` when
//! the load's splash goes up, `draw_departure` in each loading frame).
use crate::rt::win::{GetModuleHandleA, GetProcAddress};

type SceneFn = unsafe extern "C" fn(*const u8, u32, u32, u32, u32) -> u32;

/// render3d's functions, looked up once (the ASI loader loads it before the first menu).
static mut FN: Option<Option<SceneFn>> = None;
static mut WARP_FN: Option<Option<SceneFn>> = None;
/// The scene drew under this frame, and the view it drew.
static mut SHOWN: bool = false;
static mut VIEW: Option<&'static core::ffi::CStr> = None;
/// This load's departure: the view it leaves from (None: no departure).
static mut DEPART: Option<&'static core::ffi::CStr> = None;
/// The departure drew under this loading frame.
static mut DEPART_SHOWN: bool = false;
/// The campaign selection's view, while it shows (None: the first, the Federation's).
static mut CHOICE: Option<&'static core::ffi::CStr> = None;

/// The campaign selection's spec: the view to show for the campaign pointed at.
pub fn choose(view: &'static core::ffi::CStr) {
    unsafe { CHOICE = Some(view) };
}

/// The view the campaign selection last chose (None: none yet, the Federation's).
pub fn chosen() -> Option<&'static core::ffi::CStr> {
    unsafe { CHOICE }
}

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

/// The scene a screen stands over, by its name in `engine::dialogs`: the main menu's
/// view, the campaign selection's campaign, and for every other full-screen screen of
/// the front end (Options, Instant Action, Multiplayer and its lobby, Load Game, ...)
/// the view that showed last, so the camera stays where the screen was opened from.
/// The specs dim it under their windows (`theme::backdrop`).
fn scene_for(screen: &str) -> Option<&'static core::ffi::CStr> {
    match screen {
        "main menu" => Some(c"main"),
        "single player" => Some(unsafe { CHOICE }.unwrap_or(c"federation")),
        _ => Some(unsafe { LAST_VIEW }.unwrap_or(c"main")),
    }
}

/// The view the scene last drew, kept across screens without one of their own.
static mut LAST_VIEW: Option<&'static core::ffi::CStr> = None;

/// The scene this frame stands over and render3d's function, if any.
fn wanted() -> Option<(&'static core::ffi::CStr, SceneFn)> {
    if !crate::config::get().menus_scene || super::backdrop::over_mission() {
        return None;
    }
    let all = crate::engine::dialogs::stack();
    let full = all.iter().rposition(|s| s.w() >= crate::engine::dialogs::DESIGN_W && s.h() >= crate::engine::dialogs::DESIGN_H)?;
    let screen = crate::engine::dialogs::screen_name(all[full].proc_addr);
    if screen != "single player" {
        // Each visit to the campaign selection starts at the Federation's view.
        unsafe { CHOICE = None };
    }
    let name = scene_for(screen)?;
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
    unsafe {
        SHOWN = drew;
        VIEW = if drew { Some(name) } else { None };
        if drew {
            LAST_VIEW = Some(name);
        }
    }
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

/// Draws the view `name` of render3d's map into the whole back buffer, `w` x `h`, for
/// a screen of ours (the Credits). False when it drew nothing.
pub fn draw_view(name: &core::ffi::CStr, w: u32, h: u32) -> bool {
    match scene_fn() {
        Some(f) => unsafe { f(name.as_ptr() as *const u8, 0, 0, w, h) != 0 },
        None => false,
    }
}

/// Whether the scene is under this frame: the spec draws over it, not the stock art.
pub fn shown() -> bool {
    unsafe { SHOWN }
}

fn warp_fn() -> Option<SceneFn> {
    unsafe {
        if let Some(f) = WARP_FN {
            return f;
        }
        let m = GetModuleHandleA(c"Render3D.asi".as_ptr() as *const u8);
        let p = if m.is_null() { 0 } else { GetProcAddress(m, c"Render3D_MenuWarp".as_ptr() as *const u8) };
        let f = if p == 0 { None } else { Some(core::mem::transmute::<usize, SceneFn>(p)) };
        WARP_FN = Some(f);
        f
    }
}

/// A mission's load has begun: it departs from the scene if the scene was under the
/// last frame of the menus (taken once: a later load straight from a mission, the next
/// of a campaign, does not).
pub fn begin_departure() {
    unsafe {
        DEPART = if SHOWN && warp_fn().is_some() { VIEW } else { None };
        SHOWN = false;
        VIEW = None;
        DEPART_SHOWN = false;
        if let Some(v) = DEPART {
            crate::log!("menus: the load departs from the scene {}", v.to_str().unwrap_or(""));
        }
    }
}

/// In a loading frame, inside its scene with the back buffer `w` x `h` bound, before
/// the loading screen draws: the departure, if this load has one.
pub fn draw_departure(w: u32, h: u32) {
    let drew = match (unsafe { DEPART }, warp_fn()) {
        (Some(v), Some(f)) => (unsafe { f(v.as_ptr() as *const u8, 0, 0, w, h) }) != 0,
        _ => false,
    };
    unsafe { DEPART_SHOWN = drew };
}

/// Whether the departure is under this loading frame: the loading screen draws over
/// it in place of the map's sky.
pub fn departure_shown() -> bool {
    unsafe { DEPART_SHOWN }
}
