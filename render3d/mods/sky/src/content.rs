//! The sky module's built-in tables (README.md, "Content sets"): data only, parsed from
//! the set's `sky.ini` (`render3d/content/sets/<set>/`) at first use. A build with no
//! set, or a set with no `sky.ini`, gets `NONE`: no recipes (the engine's own sky) and no
//! point-star texture.
use a2ini::table::Table;
use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
pub struct Content {
    /// The recipes of every stock sky with one, by background name (no extension).
    pub recipes: &'static [(&'static str, &'static str)],
    /// The sprite texture whose stars are the point stars: only they are lifted past
    /// white (the big blobs and clusters are gas, and lifted they glowed over the sky).
    pub point_stars: &'static str,
}

/// A set's `sky.ini`: its `[Recipes]` name the files under `render3d/content/`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let recipes: Vec<(&'static str, &'static str)> =
        t.entries("Recipes").iter().filter_map(|&(n, f)| Some((n, r3d_core::content::file(f)?))).collect();
    Content { recipes: recipes.leak(), point_stars: t.str("Sky", "PointStars") }
}

/// A build with no content set for this module.
pub static NONE: Content = Content { recipes: &[], point_stars: "" };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("sky", &crate::syms::TABLE, parse, &NONE)
}
