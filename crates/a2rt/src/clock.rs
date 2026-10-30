//! The animation clock's switch (`[Bench] Clock=`; render3d's and ui's README.md, "The
//! animation clock"), the part render3d and the UI share. Each plugin's own `clock.rs`
//! reads the key from its ini through `load` and says what the bench clock does there,
//! and gives `anim_now` the simulation's time.
//!
//! `Clock=wall` (the default) is the player's: animations run on the wall clock.
//! `Clock=bench` is the test bench's: an animation runs on the simulation's time
//! (`Get_Time`), which stands while the game is paused, and a cosmetic seed is a fixed
//! number, so two sessions stopped at one tick draw the same frame.
use crate::{log, Ini};
use core::sync::atomic::{AtomicBool, Ordering};

static BENCH: AtomicBool = AtomicBool::new(false);

/// Reads `[Bench] Clock=` of `ini` once, at start; `bench` is what the log says the
/// bench clock does in this plugin.
pub fn load(ini: &Ini, bench: &str) {
    let v = ini.string("Bench", "Clock").unwrap_or_default();
    let v = v.trim();
    let on = v.eq_ignore_ascii_case("bench");
    BENCH.store(on, Ordering::Relaxed);
    if on {
        log!("clock: bench: {}", bench);
    } else if !v.is_empty() && !v.eq_ignore_ascii_case("wall") {
        log!("clock: Clock={} is not wall or bench: the wall clock", v);
    }
}

/// Whether the bench clock is on.
pub fn bench() -> bool {
    BENCH.load(Ordering::Relaxed)
}

/// Seconds, for anything that moves on screen: the wall clock, or under the bench
/// clock `sim`, the simulation's time (which stands while the game is paused).
#[inline]
pub fn anim_now(sim: impl FnOnce() -> f64) -> f64 {
    if bench() { sim() } else { crate::now() }
}

/// A seed for a cosmetic random sequence: from the wall clock, or a fixed one under
/// the bench clock.
pub fn seed() -> u32 {
    if bench() { 0x2545_f491 } else { (crate::now() * 1000.0) as u64 as u32 }
}
