//! What the UI does to a hosted dialog, queued during a frame and run by the dialog
//! host after the frame is presented (`engine::dialogs::run_modal`). A stock procedure
//! answering a click can open a nested dialog, which runs a loop of its own; that must
//! not happen inside a scene, so nothing here is sent while a frame is drawn.
//!
//! Every action sends what the stock control would have been sent by user32 for the
//! same input: a click is the dialog's (or the control's) own mouse messages in its
//! client coordinates, typing is `WM_CHAR`, a choice in a list is the list's selection
//! and the notification its parent gets.
use crate::engine::dialogs;
use crate::rt::win::*;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub enum Action {
    /// A mouse message for `h` at `x`, `y` in its client coordinates.
    Mouse { h: HWND, msg: u32, keys: usize, x: i32, y: i32 },
    /// A whole click (move, down, up) at a point of the dialog `dlg`, in design
    /// coordinates (`dlg`'s own client space plus its origin).
    ClickDesign { dlg: HWND, x: i32, y: i32 },
    /// A key or a character for a control: `msg` is WM_KEYDOWN or WM_CHAR.
    Key { h: HWND, msg: u32, wp: usize },
    /// A list box's or combo box's selection, and the notification a user's choice
    /// sends its parent (`LBN_SELCHANGE`/`LBN_DBLCLK`, `CBN_SELCHANGE` then
    /// `CBN_SELENDOK`).
    Select { h: HWND, combo: bool, index: i32, double: bool },
}

static mut QUEUE: Vec<Action> = Vec::new();

pub fn push(a: Action) {
    unsafe { (*core::ptr::addr_of_mut!(QUEUE)).push(a) };
}

/// A click in design coordinates on the hosted dialog `dlg` (the dialog's procedure, or
/// the control under the point, gets it).
pub fn click_design(dlg: HWND, x: i32, y: i32) {
    push(Action::ClickDesign { dlg, x, y });
}

const LB_SETCURSEL: u32 = 0x186;
const CB_SETCURSEL: u32 = 0x14e;
const LBN_SELCHANGE: usize = 1;
const LBN_DBLCLK: usize = 2;
const CBN_SELCHANGE: usize = 1;
const CBN_SELENDOK: usize = 9;

/// Runs what was queued, in order.
pub fn run() {
    let q = core::mem::take(unsafe { &mut *core::ptr::addr_of_mut!(QUEUE) });
    let trace = crate::config::get().menus_trace;
    for a in q {
        if trace && !matches!(a, Action::Mouse { msg: WM_MOUSEMOVE, .. }) {
            crate::log!("menus: action {:?}", a);
        }
        match a {
            Action::Mouse { h, msg, keys, x, y } => {
                if unsafe { IsWindow(h) } != 0 {
                    dialogs::send_mouse(h, msg, keys, x, y);
                }
            }
            Action::ClickDesign { dlg, x, y } => {
                if unsafe { IsWindow(dlg) } == 0 {
                    continue;
                }
                let (ox, oy) = dialogs::design_origin(dlg);
                let (t, lx, ly) = dialogs::hit(dlg, x - ox, y - oy);
                dialogs::send_mouse(t, WM_MOUSEMOVE, 0, lx, ly);
                dialogs::send_mouse(t, WM_LBUTTONDOWN, MK_LBUTTON, lx, ly);
                if unsafe { IsWindow(t) } != 0 {
                    dialogs::send_mouse(t, WM_LBUTTONUP, 0, lx, ly);
                }
            }
            Action::Key { h, msg, wp } => {
                if unsafe { IsWindow(h) } != 0 {
                    unsafe { SendMessageA(h, msg, wp, if msg == WM_KEYDOWN { 1 } else { 1 }) };
                }
            }
            Action::Select { h, combo, index, double } => {
                if unsafe { IsWindow(h) } == 0 {
                    continue;
                }
                unsafe {
                    let id = GetDlgCtrlID(h) as usize & 0xffff;
                    let parent = GetParent(h);
                    if combo {
                        SendMessageA(h, CB_SETCURSEL, index as usize, 0);
                        SendMessageA(parent, WM_COMMAND, id | CBN_SELCHANGE << 16, h as isize);
                        SendMessageA(parent, WM_COMMAND, id | CBN_SELENDOK << 16, h as isize);
                    } else {
                        SendMessageA(h, LB_SETCURSEL, index as usize, 0);
                        SendMessageA(parent, WM_COMMAND, id | LBN_SELCHANGE << 16, h as isize);
                        if double {
                            SendMessageA(parent, WM_COMMAND, id | LBN_DBLCLK << 16, h as isize);
                        }
                    }
                }
            }
        }
    }
}
