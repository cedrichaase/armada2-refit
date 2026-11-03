//! The ordnance module's built-in tables (README.md, "Content"): its recipes, as data,
//! one set per content profile (`profiles/*/content.toml`). A build with no set gets
//! `NONE`: no recipes, so no special weapon is told and the engine draws its own.

pub struct Content {
    /// The special weapons' and waves' recipes and the warp flash
    /// (`render3d/content/ordnance/`), as `recipes` takes them: `Render3D.ini` overrides
    /// them section by section, key by key (`ini.rs`), so there is no copy to install and merge.
    pub recipes: &'static [(&'static str, &'static str)],
    /// A SOD name as an ODF's `shotGeometry` writes it, in either case.
    pub sod_exts: [&'static str; 2],
}

pub static STOCK: Content = Content {
    recipes: &[("ordnance", include_str!("../../../content/ordnance/ordnance.ini"))],
    sod_exts: [".sod", ".SOD"],
};

/// A build with no content set for this module.
pub static NONE: Content = Content { recipes: &[], sod_exts: ["", ""] };

/// The set this build reads (starting the module's table if nothing has yet).
pub fn get() -> &'static Content {
    // The table starts at its first use: a recipe is read before any hook of the module.
    r3d_core::engine::start_table("ordnance", &crate::syms::TABLE);
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

impl Content {
    /// `g` without a trailing SOD extension of the set's.
    pub fn strip_sod_ext<'a>(&self, g: &'a str) -> &'a str {
        let mut g = g;
        for e in self.sod_exts.iter().filter(|e| !e.is_empty()) {
            g = g.trim_end_matches(*e);
        }
        g
    }
}
