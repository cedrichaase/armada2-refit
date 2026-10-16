//! The shell's own widgets: the buttons and labels a stock screen draws into its dialog
//! (`ShellButton`, `TextLabel` and their kin, all `Shell::Widget`s), which are not
//! windows and so cannot be found by walking the dialog's children. `menus::model`
//! reads them through this file; MENUS.md, "The shell's widgets", has what is known of
//! them and why each offset is read.
//!
//! **How they are found.** Every shell widget is built through `Shell::Widget`'s
//! constructor, which stores the dialog it draws into and its vtable. Its entry is
//! re-pointed here at a function of ours that does the same two stores and remembers
//! the object; a widget whose vtable is no longer one of the shell's (its destructor
//! ends by putting `Shell::Widget`'s back, and the heap may reuse the block) is
//! forgotten. Nothing is ever called on a widget: fields are read by offset, and only
//! after the memory under it is checked to be committed and readable.
use super::hook;
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

/// `Shell::Widget::Widget(HWND)`, thiscall: stores the window at +4 and the vtable.
const FN_WIDGET_CTOR: usize = 0x5a8060;
const WIDGET_CTOR_STOCK: [u8; 21] = [
    0x55, 0x8b, 0xec, 0x8b, 0xc1, 0x8b, 0x4d, 0x08, 0x89, 0x48, 0x04, 0xc7, 0x00, 0xd4, 0xb2, 0x6b, 0x00, 0x5d, 0xc2,
    0x04, 0x00,
];
/// `Shell::Widget`'s vtable: what a widget's first field holds before its own class's
/// constructor runs, and again after its destructor.
const VT_WIDGET: usize = 0x6bb2d4;
/// Members every button class inherits from `ShellButton`, found by vtable slot:
/// `UpdateButton(HDC)` (slot 8) and `TextLabel::Draw` (slot 1, the labels).
const FN_BUTTON_UPDATE_HDC: usize = 0x5a4af0;
const FN_TEXTLABEL_DRAW: usize = 0x5a7590;
const VT_CYCLE_BUTTON: usize = 0x6bb064;
const VT_TOGGLE_BUTTON: usize = 0x6bb18c;
const VT_SELBOX: usize = 0x6bb0e4;
const VT_STD_TEXT_LABEL: usize = 0x6bbaa4;
const VT_SCROLLBAR: usize = 0x6bb164;
/// The address range of Armada2.exe's vtables (`.rdata`).
const RDATA: (usize, usize) = (0x6b0000, 0x6d0000);

// `ShellButton` fields.
const BTN_HWND: usize = 0x20;
const BTN_LABEL: usize = 0x3c; // char[64]: the label, "NULL" for none
const BTN_LABEL_LEN: usize = 0x40;
const BTN_MODE: usize = 0xc0; // 0 normal, 1 under the pointer, 2 held
const BTN_RECT: usize = 0xc4; // left, top, right, bottom in the dialog
/// ShellButtonSelBox: the part of its (shared) picture that takes the pointer, which
/// its `CheckRollOver` tests instead of the whole picture's rectangle.
const SELBOX_RECT: usize = 0x178;
const BTN_SHOWN: usize = 0xe4;
const BTN_ENABLED: usize = 0xe8;
const BTN_COLOUR: usize = 0x28; // label colour, COLORREF
const BTN_LEFT: usize = 0x174; // label left-aligned
const BTN_RIGHT: usize = 0x175; // label right-aligned
// `ShellScrollBar` fields (ctor 0x5a6280, `CheckButtons` 0x5a63f0, `Draw` 0x5a6520).
const SB_RECT: usize = 0x28; // the whole bar: left, top, right, bottom
const SB_TRACK_START: usize = 0x10; // horizontal: the end of the first arrow button
const SB_TRACK_TOP: usize = 0x14; // vertical: the end of the first arrow button
const SB_VALUE: usize = 0x3c;
const SB_TRACK_LEN: usize = 0x40; // the length of the track between the arrow buttons
const SB_HORIZONTAL: usize = 0x44;
const SB_LO: usize = 0x48;
const SB_HI: usize = 0x4c;
const TOGGLE_ON: usize = 0x188; // ShellToggleButton: on
const CYCLE_SELECTION: usize = 0x198; // CycleButton: the choice shown
// `TextLabel` fields.
const LBL_HWND: usize = 0x04;
const LBL_TEXT: usize = 0x08; // char[1024]
const LBL_RECT: usize = 0x408;
const LBL_FLAGS: usize = 0x418; // DrawText flags
const LBL_SHOWN: usize = 0x45c;
const LBL_COLOUR: usize = 0x464;
/// `StdTextLabel`: the text as a `std::string` (its pointer at +4, length at +8).
const STD_LBL_STRING: usize = 0x468;

