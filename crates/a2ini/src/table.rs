//! A content set's table (`render3d/content/sets/<set>/<module>.ini`,
//! `ui/content/sets/<set>/ui.ini`; MODDING.md, "Content sets"): ini text read by the
//! rules of `parse` (names without case, a section written twice read as one, the first
//! of a repeated key winning, one pair of quotes dropped, `;` lines skipped), but
//! borrowing from the text, so a table built into a plugin with `include_str!` hands out
//! `&'static str`s. Sections and keys keep their file order: a list of races or recipes
//! is a table's order.
use alloc::vec::Vec;

#[derive(Default, Debug)]
pub struct Table<'a> {
    /// (section as written, its (key as written, value)s), in file order.
    pub sections: Vec<(&'a str, Vec<(&'a str, &'a str)>)>,
}

impl<'a> Table<'a> {
    pub fn parse(text: &'a str) -> Table<'a> {
        let text = super::strip_bom(text);
        let mut t = Table::default();
        let mut cur: Option<usize> = None;
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with(';') {
                continue;
            }
            if let Some(rest) = l.strip_prefix('[') {
                let s = rest.split(']').next().unwrap_or("").trim();
                cur = Some(match t.sections.iter().position(|x| x.0.eq_ignore_ascii_case(s)) {
                    Some(i) => i,
                    None => {
                        t.sections.push((s, Vec::new()));
                        t.sections.len() - 1
                    }
                });
                continue;
            }
            let Some(eq) = l.find('=') else { continue };
            let key = l[..eq].trim();
            // Keys before any section go in the unnamed section "".
            let i = match cur {
                Some(i) => i,
                None => {
                    t.sections.push(("", Vec::new()));
                    cur = Some(t.sections.len() - 1);
                    t.sections.len() - 1
                }
            };
            let es = &mut t.sections[i].1;
            if key.is_empty() || es.iter().any(|e| e.0.eq_ignore_ascii_case(key)) {
                continue;
            }
            let mut v = l[eq + 1..].trim();
            if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) {
                v = &v[1..v.len() - 1];
            }
            es.push((key, v));
        }
        t
    }

    /// The entries of `[sec]` in file order (empty when there is no such section).
    pub fn entries(&self, sec: &str) -> &[(&'a str, &'a str)] {
        self.sections.iter().find(|s| s.0.eq_ignore_ascii_case(sec)).map_or(&[], |s| s.1.as_slice())
    }

    /// The sections whose name starts with `prefix` (any case), in file order, with the
    /// rest of the name: `[Race.Borg]` for `Race.`.
    pub fn sections_under<'s>(&'s self, prefix: &'s str) -> impl Iterator<Item = (&'a str, &'s [(&'a str, &'a str)])> + 's {
        self.sections.iter().filter_map(move |(n, es)| {
            let head = n.get(..prefix.len())?;
            head.eq_ignore_ascii_case(prefix).then(|| (&n[prefix.len()..], es.as_slice()))
        })
    }

    /// The value of `key` in `[sec]`, or None.
    pub fn get(&self, sec: &str, key: &str) -> Option<&'a str> {
        find(self.entries(sec), key)
    }

    /// The value of `key` in `[sec]`, or "".
    pub fn str(&self, sec: &str, key: &str) -> &'a str {
        self.get(sec, key).unwrap_or("")
    }

    /// The value of `key` in `[sec]` as a comma-separated list (items trimmed, empty
    /// ones dropped).
    pub fn list(&self, sec: &str, key: &str) -> Vec<&'a str> {
        split_list(self.str(sec, key))
    }
}

/// The value of `key` among `entries` (any case).
pub fn find<'a>(entries: &[(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    entries.iter().find(|e| e.0.eq_ignore_ascii_case(key)).map(|e| e.1)
}

/// A comma-separated list: items trimmed, empty ones dropped.
pub fn split_list(v: &str) -> Vec<&str> {
    v.split(',').map(str::trim).filter(|s| !s.is_empty()).collect()
}

/// A comma-separated list of numbers; an item that does not read is 0.
pub fn floats(v: &str) -> Vec<f32> {
    split_list(v).iter().map(|s| s.parse::<f32>().unwrap_or(0.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_of_parse() {
        let t = Table::parse("\u{feff}k0 = top\n[A]\nx = 1\n; c\nX = 2\n y = \"q\" \n[b]\nz=3\n[a]\nw = 4\nx = 5\n");
        assert_eq!(t.get("", "k0"), Some("top"));
        assert_eq!(t.entries("a"), &[("x", "1"), ("y", "q"), ("w", "4")]);
        assert_eq!(t.str("B", "Z"), "3");
        assert_eq!(t.str("b", "none"), "");
        assert_eq!(t.list("q", "q"), Vec::<&str>::new());
        assert_eq!(split_list(" a, b ,,c "), ["a", "b", "c"]);
        assert_eq!(floats("1, 2.5, x"), [1.0, 2.5, 0.0]);
    }

    #[test]
    fn sections_under_keep_order() {
        let t = Table::parse("[Race.Fed]\nn=1\n[Other]\n[race.Borg]\nn=2\n");
        let v: Vec<_> = t.sections_under("Race.").map(|(n, es)| (n, find(es, "N"))).collect();
        assert_eq!(v, [("Fed", Some("1")), ("Borg", Some("2"))]);
        assert!(Table::parse("").sections.is_empty());
    }
}
