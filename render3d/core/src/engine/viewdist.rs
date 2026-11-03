//! View distance (`View.Distance`, moved from QOL's `ViewDistance`; README.md, "View
//! distance"). Two values of ART_CFG.h end the view: the far plane `PresetView::Init`
//! gives the game camera, and the object culling distance, past 0.9 of which every
//! object fades out. `RTS_Configure` parses both and then loads the culling distance to
//! copy it to the next float (what a mission script restores). That load becomes
//! `call stub; nop`: the stub raises both values to the setting and loads ecx as the
//! `mov` did. Both are only read to draw, so this stays stock-compatible.
//!
//! Standing down with `Enable=0` leaves the engine's values (QOL patched regardless).
use crate::hooks::Hooks;
use crate::syms::{hook, sym, TABLE};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// The setting in force (f32 bits); 0 leaves the engine's values.
static DISTANCE: AtomicU32 = AtomicU32::new(0);
/// The two values ART_CFG.h gave, and what this module last stored in them.
static FAR_ORIG: AtomicU32 = AtomicU32::new(0);
static CULL_ORIG: AtomicU32 = AtomicU32::new(0);
static FAR_SET: AtomicU32 = AtomicU32::new(0);
static CULL_SET: AtomicU32 = AtomicU32::new(0);
/// A parse has stored the originals.
static PARSED: AtomicBool = AtomicBool::new(false);
static PATCHED: AtomicBool = AtomicBool::new(false);
/// The culling distance's address, for the stub (it loads ecx through it).
static mut CULL_ADDR: usize = 0;

fn f(a: &AtomicU32) -> f32 { f32::from_bits(a.load(Relaxed)) }
fn put(a: &AtomicU32, v: f32) { a.store(v.to_bits(), Relaxed) }

fn far_ptr() -> *mut f32 { crate::engine::at(sym::VIEW_FAR_PLANE) as *mut f32 }
fn cull_ptr() -> *mut f32 { crate::engine::at(sym::VIEW_CULL_DIST) as *mut f32 }

/// The distance the ini asks for (`[View] Distance`; 0 is stock).
fn wanted(ini: &crate::rt::Ini) -> f32 {
    let d = ini.float("View", "Distance", 1_000_000.0);
    if d > 0.0 { d } else { 0.0 }
}

/// Called by the stub after each parse of ART_CFG.h.
unsafe extern "C" fn parsed() {
    let (fp, cp) = (far_ptr(), cull_ptr());
    unsafe {
        put(&FAR_ORIG, *fp);
        put(&CULL_ORIG, *cp);
        PARSED.store(true, Relaxed);
        let d = f(&DISTANCE);
        if *fp < d { *fp = d; }
        if *cp < d { *cp = d; }
        put(&FAR_SET, *fp);
        put(&CULL_SET, *cp);
        if d > 0.0 {
            crate::log!("view: ART_CFG.h parsed: far plane now {}, object culling now {}", *fp as i64, *cp as i64);
        }
    }
}

/// Stands in for `mov ecx, [culling distance]`: every register but ecx is kept.
#[unsafe(naked)]
unsafe extern "C" fn stub() {
    core::arch::naked_asm!(
        "pushad",
        "pushfd",
        "call {p}",
        "popfd",
        "popad",
        "mov ecx, dword ptr [{a}]",
        "mov ecx, dword ptr [ecx]",
        "ret",
        p = sym parsed,
        a = sym CULL_ADDR,
    );
}

/// Installs the patch (the hook's checks passed, the site is the stock load). Called
/// once, with `Enable` on.
pub fn install() {
    let mut h = Hooks::core();
    if !h.ready(&TABLE, hook::VIEW_DISTANCE) {
        log_down();
        return;
    }
    let (site, cull) = (crate::engine::at(sym::VIEW_SITE), cull_ptr() as usize);
    if site == 0 || cull == 0 || far_ptr().is_null() {
        log_down();
        return;
    }
    DISTANCE.store(wanted(&crate::config::ini()).to_bits(), Relaxed);
    // `mov ecx, [abs32]`, six bytes: the original, kept to check before writing.
    let mut orig = [0x8Bu8, 0x0D, 0, 0, 0, 0];
    orig[2..].copy_from_slice(&(cull as u32).to_le_bytes());
    let mut new = [0xE8u8, 0, 0, 0, 0, 0x90];
    let rel = (stub as *const () as usize as i64 - (site as i64 + 5)) as i32;
    new[1..5].copy_from_slice(&rel.to_le_bytes());
    unsafe { CULL_ADDR = cull };
    if crate::rt::hook::patch_bytes(site, &orig, &new) {
        PATCHED.store(true, Relaxed);
        crate::log!("view: Distance={} -> far plane and object culling raised at the next parse, patched", f(&DISTANCE) as i64);
    } else {
        log_down();
    }
}

fn log_down() {
    crate::log!("view: the load of the culling distance is not stock's: View.Distance not applied");
}

/// A settings change (the overlay or an ini): the new distance, applied to the values
/// now in force. Returns the log text of what moved, or an empty string.
pub fn settings_changed() -> alloc::string::String {
    let mut s = alloc::string::String::new();
    let d = wanted(&crate::config::ini());
    if d == f(&DISTANCE) {
        return s;
    }
    put(&DISTANCE, d);
    if PATCHED.load(Relaxed) && PARSED.load(Relaxed) {
        let (fp, cp) = (far_ptr(), cull_ptr());
        let nf = if d > f(&FAR_ORIG) { d } else { f(&FAR_ORIG) };
        let nc = if d > f(&CULL_ORIG) { d } else { f(&CULL_ORIG) };
        unsafe {
            if *fp == f(&FAR_SET) { *fp = nf; }
            if *cp == f(&CULL_SET) {
                *cp = nc;
                *cp.add(1) = nc; // the copy a mission script restores from
            }
        }
        put(&FAR_SET, nf);
        put(&CULL_SET, nc);
    }
    s.push_str(&crate::sfmt!("View.Distance={}", d as i64));
    s
}
