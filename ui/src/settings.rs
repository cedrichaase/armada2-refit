//! The settings hub (tools/settings/README.md, contract C5): the one place that knows
//! every plugin's settings table and tells the plugins when `RefitPlayer.ini` changes.
//!
//! At its first frame (shell or mission) it walks the loaded modules and asks each for
//! the fixed exports (`<Plugin>_Settings`, `<Plugin>_SettingsChanged`), so no DLL is
//! looked up by its file name. Once a second it looks at the overlay's stamp; a change
//! it did not make itself (a hand edit, the bench) is passed on to every plugin's
//! `_SettingsChanged`. After its own writes (`set`, `set_row`) it calls them at once.
//! Without UI.asi an overlay change applies at the next launch.
use crate::log;
use crate::rt::{self, overlay, settings::*, win::*};
use alloc::string::String;
use alloc::vec::Vec;

/// The fixed export names (A2SET_ABI 1.1).
const EXPORTS: [&str; 6] = ["Display8_Settings", "Render3D_Settings", "UI_Settings", "QOL_Settings", "QOLRules_Settings", "Online_Settings"];

#[link(name = "kernel32")]
unsafe extern "system" {
    fn K32EnumProcessModules(process: HANDLE, modules: *mut HANDLE, cb: u32, needed: *mut u32) -> i32;
}

/// One plugin's table as the hub found it.
pub struct Table {
    /// "Render3D" (the export's name without `_Settings`).
    pub plugin: &'static str,
    /// The module's file name ("Render3D.asi").
    pub module: String,
    /// The rows this reader knows every code of, in screen order.
    pub rows: Vec<&'static Setting>,
    changed: Option<extern "C" fn()>,
    /// `<Plugin>_SettingsNote`, when the plugin exports it (C5 amendment).
    note: Option<NoteFn>,
}

/// A row's value at the hub's start and at the start of the current mission, for the
/// Refit screen's "pending" badges.
struct Snap {
    row: *const Setting,
    start: String,
    mission: String,
}

static mut TABLES: Vec<Table> = Vec::new();
static mut STARTED: bool = false;
static mut STAMP: Option<([u32; 2], u32)> = None;
static mut NEXT_CHECK: f64 = 0.0;
static mut SNAPS: Vec<Snap> = Vec::new();
/// A mission frame has been seen since the last shell or loading frame.
static mut IN_MISSION: bool = false;

/// Every table found (empty before the first frame).
#[allow(static_mut_refs)]
pub fn tables() -> &'static [Table] {
    unsafe { &TABLES }
}

/// Once per UI frame, every context. The first mission frame after the shell or a
/// loading screen takes the snapshot the Refit screen compares MISSION rows to.
#[allow(static_mut_refs)]
pub fn frame(ctx: crate::surface::Context) {
    use crate::surface::Context;
    unsafe {
        if !STARTED {
            STARTED = true;
            start();
        }
        match ctx {
            Context::Mission if !IN_MISSION => {
                IN_MISSION = true;
                for s in SNAPS.iter_mut() {
                    s.mission = value(&*s.row);
                }
            }
            Context::Shell | Context::Loading => IN_MISSION = false,
            _ => {}
        }
        let now = rt::now();
        if now < NEXT_CHECK {
            return;
        }
        NEXT_CHECK = now + 1.0;
        let s = rt::file_stamp(&overlay::path());
        if s != STAMP {
            STAMP = s;
            log!("settings: {} changed; told {}", overlay::FILE, notify());
        }
    }
}

/// Sets a row through the overlay (`overlay::set_row`) and tells the plugins.
pub fn set_row(row: &Setting, value: &str) -> bool {
    let ok = overlay::set_row(row, value);
    after_write();
    ok
}

/// Sets (or, with None, removes) a key of the overlay and tells the plugins.
pub fn set(file: &str, sec: &str, key: &str, value: Option<&str>) -> bool {
    let ok = overlay::set(file, sec, key, value);
    after_write();
    ok
}

