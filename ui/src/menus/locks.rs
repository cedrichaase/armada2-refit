//! Which campaigns of the campaign selection are locked: not yet open to the player,
//! because the campaign before them is not complete (MENUS.md, "Locked campaigns").
//!
//! The stock screen does not show it: a locked campaign's panel looks as an open one's,
//! and a click on it does nothing. What the game decides by was measured on the bench
//! (MENUS.md): per campaign, the furthest mission reached and a flag set when its last
//! mission is won; the Klingon campaign opens once the Federation one is complete, the
//! Borg once the Klingon one is. The counters do not open anything (a Federation counter
//! at 10 left the Klingons shut). So a card whose campaign `requires` another is locked
//! while that one's flag is 0.
//!
//! The progress is read from the game's memory (`engine::api::progress`), where a campaign
//! won a moment ago is already complete; without it (the hook not ready, the object not
//! plausible) from `save\shell.set`, the same bytes as the game last wrote them (bytes
//! 0..3 reached, 4..7 complete). With neither, or `CampaignLocks=0`, nothing is locked.
//!
//! A lock is drawn only: the entry is greyed with a padlock and says on hover what opens
//! it, render3d's scene shows the faction's view locked (`scene`), and a click still goes
//! to the stock panel, which decides.
//!
//! UI.log: `menus: campaign progress ...` each time what is read changes, with where it
//! was read from, and the locks.
use crate::engine::api::progress::Progress;
use alloc::string::String;
use alloc::vec::Vec;

const FILE: &str = "save\\shell.set";

struct State {
    /// The progress last read and where from (None: none), and when it was read.
    last: Option<(Option<Progress>, &'static str)>,
    checked: f64,
    /// Per card of the content set: the card it waits for, when locked.
    locked: Vec<Option<usize>>,
}

static mut STATE: State = State { last: None, checked: -1.0, locked: Vec::new() };

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

/// `save\shell.set`'s first eight bytes, as the game last wrote them.
fn from_file() -> Option<Progress> {
    let b = crate::rt::read_file(&crate::rt::game_path(FILE))?;
    if b.len() < 8 {
        return None;
    }
    Some(Progress { reached: [b[0], b[1], b[2], b[3]], complete: [b[4], b[5], b[6], b[7]], missions: [0; 4] })
}

/// Reads the progress again (four times a second) and works the locks out when it changed.
fn refresh() {
    let s = st();
    let now = crate::rt::now();
    if s.last.is_some() && now - s.checked < 0.25 {
        return;
    }
    s.checked = now;
    let cfg = crate::config::get();
    let read = if !cfg.campaign_locks {
        (None, "off (CampaignLocks=0)")
    } else if let Some(p) = crate::engine::api::progress::read() {
        (Some(p), "memory")
    } else {
        match from_file() {
            Some(p) => (Some(p), FILE),
            None => (None, "none: no progress in memory nor in save\\shell.set"),
        }
    };
    if s.last == Some(read) {
        return;
    }
    s.last = Some(read);
    let camps = crate::content::get().campaigns;
    s.locked = alloc::vec![None; camps.len()];
    let Some(p) = read.0 else {
        crate::log!("menus: campaign progress {}; every campaign shown open", read.1);
        return;
    };
    let mut said = String::new();
    for (i, c) in camps.iter().enumerate() {
        let Some(req) = c.requires else { continue };
        let Some(&done) = p.complete.get(req) else { continue };
        if done == 0 {
            s.locked[i] = Some(req);
            said.push_str(crate::sfmt!(" {} (waits for {})", c.key, camps[req].key).as_str());
        }
    }
    crate::log!(
        "menus: campaign progress from {}: reached {:?}, complete {:?}; locked:{}",
        read.1,
        p.reached,
        p.complete,
        if said.is_empty() { " none" } else { said.as_str() }
    );
}
