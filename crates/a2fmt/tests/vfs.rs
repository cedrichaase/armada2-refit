//! `a2fmt::vfs` on a temporary directory: case-insensitive lookups over a file system
//! that minds case, layering order, missing files, and the declared archive kinds.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use a2fmt::vfs::{Entry, Io, Kind, Layered, Loose, Status, Vfs, Zfs, components};

/// `std::fs`, case-sensitive (Linux), as the host has it.
#[derive(Clone, Copy)]
struct StdIo;

impl Io for StdIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(path).ok()
    }
    fn is_file(&self, path: &str) -> bool {
        fs::metadata(path).map(|m| m.is_file()).unwrap_or(false)
    }
    fn list(&self, dir: &str) -> Vec<Entry> {
        let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
        let mut v: Vec<Entry> = rd
            .filter_map(|e| e.ok())
            .map(|e| Entry { name: e.file_name().to_string_lossy().into_owned(), dir: e.path().is_dir() })
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

/// A fresh directory under the system's temp folder, removed on drop.
struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Tmp {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "a2fmt-vfs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn put(&self, rel: &str, data: &str) {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
    fn path(&self, rel: &str) -> String {
        self.0.join(rel).to_string_lossy().into_owned()
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(v: &dyn Vfs, path: &str) -> Option<String> {
    v.open(path).map(|b| String::from_utf8(b).unwrap())
}

#[test]
fn components_take_both_slashes_and_refuse_dotdot() {
    assert_eq!(components("Textures\\RGB/a.tga").unwrap(), ["Textures", "RGB", "a.tga"]);
    assert_eq!(components("/./sod//x.sod/").unwrap(), ["sod", "x.sod"]);
    assert!(components("odf/../secret").is_none());
    assert!(components("").unwrap().is_empty());
}

#[test]
fn loose_is_case_insensitive_on_a_case_sensitive_disk() {
    let t = Tmp::new();
    t.put("Textures/RGB/Mnebula4.TGA", "upper");
    t.put("Textures/RGB/fgalaxy.tga", "lower");
    t.put("SOD/sub/Kbattle.sod", "sod");
    let v = Loose::new(StdIo, &t.path(""));
    // As written, and in every other case and slash.
    assert_eq!(text(&v, "Textures/RGB/Mnebula4.TGA").as_deref(), Some("upper"));
    assert_eq!(text(&v, "textures\\rgb\\mnebula4.tga").as_deref(), Some("upper"));
    assert_eq!(text(&v, "TEXTURES/RGB/FGALAXY.TGA").as_deref(), Some("lower"));
    assert_eq!(text(&v, "sod/SUB/kbattle.SOD").as_deref(), Some("sod"));
    assert!(v.exists("textures/rgb/MNEBULA4.tga"));
    assert!(!v.exists("textures/rgb"), "a folder is not a file");
    // A listing names each entry as the disk spells it.
    let names: Vec<_> = v.list("TEXTURES\\rgb").into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["Mnebula4.TGA", "fgalaxy.tga"]);
    assert_eq!(v.list("sod"), [Entry { name: "sub".into(), dir: true }]);
}

#[test]
fn loose_missing_files_and_bad_paths() {
    let t = Tmp::new();
    t.put("odf/a.odf", "a");
    let v = Loose::new(StdIo, &t.path(""));
    assert!(v.open("odf/b.odf").is_none());
    assert!(v.open("nowhere/a.odf").is_none());
    assert!(!v.exists("odf/b.odf"));
    assert!(v.list("nowhere").is_empty());
    assert!(v.open("odf/../odf/a.odf").is_none(), "`..` names nothing");
    assert!(v.open("").is_none());
    assert_eq!(text(&v, "./odf/a.odf").as_deref(), Some("a"));
}

#[test]
fn layered_searches_in_mount_order() {
    let t = Tmp::new();
    t.put("game/Textures/RGB/a.tga", "rgb-a");
    t.put("game/Textures/RGB/b.tga", "rgb-b");
    t.put("game/Textures/c.tga", "flat-c");
    t.put("game/Textures/A.TGA", "flat-a");
    t.put("game/Sky/aqua.ini", "sky");
    t.put("game/Credits.txt", "credits");
    t.put("game/Other/z.tga", "z");
    let mut v = Layered::new();
    v.mount("textures", Box::new(Loose::new(StdIo, &t.path("game/Textures/RGB"))));
    v.mount("textures", Box::new(Loose::new(StdIo, &t.path("game/Textures"))));
    v.mount("sky", Box::new(Loose::new(StdIo, &t.path("game/Sky"))));
    v.mount("", Box::new(Loose::new(StdIo, &t.path("game"))));
    // The first root that has the file wins, in any case.
    assert_eq!(text(&v, "textures/a.tga").as_deref(), Some("rgb-a"));
    assert_eq!(text(&v, "Textures\\A.tga").as_deref(), Some("rgb-a"));
    assert_eq!(text(&v, "textures/c.tga").as_deref(), Some("flat-c"));
    assert_eq!(text(&v, "TEXTURES/b.tga").as_deref(), Some("rgb-b"));
    assert!(v.open("textures/d.tga").is_none());
    assert!(!v.exists("textures/d.tga"));
    assert!(v.exists("textures/C.TGA"));
    // An override folder, and the root behind everything.
    assert_eq!(text(&v, "Sky\\Aqua.ini").as_deref(), Some("sky"));
    assert_eq!(text(&v, "credits.txt").as_deref(), Some("credits"));
    // A mount shadows the folder of its name in the root: the game's own Textures/
    // is reached only through the textures roots (here the second one).
    assert_eq!(text(&v, "Textures/RGB/b.tga").as_deref(), Some("rgb-b"));
    assert_eq!(text(&v, "other/Z.TGA").as_deref(), Some("z"));
    // A listing merges the roots, the first of each name kept, in mount order.
    let l: Vec<_> = v.list("textures").into_iter().map(|e| e.name).collect();
    assert_eq!(l, ["a.tga", "b.tga", "RGB", "c.tga"]);
    // The root lists its mount folders as folders, besides the game folder's own.
    let root: Vec<_> = v.list("").into_iter().map(|e| (e.name, e.dir)).collect();
    assert!(root.contains(&("textures".into(), true)));
    assert_eq!(root.iter().filter(|e| e.0.eq_ignore_ascii_case("textures")).count(), 1);
    assert!(root.contains(&("Credits.txt".into(), false)));
    assert_eq!(root.iter().filter(|e| e.0.eq_ignore_ascii_case("sky")).count(), 1);
}

#[test]
fn a_mount_shadows_the_root() {
    let t = Tmp::new();
    t.put("game/Textures/RGB/a.tga", "rgb");
    t.put("game/Textures/stray.tga", "stray");
    let mut v = Layered::new();
    v.mount("", Box::new(Loose::new(StdIo, &t.path("game"))));
    v.mount("textures", Box::new(Loose::new(StdIo, &t.path("game/Textures/RGB"))));
    assert_eq!(text(&v, "textures/a.tga").as_deref(), Some("rgb"));
    // Textures/stray.tga is not under the textures root, so it is not a texture.
    assert!(v.open("textures/stray.tga").is_none());
    assert!(!v.exists("textures/stray.tga"));
    let l: Vec<_> = v.list("textures").into_iter().map(|e| e.name).collect();
    assert_eq!(l, ["a.tga"]);
}

#[test]
fn zfs_is_declared_and_serves_nothing() {
    let t = Tmp::new();
    t.put("game/odf/loose.odf", "loose");
    let mut v = Layered::new();
    // Mounted first, an archive still searches after the loose folders.
    v.mount("", Box::new(Zfs::new("data.zfs")));
    v.mount("odf", Box::new(Loose::new(StdIo, &t.path("game/odf"))));
    assert_eq!(text(&v, "odf/loose.odf").as_deref(), Some("loose"));
    assert!(v.open("odf/packed.odf").is_none());
    let m = v.mounts();
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].2, Status::Ready);
    assert!(m[0].1.starts_with("loose "));
    assert_eq!(m[1].1, "zfs data.zfs");
    match &m[1].2 {
        Status::Unsupported(why) => assert!(why.contains("no reader")),
        Status::Ready => panic!("Zfs reads nothing yet"),
    }
    assert!(Kind::Loose < Kind::Fpq && Kind::Fpq < Kind::Zfs);
}
