//! The game's keys: `Input.map` (the engine's key map) and the command bar's grid keys
//! (QOL.asi's), as the Refit screen's Keys tab lists and rebinds them (`MENUS.md`, "The
//! Keys tab").
//!
//! The player's choices are kept in the overlay like every other setting:
//! `RefitPlayer.ini [UI.Keys] <action>=<keys>` (`Control+Shift+F1`, keys by their
//! `Input.map` names), and `[QOL.Keys] CellN=<key>` for the grid. `Input.map` itself is
//! the engine's to read, so this module writes it: the stock file is kept beside it as
//! `Input.map.a2neb-backup` the first time a binding differs, every write is the stock
//! text with the player's blocks replaced (`a2fmt::inputmap::InputMap::rewrite`), and
//! with no choice left the stock file goes back and the backup goes. `sync` runs at
//! start, before the engine reads the file, and after every change; a change of
//! `Input.map` applies when the game is started again. The grid's keys are QOL's and
//! apply at once (the hub tells QOL).
use crate::rt;
use crate::{log, sfmt, settings};
use a2fmt::inputmap::{self, Binding, InputMap};
use alloc::string::String;
use alloc::vec::Vec;

/// The overlay's file and section for `Input.map`'s choices: `[UI.Keys]`.
const FILE: &str = "UI.ini";
const SEC: &str = "Keys";
/// The backup of the stock file, the suffix `a2mod` files a changed stock file by.
const BACKUP: &str = ".a2neb-backup";
/// Where the game keeps the key map, relative to its directory, in the order looked at.
const PLACES: [&str; 2] = ["Input.map", "misc\\Input.map"];
/// The command bar's cells (QOL.asi, `bar.c`).
pub const CELLS: usize = 15;

/// What a category's keys are.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The engine's own keys.
    General,
    /// The command bar's own keys: while the grid's keys are on (`BarKeys`) these do
    /// nothing (QOL empties the bar's hotkey tables around the stock handler).
    Bar,
    /// The editor's and the developers' keys (`Show advanced`).
    Debug,
}

/// A group of the Keys tab: one section of the file.
pub struct Cat {
    pub name: String,
    pub role: Role,
    /// Indices into `Map::entries`.
    pub entries: Vec<usize>,
}

/// One action the tab lists.
pub struct Entry {
    /// Index into `Map::stock.actions`.
    pub action: usize,
    pub cat: usize,
    /// What it does, for people: the comment above its block, else its name in words.
    pub label: String,
    /// The rest of the comment (or the comment of the block it follows when it has none).
    pub help: String,
}

pub struct Map {
    /// The full path of `Input.map`.
    pub path: String,
    /// The stock text (the backup's when there is one) and its parse.
    text: String,
    pub stock: InputMap,
    pub entries: Vec<Entry>,
    pub cats: Vec<Cat>,
    /// The player's choice per action (`[UI.Keys]`), as read last.
    mine: Vec<Option<Binding>>,
    /// What each action was bound to in the file the engine read at this start.
    start: Vec<Option<Binding>>,
}

static mut MAP: Option<Map> = None;
/// Why there is no map, for the tab.
static mut WHY: &str = "";

#[allow(static_mut_refs)]
pub fn map() -> Option<&'static Map> {
    unsafe { MAP.as_ref() }
}

#[allow(static_mut_refs)]
fn map_mut() -> Option<&'static mut Map> {
    unsafe { MAP.as_mut() }
}

/// Why the Keys tab has no `Input.map` rows, when it has none.
pub fn why() -> &'static str {
    unsafe { WHY }
}

