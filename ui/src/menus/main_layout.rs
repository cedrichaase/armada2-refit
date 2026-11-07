//! The main menu column's layout, title and entries as data (PLAN-FLEETOPS.md, decisions
//! 410 to 419; MENUS.md, "The main menu's layout file"): where the column stands, its
//! sizes and spacing, the title (two lines of text or a picture) and which entries are
//! shown under which words.
//!
//! Three sources, the first that names a key wins: `Menu\mainmenu.ini` in the game's
//! data directory (read again within half a second of each save), the content set's
//! `[MainMenu]` section (`ui/content/sets/<set>/ui.ini`), and the built-in defaults, which
//! are Armada II's look to the pixel. The title picture alone has a fourth, in front:
//! `[Menus] TitleImage=` in UI.ini (a key a player may have set), and behind: the set's
//! `Files.MenuTitle`. Nothing here adds an action: an entry can be relabelled or hidden,
//! never made.
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Left,
    Right,
    Centre,
}

/// One line of the title in text: the words, size in units, colour (None: the theme's).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: Option<String>,
    pub size: f32,
    pub colour: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MainMenu {
    /// The two text lines (`Title1` small, `Title2` big); None keeps the caller's words.
    pub line1: Line,
    pub line2: Line,
    pub rule: bool,
    pub rule_colour: Option<u32>,
    /// The picture's path in the game's data ("" for the text title), its explicit crop
    /// (x, y, w, h in the file's pixels) and its width in units.
    pub image: String,
    pub crop: Option<[u32; 4]>,
    pub image_width: f32,
    pub side: Side,
    /// Units from the side (centre: from the middle); None: Armada II's 8% of the width.
    pub x: Option<f32>,
    /// Units from the top to the title's top; None: the column centred in the height.
    pub y: Option<f32>,
    pub width: f32,
    pub title_gap: f32,
    pub entry_gap: f32,
    pub entry_height: f32,
    pub entry_size: f32,
    /// The shade behind the column, 0 to 1 of Armada II's.
    pub opacity: f32,
    /// (normalised entry name, what it says): None hides the entry.
    pub entries: Vec<(String, Option<String>)>,
}

impl MainMenu {
    /// Armada II's main menu as it was before the file: the geometry of the faded-art layout.
    pub fn builtin() -> MainMenu {
        MainMenu {
            line1: Line { text: None, size: 28.0, colour: None },
            line2: Line { text: None, size: 68.0, colour: None },
            rule: true,
            rule_colour: None,
            image: String::new(),
            crop: None,
            image_width: 460.0,
            side: Side::Left,
            x: None,
            y: None,
            width: 380.0,
            title_gap: 14.0,
            entry_gap: 12.0,
            entry_height: 54.0,
            entry_size: 26.0,
            opacity: 1.0,
            entries: Vec::new(),
        }
    }

    /// What entry `label` is shown as: Some(words) or None for hidden.
    pub fn entry(&self, label: &str) -> Option<String> {
        let k = norm(label);
        match self.entries.iter().find(|e| e.0 == k) {
            Some((_, v)) => v.clone(),
            None => Some(String::from(label)),
        }
    }
}

/// An entry's key: its label without case or anything but letters and figures
/// (`SINGLE PLAYER` is `singleplayer`).
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn strip(v: &str) -> &str {
    match v.find(';') {
        Some(i) => v[..i].trim(),
        None => v.trim(),
    }
}

fn num(v: &str) -> Option<f32> {
    strip(v).parse::<f32>().ok().filter(|f| f.is_finite())
}

fn flag(v: &str) -> Option<bool> {
    match strip(v).to_ascii_lowercase().as_str() {
        "1" | "on" | "yes" | "true" => Some(true),
        "0" | "off" | "no" | "false" => Some(false),
        _ => None,
    }
}

/// `#rrggbb`, `rrggbb`, `aarrggbb` or `r,g,b` (0 to 255); None when it reads as none.
pub fn colour(v: &str) -> Option<u32> {
    let v = strip(v);
    if v.contains(',') {
        let p: Vec<u32> = v.split(',').filter_map(|s| s.trim().parse::<u32>().ok()).collect();
        return (p.len() == 3 && p.iter().all(|&c| c < 256)).then(|| 0xff00_0000 | p[0] << 16 | p[1] << 8 | p[2]);
    }
    let h = v.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(0xff00_0000 | n),
        8 => Some(n),
        _ => None,
    }
}

