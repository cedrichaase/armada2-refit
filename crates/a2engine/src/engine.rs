//! The resolver (phase 8 contract P4): which build this is, and whether each hook's
//! entries hold in live memory before its plugin writes anything. The same algorithm
//! as `csdk/a2engine.h` and `tools/engine/resolve.py`, proven on the same fixtures
//! (README.md, "The resolver").
use crate::pe::{self, Companion, Identity, Image};
use crate::table::*;
use core::fmt::{self, Write};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

const READY: u8 = 1;
const WARNED: u8 = 2;

const ST_READY: u64 = 1;
const ST_FAILED: u64 = 2;
const ST_TEST: u64 = 0x100;

const W_NOT_STOCK: u64 = 1;
const W_CALLS: u64 = 2;
const W_BLOCKED: u64 = 3;
const W_UNKNOWN: u64 = 5;
const W_TEST: u64 = 6;
const W_ABSENT: u64 = 7;
const W_FOUND: u64 = 8;
const W_MODULE: u64 = 9;

/// Why a hook stands down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// `is not stock (bytes differ)`.
    NotStock,
    /// `calls something else`.
    CallsElse,
    /// A `[[changed]]` or `[[disable]]` mark of the build: its precomputed text.
    Blocked(&'static str),
    /// `unknown build`.
    Unknown,
    /// `test mismatch (A2EngineTest.ini)`.
    Test,
    /// `has no address in <build>`.
    Absent(&'static str),
    /// A scan's `is found <n> times, not <count>`.
    Found(u16, u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandDown {
    pub hook: &'static str,
    pub symbol: &'static str,
    pub why: Why,
}

/// `<symbol> <why>`, as the stand-down line has it after `stands down: `.
impl fmt::Display for StandDown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol)?;
        match self.why {
            Why::NotStock => f.write_str(" is not stock (bytes differ)"),
            Why::CallsElse => f.write_str(" calls something else"),
            Why::Blocked(w) => write!(f, " {}", w),
            Why::Unknown => f.write_str(" unknown build"),
            Why::Test => f.write_str(" test mismatch (A2EngineTest.ini)"),
            Why::Absent(b) => write!(f, " has no address in {}", b),
            Why::Found(n, c) => write!(f, " is found {} times, not {}", n, c),
        }
    }
}

/// The plugin note on an unknown build: `This <exe> is a build Refit does not know: it
/// runs stock`, `<exe>` as in the identity line.
pub const PLUGIN_NOTE: (&str, &str) = ("This ", " is a build Refit does not know: it runs stock");

/// The name of an executable that names nothing else: an image with no module name.
const NO_NAME: &str = "exe";

/// Whether `s` is the identity line every resolver logs first (`engine: <exe> timestamp
/// 0x.., image size 0x.., sections 0x..`, or `engine: <exe> identity unreadable`),
/// whatever the executable's name: for plugins that log it once over several tables.
pub fn is_identity_line(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("engine: ") else { return false };
    let Some(sp) = rest.find(' ') else { return false };
    let tail = &rest[sp + 1..];
    sp > 0 && (tail.starts_with("timestamp 0x") || tail == "identity unreadable")
}

/// A line on the stack, cut at its capacity.
pub(crate) struct Line {
    buf: [u8; 256],
    n: usize,
}

impl Line {
    pub(crate) fn new() -> Line {
        Line { buf: [0; 256], n: 0 }
    }
    pub(crate) fn as_str(&self) -> &str {
        // Only whole `str`s are written, cut at a char boundary.
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.n]) }
    }
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = self.buf.len() - self.n;
        let mut k = s.len().min(room);
        while k > 0 && !s.is_char_boundary(k) {
            k -= 1;
        }
        self.buf[self.n..self.n + k].copy_from_slice(&s.as_bytes()[..k]);
        self.n += k;
        Ok(())
    }
}

/// A table's resolver: the same memory as its `Table` (`Table::engine`).
#[repr(transparent)]
pub struct Engine(Table);

