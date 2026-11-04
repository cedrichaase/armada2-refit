//! Ini text, read and edited: the one parser of the plugins' `a2rt::Ini` and the one
//! writer of the settings overlay, `RefitPlayer.ini` (`a2rt::overlay`). `no_std` with
//! `alloc` and no Win32, so the host tests it: `tests/fixtures.rs` runs `edit` on every
//! fixture in `tools/settings/fixtures/` (and `rename` on `fixtures/rename/`). The `a2ini`
//! binary (`src/main.rs`, feature `std`) is this code for the bench, the GUI installer and
//! `tools/settings/move-keys.py`.
//! The fixtures are the spec of `edit` (tools/settings/README.md, "The writer").
#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// An ini file parsed as `GetPrivateProfileString` reads one: names without case, a
/// section written twice read as one, the first of a repeated key winning, surrounding
/// whitespace and one pair of quotes dropped, `;` lines skipped.
#[derive(Default)]
pub struct Parsed {
    /// Section names in file order (as written, each once).
    pub sections: Vec<String>,
    /// (section, key, value), the names lower-cased.
    pub entries: Vec<(String, String, String)>,
}

impl Parsed {
    pub fn clear(&mut self) {
        self.sections.clear();
        self.entries.clear();
    }
    /// The value of `key` in `[sec]` as written (a trailing `; comment` included: the
    /// plugins' `a2rt::Ini` drops it when it hands the value out), or None.
    pub fn value(&self, sec: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.0.eq_ignore_ascii_case(sec) && e.1.eq_ignore_ascii_case(key))
            .map(|e| e.2.as_str())
    }
}

/// A UTF-8 byte order mark at the start of `s`, as a real U+FEFF or as the three latin-1
/// chars (`from_latin1`) it is when the file was read byte for byte, taken off. Editors
/// on Windows write one; left on, it hides the first section's header.
fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').or_else(|| s.strip_prefix("\u{ef}\u{bb}\u{bf}")).unwrap_or(s)
}

/// Parses `text` into `f` (cleared first).
pub fn parse_into(text: &str, f: &mut Parsed) {
    f.clear();
    let text = strip_bom(text);
    let mut sec = String::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with(';') {
            continue;
        }
        if let Some(rest) = l.strip_prefix('[') {
            let s = rest.split(']').next().unwrap_or("").trim();
            sec = s.to_ascii_lowercase();
            if !f.sections.iter().any(|x| x.eq_ignore_ascii_case(s)) {
                f.sections.push(String::from(s));
            }
            continue;
        }
        let Some(eq) = l.find('=') else { continue };
        let key = l[..eq].trim().to_ascii_lowercase();
        if key.is_empty() || f.entries.iter().any(|e| e.0 == sec && e.1 == key) {
            continue;
        }
        let mut v = l[eq + 1..].trim();
        if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) {
            v = &v[1..v.len() - 1];
        }
        f.entries.push((sec.clone(), key, String::from(v)));
    }
}

pub fn parse(text: &str) -> Parsed {
    let mut f = Parsed::default();
    parse_into(text, &mut f);
    f
}

/// The two comment lines a new overlay starts with (fixture rule 8).
pub const HEADER: [&str; 2] = [
    "; RefitPlayer.ini: your choices from the Refit screen, written by UI.asi. Installers never touch it.",
    "; Delete a line to go back to the shipped setting.",
];

const WS: &[char] = &[' ', '\t', '\r', '\n', '\x0b', '\x0c'];

fn trim(s: &str) -> &str {
    s.trim_matches(WS)
}

/// A value `edit` will write: no CR or LF, no leading or trailing whitespace (rule 9).
pub fn valid_value(v: &str) -> bool {
    !v.contains(['\r', '\n']) && trim(v) == v
}

