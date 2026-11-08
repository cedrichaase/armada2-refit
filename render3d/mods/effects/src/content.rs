//! The effects module's built-in tables (README.md, "Content sets"): data only, no
//! logic, parsed from the set's `effects.ini` (`render3d/content/sets/<set>/`). `get()`
//! is the set this build reads; a build with no set, or a set with no table, gets
//! `NONE`: no names, and the module's code falls back to the engine's own look.
use a2ini::table::{floats, Table};
use alloc::vec::Vec;

/// A faction a shield's colour is keyed by.
#[derive(Debug, PartialEq)]
pub struct Faction {
    /// `[Shields]` has one colour key of this name.
    pub key: &'static str,
    /// The colour in 0..255 it defaults to.
    pub colour: [f32; 3],
    /// The first letter of its ships' ODF names (lower case); 0 is every ship no letter
    /// names (the last entry).
    pub initial: u8,
}

#[derive(Debug, PartialEq)]
pub struct Content {
    /// The weapons whose `CreateShieldHit` calls are tapped, in the order of the sites
    /// in `hooks.rs` (their log lines).
    pub site_names: &'static [&'static str],
    pub factions: &'static [Faction],
    /// The Evolver's cocoon's SOD (`8472_cocoon` in stock), broken up when it hatches.
    pub cocoon_sod: &'static str,
    /// `[Shields] RayFactions=` when the ini names none.
    pub ray_factions: &'static str,
    /// `[Races]`: a race (an ODF's `race`, lower case) to the key of its faction. A set
    /// with races keys a ship's faction by its race (Fleet Operations), one without by
    /// its ODF name's initial (stock); a race it does not name is the last faction.
    pub races: &'static [(&'static str, &'static str)],
}

/// A set's `effects.ini`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let factions = t
        .entries("Factions")
        .iter()
        .map(|&(key, v)| {
            let (initial, rgb) = v.split_once(',').unwrap_or((v, ""));
            let c = floats(rgb);
            let initial = initial.trim();
            Faction {
                key,
                colour: [c.first().copied().unwrap_or(0.0), c.get(1).copied().unwrap_or(0.0), c.get(2).copied().unwrap_or(0.0)],
                initial: if initial == "-" { 0 } else { initial.bytes().next().unwrap_or(0).to_ascii_lowercase() },
            }
        })
        .collect::<Vec<_>>();
    Content {
        site_names: t.list("Effects", "Sites").leak(),
        factions: factions.leak(),
        cocoon_sod: t.str("Effects", "CocoonSod"),
        ray_factions: t.str("Effects", "RayFactions"),
        races: t.entries("Races").to_vec().leak(),
    }
}

/// A build with no content set for this module.
pub static NONE: Content = Content { site_names: &[], factions: &[], cocoon_sod: "", ray_factions: "", races: &[] };

static CACHE: r3d_core::content::Cache<Content> = r3d_core::content::Cache::new();

/// The set this build reads (starting the module's table if nothing has yet): its
/// table parsed at first use; `NONE` while the build has no set for the module.
pub fn get() -> &'static Content {
    CACHE.get("effects", &crate::syms::TABLE, parse, &NONE)
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

    /// Whether factions go by race (the set has `[Races]`).
    pub fn by_race(&self) -> bool {
        !self.races.is_empty()
    }

    /// The faction of a race (lower case): the one `[Races]` names for it, else the last
    /// (the "other" entry); None when the set has no factions.
    pub fn faction_of_race(&self, race: &str) -> Option<usize> {
        let last = self.factions.len().checked_sub(1)?;
        let key = self.races.iter().find(|(r, _)| r.eq_ignore_ascii_case(race)).map(|(_, k)| *k);
        Some(key.and_then(|k| self.factions.iter().position(|f| f.key.eq_ignore_ascii_case(k))).unwrap_or(last))
    }
}
