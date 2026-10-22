//! The tracer: every call the engine (or a plugin) makes on an object of ours, counted
//! by context, interface, method and caller (README.md, "The inventory").
//!
//! - **Callers.** The return address of each call, and for every method but the state
//!   setters a few more return addresses into `Armada2.exe` found up the stack: the
//!   engine is built without frame pointers, so its stack is scanned for values that
//!   point just past a `call` instruction in its code. The first is exact; those after
//!   it are the usual heuristic (a stale return address in an uninitialised local
//!   passes the test too). Addresses are written as they are, and named offline.
//! - **Contexts.** A label the bench sets by writing it into `Display8.mark` beside the
//!   exe (`a2test`'s step `Mark the display context "NAME"`), read at most four times a
//!   second at `Present` and at every `CreateDevice` and `Reset`. Before the first mark
//!   the context is `start`.
//! - **Output.** `Display8.inventory.tsv` beside the exe, rewritten every two seconds of
//!   presenting and at each mark: one row per (context, interface, method, callers),
//!   with the calls, the frames they fell in and whether any came from a thread other
//!   than the device's. Answers and arguments worth seeing once (caps, formats, modes,
//!   present parameters, gamma ramps, locks, the plugins' queries) go into
//!   `Display8.log`, each distinct line once.
use crate::com::{Guid, HRESULT, Kind, Obj};
use crate::rt::win::*;
use crate::{log, sfmt};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub static ON: AtomicBool = AtomicBool::new(false);
/// The thread the device was made on; a call from any other is flagged.
pub static MAIN_THREAD: AtomicU32 = AtomicU32::new(0);

/// Return addresses kept per call: the immediate one and up to five in the engine.
pub const DEPTH: usize = 6;

struct Entry {
    ctx: u16,
    kind: Kind,
    slot: u8,
    chain: [usize; DEPTH],
    calls: u64,
    frames: u32,
    last_frame: u32,
    other_thread: u32,
}

struct State {
    /// Open addressing over `entries`: index + 1, 0 empty.
    slots: Vec<u32>,
    entries: Vec<Entry>,
    contexts: Vec<String>,
    ctx: u16,
    frame: u32,
    once: BTreeSet<u64>,
    modules: Vec<(usize, usize, String)>,
    exe: (usize, usize),
    last_mark_check: u32,
    last_write: u32,
    /// Per context: frames presented, engine draws, time in `Present` (s), its max.
    present: Vec<(u32, u64, f64, f64)>,
}

static LOCK: AtomicBool = AtomicBool::new(false);
static mut STATE: Option<State> = None;

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        LOCK.store(false, Ordering::Release);
    }
}

fn lock() -> Guard {
    while LOCK.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    Guard
}

#[allow(static_mut_refs)]
fn state() -> &'static mut State {
    unsafe { STATE.as_mut().expect("trace not started") }
}

/// Starts the tracer: finds `Armada2.exe`'s code in memory.
pub fn start() {
    let (code, end) = exe_range();
    let (base, size) = (code - 0x1000, end - code + 0x1000);
    let mut contexts = Vec::new();
    contexts.push(String::from("start"));
    unsafe {
        STATE = Some(State {
            slots: alloc::vec![0; 1 << 14],
            entries: Vec::new(),
            contexts,
            ctx: 0,
            frame: 0,
            once: BTreeSet::new(),
            modules: Vec::new(),
            exe: (code, end),
            last_mark_check: 0,
            last_write: 0,
            present: alloc::vec![(0, 0, 0.0, 0.0)],
        })
    };
    ON.store(true, Ordering::Relaxed);
    log!("trace: on, Armada2.exe at {:#x}..{:#x}", base, base + size);
}

/// `Armada2.exe`'s image in memory, past its headers: where its code is.
pub fn exe_range() -> (usize, usize) {
    let base = unsafe { GetModuleHandleA(core::ptr::null()) } as usize;
    let size = unsafe {
        let nt = base + *((base + 0x3c) as *const u32) as usize;
        *((nt + 0x50) as *const u32) as usize // OptionalHeader.SizeOfImage
    };
    (base + 0x1000, base + size)
}

