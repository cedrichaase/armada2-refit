//! Hook ownership and hand-back (README.md, "Module interface", "Hooks"): every patch
//! render3d makes in `Armada2.exe` goes through `Hooks::install`, which checks the site
//! against what stock has there before writing, logs one line per hook, and hands the
//! detour what was there before (`Prev`) to call, whether the engine's function or
//! another plugin's. A hook belongs to one owner (a module, or the core's frame) and
//! is opened and shut by its owner's on flag (`Gate`).
//!
//! The rules (binding for every module):
//! - Install at the owner's first switch-on, never uninstall: another plugin may have
//!   chained on top of us since. A module that is off at load installs nothing.
//! - Every detour starts with `if !GATE.open() { return prev(args) }`, with the
//!   original arguments, and calls `prev`, never a constant. A detour that replaces the
//!   engine's function whole calls `prev` while its gate is shut.
//! - What is patched is checked first: a `Call` site must call `site.expect`, a `Slot`
//!   must hold it. With `chain`, a target outside the executable's code (another
//!   plugin's) is accepted too, and logged `(chained)`. `expect == 0` checks nothing:
//!   a slot of an object outside `Armada2.exe` (the d3d8 device's), which no stock
//!   value describes.
//!
//! Staged in the one crate until the crates are split (`core/` then). `a2rt::hook`'s
//! primitives are unchanged; the checks are here.
use crate::core_module::ModuleId;
use crate::engine::{self, hook};
use crate::log;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

/// Where a hook goes: the name the game's map gives it, its address, and what stock
/// has there (the call's target, or the slot's value). For a `Slot`, `addr` is the
/// vtable (`Kind::Slot.vt` repeats it). Phase 8e resolves `addr` and `expect` from the
/// profile; `addr == 0` (unresolved) stands down as a site that is not stock does.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub symbol: &'static str,
    pub addr: usize,
    pub expect: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// A `call rel32` at `site.addr`, re-pointed.
    Call,
    /// Slot `slot` of the vtable at `vt`, written.
    Slot { vt: usize, slot: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct Hook {
    pub owner: ModuleId,
    /// What the hook is for, in a few words, for the log and the profile.
    pub used_by: &'static str,
    pub kind: Kind,
    pub site: Site,
    /// The detour.
    pub to: usize,
    /// Accept another plugin's target in place of stock's (logged `(chained)`).
    pub chain: bool,
}

/// What was there before: the engine's function, or another plugin's. Detours call
/// this, never a constant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prev(pub usize);

impl Prev {
    pub fn addr(self) -> usize {
        self.0
    }
}

/// A hook's owner's on flag, read by its detours: shut, the detour hands the call to
/// `Prev` with the arguments it was given.
#[derive(Clone, Copy)]
pub struct Gate(pub &'static AtomicBool);

static ALWAYS: AtomicBool = AtomicBool::new(true);

impl Gate {
    /// The core's own hooks (the frame's): always open, since `Enable=0` installs none.
    pub const ENABLE: Gate = Gate(&ALWAYS);

    #[inline(always)]
    pub fn open(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// One hook installed, for the log and Phase 8's enumeration of every site we patch.
#[derive(Clone, Copy, Debug)]
pub struct Installed {
    pub owner: &'static str,
    pub hook: Hook,
    pub prev: Prev,
    pub chained: bool,
}

static mut INSTALLED: Vec<Installed> = Vec::new();

/// Every hook installed so far, in order.
#[allow(static_mut_refs)]
pub fn installed() -> &'static [Installed] {
    unsafe { &INSTALLED }
}

/// An owner's view of the registry: its id, name and gate, given to its `hooks()`.
pub struct Hooks {
    owner: ModuleId,
    name: &'static str,
    gate: Gate,
    down: Option<&'static str>,
}

/// Whether a value is in the executable's code: a target there that is not stock's is
/// not another plugin's either, so nothing is chained onto it.
fn in_exe_code(a: usize) -> bool {
    engine::EXE_CODE.contains(&a)
}

impl Hooks {
    /// The core's own (the frame's hooks), behind `Gate::ENABLE`.
    pub fn core() -> Hooks {
        Hooks { owner: ModuleId::CORE, name: "frame", gate: Gate::ENABLE, down: None }
    }

    pub(crate) fn for_module(owner: ModuleId, name: &'static str, gate: Gate) -> Hooks {
        Hooks { owner, name, gate, down: None }
    }

    pub fn owner(&self) -> ModuleId {
        self.owner
    }

    /// The owner's gate, for its detours to keep in a static.
    pub fn gate(&self) -> Gate {
        self.gate
    }

    /// A hook the owner cannot draw without did not install: it is not switched on.
    pub fn stand_down(&mut self, why: &'static str) {
        if self.down.is_none() {
            self.down = Some(why);
        }
    }

    /// Why the owner stood down, if it did.
    pub fn down(&self) -> Option<&'static str> {
        self.down
    }

    /// What is at the hook's site now, and whether it may be patched: Some((value,
    /// chained)) if it is stock's (or another plugin's, with `chain`).
    fn current(&self, h: &Hook) -> Option<(usize, bool)> {
        if h.site.addr == 0 {
            return None;
        }
        let cur = match h.kind {
            Kind::Call => hook::call_target(h.site.addr)?,
            Kind::Slot { vt, slot } => {
                if vt == 0 {
                    return None;
                }
                engine::field(vt, slot * 4)
            }
        };
        if h.site.expect == 0 || cur == h.site.expect {
            return Some((cur, false));
        }
        if h.chain && !in_exe_code(cur) {
            return Some((cur, true));
        }
        None
    }

    /// Whether `install` would patch the hook now (nothing is written): for an owner
    /// whose hooks must all go in or none (the frame's two seams).
    pub fn check(&self, h: &Hook) -> bool {
        self.current(h).is_some()
    }

    /// Patches the hook's site, logged. None (nothing written, logged) when the site is
    /// not stock's.
    pub fn install(&mut self, h: Hook) -> Option<Prev> {
        debug_assert!(h.owner == self.owner);
        let Some((cur, chained)) = self.current(&h) else {
            log!("hooks: {} {} at {} ({:#x}) not stock: {} stands down for {}", self.name, h.used_by, h.site.symbol, h.site.addr, self.name, h.used_by);
            return None;
        };
        let ok = match h.kind {
            Kind::Call => hook::patch_call(h.site.addr, cur, h.to),
            Kind::Slot { vt, slot } => hook::patch_slot(vt, slot, h.to) == Some(cur),
        };
        if !ok {
            log!("hooks: {} {} at {} ({:#x}) could not be written: {} stands down for {}", self.name, h.used_by, h.site.symbol, h.site.addr, self.name, h.used_by);
            return None;
        }
        log!("hooks: {} {} at {} ({:#x}) installed{}", self.name, h.used_by, h.site.symbol, h.site.addr, if chained { " (chained)" } else { "" });
        let prev = Prev(cur);
        #[allow(static_mut_refs)]
        unsafe { INSTALLED.push(Installed { owner: self.name, hook: h, prev, chained }) };
        Some(prev)
    }
}
