//! What a program with no C runtime must supply itself: the allocator, the four
//! memory functions LLVM calls, `_fltused`, and the stack probe `__chkstk` for frames
//! over a page. The memory functions are written in assembly so that LLVM cannot
//! recognise their loops and turn them back into calls to themselves.
// rustc warns that defining these symbols is suspicious; here it is the point.
#![allow(suspicious_runtime_symbol_definitions)]
use super::win::*;
use core::alloc::{GlobalAlloc, Layout};

pub struct Heap;

// HeapAlloc aligns to 8. Larger alignments over-allocate and keep the original
// pointer in the word just below the aligned block.
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe {
            if l.align() <= 8 {
                return HeapAlloc(GetProcessHeap(), 0, l.size().max(1));
            }
            let raw = HeapAlloc(GetProcessHeap(), 0, l.size() + l.align() + 4);
            if raw.is_null() {
                return raw;
            }
            let a = (raw as usize + 4 + l.align() - 1) & !(l.align() - 1);
            *((a - 4) as *mut usize) = raw as usize;
            a as *mut u8
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe {
            if l.align() <= 8 {
                HeapFree(GetProcessHeap(), 0, p);
            } else {
                HeapFree(GetProcessHeap(), 0, *((p as usize - 4) as *const usize) as *mut u8);
            }
        }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        unsafe {
            if l.align() <= 8 {
                return HeapAlloc(GetProcessHeap(), 8, l.size().max(1)); // HEAP_ZERO_MEMORY
            }
            let p = self.alloc(l);
            if !p.is_null() {
                memset(p, 0, l.size());
            }
            p
        }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        unsafe {
            if l.align() <= 8 {
                return HeapReAlloc(GetProcessHeap(), 0, p, n.max(1));
            }
            let q = self.alloc(Layout::from_size_align_unchecked(n, l.align()));
            if !q.is_null() {
                memcpy(q, p, l.size().min(n));
                self.dealloc(p, l);
            }
            q
        }
    }
}

#[unsafe(no_mangle)]
pub static _fltused: i32 = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    unsafe {
        core::arch::asm!("push esi", "mov esi, {s}", "cld", "rep movsb", "pop esi",
            s = in(reg) s, inout("edi") d => _, inout("ecx") n => _);
    }
    d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    if n == 0 {
        return d;
    }
    if (d as usize) <= (s as usize) || (d as usize) >= (s as usize + n) {
        return unsafe { memcpy(d, s, n) };
    }
    unsafe {
        core::arch::asm!("push esi", "mov esi, {s}", "std", "rep movsb", "cld", "pop esi",
            s = in(reg) s.add(n - 1), inout("edi") d.add(n - 1) => _, inout("ecx") n => _);
    }
    d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 {
    unsafe {
        core::arch::asm!("cld", "rep stosb", inout("edi") d => _, inout("ecx") n => _, in("al") c as u8);
    }
    d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let (x, y) = unsafe { (core::ptr::read_volatile(a.add(i)), core::ptr::read_volatile(b.add(i))) };
        if x != y {
            return x as i32 - y as i32;
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe { memcmp(a, b, n) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut n = 0;
    unsafe {
        while core::ptr::read_volatile(s.add(n)) != 0 {
            n += 1;
        }
    }
    n
}

// 64-bit division and remainder, which LLVM leaves to the C runtime's helpers on this
// target (float formatting needs them). Each takes two 64-bit values on the stack,
// pops them itself and returns in edx:eax: `stdcall`, under the undecorated name
// (`\x01` stops LLVM adding `@16`). Shift-and-subtract, so no 64-bit division
// appears inside them to call back into themselves.
fn udivmod(n: u64, d: u64) -> (u64, u64) {
    if d == 0 {
        return (u64::MAX, n);
    }
    let (mut q, mut r) = (0u64, 0u64);
    let mut i = 64;
    while i > 0 {
        i -= 1;
        r = (r << 1) | ((n >> i) & 1);
        if r >= d {
            r -= d;
            q |= 1u64 << i;
        }
    }
    (q, r)
}


extern "stdcall" fn aulldiv(n: u64, d: u64) -> u64 {
    udivmod(n, d).0
}


extern "stdcall" fn aullrem(n: u64, d: u64) -> u64 {
    udivmod(n, d).1
}


extern "stdcall" fn alldiv(n: i64, d: i64) -> i64 {
    let q = udivmod(n.unsigned_abs(), d.unsigned_abs()).0 as i64;
    if (n < 0) != (d < 0) { q.wrapping_neg() } else { q }
}


extern "stdcall" fn allrem(n: i64, d: i64) -> i64 {
    let r = udivmod(n.unsigned_abs(), d.unsigned_abs()).1 as i64;
    if n < 0 { r.wrapping_neg() } else { r }
}

core::arch::global_asm!(
    ".globl __aulldiv", "__aulldiv:", "jmp {a}",
    ".globl __aullrem", "__aullrem:", "jmp {b}",
    ".globl __alldiv", "__alldiv:", "jmp {c}",
    ".globl __allrem", "__allrem:", "jmp {d}",
    a = sym aulldiv, b = sym aullrem, c = sym alldiv, d = sym allrem,
);

// The MSVC stack probe: eax = bytes wanted. Touches each page down to the new stack
// top, then moves esp there, keeping the return address on top.
core::arch::global_asm!(
    ".globl __chkstk",
    ".globl __alloca_probe",
    "__chkstk:",
    "__alloca_probe:",
    "push ecx",
    "lea ecx, [esp + 8]",
    "2:",
    "cmp eax, 0x1000",
    "jb 3f",
    "sub ecx, 0x1000",
    "test [ecx], eax",
    "sub eax, 0x1000",
    "jmp 2b",
    "3:",
    "sub ecx, eax",
    "test [ecx], eax",
    "mov eax, esp",
    "mov esp, ecx",
    "mov ecx, [eax]",
    "mov eax, [eax + 4]",
    "push eax",
    "ret",
);
