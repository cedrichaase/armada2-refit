//! The game's files as the UI reads them: one `a2fmt::vfs` file system over the game
//! directory, built at the first lookup, as render3d's (`render3d/core/src/files.rs`).
//! A path is the game directory's (`Sprites\cursor.spr`, `bzn\<map>.bzn`), except the
//! textures, which are `textures\<file>`, wherever the game keeps them.
//!
//! [`LAYOUT`] is the mount list; its `textures` row is `games/armada2.toml`'s `[files]
//! textures`, and `tools/layers.py --check` holds the two equal. The game directory
//! itself is mounted at the root, behind the folders: a mount shadows the folder of its
//! name there, so `textures\` is only the texture roots. With this layout every path
//! reads the file the game directory held at that path before.
use a2fmt::vfs::{Entry, FnIo, Layered, Loose, Vfs};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// files: layout armada2 (tools/layers.py --check reads these rows)
/// The mount list: (virtual folder, folders under the game directory, searched in order).
pub const LAYOUT: &[(&str, &[&str])] = &[
    ("textures", &["Textures\\RGB"]),
];
// files: end

fn list(dir: &str) -> Vec<Entry> {
    let mut pat = String::from(dir);
    pat.push_str("\\*");
    crate::rt::list_entries(&pat)
        .into_iter()
        .filter(|(n, _)| n != "." && n != "..")
        .map(|(name, dir)| Entry { name, dir })
        .collect()
}

/// `a2rt`'s Win32 file calls; Windows (and Wine) ignore case themselves.
const IO: FnIo = FnIo {
    read: crate::rt::read_file,
    is_file: crate::rt::file_exists,
    list,
    separator: '\\',
    case_insensitive: true,
};

static mut VFS: Option<Layered> = None;

/// The file system, built at the first call.
#[allow(static_mut_refs)]
pub fn vfs() -> &'static Layered {
    unsafe {
        if VFS.is_none() {
            let mut v = Layered::new();
            for (at, dirs) in LAYOUT {
                for d in *dirs {
                    v.mount(at, Box::new(Loose::new(IO, &crate::rt::game_path(d))));
                }
            }
            v.mount("", Box::new(Loose::new(IO, crate::rt::game_dir())));
            VFS = Some(v);
        }
        VFS.as_ref().unwrap()
    }
}

/// A whole file by path (`textures\allrace.tga`, `Credits.txt`).
pub fn open(path: &str) -> Option<Vec<u8>> {
    vfs().open(path)
}

/// Whether a file is there.
pub fn exists(path: &str) -> bool {
    vfs().exists(path)
}
