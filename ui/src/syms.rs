//! The engine profile of UI.asi (`profiles/gog-1.1/ui.<part>.toml`, crates/a2engine): one
//! generated table per part (`syms_<part>.rs`), started together here. A hook calls
//! `ready` on its part's engine before it writes anything and stands down with the
//! engine's reason; the rows a hook feeds get their note from `note_for`.
use crate::{syms_core, syms_dialogs, syms_forms, syms_hud, syms_info, syms_loading, syms_panels, syms_radar, syms_shell};
use a2engine::Table;
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Every table of the UI, in the order they are started (the first logs the identity).
static TABLES: [(&str, &Table); 9] = [
    ("core", &syms_core::TABLE),
    ("loading", &syms_loading::TABLE),
    ("radar", &syms_radar::TABLE),
    ("info", &syms_info::TABLE),
    ("hud", &syms_hud::TABLE),
    ("panels", &syms_panels::TABLE),
    ("shell", &syms_shell::TABLE),
    ("dialogs", &syms_dialogs::TABLE),
    ("forms", &syms_forms::TABLE),
];

/// The table being started (an index into `TABLES`): its log lines are filtered.
static STARTING: AtomicUsize = AtomicUsize::new(0);

/// The resolvers log the identity, the build and the test line once per table; the UI
/// logs them once, and each later table only its own summary, named.
fn engine_log(s: &str) {
    let k = STARTING.load(Relaxed);
    if k == 0 {
        crate::log!("{}", s);
    } else if let Some(rest) = s.strip_prefix("engine: symbols:") {
        crate::log!("engine: {} symbols:{}", TABLES[k].0, rest);
    } else if !(a2engine::is_identity_line(s) || s.starts_with("engine: build ") || s == "engine: unknown build" || s.starts_with("engine: TEST:")) {
        crate::log!("{}", s);
    }
}

/// Starts every table. Once, at startup, before any hook.
pub fn init() {
    for (k, (_, t)) in TABLES.iter().enumerate() {
        STARTING.store(k, Relaxed);
        a2engine::init(t, engine_log);
    }
}

fn nul(n: &str, buf: &'static mut [u8; 160]) -> *const u8 {
    let b = n.as_bytes();
    let len = b.len().min(159);
    buf[..len].copy_from_slice(&b[..len]);
    buf[len] = 0;
    buf.as_ptr()
}

/// The engine's note for a settings row (`Section`, `Key`): why a hook feeding it stood
/// down, or on a build Refit does not know the plugin note. None when nothing stood
/// down. Copied NUL-terminated into a static buffer (read at once by the caller).
pub fn note_for(section: &str, key: &str) -> Option<*const u8> {
    static mut BUF: [u8; 160] = [0; 160];
    let n = TABLES.iter().find_map(|(_, t)| t.engine().note_for(section, key))?;
    Some(nul(n, unsafe { &mut *core::ptr::addr_of_mut!(BUF) }))
}

/// The plugin as a whole: the note of an unknown build.
pub fn note_plugin() -> Option<*const u8> {
    static mut BUF: [u8; 160] = [0; 160];
    let n = syms_core::TABLE.engine().note_plugin()?;
    Some(nul(n, unsafe { &mut *core::ptr::addr_of_mut!(BUF) }))
}
