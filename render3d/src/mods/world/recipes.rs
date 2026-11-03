//! The world's recipes (surfaces and nebulae), read from the game directory's copy or
//! the one built in (`core_recipes`, `content.rs`), with the rules of an ini file: a
//! `;` comment and the blanks around a value are dropped, names are not case sensitive.
use crate::core_recipes::{self, Source};
use alloc::string::String;
use alloc::vec::Vec;

/// A recipe's text, parsed.
pub struct Text(a2ini::Parsed);

impl Text {
    /// The value of `key` in `[sec]`, as `Ini::string` gives it.
    pub fn string(&self, sec: &str, key: &str) -> Option<String> {
        let v = self.0.value(sec, key)?;
        Some(String::from(match v.find(';') {
            Some(c) => v[..c].trim(),
            None => v.trim(),
        }))
    }
}

/// What a lookup found: the recipe's bytes (to notice an edit), parsed, and whether it is
/// the game directory's copy.
pub struct Found {
    pub bytes: Vec<u8>,
    pub text: Text,
    pub game: bool,
}

/// The recipe of `kind` for `name`: the game directory's `<Dir>\<name>.ini`, else the
/// one built in under the lower-cased name.
pub fn find(kind: &'static str, table: &'static [(&'static str, &'static str)], name: &str) -> Option<Found> {
    core_recipes::builtin(kind, table);
    let r = core_recipes::text(kind, name).or_else(|| {
        let low = crate::rt::lower(name);
        if low != name { core_recipes::text(kind, &low) } else { None }
    })?;
    let text = Text(a2ini::parse(&r.text));
    Some(Found { game: r.source == Source::Game, bytes: r.text.into_bytes(), text })
}
