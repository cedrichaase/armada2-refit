//! The UI's built-in tables of game content (README.md, "Content sets"): the names of
//! the game's races, resources, campaigns and files the screens and panels show or
//! match, parsed at first use from the set's `ui.ini` (`ui/content/sets/<set>/`, built
//! in by `build.rs`; the set is the one `profiles/<build>/content.toml` gives the UI).
//! Data only. `get()` is the set this build reads; a build with no set, or a set with
//! no `ui.ini`, gets `NONE`: no races, no resources, no campaigns and no files, so the
//! screens that need them stand down to the engine's own.
use a2ini::table::{find, floats, split_list, Table};
use alloc::boxed::Box;
use alloc::string::String;
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
    /// The main menu's title lettering on a game that has it as art (a PNG under the game
    /// directory, read at run time; `menus::title_art`), or "" for the text heading.
    pub menu_title: &'static str,
    /// The `.spr` file of the HUD sprites every race shares (Roots: `gui_global.spr`), looked in
    /// after the race's own when a sprite's art is read from its `.spr` line
    /// (`engine::sprite::spr_entry`); "" for none.
    pub gui_sprites: &'static str,
    /// The `.spr` of the bitmap font whose pictures the resource icons are (`Sprites\<file>`,
    /// its page's texture read from `textures\`); "" for none (`engine::fontglyph`).
    pub icon_font: &'static str,
}

/// Where the resources panel sits.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ResourcePlacement {
    Top,
    Component,
    Corner,
}

#[derive(Debug, PartialEq)]
pub struct Content {
    pub races: &'static [RaceNames],
    /// `eResource`, as `g_eResourceToString` names them.
    pub resources: &'static [&'static str],
    /// Where the resources panel sits (`[Resources] Placement`): at the top centre of the
    /// screen (stock), over the game's own resource panel's rectangle (`component`), or
    /// flush in the top left corner (`corner`: Fleet Operations: Roots, whose HUD fills the
    /// top centre).
    pub resources_placement: ResourcePlacement,
    /// `[ResourceIcons]`: a resource's name (as the game's table gives it, lower case) = the
    /// code of the picture its font draws for it (hex), as `[Glyphs]` names it; a resource
    /// not listed keeps the panel's coloured bar.
    pub resource_icons: &'static [(&'static str, u8)],
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
    /// The codes the game's fonts draw as a picture or a figure of their own
    /// (`[Glyphs]`, a code in hex = its word or figure), for `text::game::display`.
    pub glyphs: &'static [(u8, &'static str)],
    /// `[Loading] Bar`: the loading screen's bar when `[Loading] Bar=` in UI.ini is empty
    /// (`panel`: a square bar in a thin box; empty: the classic one).
    pub loading_bar: &'static str,
}

/// A build with no content set for the UI.
pub static NONE: Content = Content {
    races: &[],
    resources: &[],
    resources_placement: ResourcePlacement::Top,
    resource_icons: &[],
    campaigns: &[],
    tour: &[],
    main_scene: None,
    first_campaign_scene: None,
    files: Files { race_icons: "", credits: "", credits_logos: "", cursor_sprites: "", menu_title: "", gui_sprites: "", icon_font: "" },
    gallery_ships: &[],
    gallery_factions: &[],
    glyphs: &[],
    loading_bar: "",
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
        resources_placement: match t.str("Resources", "Placement").trim() {
            "component" => ResourcePlacement::Component,
            "corner" => ResourcePlacement::Corner,
            _ => ResourcePlacement::Top,
        },
        resource_icons: t
            .entries("ResourceIcons")
            .iter()
            .filter_map(|&(k, v)| u8::from_str_radix(v.trim(), 16).ok().map(|b| (k.trim(), b)))
            .collect::<Vec<_>>()
            .leak(),
        campaigns: campaigns.leak(),
        tour: t.list("Scenes", "Tour").iter().map(|s| cstr(s)).collect::<Vec<_>>().leak(),
        main_scene: scene(t.str("Scenes", "Main")),
        first_campaign_scene: scene(t.str("Scenes", "FirstCampaign")),
        files: Files {
            race_icons: t.str("Files", "RaceIcons"),
            credits: t.str("Files", "Credits"),
            credits_logos: t.str("Files", "CreditsLogos"),
            cursor_sprites: t.str("Files", "CursorSprites"),
            menu_title: t.str("Files", "MenuTitle"),
            gui_sprites: t.str("Files", "GuiSprites"),
            icon_font: t.str("Files", "IconFont"),
        },
        gallery_ships: t.list("Gallery", "Ships").leak(),
        gallery_factions: t.list("Gallery", "Factions").leak(),
        glyphs: t
            .entries("Glyphs")
            .iter()
            .filter_map(|&(k, v)| u8::from_str_radix(k.trim(), 16).ok().map(|b| (b, v)))
            .collect::<Vec<_>>()
            .leak(),
        loading_bar: t.str("Loading", "Bar").trim(),
    }
}