fn exists(path: &str) -> bool {
    rt::file_exists(path)
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn to_latin1(s: &str) -> Vec<u8> {
    s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

/// The display name and role of a section of the file.
fn section_of(name: &str) -> (String, Role) {
    const NAMES: [(&str, &str, Role); 14] = [
        ("Key Descriptions", "General", Role::General),
        ("Saving", "Saving", Role::General),
        ("Interface Elements", "Interface", Role::General),
        ("Camera navigation", "Camera", Role::General),
        ("Selection", "Selection", Role::General),
        ("Button Bar Menu Shortcuts", "Bar: menus", Role::Bar),
        ("Action Menu Shortcuts", "Bar: actions", Role::Bar),
        ("Special weapons", "Bar: special weapons", Role::Bar),
        ("Orders menu", "Bar: orders", Role::Bar),
        ("AI menu", "Bar: AI", Role::Bar),
        ("Trade menu", "Bar: trade", Role::Bar),
        ("Build menu", "Bar: build", Role::Bar),
        ("Research menu", "Bar: research", Role::Bar),
        ("Debugging", "Editor and debugging", Role::Debug),
    ];
    for (start, shown, role) in NAMES {
        if name.starts_with(start) {
            return (String::from(shown), role);
        }
    }
    (String::from(if name.is_empty() { "Other" } else { name }), Role::General)
}

/// Actions that stand among the editor's keys in the file but are everyone's.
const EVERYONE: [&str; 2] = ["pauseGame", "mainMenu"];

/// `group_select_1` / `toggleEditMode` in words: "Group select 1", "Toggle edit mode".
fn words(name: &str) -> String {
    let mut s = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c == '_' {
            s.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_ascii_uppercase() && prev_lower {
            s.push(' ');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        s.push(if s.is_empty() { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() });
    }
    s
}

fn capital(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        out.push(if i == 0 { c.to_ascii_uppercase() } else { c });
    }
    out
}

fn build(path: String, text: String) -> Map {
    let stock = InputMap::parse(&text);
    let mut cats: Vec<Cat> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut last_help = String::new();
    let mut last_place = (String::new(), String::new());
    for (i, a) in stock.actions.iter().enumerate() {
        let place = (a.section.clone(), a.group.clone());
        if place != last_place {
            last_help.clear();
            last_place = place;
        }
        // Only what is on the keyboard: the modifiers' and the mouse's own blocks are not.
        if a.uses_mouse() || a.keys().is_empty() {
            continue;
        }
        let (name, role) = if EVERYONE.contains(&a.name.as_str()) { (String::from("General"), Role::General) } else { section_of(&a.section) };
        let cat = match cats.iter().position(|c| c.name == name) {
            Some(c) => c,
            None => {
                cats.push(Cat { name, role, entries: Vec::new() });
                cats.len() - 1
            }
        };
        let (label, help) = match a.comment.split_first() {
            Some((first, rest)) => {
                let mut h = String::new();
                for r in rest {
                    if !h.is_empty() {
                        h.push(' ');
                    }
                    h.push_str(r);
                }
                last_help = String::from(first.as_str());
                (capital(first), h)
            }
            None => (words(&a.name), last_help.clone()),
        };
        cats[cat].entries.push(entries.len());
        entries.push(Entry { action: i, cat, label, help });
    }
    // The everyone's-keys first, the editor's last, the file's order between.
    cats.sort_by_key(|c| match c.role {
        Role::General => 0,
        Role::Bar => 1,
        Role::Debug => 2,
    });
    for (ci, c) in cats.iter().enumerate() {
        for &e in &c.entries {
            entries[e].cat = ci;
        }
    }
    let n = stock.actions.len();
    Map { path, text, stock, entries, cats, mine: alloc::vec![None; n], start: alloc::vec![None; n] }
}

/// The player's choices, read again from the overlay.
fn read_mine(m: &mut Map) {
    let ini = rt::Ini::open(&rt::game_path(FILE));
    for (i, a) in m.stock.actions.iter().enumerate() {
        m.mine[i] = None;
        if !a.rebindable() {
            continue;
        }
        if let Some(v) = ini.string(SEC, &a.name)
            && !v.is_empty()
        {
            match Binding::read(&v) {
                Some(b) => m.mine[i] = Some(b),
                None => log!("keys: [UI.Keys] {}={} is not a key this screen knows; stock's kept", a.name, v),
            }
        }
    }
}

/// The text `Input.map` should hold: stock's with the player's blocks.
fn target(m: &Map) -> String {
    let ch: Vec<(&str, &Binding)> = m.stock.actions.iter().zip(m.mine.iter()).filter_map(|(a, b)| b.as_ref().map(|b| (a.name.as_str(), b))).collect();
    m.stock.rewrite(&m.text, &ch)
}

/// Writes `bytes` to `path` through a temporary file renamed over it.
fn replace(path: &str, bytes: &[u8]) -> bool {
    let tmp = sfmt!("{}.tmp", path);
    if rt::write_file(&tmp, bytes) && rt::move_file(&tmp, path) {
        return true;
    }
    rt::delete_file(&tmp);
    false
}

/// Makes `Input.map` what the player's choices say (module doc). True when the file
/// holds that now.
fn sync(m: &Map) -> bool {
    let bak = sfmt!("{}{}", m.path, BACKUP);
    let want = target(m);
    let live = rt::read_file(&m.path).map(|b| latin1(&b));
    let n = m.mine.iter().filter(|b| b.is_some()).count();
    if want == m.text {
        if exists(&bak) {
            if live.as_deref() != Some(m.text.as_str()) && !replace(&m.path, &to_latin1(&m.text)) {
                log!("keys: {} could not be written; the stock keys are in {}", m.path, bak);
                return false;
            }
            rt::delete_file(&bak);
            log!("keys: Input.map is stock again (no key of yours); the backup is gone");
        }
        return true;
    }
    if live.as_deref() == Some(want.as_str()) {
        return true;
    }
    if !exists(&bak) {
        let Some(orig) = rt::read_file(&m.path) else {
            log!("keys: {} could not be read; not written", m.path);
            return false;
        };
        if !rt::write_file(&bak, &orig) {
            log!("keys: {} could not be written; Input.map left as it is", bak);
            return false;
        }
    }
    if !replace(&m.path, &to_latin1(&want)) {
        log!("keys: {} could not be written; it keeps its keys", m.path);
        return false;
    }
    log!("keys: Input.map written: {} key{} of yours, the stock file in Input.map{}", n, if n == 1 { "" } else { "s" }, BACKUP);
    true
}

/// At start, before the engine reads the key map: finds and reads it, and writes the
/// player's choices into it.
pub fn start() {
    let Some(rel) = PLACES.iter().find(|r| exists(&rt::game_path(r))) else {
        unsafe { WHY = "Input.map was not found in the game folder." };
        log!("keys: no Input.map in {}", rt::game_dir());
        return;
    };
    let path = rt::game_path(rel);
    let bak = sfmt!("{}{}", path, BACKUP);
    let backed = exists(&bak);
    let from = if backed { bak } else { path.clone() };
    let Some(bytes) = rt::read_file(&from) else {
        unsafe { WHY = "Input.map could not be read." };
        log!("keys: {} could not be read", from);
        return;
    };
    let mut m = build(path, latin1(&bytes));
    read_mine(&mut m);
    let ok = sync(&m);
    let read = if ok { target(&m) } else { rt::read_file(&m.path).map_or_else(|| m.text.clone(), |b| latin1(&b)) };
    let now = InputMap::parse(&read);
    for (i, a) in m.stock.actions.iter().enumerate() {
        m.start[i] = now.find(&a.name).and_then(|b| b.binding());
    }
    let rebind = m.stock.actions.iter().filter(|a| a.rebindable()).count();
    log!(
        "keys: {} ({}): {} actions, {} rebindable, {} of yours",
        rel,
        if backed { "stock from the backup" } else { "stock" },
        m.stock.actions.len(),
        rebind,
        m.mine.iter().filter(|b| b.is_some()).count()
    );
    unsafe { MAP = Some(m) };
}

/// Reads the player's choices again (the tab opened: the overlay may have changed by
/// hand) and writes `Input.map` to match.
pub fn refresh() {
    if let Some(m) = map_mut() {
        read_mine(m);
        sync(m);
    }
}

/// The action an entry is.
pub fn action(e: &Entry) -> &'static inputmap::Action {
    &map().unwrap().stock.actions[e.action]
}

/// What an entry is bound to now: the player's choice, else stock's.
pub fn binding(e: &Entry) -> Option<Binding> {
    let m = map()?;
    let a = &m.stock.actions[e.action];
    match &m.mine[e.action] {
        Some(b) => Some(a.bound_to(b)),
        None => a.binding(),
    }
}

/// Stock's binding of an entry.
pub fn stock_binding(e: &Entry) -> Option<Binding> {
    map().and_then(|m| m.stock.actions[e.action].binding())
}

/// Whether the player chose an entry's keys.
pub fn is_mine(e: &Entry) -> bool {
    map().is_some_and(|m| m.mine[e.action].is_some())
}

/// Whether an entry's keys differ from the ones the engine read at this start.
pub fn pending(e: &Entry) -> bool {
    map().is_some_and(|m| binding(e) != m.start[e.action])
}

/// Binds an entry to `b`, or back to stock (None): the overlay, then `Input.map`.
pub fn set(e: &Entry, b: Option<&Binding>) -> bool {
    let Some(m) = map_mut() else { return false };
    let a = &m.stock.actions[e.action];
    let stock = a.binding();
    // Stock's own keys are no choice: the key is removed.
    let b = b.filter(|b| stock.as_ref() != Some(&a.bound_to(b)));
    let spelled = b.map(|b| b.spell());
    let name = a.name.clone();
    let old = binding(e).map(|b| b.label()).unwrap_or_default();
    let ok = settings::set(FILE, SEC, &name, spelled.as_deref());
    read_mine(m);
    let written = sync(m);
    let new = binding(e).map(|b| b.label()).unwrap_or_default();
    log!("keys: {} {} -> {}{}", name, old, new, if ok && written { " (restart)" } else { " (not written)" });
    ok && written
}

/// Every choice of the player's for `Input.map` and the grid removed; the number removed.
pub fn reset() -> usize {
    let mut keys: Vec<(&str, &str, &str)> = Vec::new();
    let names: Vec<String> = (1..=CELLS).map(|c| sfmt!("Cell{}", c)).collect();
    if let Some(m) = map() {
        for a in m.stock.actions.iter() {
            keys.push((FILE, SEC, a.name.as_str()));
        }
    }
    for n in names.iter() {
        keys.push(("QOL.ini", "Keys", n.as_str()));
    }
    let n = settings::remove_many(&keys);
    refresh();
    n
}

/// The entries bound so that some press answers both them and `b`, other than `skip`;
/// the command bar's own keys only when `bar` (they are off while the grid's keys are on).
pub fn others(b: &Binding, skip: Option<usize>, bar: bool) -> Vec<usize> {
    let Some(m) = map() else { return Vec::new() };
    m.entries
        .iter()
        .enumerate()
        .filter(|(i, e)| Some(*i) != skip && (bar || m.cats[e.cat].role != Role::Bar) && m.cats[e.cat].role != Role::Debug)
        .filter(|(_, e)| binding(e).is_some_and(|x| x.overlaps(b)))
        .map(|(i, _)| i)
        .collect()
}

// ---- the grid's keys (QOL.asi) ------------------------------------------------------

type KeyNameFn = extern "C" fn(u32) -> *const u8;

static mut GRID: Option<KeyNameFn> = None;
static mut GRID_ASKED: bool = false;

#[allow(static_mut_refs)]
fn grid_fn() -> Option<KeyNameFn> {
    unsafe {
        if !GRID_ASKED {
            GRID_ASKED = true;
            let a = settings::find_export("QOL_BarKeyName");
            if a != 0 {
                GRID = Some(core::mem::transmute::<usize, KeyNameFn>(a));
            }
        }
        GRID
    }
}

/// Whether QOL.asi says which key presses each cell (a QOL that has `[Keys]`).
pub fn has_grid() -> bool {
    grid_fn().is_some_and(|f| !f(0).is_null())
}

/// The key of a cell (0..15), by its `Input.map` name.
pub fn grid_key(cell: usize) -> Option<&'static str> {
    let p = grid_fn()?(cell as u32);
    if p.is_null() {
        return None;
    }
    let s = unsafe { rt::cstr_at(p, 32) };
    inputmap::key_by_name(s).map(|k| k.name)
}

/// Whether the player chose a cell's key.
pub fn grid_is_mine(cell: usize) -> bool {
    settings::holds("QOL.ini", "Keys", &sfmt!("Cell{}", cell + 1))
}

/// Sets a cell's key (None: the ini's), live: the hub tells QOL.
pub fn set_grid(cell: usize, key: Option<&str>) -> bool {
    let k = sfmt!("Cell{}", cell + 1);
    let old = grid_key(cell).unwrap_or("-");
    let ok = settings::set("QOL.ini", "Keys", &k, key);
    log!("keys: grid {} {} -> {}", k, old, grid_key(cell).unwrap_or("-"));
    ok
}

/// Whether a key may press a cell: not extended (QOL reads it by its scancode's virtual
/// key), and not Esc or Pause.
pub fn grid_can_take(k: &inputmap::Key) -> bool {
    k.scan < 0x100 && k.name != "Esc" && k.name != "Pause"
}
