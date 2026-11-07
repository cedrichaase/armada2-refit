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
use super::hook as patcher;
use crate::syms_shell::{field, hook, sym, value, TABLE};
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

/// `Shell::Widget::Widget(HWND)`, thiscall (`sym::SHELLUI_WIDGET_CTOR`): stores the
/// window at +4 and the vtable `Shell::Widget`'s, what a widget's first field holds before
/// its own class's constructor runs, and again after its destructor.
///
/// Members every button class inherits from `ShellButton`, found by vtable slot:
/// `UpdateButton(HDC)` (slot 8) and `TextLabel::Draw` (slot 1, the labels).
/// `ComboBox` (a `ShellButton` that draws a hidden Win32 combo box's choice and its own
/// pull-down list; Game Setup's player rows): `shellcombo.rs` reads the rest.
/// The Internet lobby's rows: `GameSelectionSlot` (a game the search found; its
/// `SetDescriptor` writes the row's text as the button's label) and `PlayerListButton`
/// (a player in the room). Both are `ShellButton`s with an `UpdateButton(HDC)` of their
/// own, so slot 8 does not name them (MENUS.md, "The Internet lobby").
/// The address range of Armada2.exe's vtables (`.rdata`).
#[inline(always)]
fn val_rdata() -> (usize, usize) { (value::SHELLUI_RDATA_START() as usize, value::SHELLUI_RDATA_END() as usize) }

// `ShellButton` fields, and `ShellScrollBar`'s, `TextLabel`'s and the other classes':
// `syms_shell::field` (label "NULL" for none; mode 0 normal, 1 under the pointer, 2 held;
// the rectangle is left, top, right, bottom in the dialog; `ShellButtonSelBox` takes the
// pointer in a part of its shared picture, which its `CheckRollOver` tests; `TextLabel`'s
// flags are DrawText's; `StdTextLabel` keeps the text as a `std::string`, its pointer at
// +4 and length at +8).
#[inline(always)]
fn btn_label_len() -> usize { value::SHELLUI_BTN_LABEL_LEN() as usize }
#[inline(always)]
fn lbl_text_len() -> usize { value::SHELLUI_LBL_TEXT_LEN() as usize }

/// Widgets remembered, in the order they were made.
static mut WIDGETS: Vec<(usize, u32)> = Vec::new();
static mut GENERATION: u32 = 0;
static mut INSTALLED: bool = false;
const MAX_WIDGETS: usize = 4096;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

fn vt_widget() -> usize {
    eng().addr(sym::SHELLUI_VT_WIDGET)
}

