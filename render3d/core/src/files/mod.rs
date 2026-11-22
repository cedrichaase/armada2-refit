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
//!
//! A game with mods ([`Game::mods`], Roots) mounts the whole list once per folder of the
//! active mod's chain ([`mods`]), each a level of the file system above the one under it:
//! the active mod's `Textures\`, `SOD\`, `odf\`, `ODF.fpq` and so on over its parent's
//! over the game's own, as the game's file system layers them. `data\` is the folder
//! itself (`data\RTS_CFG.h`), mod over game alike.
use crate::{log, rt};
pub use a2fmt::vfs::Entry;
use a2fmt::vfs::{FnIo, Fpq, Layered, Loose, Status, Vfs};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

pub mod mods;

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
    ("data", &[""]),
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
    ("data", &[""]),
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
    /// The game's mods (`games/<id>.toml` `[files] mods`): None for a game without.
    pub mods: Option<mods::Rules>,
}

const ARMADA2: Game = Game { layout: LAYOUT, archives: &[], texture_exts: &[".tga"], npot_textures: false, mods: None };

// files: mods fo-roots (games/fo-roots.toml [files] mods)
const MODS_FO_ROOTS: mods::Rules = mods::Rules { dir: "Mods", switches: &["-mod", "/mod"] };
// files: end

/// The games other than Armada II, by their executable's file name (any case).
pub const GAMES: &[(&str, Game)] = &[
    (
        "ArmadaL.exe",
        Game { layout: LAYOUT_FO_ROOTS, archives: &[("odf", "ODF.fpq")], texture_exts: &[".tga", ".dds"], npot_textures: true, mods: Some(MODS_FO_ROOTS) },
    ),
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

/// The process's command line (`GetCommandLineW`, found by export: the plugins import
/// nothing for it); empty if it cannot be had.
fn command_line() -> String {
    let f = rt::find_export("GetCommandLineW");
    if f == 0 {
        return String::new();
    }
    let get: extern "system" fn() -> *const u16 = unsafe { core::mem::transmute(f) };
    let p = get();
    if p.is_null() {
        return String::new();
    }
    let mut n = 0;
    while n < 32768 && unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    rt::from_wide(unsafe { core::slice::from_raw_parts(p, n) })
}

/// The folders the files come from, top first: (folder under the game directory with a
/// trailing backslash, `""` for the game's own; the mod's name, for the log). The active
/// mod's chain, then the game's own folder unless the chain is standalone.
fn roots(g: &Game) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let Some(rules) = &g.mods else {
        out.push((String::new(), None));
        return out;
    };
    let folder = |m: &str| {
        let mut f = String::from(rules.dir);
        f.push('\\');
        f.push_str(m);
        f.push('\\');
        f
    };
    let Some(name) = mods::active(&command_line(), rules.switches) else {
        log!("files: no active mod (no {} on the command line): the game's own folder alone", rules.switches.join(" or "));
        out.push((String::new(), None));
        return out;
    };
    let c = mods::chain(&name, |m| {
        let mut p = folder(m);
        p.push_str("info.ini");
        rt::read_file(&rt::game_path(&p))
    });
    if c.mods.is_empty() {
        log!("files: the command line names mod '{}', which is no mod under {}\\: the game's own folder alone", name, rules.dir);
    } else {
        log!(
            "files: active mod '{}': {}{}{}",
            name,
            c.mods.join(", then "),
            if c.base { ", then the game's own folder" } else { " (standalone: the game's own folder is not searched)" },
            if let Some(m) = &c.missing { crate::sfmt!("; its parent '{}' is no mod, the chain stops there", m) } else { String::new() }
        );
    }
    for m in &c.mods {
        out.push((folder(m), Some(m.clone())));
    }
    if c.base {
        out.push((String::new(), None));
    }
    out
}

/// The file system, built at the first call.
#[allow(static_mut_refs)]
pub fn vfs() -> &'static Layered {
    unsafe {
        if VFS.is_none() {
            let mut v = Layered::new();
            let g = game();
            let roots = roots(g);
            // The bottom folder is level 0, the active mod's the highest.
            for (level, (root, m)) in roots.iter().rev().enumerate() {
                let level = level as u32;
                for (at, dirs) in g.layout {
                    for d in *dirs {
                        let mut p = root.clone();
                        p.push_str(d);
                        v.mount_level(at, level, Box::new(Loose::new(IO, &rt::game_path(&p))));
                    }
                }
                for (at, file) in g.archives {
                    let mut rel = root.clone();
                    rel.push_str(file);
                    let path = rt::game_path(&rel);
                    match rt::read_file(&path).map(|d| Fpq::new(&rel, d)) {
                        Some(Ok(a)) => {
                            log!("files: {} mounted at {}: {} files", rel, at, a.archive().entries().len());
                            v.mount_level(at, level, Box::new(a));
                        }
                        Some(Err(e)) => log!("files: {} does not read: {:?}", rel, e),
                        // A mod need not pack its files.
                        None if m.is_some() => {}
                        None => log!("files: no {}", rel),
                    }
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

/// The sources serving a virtual folder, top first, each with the folder's path in it:
/// for an index that finds a name in the top source that has it, wherever it sits.
pub fn sources(dir: &str) -> Vec<(&'static dyn Vfs, String)> {
    vfs().sources(dir)
}

/// The source that serves a file, for the log (`loose C:\...\Mods\X\`), or None.
pub fn served_by(path: &str) -> Option<String> {
    let (dir, name) = path.rsplit_once(['\\', '/']).unwrap_or(("", path));
    vfs().sources(dir).into_iter().find(|(s, rest)| s.exists(&join(rest, name))).map(|(s, _)| s.label())
}

/// `rest\name`, or `name` when `rest` is empty.
pub fn join(rest: &str, name: &str) -> String {
    let mut p = String::from(rest);
    if !p.is_empty() {
        p.push('\\');
    }
    p.push_str(name);
    p
}
