//! The overlays module's built-in tables (README.md, "Content sets"): data only, parsed
//! from the set's `overlays.ini` (`render3d/content/sets/<set>/`) at first use. A build
//! with no set, or a set with no `overlays.ini`, gets `NONE`: no selection model, so
//! nothing is drawn at a selected object.
use a2ini::table::Table;

#[derive(Debug, PartialEq)]
pub struct Content {
    /// The selection bubble's model (`select.sod`), drawn at each selected object.
    pub selection_model: &'static str,
}

/// A set's `overlays.ini`.
pub fn parse(text: &'static str) -> Content {
    Content { selection_model: Table::parse(text).str("Overlays", "SelectionModel") }
}

/// A build with no content set for this module.
pub static NONE: Content = Content { selection_model: "" };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("overlays", &crate::syms::TABLE, parse, &NONE)
}
