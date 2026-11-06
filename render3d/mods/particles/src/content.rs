//! The particles module's built-in tables (README.md, "Content sets"): names of game
//! data, never addresses, parsed from the set's `particles.ini`
//! (`render3d/content/sets/<set>/`) at first use. A build with no set, or a set with no
//! `particles.ini`, gets `NONE`: no names, so no vent, pulsar or damage-node emitter is
//! told.
use a2ini::table::Table;

#[derive(Debug, PartialEq)]
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

/// A set's `particles.ini`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let s = "Particles";
    let vents = t.list(s, "VentSprites");
    Content {
        systems: t.list(s, "Systems").leak(),
        vent_sprites: [vents.first().copied().unwrap_or(""), vents.get(1).copied().unwrap_or("")],
        chain_emitter: t.str(s, "ChainEmitter"),
        manheim_emitter: t.str(s, "ManheimEmitter"),
        sod_ext: t.str(s, "SodExt"),
    }
}

/// A build with no content set for this module.
pub static NONE: Content =
    Content { systems: &[], vent_sprites: ["", ""], chain_emitter: "", manheim_emitter: "", sod_ext: "" };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("particles", &crate::syms::TABLE, parse, &NONE)
}
