//! The VCL's published properties, read by name from a control's class information
//! (ui/MENUS.md, "Roots' forms"; the state of the controls Roots draws as pictures).
//!
//! A check box, a slider and the like keep their state in a published property (`Checked`,
//! `Value`, `MinValue`, `MaxValue`). The language runtime lays a class's run-time type
//! information out the same way for every class: a slot before the class pointer holds
//! the class's type record, whose property table lists each property's name and how it is
//! read: a field of the object (a flag and an offset), a virtual method or a static one.
//! Only the first is read here, straight from the object; a property read through a
//! method says so (`None`) and the control then keeps pressing through, as before. Nothing
//! is called and nothing is written. Every pointer is checked.
use alloc::string::String;
use alloc::vec::Vec;

/// The class's type record sits this far before the class pointer (the runtime's layout,
/// not FleetOpsHook's).
const VMT_TYPE_INFO: usize = 0x3c; // not an offset into game memory: a VCL class slot, before the class pointer
/// How a property is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Getter {
    /// A field of the object at this offset.
    Field(usize),
    /// A virtual or a static method: not read.
    Method,
}

/// A published property: its name, the byte size of what it holds is not recorded (the
/// caller knows the type), and how it is read.
#[derive(Clone, Debug)]
pub struct Prop {
    pub name: String,
    pub getter: Getter,
}

fn readable(p: usize, n: usize) -> bool {
    p >= 0x10000 && super::readable(p, n)
}

fn rd<T: Copy>(p: usize) -> T {
    unsafe { super::rd::<T>(p) }
}

fn short_string(p: usize) -> Option<String> {
    if !readable(p, 1) {
        return None;
    }
    let n = rd::<u8>(p) as usize;
    if !readable(p + 1, n) {
        return None;
    }
    let b = unsafe { core::slice::from_raw_parts((p + 1) as *const u8, n) };
    core::str::from_utf8(b).ok().map(String::from)
}

/// The published properties of a class and of every class it derives from, nearest first.
/// `in_image` says whether an address lies in the module that owns the classes; a type
/// record outside it ends the walk.
pub fn published(vmt: usize, in_image: impl Fn(usize) -> bool) -> Vec<Prop> {
    let mut out = Vec::new();
    let mut v = vmt;
    for _ in 0..40 {
        if !in_image(v) || !readable(v - VMT_TYPE_INFO, 4) {
            break;
        }
        let ti = rd::<usize>(v - VMT_TYPE_INFO);
        if !in_image(ti) || !readable(ti, 2) {
            break;
        }
        // Kind (1 byte), the name, then the class's data: its class pointer, its parent's
        // type record's address, the property count, the unit's name, the properties.
        let Some(name) = short_string(ti + 1) else { break };
        let data = ti + 2 + name.len();
        if !readable(data, 10) {
            break;
        }
        let parent = rd::<usize>(data + 4);
        let Some(unit) = short_string(data + 10) else { break };
        let mut p = data + 10 + 1 + unit.len();
        if !readable(p, 2) {
            break;
        }
        let n = rd::<u16>(p) as usize;
        p += 2;
        for _ in 0..n.min(256) {
            if !readable(p, 27) {
                break;
            }
            let get = rd::<u32>(p + 4);
            let Some(pname) = short_string(p + 26) else { break };
            let getter = if get >> 24 == 0xff { Getter::Field((get & 0x00ff_ffff) as usize) } else { Getter::Method };
            let len = 27 + pname.len();
            out.push(Prop { name: pname, getter });
            p += len;
        }
        // The parent's type record's address is stored by its address's own cell.
        if parent == 0 || !readable(parent, 4) {
            break;
        }
        let pti = rd::<usize>(parent);
        if !in_image(pti) {
            break;
        }
        // Walk on from the parent's class: its type record is at `pti`; the class pointer
        // is in its data, after the name.
        let Some(pname) = short_string(pti + 1) else { break };
        let pdata = pti + 2 + pname.len();
        if !readable(pdata, 4) {
            break;
        }
        v = rd::<usize>(pdata);
    }
    out
}

/// A property held in a field, read as an `i32` of `size` bytes (1, 2 or 4), by name;
/// `None` when the class has no such property or reads it through a method.
pub fn field_of(obj: usize, props: &[Prop], name: &str, size: usize) -> Option<i32> {
    let Getter::Field(off) = props.iter().find(|p| p.name == name)?.getter else { return None };
    if !readable(obj + off, size) {
        return None;
    }
    Some(match size {
        1 => rd::<u8>(obj + off) as i32,
        2 => rd::<i16>(obj + off) as i32,
        _ => rd::<i32>(obj + off),
    })
}