/// Removes the overlay keys `(file, section, key)`, then tells the plugins once; the number
/// of keys the overlay held.
pub fn remove_many(keys: &[(&str, &str, &str)]) -> usize {
    let mut n = 0;
    for &(file, sec, key) in keys {
        if holds(file, sec, key) && overlay::set(file, sec, key, None) {
            n += 1;
        }
    }
    after_write();
    n
}

/// Whether the overlay holds a non-empty value for `[sec] key` of the plugin ini `file`.
pub fn holds(file: &str, sec: &str, key: &str) -> bool {
    let ini = rt::Ini::open(&overlay::path());
    ini.string(&overlay::section(file, sec), key).is_some_and(|v| !v.is_empty())
}

/// Whether the overlay holds the row's key (the Refit screen's "changed" dot).
pub fn overlay_holds(row: &Setting) -> bool {
    holds(row.file(), row.section(), row.key())
}

/// The row's effective value: the overlay's, else the plugin's ini's, else the default.
pub fn value(row: &Setting) -> String {
    rt::settings::value(&rt::settings::ini(row), row)
}

/// A row's value changed but not applied yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pending {
    No,
    /// A RESTART row that differs from its value at the start.
    Restart,
    /// A MISSION row that differs from its value at the start of this mission.
    Mission,
}

#[allow(static_mut_refs)]
pub fn pending(row: &Setting) -> Pending {
    let Some(s) = (unsafe { SNAPS.iter().find(|s| core::ptr::eq(s.row, row)) }) else { return Pending::No };
    let now = value(row);
    let differs = |was: &str| !now.trim().eq_ignore_ascii_case(was.trim());
    match row.apply {
        APPLY_RESTART if differs(&s.start) => Pending::Restart,
        APPLY_MISSION if crate::menus::backdrop::over_mission() && differs(&s.mission) => Pending::Mission,
        _ => Pending::No,
    }
}

/// Why a row cannot be changed now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lock {
    No,
    /// A network game, and every player must have the same.
    Network,
    /// A mission whose mode could not be read: locked to be safe.
    Unknown,
}

/// Rows every player must share are locked in a network mission (and where that cannot
/// be told). The Refit screen is reached only from Options, so the setup screens never
/// show it.
pub fn locked(row: &Setting) -> Lock {
    if row.scope != SCOPE_MATCH || !crate::menus::backdrop::over_mission() {
        return Lock::No;
    }
    match crate::engine::api::setup::network_game() {
        Some(false) => Lock::No,
        Some(true) => Lock::Network,
        None => Lock::Unknown,
    }
}

/// `<Plugin>_SettingsNote` for `row` of the table `plugin` (None: the plugin as a whole).
pub fn note(plugin: &str, row: Option<&Setting>) -> Option<String> {
    let t = tables().iter().find(|t| t.plugin == plugin)?;
    let f = t.note?;
    let p = f(row.map_or(core::ptr::null(), |r| r as *const Setting));
    if p.is_null() {
        return None;
    }
    let s = unsafe { rt::cstr_at(p, 120) };
    (!s.is_empty()).then(|| String::from(s))
}

#[allow(static_mut_refs)]
fn after_write() {
    unsafe {
        if !STARTED {
            return;
        }
        let s = rt::file_stamp(&overlay::path());
        if s != STAMP {
            STAMP = s;
            log!("settings: {} written; told {}", overlay::FILE, notify());
        }
    }
}

/// Every plugin's `_SettingsChanged`, after the UI's own cache is marked stale; the
/// names told, for the log.
#[allow(static_mut_refs)]
fn notify() -> String {
    rt::ini_refresh_all();
    let mut told = String::new();
    for t in unsafe { TABLES.iter() } {
        if let Some(f) = t.changed {
            f();
            if !told.is_empty() {
                told.push_str(", ");
            }
            told.push_str(t.plugin);
        }
    }
    if told.is_empty() {
        told.push_str("nobody");
    }
    told
}

