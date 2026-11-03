//! The macros every interface file uses: calling a slot and reading the frame pointer.

/// A method of a COM object by slot, called with `this` first.
#[macro_export]
macro_rules! vcall {
    ($obj:expr, $slot:expr, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {{
        let o = $obj as *mut core::ffi::c_void;
        #[allow(unused_unsafe)]
        unsafe {
            let vt = *(o as *const *const usize);
            let f: unsafe extern "system" fn(*mut core::ffi::c_void $(, $t)*) -> $r =
                core::mem::transmute(*vt.add($slot));
            f(o $(, $a)*)
        }
    }};
}

/// This function's frame pointer (`ebp`): every function keeps one
/// (`force-frame-pointers`), so `[ebp+4]` is the return address into the caller.
#[macro_export]
macro_rules! frame {
    () => {{
        let fp: usize;
        #[allow(unused_unsafe)]
        unsafe {
            core::arch::asm!("mov {}, ebp", out(reg) fp, options(nomem, nostack, preserves_flags))
        };
        fp
    }};
}
