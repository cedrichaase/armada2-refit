//! One file system over a game's files: the place each game's file layout is data.
//!
//! A [`Vfs`] opens, tests and lists files by a path relative to its own root. Paths are
//! case-insensitive (stock mixes `.tga` and `.TGA`, `SOD` and `sod`) and take either
//! slash; empty and `.` components are skipped and a path with a `..` component is
//! refused (it names nothing).
//!
//! Sources:
//!
//! - [`Loose`]: a folder of loose files, read through an [`Io`] the caller supplies (this
//!   crate makes no system calls: the plugins hand in `a2rt`'s Win32 reads, the host
//!   tests `std::fs`).
//! - [`Fpq`] (Fleet Operations' `ODF.fpq`, read by [`crate::fpq`]): read-only, held in
//!   memory, mounted at `odf`.
//! - [`Zfs`] (Armada's `.zfs`): declared, with its place in the mount order, and serving
//!   nothing yet; its [`Vfs::status`] says so in a line fit for a log (`PLAN-ARMADA1.md`,
//!   A1-11).
//! - Planned, not written: an `Engine` source that reads through the game's own file
//!   functions, for an archive whose format is not ours to read. It would mount last, as
//!   the fallback behind every source of ours.
//!
//! [`Layered`] mounts sources at virtual folders (`textures`, `odf`, or `""` for the
//! root) and searches them in [`Kind`] order, then in the order they were mounted: the
//! first source that has a file serves it.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::fpq;

/// One entry of a directory listing: its name as the source spells it, and whether it
/// is a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
}

/// Whether a source serves anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    /// Mounted, serving nothing; the reason, for the log.
    Unsupported(&'static str),
}

/// The kinds of source, in their mount order: loose files win over archives, so a file
/// put in a folder replaces the packed one, as the game folder's recipe wins over a
/// built-in one. (Our order; whether each engine prefers loose files the same way is for
/// FO-5 and A1-11 to measure before either archive is read.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Loose,
    /// Fleet Operations: Roots' archive (`ODF.fpq`).
    Fpq,
    /// Armada's archives (`*.zfs`).
    Zfs,
}

/// A file system: paths relative to its root, case-insensitive, either slash.
pub trait Vfs {
    /// The whole file, or None when there is none.
    fn open(&self, path: &str) -> Option<Vec<u8>>;
    /// Whether a file (not a folder) is there.
    fn exists(&self, path: &str) -> bool;
    /// A folder's entries in the source's own order; empty for a folder that is not
    /// there.
    fn list(&self, dir: &str) -> Vec<Entry>;
    fn kind(&self) -> Kind {
        Kind::Loose
    }
    fn status(&self) -> Status {
        Status::Ready
    }
    /// What it is, for the log (`loose C:\...\Textures\RGB`).
    fn label(&self) -> String;
}

/// The system calls a [`Loose`] source needs, on native paths it builds from its root
/// and [`Io::separator`].
pub trait Io {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn is_file(&self, path: &str) -> bool;
    /// A folder's entries, without `.` and `..`; empty when it is not there.
    fn list(&self, dir: &str) -> Vec<Entry>;
    fn separator(&self) -> char {
        '/'
    }
    /// True where the file system itself ignores case (Windows, and Wine over it): paths
    /// go to it as written, with no listing to match them.
    fn case_insensitive(&self) -> bool {
        false
    }
}

/// An [`Io`] made of plain functions, for a caller that has them already (`a2rt`'s
/// `read_file`, `file_exists`, and a listing over `list_entries`).
#[derive(Clone, Copy)]
pub struct FnIo {
    pub read: fn(&str) -> Option<Vec<u8>>,
    pub is_file: fn(&str) -> bool,
    pub list: fn(&str) -> Vec<Entry>,
    pub separator: char,
    pub case_insensitive: bool,
}

impl Io for FnIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        (self.read)(path)
    }
    fn is_file(&self, path: &str) -> bool {
        (self.is_file)(path)
    }
    fn list(&self, dir: &str) -> Vec<Entry> {
        (self.list)(dir)
    }
    fn separator(&self) -> char {
        self.separator
    }
    fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }
}

