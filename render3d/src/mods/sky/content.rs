//! The sky module's built-in tables (README.md, "Content"): data only, no logic.
//! Phase 8f turns these into the per-(game, build) content profile.

/// The recipes of every stock sky with one, by background name (no extension).
pub static RECIPES: &[(&str, &str)] = &[
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
];

/// The sprite texture whose stars are the point stars: only they are lifted past white
/// (the big blobs and clusters are gas, and lifted they glowed over the sky).
pub const POINT_STARS: &str = "mstars";
