//! The hulls module's built-in tables (README.md, "Content sets"): names of the game's
//! data (SOD nodes, sprites, the SOD of the Evolver's cocoon) and stock's own numbers it
//! reproduces, parsed from the set's `hulls.ini` (`render3d/content/sets/<set>/`) at
//! first use. A build with no set, or a set with no `hulls.ini`, gets `NONE`: empty
//! names, which match nothing. Nothing here is an address (those are `hooks.rs`'s) and
//! nothing is drawing.
use a2ini::table::{floats, Table};
use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
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
    /// The `[Hulls]` key prefix of the bump-mapped hulls' own profile (`Borg`: in stock
    /// the Borg alone are bump-mapped, `BorgAmbient`, `BorgSun`, ...); "" gives them the
    /// hull profile.
    pub bump_profile: &'static str,
}

/// A set's `hulls.ini`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let s = "Hulls";
    let colours = t
        .entries("ResourceColours")
        .iter()
        .map(|&(_, v)| {
            let c = floats(v);
            [c.first().copied().unwrap_or(0.0), c.get(1).copied().unwrap_or(0.0), c.get(2).copied().unwrap_or(0.0)]
        })
        .collect::<Vec<_>>();
    Content {
        cocoon_sod: t.str(s, "CocoonSod"),
        odf_ext: t.str(s, "OdfExt"),
        hardpoints_node: t.str(s, "HardpointsNode"),
        lights_nodes: t.list(s, "LightsNodes").leak(),
        damage_parent_nodes: t.list(s, "DamageParentNodes").leak(),
        borg_node: t.str(s, "BorgNode"),
        ore_node: t.str(s, "OreNode"),
        scrap_glow_sprite: t.str(s, "ScrapGlowSprite"),
        resource_colours: colours.leak(),
        bump_profile: t.str(s, "BumpProfile"),
    }
}

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
    bump_profile: "",
};

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("hulls", &crate::syms::TABLE, parse, &NONE)
}

impl Content {
    /// The colour of a resource kind when the engine has none: white.
    pub fn resource_colour(&self, kind: usize) -> [f32; 3] {
        self.resource_colours.get(kind).copied().unwrap_or([1.0, 1.0, 1.0])
    }
}
