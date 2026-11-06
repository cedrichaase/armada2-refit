//! ODF: the object definition files (`odf/<category>/*.odf`).
//!
//! Plain text, `key = value` per line, `//` and `#` comments, and `#include "x.odf"`
//! to inherit another file's keys. A value may run on over the following lines when
//! they hold no `=` of their own (lists of quoted names). A few files carry
//! `[Section]` headers; Armada II looks keys up without them, so they are recorded on
//! each entry and otherwise ignored.
//!
//! Inheritance: a file's own keys win over anything it includes; includes are searched
//! in the order written, depth first. Within one file, when a key is written twice,
//! the later one wins. Names are matched ASCII case-insensitively, keys likewise.
//! Includes name a file anywhere under `odf/`, by file name alone.
//!
//! Fleet Operations layers its files (a mod's over its parent's over the game's), and an
//! include of the very file being loaded is its **next version down**
//! ([`Odf::load_layered`]): a mod's `x.odf` that includes `x.odf` inherits the parent's
//! `x.odf` and changes only what it writes. Each such include, in the order the tree is
//! read, takes the next version below the last one taken, as Fleet Operations' file
//! system does. Stock has no versions, so there such an include is a loop and is cut.

use alloc::string::String;
use alloc::vec::Vec;

use crate::text::{lines, parse_f32, parse_i32, strip_comment};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// As written.
    pub key: String,
    /// The raw value, comments removed, continuation lines joined with a space.
    pub value: String,
    /// The `[Section]` it sits under, empty when none.
    pub section: String,
    /// 1-based.
    pub line: usize,
}

/// One file, includes not followed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OdfFile {
    pub entries: Vec<Entry>,
    /// `#include` names in order, without quotes.
    pub includes: Vec<String>,
    /// Lines that were neither a key, a continuation, an include nor a section.
    pub stray_lines: Vec<usize>,
}

impl OdfFile {
    pub fn parse(data: &[u8]) -> OdfFile {
        let mut f = OdfFile::default();
        let mut section = String::new();
        for (i, raw) in lines(data).iter().enumerate() {
            let lno = i + 1;
            let t = raw.trim();
            // `#include` before comment stripping, since `#` also starts a comment.
            if let Some(rest) = t.strip_prefix('#') {
                let rest = rest.trim_start();
                if rest.get(..7).is_some_and(|w| w.eq_ignore_ascii_case("include")) {
                    let arg = strip_comment(&rest[7..], false).trim();
                    let name = arg.trim_matches('"').trim();
                    if !name.is_empty() {
                        f.includes.push(String::from(name));
                    } else {
                        f.stray_lines.push(lno);
                    }
                }
                continue;
            }
            let l = strip_comment(t, true).trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('[') && l.ends_with(']') {
                section = String::from(l[1..l.len() - 1].trim());
                continue;
            }
            match l.find('=') {
                Some(eq) if is_key(l[..eq].trim()) => f.entries.push(Entry {
                    key: String::from(l[..eq].trim()),
                    value: String::from(l[eq + 1..].trim()),
                    section: section.clone(),
                    line: lno,
                }),
                _ => match f.entries.last_mut() {
                    // A value continued on this line.
                    Some(e) => {
                        if !e.value.is_empty() {
                            e.value.push(' ');
                        }
                        e.value.push_str(l);
                    }
                    None => f.stray_lines.push(lno),
                },
            }
        }
        f
    }

    /// This file's own value for a key (the last one written).
    pub fn own(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.key.eq_ignore_ascii_case(key))
            .map(|e| e.value.as_str())
    }
}

fn is_key(k: &str) -> bool {
    !k.is_empty()
        && k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
        && !k.as_bytes()[0].is_ascii_digit()
}

/// Why [`Odf::load`] failed.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// The root file was not found.
    NotFound(String),
}

/// A file with its includes resolved: own entries first, then each include's tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Odf {
    pub name: String,
    pub file: OdfFile,
    pub parents: Vec<Odf>,
    /// Includes that could not be fetched, or that would have looped.
    pub missing: Vec<String>,
}

impl Odf {
    /// Loads `name` and, recursively, what it includes. `fetch` returns a file's bytes
    /// by name (`"fgalaxy.odf"`), looked up case-insensitively anywhere under `odf/`.
    /// An include that is missing or would loop is listed in `missing` and skipped,
    /// as the engine shrugs off a missing include.
    pub fn load<F>(name: &str, mut fetch: F) -> Result<Odf, Error>
    where
        F: FnMut(&str) -> Option<Vec<u8>>,
    {
        let mut stack: Vec<String> = Vec::new();
        let mut budget = MAX_INCLUDES;
        // A total of decoded bytes too: each include may be a big packed file, and a tree
        // of a thousand of them must not decode a thousand times over.
        let mut bytes = MAX_BYTES;
        let mut counted = |n: &str, v: usize| -> Option<Vec<u8>> {
            if v > 0 {
                return None;
            }
            let d = fetch(n)?;
            bytes = bytes.checked_sub(d.len())?;
            Some(d)
        };
        let mut w = Walk { root: String::new(), version: 0, stack: &mut stack, budget: &mut budget };
        load_rec(name, 0, &mut counted, &mut w).ok_or_else(|| Error::NotFound(String::from(name)))
    }

