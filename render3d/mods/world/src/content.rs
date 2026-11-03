//! The world module's built-in tables (README.md, "World", "Content"): data only, no
//! logic, one set per content profile (`profiles/*/content.toml`). The recipes below are
//! the ones `render3d/install.sh` copies into the game directory's `Surfaces\` and
//! `Nebulae\` (a copy there wins: `recipes::text`), built in so the look needs nothing
//! installed. Names are lower case: a lookup that misses tries the lower-cased name. A
//! build with no set gets `NONE`: no recipes, so the planets, moons, asteroids and
//! nebulae keep the engine's own look.

pub struct Content {
    /// `render3d/content/surfaces/`: a texture's surface recipe, by lower-cased texture name.
    pub surfaces: &'static [(&'static str, &'static str)],
    /// `render3d/content/nebulae/`: a nebula class's gas recipe, by lower-cased ODF name.
    pub nebulae: &'static [(&'static str, &'static str)],
    /// The engine's material for the dilithium moons' glow shell (`Scrap::Render` sets its
    /// colour; the module draws the shell added).
    pub moon_glow_material: &'static str,
}

pub static STOCK: Content = Content {
    surfaces: &[
        ("master1", include_str!("../../../content/surfaces/Master1.ini")),
        ("master2", include_str!("../../../content/surfaces/Master2.ini")),
        ("mdmoon", include_str!("../../../content/surfaces/Mdmoon.ini")),
        ("pb_clssd", include_str!("../../../content/surfaces/PB_CLSSD.ini")),
        ("pb_clssh", include_str!("../../../content/surfaces/PB_CLSSH.ini")),
        ("pb_clssj", include_str!("../../../content/surfaces/PB_CLSSJ.ini")),
        ("pb_clssk", include_str!("../../../content/surfaces/PB_CLSSK.ini")),
        ("pb_clssl", include_str!("../../../content/surfaces/PB_CLSSL.ini")),
        ("pb_clssm", include_str!("../../../content/surfaces/PB_CLSSM.ini")),
    ],
    nebulae: &[
        ("fluidicnebula", include_str!("../../../content/nebulae/fluidicnebula.ini")),
        ("generatednebula", include_str!("../../../content/nebulae/generatednebula.ini")),
        ("latinum", include_str!("../../../content/nebulae/latinum.ini")),
        ("mnebula10", include_str!("../../../content/nebulae/mnebula10.ini")),
        ("mnebula12", include_str!("../../../content/nebulae/mnebula12.ini")),
        ("mnebula6", include_str!("../../../content/nebulae/mnebula6.ini")),
        ("mnebula7", include_str!("../../../content/nebulae/mnebula7.ini")),
        ("mnebula8", include_str!("../../../content/nebulae/mnebula8.ini")),
        ("mnebula9", include_str!("../../../content/nebulae/mnebula9.ini")),
    ],
    moon_glow_material: "glowdmoon",
};

/// A build with no content set for this module.
pub static NONE: Content = Content { surfaces: &[], nebulae: &[], moon_glow_material: "" };

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
