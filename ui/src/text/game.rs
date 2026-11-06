//! The engine's text as the UI draws it (README.md, "The engine's text"). The game's
//! strings are bytes in its fonts' code page: Windows-1252, whose 0x80..0x9f hold the
//! bullet, the dashes and the curly quotes where Latin-1 has control characters. A game
//! whose fonts draw some codes as pictures or as figures of their own (Fleet Operations:
//! Roots' resource icons, unit classes and red figures, PLAN-FLEETOPS.md decision 160)
//! names them in its content set's `[Glyphs]` (`content::Content::glyphs`), and the UI
//! writes the word or the figure there instead. The strings come in as `char`s of one
//! byte each (`engine::buttons::latin1`, the string streams of `engine::msvc`). The rules
//! are `content::Content::display`'s, where the host tests reach them.
use alloc::string::String;

/// The game's string `s` (one byte per `char`) as the UI draws it, by the build's
/// content set.
pub fn display(s: &str) -> String {
    crate::content::get().display(s)
}