/// Widgets remembered, in the order they were made.
static mut WIDGETS: Vec<(usize, u32)> = Vec::new();
static mut GENERATION: u32 = 0;
static mut INSTALLED: bool = false;
const MAX_WIDGETS: usize = 4096;

pub fn install() -> bool {
    let ours = widget_ctor as *const () as usize;
    let rel = (ours as i64 - (FN_WIDGET_CTOR as i64 + 5)) as i32;
    let mut jmp = [0xe9u8, 0, 0, 0, 0];
    jmp[1..5].copy_from_slice(&rel.to_le_bytes());
    if !hook::patch_bytes(FN_WIDGET_CTOR, &WIDGET_CTOR_STOCK, &jmp) {
        crate::log!("menus: Shell::Widget's constructor is not stock: the shell's own buttons are not read");
        return false;
    }
    unsafe { INSTALLED = true };
    true
}

pub fn installed() -> bool {
    unsafe { INSTALLED }
}

/// In place of `Shell::Widget::Widget`: the constructor's two stores, and the object
/// remembered.
unsafe extern "thiscall" fn widget_ctor(this: usize, hwnd: usize) -> usize {
    unsafe {
        core::ptr::write_unaligned((this + 4) as *mut usize, hwnd);
        core::ptr::write_unaligned(this as *mut usize, VT_WIDGET);
        let v = &mut *core::ptr::addr_of_mut!(WIDGETS);
        if v.len() >= MAX_WIDGETS {
            prune();
            if v.len() >= MAX_WIDGETS {
                v.remove(0);
            }
        }
        v.push((this, GENERATION));
    }
    this
}

/// Whether `n` bytes at `p` are committed and readable.
fn readable(p: usize, n: usize) -> bool {
    let mut m = MemoryBasicInformation::default();
    if unsafe { VirtualQuery(p as *const u8, &mut m, core::mem::size_of::<MemoryBasicInformation>()) } == 0 {
        return false;
    }
    // PAGE_NOACCESS (1) and guard pages (0x100) cannot be read.
    m.state == MEM_COMMIT && m.protect & 0x101 == 0 && m.protect != 0 && p + n <= m.base + m.region_size
}

fn rd<T: Copy>(a: usize) -> T {
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

fn vt_ok(vt: usize) -> bool {
    vt >= RDATA.0 && vt < RDATA.1 && vt != VT_WIDGET
}

/// Forgets the widgets that are gone.
fn prune() {
    let now_gen = unsafe { GENERATION };
    let v = unsafe { &mut *core::ptr::addr_of_mut!(WIDGETS) };
    v.retain(|&(w, g)| {
        if !readable(w, 8) {
            return false;
        }
        let vt: usize = rd(w);
        // Still Shell::Widget's own: either being built (keep it a while) or destroyed.
        if vt == VT_WIDGET {
            return g == now_gen;
        }
        vt_ok(vt)
    });
}

/// What kind of shell widget it is, by its class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A `ShellButton` or a class built on it: a picture with an optional label.
    Button,
    /// A `CycleButton`: a label that steps through a list of choices per click.
    Cycle,
    /// A `ShellToggleButton`: on or off, its label saying which.
    Toggle,
    /// A `ShellButtonSelBox`: a row of a selection box.
    SelBox,
    /// A `TextLabel` or `StdTextLabel`: text the screen draws.
    Label,
    /// A `ShellScrollBar`.
    Scroll,
}

/// A shell widget as read this frame.
#[derive(Clone, Debug)]
pub struct Widget {
    pub obj: usize,
    pub kind: Kind,
    /// The dialog it draws into.
    pub hwnd: HWND,
    /// In the dialog's client coordinates (the 800x600 design space for full-screen
    /// ones): left, top, right, bottom.
    pub rect: [i32; 4],
    pub label: String,
    pub shown: bool,
    pub enabled: bool,
    /// 0 normal, 1 under the pointer, 2 held (buttons).
    pub mode: u32,
    /// ShellToggleButton: on. CycleButton: the choice's index, as `on as u32` is 0.
    pub on: bool,
    pub choice: i32,
    /// The label's colour, 0x00BBGGRR.
    pub colour: u32,
    /// -1 left, 0 centred, 1 right.
    pub align: i32,
    /// `ShellScrollBar`: the value, its range, and where the track lies along the bar
    /// (first pixel, length), in the dialog's coordinates; `horizontal`.
    pub value: i32,
    pub range: (i32, i32),
    pub track: (i32, i32),
    pub horizontal: bool,
}

fn cstr_at(a: usize, max: usize) -> String {
    let mut s = String::new();
    for i in 0..max {
        let c: u8 = rd(a + i);
        if c == 0 {
            break;
        }
        s.push(char::from(c));
    }
    s
}

