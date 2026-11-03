//! What the plugins know of the game's executable. Today: its identity (`pe`), the three
//! numbers every plugin logs at start and the bench records. `no_std`, no dependency,
//! so the plugins link it and the host tests run it on synthetic PE images.
#![no_std]

pub mod pe;