/// A path's components: either slash, empty and `.` components skipped; None for a path
/// with a `..`.
pub fn components(path: &str) -> Option<Vec<&str>> {
    let mut out = Vec::new();
    for c in path.split(['/', '\\']) {
        match c {
            "" | "." => {}
            ".." => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

fn join(parts: &[&str], sep: char) -> String {
    let mut s = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            s.push(sep);
        }
        s.push_str(p);
    }
    s
}

/// A folder of loose files under `root` (a native path; either slash, made the
/// [`Io`]'s). Where the file system minds case, a path that does not open as written is
/// matched against the folders' listings, each listed once and kept (so a file added
/// later is found only as written).
pub struct Loose<I: Io> {
    io: I,
    root: String,
    dirs: RefCell<BTreeMap<String, Vec<Entry>>>,
}

impl<I: Io> Loose<I> {
    pub fn new(io: I, root: &str) -> Loose<I> {
        let sep = io.separator();
        let root: String = root.chars().map(|c| if c == '/' || c == '\\' { sep } else { c }).collect();
        Loose { io, root, dirs: RefCell::new(BTreeMap::new()) }
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    /// The native path of components under the root.
    fn native(&self, parts: &[&str]) -> String {
        let sep = self.io.separator();
        let mut s = self.root.clone();
        if !parts.is_empty() && !s.is_empty() && !s.ends_with(sep) {
            s.push(sep);
        }
        s.push_str(&join(parts, sep));
        s
    }

    /// The components as the file system spells them, matched case-insensitively
    /// through the listings (an exact match first); None when one is not there.
    fn resolve(&self, parts: &[&str], last_dir: bool) -> Option<Vec<String>> {
        let mut real: Vec<String> = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            let want_dir = i + 1 < parts.len() || last_dir;
            let (key, native) = {
                let here: Vec<&str> = real.iter().map(|s| s.as_str()).collect();
                (join(&here, '/'), self.native(&here))
            };
            let name = {
                let mut dirs = self.dirs.borrow_mut();
                let entries = dirs.entry(key).or_insert_with(|| self.io.list(&native));
                let hit = entries.iter().find(|e| e.dir == want_dir && e.name == *p)
                    .or_else(|| entries.iter().find(|e| e.dir == want_dir && e.name.eq_ignore_ascii_case(p)))?;
                hit.name.clone()
            };
            real.push(name);
        }
        Some(real)
    }
}

impl<I: Io> Vfs for Loose<I> {
    fn open(&self, path: &str) -> Option<Vec<u8>> {
        let parts = components(path)?;
        if parts.is_empty() {
            return None;
        }
        if let Some(d) = self.io.read(&self.native(&parts)) {
            return Some(d);
        }
        if self.io.case_insensitive() {
            return None;
        }
        let real = self.resolve(&parts, false)?;
        let real: Vec<&str> = real.iter().map(|s| s.as_str()).collect();
        if real == parts { None } else { self.io.read(&self.native(&real)) }
    }

    fn exists(&self, path: &str) -> bool {
        let Some(parts) = components(path) else { return false };
        if parts.is_empty() {
            return false;
        }
        if self.io.is_file(&self.native(&parts)) {
            return true;
        }
        !self.io.case_insensitive() && self.resolve(&parts, false).is_some()
    }

    fn list(&self, dir: &str) -> Vec<Entry> {
        let Some(parts) = components(dir) else { return Vec::new() };
        if self.io.case_insensitive() || parts.is_empty() {
            return self.io.list(&self.native(&parts));
        }
        match self.resolve(&parts, true) {
            Some(real) => {
                let real: Vec<&str> = real.iter().map(|s| s.as_str()).collect();
                self.io.list(&self.native(&real))
            }
            None => Vec::new(),
        }
    }

    fn label(&self) -> String {
        let mut s = String::from("loose ");
        s.push_str(&self.root);
        s
    }
}

macro_rules! archive {
    ($name:ident, $kind:expr, $what:literal, $why:literal) => {
        #[doc = $what]
        ///
        /// Mounted in its place, serving nothing: the reader is not written, and this crate
        /// guesses at no format.
        pub struct $name {
            file: String,
        }

        impl $name {
            /// The archive at a native path (or one under the game folder, for the log).
            pub fn new(file: &str) -> $name {
                $name { file: String::from(file) }
            }
        }

        impl Vfs for $name {
            fn open(&self, _: &str) -> Option<Vec<u8>> {
                None
            }
            fn exists(&self, _: &str) -> bool {
                false
            }
            fn list(&self, _: &str) -> Vec<Entry> {
                Vec::new()
            }
            fn kind(&self) -> Kind {
                $kind
            }
            fn status(&self) -> Status {
                Status::Unsupported($why)
            }
            fn label(&self) -> String {
                let mut s = String::from(stringify!($name));
                s.make_ascii_lowercase();
                s.push(' ');
                s.push_str(&self.file);
                s
            }
        }
    };
}

/// Fleet Operations: Roots' archive (`ODF.fpq`), read by [`crate::fpq`]. The archive's
/// single top folder (`odf`) is the mount's own name, so it is left out of the paths a
/// mounted `Fpq` serves (`system/x.odf` at `odf`); a path with the top folder, or with
/// the file's name alone, is found as well, since the archive keys its files by name.
pub struct Fpq {
    file: String,
    archive: fpq::Archive,
    /// The one folder with no parent, when there is exactly one.
    root: Option<String>,
}

impl Fpq {
    /// The archive in `data`; `file` is its path, for the log.
    pub fn new(file: &str, data: Vec<u8>) -> Result<Fpq, fpq::Error> {
        let archive = fpq::Archive::parse(data)?;
        let mut tops = archive.folders().iter().filter(|f| f.parent.is_none());
        let root = match (tops.next(), tops.next()) {
            (Some(f), None) => Some(f.name.clone()),
            _ => None,
        };
        Ok(Fpq { file: String::from(file), archive, root })
    }

    pub fn archive(&self) -> &fpq::Archive {
        &self.archive
    }

    /// A file's folders as the mounted view has them (the top folder left out).
    fn view_chain(&self, e: &fpq::Entry) -> Vec<&str> {
        let mut c = e.folder.map(|f| self.archive.folder_names(f)).unwrap_or_default();
        if self.root.is_some() && !c.is_empty() {
            c.remove(0);
        }
        c
    }
}

impl Vfs for Fpq {
    fn open(&self, path: &str) -> Option<Vec<u8>> {
        self.archive.open(path)
    }

    fn exists(&self, path: &str) -> bool {
        self.archive.find(path).is_some()
    }

    fn list(&self, dir: &str) -> Vec<Entry> {
        let Some(mut want) = components(dir) else { return Vec::new() };
        if let (Some(root), Some(first)) = (&self.root, want.first()) {
            if first.eq_ignore_ascii_case(root) {
                want.remove(0);
            }
        }
        let mut out: Vec<Entry> = Vec::new();
        for e in self.archive.entries() {
            let chain = self.view_chain(e);
            if chain.len() < want.len() || !chain.iter().zip(&want).all(|(a, b)| a.eq_ignore_ascii_case(b)) {
                continue;
            }
            let entry = match chain.get(want.len()) {
                Some(sub) => Entry { name: String::from(*sub), dir: true },
                None => Entry { name: e.name.clone(), dir: false },
            };
            if !out.iter().any(|o| o.dir == entry.dir && o.name.eq_ignore_ascii_case(&entry.name)) {
                out.push(entry);
            }
        }
        out
    }

    fn kind(&self) -> Kind {
        Kind::Fpq
    }

    fn label(&self) -> String {
        let mut s = String::from("fpq ");
        s.push_str(&self.file);
        s
    }
}

archive!(Zfs, Kind::Zfs, "An Armada `.zfs` archive.",
    "no reader for Armada's .zfs archives yet (PLAN-ARMADA1.md, A1-11): its files are not served");

struct Mount {
    at: Vec<String>,
    src: Box<dyn Vfs>,
}

/// Sources mounted at virtual folders. A path is served by the mounts at the deepest
/// folder that holds it (`textures/a.tga` by those at `textures`, not by a root mount's
/// own `textures` folder), searched in [`Kind`] order and then in the order they were
/// mounted.
#[derive(Default)]
pub struct Layered {
    mounts: Vec<Mount>,
}

/// The rest of `parts` under `at`, compared without regard to case.
fn under<'p, 's>(at: &[String], parts: &'p [&'s str]) -> Option<&'p [&'s str]> {
    if parts.len() < at.len() || !at.iter().zip(parts).all(|(a, p)| a.eq_ignore_ascii_case(p)) {
        return None;
    }
    Some(&parts[at.len()..])
}

