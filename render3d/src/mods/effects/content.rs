//! The effects module's built-in tables (README.md, "Content"): data only, no logic.
//! Phase 8f turns these into the per-(game, build) content profile.

/// The weapons whose `CreateShieldHit` calls are tapped, in the order of the sites in
/// `hooks.rs`.
pub const SITE_NAME: [&str; 4] = ["beam", "shot", "mine", "missile"];

/// The factions a shield's colour is keyed by (`[Shields]` has one key of each name),
/// and the colours in 0..255 they default to.
pub const NFAC: usize = 7;
pub const FAC_KEY: [&str; NFAC] = ["Federation", "Klingon", "Romulan", "Borg", "Cardassian", "Species8472", "Other"];
pub const FAC_DEF: [[f32; 3]; NFAC] =
    [[90.0, 170.0, 255.0], [12.0, 255.0, 6.0], [90.0, 255.0, 130.0], [110.0, 255.0, 120.0], [255.0, 200.0, 80.0], [255.0, 140.0, 40.0], [140.0, 200.0, 255.0]];