impl Table {
    /// This table's resolver (`init` first; before it, every hook is unknown).
    pub fn engine(&'static self) -> &'static Engine {
        unsafe { &*(self as *const Table as *const Engine) }
    }
}

/// `*` matches any run of characters; the rest literally. Whole id.
pub fn glob(pat: &str, s: &str) -> bool {
    let (p, s) = (pat.as_bytes(), s.as_bytes());
    let (mut i, mut j, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while j < s.len() {
        if i < p.len() && p[i] == b'*' {
            star = i;
            mark = j;
            i += 1;
        } else if i < p.len() && p[i] == s[j] {
            i += 1;
            j += 1;
        } else if star != usize::MAX {
            i = star + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1;
    }
    i == p.len()
}

/// The `Mismatch=` value of an `A2EngineTest.ini`'s `[Test]` section.
pub fn test_mismatch(text: &str) -> Option<&str> {
    let mut sect = false;
    let mut val = None;
    for raw in text.lines() {
        let l = raw.trim();
        if l.is_empty() || l.starts_with(';') || l.starts_with('#') {
            continue;
        }
        if l.starts_with('[') && l.ends_with(']') {
            sect = l[1..l.len() - 1].trim().eq_ignore_ascii_case("test");
            continue;
        }
        if let (true, Some(eq)) = (sect, l.find('=')) {
            if l[..eq].trim().eq_ignore_ascii_case("mismatch") {
                val = Some(l[eq + 1..].trim());
            }
        }
    }
    val.filter(|v| v.split(',').any(|g| !g.trim().is_empty()))
}

/// Starts the resolver of `t` on an image (`mem`, read at RVAs) with an optional
/// `A2EngineTest.ini` text: the host's way in (tests), and what `init` calls.
/// Logs the identity line, the build line, the summary, and the TEST line. Once per
/// table; later calls return the same engine.
pub fn init_with(t: &'static Table, log: fn(&str), mem: &'static dyn Image, test_ini: Option<&str>) -> &'static Engine {
    let rt = t.rt;
    if rt.build.load(Acquire) != 0 {
        return t.engine();
    }
    unsafe {
        *rt.mem.get() = Some(mem);
        *rt.log.get() = Some(log);
    }
    let id = pe::identify(mem);
    // A build is the one whose three numbers match and whose on-disk section CRCs
    // (`[pe] file_crc`, read only for a candidate that sets them) match too; a candidate
    // that sets them is preferred to one that does not.
    let mut crc_line: Option<usize> = None; // the first candidate that sets CRCs
    let mut found: Option<(usize, Identity)> = None;
    if let Some(id) = id {
        let cand = |b: &Build| b.timestamp == id.timestamp && b.image_size == id.image_size && b.sections == id.sections;
        for (i, b) in t.builds.iter().enumerate().filter(|(_, b)| cand(b) && !b.file_crc.is_empty()) {
            crc_line.get_or_insert(i);
            if b.file_crc.iter().all(|f| mem.file_crc(f.section) == Some(f.crc)) {
                found = Some((i, id));
                break;
            }
        }
        if found.is_none() {
            found = t.builds.iter().position(|b| cand(b) && b.file_crc.is_empty()).map(|i| (i, id));
        }
    }
    // The executable's name: the matched build's, else the module's, else NO_NAME.
    let mut module = [0u8; 64];
    let exe = match found.map(|(i, _)| t.builds[i].exe).filter(|e| !e.is_empty()) {
        Some(e) => e,
        None => {
            let n = mem.name(&mut module).min(module.len());
            core::str::from_utf8(&module[..n]).ok().filter(|s| !s.is_empty()).unwrap_or(NO_NAME)
        }
    };
    rt.exe.set(exe);
    let mut note = Line::new();
    let _ = write!(note, "{}{}{}", PLUGIN_NOTE.0, exe, PLUGIN_NOTE.1);
    rt.plugin_note.set(note.as_str());
    let mut line = Line::new();
    match id {
        Some(id) => {
            let _ = write!(line, "engine: {} {}", exe, id);
        }
        None => {
            let _ = write!(line, "engine: {} identity unreadable", exe);
        }
    }
    log(line.as_str());
    if let Some(i) = crc_line {
        let mut line = Line::new();
        let _ = write!(line, "engine: {} file crc:", exe);
        for (k, f) in t.builds[i].file_crc.iter().enumerate() {
            let _ = line.write_str(if k == 0 { " " } else { ", " });
            match mem.file_crc(f.section) {
                Some(c) => {
                    let _ = write!(line, "{} {:#010x}", f.section, c);
                }
                None => {
                    let _ = write!(line, "{} unreadable", f.section);
                }
            }
        }
        log(line.as_str());
    }
    let mut line = Line::new();
    let (mut n, mut h) = match found {
        Some((i, id)) => {
            let b = &t.builds[i];
            rt.delta.store((id.base as u32).wrapping_sub(b.image_base), Relaxed);
            let _ = write!(line, "engine: build {}", b.key);
            (b.n_resolved, b.blocked.len())
        }
        None => {
            let _ = line.write_str("engine: unknown build");
            (t.n_scan, 0)
        }
    };
    log(line.as_str());
    // The companion modules the build's marks describe: another one than profiled makes
    // every hook that depends on its run-time patches stand down.
    if let Some((i, _)) = found {
        let b = &t.builds[i];
        let mut bad = false;
        for m in b.modules {
            let mut line = Line::new();
            match mem.companion(m.name) {
                Companion::Crc(c) if c == m.text_crc => {
                    let _ = write!(line, "engine: companion {} .text crc {:#010x} matches the profile", m.name, c);
                }
                Companion::Crc(c) => {
                    bad = true;
                    let _ = write!(
                        line,
                        "engine: companion {} .text crc {:#010x} is not the profiled {:#010x}: the hooks that depend on it stand down",
                        m.name, c, m.text_crc
                    );
                }
                Companion::NotLoaded => {
                    bad = true;
                    let _ = write!(
                        line,
                        "engine: companion {} is not loaded (profiled .text crc {:#010x}): the hooks that depend on it stand down",
                        m.name, m.text_crc
                    );
                }
                Companion::Unreadable => {
                    bad = true;
                    let _ = write!(
                        line,
                        "engine: companion {} cannot be read (profiled .text crc {:#010x}): the hooks that depend on it stand down",
                        m.name, m.text_crc
                    );
                }
            }
            log(line.as_str());
        }
        if bad {
            rt.companion_bad.store(1, Relaxed);
            n = b.n_resolved_mod;
            h += b.blocked_mod.len();
        }
    }
    if let Some(id) = id {
        rt.code.0.store(id.code.0, Relaxed);
        rt.code.1.store(id.code.1, Relaxed);
        rt.base.store(id.base, Relaxed);
    }
    let mut line = Line::new();
    let _ = write!(line, "engine: symbols: {}/{} resolved; {} hooks blocked", n, t.syms.len(), h);
    log(line.as_str());
    if let Some(globs) = test_ini.and_then(test_mismatch) {
        let mut line = Line::new();
        let _ = line.write_str("engine: TEST: A2EngineTest.ini forces a mismatch for ");
        let mut first = true;
        for g in globs.split(',').map(str::trim).filter(|g| !g.is_empty()) {
            let _ = write!(line, "{}{}", if first { "" } else { ", " }, g);
            first = false;
            for (k, hd) in t.hooks.iter().enumerate() {
                if glob(g, hd.id) {
                    rt.hooks[k].fetch_or(ST_TEST, Relaxed);
                }
            }
        }
        log(line.as_str());
    }
    rt.build.store(found.map(|(i, _)| 2 + i as u32).unwrap_or(1), Release);
    t.engine()
}

/// Starts the resolver of `t` on this process's executable, reading
/// `A2EngineTest.ini` beside it if there is one (the bench's fault injection).
#[cfg(windows)]
pub fn init(t: &'static Table, log: fn(&str)) -> &'static Engine {
    let mut buf = [0u8; 1024];
    let n = win::read_test_ini(&mut buf);
    let text = if n > 0 { core::str::from_utf8(&buf[..n]).ok() } else { None };
    init_with(t, log, &win::PROCESS, text)
}

#[cfg(windows)]
mod win {
    use crate::pe::Image;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleA(name: *const u8) -> usize;
        fn GetModuleFileNameA(m: usize, out: *mut u8, cap: u32) -> u32;
        fn CreateFileA(p: *const u8, access: u32, share: u32, sa: usize, disp: u32, flags: u32, t: usize) -> isize;
        fn ReadFile(h: isize, out: *mut u8, n: u32, got: *mut u32, ov: usize) -> i32;
        fn CloseHandle(h: isize) -> i32;
        fn SetFilePointer(h: isize, lo: i32, hi: *mut i32, method: u32) -> u32;
    }

    /// The CRC-32 of section `name` of the file at `path` (NUL-terminated), read in 4 KiB
    /// steps (`pe::section_crc_file`): the file is never held.
    fn file_crc_of(path: &[u8], name: &str) -> Option<u32> {
        // GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, OPEN_EXISTING
        let h = unsafe { CreateFileA(path.as_ptr(), 0x8000_0000, 3, 0, 3, 0x80, 0) };
        if h == -1 {
            return None;
        }
        let mut read = |at: usize, out: &mut [u8]| {
            if at > i32::MAX as usize || unsafe { SetFilePointer(h, at as i32, core::ptr::null_mut(), 0) } == u32::MAX {
                return false;
            }
            let mut got = 0u32;
            let ok = unsafe { ReadFile(h, out.as_mut_ptr(), out.len() as u32, &mut got, 0) };
            ok != 0 && got as usize == out.len()
        };
        let c = crate::pe::section_crc_file(&mut read, name);
        unsafe { CloseHandle(h) };
        c
    }

    /// Several tables of a plugin ask the same questions: the answers (a CRC of a few
    /// megabytes of file) are kept by the key's 64-bit FNV, in a few racy-but-idempotent
    /// slots. A failure is not kept.
    const SLOTS: usize = 8;
    static KEYS: [core::sync::atomic::AtomicU64; SLOTS] = [const { core::sync::atomic::AtomicU64::new(0) }; SLOTS];
    static VALS: [core::sync::atomic::AtomicU32; SLOTS] = [const { core::sync::atomic::AtomicU32::new(0) }; SLOTS];

    fn cached(parts: &[&[u8]], f: impl FnOnce() -> Option<u32>) -> Option<u32> {
        use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
        let mut key: u64 = 0xcbf2_9ce4_8422_2325;
        for p in parts {
            for &b in p.iter().chain(&[0u8]) {
                key = (key ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        let key = key | 1;
        for i in 0..SLOTS {
            if KEYS[i].load(Acquire) == key {
                return Some(VALS[i].load(Relaxed));
            }
        }
        let v = f()?;
        for i in 0..SLOTS {
            if KEYS[i].load(Relaxed) == 0 {
                VALS[i].store(v, Relaxed);
                KEYS[i].store(key, Release);
                break;
            }
        }
        Some(v)
    }

    /// A NUL-terminated copy of `s` on the stack (names of a section or module are short).
    fn cstr(s: &str, buf: &mut [u8; 32]) -> bool {
        if s.len() >= buf.len() {
            return false;
        }
        buf[..s.len()].copy_from_slice(s.as_bytes());
        buf[s.len()] = 0;
        true
    }

    /// The main module, mapped: reads at RVAs from its base.
    pub struct Process;
    pub static PROCESS: Process = Process;

    impl Image for Process {
        fn read(&self, at: usize, out: &mut [u8]) -> bool {
            let base = self.base();
            if base == 0 {
                return false;
            }
            // Only offsets inside the image are read: the headers' own, and entries the
            // generator checked against the build's image (lint rule 1), on that build.
            unsafe { core::ptr::copy_nonoverlapping((base + at) as *const u8, out.as_mut_ptr(), out.len()) };
            true
        }
        fn base(&self) -> usize {
            unsafe { GetModuleHandleA(core::ptr::null()) }
        }
        fn file_crc(&self, section: &str) -> Option<u32> {
            cached(&[b"exe", section.as_bytes()], || {
                let mut path = [0u8; 300];
                let n = unsafe { GetModuleFileNameA(0, path.as_mut_ptr(), 260) } as usize;
                if n == 0 || n >= 260 {
                    return None;
                }
                file_crc_of(&path[..n + 1], section)
            })
        }
        fn companion(&self, name: &str) -> crate::pe::Companion {
            use crate::pe::Companion;
            let mut c = [0u8; 32];
            if !cstr(name, &mut c) {
                return Companion::Unreadable;
            }
            let m = unsafe { GetModuleHandleA(c.as_ptr()) };
            if m == 0 {
                return Companion::NotLoaded;
            }
            // Its file, not its image: the image may be relocated or patched by then.
            let crc = cached(&[b"mod", name.as_bytes()], || {
                let mut path = [0u8; 300];
                let n = unsafe { GetModuleFileNameA(m, path.as_mut_ptr(), 260) } as usize;
                if n == 0 || n >= 260 {
                    return None;
                }
                file_crc_of(&path[..n + 1], ".text")
            });
            crc.map_or(Companion::Unreadable, Companion::Crc)
        }
        fn name(&self, out: &mut [u8]) -> usize {
            let mut path = [0u8; 300];
            let n = unsafe { GetModuleFileNameA(0, path.as_mut_ptr(), 260) } as usize;
            let n = n.min(path.len());
            let from = path[..n].iter().rposition(|&c| c == b'\\' || c == b'/').map_or(0, |i| i + 1);
            let k = (n - from).min(out.len());
            out[..k].copy_from_slice(&path[from..from + k]);
            k
        }
    }

    /// `A2EngineTest.ini` in the exe's directory, into `buf`: its length, 0 if none.
    pub fn read_test_ini(buf: &mut [u8]) -> usize {
        let mut path = [0u8; 300];
        let n = unsafe { GetModuleFileNameA(0, path.as_mut_ptr(), 260) } as usize;
        let Some(slash) = path[..n].iter().rposition(|&c| c == b'\\' || c == b'/') else { return 0 };
        let name = b"A2EngineTest.ini\0";
        if slash + 1 + name.len() > path.len() {
            return 0;
        }
        path[slash + 1..slash + 1 + name.len()].copy_from_slice(name);
        // GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING
        let h = unsafe { CreateFileA(path.as_ptr(), 0x8000_0000, 1, 0, 3, 0x80, 0) };
        if h == -1 {
            return 0;
        }
        let mut got = 0u32;
        let ok = unsafe { ReadFile(h, buf.as_mut_ptr(), buf.len() as u32, &mut got, 0) };
        unsafe { CloseHandle(h) };
        if ok == 0 { 0 } else { got as usize }
    }
}

impl Engine {
    fn t(&self) -> &'static Table {
        unsafe { &*(self as *const Engine as *const Table) }
    }

    fn log(&self, s: &str) {
        if let Some(f) = unsafe { *self.t().rt.log.get() } {
            f(s);
        }
    }

    fn mem(&self) -> Option<&'static dyn Image> {
        unsafe { *self.t().rt.mem.get() }
    }

    fn build_index(&self) -> Option<usize> {
        match self.t().rt.build.load(Acquire) {
            0 | 1 => None,
            b => Some(b as usize - 2),
        }
    }

    /// The executable's name as logged: the matched build's (`Armada2.exe`), else the
    /// module's own file name; "" before `init`.
    pub fn exe(&self) -> &'static str {
        self.t().rt.exe.get().unwrap_or("")
    }

    /// The table's game (`armada2`), its profiles' `game`.
    pub fn game(&self) -> &'static str {
        self.t().game
    }

    /// `armada2/gog-1.1`, or None on an unknown build (or before `init`).
    pub fn build(&self) -> Option<&'static str> {
        self.build_index().map(|i| self.t().builds[i].key)
    }

