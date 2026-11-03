//! The particles' feed (README, "Module interface", "Services"): a queue the core owns.
//! A module that draws an instance with emitters (a station being built, a worker bee,
//! a bolt with a contrail, a special weapon's effect) pushes the instance's emitters
//! here, at the transform it draws it with, and the particles module drains the queue
//! into its simulation before it draws. With no particles module the queue is never
//! filled (`live()` is false), which is the "particles off" of today: the producer hands
//! the instance to the engine, whose own draw keeps the emitters going.
//!
//! Staged in the one crate until the crates are split (`core/particles.rs` then).
use crate::frame::steps::View;
use crate::math::{M34, V3};
use alloc::string::String;
use alloc::vec::Vec;

/// One instance's emitters this frame.
pub struct Feed {
    /// Names the instance (its address will do); call it every frame it exists.
    pub key: usize,
    /// The SOD (or the special effect's) carrying the emitters.
    pub sod: String,
    /// The transform it is drawn with, scale included.
    pub world: M34,
    pub time: f32,
    pub eye: V3,
}

static mut LIVE: bool = false;
static mut QUEUE: Vec<Feed> = Vec::new();

/// Whether the particles module draws the particles in this view (`feed` does
/// something): a module that would draw an instance with emitters leaves it to the
/// engine otherwise.
pub fn live() -> bool {
    unsafe { LIVE }
}

/// The particles module's: it is (not) drawing this view's particles.
pub fn set_live(on: bool) {
    unsafe { LIVE = on };
}

/// An instance's emitters at `world`, to be simulated once this frame and drawn with
/// every other particle. Does nothing while the particles module is not live.
#[allow(static_mut_refs)]
pub fn feed(key: usize, sod: &str, world: &M34, v: &View) {
    if live() {
        unsafe { QUEUE.push(Feed { key, sod: String::from(sod), world: *world, time: v.time, eye: v.eye }) };
    }
}

/// The queue, in the order it was filled, for the particles module to take.
#[allow(static_mut_refs)]
pub fn take() -> Vec<Feed> {
    unsafe { core::mem::take(&mut QUEUE) }
}
