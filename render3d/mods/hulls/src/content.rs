//! The hulls module's built-in tables: names of the game's data (SOD nodes, sprites,
//! the SOD of the Evolver's cocoon) and stock's own numbers it reproduces, one set per
//! content profile (`profiles/*/content.toml`). Nothing here is an address (those are
//! `hooks.rs`'s) and nothing is drawing (that is the module's). A build with no set
//! gets `NONE`: empty names, which match nothing.

pub struct Content {
    /// The Evolver's cocoon: the SOD of the class the engine keeps (`8472_cocoon.sod`).
    pub cocoon_sod: &'static str,
    /// A class's ODF name ends in this; the SOD is named after the stem.
    pub odf_ext: &'static str,
    /// The node a SOD's hardpoints hang from: hidden when a model is shown clean.
    pub hardpoints_node: &'static str,
    /// The node of a SOD's running lights (either spelling): hidden in a clean model of
    /// a low level of detail, and in a build ghost.
    pub lights_nodes: &'static [&'static str],
    /// The nodes of a model with no class behind it (the menu scene) whose children are
    /// the systems' damage effects and the crew fires.
    pub damage_parent_nodes: &'static [&'static str],
    /// The Borg node of such a model, hidden too.
    pub borg_node: &'static str,
    /// A freighter's cargo markings hang from this node (`Freighter::InitializeGeometry`).
    pub ore_node: &'static str,
    /// The sprite node of a scrap object's glow (the dilithium moons).
    pub scrap_glow_sprite: &'static str,
    /// `ResourceComponent::s_ResourceColor` as stock initialises it (crew, officers,
    /// dilithium, latinum, metal, biomatter), for when `InitResourceComponents` has not run.
    pub resource_colours: &'static [[f32; 3]],
}

pub static STOCK: Content = Content {
    cocoon_sod: "8472_cocoon",
    odf_ext: ".odf",
    hardpoints_node: "hardpoints",
    lights_nodes: &["lights", "light"],
    damage_parent_nodes: &["life", "engines", "weapons", "shield", "shields", "sensors", "crew", "damage"],
    borg_node: "borg",
    ore_node: "ore",
    scrap_glow_sprite: "glowdmoon",
    resource_colours: &[[0.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.6, 0.9, 1.0], [1.0, 1.0, 0.0], [0.8, 0.8, 0.8], [1.0, 0.0, 0.0]],
};

/// A build with no content set for this module.
pub static NONE: Content = Content {
    cocoon_sod: "",
    odf_ext: "",
    hardpoints_node: "",
    lights_nodes: &[],
    damage_parent_nodes: &[],
    borg_node: "",
    ore_node: "",
    scrap_glow_sprite: "",
    resource_colours: &[],
};

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

impl Content {
    /// The colour of a resource kind when the engine has none: white.
    pub fn resource_colour(&self, kind: usize) -> [f32; 3] {
        self.resource_colours.get(kind).copied().unwrap_or([1.0, 1.0, 1.0])
    }
}
