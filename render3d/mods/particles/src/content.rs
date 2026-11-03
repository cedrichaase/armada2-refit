//! The particles module's built-in tables (README.md, "Content"): names of game data,
//! never addresses, one set per content profile (`profiles/*/content.toml`). A build
//! with no set gets `NONE`: no names, so no vent, pulsar or damage-node emitter is told.

pub struct Content {
    /// The engine's damage-node names (`CraftClass::InitializeDamageNodes`): the first
    /// node of each in a SOD (case-insensitive), in the order of `hooks`' bytes.
    pub systems: &'static [&'static str],
    /// The vents' sprites, by default (plasma, gas); `[Particles] PlasmaSprite=` and
    /// `GasSprite=` name others.
    pub vent_sprites: [&'static str; 2],
    /// The free emitters the chain pulsar and the Manheim effect each fire.
    pub chain_emitter: &'static str,
    pub manheim_emitter: &'static str,
    /// A SOD name as the engine writes it; the extension is dropped for the asset cache.
    pub sod_ext: &'static str,
}

pub static STOCK: Content = Content {
    systems: &["shields", "engines", "target", "sensors", "life", "borg"],
    vent_sprites: ["ventplas", "ventgas"],
    chain_emitter: "crpulsar",
    manheim_emitter: "gemini",
    sod_ext: ".sod",
};

/// A build with no content set for this module.
pub static NONE: Content =
    Content { systems: &[], vent_sprites: ["", ""], chain_emitter: "", manheim_emitter: "", sod_ext: "" };

/// The set this build reads.
pub fn get() -> &'static Content {
    match crate::syms::TABLE.engine().content() {
        Some(id) => by_set(id),
        None => &NONE,
    }
}

fn by_set(id: &str) -> &'static Content {
    match id {
        "stock" => &STOCK,
        _ => &NONE,
    }
}
