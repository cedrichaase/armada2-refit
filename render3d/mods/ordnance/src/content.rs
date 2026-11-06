//! The ordnance module's built-in tables (README.md, "Content sets"): its recipes, as
//! data, parsed from the set's `ordnance.ini` (`render3d/content/sets/<set>/`) at first
//! use. A build with no set, or a set with no `ordnance.ini`, gets `NONE`: no recipes, so
//! no special weapon is told and the engine draws its own.
use a2ini::table::Table;
use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
pub struct Content {
    /// The special weapons' and waves' recipes and the warp flash
    /// (`render3d/content/ordnance/`), as `recipes` takes them: `Render3D.ini` overrides
    /// them section by section, key by key (`ini.rs`), so there is no copy to install and merge.
    pub recipes: &'static [(&'static str, &'static str)],
    /// A SOD name as an ODF's `shotGeometry` writes it, in either case.
    pub sod_exts: [&'static str; 2],
}

/// A set's `ordnance.ini`: its `[Recipes]` name the files under `render3d/content/`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let exts = t.list("Ordnance", "SodExts");
    Content {
        recipes: recipes(&t, "Recipes").leak(),
        sod_exts: [exts.first().copied().unwrap_or(""), exts.get(1).copied().unwrap_or("")],
    }
}

/// A table's recipes: (name, the built-in text of the file it names); a file not built
/// in is left out.
fn recipes(t: &Table<'static>, sec: &str) -> Vec<(&'static str, &'static str)> {
    t.entries(sec).iter().filter_map(|&(n, f)| Some((n, r3d_core::content::file(f)?))).collect()
}

/// A build with no content set for this module.
pub static NONE: Content = Content { recipes: &[], sod_exts: ["", ""] };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("ordnance", &crate::syms::TABLE, parse, &NONE)
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
