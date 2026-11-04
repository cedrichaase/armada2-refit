//! Which campaigns of the campaign selection are locked: not yet open to the player,
//! because the campaign before them is not finished (MENUS.md, "Locked campaigns").
//!
//! The stock screen does not show it: a locked campaign's panel looks and behaves as an
//! open one's, so the state is read from the player's campaign progress,
//! `save\shell.set`, whose bytes are progress counters (missions completed;
//! platform/README.md, "Campaign progress format"). Which byte counts which campaign has
//! **not been measured**: `[Menus] CampaignProgress=` gives the offset per card (the
//! content set's order; -1 for none) and the default is a guess. A campaign that
//! `requires` another is locked while that one's counter is below its mission count.
//! With no `shell.set`, `CampaignLocks=0` or no offset for the campaign it needs,
//! nothing is locked.
//!
//! A lock is advice only: the entry is drawn greyed with a padlock and says on hover what
//! opens it, render3d's scene shows the faction's view locked (`scene`), but a click
//! still goes to the stock panel, so a wrong guess never keeps an open campaign shut.
//!
//! UI.log: `menus: campaign progress ...` each time the file is read, with its bytes
//! and the locks, which is what the bench needs to measure the layout.
use alloc::string::String;
use alloc::vec::Vec;

const FILE: &str = "save\\shell.set";

struct State {
    /// The file's stamp at the last read (None: missing), and when it was looked at.
    stamp: Option<Option<([u32; 2], u32)>>,
    checked: f64,
    /// Per card of the content set: the card it waits for, when locked.
    locked: Vec<Option<usize>>,
}

static mut STATE: State = State { stamp: None, checked: 0.0, locked: Vec::new() };

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

/// The card `i` waits for (the campaign to finish first) while it is locked; None when
/// it is open, or when that cannot be told.
pub fn locked(i: usize) -> Option<usize> {
    refresh();
    st().locked.get(i).copied().flatten()
}

/// Every locked card's scene (render3d's view), comma-separated ("" for none).
pub fn locked_scenes() -> String {
    refresh();
    let camps = crate::content::get().campaigns;
    let mut out = String::new();
    for (i, l) in st().locked.iter().enumerate() {
        if let (Some(_), Some(c)) = (l, camps.get(i)) {
            if !out.is_empty() {
                out.push(',');
            }
            out.push_str(c.scene.to_str().unwrap_or(""));
        }
    }
    out
}

/// Reads the progress again when the file has changed (looked at once a second).
fn refresh() {
    let s = st();
    let now = crate::rt::now();
    if s.stamp.is_some() && now - s.checked < 1.0 {
        return;
    }
    s.checked = now;
    let cfg = crate::config::get();
    let path = crate::rt::game_path(FILE);
    let stamp = crate::rt::file_stamp(&path);
    if s.stamp == Some(stamp) {
        return;
    }
    s.stamp = Some(stamp);
    let camps = crate::content::get().campaigns;
    s.locked = alloc::vec![None; camps.len()];
    if !cfg.campaign_locks {
        crate::log!("menus: campaign locks off (CampaignLocks=0)");
        return;
    }
    let Some(bytes) = stamp.and_then(|_| crate::rt::read_file(&path)) else {
        crate::log!("menus: campaign progress: no {}, every campaign shown open", FILE);
        return;
    };
    let counter = |card: usize| -> Option<u8> {
        let off = *cfg.campaign_progress.get(card)?;
        if off < 0 { None } else { bytes.get(off as usize).copied() }
    };
    let mut said = String::new();
    for (i, c) in camps.iter().enumerate() {
        let Some(req) = c.requires else { continue };
        let Some(n) = counter(req) else { continue };
        if n < camps[req].missions {
            s.locked[i] = Some(req);
            said.push_str(crate::sfmt!(" {} (waits for {}: {}/{})", c.key, camps[req].key, n, camps[req].missions).as_str());
        }
    }
    let mut dump = String::new();
    for b in &bytes {
        dump.push_str(crate::sfmt!("{} ", b).as_str());
    }
    crate::log!("menus: campaign progress ({}, {} bytes): {}", FILE, bytes.len(), dump.trim_end());
    crate::log!("menus: campaign progress offsets {:?}; locked:{}", cfg.campaign_progress.as_slice(), if said.is_empty() { " none" } else { said.as_str() });
}