    /// As [`Odf::load`], over layered files: `fetch(name, v)` returns version `v` of a
    /// file (0 the top one, 1 the one below it, ...), and an include of the root file's
    /// own name takes the next version down (the module's doc).
    pub fn load_layered<F>(name: &str, mut fetch: F) -> Result<Odf, Error>
    where
        F: FnMut(&str, usize) -> Option<Vec<u8>>,
    {
        let mut stack: Vec<String> = Vec::new();
        let mut budget = MAX_INCLUDES;
        let mut bytes = MAX_BYTES;
        let mut counted = |n: &str, v: usize| -> Option<Vec<u8>> {
            let d = fetch(n, v)?;
            bytes = bytes.checked_sub(d.len())?;
            Some(d)
        };
        let mut w = Walk { root: name.to_ascii_lowercase(), version: 0, stack: &mut stack, budget: &mut budget };
        load_rec(name, 0, &mut counted, &mut w).ok_or_else(|| Error::NotFound(String::from(name)))
    }

    pub fn parse_single(name: &str, data: &[u8]) -> Odf {
        Odf { name: String::from(name), file: OdfFile::parse(data), parents: Vec::new(), missing: Vec::new() }
    }

    /// The raw value of a key, inherited through includes.
    pub fn get(&self, key: &str) -> Option<&str> {
        if let Some(v) = self.file.own(key) {
            return Some(v);
        }
        self.parents.iter().find_map(|p| p.get(key))
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// The first string of a value: the first quoted string, or else the first word.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(first_string)
    }

    /// Every string of a value: each quoted string, or each word when none is quoted.
    pub fn get_strs(&self, key: &str) -> Vec<&str> {
        self.get(key).map(strings).unwrap_or_default()
    }

    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|v| parse_f32(v.trim_matches('"')))
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(|v| parse_i32(v.trim_matches('"')))
    }

    /// Numbers in a value (`preferredPosition = 1 2 0 0`, colours, ...).
    pub fn get_f32s(&self, key: &str) -> Vec<f32> {
        self.get(key)
            .map(|v| {
                v.split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')' || c == '"')
                    .filter_map(parse_f32)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `true`/`false`, `yes`/`no`, or a number (non-zero is true).
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let v = self.get(key)?.trim().trim_matches('"').trim();
        if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes") {
            Some(true)
        } else if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("no") {
            Some(false)
        } else {
            parse_f32(v).map(|x| x != 0.0)
        }
    }

    /// Every key visible through inheritance, with its effective value, own keys first.
    pub fn all(&self) -> Vec<(&str, &str)> {
        let mut out: Vec<(&str, &str)> = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect<'a>(&'a self, out: &mut Vec<(&'a str, &'a str)>) {
        for e in self.file.entries.iter().rev() {
            if !out.iter().any(|(k, _)| k.eq_ignore_ascii_case(&e.key)) {
                out.push((e.key.as_str(), e.value.as_str()));
            }
        }
        for p in &self.parents {
            p.collect(out);
        }
    }

    /// `classLabel`, unquoted and lower case: what kind of object this is.
    pub fn class_label(&self) -> Option<String> {
        self.get_str("classLabel").map(|s| s.to_ascii_lowercase())
    }
}

/// Most files one [`Odf::load`] opens through includes (stock trees are a few dozen).
const MAX_INCLUDES: u32 = 1024;
/// Most bytes the files of one [`Odf::load`] may total, as `fetch` returns them.
const MAX_BYTES: usize = 256 << 20;

/// What a load carries down the tree.
struct Walk<'a> {
    /// The root file's name, lower case, when its own name reaches its next version
    /// down (`load_layered`); empty otherwise.
    root: String,
    /// The last version of the root taken.
    version: usize,
    stack: &'a mut Vec<String>,
    budget: &'a mut u32,
}

fn load_rec(name: &str, version: usize, fetch: &mut dyn FnMut(&str, usize) -> Option<Vec<u8>>, w: &mut Walk) -> Option<Odf> {
    let data = fetch(name, version)?;
    let file = OdfFile::parse(&data);
    w.stack.push(name.to_ascii_lowercase());
    let mut parents = Vec::new();
    let mut missing = Vec::new();
    for inc in &file.includes {
        let lower = inc.to_ascii_lowercase();
        // The root's own name: its next version down, if there is one.
        let next = !w.root.is_empty() && lower == w.root;
        if (!next && w.stack.contains(&lower)) || w.stack.len() > 32 {
            missing.push(inc.clone());
            continue;
        }
        // Shared parents (a diamond) are legitimate, so this is a total budget rather than
        // once-per-file; it stops a chain of files each including the next twice.
        if *w.budget == 0 {
            missing.push(inc.clone());
            continue;
        }
        *w.budget -= 1;
        let v = if next {
            w.version += 1;
            w.version
        } else {
            0
        };
        match load_rec(inc, v, fetch, w) {
            Some(p) => parents.push(p),
            None => missing.push(inc.clone()),
        }
    }
    w.stack.pop();
    Some(Odf { name: String::from(name), file, parents, missing })
}

