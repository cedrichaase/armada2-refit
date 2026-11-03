//! The hulls module's built-in tables: names of the game's data (SOD nodes, sprites,
//! the SOD of the Evolver's cocoon) and stock's own numbers it reproduces. Nothing here
//! is an address (those are `hooks.rs`'s) and nothing is drawing (that is the module's).

/// The Evolver's cocoon: the SOD of the class the engine keeps (`8472_cocoon.sod`).
pub const COCOON_SOD: &str = "8472_cocoon";

/// A class's ODF name ends in this; the SOD is named after the stem.
pub const ODF_EXT: &str = ".odf";

/// The node a SOD's hardpoints hang from: hidden when a model is shown clean.
pub const HARDPOINTS_NODE: &str = "hardpoints";

/// The node of a SOD's running lights (either spelling): hidden in a clean model of a
/// low level of detail, and in a build ghost.
pub const LIGHTS_NODES: [&str; 2] = ["lights", "light"];

/// The nodes of a model with no class behind it (the menu scene) whose children are
/// the systems' damage effects and the crew fires.
pub const DAMAGE_PARENT_NODES: [&str; 8] = ["life", "engines", "weapons", "shield", "shields", "sensors", "crew", "damage"];

/// The Borg node of such a model, hidden too.
pub const BORG_NODE: &str = "borg";

/// A freighter's cargo markings hang from this node (`Freighter::InitializeGeometry`).
pub const ORE_NODE: &str = "ore";

/// The sprite node of a scrap object's glow (the dilithium moons).
pub const SCRAP_GLOW_SPRITE: &str = "glowdmoon";

/// `ResourceComponent::s_ResourceColor` as stock initialises it (crew, officers,
/// dilithium, latinum, metal, biomatter), for when `InitResourceComponents` has not run.
pub const RESOURCE_COLOURS_STOCK: [[f32; 3]; 6] =
    [[0.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.6, 0.9, 1.0], [1.0, 1.0, 0.0], [0.8, 0.8, 0.8], [1.0, 0.0, 0.0]];