/// Where a key is looked up: the files' tables, nearest first.
struct Sources<'a>(Vec<(&'a a2ini::Table<'a>, &'static str)>);

impl<'a> Sources<'a> {
    /// The first source that names `key`, its value trimmed of a trailing comment.
    fn get(&self, key: &str) -> Option<&'a str> {
        self.0.iter().find_map(|(t, sec)| t.get(sec, key)).map(strip)
    }
    fn f(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(num)
    }
    fn pos(&self, key: &str, min: f32) -> Option<f32> {
        self.f(key).filter(|&v| v >= min)
    }
    fn c(&self, key: &str) -> Option<u32> {
        self.get(key).and_then(colour)
    }
}

/// The layout the three sources give. `file` and `set` are ini text (the override file's,
/// the set's whole table); `ui_title` is UI.ini's `[Menus] TitleImage`, `files_title` the
/// set's `Files.MenuTitle`.
pub fn resolve(file: Option<&str>, set: &str, ui_title: &str, files_title: &str) -> MainMenu {
    let ft = file.map(a2ini::Table::parse);
    let st = a2ini::Table::parse(set);
    let mut v = Vec::new();
    if let Some(t) = &ft {
        v.push((t, "MainMenu"));
    }
    v.push((&st, "MainMenu"));
    let s = Sources(v);
    let mut m = MainMenu::builtin();
    let line = |text: &str, size: &str, col: &str, d: &mut Line| {
        d.text = s.get(text).map(String::from);
        d.size = s.pos(size, 4.0).unwrap_or(d.size);
        d.colour = s.c(col);
    };
    line("Title1", "Title1Size", "Title1Colour", &mut m.line1);
    line("Title2", "Title2Size", "Title2Colour", &mut m.line2);
    m.rule = s.get("Rule").and_then(flag).unwrap_or(m.rule);
    m.rule_colour = s.c("RuleColour");
    // The picture: UI.ini, the file, the set's [MainMenu], the set's Files.MenuTitle; a
    // key left empty names nothing and the next source speaks.
    let pic = if !ui_title.trim().is_empty() {
        ui_title.trim()
    } else {
        s.get("TitleImage").filter(|p| !p.is_empty()).unwrap_or_else(|| files_title.trim())
    };
    m.image = if pic.eq_ignore_ascii_case("none") { String::new() } else { String::from(pic) };
    m.crop = s.get("TitleCrop").and_then(|c| {
        let p: Vec<u32> = c.split(',').filter_map(|x| x.trim().parse::<u32>().ok()).collect();
        (p.len() == 4 && p[2] > 0 && p[3] > 0).then(|| [p[0], p[1], p[2], p[3]])
    });
    m.image_width = s.pos("TitleWidth", 16.0).unwrap_or(m.image_width);
    m.side = match s.get("Side").map(|x| x.to_ascii_lowercase()).as_deref() {
        Some("right") => Side::Right,
        Some("centre") | Some("center") => Side::Centre,
        _ => Side::Left,
    };
    m.x = s.f("X");
    m.y = s.f("Y");
    m.width = s.pos("Width", 40.0).unwrap_or(m.width);
    m.title_gap = s.f("TitleGap").unwrap_or(m.title_gap).max(0.0);
    m.entry_gap = s.pos("EntryGap", 0.0).unwrap_or(m.entry_gap);
    m.entry_height = s.pos("EntryHeight", 8.0).unwrap_or(m.entry_height);
    m.entry_size = s.pos("EntrySize", 4.0).unwrap_or(m.entry_size);
    m.opacity = s.f("Opacity").unwrap_or(m.opacity).clamp(0.0, 1.0);
    // [Entries] in the file, [MainMenu.Entries] in the set; the file's rows first.
    let mut rows: Vec<(String, Option<String>)> = Vec::new();
    let mut add = |es: &[(&str, &str)]| {
        for &(k, val) in es {
            let k = norm(k);
            if k.is_empty() || rows.iter().any(|r| r.0 == k) {
                continue;
            }
            let val = strip(val);
            let hide = flag(val) == Some(false) || val.eq_ignore_ascii_case("hide");
            rows.push((k, if hide { None } else if flag(val) == Some(true) || val.is_empty() { Some(String::new()) } else { Some(String::from(val)) }));
        }
    };
    if let Some(t) = &ft {
        add(t.entries("Entries"));
        add(t.entries("MainMenu.Entries"));
    }
    add(st.entries("MainMenu.Entries"));
    // An empty row (`Credits=1`) shows the entry as it is: no row.
    m.entries = rows.into_iter().filter(|r| r.1.as_deref() != Some("")).collect();
    m
}

