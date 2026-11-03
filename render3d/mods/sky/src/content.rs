//! The sky module's built-in tables (README.md, "Content"): data only, no logic, one
//! set per content profile (`profiles/*/content.toml`). A build with no set gets
//! `NONE`: no recipes (the engine's own sky) and no point-star texture.

pub struct Content {
    /// The recipes of every stock sky with one, by background name (no extension).
    pub recipes: &'static [(&'static str, &'static str)],
    /// The sprite texture whose stars are the point stars: only they are lifted past
    /// white (the big blobs and clusters are gas, and lifted they glowed over the sky).
    pub point_stars: &'static str,
}

pub static STOCK: Content = Content {
    recipes: &[
        ("mbg02", include_str!("../../../content/sky/mbg02.ini")),
        ("mbgaqu", include_str!("../../../content/sky/mbgaqu.ini")),
        ("mbgbaku", include_str!("../../../content/sky/mbgbaku.ini")),
        ("mbgblue", include_str!("../../../content/sky/mbgblue.ini")),
        ("mbgborg", include_str!("../../../content/sky/mbgborg.ini")),
        ("mbgdk", include_str!("../../../content/sky/mbgdk.ini")),
        ("mbgdom1", include_str!("../../../content/sky/mbgdom1.ini")),
        ("mbgflu", include_str!("../../../content/sky/mbgflu.ini")),
        ("mbggb", include_str!("../../../content/sky/mbggb.ini")),
        ("mbgkl", include_str!("../../../content/sky/mbgkl.ini")),
        ("mbgkling", include_str!("../../../content/sky/mbgkling.ini")),
        ("mbgpur", include_str!("../../../content/sky/mbgpur.ini")),
        ("mbgred", include_str!("../../../content/sky/mbgred.ini")),
        ("mbgrg", include_str!("../../../content/sky/mbgrg.ini")),
    ],
    point_stars: "mstars",
};

/// A build with no content set for this module.
pub static NONE: Content = Content { recipes: &[], point_stars: "" };

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
