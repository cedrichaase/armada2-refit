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

/// The fixed export names (A2SET_ABI 1.0). `Display8_Settings` is reserved.
const EXPORTS: [&str; 6] = ["Render3D_Settings", "UI_Settings", "QOL_Settings", "QOLRules_Settings", "Online_Settings", "BinkProxy_Settings"];

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
}

static mut TABLES: Vec<Table> = Vec::new();
static mut STARTED: bool = false;
static mut STAMP: Option<([u32; 2], u32)> = None;
static mut NEXT_CHECK: f64 = 0.0;

/// Every table found (empty before the first frame).
#[allow(static_mut_refs)]
pub fn tables() -> &'static [Table] {
    unsafe { &TABLES }
}

/// Once per UI frame, every context.
#[allow(static_mut_refs)]
pub fn frame() {
    unsafe {
        if !STARTED {
            STARTED = true;
            start();
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
        let rows = if base.is_null() || count == 0 { Some(Vec::new()) } else { read_rows(export, base, count) };
        let Some(rows) = rows_or_refused(export, &module, rows) else { continue };
        log!("settings: {} in {}, {} rows{}", export, module, rows.len(), if changed.is_some() { "" } else { " (no _SettingsChanged)" });
        unsafe { TABLES.push(Table { plugin, module, rows, changed }) };
    }
    unsafe {
        STAMP = rt::file_stamp(&overlay::path());
        NEXT_CHECK = rt::now() + 1.0;
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
        } else if r.tab > TAB_MOVIES {
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
