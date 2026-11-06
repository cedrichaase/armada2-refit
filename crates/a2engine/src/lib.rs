//! What the plugins know of the game's executable (README.md): its identity (`pe`), the
//! three numbers every plugin logs and the profiles are keyed by; the generated symbol
//! tables' types (`table`); and the resolver (`engine`), which picks the build and
//! checks each hook's entries in live memory before its plugin writes. `no_std`, no
//! dependency, so the plugins link it and the host tests run it on synthetic PE images.
#![no_std]

pub mod engine;
pub mod pe;
pub mod table;

pub use engine::{glob, init_with, is_identity_line, test_mismatch, Engine, StandDown, Why, PLUGIN_NOTE};
#[cfg(windows)]
pub use engine::init;
pub use table::{Block, Build, Entry, HookDef, HookId, Kind, Note, Runtime, Sym, SymDef, Table, NONE, NOTE_MAX};
