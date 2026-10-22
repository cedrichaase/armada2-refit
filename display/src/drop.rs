//! Draws accepted and not passed on, by caller (`[Drop]` in `Display8.ini`): how a draw
//! the inventory lists is identified on screen (a shot with and without it), and how
//! the takeover of the engine's last draws is proved complete before the stub exists.
//!
//!     [Drop]
//!     Callers=0x6a1234, 0x5f00ab    ; return addresses into Armada2.exe
//!
//! A draw is dropped when any return address on its call chain (`trace::walk`: the
//! exact one and those found up the stack) is one of these. The key is read again
//! while the game runs (every 60 presents), so one session can shoot a frame with
//! and without a draw.
use crate::rt::Ini;
use crate::trace;
use crate::{log, rt};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

static mut CALLERS: Vec<usize> = Vec::new();
static mut EXE: (usize, usize) = (0, 0);
static mut PRESENTS: u32 = 0;
/// Draws dropped so far.
pub static DROPPED: AtomicU32 = AtomicU32::new(0);

fn parse(list: &str) -> Vec<usize> {
    list.split([',', ' ', '\t'])
        .filter(|s| !s.is_empty())
        .filter_map(|s| rt::parse_i32(s).map(|n| n as u32 as usize))
        .collect()
}

#[allow(static_mut_refs)]
pub fn load(ini: &Ini) {
    let v = ini.string("Drop", "Callers").map(|s| parse(&s)).unwrap_or_default();
    unsafe {
        if v == CALLERS {
            return;
        }
        EXE = trace::exe_range();
        log!("drop: draws called through any of {} return addresses are not passed on ({} dropped so far)", v.len(), DROPPED.load(Ordering::Relaxed));
        CALLERS = v;
    }
}

/// At each `Present`: `[Drop]` read again every 60.
pub fn present() {
    unsafe {
        PRESENTS = PRESENTS.wrapping_add(1);
        if PRESENTS % 60 == 0 {
            load(&Ini::open(&rt::game_path("Display8.ini")));
        }
    }
}

/// Whether the draw whose method has frame pointer `fp` is to be dropped.
#[allow(static_mut_refs)]
pub fn dropped(fp: usize) -> bool {
    let callers = unsafe { &CALLERS };
    if callers.is_empty() {
        return false;
    }
    let chain = trace::walk(fp, unsafe { EXE }, true);
    if chain.iter().any(|a| *a != 0 && callers.contains(a)) {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return true;
    }
    false
}
