//! The world module's built-in tables (README.md, "Content sets"): data only, no logic,
//! parsed from the set's `world.ini` (`render3d/content/sets/<set>/`) at first use. A
//! build with no set, or a set with no `world.ini`, gets `NONE`: no recipes, so the
//! planets, moons, asteroids and nebulae keep the engine's own look. The recipes are the
//! ones `render3d/install.sh` copies into the game directory's `Surfaces\` and `Nebulae\`
//! (a copy there wins: `recipes::text`), built in so the look needs nothing installed.
//! Names are lower case: a lookup that misses tries the lower-cased name.
use a2ini::table::Table;
use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
pub struct Content {
    /// `render3d/content/surfaces/`: a texture's surface recipe, by lower-cased texture name.
    pub surfaces: &'static [(&'static str, &'static str)],
    /// `render3d/content/nebulae/`: a nebula class's gas recipe, by lower-cased ODF name.
    pub nebulae: &'static [(&'static str, &'static str)],
    /// The engine's material for the dilithium moons' glow shell (`Scrap::Render` sets its
    /// colour; the module draws the shell added).
    pub moon_glow_material: &'static str,
}

/// A set's `world.ini`: `[Surfaces]` and `[Nebulae]` name the files under
/// `render3d/content/`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let recipes = |sec: &str| -> &'static [(&'static str, &'static str)] {
        t.entries(sec).iter().filter_map(|&(n, f)| Some((n, r3d_core::content::file(f)?))).collect::<Vec<_>>().leak()
    };
    Content { surfaces: recipes("Surfaces"), nebulae: recipes("Nebulae"), moon_glow_material: t.str("World", "MoonGlowMaterial") }
}

/// A build with no content set for this module.
pub static NONE: Content = Content { surfaces: &[], nebulae: &[], moon_glow_material: "" };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("world", &crate::syms::TABLE, parse, &NONE)
}
