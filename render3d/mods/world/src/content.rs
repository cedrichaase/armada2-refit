//! The world module's built-in tables (README.md, "World", "Content"): data only, no
//! logic. The recipes below are the ones `render3d/install.sh` copies into the game
//! directory's `Surfaces\` and `Nebulae\` (a copy there wins: `recipes::text`), built
//! in so the look needs nothing installed. Names are lower case: a lookup that misses
//! tries the lower-cased name.

/// `render3d/content/surfaces/`: a texture's surface recipe, by lower-cased texture name.
pub static SURFACES: &[(&str, &str)] = &[
    ("master1", include_str!("../../../content/surfaces/Master1.ini")),
    ("master2", include_str!("../../../content/surfaces/Master2.ini")),
    ("mdmoon", include_str!("../../../content/surfaces/Mdmoon.ini")),
    ("pb_clssd", include_str!("../../../content/surfaces/PB_CLSSD.ini")),
    ("pb_clssh", include_str!("../../../content/surfaces/PB_CLSSH.ini")),
    ("pb_clssj", include_str!("../../../content/surfaces/PB_CLSSJ.ini")),
    ("pb_clssk", include_str!("../../../content/surfaces/PB_CLSSK.ini")),
    ("pb_clssl", include_str!("../../../content/surfaces/PB_CLSSL.ini")),
    ("pb_clssm", include_str!("../../../content/surfaces/PB_CLSSM.ini")),
];

/// `render3d/content/nebulae/`: a nebula class's gas recipe, by lower-cased ODF name.
pub static NEBULAE: &[(&str, &str)] = &[
    ("fluidicnebula", include_str!("../../../content/nebulae/fluidicnebula.ini")),
    ("generatednebula", include_str!("../../../content/nebulae/generatednebula.ini")),
    ("latinum", include_str!("../../../content/nebulae/latinum.ini")),
    ("mnebula10", include_str!("../../../content/nebulae/mnebula10.ini")),
    ("mnebula12", include_str!("../../../content/nebulae/mnebula12.ini")),
    ("mnebula6", include_str!("../../../content/nebulae/mnebula6.ini")),
    ("mnebula7", include_str!("../../../content/nebulae/mnebula7.ini")),
    ("mnebula8", include_str!("../../../content/nebulae/mnebula8.ini")),
    ("mnebula9", include_str!("../../../content/nebulae/mnebula9.ini")),
];

/// The engine's material for the dilithium moons' glow shell (`Scrap::Render` sets its
/// colour; the module draws the shell added).
pub const MOON_GLOW_MATERIAL: &str = "glowdmoon";
