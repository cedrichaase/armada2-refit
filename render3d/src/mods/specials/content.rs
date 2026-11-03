//! The specials module's built-in tables: the three effect kinds it gives a look to,
//! and the look's shipped defaults (`[Specials]` overrides each key).

/// The kinds, by `look::kind`'s index, as `[Specials]` spells their keys: Computer
/// Override, the Energy Dissipator's drain, Psionic Disruption.
pub const NAMES: [&str; 3] = ["Override", "Drain", "Psionic"];

/// One kind's defaults: (colour 0..255, gain, strength, creep s, scale, rim, light
/// colour 0..255, light brightness, light range in the victim's radii).
pub type Defaults = ([f32; 3], f32, f32, f32, f32, f32, [f32; 3], f32, f32);

pub const DEFAULTS: [Defaults; 3] = [
    ([60.0, 255.0, 110.0], 1.6, 1.0, 2.5, 14.0, 0.15, [80.0, 255.0, 120.0], 1.6, 3.0),
    ([235.0, 80.0, 190.0], 1.4, 0.9, 1.5, 10.0, 0.3, [235.0, 90.0, 200.0], 1.0, 2.5),
    ([150.0, 110.0, 255.0], 1.6, 1.0, 0.8, 3.0, 0.6, [160.0, 110.0, 255.0], 1.4, 3.0),
];