impl Layered {
    pub fn new() -> Layered {
        Layered { mounts: Vec::new() }
    }

    /// Mounts a source at a virtual folder (`""` for the root): after every source of
    /// its kind or an earlier one, before any of a later kind. A folder that is not a
    /// path (`..`) mounts nothing.
    pub fn mount(&mut self, at: &str, src: Box<dyn Vfs>) {
        let Some(parts) = components(at) else { return };
        let at = parts.into_iter().map(String::from).collect();
        let kind = src.kind();
        let i = self.mounts.iter().position(|m| m.src.kind() > kind).unwrap_or(self.mounts.len());
        self.mounts.insert(i, Mount { at, src });
    }

    /// Every mount in search order: its folder (`/`-joined), the source's label and
    /// status, for the log.
    pub fn mounts(&self) -> Vec<(String, String, Status)> {
        self.mounts.iter().map(|m| {
            let at: Vec<&str> = m.at.iter().map(|s| s.as_str()).collect();
            (join(&at, '/'), m.src.label(), m.src.status())
        }).collect()
    }
}

impl Layered {
    /// The mounts that serve a path: those at the deepest mount folder holding it (a
    /// mount shadows its folder in a mount above it, as a mounted file system does),
    /// each with the rest of the path.
    fn serving<'m, 'p, 's>(&'m self, parts: &'p [&'s str]) -> impl Iterator<Item = (&'m Mount, String)> + use<'m, 'p, 's> {
        let depth = self.mounts.iter().filter(|m| under(&m.at, parts).is_some()).map(|m| m.at.len()).max();
        self.mounts.iter().filter_map(move |m| {
            let rest = under(&m.at, parts)?;
            (Some(m.at.len()) == depth).then(|| (m, join(rest, '/')))
        })
    }
}

