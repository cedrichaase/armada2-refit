//! Recipes (README.md, "Module interface", "Recipes"): a module's data files, looked up
//! by kind and name. The game directory's copy wins (`Sky\<name>.ini`,
//! `Surfaces\<name>.ini`, `Nebulae\<name>.ini`, `Menu\<name>.ini`), so a recipe can be
//! tuned without a build; else the one the module built in (`builtin`, its
//! `include_str!`s), so nothing needs to be installed for the look.
//!
//! A module registers its content set's recipes (`content::get()`, README.md, "Content
//! sets"); a build with no set registers none.
//!
//! Staged in the one crate until the crates are split (`core/` then).
use alloc::string::String;
use alloc::vec::Vec;

/// Where a recipe's text came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// The game directory's file (`<Dir>\<name>.ini`).
    Game,
    /// Built into the plugin.
    BuiltIn,
}

pub struct RecipeText {
    pub text: String,
    pub source: Source,
}

/// A kind's directory in the game directory, or None for a kind that is built in only
/// (ordnance: its `Render3D.ini` sections override it section by section instead).
fn dir(kind: &str) -> Option<&'static str> {
    match kind {
        "sky" => Some("Sky\\"),
        "surfaces" => Some("Surfaces\\"),
        "nebulae" => Some("Nebulae\\"),
        "menu" => Some("Menu\\"),
        _ => None,
    }
}

static mut BUILTIN: Vec<(&'static str, &'static [(&'static str, &'static str)])> = Vec::new();

/// A module's built-in recipes of a kind, as (name, text): registered once (again is a
/// no-op), before the first lookup.
#[allow(static_mut_refs)]
pub fn builtin(kind: &'static str, table: &'static [(&'static str, &'static str)]) {
    let b = unsafe { &mut BUILTIN };
    if !b.iter().any(|(k, t)| *k == kind && core::ptr::eq(*t, table)) {
        b.push((kind, table));
    }
}

/// The built-in recipe of a kind and name, if a module registered one.
#[allow(static_mut_refs)]
pub fn built_in(kind: &str, name: &str) -> Option<&'static str> {
    let b = unsafe { &BUILTIN };
    b.iter().filter(|(k, _)| *k == kind).flat_map(|(_, t)| t.iter()).find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// The recipe of a kind and name: the game directory's `<Dir>\<name>.ini` first, then
/// the built-in one.
pub fn text(kind: &str, name: &str) -> Option<RecipeText> {
    if let Some(d) = dir(kind) {
        let mut path = String::from(d);
        path.push_str(name);
        path.push_str(".ini");
        if let Some(b) = crate::files::open(&path) {
            return Some(RecipeText { text: String::from_utf8_lossy(&b).into_owned(), source: Source::Game });
        }
    }
    built_in(kind, name).map(|t| RecipeText { text: String::from(t), source: Source::BuiltIn })
}
