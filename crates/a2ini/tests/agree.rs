//! The parser (`parse`) and the edit (`edit`) must see the same keys: an edit that sets
//! a key must be read back, and one that removes a key must leave the others alone.
use a2ini::{edit, parse};

#[test]
fn mid_file_bom_is_text_for_both() {
    let text = "L==x\n;K=3\n\n\u{feff}[S]";
    let out = edit(text, "S", "k", Some("a;b"));
    assert_eq!(parse(&out).value("S", "k"), Some("a;b"), "{out:?}");
}

#[test]
fn leading_bom_still_hides_nothing() {
    let out = edit("\u{feff}[S]\nk=1\n", "S", "k", Some("2"));
    assert_eq!(out, "\u{feff}[S]\nk=2\n");
    assert_eq!(parse(&out).value("S", "k"), Some("2"));
}

#[test]
fn unicode_space_is_not_whitespace() {
    // Scale<NBSP>= is a different key to both: the edit appends Scale, the parser reads it.
    let text = "[UI.HUD]\r\nScale\u{a0}=2";
    let out = edit(text, "UI.HUD", "Scale", Some("1"));
    assert_eq!(parse(&out).value("UI.HUD", "Scale"), Some("1"), "{out:?}");
    assert_eq!(parse(&out).value("UI.HUD", "Scale\u{a0}"), Some("2"));
    // ASCII space/tab around the key and value is still trimmed by both.
    let out = edit("[S]\n \tk \t= 2 \n", "S", "k", Some("3"));
    assert_eq!(parse(&out).value("S", "k"), Some("3"));
}
