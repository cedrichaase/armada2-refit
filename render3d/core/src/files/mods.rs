//! Fleet Operations' mods, pure (host-tested in `render3d/hosttest`): which mod the
//! command line makes active, and the chain of folders its files come from.
//!
//! A mod is a folder `Mods\<name>\` of the game's data folder with an `info.ini`. The
//! game takes the first `-mod <name>` or `/mod <name>` on its command line (any case; the
//! name in quotes when it has spaces) and, when a folder of that name is a mod, layers
//! its files over the game's: the active mod's first, then its `ParentMod`'s, and so on
//! up the chain, then the game's own folder last, **unless a mod in the chain says
//! `standalone=1`**, in which case the game's folder is not searched at all. Within a
//! folder its `ODF.fpq` and its loose folders are both searched (render3d keeps loose
//! over packed, as on a folder of its own). With no switch, or a name that is no mod, the
//! game's folder alone. Measured on Roots 5.0Z11's file system (`PLAN-FLEETOPS.md`, FO-5
//! step 6); the rules are its, written here as data a reader can follow.
use alloc::string::String;
use alloc::vec::Vec;

/// A game's mod layout (`games/<id>.toml` `[files] mods`).
pub struct Rules {
    /// The folder of the mods, under the data folder (`Mods`).
    pub dir: &'static str,
    /// The command-line switches that name the active mod (`-mod`, `/mod`).
    pub switches: &'static [&'static str],
}

/// What a mod's `info.ini` says of its place in the chain.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Info {
    /// `ParentMod=`, the folder name of the mod under it, unquoted; None when empty.
    pub parent: Option<String>,
    /// `standalone=1`: the game's own folder is not searched under this chain.
    pub standalone: bool,
}

/// The active mod's chain: the mods' folder names, the active one first, and whether the
/// game's own folder is searched after them.
#[derive(Debug, PartialEq, Eq)]
pub struct Chain {
    pub mods: Vec<String>,
    pub base: bool,
    /// A parent the chain names that is no mod (its `info.ini` does not read): the chain
    /// stops there, as the game's does.
    pub missing: Option<String>,
}

/// The command line's arguments as Windows hands them to a program: split at blanks, a
/// quoted run kept whole and its quotes taken out (`"STA2 Classic Mod"`); the first is
/// the program itself.
pub fn args(cmdline: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in cmdline.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            ' ' | '\t' if !quoted => {
                if any {
                    out.push(core::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// The name the first of `switches` on the command line gives (the argument after it),
/// skipping the program's own name.
pub fn active(cmdline: &str, switches: &[&str]) -> Option<String> {
    let a = args(cmdline);
    let i = a.iter().skip(1).position(|s| switches.iter().any(|w| w.eq_ignore_ascii_case(s)))? + 1;
    a.get(i + 1).filter(|n| !n.is_empty()).cloned()
}

/// A mod's `info.ini`: `ParentMod` and `standalone` of its `[mod]` section (keys any case).
pub fn info(text: &[u8]) -> Info {
    let mut out = Info::default();
    let mut in_mod = true;
    for raw in text.split(|&c| c == b'\n') {
        let Ok(line) = core::str::from_utf8(raw) else { continue };
        let l = line.trim();
        if l.starts_with('[') && l.ends_with(']') {
            in_mod = l[1..l.len() - 1].trim().eq_ignore_ascii_case("mod");
            continue;
        }
        if !in_mod || l.starts_with(';') {
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let v = v.trim().trim_matches('"').trim();
        if k.trim().eq_ignore_ascii_case("ParentMod") {
            out.parent = (!v.is_empty()).then(|| String::from(v));
        } else if k.trim().eq_ignore_ascii_case("standalone") {
            out.standalone = v == "1" || v.eq_ignore_ascii_case("true");
        }
    }
    out
}

/// The chain from the active mod `name` down: `read(m)` returns mod `m`'s `info.ini`, or
/// None when `m` is no mod. An active name that is no mod gives the game's folder alone.
pub fn chain(name: &str, mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Chain {
    let mut c = Chain { mods: Vec::new(), base: true, missing: None };
    let mut next = Some(String::from(name));
    while let Some(m) = next.take() {
        if c.mods.iter().any(|x| x.eq_ignore_ascii_case(&m)) || c.mods.len() >= 8 {
            break;
        }
        let Some(text) = read(&m) else {
            if !c.mods.is_empty() {
                c.missing = Some(m);
            }
            break;
        };
        let i = info(&text);
        c.base &= !i.standalone;
        c.mods.push(m);
        next = i.parent;
    }
    c
}
