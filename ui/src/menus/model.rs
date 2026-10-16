//! The control model: what a hosted dialog shows, read afresh every frame from the
//! hidden dialog itself.
//!
//! A shell screen is made of two kinds of control (MENUS.md, "What a screen is made
//! of"):
//!
//! - **The shell's own widgets** (`engine::shellui`): `ShellButton`s and their kin, and
//!   `TextLabel`s, which the screen's procedure draws into the dialog and hit-tests
//!   itself on the dialog's `WM_MOUSEMOVE`/`WM_LBUTTONDOWN`/`WM_LBUTTONUP`. Most screens
//!   are only these.
//! - **Windows**: edit boxes, list boxes, combo boxes, scroll bars and the Admiral's
//!   Log's owner-drawn buttons, real child windows that draw and hit-test themselves.
//!
//! Labels come from the game, so they are its own (localised) strings. What the model
//! cannot read (a picture button's art, an owner-drawn list, a map preview) is drawn
//! from the dialog's mirror (`generic`), cropped to the control.
use crate::engine::dialogs::{self, Slot};
use crate::engine::shellui::{self, Kind};
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

/// A window control's class, as far as the model knows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Edit,
    List,
    Combo,
    ScrollBar,
    /// A push button, check box or radio button (`BS_*` in `style`).
    Button,
    /// Owner-drawn: its parent paints it (the Admiral's Log's tabs).
    OwnerDrawn,
    Static,
    /// A pane of the Admiral's Log (a dialog inside the dialog).
    Pane,
    Other,
}

/// What a control is.
#[derive(Clone, Debug)]
pub enum What {
    /// One of the shell's own widgets.
    Shell(Kind),
    /// A child window.
    Window(Class),
}

#[derive(Clone, Debug)]
pub struct Control {
    pub what: What,
    /// The shell widget's object, or the window: identifies the control from frame to
    /// frame.
    pub key: usize,
    /// The window (`Window`), or the dialog it draws into (`Shell`).
    pub hwnd: HWND,
    /// The window's control id (0 for shell widgets).
    pub id: i32,
    pub style: u32,
    /// In design coordinates (the 800x600 space every screen is laid out in): x, y, w, h.
    pub rect: [i32; 4],
    /// Its label, an edit's text, a combo's current choice.
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
    /// 0 normal, 1 under the pointer, 2 held (shell buttons, as the shell draws them).
    pub hover: u32,
    /// A check box or toggle that is on.
    pub checked: bool,
    /// A list's or combo's items (empty when its items are not strings), and the
    /// selected one (-1: none).
    pub items: Vec<String>,
    pub selected: i32,
    /// The first item a list shows.
    pub top: i32,
    /// A scroll bar's position and range.
    pub pos: i32,
    pub range: (i32, i32),
    /// A shell scroll bar: where its track lies along it in design coordinates (first
    /// pixel, length), and whether it runs horizontally.
    pub track: (i32, i32),
    pub horizontal: bool,
    /// The label's colour from the game, 0xAARRGGBB; -1 left, 0 centred, 1 right.
    pub colour: u32,
    pub align: i32,
}

/// A hosted dialog as the UI sees it.
pub struct Model {
    pub hwnd: HWND,
    /// "main menu", ... (`engine::dialogs::screen_name`), "" when unknown.
    pub screen: &'static str,
    pub template: usize,
    /// Its rectangle in design coordinates.
    pub rect: [i32; 4],
    pub controls: Vec<Control>,
}

impl Model {
    /// The shell button whose rectangle holds the design point `x`, `y`.
    pub fn button_at(&self, x: i32, y: i32) -> Option<&Control> {
        self.controls.iter().find(|c| {
            matches!(c.what, What::Shell(k) if k != Kind::Label)
                && c.visible
                && x >= c.rect[0]
                && y >= c.rect[1]
                && x < c.rect[0] + c.rect[2]
                && y < c.rect[1] + c.rect[3]
        })
    }
    /// The first control with this label (any case), shown or not.
    pub fn by_label(&self, s: &str) -> Option<&Control> {
        self.controls.iter().find(|c| c.text.eq_ignore_ascii_case(s))
    }
    pub fn by_id(&self, id: i32) -> Option<&Control> {
        self.controls.iter().find(|c| c.id == id && matches!(c.what, What::Window(_)))
    }
}

fn bgr_to_argb(c: u32) -> u32 {
    0xff00_0000 | (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff)
}

