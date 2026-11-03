//! Render-side randomness: a hash of a seed, never `g_random` (nothing the simulation
//! reads is drawn from it; README, "Render only").

/// 32-bit integer hash (the plugins' `hash32`).
pub fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

/// 0..1 from a seed.
pub fn rnd(seed: u32) -> f32 {
    (hash32(seed) & 0xffffff) as f32 / 16777216.0
}
