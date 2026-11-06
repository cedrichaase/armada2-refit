//! The player's campaign progress as the game holds it (`g_shellSettings`, the object
//! `save\shell.set` is loaded into and saved from), for the campaign selection's locks
//! (`menus::locks`; MENUS.md, "Locked campaigns"). Read only.
//!
//! Measured on the bench against crafted `shell.set` files and the running game's memory:
//! the file's bytes 0..7 sit at `+0x0d` of the object, four per campaign in the order
//! Tutorials, Federation, Klingon, Borg: first the furthest mission reached (0-based:
//! `n + 1` missions to choose from), then whether the campaign is complete (its last
//! mission won). The four campaigns' mission counts follow at `+0x15` (6, 10, 10, 10).
//! The game writes the file only now and then (when the player leaves, and when a won
//! mission advances the counter), so a campaign finished a moment ago is complete here
//! and not yet in the file.
use super::rd;
use crate::syms_shell::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// One campaign's progress per card (the content set's order).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub reached: [u8; 4],
    pub complete: [u8; 4],
    pub missions: [u8; 4],
}

/// The progress in memory, or None: the hook is not ready on this build, or what the
/// object holds does not look like progress (a counter past its campaign's missions, a
/// flag that is not 0 or 1, a campaign of no missions).
pub fn read() -> Option<Progress> {
    if eng().ready(hook::SHELL_READS_PROGRESS).is_err() {
        return None;
    }
    let base = eng().addr(sym::PROGRESS_SETTINGS);
    if base == 0 {
        return None;
    }
    let bytes = |off: usize| -> [u8; 4] { unsafe { rd::<[u8; 4]>(base + off) } };
    let p = Progress {
        reached: bytes(field::PROGRESS_REACHED()),
        complete: bytes(field::PROGRESS_COMPLETE()),
        missions: bytes(field::PROGRESS_MISSIONS()),
    };
    let sane = (0..4).all(|i| p.missions[i] > 0 && p.missions[i] <= 32 && p.reached[i] < p.missions[i] && p.complete[i] <= 1);
    if sane { Some(p) } else { None }
}
