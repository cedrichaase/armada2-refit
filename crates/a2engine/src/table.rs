//! The generated symbol tables' types (`tools/engine/gen.py` writes one `Table` per
//! profile file; README.md, "The generated code"). Every build of the game is in it,
//! flattened; the run-time state lives in statics beside it (`Runtime`).
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// No entry (a `target` of `"any"`, no vtable).
pub const NONE: u16 = 0xffff;
/// The longest SettingsNote text (tools/settings/README.md, "C5 amendment").
pub const NOTE_MAX: usize = 120;

/// A symbol of the table: its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sym(pub u16);

/// A hook of the table: its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Fn,
    Call,
    Jmp,
    Slot,
    Label,
    Patch,
    Scan,
    Vtable,
    Global,
    Rtti,
    Field,
    Value,
}

pub struct SymDef {
    /// The profile's `id` (snake case).
    pub id: &'static str,
    /// The map name it belongs to, as logged.
    pub symbol: &'static str,
    pub kind: Kind,
}

pub struct HookDef {
    /// `<plugin>.<hook>`.
    pub id: &'static str,
    /// `unknown = "run"`: may run on an unknown build (scan entries only).
    pub run_unknown: bool,
    /// The settings rows (`Section.Key`) whose SettingsNote this hook feeds.
    pub rows: &'static [&'static str],
    /// Its entries, in table order.
    pub syms: &'static [u16],
}

/// One symbol in one build. Addresses are the build's, unrelocated.
#[derive(Clone, Copy)]
pub struct Entry {
    /// False when this build has no such entry (one a child build adds).
    pub present: bool,
    pub addr: u32,
    /// `fn`'s optional end (exclusive), 0 if none.
    pub end: u32,
    /// `call`/`jmp`/`slot`: the target `fn`, or `NONE` (`"any"`).
    pub target: u16,
    /// `slot`: the vtable.
    pub vtable: u16,
    /// `slot`: the slot's index.
    pub index: u16,
    pub chain: bool,
    /// The check bytes, or a scan's pattern.
    pub check: &'static [u8],
    /// A scan's mask (1 = the byte must match).
    pub mask: &'static [u8],
    /// A scan's section.
    pub section: &'static str,
    /// A scan's expected match count.
    pub count: u16,
    /// A field's offset or a value.
    pub value: i32,
}

impl Entry {
    pub const ABSENT: Entry = Entry {
        present: false,
        addr: 0,
        end: 0,
        target: NONE,
        vtable: NONE,
        index: 0,
        chain: false,
        check: &[],
        mask: &[],
        section: "",
        count: 0,
        value: 0,
    };
}

/// A hook blocked in a build, with its precomputed reason (README.md, "Inheritance").
pub struct Block {
    pub hook: u16,
    pub symbol: &'static str,
    /// `is changed by <build> (not yet verified)` or `is off in <build>: <why>`.
    pub why: &'static str,
    pub note: &'static str,
}

/// A section's CRC-32 in the executable's file on disk (build.toml `[pe] file_crc`).
pub struct FileCrc {
    /// `.text`.
    pub section: &'static str,
    pub crc: u32,
}

/// A companion module the build's run-time marks describe (build.toml `[[module]]`),
/// named by the CRC-32 of its `.text` in its file.
pub struct Module {
    /// `FleetOpsHook.dll`.
    pub name: &'static str,
    pub text_crc: u32,
}

pub struct Build {
    /// `gog-1.1`.
    pub name: &'static str,
    /// `armada2/gog-1.1`.
    pub key: &'static str,
    /// The executable's name (build.toml's first `exe`, `Armada2.exe`): what the
    /// identity line and the notes name in this build.
    pub exe: &'static str,
    pub timestamp: u32,
    pub image_size: u32,
    pub sections: u32,
    pub image_base: u32,
    /// The `N` of `symbols: N/M resolved` (entries present, minus those used only by
    /// blocked hooks).
    pub n_resolved: u16,
    /// The content set this table's plugin reads in this build (`profiles/*/content.toml`,
    /// README.md, "Content"), "" for none.
    pub content: &'static str,
    /// One per symbol, in table order.
    pub entries: &'static [Entry],
    pub blocked: &'static [Block],
    /// On-disk section CRC-32s the executable must also match (README.md, "Identity"):
    /// empty for a build told by its three numbers alone.
    pub file_crc: &'static [FileCrc],
    /// The companion modules this build's marks describe; empty for none.
    pub modules: &'static [Module],
    /// The hooks, besides `blocked`, that stand down when a companion module is not the
    /// one profiled (they depend on its run-time patches).
    pub blocked_mod: &'static [Block],
    /// `n_resolved` with `blocked_mod` blocked too.
    pub n_resolved_mod: u16,
}