/// The first quoted string of a value, or its first word.
pub fn first_string(v: &str) -> Option<&str> {
    let v = v.trim();
    if let Some(rest) = v.strip_prefix('"') {
        return Some(match rest.find('"') {
            Some(e) => &rest[..e],
            None => rest,
        });
    }
    v.split_whitespace().next()
}

/// Each quoted string of a value, or each word when nothing is quoted.
pub fn strings(v: &str) -> Vec<&str> {
    if v.contains('"') {
        let mut out = Vec::new();
        let mut rest = v;
        while let Some(s) = rest.find('"') {
            let after = &rest[s + 1..];
            match after.find('"') {
                Some(e) => {
                    out.push(&after[..e]);
                    rest = &after[e + 1..];
                }
                None => {
                    out.push(after);
                    break;
                }
            }
        }
        out
    } else {
        v.split_whitespace().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inheritance() {
        let base = b"[GameObjectClass]\r\nclassLabel = \"craft\"\r\nmaxHealth = 100\r\nscaleSOD=2.5 // c\r\n";
        let ship = b"#include \"base.odf\"\r\n# a note\r\nmaxHealth = 720\r\nnames = \r\n\t\"USS A\" \"USS B\"\r\n\t\"USS C\"\r\nflag = 1\r\npos = 1\t2 0 0\r\n";
        let odf = Odf::load("ship.odf", |n| match n {
            "ship.odf" => Some(ship.to_vec()),
            "base.odf" => Some(base.to_vec()),
            _ => None,
        })
        .unwrap();
        assert_eq!(odf.get_i32("MAXHEALTH"), Some(720));
        assert_eq!(odf.class_label().as_deref(), Some("craft"));
        assert_eq!(odf.get_f32("scaleSOD"), Some(2.5));
        assert_eq!(odf.get_strs("names"), ["USS A", "USS B", "USS C"]);
        assert_eq!(odf.get_bool("flag"), Some(true));
        assert_eq!(odf.get_f32s("pos"), [1.0, 2.0, 0.0, 0.0]);
        assert_eq!(odf.parents[0].file.entries[0].section, "GameObjectClass");
        assert!(odf.file.stray_lines.is_empty());
    }

    #[test]
    fn own_name_is_the_next_version_down() {
        // A mod's ship.odf over the game's: it includes its own name, so it inherits the
        // game's ship.odf; the game's includes base.odf as usual.
        let versions: [&[u8]; 2] = [
            b"#include \"Ship.odf\"\nmaxHealth = 900\n",
            b"#include \"base.odf\"\nmaxHealth = 720\nrace = \"borg\"\n",
        ];
        let odf = Odf::load_layered("ship.odf", |n, v| match n.to_ascii_lowercase().as_str() {
            "ship.odf" => versions.get(v).map(|d| d.to_vec()),
            "base.odf" if v == 0 => Some(b"classLabel = \"craft\"\n".to_vec()),
            _ => None,
        })
        .unwrap();
        assert_eq!(odf.get_i32("maxHealth"), Some(900));
        assert_eq!(odf.get_str("race"), Some("borg"));
        assert_eq!(odf.class_label().as_deref(), Some("craft"));
        assert!(odf.missing.is_empty());
        // The bottom version including itself again: no version below, so it is missing.
        let odf = Odf::load_layered("a.odf", |n, v| (n == "a.odf" && v == 0).then(|| b"#include \"a.odf\"\nx = 1\n".to_vec())).unwrap();
        assert_eq!(odf.get_i32("x"), Some(1));
        assert_eq!(odf.missing, ["a.odf"]);
        // Unlayered, the same include is a loop and is cut.
        let odf = Odf::load("a.odf", |n| (n == "a.odf").then(|| b"#include \"a.odf\"\nx = 1\n".to_vec())).unwrap();
        assert_eq!(odf.missing, ["a.odf"]);
    }

    #[test]
    fn include_loop_is_cut() {
        let odf = Odf::load("a.odf", |n| match n {
            "a.odf" => Some(b"#include \"b.odf\"\nx = 1\n".to_vec()),
            "b.odf" => Some(b"#include \"A.ODF\"\ny = 2\n".to_vec()),
            _ => None,
        })
        .unwrap();
        assert_eq!(odf.get_i32("y"), Some(2));
        assert_eq!(odf.parents[0].missing, ["A.ODF"]);
    }
}
