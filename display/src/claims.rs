//! Claims (contract 1.1; `gpu/src/display.rs`, "Claims"; README.md, "Claims"): which
//! categories each plugin draws now, so that passthrough draws what nobody does.
//!
//! A plugin registers at load (`display8v1_register`) with a callback that says what it
//! draws now; Display8 asks every callback at each of the engine's `BeginScene`s in stub
//! mode (`ask`), and logs each plugin's claims when they change. A plugin that stood
//! down never registers (another contract) or claims nothing (no device to draw on), so
//! what it would have drawn is the engine's.
use crate::{log, sfmt};
use alloc::string::String;
use alloc::vec::Vec;
use gpu::display::{read, ClaimsCallback, Plugin8, CLAIM_NAMES};

struct Reg {
    module: usize,
    name: String,
    claims: Option<ClaimsCallback>,
    /// What it claimed when last asked (u32::MAX: never asked).
    last: u32,
}

static mut REGS: Vec<Reg> = Vec::new();

/// The claims' names, for the log ("none" for none).
pub fn names(bits: u32) -> String {
    let mut s = String::new();
    for (b, n) in CLAIM_NAMES {
        if bits & b != 0 {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(n);
        }
    }
    let rest = bits & !CLAIM_NAMES.iter().fold(0, |a, (b, _)| a | b);
    if rest != 0 {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&sfmt!("{:#x}", rest));
    }
    if s.is_empty() {
        s.push_str("none");
    }
    s
}

#[allow(static_mut_refs)]
fn regs() -> &'static mut Vec<Reg> {
    unsafe { &mut REGS }
}

/// `display8v1_register`: a plugin and its claims callback; its id (index + 1).
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_register(p: *const Plugin8) -> u32 {
    if p.is_null() {
        return 0;
    }
    let p = unsafe { read(p) };
    let (module, name) = crate::api::module_of(p.caller);
    let r = regs();
    let at = match r.iter().position(|x| module != 0 && x.module == module) {
        Some(i) => {
            r[i].claims = p.claims;
            r[i].last = u32::MAX;
            i
        }
        None => {
            if r.len() >= 16 {
                log!("claims: {} registers, and 16 plugins have already: refused", name);
                return 0;
            }
            r.push(Reg { module, name: name.clone(), claims: p.claims, last: u32::MAX });
            r.len() - 1
        }
    };
    log!("claims: {} registers (id {}){}", name, at + 1, if p.claims.is_some() { "" } else { ", with no callback: it claims nothing" });
    (at + 1) as u32
}

/// `display8v1_claims`: what every registered plugin but `except` claims now.
#[unsafe(no_mangle)]
pub extern "C" fn display8v1_claims(except: u32) -> u32 {
    let mut all = 0;
    for (i, r) in regs().iter().enumerate() {
        if i as u32 + 1 == except {
            continue;
        }
        if let Some(f) = r.claims {
            all |= f();
        }
    }
    all
}

/// Every plugin's claims now, logged when a plugin's change: what passthrough routes by
/// (`passthrough::begin_scene`).
pub fn ask() -> u32 {
    static mut TOLD: bool = false;
    if regs().is_empty() && !unsafe { core::mem::replace(&mut *core::ptr::addr_of_mut!(TOLD), true) } {
        log!("claims: no plugin has registered (yet): every category is the engine's, drawn by passthrough");
    }
    let mut all = 0;
    for r in regs().iter_mut() {
        let now = r.claims.map_or(0, |f| f());
        if now != r.last {
            log!("claims: {} claims {} (was {})", r.name, names(now), if r.last == u32::MAX { String::from("not asked") } else { names(r.last) });
            r.last = now;
        }
        all |= now;
    }
    all
}
