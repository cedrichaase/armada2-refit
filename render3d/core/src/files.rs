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
//! archives`) mount behind the folders; Armada II has none. With this layout every
//! path resolves to the file the game directory held at that path before the file system
//! existed, so stock finds the same files in the same order.
//!
//! Another game's executable picks its own list ([`GAMES`], by the executable's file
//! name): Fleet Operations: Roots ([`LAYOUT_FO_ROOTS`], `games/fo-roots.toml`) keeps its
//! textures in `Textures\` itself, TGA and DDS, and its ODFs in `ODF.fpq`.
use crate::{log, rt};
pub use a2fmt::vfs::Entry;
use a2fmt::vfs::{FnIo, Fpq, Layered, Loose, Status, Vfs};
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

// files: layout fo-roots (tools/layers.py --check reads these rows)
/// Fleet Operations: Roots' mount list (`games/fo-roots.toml` `[files]`): the game
/// directory is its `Data\`; the archives below mount behind the folders.
pub const LAYOUT_FO_ROOTS: &[(&str, &[&str])] = &[
    ("textures", &["Textures"]),
    ("sod", &["SOD"]),
    ("sprites", &["Sprites"]),
    ("odf", &["odf"]),
    ("sky", &["Sky"]),
    ("surfaces", &["Surfaces"]),
    ("nebulae", &["Nebulae"]),
    ("menu", &["Menu"]),
];
// files: end

/// A game's files: its mount list, its archives (virtual folder, file under the game
/// directory, behind the folders), the texture file extensions in the order a name
/// present with both is taken.
pub struct Game {
    pub layout: &'static [(&'static str, &'static [&'static str])],
    pub archives: &'static [(&'static str, &'static str)],
    pub texture_exts: &'static [&'static str],
    /// Textures whose sides are not powers of two are drawn (Roots' DDS art has 768s);
    /// Armada II's are all powers of two, and one that is not is left out, as before.
    pub npot_textures: bool,
}

const ARMADA2: Game = Game { layout: LAYOUT, archives: &[], texture_exts: &[".tga"], npot_textures: false };

/// The games other than Armada II, by their executable's file name (any case).
pub const GAMES: &[(&str, Game)] = &[
    ("ArmadaL.exe", Game { layout: LAYOUT_FO_ROOTS, archives: &[("odf", "ODF.fpq")], texture_exts: &[".tga", ".dds"], npot_textures: true }),
];

/// This process's game: by the executable's name, Armada II's for any other.
pub fn game() -> &'static Game {
    let exe = rt::exe_name();
    GAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(exe)).map(|(_, g)| g).unwrap_or(&ARMADA2)
}

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
            let g = game();
            for (at, dirs) in g.layout {
                for d in *dirs {
                    v.mount(at, Box::new(Loose::new(IO, &rt::game_path(d))));
                }
            }
            for (at, file) in g.archives {
                let path = rt::game_path(file);
                match rt::read_file(&path).map(|d| Fpq::new(file, d)) {
                    Some(Ok(a)) => {
                        log!("files: {} mounted at {}: {} files", file, at, a.archive().entries().len());
                        v.mount(at, Box::new(a));
                    }
                    Some(Err(e)) => log!("files: {} does not read: {:?}", file, e),
                    None => log!("files: no {}", file),
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