pub fn install() -> bool {
    if eng().ready(hook::SHELL_WIDGETS).is_err() || eng().ready(hook::SHELL_READS_WIDGETS).is_err() {
        return false;
    }
    let ctor = eng().addr(sym::SHELLUI_WIDGET_CTOR);
    let ours = widget_ctor as *const () as usize;
    let rel = (ours as i64 - (ctor as i64 + 5)) as i32;
    let mut jmp = [0xe9u8, 0, 0, 0, 0];
    jmp[1..5].copy_from_slice(&rel.to_le_bytes());
    // `ready` checked the constructor's bytes.
    let mut stock = [0u8; 21];
    unsafe { core::ptr::copy_nonoverlapping(ctor as *const u8, stock.as_mut_ptr(), 21) };
    if !patcher::patch_bytes(ctor, &stock, &jmp) {
        crate::log!("menus: could not patch Shell::Widget's constructor: the shell's own buttons are not read");
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
        core::ptr::write_unaligned(this as *mut usize, vt_widget());
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

/// Whether `n` bytes at `p` are committed and readable. The bytes may span regions: on
/// Roots, FleetOpsHook re-protects pages of the executable's `.rdata` as it patches it,
/// which splits it into regions, and a vtable can straddle two (decision 250).
fn readable(p: usize, n: usize) -> bool {
    let end = p.saturating_add(n);
    let mut at = p;
    for _ in 0..4 {
        let mut m = MemoryBasicInformation::default();
        if unsafe { VirtualQuery(at as *const u8, &mut m, core::mem::size_of::<MemoryBasicInformation>()) } == 0 {
            return false;
        }
        // PAGE_NOACCESS (1) and guard pages (0x100) cannot be read.
        if !(m.state == MEM_COMMIT && m.protect & 0x101 == 0 && m.protect != 0) {
            return false;
        }
        let next = m.base + m.region_size;
        if end <= next {
            return true;
        }
        at = next;
    }
    false
}

fn rd<T: Copy>(a: usize) -> T {
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

fn vt_ok(vt: usize) -> bool {
    vt >= val_rdata().0 && vt < val_rdata().1 && vt != vt_widget()
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
        if vt == vt_widget() {
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
    /// A `ComboBox`: the choice of a hidden combo box, with a pull-down of its own.
    Combo,
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
    let vt_scrollbar = eng().addr(sym::SHELLUI_VT_SCROLLBAR);
    let vt_combo = eng().addr(sym::SHELLUI_VT_COMBO_BOX);
    let vt_game_slot = eng().addr(sym::SHELLUI_VT_GAME_SLOT);
    let vt_player_list = eng().addr(sym::SHELLUI_VT_PLAYER_LIST);
    let vt_cycle = eng().addr(sym::SHELLUI_VT_CYCLE_BUTTON);
    let vt_toggle = eng().addr(sym::SHELLUI_VT_TOGGLE_BUTTON);
    let vt_selbox = eng().addr(sym::SHELLUI_VT_SELBOX);
    let vt_std = eng().addr(sym::SHELLUI_VT_STD_TEXT_LABEL);
    let slot = |i: usize| rd::<usize>(vt + i * 4);
    if slot(8) == eng().addr(sym::SHELLUI_BUTTON_UPDATE_HDC) || vt == vt_scrollbar || vt == vt_combo || vt == vt_game_slot || vt == vt_player_list {
        if vt == vt_scrollbar {
            let horizontal = rd::<u8>(w + field::SHELLUI_SB_HORIZONTAL()) != 0;
            let r = [rd::<i32>(w + field::SHELLUI_SB_RECT()), rd::<i32>(w + field::SHELLUI_SB_RECT() + 4), rd::<i32>(w + field::SHELLUI_SB_RECT() + 8), rd::<i32>(w + field::SHELLUI_SB_RECT() + 12)];
            let start = rd::<i32>(w + if horizontal { field::SHELLUI_SB_TRACK_START() } else { field::SHELLUI_SB_TRACK_TOP() });
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
                value: rd::<i32>(w + field::SHELLUI_SB_VALUE()),
                range: (rd::<i32>(w + field::SHELLUI_SB_LO()), rd::<i32>(w + field::SHELLUI_SB_HI())),
                track: (start, rd::<i32>(w + field::SHELLUI_SB_TRACK_LEN())),
                horizontal,
            });
        }
        let kind = if vt == vt_cycle {
            Kind::Cycle
        } else if vt == vt_toggle {
            Kind::Toggle
        } else if vt == vt_selbox {
            Kind::SelBox
        } else if vt == vt_combo {
            Kind::Combo
        } else {
            Kind::Button
        };
        let mut label = cstr_at(w + field::SHELLUI_BTN_LABEL(), btn_label_len());
        if label == "NULL" {
            label.clear();
        }
        let at = if kind == Kind::SelBox { field::SHELLUI_SELBOX_RECT() } else { field::SHELLUI_BTN_RECT() };
        let r = [rd::<i32>(w + at), rd::<i32>(w + at + 4), rd::<i32>(w + at + 8), rd::<i32>(w + at + 12)];
        return Some(Widget {
            obj: w,
            kind,
            hwnd: rd::<usize>(w + field::SHELLUI_BTN_HWND()) as HWND,
            rect: r,
            label,
            shown: rd::<u32>(w + field::SHELLUI_BTN_SHOWN()) != 0,
            enabled: rd::<u32>(w + field::SHELLUI_BTN_ENABLED()) != 0,
            mode: rd::<u32>(w + field::SHELLUI_BTN_MODE()),
            on: kind == Kind::Toggle && rd::<u8>(w + field::SHELLUI_TOGGLE_ON()) != 0,
            choice: if kind == Kind::Cycle { rd::<i32>(w + field::SHELLUI_CYCLE_SELECTION()) } else { -1 },
            colour: rd::<u32>(w + field::SHELLUI_BTN_COLOUR()),
            align: if rd::<u8>(w + field::SHELLUI_BTN_LEFT()) != 0 { -1 } else if rd::<u8>(w + field::SHELLUI_BTN_RIGHT()) != 0 { 1 } else { 0 },
            value: 0,
            range: (0, 0),
            track: (0, 0),
            horizontal: true,
        });
    }
    if slot(1) == eng().addr(sym::SHELLUI_TEXTLABEL_DRAW) {
        let label = if vt == vt_std {
            let p: usize = rd(w + field::SHELLUI_STD_LBL_STRING() + 4);
            let n: usize = rd(w + field::SHELLUI_STD_LBL_STRING() + 8);
            if p != 0 && n < 4096 && readable(p, n) { cstr_at(p, n) } else { cstr_at(w + field::SHELLUI_LBL_TEXT(), lbl_text_len()) }
        } else {
            cstr_at(w + field::SHELLUI_LBL_TEXT(), lbl_text_len())
        };
        let flags: u32 = rd(w + field::SHELLUI_LBL_FLAGS());
        let r = [rd::<i32>(w + field::SHELLUI_LBL_RECT()), rd::<i32>(w + field::SHELLUI_LBL_RECT() + 4), rd::<i32>(w + field::SHELLUI_LBL_RECT() + 8), rd::<i32>(w + field::SHELLUI_LBL_RECT() + 12)];
        return Some(Widget {
            obj: w,
            kind: Kind::Label,
            hwnd: rd::<usize>(w + field::SHELLUI_LBL_HWND()) as HWND,
            rect: r,
            label,
            shown: rd::<u32>(w + field::SHELLUI_LBL_SHOWN()) != 0,
            enabled: true,
            mode: 0,
            on: false,
            choice: -1,
            colour: rd::<u32>(w + field::SHELLUI_LBL_COLOUR()),
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
    if !installed() {
        return Vec::new();
    }
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

/// UI.log: the widgets made for `hwnd` that `read` does not know (their vtable and the
/// two slots it tells classes by): what another build's classes look like.
pub fn log_unread(hwnd: HWND) {
    if !installed() {
        return;
    }
    let v = unsafe { &*core::ptr::addr_of!(WIDGETS) };
    let mine = v.iter().filter(|&&(w, _)| readable(w, 8) && rd::<usize>(w + 4) == hwnd as usize).count();
    crate::log!("menus: {} shell widgets remembered, {} made for {:p}", v.len(), mine, hwnd);
    for &(w, _) in v.iter() {
        if !readable(w, 8) || rd::<usize>(w + 4) != hwnd as usize || read(w).is_some() {
            continue;
        }
        let vt: usize = rd(w);
        let (s1, s8) = if readable(vt, 0x24) { (rd::<usize>(vt + 4), rd::<usize>(vt + 32)) } else { (0, 0) };
        crate::log!("menus: widget {:#x} of {:p} not read: vtable {:#x} (in the exe's .rdata: {}), slots 1 {:#x} and 8 {:#x}", w, hwnd, vt, vt_ok(vt), s1, s8);
    }
}
