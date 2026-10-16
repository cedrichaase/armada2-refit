//! The shell's `ComboBox`: Game Setup's player rows (player or computer, side, team)
//! are not window controls but `ShellButton`s of this class, each showing the choice
//! of a hidden Win32 combo box and drawing a pull-down list of its own into the dialog
//! when clicked. `shellui` finds them (`Kind::Combo`); this file reads what a spec
//! needs to show one and to choose an item the stock way (MENUS.md, "Game Setup").
//!
//! Fields, each from the method that writes or reads it (only read, never called):
//!
//! | Offset | What | From |
//! |---|---|---|
//! | +0x178 | items in the pull-down | `ComboBox::Send` counts `CB_ADDSTRING`/`CB_INSERTSTRING` |
//! | +0x17c | an item's height in the pull-down | `Send`, `InDropDownArea` |
//! | +0x180 | the pull-down's rectangle (left, top, right, bottom), the button's own included | `Send` (from the button's rectangle and the count), `InDropDownArea` |
//! | +0x198 | the hidden combo box | `Send`, which forwards every message to it |
//! | +0x1b4 | the pull-down opens downwards (else upwards) | `Send` |
//!
//! The pull-down's items follow the button's own row: opening downwards item `i` is the
//! `i+1`th row of `h` below the rectangle's top (`InDropDownArea` takes the first `h`
//! as the button itself); opening upwards it is the `i`th from the top, and the last
//! `h` is the button.
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

const COUNT: usize = 0x178;
const ITEM_H: usize = 0x17c;
const PULL: usize = 0x180;
const COMBO: usize = 0x198;
const DOWN: usize = 0x1b4;

fn rd<T: Copy>(a: usize) -> T {
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

/// The hidden combo box behind the `ComboBox` at `obj` (a widget `shellui` returned
/// this frame), if it is a window.
pub fn combo(obj: usize) -> Option<HWND> {
    let h = rd::<usize>(obj + COMBO) as HWND;
    if h.is_null() || unsafe { IsWindow(h) } == 0 {
        return None;
    }
    Some(h)
}

const CB_GETCOUNT: u32 = 0x146;
const CB_GETCURSEL: u32 = 0x147;
const CB_GETLBTEXTLEN: u32 = 0x149;
const CB_GETLBTEXT: u32 = 0x148;

/// Its items and the chosen one (-1: none), read from the hidden combo box.
pub fn items(obj: usize) -> (Vec<String>, i32) {
    let Some(h) = combo(obj) else { return (Vec::new(), -1) };
    let n = unsafe { SendMessageA(h, CB_GETCOUNT, 0, 0) }.clamp(0, 64);
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        let l = unsafe { SendMessageA(h, CB_GETLBTEXTLEN, i as usize, 0) };
        if !(0..512).contains(&l) {
            v.push(String::new());
            continue;
        }
        let mut b = alloc::vec![0u8; l as usize + 1];
        let got = unsafe { SendMessageA(h, CB_GETLBTEXT, i as usize, b.as_mut_ptr() as isize) }.clamp(0, l) as usize;
        v.push(b[..got].iter().map(|&c| char::from(c)).collect());
    }
    let sel = unsafe { SendMessageA(h, CB_GETCURSEL, 0, 0) } as i32;
    (v, sel)
}

/// The point (in the dialog's client coordinates) of item `i` in the pull-down, which
/// a click there chooses once the pull-down is open.
pub fn item_point(obj: usize, i: i32) -> Option<(i32, i32)> {
    let count: i32 = rd(obj + COUNT);
    let h: i32 = rd(obj + ITEM_H);
    if i < 0 || i >= count || !(4..=64).contains(&h) {
        return None;
    }
    let (l, t, r, _b) = (rd::<i32>(obj + PULL), rd::<i32>(obj + PULL + 4), rd::<i32>(obj + PULL + 8), rd::<i32>(obj + PULL + 12));
    let down = rd::<u8>(obj + DOWN) != 0;
    let row = if down { i + 1 } else { i };
    Some(((l + r) / 2, t + row * h + h / 2))
}
