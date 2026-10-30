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
pub use crate::rt::clock::bench;

/// `Get_Time`'s float: the simulation's time in seconds, which stands while paused.
const SIM_TIME: usize = 0x7b_0fc8;

/// Reads `[Bench] Clock=` once, at start (`rt::clock`).
pub fn load() {
    let ini = crate::rt::Ini::open(&crate::rt::game_path("UI.ini"));
    crate::rt::clock::load(&ini, "animations on the simulation's time, transitions settled at once");
}

/// Seconds, for anything that moves on screen: the wall clock, or under the bench
/// clock the simulation's time.
pub fn anim_now() -> f64 {
    crate::rt::clock::anim_now(|| unsafe { crate::engine::rd::<f32>(SIM_TIME) as f64 })
}