fn read(w: usize) -> Option<Widget> {
    if !readable(w, 0x470) {
        // The largest field read; a smaller object at the end of a region is skipped.
        if !readable(w, 0x200) {
            return None;
        }
    }
    let vt: usize = rd(w);
    if !vt_ok(vt) || !readable(vt, 0x24) {
        return None;
    }
    let slot = |i: usize| rd::<usize>(vt + i * 4);
    if slot(8) == FN_BUTTON_UPDATE_HDC || vt == VT_SCROLLBAR {
        if vt == VT_SCROLLBAR {
            let horizontal = rd::<u8>(w + SB_HORIZONTAL) != 0;
            let r = [rd::<i32>(w + SB_RECT), rd::<i32>(w + SB_RECT + 4), rd::<i32>(w + SB_RECT + 8), rd::<i32>(w + SB_RECT + 12)];
            let start = rd::<i32>(w + if horizontal { SB_TRACK_START } else { SB_TRACK_TOP });
            return Some(Widget {
                obj: w,
                kind: Kind::Scroll,
                hwnd: rd::<usize>(w + 4) as HWND,
                rect: r,
                label: String::new(),
                shown: true,
                enabled: true,
                mode: 0,
                on: false,
                choice: -1,
                colour: 0,
                align: 0,
                value: rd::<i32>(w + SB_VALUE),
                range: (rd::<i32>(w + SB_LO), rd::<i32>(w + SB_HI)),
                track: (start, rd::<i32>(w + SB_TRACK_LEN)),
                horizontal,
            });
        }
        let kind = match vt {
            VT_CYCLE_BUTTON => Kind::Cycle,
            VT_TOGGLE_BUTTON => Kind::Toggle,
            VT_SELBOX => Kind::SelBox,
            _ => Kind::Button,
        };
        let mut label = cstr_at(w + BTN_LABEL, BTN_LABEL_LEN);
        if label == "NULL" {
            label.clear();
        }
        let at = if kind == Kind::SelBox { SELBOX_RECT } else { BTN_RECT };
        let r = [rd::<i32>(w + at), rd::<i32>(w + at + 4), rd::<i32>(w + at + 8), rd::<i32>(w + at + 12)];
        return Some(Widget {
            obj: w,
            kind,
            hwnd: rd::<usize>(w + BTN_HWND) as HWND,
            rect: r,
            label,
            shown: rd::<u32>(w + BTN_SHOWN) != 0,
            enabled: rd::<u32>(w + BTN_ENABLED) != 0,
            mode: rd::<u32>(w + BTN_MODE),
            on: kind == Kind::Toggle && rd::<u8>(w + TOGGLE_ON) != 0,
            choice: if kind == Kind::Cycle { rd::<i32>(w + CYCLE_SELECTION) } else { -1 },
            colour: rd::<u32>(w + BTN_COLOUR),
            align: if rd::<u8>(w + BTN_LEFT) != 0 { -1 } else if rd::<u8>(w + BTN_RIGHT) != 0 { 1 } else { 0 },
            value: 0,
            range: (0, 0),
            track: (0, 0),
            horizontal: true,
        });
    }
    if slot(1) == FN_TEXTLABEL_DRAW {
        let label = if vt == VT_STD_TEXT_LABEL {
            let p: usize = rd(w + STD_LBL_STRING + 4);
            let n: usize = rd(w + STD_LBL_STRING + 8);
            if p != 0 && n < 4096 && readable(p, n) { cstr_at(p, n) } else { cstr_at(w + LBL_TEXT, 1024) }
        } else {
            cstr_at(w + LBL_TEXT, 1024)
        };
        let flags: u32 = rd(w + LBL_FLAGS);
        let r = [rd::<i32>(w + LBL_RECT), rd::<i32>(w + LBL_RECT + 4), rd::<i32>(w + LBL_RECT + 8), rd::<i32>(w + LBL_RECT + 12)];
        return Some(Widget {
            obj: w,
            kind: Kind::Label,
            hwnd: rd::<usize>(w + LBL_HWND) as HWND,
            rect: r,
            label,
            shown: rd::<u32>(w + LBL_SHOWN) != 0,
            enabled: true,
            mode: 0,
            on: false,
            choice: -1,
            colour: rd::<u32>(w + LBL_COLOUR),
            // DT_CENTER 1, DT_RIGHT 2
            align: if flags & 1 != 0 { 0 } else if flags & 2 != 0 { 1 } else { -1 },
            value: 0,
            range: (0, 0),
            track: (0, 0),
            horizontal: true,
        });
    }
    None
}

/// The shell widgets that draw into `hwnd`, in the order they were made.
pub fn widgets_of(hwnd: HWND) -> Vec<Widget> {
    unsafe { GENERATION = GENERATION.wrapping_add(1) };
    prune();
    let v = unsafe { &*core::ptr::addr_of!(WIDGETS) };
    let mut out = Vec::new();
    for &(w, _) in v.iter() {
        if let Some(x) = read(w) {
            if x.hwnd == hwnd && !out.iter().any(|o: &Widget| o.obj == x.obj) {
                out.push(x);
            }
        }
    }
    out
}
