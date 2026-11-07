//! The crash catcher (`[Crash]` in `Display8.ini`; README.md, "The crash catcher") and the
//! address-space log.
//!
//! **Why it exists.** An exception in the engine reaches the handler of
//! `DebugException_Execute`, which the main loop runs inside. That handler walks the call
//! stack with `DebugCallStack_DumpFrom` from the frame pointer the exception left; under Wine
//! that walk faults again, inside the handler, and the nested fault hits a handler pointer
//! the damage had overwritten, so the handler is entered again and again, never writes
//! `exception.txt`, never exits, and the original exception is lost (2026-11-06, the white
//! screen entering a third mission). Two parts:
//!
//! - a vectored exception handler, which sees every exception first and writes one record
//!   into `Display8.log` for the first ones of the fatal classes (code, address as
//!   module+offset, flags, registers, 64 stack words, the return addresses in them and a
//!   frame-pointer walk that checks every read); it knows nothing of the engine. It is a
//!   first-chance handler: an exception the game then handles itself would be logged as
//!   well, and says so ("first chance");
//! - the one call of `DebugCallStack_DumpFrom` in the engine's handler, re-pointed to
//!   [`dump_from`], which does not walk: it logs that the engine's handler was reached, the
//!   address space, and ends the process with `TerminateProcess` (the log is written
//!   unbuffered; `ExitProcess` would run detach code of a process whose state is damaged).
//!   Only this part needs the engine (`display.crash` in the profile); a build that does not
//!   resolve it gets neither part.
//!
//! A nested-exception guard keeps the handler itself from looping: an exception on the thread
//! that is writing a record ends the process at once, and so does the 33rd fatal exception.
//!
//! **The address space** (`address_space`): the largest free block, the free total, what is
//! committed and reserved, found by walking `VirtualQuery`. Logged at every device creation
//! and reset, in each crash record, and at a `Present` when the committed total has moved by
//! 64 MB since the last line or a minute has gone by (a mission load moves it by hundreds),
//! which tells a 32-bit address space running out from damage.
use crate::engine_syms::{hook, sym};
use crate::rt::win::*;
use crate::rt::{hook as patch, Ini};
use crate::settings_table::CRASH_CATCH;
use crate::{log, rt};
use core::fmt::Write;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: unsafe extern "system" fn(*mut ExcPointers) -> i32) -> *mut c_void;
    fn TerminateProcess(process: HANDLE, code: u32) -> i32;
}

#[repr(C)]
struct ExcRecord {
    code: u32,
    flags: u32,
    nested: *const ExcRecord,
    address: usize,
    n_params: u32,
    info: [usize; 15],
}

#[repr(C)]
struct ExcPointers {
    record: *const ExcRecord,
    context: *const u8,
}

/// The exit code of a process the catcher ended.
const EXIT_CODE: u32 = 0xA2C0_DE01;
/// Fatal exceptions after which the handler is taken to be looping.
const LOOP_LIMIT: u32 = 32;
/// Records written in full; later ones get one line.
const FULL_RECORDS: u32 = 4;
const MB: u64 = 1 << 20;

static CAUGHT: AtomicU32 = AtomicU32::new(0);
/// The thread writing a record, 0 for none.
static WRITER: AtomicU32 = AtomicU32::new(0);
static TEST_FRAMES: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicBool = AtomicBool::new(false);

/// A line built on the stack: a record is written while the heap may be damaged.
struct Line {
    b: [u8; 400],
    n: usize,
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let k = s.len().min(self.b.len() - self.n);
        self.b[self.n..self.n + k].copy_from_slice(&s.as_bytes()[..k]);
        self.n += k;
        Ok(())
    }
}