    /// The content set this plugin reads in this build (phase 8f): the id its content
    /// tables are chosen by. None on an unknown build, before `init`, and in a build whose
    /// profile gives the plugin no set: the plugin then has no built-in names and shows
    /// the engine's own look.
    pub fn content(&self) -> Option<&'static str> {
        self.build_index().map(|i| self.t().builds[i].content).filter(|c| !c.is_empty())
    }

    /// The identity's first executable section, as mapped: replaces `EXE_CODE`.
    pub fn code(&self) -> (usize, usize) {
        let rt = self.t().rt;
        (rt.code.0.load(Relaxed), rt.code.1.load(Relaxed))
    }

    fn entries(&self) -> &'static [Entry] {
        let t = self.t();
        &t.builds[self.build_index().unwrap_or(0)].entries
    }

    fn read(&self, va: u32, out: &mut [u8]) -> bool {
        let Some(m) = self.mem() else { return false };
        let base = self.t().rt.base.load(Relaxed) as u32;
        let Some(rva) = va.checked_sub(base) else { return false };
        m.read(rva as usize, out)
    }

    fn u32_at(&self, va: u32) -> Option<u32> {
        let mut b = [0u8; 4];
        self.read(va, &mut b).then(|| u32::from_le_bytes(b))
    }

    fn in_code(&self, a: u32) -> bool {
        let (lo, hi) = self.code();
        (a as usize) >= lo && (a as usize) < hi
    }

    /// Entry k's address or value, unchecked; Err(the failing symbol, why).
    fn resolve(&self, k: usize) -> Result<u32, (u16, u64, u64)> {
        let t = self.t();
        let e = &self.entries()[k];
        if !e.present {
            return Err((k as u16, W_ABSENT, 0));
        }
        let delta = t.rt.delta.load(Relaxed);
        Ok(match t.syms[k].kind {
            Kind::Field | Kind::Value => e.value as u32,
            Kind::Scan => return self.scan(k, e),
            Kind::Slot => self.resolve(e.vtable as usize)?.wrapping_add(4 * e.index as u32),
            _ => e.addr.wrapping_add(delta),
        })
    }

    fn scan(&self, k: usize, e: &Entry) -> Result<u32, (u16, u64, u64)> {
        let mut hits = 0u32;
        let mut first = 0u32;
        let found = self.mem().and_then(|m| pe::section(m, e.section));
        if let (Some((rva, size)), false) = (found, e.check.is_empty()) {
            let base = self.t().rt.base.load(Relaxed) as u32;
            let n = e.check.len();
            let mut chunk = [0u8; 4096 + 32];
            let mut at = 0usize;
            while at + n <= size {
                let take = (size - at).min(chunk.len());
                if !self.read(base.wrapping_add((rva + at) as u32), &mut chunk[..take]) {
                    break;
                }
                let last = take - n; // positions 0..=last fit in this chunk
                for i in 0..=last {
                    if (0..n).all(|j| e.mask[j] == 0 || chunk[i + j] == e.check[j]) {
                        if hits == 0 {
                            first = base.wrapping_add((rva + at + i) as u32);
                        }
                        hits += 1;
                    }
                }
                at += last + 1;
            }
        }
        if hits != e.count as u32 {
            return Err((k as u16, W_FOUND, hits.min(0xffff) as u64));
        }
        Ok(first)
    }

    /// None if entry k holds in memory; else (failing symbol, why, aux).
    fn check(&self, h: &HookDef, k: usize, out: &mut u32, emit: bool) -> Option<(u16, u64, u64)> {
        let t = self.t();
        let a = match self.resolve(k) {
            Ok(a) => a,
            Err(e) => return Some(e),
        };
        *out = a;
        let e = &self.entries()[k];
        let sd = &t.syms[k];
        match sd.kind {
            Kind::Fn | Kind::Label | Kind::Patch if !e.check.is_empty() => {
                let mut b = [0u8; 32];
                let n = e.check.len().min(32);
                if !self.read(a, &mut b[..n]) || b[..n] != e.check[..n] {
                    return Some((k as u16, W_NOT_STOCK, 0));
                }
            }
            Kind::Call | Kind::Jmp => {
                let ta = match self.resolve(e.target as usize) {
                    Ok(v) => v,
                    Err(e) => return Some(e),
                };
                let mut b = [0u8; 5];
                let op = if sd.kind == Kind::Call { 0xe8 } else { 0xe9 };
                if !self.read(a, &mut b) || b[0] != op {
                    return Some((k as u16, W_CALLS, 0));
                }
                let dest = a.wrapping_add(5).wrapping_add(u32::from_le_bytes([b[1], b[2], b[3], b[4]]));
                if dest != ta {
                    if e.chain && !self.in_code(dest) {
                        if !emit {
                            return None;
                        }
                        let mut l = Line::new();
                        let _ = write!(l, "engine: {}: {} at {:#x} calls {:#x} (chained)", h.id, sd.symbol, a, dest);
                        self.log(l.as_str());
                    } else {
                        return Some((k as u16, W_CALLS, 0));
                    }
                }
            }
            Kind::Slot => {
                let Some(v) = self.u32_at(a) else { return Some((k as u16, W_CALLS, 0)) };
                if e.target != NONE {
                    let ta = match self.resolve(e.target as usize) {
                        Ok(v) => v,
                        Err(e) => return Some(e),
                    };
                    if v != ta {
                        if e.chain && !self.in_code(v) {
                            if !emit {
                                return None;
                            }
                            let mut l = Line::new();
                            let _ = write!(l, "engine: {}: {} at {:#x} holds {:#x} (chained)", h.id, sd.symbol, a, v);
                            self.log(l.as_str());
                        } else {
                            return Some((k as u16, W_CALLS, 0));
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// The build's block `i`: of `blocked` (W_BLOCKED) or of `blocked_mod` (W_MODULE).
    fn block(&self, why: u64, i: usize) -> &'static Block {
        let b = &self.t().builds[self.build_index().unwrap_or(0)];
        if why == W_MODULE { &b.blocked_mod[i] } else { &b.blocked[i] }
    }

    fn why_of(&self, st: u64) -> StandDown {
        let t = self.t();
        let why = (st >> 16) & 0xff;
        let sym = ((st >> 24) & 0xffff) as usize;
        let aux = ((st >> 40) & 0xffff) as usize;
        let symbol = t.syms.get(sym).map(|s| s.symbol).unwrap_or("");
        let bname = t.builds[self.build_index().unwrap_or(0)].name;
        let w = match why {
            W_NOT_STOCK => Why::NotStock,
            W_CALLS => Why::CallsElse,
            W_BLOCKED | W_MODULE => {
                let blk = self.block(why, aux);
                return StandDown { hook: "", symbol: blk.symbol, why: Why::Blocked(blk.why) };
            }
            W_TEST => Why::Test,
            W_ABSENT => Why::Absent(bname),
            W_FOUND => Why::Found(aux as u16, self.entries()[sym].count),
            _ => Why::Unknown,
        };
        StandDown { hook: "", symbol, why: w }
    }

    /// Whether hook `h` may write: checked once (every entry, in live memory, before any
    /// failure is reported), then cached. A failure is logged once,
    /// `engine: <hook> stands down: <symbol> <why>`.
    pub fn ready(&self, h: HookId) -> Result<(), StandDown> {
        let t = self.t();
        let hk = h.0 as usize;
        let Some(hd) = t.hooks.get(hk) else {
            return Err(StandDown { hook: "?", symbol: "", why: Why::Unknown });
        };
        let slot = &t.rt.hooks[hk];
        let st = slot.load(Acquire);
        match st & 0xff {
            ST_READY => return Ok(()),
            ST_FAILED => return Err(StandDown { hook: hd.id, ..self.why_of(st) }),
            _ => {}
        }
        let sym0 = hd.syms.first().copied().unwrap_or(NONE) as u64;
        let bi = self.build_index();
        let mut fail: Option<(u64, u64, u64)> = None; // (sym, why, aux)
        if bi.is_none() && !hd.run_unknown {
            fail = Some((sym0, W_UNKNOWN, 0));
        } else if let Some(b) = bi.map(|i| &t.builds[i]) {
            if let Some(n) = b.blocked.iter().position(|x| x.hook as usize == hk) {
                fail = Some((sym0, W_BLOCKED, n as u64));
            } else if t.rt.companion_bad.load(Relaxed) != 0 {
                if let Some(n) = b.blocked_mod.iter().position(|x| x.hook as usize == hk) {
                    fail = Some((sym0, W_MODULE, n as u64));
                }
            }
        }
        if fail.is_none() && st & ST_TEST != 0 {
            fail = Some((sym0, W_TEST, 0));
        }
        let mut got = [0u32; 64];
        if fail.is_none() {
            for (n, &k) in hd.syms.iter().enumerate() {
                let mut a = 0;
                if let Some((s, w, x)) = self.check(hd, k as usize, &mut a, false) {
                    if fail.is_none() {
                        fail = Some((s as u64, w, x));
                    }
                }
                if n < got.len() {
                    got[n] = a;
                } else {
                    t.rt.addr[k as usize].store(a as usize, Relaxed);
                }
            }
        }
        if let Some((s, w, x)) = fail {
            let packed = ST_FAILED | (st & ST_TEST) | (w << 16) | ((s & 0xffff) << 24) | ((x & 0xffff) << 40);
            let sd = StandDown { hook: hd.id, ..self.why_of(packed) };
            let mut l = Line::new();
            let _ = write!(l, "engine: {} stands down: {}", hd.id, sd);
            self.log(l.as_str());
            let mut note = Line::new();
            match sd.why {
                Why::Unknown => {
                    let _ = note.write_str(self.note_plugin_text());
                }
                Why::Test => {
                    let _ = write!(note, "Stands down: {}: test mismatch (A2EngineTest.ini)", sd.symbol);
                }
                Why::Blocked(_) => {
                    let _ = note.write_str(self.block((packed >> 16) & 0xff, x as usize).note);
                }
                _ => {
                    let _ = write!(note, "Stands down: {} is not stock in this {}", sd.symbol, self.exe());
                }
            }
            t.rt.notes[hk].set(note.as_str());
            slot.store(packed, Release);
            return Err(sd);
        }
        // The chained sites are logged once the hook is known ready (the same checks again,
        // which pass: memory the plugin has not written yet).
        for &k in hd.syms {
            let mut a = 0;
            let _ = self.check(hd, k as usize, &mut a, true);
        }
        for (n, &k) in hd.syms.iter().enumerate() {
            if n < got.len() {
                t.rt.addr[k as usize].store(got[n] as usize, Relaxed);
            }
            t.rt.flags[k as usize].fetch_or(READY, Release);
        }
        slot.store(ST_READY | (st & ST_TEST), Release);
        Ok(())
    }

    /// Whether a hook that uses the symbol is ready, without the warning `addr` logs when
    /// it is not: for a read that may come before the hook does (a settings load).
    pub fn resolved(&self, s: Sym) -> bool {
        self.t().rt.flags.get(s.0 as usize).is_some_and(|f| f.load(Acquire) & READY != 0)
    }

    /// The symbol's address (relocated), offset or value: 0 unless a hook that uses it
    /// is ready (logged once: `engine: <id> read before its hook was ready`).
    pub fn addr(&self, s: Sym) -> usize {
        let t = self.t();
        let k = s.0 as usize;
        let Some(f) = t.rt.flags.get(k) else { return 0 };
        if f.load(Acquire) & READY != 0 {
            return t.rt.addr[k].load(Relaxed);
        }
        if f.fetch_or(WARNED, Relaxed) & WARNED == 0 {
            let mut l = Line::new();
            let _ = write!(l, "engine: {} read before its hook was ready", t.syms[k].id);
            self.log(l.as_str());
        }
        0
    }

    /// A `field`'s offset in this build (README.md, "Fields and values"): what the
    /// generated `field::X()` returns. On an unknown build, and before `init`, the root
    /// build's (only scan hooks run there, so no hook of ours reads it); 0 when this build
    /// has no such field (every hook that uses it stands down). Not tied to a hook's
    /// readiness, so a read in a hot path costs one load and an index.
    #[inline]
    pub fn field(&self, s: Sym) -> usize {
        self.value(s) as usize
    }

    /// A `value`'s value in this build, as `field` (the generated `value::X()`).
    #[inline]
    pub fn value(&self, s: Sym) -> i32 {
        let t = self.t();
        let b = match t.rt.build.load(Acquire) {
            0 | 1 => 0,
            b => b as usize - 2,
        };
        match t.builds.get(b).and_then(|b| b.entries.get(s.0 as usize)) {
            Some(e) if e.present => e.value,
            _ => 0,
        }
    }

    /// A `fn` with an `end`: its range, relocated; (0, 0) unless ready.
    pub fn range(&self, s: Sym) -> (usize, usize) {
        let a = self.addr(s);
        if a == 0 {
            return (0, 0);
        }
        let e = &self.entries()[s.0 as usize];
        let end = if e.end != 0 { e.end.wrapping_add(self.t().rt.delta.load(Relaxed)) as usize } else { a };
        (a, end)
    }

    /// (symbol, site, expected target) for render3d's `core_hooks::Site`: the target of
    /// a `call`/`jmp`, the target of a `slot`; (symbol, 0, 0) unless ready.
    pub fn site(&self, s: Sym) -> (&'static str, usize, usize) {
        let t = self.t();
        let k = s.0 as usize;
        let symbol = t.syms.get(k).map(|d| d.symbol).unwrap_or("");
        let a = self.addr(s);
        if a == 0 {
            return (symbol, 0, 0);
        }
        let e = &self.entries()[k];
        let expect = match t.syms[k].kind {
            Kind::Call | Kind::Jmp | Kind::Slot if e.target != NONE => self.resolve(e.target as usize).unwrap_or(0) as usize,
            _ => 0,
        };
        (symbol, a, expect)
    }

    /// The SettingsNote text of a hook that stood down.
    pub fn note(&self, h: HookId) -> Option<&'static str> {
        let t = self.t();
        let k = h.0 as usize;
        if t.rt.hooks.get(k)?.load(Acquire) & 0xff != ST_FAILED {
            return None;
        }
        t.rt.notes[k].get()
    }

    /// The note of the first hook feeding the row `section.key` that stood down; the
    /// plugin's note on an unknown build.
    pub fn note_for(&self, section: &str, key: &str) -> Option<&'static str> {
        let t = self.t();
        for (k, hd) in t.hooks.iter().enumerate() {
            let feeds = hd.rows.iter().any(|r| match r.find('.') {
                Some(d) => &r[..d] == section && &r[d + 1..] == key,
                None => false,
            });
            if feeds {
                if let Some(n) = self.note(HookId(k as u16)) {
                    return Some(n);
                }
            }
        }
        self.note_plugin()
    }

    /// `This <exe> is a build Refit does not know: it runs stock` on an unknown build,
    /// else None.
    pub fn note_plugin(&self) -> Option<&'static str> {
        if self.t().rt.build.load(Acquire) == 1 { Some(self.note_plugin_text()) } else { None }
    }

    fn note_plugin_text(&self) -> &'static str {
        self.t().rt.plugin_note.get().unwrap_or("")
    }
}
