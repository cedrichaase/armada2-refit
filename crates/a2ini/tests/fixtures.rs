//! `edit` against every fixture of tools/settings/fixtures (the spec it shares with
//! the host tool `a2ini`): `NN-name.in`, `.args` (a JSON array: section, key, and
//! the value or null to remove the key) and `.out`, all latin-1.
use std::path::PathBuf;

/// The three entries of a fixture's `.args`, a JSON array of strings and null.
fn args_of(text: &str) -> Vec<Option<String>> {
    let mut out: Vec<Option<String>> = Vec::new();
    let inner = text.trim().strip_prefix('[').and_then(|t| t.strip_suffix(']')).expect("args: not an array");
    let mut it = inner.chars().peekable();
    loop {
        while it.peek().is_some_and(|c| c.is_whitespace() || *c == ',') {
            it.next();
        }
        match it.next() {
            None => break,
            Some('n') => {
                assert_eq!(it.by_ref().take(3).collect::<String>(), "ull");
                out.push(None);
            }
            Some('"') => {
                let mut s = String::new();
                loop {
                    match it.next().expect("args: open string") {
                        '"' => break,
                        '\\' => match it.next().unwrap() {
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            'u' => {
                                let h: String = it.by_ref().take(4).collect();
                                s.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
                            }
                            c => s.push(c),
                        },
                        c => s.push(c),
                    }
                }
                out.push(Some(s));
            }
            Some(c) => panic!("args: unexpected {c:?}"),
        }
    }
    out
}

fn args(text: &str) -> (String, String, Option<String>) {
    let a = args_of(text);
    assert_eq!(a.len(), 3, "args: want [section, key, value]");
    (a[0].clone().unwrap(), a[1].clone().unwrap(), a[2].clone())
}

#[test]
fn fixtures() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/settings/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("tools/settings/fixtures")
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".in").map(String::from))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no fixtures");
    let mut bad = Vec::new();
    for n in &names {
        let read = |ext: &str| a2ini::from_latin1(&std::fs::read(dir.join(format!("{n}.{ext}"))).unwrap());
        let (sec, key, val) = args(&read("args"));
        let got = a2ini::edit(&read("in"), &sec, &key, val.as_deref());
        if got != read("out") {
            eprintln!("FAIL {n}\n  want {:?}\n  got  {:?}", read("out"), got);
            bad.push(n.clone());
        }
    }
    assert!(bad.is_empty(), "{} of {} fixtures fail: {bad:?}", bad.len(), names.len());
    println!("{} fixtures pass", names.len());
}

/// `rename` on fixtures/rename: `.args` is [from section, from key, to section, to key].
#[test]
fn rename_fixtures() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/settings/fixtures/rename");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("tools/settings/fixtures/rename")
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".in").map(String::from))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no rename fixtures");
    for n in &names {
        let read = |ext: &str| a2ini::from_latin1(&std::fs::read(dir.join(format!("{n}.{ext}"))).unwrap());
        let a: Vec<String> = args_of(&read("args")).into_iter().map(Option::unwrap).collect();
        assert_eq!(a.len(), 4, "{n}: want four names");
        assert_eq!(a2ini::rename(&read("in"), &a[0], &a[1], &a[2], &a[3]), read("out"), "{n}");
    }
}

#[test]
fn latin1_round_trip() {
    let b: Vec<u8> = (0..=255u8).collect();
    assert_eq!(a2ini::to_latin1(&a2ini::from_latin1(&b)), b);
}

#[test]
fn parse_reads_as_get_private_profile() {
    let p = a2ini::parse("; c\r\n[A]\r\nk = \"1\" \r\nK=2\r\n[a]\r\nx=3\r\n[B]\r\n");
    assert_eq!(p.value("a", "K"), Some("1"));
    assert_eq!(p.value("A", "x"), Some("3"));
    assert_eq!(p.sections, ["A", "B"]);
}

#[test]
fn parse_skips_a_byte_order_mark() {
    for bom in ["\u{feff}", "\u{ef}\u{bb}\u{bf}"] {
        let p = a2ini::parse(&format!("{bom}[UI]\r\nA=1\r\n"));
        assert_eq!(p.value("ui", "a"), Some("1"));
        assert_eq!(p.sections, ["UI"]);
    }
}