static CACHE: crate::content_sets::Cache<Content> = crate::content_sets::Cache::new();

/// The set this build reads: its table parsed at first use; `NONE` while the build has
/// no set for the UI.
pub fn get() -> &'static Content {
    CACHE.get(crate::engine::api::content_set, parse, &NONE)
}

impl Content {
    /// What the game's fonts draw for `c` (a byte), when the set names it.
    fn glyph(&self, c: char) -> Option<&'static str> {
        let b = c as u32;
        if b > 0xff {
            return None;
        }
        self.glyphs.iter().find(|g| g.0 as u32 == b).map(|g| g.1)
    }

    /// Whether `s` (as `display` gave it) is one of the set's glyphs alone: a picture the
    /// game's font draws, written as the set's word, not the game's own text.
    pub fn is_glyph(&self, s: &str) -> bool {
        let s = s.trim();
        !s.is_empty() && self.glyphs.iter().any(|g| g.1 == s)
    }

    /// The game's string `s` (one byte per `char`) as the UI draws it (`text::game`):
    /// the set's glyphs as their words and figures (a word set apart by a space from a
    /// figure or a letter beside it: the icons sit against their figures, "250\u{85}",
    /// "Buy \u{85} for"), the rest as Windows-1252, a tab as a space. Characters past
    /// one byte are kept, and no glyph gives a code a byte of its own spells, so a
    /// string that went through here comes back the same.
    pub fn display(&self, s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        for (i, &c) in chars.iter().enumerate() {
            if let Some(g) = self.glyph(c) {
                if is_word(g) {
                    if out.chars().last().is_some_and(|p| p.is_alphanumeric()) {
                        out.push(' ');
                    }
                    out.push_str(g);
                    let next = chars.get(i + 1).map(|&n| self.glyph(n).and_then(|g| g.chars().next()).unwrap_or(n));
                    if next.is_some_and(|n| n.is_alphanumeric()) {
                        out.push(' ');
                    }
                } else {
                    out.push_str(g);
                }
                continue;
            }
            out.push(if c == '\t' { ' ' } else { cp1252(c) });
        }
        out
    }

    /// A resource's name, "" past the list.
    pub fn resource(&self, kind: usize) -> &'static str {
        self.resources.get(kind).copied().unwrap_or("")
    }
}

// ---- the game's code page (text::game) -------------------------------------------------

/// Windows-1252's 0x80..0x9f; the five codes it leaves unassigned stay what they are.
const CP1252: [char; 32] = [
    '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{178}', '\u{9e}',
];

/// A byte of the game's code page as a character.
pub fn cp1252(c: char) -> char {
    match c as u32 {
        v @ 0x80..=0x9f => CP1252[(v - 0x80) as usize],
        _ => c,
    }
}

/// A glyph that is a word (a picture of a name, an icon written as its name), not a
/// figure or a separator.
fn is_word(s: &str) -> bool {
    s.chars().count() > 1 && s.chars().next().is_some_and(|c| c.is_alphabetic())
}
