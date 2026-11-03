//! The particles module's built-in tables (README.md, "Content"): names of game data,
//! never addresses.

/// The engine's damage-node names (`CraftClass::InitializeDamageNodes`, 0x4bfc70): the
/// first node of each in a SOD (case-insensitive), in the order of `hooks`' bytes.
pub const SYSTEMS: [&str; 6] = ["shields", "engines", "target", "sensors", "life", "borg"];

/// The vents' sprites, by default; `[Particles] PlasmaSprite=` and `GasSprite=` name others.
pub const VENT_SPRITES: [&str; 2] = ["ventplas", "ventgas"];

/// The free emitters the chain pulsar and the Manheim effect each fire.
pub const CHAIN_EMITTER: &str = "crpulsar";
pub const MANHEIM_EMITTER: &str = "gemini";

/// A SOD name as the engine writes it; the extension is dropped for the asset cache.
pub const SOD_EXT: &str = ".sod";
