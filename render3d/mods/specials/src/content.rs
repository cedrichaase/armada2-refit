//! The specials module's built-in tables: the three effect kinds it gives a look to,
//! and the look's shipped defaults (`[Specials]` overrides each key), one set per
//! content profile (`profiles/*/content.toml`). A build with no set gets `NONE`: no
//! kind has a look, and the engine draws its own effects.

/// One kind's defaults: (colour 0..255, gain, strength, creep s, scale, rim, light
/// colour 0..255, light brightness, light range in the victim's radii).
pub type Defaults = ([f32; 3], f32, f32, f32, f32, f32, [f32; 3], f32, f32);

pub struct Content {
    /// The kinds, by `look::kind`'s index, as `[Specials]` spells their keys: Computer
    /// Override, the Energy Dissipator's drain, Psionic Disruption.
    pub names: &'static [&'static str],
    pub defaults: &'static [Defaults],
}

pub static STOCK: Content = Content {
    names: &["Override", "Drain", "Psionic"],
    defaults: &[
        ([60.0, 255.0, 110.0], 1.6, 1.0, 2.5, 14.0, 0.15, [80.0, 255.0, 120.0], 1.6, 3.0),
        ([235.0, 80.0, 190.0], 1.4, 0.9, 1.5, 10.0, 0.3, [235.0, 90.0, 200.0], 1.0, 2.5),
        ([150.0, 110.0, 255.0], 1.6, 1.0, 0.8, 3.0, 0.6, [160.0, 110.0, 255.0], 1.4, 3.0),
    ],
};

/// A build with no content set for this module.
pub static NONE: Content = Content { names: &[], defaults: &[] };

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

impl Content {
    pub fn name(&self, k: usize) -> &'static str {
        self.names.get(k).copied().unwrap_or("?")
    }
}