fn class_name(h: HWND) -> String {
    let mut b = [0u8; 32];
    let n = unsafe { GetClassNameA(h, b.as_mut_ptr(), 32) }.max(0) as usize;
    let mut s = String::new();
    for &c in &b[..n] {
        s.push(char::from(c.to_ascii_lowercase()));
    }
    s
}

pub fn window_text(h: HWND) -> String {
    let n = unsafe { SendMessageA(h, WM_GETTEXTLENGTH, 0, 0) }.clamp(0, 4096) as usize;
    let mut b = alloc::vec![0u8; n + 1];
    let got = unsafe { SendMessageA(h, WM_GETTEXT, n + 1, b.as_mut_ptr() as isize) }.clamp(0, n as isize) as usize;
    b[..got].iter().map(|&c| char::from(c)).collect()
}

const BS_TYPEMASK: u32 = 0xf;
const BS_OWNERDRAW: u32 = 0xb;
const BM_GETCHECK: u32 = 0xf0;
const LB_GETCOUNT: u32 = 0x18b;
const LB_GETCURSEL: u32 = 0x188;
const LB_GETTEXTLEN: u32 = 0x18a;
const LB_GETTEXT: u32 = 0x189;
const LB_GETTOPINDEX: u32 = 0x18e;
const LBS_HASSTRINGS: u32 = 0x40;
const LBS_OWNERDRAWFIXED: u32 = 0x10;
const LBS_OWNERDRAWVARIABLE: u32 = 0x20;
const CB_GETCOUNT: u32 = 0x146;
const CB_GETCURSEL: u32 = 0x147;
const CB_GETLBTEXTLEN: u32 = 0x149;
const CB_GETLBTEXT: u32 = 0x148;
const CBS_HASSTRINGS: u32 = 0x200;
const CBS_OWNERDRAWFIXED: u32 = 0x10;
const CBS_OWNERDRAWVARIABLE: u32 = 0x20;
const SBM_GETPOS: u32 = 0xe1;
const SBM_GETRANGE: u32 = 0xe3;
const MAX_ITEMS: i32 = 300;

fn items(h: HWND, count: u32, len: u32, text: u32) -> Vec<String> {
    let n = unsafe { SendMessageA(h, count, 0, 0) }.clamp(0, MAX_ITEMS as isize);
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        let l = unsafe { SendMessageA(h, len, i as usize, 0) };
        if !(0..4096).contains(&l) {
            v.push(String::new());
            continue;
        }
        let mut b = alloc::vec![0u8; l as usize + 1];
        let got = unsafe { SendMessageA(h, text, i as usize, b.as_mut_ptr() as isize) }.clamp(0, l) as usize;
        v.push(b[..got].iter().map(|&c| char::from(c)).collect());
    }
    v
}

fn blank(what: What, key: usize, hwnd: HWND, rect: [i32; 4]) -> Control {
    Control {
        what,
        key,
        hwnd,
        id: 0,
        style: 0,
        rect,
        text: String::new(),
        visible: true,
        enabled: true,
        hover: 0,
        checked: false,
        items: Vec::new(),
        selected: -1,
        top: 0,
        pos: 0,
        range: (0, 0),
        track: (0, 0),
        horizontal: true,
        colour: 0xffffffff,
        align: -1,
    }
}

