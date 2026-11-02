//! `RefitPlayer.ini`, the player's settings overlay in the game directory
//! (tools/settings/README.md): read in front of every ini of the game directory by
//! `Ini::lookup`, written only by UI.asi through `set` / `set_row` (one writer
//! process, one writing DLL). Installers, `a2mod` and the release package never touch
//! it. The edit is `a2ini::edit`, which keeps every line it does not change.
use crate::settings::Setting;
use crate::{Ini, cstr, game_path, log};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, Ordering};

pub const FILE: &str = "RefitPlayer.ini";
/// The UI's player file before the overlay (ui 0.29.0 migrates its one key).
pub const OLD_FILE: &str = "UIPlayer.ini";

static PATH: AtomicPtr<Vec<u8>> = AtomicPtr::new(core::ptr::null_mut());

/// The overlay's path, NUL-terminated (made once, after `init`).
pub(crate) fn path_c() -> &'static [u8] {
    let mut p = PATH.load(Ordering::Acquire);
    if p.is_null() {
        p = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(cstr(&game_path(FILE))));
        PATH.store(p, Ordering::Release);
    }
    unsafe { &*p }
}

/// The overlay's path.
pub fn path() -> String {
    game_path(FILE)
}

/// The overlay's section for `[sec]` of the plugin ini `file`: `Render3D.World` for
/// ("Render3D.ini", "World").
pub fn section(file: &str, sec: &str) -> String {
    let stem = if file.len() > 4 && file[file.len() - 4..].eq_ignore_ascii_case(".ini") { &file[..file.len() - 4] } else { file };
    crate::sfmt!("{}.{}", stem, sec)
}

/// (lower-cased "Stem.Section.key", value) of every overlay value logged.
static mut SEEN: Vec<(String, String)> = Vec::new();

/// Logs an overlay value the first time it is returned and whenever it changes:
/// `settings: RefitPlayer.ini [Render3D.World] Surfaces=0 overrides Render3D.ini's Surfaces=1`.
#[allow(static_mut_refs)]
pub(crate) fn note(osec: &str, key: &str, value: &str, file: &str, mine: impl FnOnce() -> Option<String>) {
    if !crate::log_is_open() {
        return;
    }
    let mut id = crate::lower(osec);
    id.push('.');
    id.push_str(&crate::lower(key));
    let seen = unsafe { &mut SEEN };
    match seen.iter_mut().find(|e| e.0 == id) {
        Some(e) if e.1 == value => return,
        Some(e) => e.1 = String::from(value),
        None => seen.push((id, String::from(value))),
    }
    match mine() {
        Some(m) => log!("settings: {} [{}] {}={} overrides {}'s {}={}", FILE, osec, key, value, file, key, m),
        None => log!("settings: {} [{}] {}={} ({} has no {})", FILE, osec, key, value, file, key),
    }
}

/// Sets `[<Stem>.<sec>] key` of the overlay to `value`, or removes it (None). Nothing is
/// written when the text does not change; otherwise the whole file goes to
/// `RefitPlayer.ini.tmp`, which is renamed over the overlay, and every cached ini is
/// marked stale. False when the value was refused or the file could not be written.
pub fn set(file: &str, sec: &str, key: &str, value: Option<&str>) -> bool {
    let osec = section(file, sec);
    if let Some(v) = value
        && !a2ini::valid_value(v)
    {
        log!("settings: {} [{}] {}={:?} refused (CR, LF or surrounding whitespace)", FILE, osec, key, v);
        return false;
    }
    let path = path();
    let old = match crate::read_file(&path) {
        Some(b) => a2ini::from_latin1(&b),
        None if crate::file_exists(&path) => {
            log!("settings: {} could not be read; [{}] {} not written", FILE, osec, key);
            return false;
        }
        None => String::new(),
    };
    let new = a2ini::edit(&old, &osec, key, value);
    if new == old {
        return true;
    }
    let tmp = crate::sfmt!("{}.tmp", path);
    if !crate::write_file(&tmp, &a2ini::to_latin1(&new)) || !crate::move_file(&tmp, &path) {
        crate::delete_file(&tmp);
        log!("settings: {} could not be written; [{}] {} unchanged", FILE, osec, key);
        return false;
    }
    crate::ini_refresh_all();
    match value {
        Some(v) => log!("settings: {} [{}] {}={} written", FILE, osec, key, v),
        None => log!("settings: {} [{}] {} removed", FILE, osec, key),
    }
    true
}

/// Sets a row to `value`, or removes its key when `value` is what the plugin reads
/// without the overlay (its ini's value, else the row's default; compared trimmed and
/// without regard to ASCII case): only choices that differ are stored.
pub fn set_row(row: &Setting, value: &str) -> bool {
    let plain = Ini::open_plain(&game_path(row.file()));
    let base = match plain.string(row.section(), row.key()) {
        Some(v) if !v.is_empty() => v,
        _ => String::from(row.def()),
    };
    let same = value.trim().eq_ignore_ascii_case(base.trim());
    set(row.file(), row.section(), row.key(), if same { None } else { Some(value) })
}
