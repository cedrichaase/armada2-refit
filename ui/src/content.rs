//! The UI's built-in tables of game content (README.md, "Content sets"): the names of
//! the game's races, resources, campaigns and files the screens and panels show or
//! match, parsed at first use from the set's `ui.ini` (`ui/content/sets/<set>/`, built
//! in by `build.rs`; the set is the one `profiles/<build>/content.toml` gives the UI).
//! Data only. `get()` is the set this build reads; a build with no set, or a set with
//! no `ui.ini`, gets `NONE`: no races, no resources, no campaigns and no files, so the
//! screens that need them stand down to the engine's own.
use a2ini::table::{find, floats, split_list, Table};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::ffi::CStr;

/// A race's names (`[Race.*]`, in the set's order: `widgets::theme::Race`).
#[derive(Debug, PartialEq)]
pub struct RaceNames {
    /// As shown (`Species 8472`).
    pub name: &'static str,
    /// The theme's look for it (`widgets::theme::Form::from_id`: `lcars`, `cube`, ...).
    pub theme: &'static str,
    /// As `[UI] Theme=` or the game spells it (lower case): the first is the full name.
    pub aliases: &'static [&'static str],
}

/// A campaign card of the single player screen, in the card order.
#[derive(Debug, PartialEq)]
pub struct Campaign {
    /// The card's name, when the stock labels do not give one.
    pub name: &'static str,
    /// A word the stock label of this campaign contains (lower case), which matches the
    /// label to the card.
    pub key: &'static str,
    /// The view of render3d's map behind the card.
    pub scene: &'static CStr,
    /// The emblem: a race icon's cell in `Files::race_icons` (84 px cells of 256), or None.
    pub emblem: Option<(u32, u32)>,
    /// The prefix of the campaign's map names (lower case), for the loading screen.
    pub map: &'static str,
    /// The loading screen's line for its maps, and whether the mission's number follows.
    pub kicker: &'static str,
    pub numbered: bool,
    /// The card (this list's index) whose campaign must be complete before this one
    /// opens (`menus::locks`), or None: always open. The list's order is the game's own
    /// order of the campaigns' progress (`engine::progress`).
    pub requires: Option<usize>,
}

/// The game's files the UI reads, by their path in `files` (`textures\<file>` for a
/// texture, the rest under the game directory; "" for none).
#[derive(Debug, PartialEq)]
pub struct Files {
    /// The race icons of the campaign cards.
    pub race_icons: &'static str,
    /// The credits' text and logos.
    pub credits: &'static str,
    pub credits_logos: &'static str,
    /// The cursor sprites.
    pub cursor_sprites: &'static str,
}

#[derive(Debug, PartialEq)]
pub struct Content {
    pub races: &'static [RaceNames],
    /// `eResource`, as `g_eResourceToString` names them.
    pub resources: &'static [&'static str],
    /// Where the resources panel sits: false at the top centre of the screen (stock), true
    /// over the game's own resource panel's rectangle (`[Resources] Placement = component`,
    /// Fleet Operations: Roots, whose HUD fills the top centre).
    pub resources_at_component: bool,
    pub campaigns: &'static [Campaign],
    /// The views of render3d's map the credits tour.
    pub tour: &'static [&'static CStr],
    /// The view behind the main menu, and the single player screen's first.
    pub main_scene: Option<&'static CStr>,
    pub first_campaign_scene: Option<&'static CStr>,
    pub files: Files,
    /// The widget gallery's sample lists (`gallery.rs`).
    pub gallery_ships: &'static [&'static str],
    pub gallery_factions: &'static [&'static str],
}

/// A build with no content set for the UI.
pub static NONE: Content = Content {
    races: &[],
    resources: &[],
    resources_at_component: false,
    campaigns: &[],
    tour: &[],
    main_scene: None,
    first_campaign_scene: None,
    files: Files { race_icons: "", credits: "", credits_logos: "", cursor_sprites: "" },
    gallery_ships: &[],
    gallery_factions: &[],
};

/// A name as the engine's C string, made once and kept for the plugin's life.
fn cstr(s: &str) -> &'static CStr {
    let c = CString::new(s).unwrap_or_default();
    Box::leak(c.into_boxed_c_str())
}

fn scene(s: &str) -> Option<&'static CStr> {
    (!s.is_empty()).then(|| cstr(s))
}

/// A set's `ui.ini`.
pub fn parse(text: &'static str) -> Content {
    let t = Table::parse(text);
    let races = t
        .sections_under("Race.")
        .map(|(_, es)| RaceNames {
            name: find(es, "Name").unwrap_or(""),
            theme: find(es, "Theme").unwrap_or(""),
            aliases: split_list(find(es, "Aliases").unwrap_or("")).leak(),
        })
        .collect::<Vec<_>>();
    let campaigns = t
        .sections_under("Campaign.")
        .map(|(_, es)| {
            let e = floats(find(es, "Emblem").unwrap_or(""));
            Campaign {
                name: find(es, "Name").unwrap_or(""),
                key: find(es, "Key").unwrap_or(""),
                scene: cstr(find(es, "Scene").unwrap_or("")),
                emblem: (e.len() >= 2).then(|| (e[0] as u32, e[1] as u32)),
                map: find(es, "Map").unwrap_or(""),
                kicker: find(es, "Kicker").unwrap_or(""),
                numbered: find(es, "Numbered").is_some_and(|v| v.trim() == "1"),
                requires: None,
            }
        })
        .collect::<Vec<_>>();
    // `Requires` names another card by its key; it becomes that card's index.
    let mut campaigns = campaigns;
    let reqs: Vec<Option<usize>> = t
        .sections_under("Campaign.")
        .map(|(_, es)| {
            let k = find(es, "Requires").unwrap_or("").trim();
            if k.is_empty() { None } else { campaigns.iter().position(|c| c.key == k) }
        })
        .collect();
    for (c, r) in campaigns.iter_mut().zip(reqs) {
        c.requires = r;
    }
    Content {
        races: races.leak(),
        resources: t.list("Resources", "Names").leak(),
        resources_at_component: t.str("Resources", "Placement").trim() == "component",
        campaigns: campaigns.leak(),
        tour: t.list("Scenes", "Tour").iter().map(|s| cstr(s)).collect::<Vec<_>>().leak(),
        main_scene: scene(t.str("Scenes", "Main")),
        first_campaign_scene: scene(t.str("Scenes", "FirstCampaign")),
        files: Files {
            race_icons: t.str("Files", "RaceIcons"),
            credits: t.str("Files", "Credits"),
            credits_logos: t.str("Files", "CreditsLogos"),
            cursor_sprites: t.str("Files", "CursorSprites"),
        },
        gallery_ships: t.list("Gallery", "Ships").leak(),
        gallery_factions: t.list("Gallery", "Factions").leak(),
    }
}

static CACHE: crate::content_sets::Cache<Content> = crate::content_sets::Cache::new();

/// The set this build reads: its table parsed at first use; `NONE` while the build has
/// no set for the UI.
pub fn get() -> &'static Content {
    CACHE.get(crate::engine::api::content_set, parse, &NONE)
}

impl Content {
    /// A resource's name, "" past the list.
    pub fn resource(&self, kind: usize) -> &'static str {
        self.resources.get(kind).copied().unwrap_or("")
    }
}
