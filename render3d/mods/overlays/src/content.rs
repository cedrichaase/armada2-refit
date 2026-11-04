//! The overlays module's built-in tables (README.md, "overlays", "Content"): data only,
//! one set per content profile (`profiles/*/content.toml`). A build with no set gets
//! `NONE`: no selection model, so nothing is drawn at a selected object.

pub struct Content {
    /// The selection bubble's model (`select.sod`), drawn at each selected object.
    pub selection_model: &'static str,
}

pub static STOCK: Content = Content { selection_model: "select" };

/// A build with no content set for this module.
pub static NONE: Content = Content { selection_model: "" };

/// The set this build reads (starting the module's table if nothing has yet).
pub fn get() -> &'static Content {
    // The table starts at its first use: a recipe is read before any hook of the module.
    r3d_core::engine::start_table("overlays", &crate::syms::TABLE);
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
