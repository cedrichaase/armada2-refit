//! Campaign progress for the menu scene (GitHub issue 15): how many missions of each
//! campaign the player has completed, told to render3d (`Render3D_MenuProgress`), whose
//! map then shows the ships and gas of that much progress (render3d's README, "The menu
//! scene", "Campaign progress"). Only read: nothing here writes a save or a setting.
//!
//! The progress is the one the campaign locks read (`locks::current`, MENUS.md, "Locked
//! campaigns"): the game's memory, else `save\shell.set`. Per campaign, the furthest
//! mission reached (0-based) is the count of missions complete, plus one when the
//! campaign's flag says its last mission is won. Read again at most once a second.
//!
//! A campaign neither source knows of is not told: render3d's map then uses its own
//! `[Progress] Default=`.
use crate::rt::win::{GetModuleHandleA, GetProcAddress};
use alloc::vec::Vec;

type ProgressFn = unsafe extern "C" fn(*const u8, u32) -> u32;

/// render3d's function, looked up once.
static mut FN: Option<Option<ProgressFn>> = None;
/// What render3d was last told, per card of the content set.
static mut TOLD: Vec<Option<u32>> = Vec::new();
/// When the progress was last read (`rt::now`).
static mut READ_AT: f64 = -1.0e9;

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

/// Before the menu scene draws: tells render3d each campaign's count where it changed.
#[allow(static_mut_refs)]
pub fn tell() {
    let Some(f) = progress_fn() else { return };
    let now = crate::rt::now();
    unsafe {
        if now - READ_AT < 1.0 {
            return;
        }
        READ_AT = now;
    }
    let (Some(p), from) = super::locks::current() else { return };
    let told = unsafe { &mut TOLD };
    let camps = crate::content::get().campaigns;
    if told.len() != camps.len() {
        *told = alloc::vec![None; camps.len()];
    }
    for (i, camp) in camps.iter().enumerate() {
        let (Some(&reached), Some(&complete)) = (p.reached.get(i), p.complete.get(i)) else { continue };
        let n = reached as u32 + (complete != 0) as u32;
        if told[i] != Some(n) {
            told[i] = Some(n);
            crate::log!("menus: progress {}: {} missions complete (from {})", camp.key, n, from);
            unsafe { f(camp.scene.as_ptr() as *const u8, n) };
        }
    }
}
