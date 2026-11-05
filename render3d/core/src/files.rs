//! The game's files as render3d sees them: one `a2fmt::vfs` file system, built at the
//! first lookup, over the folders of the game directory. Every asset (`assets`), recipe
//! (`recipes`) and texture a module reads goes through it, by a virtual path:
//! `textures\<file>`, `sod\...`, `sprites\...`, `odf\...`, and the recipe override
//! folders `sky\`, `surfaces\`, `nebulae\` and `menu\` (MODDING.md, "Content recipes").
//!
//! [`LAYOUT`] is the mount list: each virtual folder, and the folders under the game
//! directory it searches, in order. Its rows `textures`, `sod`, `sprites` and `odf` are
//! `games/armada2.toml`'s `[files]`, and `tools/layers.py --check` holds the two equal;
//! the override folders are ours and the same for every game. Archives (`[files]
//! archives`) would mount behind the folders; Armada II has none. With this layout every
//! path resolves to the file the game directory held at that path before the file system
//! existed, so stock finds the same files in the same order.
use crate::{log, rt};
pub use a2fmt::vfs::Entry;
use a2fmt::vfs::{FnIo, Layered, Loose, Status, Vfs};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// files: layout armada2 (tools/layers.py --check reads these rows)
/// The mount list: (virtual folder, folders under the game directory, searched in order).
pub const LAYOUT: &[(&str, &[&str])] = &[
    ("textures", &["Textures\\RGB"]),
    ("sod", &["SOD"]),
    ("sprites", &["Sprites"]),
    ("odf", &["odf"]),
    ("sky", &["Sky"]),
    ("surfaces", &["Surfaces"]),
    ("nebulae", &["Nebulae"]),
    ("menu", &["Menu"]),
];
// files: end

fn list(dir: &str) -> Vec<Entry> {
    let mut pat = String::from(dir);
    pat.push_str("\\*");
    rt::list_entries(&pat)
        .into_iter()
        .filter(|(n, _)| n != "." && n != "..")
        .map(|(name, dir)| Entry { name, dir })
        .collect()
}

/// `a2rt`'s Win32 file calls; Windows (and Wine) ignore case themselves.
const IO: FnIo = FnIo { read: rt::read_file, is_file: rt::file_exists, list, separator: '\\', case_insensitive: true };

static mut VFS: Option<Layered> = None;

/// The file system, built at the first call.
#[allow(static_mut_refs)]
pub fn vfs() -> &'static Layered {
    unsafe {
        if VFS.is_none() {
            let mut v = Layered::new();
            for (at, dirs) in LAYOUT {
                for d in *dirs {
                    v.mount(at, Box::new(Loose::new(IO, &rt::game_path(d))));
                }
            }
            for (at, label, status) in v.mounts() {
                if let Status::Unsupported(why) = status {
                    log!("files: {} at {}: {}", label, at, why);
                }
            }
            VFS = Some(v);
        }
        VFS.as_ref().unwrap()
    }
}

/// A whole file by virtual path (`textures\mnebula4.tga`, `Sky\aqua.ini`).
pub fn open(path: &str) -> Option<Vec<u8>> {
    vfs().open(path)
}

/// Whether a file is there.
pub fn exists(path: &str) -> bool {
    vfs().exists(path)
}

/// A virtual folder's entries, as (name, is_folder).
pub fn list_dir(dir: &str) -> Vec<Entry> {
    vfs().list(dir)
}
