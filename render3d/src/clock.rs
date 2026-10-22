//! The animation clock (`[Bench] Clock=`, README.md "The animation clock").
//!
//! Everything that moves on screen and is not simulated by the engine reads its time
//! from `anim_now()`, never from `rt::now()`, and every cosmetic random sequence takes
//! its seed from `seed()`. `rt::now()` stays for what is not seen: timing logs, how
//! often a file is looked at again, a cache's age.
//!
//! `Clock=wall` (the default) is the player's: animations run on the wall clock, as
//! they always did, and a paused game still flickers and pulses. `Clock=bench` is the
//! test bench's: `anim_now()` is the simulation's time (`Get_Time`), so a paused
//! simulation freezes every animation, a seed is a fixed number, and the frame-to-frame
//! sequences (the nebulae's `Temporal=` jitter) settle on one picture while the clock
//! stands. Two sessions of one scene paused at one tick then draw the same frame.
use crate::log;
use core::sync::atomic::{AtomicBool, Ordering};

static BENCH: AtomicBool = AtomicBool::new(false);

/// Reads `[Bench] Clock=` once, at start.
pub fn load() {
    let ini = crate::config::ini();
    let v = ini.string("Bench", "Clock").unwrap_or_default();
    let v = v.trim();
    let bench = v.eq_ignore_ascii_case("bench");
    BENCH.store(bench, Ordering::Relaxed);
    if bench {
        log!("clock: bench: animations on the simulation's time, fixed seeds, the nebulae's frames settle while it stands");
    } else if !v.is_empty() && !v.eq_ignore_ascii_case("wall") {
        log!("clock: Clock={} is not wall or bench: the wall clock", v);
    }
}

/// Whether the bench clock is on.
pub fn bench() -> bool {
    BENCH.load(Ordering::Relaxed)
}

/// Seconds, for anything that moves on screen: the wall clock, or under the bench
/// clock the simulation's time (which stands while the game is paused).
pub fn anim_now() -> f64 {
    if bench() { crate::engine::sim_time() as f64 } else { crate::rt::now() }
}

/// A seed for a cosmetic random sequence: from the wall clock, or a fixed one under
/// the bench clock.
pub fn seed() -> u32 {
    if bench() { 0x2545_f491 } else { (crate::rt::now() * 1000.0) as u64 as u32 }
}

/// What a per-object cosmetic seed is made from: the object's address under the wall
/// clock (as always), and under the bench clock its handle (+0x28 of every `Entity`,
/// given out in order as objects are made), shifted to where an address's bits are: the
/// heap's addresses differ from launch to launch, the handles do not.
pub fn entity_key(e: usize) -> usize {
    if bench() && e != 0 { crate::engine::field(e, 0x28) << 4 } else { e }
}
