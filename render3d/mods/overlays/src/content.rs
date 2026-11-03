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
