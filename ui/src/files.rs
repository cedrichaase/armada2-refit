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
//!
//! Another game's executable picks its own list (`GAMES`, by the executable's file name,
//! as render3d's): Fleet Operations: Roots (`LAYOUT_FO_ROOTS`, `games/fo-roots.toml`)
//! keeps its textures in `Textures\` itself, TGA and DDS (`texture_exts`). Roots' mods
//! (`Data\Mods\<name>`) are not mounted: the UI reads only a map's sky from there today.
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

// files: layout fo-roots (tools/layers.py --check reads these rows)
/// Fleet Operations: Roots' mount list (`games/fo-roots.toml` `[files]`): the game
/// directory is its `Data\`.
pub const LAYOUT_FO_ROOTS: &[(&str, &[&str])] = &[
    ("textures", &["Textures"]),
];
// files: end

/// A game's files: its mount list and its texture file extensions, in the order a name
/// present with both is taken.
pub struct Game {
    pub layout: &'static [(&'static str, &'static [&'static str])],
    pub texture_exts: &'static [&'static str],
    /// The HUD draws the game's own icon art before the UI's flat icons (`icons::game_art_first`):
    /// on Roots, whose icons are Roots' own (PLAN-FLEETOPS.md, decision 340).
    pub game_icons: bool,
}

const ARMADA2: Game = Game { layout: LAYOUT, texture_exts: &[".tga"], game_icons: false };

/// The games other than Armada II, by their executable's file name (any case).
const GAMES: &[(&str, Game)] = &[("ArmadaL.exe", Game { layout: LAYOUT_FO_ROOTS, texture_exts: &[".tga", ".dds"], game_icons: true })];

/// This process's game: by the executable's name, Armada II's for any other.
pub fn game() -> &'static Game {
    let exe = crate::rt::exe_name();
    GAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(exe)).map(|(_, g)| g).unwrap_or(&ARMADA2)
}

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
            for (at, dirs) in game().layout {
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