impl Vfs for Layered {
    fn open(&self, path: &str) -> Option<Vec<u8>> {
        let parts = components(path)?;
        self.serving(&parts).find_map(|(m, rest)| m.src.open(&rest))
    }

    fn exists(&self, path: &str) -> bool {
        let Some(parts) = components(path) else { return false };
        self.serving(&parts).any(|(m, rest)| m.src.exists(&rest))
    }

    /// The entries of every source serving the folder, the first of each name (in any
    /// case) kept, and the mount folders inside it as folders.
    fn list(&self, dir: &str) -> Vec<Entry> {
        let Some(parts) = components(dir) else { return Vec::new() };
        let mut out: Vec<Entry> = Vec::new();
        let add = |e: Entry, out: &mut Vec<Entry>| {
            if !out.iter().any(|o| o.name.eq_ignore_ascii_case(&e.name)) {
                out.push(e);
            }
        };
        for m in &self.mounts {
            if m.at.len() > parts.len() && m.at.iter().zip(&parts).all(|(a, p)| a.eq_ignore_ascii_case(p)) {
                add(Entry { name: m.at[parts.len()].clone(), dir: true }, &mut out);
            }
        }
        for (m, rest) in self.serving(&parts) {
            for e in m.src.list(&rest) {
                add(e, &mut out);
            }
        }
        out
    }

    fn label(&self) -> String {
        String::from("layered")
    }
}
