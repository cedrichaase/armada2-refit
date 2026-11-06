//! Content sets (README.md, "Content sets"; MODDING.md, "Content sets"): a set is a
//! folder, `ui/content/sets/<set>/`, holding the UI's table `ui.ini`; every `.ini` under
//! `ui/content/` is built in (`build.rs`). `content.rs` parses the table of the set the
//! engine profile gives the UI (`engine::api::content_set`, `profiles/<build>/content.toml`).

mod files {
    include!(concat!(env!("OUT_DIR"), "/content_files.rs"));
}

/// The set `set`'s `ui.ini`, if the set has one.
pub fn table(set: &str) -> Option<&'static str> {
    files::FILES
        .iter()
        .find(|(p, _)| p.strip_prefix("sets/").and_then(|r| r.strip_prefix(set)).is_some_and(|r| r == "/ui.ini"))
        .map(|(_, t)| *t)
}

/// The UI's parsed table, made once: `get` returns the cached one, else parses the
/// table of the build's set (a set with no `ui.ini` is the empty text). With no set
/// (an unknown build, `none`, or before the table starts) it is `none`, not cached.
pub struct Cache<T: 'static> {
    cell: core::cell::UnsafeCell<Option<&'static T>>,
}

// The UI runs on the game's one thread.
unsafe impl<T> Sync for Cache<T> {}

impl<T> Cache<T> {
    pub const fn new() -> Self {
        Cache { cell: core::cell::UnsafeCell::new(None) }
    }

    pub fn get(&self, set: fn() -> Option<&'static str>, parse: fn(&'static str) -> T, none: &'static T) -> &'static T {
        if let Some(c) = unsafe { *self.cell.get() } {
            return c;
        }
        let Some(set) = set() else { return none };
        let c: &'static T = alloc::boxed::Box::leak(alloc::boxed::Box::new(parse(table(set).unwrap_or(""))));
        unsafe { *self.cell.get() = Some(c) };
        c
    }
}