/// A section name `edit` will write: not empty, no `]`, CR or LF, no surrounding
/// whitespace.
pub fn valid_section(s: &str) -> bool {
    !s.is_empty() && trim(s) == s && !s.contains(['\r', '\n', ']'])
}

/// A key name `edit` will write: not empty, no `=`, CR or LF, no surrounding
/// whitespace, not starting `;`, `#` or `[`.
pub fn valid_key(k: &str) -> bool {
    !k.is_empty() && trim(k) == k && !k.contains(['\r', '\n', '=']) && !k.starts_with([';', '#', '['])
}

struct Line<'a> {
    content: String,
    term: &'a str,
}

fn split(text: &str) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find('\n') {
            None => {
                out.push(Line { content: String::from(rest), term: "" });
                break;
            }
            Some(j) => {
                if j > 0 && rest.as_bytes()[j - 1] == b'\r' {
                    out.push(Line { content: String::from(&rest[..j - 1]), term: "\r\n" });
                } else {
                    out.push(Line { content: String::from(&rest[..j]), term: "\n" });
                }
                rest = &rest[j + 1..];
            }
        }
    }
    out
}

/// The file's own line ending: its first; CRLF for a file without one (rule 1).
fn eol(text: &str) -> &'static str {
    match text.find('\n') {
        Some(j) if j > 0 && text.as_bytes()[j - 1] == b'\r' => "\r\n",
        Some(_) => "\n",
        None => "\r\n",
    }
}

fn header(content: &str) -> Option<&str> {
    let t = trim(strip_bom(content));
    let rest = t.strip_prefix('[')?;
    Some(trim(rest.split(']').next().unwrap_or("")))
}

/// The key of a key line (rule 2), else None.
fn key_of(content: &str) -> Option<&str> {
    let t = trim(strip_bom(content));
    if t.is_empty() || t.starts_with([';', '#', '[']) {
        return None;
    }
    let eq = t.find('=')?;
    Some(trim(&t[..eq]))
}

fn blank(content: &str) -> bool {
    trim(content).is_empty()
}

/// (header index, end index) of every occurrence of `section`.
fn occurrences(lines: &[Line], section: &str) -> Vec<(usize, usize)> {
    let mut occ: Vec<(usize, usize)> = Vec::new();
    let mut open = false;
    for (i, l) in lines.iter().enumerate() {
        let Some(h) = header(&l.content) else { continue };
        if open {
            occ.last_mut().unwrap().1 = i;
            open = false;
        }
        if h.eq_ignore_ascii_case(section) {
            occ.push((i, lines.len()));
            open = true;
        }
    }
    occ
}