/// The address of an export of any loaded module, or 0: a plugin is found by what it
/// exports, never by its file name. A few microseconds; ask when needed, not every frame.
pub fn find_export(name: &str) -> usize {
    let mut mods: Vec<HANDLE> = alloc::vec![core::ptr::null_mut(); 512];
    let mut needed = 0u32;
    if unsafe { K32EnumProcessModules(GetCurrentProcess(), mods.as_mut_ptr(), (mods.len() * 4) as u32, &mut needed) } == 0 {
        return 0;
    }
    mods.truncate((needed as usize / 4).min(mods.len()));
    let c = rt::cstr(name);
    mods.iter().map(|&m| unsafe { GetProcAddress(m, c.as_ptr()) }).find(|&f| f != 0).unwrap_or(0)
}

fn module_name(m: HANDLE) -> String {
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(m, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let p = &buf[..n.min(buf.len())];
    let start = p.iter().rposition(|&c| c == b'\\' || c == b'/').map_or(0, |i| i + 1);
    String::from_utf8_lossy(&p[start..]).into_owned()
}

#[allow(static_mut_refs)]
fn start() {
    crate::prefs::migrate();
    let mut mods: Vec<HANDLE> = alloc::vec![core::ptr::null_mut(); 512];
    let mut needed = 0u32;
    let ok = unsafe { K32EnumProcessModules(GetCurrentProcess(), mods.as_mut_ptr(), (mods.len() * 4) as u32, &mut needed) } != 0;
    if !ok {
        log!("settings: the modules could not be listed; no tables");
    }
    mods.truncate(if ok { (needed as usize / 4).min(mods.len()) } else { 0 });
    for export in EXPORTS {
        let name = rt::cstr(export);
        let found = mods.iter().find_map(|&m| {
            let f = unsafe { GetProcAddress(m, name.as_ptr()) };
            (f != 0).then_some((m, f))
        });
        let Some((m, f)) = found else {
            log!("settings: no {}", export);
            continue;
        };
        let plugin = &export[..export.len() - "_Settings".len()];
        let module = module_name(m);
        let get: extern "C" fn(*mut u32) -> *const Setting = unsafe { core::mem::transmute(f) };
        let mut count = 0u32;
        let base = get(&mut count);
        let changed_name = rt::cstr(&rt::sfmt!("{}_SettingsChanged", plugin));
        let cf = unsafe { GetProcAddress(m, changed_name.as_ptr()) };
        let changed: Option<extern "C" fn()> = (cf != 0).then(|| unsafe { core::mem::transmute::<usize, extern "C" fn()>(cf) });
        let note_name = rt::cstr(&rt::sfmt!("{}_SettingsNote", plugin));
        let nf = unsafe { GetProcAddress(m, note_name.as_ptr()) };
        let note: Option<NoteFn> = (nf != 0).then(|| unsafe { core::mem::transmute::<usize, NoteFn>(nf) });
        let rows = if base.is_null() || count == 0 { Some(Vec::new()) } else { read_rows(export, base, count) };
        let Some(rows) = rows_or_refused(export, &module, rows) else { continue };
        log!("settings: {} in {}, {} rows{}", export, module, rows.len(), if changed.is_some() { "" } else { " (no _SettingsChanged)" });
        for r in rows.iter() {
            let v = value(r);
            unsafe { SNAPS.push(Snap { row: *r as *const Setting, start: v.clone(), mission: v }) };
        }
        unsafe { TABLES.push(Table { plugin, module, rows, changed, note }) };
    }
    retire_overlay();
    migrate_moved();
    unsafe {
        STAMP = rt::file_stamp(&overlay::path());
        NEXT_CHECK = rt::now() + 1.0;
    }
}

/// What the overlay may no longer hold (R-C3): a keyed value of a retired kind. `Token`
/// takes one token out of a comma list (the key goes when none is left); `Key` takes the
/// key out. Direct3D 9 and Display8's forward mode are gone, and Display8 skips them on
/// its own read side; the hub, the overlay's only writer, cleans the file.
enum Retired {
    Token(&'static str),
    Key,
}
const RETIRED: [(&str, &str, &str, Retired); 3] = [
    ("Display8.ini", "Display8", "Backend", Retired::Token("d3d9")),
    ("Display8.ini", "Display8", "Mode", Retired::Key),
    ("Display8.ini", "Display8", "Real", Retired::Key),
];

/// Applies `RETIRED` to the overlay, once at the first frame, logged per key; the
/// plugins are told after. A `Backend=` list like `d3d12,vulkan` keeps its other tokens.
fn retire_overlay() {
    let mut any = false;
    for (file, sec, key, kind) in RETIRED.iter() {
        let ini = rt::Ini::open(&overlay::path());
        let section = overlay::section(file, sec);
        let Some(v) = ini.string(&section, key) else { continue };
        let new: Option<String> = match kind {
            Retired::Key => None,
            Retired::Token(t) => {
                let all = v.split(',').filter(|x| !x.trim().is_empty()).count();
                let kept: Vec<&str> = v.split(',').map(|x| x.trim()).filter(|x| !x.is_empty() && !x.eq_ignore_ascii_case(t)).collect();
                if kept.len() == all {
                    continue; // no such token
                }
                if kept.is_empty() { None } else { Some(kept.join(",")) }
            }
        };
        let ok = overlay::set(file, sec, key, new.as_deref());
        match &new {
            None => log!("settings: {} [{}] {}={} retired; removed{}", overlay::FILE, section, key, v, if ok { "" } else { " (failed)" }),
            Some(n) => log!("settings: {} [{}] {}={} retired; now {}{}", overlay::FILE, section, key, v, n, if ok { "" } else { " (failed)" }),
        }
        any |= ok;
    }
    if any {
        rt::ini_refresh_all();
        log!("settings: told {}", notify());
    }
}

/// A row's `moved_from` (C2): the overlay's value of the key it took over goes to the
/// row's own key (the new key wins when both are there), and the plugins are told, so a
/// choice made under the old name survives the move. Once: the old key is gone after.
#[allow(static_mut_refs)]
fn migrate_moved() {
    let mut any = false;
    for t in unsafe { TABLES.iter() } {
        for r in t.rows.iter() {
            let m = r.moved_from();
            let Some((stem, rest)) = m.split_once('.') else { continue };
            let Some((sec, key)) = rest.rsplit_once('.') else { continue };
            let from = rt::sfmt!("{}.{}", stem, sec);
            let to = overlay::section(r.file(), r.section());
            any |= overlay::rename(&from, key, &to, r.key());
        }
    }
    if any {
        // the rows' values changed under the snapshots taken before the move
        for s in unsafe { SNAPS.iter_mut() } {
            s.start = value(unsafe { &*s.row });
            s.mission = s.start.clone();
        }
        rt::ini_refresh_all();
        log!("settings: {} moved keys; told {}", overlay::FILE, notify());
    }
}

/// None: the table is refused (rows smaller than version 1's).
fn rows_or_refused(export: &str, module: &str, rows: Option<Vec<&'static Setting>>) -> Option<Vec<&'static Setting>> {
    if rows.is_none() {
        log!("settings: {} in {}: rows smaller than {} bytes; table refused", export, module, MIN_ROW_SIZE);
    }
    rows
}

/// The rows of a table, stepping by `rows[0].size`; a row with a code this reader does
/// not know is hidden and logged. None when the rows are smaller than version 1's.
fn read_rows(export: &str, base: *const Setting, count: u32) -> Option<Vec<&'static Setting>> {
    let size = unsafe { (*base).size };
    if size < MIN_ROW_SIZE {
        return None;
    }
    let mut out = Vec::new();
    for i in 0..count as usize {
        let r: &'static Setting = unsafe { &*((base as *const u8).add(i * size as usize) as *const Setting) };
        let bad = if !matches!(r.kind, KIND_BOOL | KIND_ENUM) {
            Some(("kind", r.kind))
        } else if !matches!(r.apply, APPLY_LIVE | APPLY_MISSION | APPLY_RESTART) {
            Some(("apply", r.apply))
        } else if !matches!(r.scope, SCOPE_LOCAL | SCOPE_MATCH) {
            Some(("scope", r.scope))
        } else if r.tab > TAB_MULTIPLAYER {
            Some(("tab", r.tab))
        } else {
            None
        };
        match bad {
            Some((what, v)) => log!("settings: {} row {} ([{}] {}) hidden: {} {} unknown", export, i, r.section(), r.key(), what, v),
            None => out.push(r),
        }
    }
    Some(out)
}