// ---- the file, read again when it changes ----------------------------------------------

struct State {
    /// The override file's stamp and when it was last looked at, UI.ini's picture key.
    stamp: Option<([u32; 2], u32)>,
    checked: f64,
    ui_title: String,
    layout: &'static MainMenu,
}

static mut STATE: Option<State> = None;

fn path() -> String {
    crate::rt::game_path("Menu\\mainmenu.ini")
}

fn build(stamp: Option<([u32; 2], u32)>) -> &'static MainMenu {
    let file = stamp.and_then(|_| crate::rt::read_file(&path())).map(|b| String::from_utf8_lossy(&b).into_owned());
    let set = crate::engine::api::content_set().and_then(crate::content_sets::table).unwrap_or("");
    let cfg = crate::config::get().menus_title_image.clone();
    let m = resolve(file.as_deref(), set, &cfg, crate::content::get().files.menu_title);
    crate::log!(
        "menus: main menu layout: {}{}; title {}",
        if file.is_some() { "Menu\\mainmenu.ini, " } else { "" },
        if set.contains("[MainMenu") { "the set's [MainMenu]" } else { "the built-in defaults" },
        if m.image.is_empty() { String::from("text") } else { m.image.clone() }
    );
    alloc::boxed::Box::leak(alloc::boxed::Box::new(m))
}

/// The layout now: the file's, looked at twice a second and read again when its write
/// time or size changed, or when UI.ini's picture key did.
#[allow(static_mut_refs)]
pub fn get() -> &'static MainMenu {
    let now = crate::rt::now();
    let st = unsafe { &mut STATE };
    let cfg = &crate::config::get().menus_title_image;
    match st {
        None => {
            let stamp = crate::rt::file_stamp(&path());
            *st = Some(State { stamp, checked: now, ui_title: cfg.clone(), layout: build(stamp) });
        }
        Some(s) if now - s.checked >= 0.5 => {
            s.checked = now;
            let stamp = crate::rt::file_stamp(&path());
            if stamp != s.stamp || s.ui_title != *cfg {
                s.stamp = stamp;
                s.ui_title = cfg.clone();
                s.layout = build(stamp);
            }
        }
        _ => {}
    }
    st.as_ref().unwrap().layout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_when_nothing_is_named() {
        assert_eq!(resolve(None, "", "", ""), MainMenu::builtin());
    }

    #[test]
    fn file_beats_set_and_picture_chain() {
        let set = "[MainMenu]\nTitleGap = 40\nSide = left\nTitleImage = a.png\n[MainMenu.Entries]\nCredits = 0\n[Files]\nMenuTitle = f.png\n";
        let m = resolve(None, set, "", "f.png");
        assert_eq!((m.title_gap, m.image.as_str()), (40.0, "a.png"));
        assert_eq!(m.entry("CREDITS"), None);
        let file = "[MainMenu]\nSide=Right\nTitleGap=5 ; tight\nTitleImage=none\nTitle2=ROOTS\n[Entries]\nSinglePlayer=Campaign\nCredits=1\n";
        let m = resolve(Some(file), set, "", "f.png");
        assert_eq!((m.side, m.title_gap, m.image.as_str()), (Side::Right, 5.0, ""));
        assert_eq!(m.line2.text.as_deref(), Some("ROOTS"));
        assert_eq!(m.entry("SINGLE PLAYER").as_deref(), Some("Campaign"));
        assert_eq!(m.entry("CREDITS").as_deref(), Some("CREDITS"));
        assert_eq!(resolve(Some(file), set, "ui.png", "f.png").image, "ui.png");
        assert_eq!(resolve(None, "", "", "f.png").image, "f.png");
        assert_eq!(resolve(None, "[MainMenu]\nTitleImage=\n", "", "f.png").image, "f.png");
    }

    #[test]
    fn colours() {
        assert_eq!(colour("#ff8000"), Some(0xffff8000));
        assert_eq!(colour("255, 128, 0 ; x"), Some(0xffff8000));
        assert_eq!(colour("zz"), None);
    }
}