/// The text of an ini with `[section] key` set to `value`, or removed when `value` is
/// None. Everything untouched is kept byte for byte; the input comes back unchanged
/// when nothing changes, and when the value or a name is refused (`valid_*`).
pub fn edit(text: &str, section: &str, key: &str, value: Option<&str>) -> String {
    if !valid_section(section) || !valid_key(key) || value.is_some_and(|v| !valid_value(v)) {
        return String::from(text);
    }
    let eol = eol(text);
    let mut lines = split(text);
    let occ = occurrences(&lines, section);
    let mut matches: Vec<usize> = Vec::new();
    for &(h, e) in &occ {
        for i in h + 1..e {
            if key_of(&lines[i].content).is_some_and(|k| k.eq_ignore_ascii_case(key)) {
                matches.push(i);
            }
        }
    }
    match value {
        Some(v) => {
            if let Some(&first) = matches.first() {
                let c = &lines[first].content;
                let eq = c.find('=').unwrap_or(c.len());
                let mut n = String::from(&c[..eq + 1]);
                n.push_str(v);
                lines[first].content = n;
                for &i in matches[1..].iter().rev() {
                    lines.remove(i);
                }
            } else if let Some(&(h, e)) = occ.last() {
                let mut at = h;
                for i in h + 1..e {
                    if key_of(&lines[i].content).is_some() {
                        at = i;
                    }
                }
                if lines[at].term.is_empty() {
                    lines[at].term = eol;
                }
                lines.insert(at + 1, Line { content: kv(key, v), term: eol });
            } else {
                if text.is_empty() {
                    for h in HEADER {
                        lines.push(Line { content: String::from(h), term: eol });
                    }
                }
                if let Some(last) = lines.last_mut() {
                    if last.term.is_empty() {
                        last.term = eol;
                    }
                    if !blank(&last.content) {
                        lines.push(Line { content: String::new(), term: eol });
                    }
                }
                let mut s = String::from("[");
                s.push_str(section);
                s.push(']');
                lines.push(Line { content: s, term: eol });
                lines.push(Line { content: kv(key, v), term: eol });
            }
        }
        None => {
            let mut drop = alloc::vec![false; lines.len()];
            for &i in &matches {
                drop[i] = true;
            }
            for &(h, e) in &occ {
                if !(h + 1..e).any(|i| drop[i]) {
                    continue;
                }
                if (h + 1..e).all(|i| drop[i] || blank(&lines[i].content)) {
                    for d in &mut drop[h..e] {
                        *d = true;
                    }
                }
            }
            let mut i = 0;
            lines.retain(|_| {
                let keep = !drop[i];
                i += 1;
                keep
            });
        }
    }
    let mut out = String::with_capacity(text.len() + 64);
    for l in &lines {
        out.push_str(&l.content);
        out.push_str(l.term);
    }
    if out == text {
        return String::from(text);
    }
    if lines.last().is_some_and(|l| l.term.is_empty()) {
        out.push_str(eol);
    }
    out
}

/// The raw value of the first `[section] key` line (text after `=`, trimmed), or None.
fn raw_value(text: &str, section: &str, key: &str) -> Option<String> {
    let lines = split(text);
    for (h, e) in occurrences(&lines, section) {
        for l in &lines[h + 1..e] {
            if key_of(&l.content).is_some_and(|k| k.eq_ignore_ascii_case(key)) {
                let c = &l.content;
                return Some(String::from(trim(&c[c.find('=').map_or(c.len(), |i| i + 1)..])));
            }
        }
    }
    None
}

/// Moves `[from_sec] from_key` to `[to_sec] to_key` (a settings row's `moved_from`,
/// tools/settings/README.md): the value goes over with its key; when the new key is there
/// already (with a value) it wins and the old one is only removed. Unchanged when the old key is absent,
/// both are the same, or a name or the value is refused.
pub fn rename(text: &str, from_sec: &str, from_key: &str, to_sec: &str, to_key: &str) -> String {
    if (from_sec.eq_ignore_ascii_case(to_sec) && from_key.eq_ignore_ascii_case(to_key)) || !valid_section(to_sec) || !valid_key(to_key) {
        return String::from(text);
    }
    // An empty value (`Key=`) is no choice of the player's: it does not move and does not
    // hold the new key against a value that does.
    let Some(v) = raw_value(text, from_sec, from_key).filter(|v| !v.is_empty()) else { return String::from(text) };
    if raw_value(text, to_sec, to_key).is_some_and(|n| !n.is_empty()) {
        return edit(text, from_sec, from_key, None);
    }
    if !valid_value(&v) {
        return String::from(text);
    }
    let set = edit(text, to_sec, to_key, Some(&v));
    edit(&set, from_sec, from_key, None)
}

fn kv(key: &str, v: &str) -> String {
    let mut s = String::from(key);
    s.push('=');
    s.push_str(v);
    s
}

/// Bytes as latin-1 text, one char per byte, so that `edit` keeps every byte it does
/// not touch whatever the file's encoding.
pub fn from_latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The inverse of `from_latin1` (a char above U+00FF, which `edit` never makes from
/// latin-1 input, becomes `?`).
pub fn to_latin1(s: &str) -> Vec<u8> {
    s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}
