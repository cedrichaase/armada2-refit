//! The UI's built-in tables of game content (README.md, "Content"): the names of the
//! game's races, resources and campaigns the screens and panels show or match, one set
//! per content profile (`profiles/*/content.toml`). Data only. `get()` is the set this
//! build reads; a build with no set gets `NONE`: no races, no resources and no
//! campaigns, so the screens that need them stand down to the engine's own.
use core::ffi::CStr;

/// A race's names, in `widgets::theme::Race::ALL`'s order.
pub struct RaceNames {
    /// As shown (`Species 8472`).
    pub name: &'static str,
    /// As `[UI] Theme=` or the game spells it (lower case): the first is the full name.
    pub aliases: &'static [&'static str],
}

/// A campaign card of the single player screen, in the card order.
pub struct Campaign {
    /// The card's name, when the stock labels do not give one.
    pub name: &'static str,
    /// A word the stock label of this campaign contains (lower case), which matches the
    /// label to the card.
    pub key: &'static str,
    /// The view of render3d's map behind the card.
    pub scene: &'static CStr,
    /// The emblem: a race icon's cell in `allrace` (84 px cells of 256), or None.
    pub emblem: Option<(u32, u32)>,
}

pub struct Content {
    pub races: &'static [RaceNames],
    /// `eResource`, as `g_eResourceToString` names them.
    pub resources: &'static [&'static str],
    pub campaigns: &'static [Campaign],
    /// The views of render3d's map the credits tour.
    pub tour: &'static [&'static CStr],
    /// The view behind the main menu, and the single player screen's first.
    pub main_scene: Option<&'static CStr>,
    pub first_campaign_scene: Option<&'static CStr>,
}

pub static STOCK: Content = Content {
    races: &[
        RaceNames { name: "Federation", aliases: &["federation", "fed"] },
        RaceNames { name: "Klingon", aliases: &["klingon", "kli", "kling"] },
        RaceNames { name: "Romulan", aliases: &["romulan", "rom"] },
        RaceNames { name: "Cardassian", aliases: &["cardassian", "card", "car"] },
        RaceNames { name: "Borg", aliases: &["borg", "bor"] },
        RaceNames { name: "Species 8472", aliases: &["species8472", "species 8472", "8472", "species"] },
    ],
    resources: &["Crew", "Officers", "Dilithium", "Latinum", "Metal", "Biomatter"],
    campaigns: &[
        Campaign { name: "Tutorials", key: "tutorial", scene: c"tutorials", emblem: None },
        Campaign { name: "Federation Campaign", key: "federation", scene: c"federation", emblem: Some((0, 0)) },
        Campaign { name: "Klingon Campaign", key: "klingon", scene: c"klingon", emblem: Some((84, 0)) },
        Campaign { name: "Borg Campaign", key: "borg", scene: c"borg", emblem: Some((84, 84)) },
    ],
    tour: &[c"main", c"federation", c"klingon", c"tutorials", c"borg"],
    main_scene: Some(c"main"),
    first_campaign_scene: Some(c"federation"),
};

/// A build with no content set for the UI.
pub static NONE: Content =
    Content { races: &[], resources: &[], campaigns: &[], tour: &[], main_scene: None, first_campaign_scene: None };

/// The set this build reads.
pub fn get() -> &'static Content {
    match crate::engine::api::content_set() {
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
    /// A resource's name, "" past the list.
    pub fn resource(&self, kind: usize) -> &'static str {
        self.resources.get(kind).copied().unwrap_or("")
    }
}