/// State setters and getters, called thousands of times a frame: only their
/// immediate caller is kept.
fn shallow(kind: Kind, slot: u8) -> bool {
    kind == Kind::Device
        && matches!(slot, 1 | 2 | 37 | 38 | 42 | 43 | 50 | 51 | 61 | 62 | 63 | 76 | 77 | 79 | 83 | 85 | 88 | 91)
}

pub fn looks_like_return(exe: (usize, usize), ra: usize) -> bool {
    if ra < exe.0 + 8 || ra >= exe.1 {
        return false;
    }
    let b = |n: usize| unsafe { *((ra - n) as *const u8) };
    if b(5) == 0xe8 {
        return true;
    }
    // FF /2: an indirect call, 2, 3, 4, 6 or 7 bytes long.
    [2usize, 3, 4, 6, 7].iter().any(|&n| b(n) == 0xff && (b(n - 1) >> 3) & 7 == 2)
}

fn stack_base() -> usize {
    let v: usize;
    unsafe { core::arch::asm!("mov {}, fs:[4]", out(reg) v, options(nostack, readonly, preserves_flags)) };
    v
}

pub fn walk(fp: usize, exe: (usize, usize), deep: bool) -> [usize; DEPTH] {
    let mut c = [0usize; DEPTH];
    if fp == 0 {
        return c;
    }
    c[0] = unsafe { *((fp + 4) as *const usize) };
    if !deep {
        return c;
    }
    let top = stack_base().min(fp + 8 + 16384);
    let mut p = fp + 8;
    let mut n = 1;
    while p + 4 <= top && n < DEPTH {
        let v = unsafe { *(p as *const usize) };
        if looks_like_return(exe, v) && v != c[n - 1] {
            c[n] = v;
            n += 1;
        }
        p += 4;
    }
    c
}

fn hash(ctx: u16, kind: Kind, slot: u8, chain: &[usize; DEPTH]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    let mut mix = |v: u32| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0193);
    };
    mix(ctx as u32);
    mix(kind as u32);
    mix(slot as u32);
    for &a in chain {
        mix(a as u32);
    }
    h
}

/// One call on an object of ours. `fp` is the method's own frame pointer.
pub fn call(fp: usize, kind: Kind, slot: u8) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let exe = unsafe { STATE.as_ref().map(|s| s.exe).unwrap_or((0, 0)) };
    let chain = walk(fp, exe, !shallow(kind, slot));
    let other = {
        let m = MAIN_THREAD.load(Ordering::Relaxed);
        m != 0 && unsafe { GetCurrentThreadId() } != m
    };
    let _g = lock();
    let s = state();
    let ctx = s.ctx;
    let h = hash(ctx, kind, slot, &chain);
    let mask = s.slots.len() - 1;
    let mut i = h as usize & mask;
    let idx = loop {
        let e = s.slots[i];
        if e == 0 {
            s.entries.push(Entry { ctx, kind, slot, chain, calls: 0, frames: 0, last_frame: u32::MAX, other_thread: 0 });
            s.slots[i] = s.entries.len() as u32;
            if s.entries.len() * 2 > s.slots.len() {
                grow(s);
            }
            break s.entries.len() - 1;
        }
        let en = &s.entries[e as usize - 1];
        if en.ctx == ctx && en.kind == kind && en.slot == slot && en.chain == chain {
            break e as usize - 1;
        }
        i = (i + 1) & mask;
    };
    let frame = s.frame;
    let e = &mut s.entries[idx];
    e.calls += 1;
    if e.last_frame != frame {
        e.last_frame = frame;
        e.frames += 1;
    }
    if other {
        e.other_thread += 1;
    }
    if kind == Kind::Device && (70..=73).contains(&slot) && in_range(s.exe, chain[0]) {
        s.present[ctx as usize].1 += 1;
    }
}

fn in_range(r: (usize, usize), a: usize) -> bool {
    a >= r.0 && a < r.1
}

fn grow(s: &mut State) {
    let n = s.slots.len() * 2;
    s.slots = alloc::vec![0; n];
    for (k, e) in s.entries.iter().enumerate() {
        let mut i = hash(e.ctx, e.kind, e.slot, &e.chain) as usize & (n - 1);
        while s.slots[i] != 0 {
            i = (i + 1) & (n - 1);
        }
        s.slots[i] = k as u32 + 1;
    }
}

fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A line for the log, written the first time it is seen in a context.
pub fn once(line: &str) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let (ctxname, new) = {
        let _g = lock();
        let s = state();
        let name = s.contexts[s.ctx as usize].clone();
        let h = fnv64(&name) ^ fnv64(line).rotate_left(1);
        (name, s.once.insert(h))
    };
    if new {
        log!("[{}] {}", ctxname, line);
    }
}

/// The module an address is in, and the address within it, as text: an address in
/// `Armada2.exe` as it is (the map names it), any other as `Name.dll+offset`.
pub fn place(a: usize) -> String {
    if a == 0 {
        return String::from("-");
    }
    let s = state();
    if in_range(s.exe, a) {
        return sfmt!("{:#x}", a);
    }
    for (b, n, name) in &s.modules {
        if a >= *b && a < *b + *n {
            return sfmt!("{}+{:#x}", name, a - b);
        }
    }
    let mut mi = MemInfo::default();
    if unsafe { VirtualQuery(a as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 || mi.alloc_base == 0 {
        return sfmt!("?{:#x}", a);
    }
    let base = mi.alloc_base;
    let mut buf = [0u8; 300];
    let n = unsafe { GetModuleFileNameA(base as HANDLE, buf.as_mut_ptr(), 300) } as usize;
    let full = core::str::from_utf8(&buf[..n]).unwrap_or("?");
    let name = String::from(full.rsplit(['\\', '/']).next().unwrap_or(full));
    let size = unsafe {
        if n > 0 && *(base as *const u16) == 0x5a4d {
            let nt = base + *((base + 0x3c) as *const u32) as usize;
            *((nt + 0x50) as *const u32) as usize
        } else {
            mi.size
        }
    };
    s.modules.push((base, size, name.clone()));
    sfmt!("{}+{:#x}", name, a - base)
}

/// The immediate caller of the method whose frame pointer is `fp`, as text.
pub fn caller(fp: usize) -> String {
    if !ON.load(Ordering::Relaxed) || fp == 0 {
        return String::new();
    }
    let a = unsafe { *((fp + 4) as *const usize) };
    let _g = lock();
    place(a)
}

/// The callers of the method whose frame pointer is `fp`, as text (deep).
pub fn chain(fp: usize) -> String {
    if !ON.load(Ordering::Relaxed) || fp == 0 {
        return String::new();
    }
    let exe = state().exe;
    let c = walk(fp, exe, true);
    let _g = lock();
    let mut t = String::new();
    for (i, &a) in c.iter().enumerate() {
        if a == 0 {
            break;
        }
        if i > 0 {
            t.push(' ');
        }
        t.push_str(&place(a));
    }
    t
}

/// A `QueryInterface` for something not ours: a plugin reaching Direct3D 9 through
/// an engine object (the mirror table of phase 0, item 4).
pub fn query_seen(fp: usize, this: *mut Obj, iid: &Guid, hr: HRESULT) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let o = unsafe { &*this };
    once(&sfmt!(
        "QI {}{} {} for {} -> {:#x} by {}",
        o.kind.name(),
        o.role.name(),
        crate::d3d8::describe(this),
        iid,
        hr as u32,
        caller(fp)
    ));
}

/// The context's index, made if new.
fn context_index(s: &mut State, name: &str) -> u16 {
    if let Some(i) = s.contexts.iter().position(|c| c == name) {
        return i as u16;
    }
    s.contexts.push(String::from(name));
    s.present.push((0, 0, 0.0, 0.0));
    (s.contexts.len() - 1) as u16
}

/// Reads `Display8.mark`, if the bench left one, and switches context. `force` reads
/// it whatever the time since the last look.
pub fn check_mark(force: bool) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let now = unsafe { GetTickCount() };
    {
        let _g = lock();
        let s = state();
        if !force && now.wrapping_sub(s.last_mark_check) < 250 {
            return;
        }
        s.last_mark_check = now;
    }
    let path = crate::rt::game_path("Display8.mark");
    let Some(bytes) = crate::rt::read_file(&path) else { return };
    crate::rt::delete_file(&path);
    let name = String::from(String::from_utf8_lossy(&bytes).trim());
    if name.is_empty() {
        return;
    }
    let frame = {
        let _g = lock();
        let s = state();
        s.ctx = context_index(s, &name);
        s.frame
    };
    log!("trace: context \"{}\" from frame {}", name, frame);
    write_inventory();
}

