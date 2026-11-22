//! Content sets (README.md, "Content sets"; MODDING.md, "Content sets"): a set is a
//! folder, `render3d/content/sets/<set>/`, holding one table per module
//! (`<module>.ini`), and every file under `render3d/content/` is built in (`build.rs`).
//! A module's `content.rs` parses its table at first use; the set id is the one the
//! engine profile gives the module (`Engine::content()`, `profiles/<build>/content.toml`).
//! A set with no table for a module, or no set, is the empty table: the module has no
//! built-in names and the engine's own look shows through.

mod files {
    include!(concat!(env!("OUT_DIR"), "/content_files.rs"));
}

/// A built-in file of `render3d/content/` by its path there (`sky/mbg02.ini`).
pub fn file(path: &str) -> Option<&'static str> {
    files::FILES.iter().find(|(p, _)| *p == path).map(|(_, t)| *t)
}

/// The set `set`'s table for `module` (`sets/<set>/<module>.ini`), if the set has one.
pub fn set_table(set: &str, module: &str) -> Option<&'static str> {
    files::FILES.iter().find(|(p, _)| {
        p.strip_prefix("sets/")
            .and_then(|r| r.strip_prefix(set))
            .and_then(|r| r.strip_prefix('/'))
            .and_then(|r| r.strip_prefix(module))
            .is_some_and(|r| r == ".ini")
    }).map(|(_, t)| *t)
}

/// The table of the set this build gives `module` (starting the module's symbol table
/// if nothing has yet: a recipe is read before any hook). None while the build has no
/// set for it (an unknown build, `none`) or the set has no table for it.
pub fn table(module: &'static str, t: &'static a2engine::Table) -> Option<&'static str> {
    crate::engine::start_table(module, t);
    set_table(t.engine().content()?, module)
}

/// Whether the build gives `module` a set at all (its table may still be absent).
pub fn has_set(t: &'static a2engine::Table) -> bool {
    t.engine().content().is_some()
}

/// A module's parsed table, made once: `get` returns the cached one, else parses the
/// table `table` gives (None: `none`, not cached, since the set is not known before the
/// symbol table starts).
pub struct Cache<T: 'static> {
    cell: core::cell::UnsafeCell<Option<&'static T>>,
}

// The plugin's frame and hooks run on the game's one thread.
unsafe impl<T> Sync for Cache<T> {}

impl<T> Cache<T> {
    pub const fn new() -> Self {
        Cache { cell: core::cell::UnsafeCell::new(None) }
    }

    pub fn get(&self, module: &'static str, t: &'static a2engine::Table, parse: fn(&'static str) -> T, none: &'static T) -> &'static T {
        if let Some(c) = unsafe { *self.cell.get() } {
            return c;
        }
        let text = table(module, t);
        if !has_set(t) {
            return none;
        }
        let c: &'static T = alloc::boxed::Box::leak(alloc::boxed::Box::new(parse(text.unwrap_or(""))));
        unsafe { *self.cell.get() = Some(c) };
        c
    }
}