fn window_control(h: HWND, ox: i32, oy: i32, parent: HWND) -> Control {
    let st = dialogs::style(h);
    let r = dialogs::rect_in(h, parent);
    let rect = [ox + r.left, oy + r.top, r.right - r.left, r.bottom - r.top];
    let cls = class_name(h);
    let class = if dialogs::is_hosted(h) {
        Class::Pane
    } else {
        match cls.as_str() {
            "edit" => Class::Edit,
            "listbox" => Class::List,
            "combobox" => Class::Combo,
            "scrollbar" => Class::ScrollBar,
            "static" => Class::Static,
            "button" if st & BS_TYPEMASK == BS_OWNERDRAW => Class::OwnerDrawn,
            "button" => Class::Button,
            _ => Class::Other,
        }
    };
    let mut c = blank(What::Window(class), h as usize, h, rect);
    c.id = unsafe { GetDlgCtrlID(h) };
    c.style = st;
    c.visible = st & WS_VISIBLE != 0;
    c.enabled = st & WS_DISABLED == 0;
    match class {
        Class::Edit | Class::Static | Class::Button | Class::OwnerDrawn => c.text = window_text(h),
        _ => {}
    }
    match class {
        Class::Button => c.checked = unsafe { SendMessageA(h, BM_GETCHECK, 0, 0) } == 1,
        Class::List => {
            let strings = st & LBS_HASSTRINGS != 0 || st & (LBS_OWNERDRAWFIXED | LBS_OWNERDRAWVARIABLE) == 0;
            if strings {
                c.items = items(h, LB_GETCOUNT, LB_GETTEXTLEN, LB_GETTEXT);
            }
            c.selected = unsafe { SendMessageA(h, LB_GETCURSEL, 0, 0) } as i32;
            c.top = unsafe { SendMessageA(h, LB_GETTOPINDEX, 0, 0) } as i32;
        }
        Class::Combo => {
            let strings = st & CBS_HASSTRINGS != 0 || st & (CBS_OWNERDRAWFIXED | CBS_OWNERDRAWVARIABLE) == 0;
            if strings {
                c.items = items(h, CB_GETCOUNT, CB_GETLBTEXTLEN, CB_GETLBTEXT);
            }
            c.selected = unsafe { SendMessageA(h, CB_GETCURSEL, 0, 0) } as i32;
            if c.selected >= 0 && (c.selected as usize) < c.items.len() {
                c.text = c.items[c.selected as usize].clone();
            } else {
                c.text = window_text(h);
            }
        }
        Class::ScrollBar => {
            c.pos = unsafe { SendMessageA(h, SBM_GETPOS, 0, 0) } as i32;
            let (mut lo, mut hi) = (0i32, 0i32);
            unsafe { SendMessageA(h, SBM_GETRANGE, &mut lo as *mut i32 as usize, &mut hi as *mut i32 as isize) };
            c.range = (lo, hi);
        }
        _ => {}
    }
    c
}

fn shell_controls(dlg: HWND, ox: i32, oy: i32, out: &mut Vec<Control>) {
    if !shellui::installed() {
        return;
    }
    for w in shellui::widgets_of(dlg) {
        let r = w.rect;
        let mut c = blank(What::Shell(w.kind), w.obj, dlg, [ox + r[0], oy + r[1], r[2] - r[0], r[3] - r[1]]);
        c.text = w.label;
        c.visible = w.shown;
        c.enabled = w.enabled;
        c.hover = w.mode;
        c.checked = w.on;
        c.selected = w.choice;
        if w.kind == Kind::Scroll {
            c.pos = w.value;
            c.range = w.range;
            c.horizontal = w.horizontal;
            c.track = (w.track.0 + if w.horizontal { ox } else { oy }, w.track.1);
        }
        c.colour = bgr_to_argb(w.colour);
        c.align = w.align;
        out.push(c);
    }
}

fn walk(dlg: HWND, ox: i32, oy: i32, out: &mut Vec<Control>) {
    shell_controls(dlg, ox, oy, out);
    for h in dialogs::children(dlg) {
        let c = window_control(h, ox, oy, dlg);
        let pane = matches!(c.what, What::Window(Class::Pane));
        let (px, py, vis) = (c.rect[0], c.rect[1], c.visible);
        out.push(c);
        if pane && vis {
            walk(h, px, py, out);
        }
    }
}

/// Reads a hosted modal dialog.
pub fn read(s: &Slot) -> Model {
    let (ox, oy) = dialogs::design_origin(s.hwnd);
    let mut controls = Vec::new();
    walk(s.hwnd, ox, oy, &mut controls);
    Model {
        hwnd: s.hwnd,
        screen: dialogs::screen_name(s.proc_addr),
        template: s.template,
        rect: [ox, oy, s.w(), s.h()],
        controls,
    }
}

/// UI.log: a dialog's controls, once per screen, for writing specs.
pub fn log(m: &Model) {
    crate::log!("menus: model of \"{}\" (template {}), {} controls:", m.screen, m.template, m.controls.len());
    for c in m.controls.iter().take(80) {
        crate::log!(
            "menus:   {:?} id {} @{},{} {}x{}{}{} \"{}\"{}",
            c.what,
            c.id,
            c.rect[0],
            c.rect[1],
            c.rect[2],
            c.rect[3],
            if c.visible { "" } else { " hidden" },
            if c.enabled { "" } else { " disabled" },
            c.text,
            if c.items.is_empty() { String::new() } else { crate::sfmt!(" [{} items, selected {}]", c.items.len(), c.selected) }
        );
    }
}