pub struct Table {
    pub game: &'static str,
    pub plugin: &'static str,
    pub syms: &'static [SymDef],
    pub hooks: &'static [HookDef],
    pub builds: &'static [Build],
    /// The `N` on an unknown build: the scan entries.
    pub n_scan: u16,
    pub rt: &'static Runtime,
}

/// A note's text, written once when its hook stands down.
pub struct Note {
    pub(crate) buf: UnsafeCell<[u8; NOTE_MAX]>,
    pub(crate) len: AtomicU8,
}

// Written once, before the hook's state is published (Release), never again.
unsafe impl Sync for Note {}

impl Note {
    pub const fn new() -> Note {
        Note { buf: UnsafeCell::new([0; NOTE_MAX]), len: AtomicU8::new(0) }
    }

    pub(crate) fn set(&self, s: &str) {
        let n = s.len().min(NOTE_MAX);
        unsafe { (&mut *self.buf.get())[..n].copy_from_slice(&s.as_bytes()[..n]) };
        self.len.store(n as u8, Ordering::Release);
    }

    pub(crate) fn get(&self) -> Option<&str> {
        let n = self.len.load(Ordering::Acquire) as usize;
        if n == 0 {
            return None;
        }
        core::str::from_utf8(unsafe { &(&*self.buf.get())[..n] }).ok()
    }
}

/// What a table's resolver keeps at run time (statics generated beside the table).
pub struct Runtime {
    /// Per hook: state (bits 0..8), the test flag (bit 8), the why (16..24), the
    /// failing symbol (24..40), the why's argument (40..56).
    pub(crate) hooks: &'static [AtomicU64],
    /// Per symbol: READY, WARNED.
    pub(crate) flags: &'static [AtomicU8],
    pub(crate) addr: &'static [AtomicUsize],
    pub(crate) notes: &'static [Note],
    /// 0 not yet started, 1 unknown build, 2 + i build i.
    pub(crate) build: AtomicU32,
    pub(crate) delta: AtomicU32,
    pub(crate) code: (AtomicUsize, AtomicUsize),
    pub(crate) base: AtomicUsize,
    pub(crate) mem: UnsafeCell<Option<&'static dyn crate::pe::Image>>,
    pub(crate) log: UnsafeCell<Option<fn(&str)>>,
    /// The executable's name as logged (the build's, else the module's), and the plugin
    /// note that names it; written in `init`.
    pub(crate) exe: Note,
    pub(crate) plugin_note: Note,
    /// 1 when a companion module of the matched build is not the one profiled.
    pub(crate) companion_bad: AtomicU8,
}

// `mem` and `log` are written once, in `init`, before `build` is published.
unsafe impl Sync for Runtime {}

impl Runtime {
    pub const fn new(
        hooks: &'static [AtomicU64],
        flags: &'static [AtomicU8],
        addr: &'static [AtomicUsize],
        notes: &'static [Note],
    ) -> Runtime {
        Runtime {
            hooks,
            flags,
            addr,
            notes,
            build: AtomicU32::new(0),
            delta: AtomicU32::new(0),
            code: (AtomicUsize::new(0), AtomicUsize::new(0)),
            base: AtomicUsize::new(0),
            mem: UnsafeCell::new(None),
            log: UnsafeCell::new(None),
            exe: Note::new(),
            plugin_note: Note::new(),
            companion_bad: AtomicU8::new(0),
        }
    }
}
