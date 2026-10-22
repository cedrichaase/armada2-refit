//! The macros every interface file uses: calling a slot, reading the frame pointer,
//! and a method that passes through.

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

/// A method that passes straight through to the real object, traced. `$k` is the
/// interface, `$slot` the method's slot.
#[macro_export]
macro_rules! pass {
    ($k:ident $slot:literal $name:ident($($a:ident: $t:ty),*) -> $r:ty) => {
        unsafe extern "system" fn $name(this: *mut $crate::com::Obj $(, $a: $t)*) -> $r {
            let fp = $crate::frame!();
            $crate::trace::call(fp, $crate::com::Kind::$k, $slot);
            let real = unsafe { (*this).real };
            $crate::vcall!(real, $slot, fn($($t),*) -> $r, $($a),*)
        }
    };
}

/// `QueryInterface`, `AddRef` and `Release` for one kind.
#[macro_export]
macro_rules! unknown {
    ($k:ident, $qi:ident, $addref:ident, $release:ident) => {
        unsafe extern "system" fn $qi(this: *mut $crate::com::Obj, iid: *const $crate::com::Guid, out: *mut *mut core::ffi::c_void) -> $crate::com::HRESULT {
            let fp = $crate::frame!();
            $crate::trace::call(fp, $crate::com::Kind::$k, 0);
            $crate::com::query(this, iid, out, fp)
        }
        unsafe extern "system" fn $addref(this: *mut $crate::com::Obj) -> u32 {
            let real = unsafe { (*this).real };
            $crate::vcall!(real, 1, fn() -> u32,)
        }
        unsafe extern "system" fn $release(this: *mut $crate::com::Obj) -> u32 {
            unsafe { $crate::com::release(this) }
        }
    };
}

/// A draw: passes through as `pass!` does, unless `[Drop]` names one of its callers
/// (`drop.rs`), when it is accepted and not passed on.
#[macro_export]
macro_rules! draw {
    ($slot:literal $name:ident($($a:ident: $t:ty),*)) => {
        unsafe extern "system" fn $name(this: *mut $crate::com::Obj $(, $a: $t)*) -> $crate::com::HRESULT {
            let fp = $crate::frame!();
            $crate::trace::call(fp, $crate::com::Kind::Device, $slot);
            $crate::detail::draw(fp, $slot, &[$($a as usize),*]);
            if $crate::drop::dropped(fp) {
                return 0;
            }
            let real = unsafe { (*this).real };
            $crate::vcall!(real, $slot, fn($($t),*) -> $crate::com::HRESULT, $($a),*)
        }
    };
}
