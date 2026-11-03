//! The effects module's built-in tables (README.md, "Content"): data only, no logic,
//! one set per content profile (`profiles/*/content.toml`). `get()` is the set this
//! build reads; a build with no set gets `NONE`: no names, and the module's code falls
//! back to the engine's own look.

/// A faction a shield's colour is keyed by.
pub struct Faction {
    /// `[Shields]` has one colour key of this name.
    pub key: &'static str,
    /// The colour in 0..255 it defaults to.
    pub colour: [f32; 3],
    /// The first letter of its ships' ODF names (lower case); 0 is every ship no letter
    /// names (the last entry).
    pub initial: u8,
}

pub struct Content {
    /// The weapons whose `CreateShieldHit` calls are tapped, in the order of the sites
    /// in `hooks.rs` (their log lines).
    pub site_names: &'static [&'static str],
    pub factions: &'static [Faction],
}

pub static STOCK: Content = Content {
    site_names: &["beam", "shot", "mine", "missile"],
    factions: &[
        Faction { key: "Federation", colour: [90.0, 170.0, 255.0], initial: b'f' },
        Faction { key: "Klingon", colour: [12.0, 255.0, 6.0], initial: b'k' },
        Faction { key: "Romulan", colour: [90.0, 255.0, 130.0], initial: b'r' },
        Faction { key: "Borg", colour: [110.0, 255.0, 120.0], initial: b'b' },
        Faction { key: "Cardassian", colour: [255.0, 200.0, 80.0], initial: b'c' },
        Faction { key: "Species8472", colour: [255.0, 140.0, 40.0], initial: b'8' },
        Faction { key: "Other", colour: [140.0, 200.0, 255.0], initial: 0 },
    ],
};

/// A build with no content set for this module.
pub static NONE: Content = Content { site_names: &[], factions: &[] };

/// The set this build reads (starting the module's table if nothing has yet).
pub fn get() -> &'static Content {
    // The table starts at its first use: a recipe is read before any hook of the module.
    r3d_core::engine::start_table("effects", &crate::syms::TABLE);
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
    pub fn site_name(&self, i: usize) -> &'static str {
        self.site_names.get(i).copied().unwrap_or("?")
    }

    /// The faction of an ODF name: the entry whose initial it starts with, else the last
    /// (the "other" entry); None when the set has no factions.
    pub fn faction_of(&self, odf: &str) -> Option<usize> {
        let c = odf.bytes().next().map(|c| c.to_ascii_lowercase());
        let last = self.factions.len().checked_sub(1)?;
        Some(c.and_then(|c| self.factions.iter().position(|f| f.initial != 0 && f.initial == c)).unwrap_or(last))
    }
}
