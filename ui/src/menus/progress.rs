//! Campaign progress for the menu scene (GitHub issue 15): how many missions of each
//! campaign the player has completed, told to render3d (`Render3D_MenuProgress`), whose
//! map then shows the ships and gas of that much progress (render3d's README, "The menu
//! scene", "Campaign progress"). Only read: nothing here writes a save or a setting.
//!
//! Two sources, the larger count winning:
//!
//! - **The mission list.** The stock list (`do_missionSelect`) offers every mission
//!   reached: the missions completed and the next one, so a campaign with `n` rows has at
//!   least `n - 1` complete (platform/README.md, "Campaign progress format": setting
//!   every counter to 1 opened two missions). Seen by `mission_select`, for the campaign
//!   the selection last pointed at, and kept for the session.
//! - **`save\shell.set`**, the game's own progress file, 78 bytes of per-campaign
//!   counters, when `[Menus] ProgressBytes` says where each campaign's counter is
//!   (`federation:<offset>,klingon:<offset>,...`). Which byte is which has not been
//!   measured yet (offset 11 is not the Federation's), so the shipped UI.ini leaves it
//!   empty and only the mission list counts. Read again at most every 2 s.
//!
//! A campaign neither source knows of is not told: render3d's map then uses its own
//! `[Progress] Default=`.
use crate::rt::win::{GetModuleHandleA, GetProcAddress};
use crate::rt::{self};
use alloc::string::String;
use alloc::vec::Vec;

type ProgressFn = unsafe extern "C" fn(*const u8, u32) -> u32;

/// render3d's function, looked up once.
static mut FN: Option<Option<ProgressFn>> = None;
/// Each campaign's count, as seen on its mission list this session.
static mut SEEN: Vec<(String, u32)> = Vec::new();
/// What render3d was last told, per campaign.
static mut TOLD: Vec<(String, u32)> = Vec::new();
/// When `shell.set` was last read (`rt::now`), and what it said.
static mut READ_AT: f64 = -1.0e9;
static mut FILE: Vec<(String, u32)> = Vec::new();

/// The most missions a campaign has (`mshell.set`: ten each, six tutorials).
const MOST: u32 = 10;

fn progress_fn() -> Option<ProgressFn> {
    unsafe {
        if let Some(f) = FN {
            return f;
        }
        let m = GetModuleHandleA(c"Render3D.asi".as_ptr() as *const u8);
        let p = if m.is_null() { 0 } else { GetProcAddress(m, c"Render3D_MenuProgress".as_ptr() as *const u8) };
        let f = if p == 0 { None } else { Some(core::mem::transmute::<usize, ProgressFn>(p)) };
        crate::log!("menus: campaign progress {}", if f.is_some() { "goes to render3d's menu scene" } else { "is not used (no Render3D_MenuProgress)" });
        FN = Some(f);
        f
    }
}

fn get(list: &[(String, u32)], c: &str) -> Option<u32> {
    list.iter().find(|e| e.0 == c).map(|e| e.1)
}

fn put(list: &mut Vec<(String, u32)>, c: &str, n: u32) {
    match list.iter_mut().find(|e| e.0 == c) {
        Some(e) => e.1 = n,
        None => list.push((String::from(c), n)),
    }
}

/// The mission list of `campaign` (a view name of the map: `federation`, ...) shows
/// `rows` missions.
#[allow(static_mut_refs)]
pub fn saw_missions(campaign: &str, rows: usize) {
    if rows == 0 {
        return;
    }
    let n = (rows as u32 - 1).min(MOST);
    let seen = unsafe { &mut SEEN };
    if get(seen, campaign).is_none_or(|was| n > was) {
        crate::log!("menus: progress {}: its mission list offers {} ({} complete at least)", campaign, rows, n);
        put(seen, campaign, n);
    }
}

/// `[Menus] ProgressBytes`: (campaign, byte offset in `shell.set`).
fn offsets() -> Vec<(String, usize)> {
    let mut v = Vec::new();
    for item in crate::config::get().menus_progress_bytes.split(',') {
        let Some((c, o)) = item.split_once(':') else { continue };
        let Ok(o) = o.trim().parse::<usize>() else { continue };
        let c = rt::lower(c.trim());
        if !c.is_empty() {
            v.push((c, o));
        }
    }
    v
}

#[allow(static_mut_refs)]
fn read_file() {
    let now = rt::now();
    unsafe {
        if now - READ_AT < 2.0 {
            return;
        }
        READ_AT = now;
    }
    let offs = offsets();
    if offs.is_empty() {
        return;
    }
    let Some(bytes) = rt::read_file(&rt::game_path("save\\shell.set")) else { return };
    let file = unsafe { &mut FILE };
    for (c, o) in offs {
        if let Some(&b) = bytes.get(o) {
            put(file, &c, (b as u32).min(MOST));
        }
    }
}

/// Before the menu scene draws: tells render3d each campaign's count where it changed.
#[allow(static_mut_refs)]
pub fn tell() {
    let Some(f) = progress_fn() else { return };
    read_file();
    let (seen, file, told) = unsafe { (&SEEN, &FILE, &mut TOLD) };
    for camp in crate::content::get().campaigns {
        let Ok(c) = camp.scene.to_str() else { continue };
        let n = match (get(seen, c), get(file, c)) {
            (Some(a), Some(b)) => a.max(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => continue,
        };
        if get(told, c) != Some(n) {
            put(told, c, n);
            unsafe { f(camp.scene.as_ptr() as *const u8, n) };
        }
    }
}
