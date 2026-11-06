//! The specials module's built-in tables (README.md, "Content sets"): the effect kinds it
//! gives a look to, and the look's shipped defaults (`[Specials]` overrides each key),
//! parsed from the set's `specials.ini` (`render3d/content/sets/<set>/`) at first use. A
//! build with no set, or a set with no `specials.ini`, gets `NONE`: no kind has a look,
//! and the engine draws its own effects.
use a2ini::table::{floats, Table};
use alloc::vec::Vec;

/// One kind's defaults: (colour 0..255, gain, strength, creep s, scale, rim, light
/// colour 0..255, light brightness, light range in the victim's radii).
pub type Defaults = ([f32; 3], f32, f32, f32, f32, f32, [f32; 3], f32, f32);

#[derive(Debug, PartialEq)]
pub struct Content {
    /// The kinds, by `look::kind`'s index, as `[Specials]` spells their keys: Computer
    /// Override, the Energy Dissipator's drain, Psionic Disruption.
    pub names: &'static [&'static str],
    pub defaults: &'static [Defaults],
}

/// A set's `specials.ini`: `[Kinds]` in `look::kind`'s order, each `name = ` the 13
/// numbers of its `Defaults`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let kinds = t.entries("Kinds");
    let defaults = kinds
        .iter()
        .map(|&(_, v)| {
            let mut n = floats(v);
            n.resize(13, 0.0);
            ([n[0], n[1], n[2]], n[3], n[4], n[5], n[6], n[7], [n[8], n[9], n[10]], n[11], n[12])
        })
        .collect::<Vec<Defaults>>();
    Content { names: kinds.iter().map(|&(k, _)| k).collect::<Vec<_>>().leak(), defaults: defaults.leak() }
}

/// A build with no content set for this module.
pub static NONE: Content = Content { names: &[], defaults: &[] };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("specials", &crate::syms::TABLE, parse, &NONE)
}

impl Content {
    pub fn name(&self, k: usize) -> &'static str {
        self.names.get(k).copied().unwrap_or("?")
    }
}
