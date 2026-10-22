//! The animation clock (`[Bench] Clock=`, README.md "The animation clock").
//!
//! Everything that moves on screen reads its time from `Ui::time()`, which is this
//! clock, or from `anim_now()` outside a frame of widgets; never from `rt::now()`.
//! `rt::now()` stays for what is not seen: logs, how long ago a panel last drew, how
//! long a screen has been up before it takes a click.
//!
//! `Clock=wall` (the default) is the player's. `Clock=bench` is the test bench's: the
//! clock is the simulation's time (`Get_Time`), so a paused mission freezes the caret,
//! the spinner and the cursor's animation, and transitions (hover fades, a panel
//! opening) are settled at once (`widgets::anim`), since the front end has no
//! simulation to move them. Two sessions stopped at one tick then draw the same frame.
use crate::log;
use core::sync::atomic::{AtomicBool, Ordering};

static BENCH: AtomicBool = AtomicBool::new(false);

/// `Get_Time`'s float: the simulation's time in seconds, which stands while paused.
const SIM_TIME: usize = 0x7b_0fc8;

/// Reads `[Bench] Clock=` once, at start.
pub fn load() {
    let ini = crate::rt::Ini::open(&crate::rt::game_path("UI.ini"));
    let v = ini.string("Bench", "Clock").unwrap_or_default();
    let v = v.trim();
    let bench = v.eq_ignore_ascii_case("bench");
    BENCH.store(bench, Ordering::Relaxed);
    if bench {
        log!("clock: bench: animations on the simulation's time, transitions settled at once");
    } else if !v.is_empty() && !v.eq_ignore_ascii_case("wall") {
        log!("clock: Clock={} is not wall or bench: the wall clock", v);
    }
}

/// Whether the bench clock is on.
pub fn bench() -> bool {
    BENCH.load(Ordering::Relaxed)
}

/// Seconds, for anything that moves on screen: the wall clock, or under the bench
/// clock the simulation's time.
pub fn anim_now() -> f64 {
    if bench() { unsafe { crate::engine::rd::<f32>(SIM_TIME) as f64 } } else { crate::rt::now() }
}