fn emit(args: core::fmt::Arguments) {
    let mut l = Line { b: [0; 400], n: 0 };
    let _ = l.write_fmt(args);
    rt::log_str(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
}

/// Whether `len` bytes at `p` can be read now.
fn readable(p: usize, len: usize) -> bool {
    let mut mi = MemInfo::default();
    if p < 0x10000 || unsafe { VirtualQuery(p as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 {
        return false;
    }
    mi.state == 0x1000 && mi.protect & 0x101 == 0 && mi.protect != 0 && p + len <= mi.base + mi.size
}

fn word(p: usize) -> Option<usize> {
    readable(p, 4).then(|| unsafe { *(p as *const usize) })
}

/// `name+offset` of the module an address of code is in, written into `out`; None when
/// the address is not in an image.
fn locate(a: usize, out: &mut Line) -> bool {
    let mut mi = MemInfo::default();
    if a < 0x10000 || unsafe { VirtualQuery(a as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 {
        return false;
    }
    if mi.state != 0x1000 || mi.mtype != 0x0100_0000 || mi.alloc_base == 0 {
        return false;
    }
    let mut buf = [0u8; 260];
    let n = unsafe { GetModuleFileNameA(mi.alloc_base as HANDLE, buf.as_mut_ptr(), 260) } as usize;
    let full = core::str::from_utf8(&buf[..n.min(260)]).unwrap_or("?");
    let _ = write!(out, "{}+{:#x}", full.rsplit(['\\', '/']).next().unwrap_or(full), a - mi.alloc_base);
    true
}

/// A summary of the process's address space, from `VirtualQuery`.
pub fn address_space(why: &str) {
    let (mut free, mut largest, mut blocks, mut commit, mut reserve) = (0u64, 0u64, 0u32, 0u64, 0u64);
    let (mut low_free, mut low_largest) = (0u64, 0u64);
    let mut p = 0x10000usize;
    let mut top = 0usize;
    loop {
        let mut mi = MemInfo::default();
        if unsafe { VirtualQuery(p as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 || mi.size == 0 {
            break;
        }
        let size = mi.size as u64;
        match mi.state {
            0x10000 => {
                free += size;
                blocks += 1;
                largest = largest.max(size);
                if mi.base < 0x8000_0000 {
                    low_free += size;
                    low_largest = low_largest.max(size);
                }
            }
            0x1000 => commit += size,
            _ => reserve += size,
        }
        top = mi.base + mi.size - 1;
        match mi.base.checked_add(mi.size) {
            Some(n) if n > p => p = n,
            _ => break,
        }
    }
    LAST_COMMIT.store((commit / MB) as u32, Ordering::Relaxed);
    LAST_LOG.store(unsafe { GetTickCount() }, Ordering::Relaxed);
    emit(format_args!(
        "address space ({}): largest free block {} MB, free {} MB in {} blocks (below 2 GB: largest {} MB, free {} MB), committed {} MB, reserved {} MB, user space to {:#x}",
        why, largest / MB, free / MB, blocks, low_largest / MB, low_free / MB, commit / MB, reserve / MB, top
    ));
}

static LAST_COMMIT: AtomicU32 = AtomicU32::new(0);
static LAST_LOG: AtomicU32 = AtomicU32::new(0);
static LAST_CHECK: AtomicU32 = AtomicU32::new(0);

/// The exceptions of the classes that end a program: not C++ exceptions, not breakpoints,
/// thread names or floating point (which the engine masks and unmasks itself).
fn fatal_class(code: u32) -> bool {
    matches!(
        code,
        0xC000_0005 | 0xC000_0006 | 0xC000_001D | 0xC000_0025 | 0xC000_0094 | 0xC000_0095 | 0xC000_0096 | 0xC000_00FD | 0xC000_0374 | 0xC000_0409
    )
}

fn die(why: &str) -> ! {
    emit(format_args!("crash: {}; the process ends here (exit code {:#x})", why, EXIT_CODE));
    unsafe { TerminateProcess(GetCurrentProcess(), EXIT_CODE) };
    loop {}
}

/// The frame pointer's chain from `bp`, every read checked.
fn walk(mut bp: usize) {
    for i in 0..16 {
        let (Some(next), Some(ret)) = (word(bp), word(bp.wrapping_add(4))) else {
            return;
        };
        let mut l = Line { b: [0; 400], n: 0 };
        let _ = write!(l, "  frame {}: bp {:#x} return {:#x} ", i, bp, ret);
        locate(ret, &mut l);
        rt::log_str(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
        if next <= bp || next - bp > 0x10_0000 || next & 3 != 0 {
            return;
        }
        bp = next;
    }
}

/// The stack words from `sp`, and the ones that look like return addresses.
fn stack(sp: usize) {
    for row in 0..8 {
        let at = sp + row * 32;
        let mut l = Line { b: [0; 400], n: 0 };
        let _ = write!(l, "  stack+{:#04x}:", row * 32);
        for k in 0..8 {
            match word(at + k * 4) {
                Some(w) => {
                    let _ = write!(l, " {:08x}", w);
                }
                None => {
                    let _ = write!(l, " --------");
                }
            }
        }
        rt::log_str(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
    }
    let mut shown = 0;
    for k in 0..64 {
        let Some(w) = word(sp + k * 4) else { break };
        let mut l = Line { b: [0; 400], n: 0 };
        let _ = write!(l, "  stack+{:#05x} = {:#x} ", k * 4, w);
        if locate(w, &mut l) && shown < 24 {
            shown += 1;
            rt::log_str(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
        }
    }
}

/// One record of the exception `info`.
unsafe fn record(info: &ExcPointers, n: u32) {
    let r = unsafe { &*info.record };
    let mut l = Line { b: [0; 400], n: 0 };
    let _ = write!(l, "crash: exception {} (first chance): code {:#010x}, flags {:#x}, address {:#x} ", n, r.code, r.flags, r.address);
    locate(r.address, &mut l);
    let _ = write!(l, ", thread {}", unsafe { GetCurrentThreadId() });
    rt::log_str(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
    if matches!(r.code, 0xC000_0005 | 0xC000_0006) && r.n_params >= 2 {
        let what = match r.info[0] {
            0 => "reading",
            1 => "writing",
            8 => "executing",
            _ => "touching",
        };
        emit(format_args!("  the access violation was {} {:#x}", what, r.info[1]));
    }
    if info.context.is_null() {
        return;
    }
    let c = |off: usize| unsafe { core::ptr::read_unaligned(info.context.add(off) as *const u32) };
    emit(format_args!(
        "  eax {:08x} ebx {:08x} ecx {:08x} edx {:08x} esi {:08x} edi {:08x} ebp {:08x} esp {:08x} eip {:08x} eflags {:08x}",
        c(0xb0), c(0xa4), c(0xac), c(0xa8), c(0xa0), c(0x9c), c(0xb4), c(0xc4), c(0xb8), c(0xc0)
    ));
    stack(c(0xc4) as usize);
    walk(c(0xb4) as usize);
    address_space("crash");
}

unsafe extern "system" fn handler(info: *mut ExcPointers) -> i32 {
    let info = unsafe { &*info };
    if info.record.is_null() || !fatal_class(unsafe { (*info.record).code }) {
        return 0;
    }
    let me = unsafe { GetCurrentThreadId() };
    match WRITER.compare_exchange(0, me, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {}
        Err(w) if w == me => die("an exception while writing the record of an exception"),
        Err(_) => return 0,
    }
    let n = CAUGHT.fetch_add(1, Ordering::AcqRel) + 1;
    if n <= FULL_RECORDS {
        unsafe { record(info, n) };
    } else if n <= LOOP_LIMIT {
        let r = unsafe { &*info.record };
        emit(format_args!("crash: exception {}: code {:#010x}, address {:#x}", n, r.code, r.address));
    } else {
        WRITER.store(0, Ordering::Release);
        die("more than 32 fatal exceptions: the engine's handler is looping");
    }
    WRITER.store(0, Ordering::Release);
    0
}

/// The engine's `DebugCallStack_DumpFrom(file, frame pointer)` as called by its exception
/// handler. No walk: the record above has the exception, and this ends the process.
unsafe extern "C" fn dump_from(_file: *const u8, bp: usize) {
    if WRITER.swap(unsafe { GetCurrentThreadId() }, Ordering::AcqRel) != 0 {
        die("the engine's crash handler was entered while a record was written");
    }
    emit(format_args!(
        "crash: the engine's exception handler is at its call-stack dump (frame pointer {:#x}); {} fatal exception(s) were seen before it{}",
        bp,
        CAUGHT.load(Ordering::Acquire),
        if CAUGHT.load(Ordering::Acquire) == 0 { " (none of a fatal class: an assertion, a breakpoint or a C++ exception)" } else { "" }
    ));
    if CAUGHT.load(Ordering::Acquire) == 0 {
        // No record was written: the walk and the address space are all there is.
        walk(bp);
        address_space("crash");
    }
    die("the engine's handler would walk the stack here, which faults under Wine and never ends");
}

pub fn configure(ini: &Ini) {
    use crate::rt::settings::on;
    if crate::bench_clone() {
        let n = ini.int("Crash", "TestFrames", 0);
        if n > 0 {
            TEST_FRAMES.store(n as u32, Ordering::Relaxed);
            log!("crash: TestFrames={}: a fault on purpose at that frame (the test bench)", n);
        }
    }
    if !on(ini, CRASH_CATCH) {
        log!("crash: Catch=0: the engine's crash handler is as it is");
        return;
    }
    let e = crate::engine();
    if e.ready(hook::CRASH).is_err() {
        return;
    }
    if !patch::patch_call(e.addr(sym::SITE_DUMP_FROM), e.addr(sym::DUMP_FROM), dump_from as *const () as usize) {
        log!("crash: the engine's exception handler is not stock: no crash catcher");
        return;
    }
    if unsafe { AddVectoredExceptionHandler(1, handler) }.is_null() {
        log!("crash: the vectored handler could not be installed; the engine's handler no longer walks the stack, but no record is written");
    }
    ARMED.store(true, Ordering::Relaxed);
    log!("crash: the catcher is on: a record of the fatal exceptions in this log, and the engine's call-stack walk ends the process");
}

/// At each `Present`: the address space when it has moved, and the test fault.
pub fn present(frame: u32) {
    let t = TEST_FRAMES.load(Ordering::Relaxed);
    if t != 0 && frame >= t {
        TEST_FRAMES.store(0, Ordering::Relaxed);
        log!("crash: TestFrames: writing to an unmapped address now");
        unsafe { core::ptr::write_volatile(0xdead_0010usize as *mut u32, 1) };
    }
    let now = unsafe { GetTickCount() };
    if now.wrapping_sub(LAST_CHECK.load(Ordering::Relaxed)) < 2000 {
        return;
    }
    LAST_CHECK.store(now, Ordering::Relaxed);
    let mut commit = 0u64;
    let mut p = 0x10000usize;
    loop {
        let mut mi = MemInfo::default();
        if unsafe { VirtualQuery(p as *const u8, &mut mi, core::mem::size_of::<MemInfo>()) } == 0 || mi.size == 0 {
            break;
        }
        if mi.state == 0x1000 {
            commit += mi.size as u64;
        }
        match mi.base.checked_add(mi.size) {
            Some(n) if n > p => p = n,
            _ => break,
        }
    }
    let last = LAST_COMMIT.load(Ordering::Relaxed) as i64;
    let moved = ((commit / MB) as i64 - last).abs() >= 64;
    if moved || now.wrapping_sub(LAST_LOG.load(Ordering::Relaxed)) >= 60_000 {
        let mut l = Line { b: [0; 400], n: 0 };
        let _ = write!(l, "frame {}, committed {:+} MB", frame, (commit / MB) as i64 - last);
        address_space(core::str::from_utf8(&l.b[..l.n]).unwrap_or("?"));
    }
}