/// The end of a presented frame: `secs` spent in the real `Present`.
pub fn frame_end(secs: f64) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let now = unsafe { GetTickCount() };
    let (write, stats) = {
        let _g = lock();
        let s = state();
        s.frame = s.frame.wrapping_add(1);
        let p = &mut s.present[s.ctx as usize];
        p.0 += 1;
        p.2 += secs;
        if secs > p.3 {
            p.3 = secs;
        }
        let write = now.wrapping_sub(s.last_write) >= 2000;
        if write {
            s.last_write = now;
        }
        (write, s.frame % 600 == 0)
    };
    if stats {
        let _g = lock();
        let s = state();
        let c = s.ctx as usize;
        let p = s.present[c];
        log!(
            "trace: frame {} in \"{}\": {} frames there, {} engine draws, Present {:.2} ms mean, {:.2} ms max",
            s.frame,
            s.contexts[c],
            p.0,
            p.1,
            if p.0 > 0 { p.2 * 1000.0 / p.0 as f64 } else { 0.0 },
            p.3 * 1000.0
        );
    }
    check_mark(false);
    if write {
        write_inventory();
    }
}

/// Rewrites `Display8.inventory.tsv`.
pub fn write_inventory() {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let mut t = String::new();
    {
        let _g = lock();
        let s = state();
        t.push_str("context\tinterface\tmethod\tcalls\tframes\tother_thread\tcaller\tchain\n");
        let mut order: Vec<usize> = (0..s.entries.len()).collect();
        order.sort_by(|&a, &b| {
            let (x, y) = (&s.entries[a], &s.entries[b]);
            (x.ctx, x.kind, x.slot, core::cmp::Reverse(x.calls)).cmp(&(y.ctx, y.kind, y.slot, core::cmp::Reverse(y.calls)))
        });
        for i in order {
            let (ctx, kind, slot, chain, calls, frames, other) = {
                let e = &s.entries[i];
                (e.ctx, e.kind, e.slot, e.chain, e.calls, e.frames, e.other_thread)
            };
            let _ = write!(
                t,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t",
                s.contexts[ctx as usize],
                kind.name(),
                crate::d3d8::method_name(kind, slot),
                calls,
                frames,
                other,
                place(chain[0])
            );
            for (k, &a) in chain.iter().enumerate().skip(1) {
                if a == 0 {
                    break;
                }
                if k > 1 {
                    t.push(' ');
                }
                t.push_str(&place(a));
            }
            t.push('\n');
        }
        t.push_str("#presents\n");
        for (i, c) in s.contexts.iter().enumerate() {
            let p = s.present[i];
            let _ = writeln!(t, "#{}\t{}\t{}\t{:.3}\t{:.3}", c, p.0, p.1, if p.0 > 0 { p.2 * 1000.0 / p.0 as f64 } else { 0.0 }, p.3 * 1000.0);
        }
    }
    let path = crate::rt::game_path("Display8.inventory.tsv");
    let tmp = crate::rt::game_path("Display8.inventory.tmp");
    if crate::rt::write_file(&tmp, t.as_bytes()) {
        crate::rt::move_file(&tmp, &path);
    }
}

/// Raw bytes as hex, for answers logged whole (the caps).
pub fn hex(p: *const c_void, n: usize) -> String {
    let mut t = String::new();
    let b = unsafe { core::slice::from_raw_parts(p as *const u8, n) };
    for (i, w) in b.chunks(4).enumerate() {
        if i > 0 {
            t.push(' ');
        }
        let v = u32::from_le_bytes([w[0], *w.get(1).unwrap_or(&0), *w.get(2).unwrap_or(&0), *w.get(3).unwrap_or(&0)]);
        let _ = write!(t, "{:08x}", v);
    }
    t
}
