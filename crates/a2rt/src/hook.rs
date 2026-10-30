//! Patching Armada2.exe in memory: a `call rel32` re-pointed (checked against the
//! target stock calls, so another plugin's patch makes us stand down instead of
//! stacking), and vtable slots.
use crate::win::*;

unsafe fn writable(p: *mut u8, n: usize, f: impl FnOnce()) -> bool {
    let mut old = 0u32;
    unsafe {
        if VirtualProtect(p, n, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return false;
        }
        f();
        VirtualProtect(p, n, old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), p, n);
    }
    true
}

/// The target of the `call rel32` at `site`, or None if `site` is not one.
pub fn call_target(site: usize) -> Option<usize> {
    unsafe {
        let p = site as *const u8;
        if *p != 0xe8 {
            return None;
        }
        let rel = core::ptr::read_unaligned(p.add(1) as *const i32);
        Some((site as i64 + 5 + rel as i64) as usize)
    }
}

/// Re-points the call at `site` from `expect` to `to`. False (and nothing written) if
/// the site does not call `expect`.
pub fn patch_call(site: usize, expect: usize, to: usize) -> bool {
    if call_target(site) != Some(expect) {
        return false;
    }
    let rel = (to as i64 - (site as i64 + 5)) as i32;
    unsafe { writable((site + 1) as *mut u8, 4, || core::ptr::write_unaligned((site + 1) as *mut i32, rel)) }
}

/// Writes `to` into vtable slot `slot` at `vt`, returning what was there.
pub fn patch_slot(vt: usize, slot: usize, to: usize) -> Option<usize> {
    let p = (vt + slot * 4) as *mut usize;
    let old = unsafe { *p };
    if unsafe { writable(p as *mut u8, 4, || *p = to) } { Some(old) } else { None }
}

/// Writes bytes at an address (for short in-place patches that check what they
/// replace first).
pub fn patch_bytes(at: usize, expect: &[u8], new: &[u8]) -> bool {
    let p = at as *mut u8;
    unsafe {
        if core::slice::from_raw_parts(p, expect.len()) != expect {
            return false;
        }
        writable(p, new.len(), || core::ptr::copy_nonoverlapping(new.as_ptr(), p, new.len()))
    }
}
