//! The dialog host: every shell screen is a stock modal dialog, and runs here hidden,
//! drawn and driven by the UI (`ui/MENUS.md`).
//!
//! Contract (see `crate::surface::frame`): while a hosted dialog runs, a loop of ours on
//! the game's thread pumps the queue and, each frame, opens a scene, calls
//! `crate::surface::frame(Context::Menus)` and presents; `menus` draws the dialogs from
//! what this file exposes (`stack()`, each dialog's `Surface`) and acts on them through
//! `menus::actions`, which this loop runs after the frame is presented.
//!
//! What is hooked, all through Armada2.exe's import table (the user32 functions the
//! shell calls), each forwarding to user32 for any window that is not hosted:
//!
//! - `DialogBoxParamA` creates the stock template, hidden, as a child of the 3D window,
//!   and runs `run_modal` in place of user32's modal loop. `EndDialog` on a hosted
//!   dialog records its result and ends that loop. A dialog that cannot be hosted runs
//!   stock (and the log says why).
//! - `CreateDialogParamA` (the Admiral's Log's panes): a pane of a hosted dialog is made
//!   its hidden child too, and drawn into its parent.
//! - `GetDC`/`ReleaseDC`/`BeginPaint`/`EndPaint` hand a hosted dialog a memory DC of
//!   its own (its `Surface`), so everything the shell draws lands there. A hidden window
//!   is never sent `WM_PAINT`, so `InvalidateRect`/`RedrawWindow` mark the dialog and
//!   the loop sends the paint itself.
//! - `MoveWindow`/`SetWindowPos`/`ShowWindow`/`GetWindow`: the shell places its menus
//!   in screen coordinates against their owner; a hosted dialog keeps its design
//!   geometry (a full-screen one is 800x600 at 0,0), stays hidden, and answers
//!   `GW_OWNER` with the owner the shell asked for.
//! - The two `push 16 / push 600 / push 800` of `SetActiveDisplay_Internal`: the shell's
//!   display mode raised to the desktop's, so the shell and the 3D display share one
//!   device and the UI draws at the screen's resolution.
use super::{hook, shell};

/// Reads a value at an address of Armada2.exe's image (always mapped).
fn rd<T: Copy>(a: usize) -> T {
    unsafe { super::rd::<T>(a) }
}
use crate::rt::win::*;
use crate::surface::Context;
use alloc::vec::Vec;

/// The design space every shell screen is laid out in.
pub const DESIGN_W: i32 = 800;
pub const DESIGN_H: i32 = 600;

/// `SetActiveDisplay_Internal`'s two `push 16; push 600; push 800`, the shell's mode.
const MODE_SITES: [usize; 2] = [0x62c4d4, 0x62c633];
const MODE_STOCK: [u8; 12] = [0x6a, 0x10, 0x68, 0x58, 0x02, 0x00, 0x00, 0x68, 0x20, 0x03, 0x00, 0x00];
/// The shell's code: only dialog procedures in it are hosted (the map editor's dialogs
/// are not shell screens).
const SHELL_CODE: (usize, usize) = (0x5a0000, 0x610000);
/// `EscapeMenuDlgProc`, the in-mission menu, and its Return to Game button's centre in
/// design coordinates (x 5, y 568, 172x22: menus/README.md, "Esc").
const ESCAPE_MENU_PROC: usize = 0x5cddb0;
const ESCAPE_RETURN_AT: (i32, i32) = (91, 579);
/// The shell's dialog procedures by name: what a screen spec (`menus::screens`) is
/// chosen by, and what UI.log calls a dialog. Templates are shared between screens
/// (0x123 is the main menu, the in-mission menu, Load Game, Game and Sound Settings),
/// the procedures are not. MENUS.md has the table with the `do_*` function of each.
const SCREENS: &[(usize, &str)] = &[
    (0x5d57c0, "main menu"),
    (0x5dbdc0, "single player"),
    (0x5d6d80, "mission select"),
    (0x5d48c0, "load game"),
    (0x5d9320, "save game"),
    (0x5cddb0, "options"),
    (0x5cf930, "game settings"),
    (0x5d1540, "graphics settings"),
    (0x5dce60, "sound settings"),
    (0x5c2270, "game setup"),
    (0x5ba900, "advanced options"),
    (0x5b81e0, "map select"),
    (0x5ba140, "map transfer"),
    (0x5bf5a0, "multiplayer connection"),
    (0x5b15f0, "internet game"),
    (0x5b6c10, "manual ip"),
    (0x5cb280, "room select"),
    (0x5ab900, "create room"),
    (0x5ac3d0, "game name"),
    (0x5ca690, "password"),
    (0x5a85b0, "change password"),
    (0x5a9db0, "create account"),
    (0x5cc620, "gamespy login"),
    (0x5ca0b0, "net waiting"),
    (0x5b07a0, "help"),
    (0x5d3bd0, "licence agreement"),
    (0x5d7730, "message"),
    (0x5e1990, "yes or no"),
    (0x5de0d0, "technology tree"),
    (0x5e22c0, "admiral's log"),
    (0x5f6010, "admiral's log pane"),
    (0x5a2de0, "shell screen"),
];

/// The name of a hosted dialog's screen ("main menu"), or "" for one not in the table.
pub fn screen_name(proc_addr: usize) -> &'static str {
    SCREENS.iter().find(|s| s.0 == proc_addr).map_or("", |s| s.1)
}

const MAX_SLOTS: usize = 32;

/// An 800x600 32-bit memory DC that a hosted dialog draws into, top-down, BGRX.
#[derive(Clone, Copy)]
pub struct Surface {
    pub dc: HDC,
    bmp: HGDIOBJ,
    old: HGDIOBJ,
    pub bits: *mut u8,
}

impl Surface {
    const NONE: Surface = Surface { dc: core::ptr::null_mut(), bmp: core::ptr::null_mut(), old: core::ptr::null_mut(), bits: core::ptr::null_mut() };

    fn make() -> Surface {
        let bi = BitmapInfoHeader { size: 40, width: DESIGN_W, height: -DESIGN_H, planes: 1, bit_count: 32, ..Default::default() };
        unsafe {
            let dc = CreateCompatibleDC(core::ptr::null_mut());
            if dc.is_null() {
                return Surface::NONE;
            }
            let mut bits = core::ptr::null_mut();
            let bmp = CreateDIBSection(dc, &bi, 0, &mut bits, core::ptr::null_mut(), 0);
            if bmp.is_null() || bits.is_null() {
                DeleteDC(dc);
                return Surface::NONE;
            }
            let old = SelectObject(dc, bmp);
            PatBlt(dc, 0, 0, DESIGN_W, DESIGN_H, BLACKNESS);
            Surface { dc, bmp, old, bits }
        }
    }

    fn free(&mut self) {
        if !self.dc.is_null() {
            unsafe {
                SelectObject(self.dc, self.old);
                DeleteObject(self.bmp);
                DeleteDC(self.dc);
            }
        }
        *self = Surface::NONE;
    }

    pub fn ok(&self) -> bool {
        !self.dc.is_null()
    }

    /// Puts the bitmap back in the DC: a stock helper of the Admiral's Log selects the
    /// 1x1 default bitmap into whatever DC it is handed (menus/README.md).
    pub fn reselect(&self) {
        if self.ok() {
            unsafe { SelectObject(self.dc, self.bmp) };
        }
    }
}

/// A hosted dialog.
#[derive(Clone, Copy)]
pub struct Slot {
    pub hwnd: HWND,
    proc_: usize,
    /// The owner the shell asked for (the 3D window, or the dialog that opened it).
    pub owner: HWND,
    /// Its template: a resource id (`MENUS.md` has the table).
    pub template: usize,
    /// Its dialog procedure: what a screen spec is chosen by.
    pub proc_addr: usize,
    /// 1.. for modal dialogs, by nesting depth; 0 for a pane (CreateDialogParamA).
    pub depth: u32,
    ended: bool,
    result: isize,
    /// Where it is, in its parent's client coordinates: the design space for a modal
    /// dialog, the parent's for a pane.
    pub rect: WinRect,
    /// What it draws into.
    pub surface: Surface,
    /// What it and its controls look like this frame (modal dialogs only).
    pub frame: Surface,
    paint: bool,
    erase: bool,
    /// Bumped whenever it draws.
    pub drawn: u32,
}

impl Slot {
    const EMPTY: Slot = Slot {
        hwnd: core::ptr::null_mut(),
        proc_: 0,
        owner: core::ptr::null_mut(),
        template: 0,
        proc_addr: 0,
        depth: 0,
        ended: false,
        result: 0,
        rect: WinRect { left: 0, top: 0, right: 0, bottom: 0 },
        surface: Surface::NONE,
        frame: Surface::NONE,
        paint: false,
        erase: false,
        drawn: 0,
    };
    pub fn w(&self) -> i32 {
        self.rect.right - self.rect.left
    }
    pub fn h(&self) -> i32 {
        self.rect.bottom - self.rect.top
    }
}

static mut SLOTS: [Slot; MAX_SLOTS] = [const { Slot::EMPTY }; MAX_SLOTS];
/// The dialog being created: its first message names its window.
static mut PENDING: Option<Slot> = None;
static mut DEPTH: u32 = 0;
static mut READY: bool = false;
/// A memory DC handed out for the 3D window while a dialog is hosted, so the shell's
/// GDI drawing onto it (its background underlay) never reaches the screen over ours.
static mut SCRATCH: Surface = Surface::NONE;

// The originals of the hooked imports.
static mut O_DIALOG_BOX: usize = 0;
static mut O_CREATE_DIALOG: usize = 0;
static mut O_END_DIALOG: usize = 0;
static mut O_GET_DC: usize = 0;
static mut O_RELEASE_DC: usize = 0;
static mut O_BEGIN_PAINT: usize = 0;
static mut O_END_PAINT: usize = 0;
static mut O_INVALIDATE: usize = 0;
static mut O_REDRAW: usize = 0;
static mut O_MOVE: usize = 0;
static mut O_SET_POS: usize = 0;
static mut O_SHOW: usize = 0;
static mut O_GET_WINDOW: usize = 0;

fn slots() -> &'static mut [Slot; MAX_SLOTS] {
    unsafe { &mut *core::ptr::addr_of_mut!(SLOTS) }
}

fn slot_of(h: HWND) -> Option<&'static mut Slot> {
    if h.is_null() {
        return None;
    }
    slots().iter_mut().find(|s| s.hwnd == h)
}

fn is_surface(dc: HDC) -> bool {
    !dc.is_null() && (slots().iter().any(|s| s.surface.dc == dc) || unsafe { SCRATCH.dc } == dc)
}

pub fn ready() -> bool {
    unsafe { READY }
}

/// Whether a hosted dialog is running.
pub fn hosting() -> bool {
    unsafe { DEPTH > 0 }
}

/// The modal dialogs being hosted, outermost first.
pub fn stack() -> Vec<&'static Slot> {
    let mut v: Vec<&'static Slot> = slots().iter().filter(|s| !s.hwnd.is_null() && s.depth > 0).collect();
    v.sort_by_key(|s| s.depth);
    v
}

/// The innermost hosted dialog: the one that takes input.
pub fn top() -> Option<&'static Slot> {
    stack().last().copied()
}

/// The panes (CreateDialogParamA dialogs) drawn into `parent`.
pub fn panes_of(parent: HWND) -> Vec<&'static Slot> {
    slots().iter().filter(|s| !s.hwnd.is_null() && s.depth == 0 && unsafe { GetParent(s.hwnd) } == parent).collect()
}

pub fn is_hosted(h: HWND) -> bool {
    slot_of(h).is_some()
}

/// Whether `h` is a hosted dialog or a control in one.
pub fn in_hosted(h: HWND) -> bool {
    let mut w = h;
    for _ in 0..8 {
        if w.is_null() {
            return false;
        }
        if is_hosted(w) {
            return true;
        }
        w = unsafe { GetParent(w) };
    }
    false
}

// ---- installing ----------------------------------------------------------------------

/// The address of the import-table slot through which Armada2.exe calls `dll!name`.
fn iat_entry(dll: &str, name: &str) -> Option<usize> {
    let base = unsafe { GetModuleHandleA(core::ptr::null()) } as usize;
    let pe = base + rd::<u32>(base + 0x3c) as usize;
    let imports = rd::<u32>(pe + 0x80) as usize;
    if imports == 0 {
        return None;
    }
    let eq = |a: usize, s: &str| {
        let b = s.as_bytes();
        (0..b.len()).all(|i| rd::<u8>(a + i).eq_ignore_ascii_case(&b[i])) && rd::<u8>(a + b.len()) == 0
    };
    let mut d = base + imports;
    loop {
        let (names, dll_name, thunks) = (rd::<u32>(d) as usize, rd::<u32>(d + 12) as usize, rd::<u32>(d + 16) as usize);
        if dll_name == 0 {
            return None;
        }
        if eq(base + dll_name, dll) {
            let mut k = 0;
            let lookup = if names != 0 { names } else { thunks };
            loop {
                let t = rd::<u32>(base + lookup + k * 4) as usize;
                if t == 0 {
                    break;
                }
                if t & 0x8000_0000 == 0 && eq(base + t + 2, name) {
                    return Some(base + thunks + k * 4);
                }
                k += 1;
            }
            return None;
        }
        d += 20;
    }
}

/// Points the import of `user32!name` at `ours`, returning user32's function. None (and
/// nothing written) if the slot does not hold user32's own function: another plugin has
/// hooked it.
fn hook_user32(name: &str, ours: usize) -> Option<usize> {
    let slot = iat_entry("USER32.dll", name)?;
    let real = unsafe {
        let n = crate::rt::cstr(name);
        GetProcAddress(GetModuleHandleA(c"user32.dll".as_ptr() as *const u8), n.as_ptr())
    };
    let cur = rd::<usize>(slot);
    if real == 0 || cur != real {
        crate::log!("menus: Armada2.exe's import of {} is not user32's ({:#x}, user32 {:#x})", name, cur, real);
        return None;
    }
    hook::patch_slot(slot, 0, ours)
}

/// Whether Menus.asi (or its old name, MenuScale.asi) is loaded: it hooks the same
/// dialogs, so the UI's menus stand down.
fn menus_asi_loaded() -> bool {
    unsafe { !GetModuleHandleA(c"Menus.asi".as_ptr() as *const u8).is_null() || !GetModuleHandleA(c"MenuScale.asi".as_ptr() as *const u8).is_null() }
}

/// Installs the host, at DllMain time (the display mode must be raised before the
/// engine first reads it). False, with nothing patched, when anything is not stock.
pub fn install() -> bool {
    if menus_asi_loaded() {
        crate::log!("menus: Menus.asi is loaded: the UI's menus stand down (it hosts the same dialogs)");
        return false;
    }
    for &at in &MODE_SITES {
        if unsafe { core::slice::from_raw_parts(at as *const u8, MODE_STOCK.len()) } != MODE_STOCK {
            crate::log!("menus: the shell's display mode at {:#x} is not stock: standing down", at);
            return false;
        }
    }
    let hooks: [(&str, usize, *mut usize); 13] = [
        ("DialogBoxParamA", my_dialog_box as *const () as usize, core::ptr::addr_of_mut!(O_DIALOG_BOX)),
        ("CreateDialogParamA", my_create_dialog as *const () as usize, core::ptr::addr_of_mut!(O_CREATE_DIALOG)),
        ("EndDialog", my_end_dialog as *const () as usize, core::ptr::addr_of_mut!(O_END_DIALOG)),
        ("GetDC", my_get_dc as *const () as usize, core::ptr::addr_of_mut!(O_GET_DC)),
        ("ReleaseDC", my_release_dc as *const () as usize, core::ptr::addr_of_mut!(O_RELEASE_DC)),
        ("BeginPaint", my_begin_paint as *const () as usize, core::ptr::addr_of_mut!(O_BEGIN_PAINT)),
        ("EndPaint", my_end_paint as *const () as usize, core::ptr::addr_of_mut!(O_END_PAINT)),
        ("InvalidateRect", my_invalidate as *const () as usize, core::ptr::addr_of_mut!(O_INVALIDATE)),
        ("RedrawWindow", my_redraw as *const () as usize, core::ptr::addr_of_mut!(O_REDRAW)),
        ("MoveWindow", my_move as *const () as usize, core::ptr::addr_of_mut!(O_MOVE)),
        ("SetWindowPos", my_set_pos as *const () as usize, core::ptr::addr_of_mut!(O_SET_POS)),
        ("ShowWindow", my_show as *const () as usize, core::ptr::addr_of_mut!(O_SHOW)),
        ("GetWindow", my_get_window as *const () as usize, core::ptr::addr_of_mut!(O_GET_WINDOW)),
    ];
    // Every slot is checked before any is written.
    for &(name, _, _) in &hooks {
        let Some(slot) = iat_entry("USER32.dll", name) else {
            crate::log!("menus: Armada2.exe does not import {}: standing down", name);
            return false;
        };
        let real = unsafe {
            let n = crate::rt::cstr(name);
            GetProcAddress(GetModuleHandleA(c"user32.dll".as_ptr() as *const u8), n.as_ptr())
        };
        if real == 0 || rd::<usize>(slot) != real {
            crate::log!("menus: Armada2.exe's import of {} is hooked by something else: standing down", name);
            return false;
        }
    }
    for &(name, ours, orig) in &hooks {
        match hook_user32(name, ours) {
            Some(o) => unsafe { *orig = o },
            None => {
                crate::log!("menus: could not hook {}", name);
                return false;
            }
        }
    }
    raise_shell_mode();
    if !super::shellui::install() {
        crate::log!("menus: the generic screens draw every dialog as the game paints it");
    }
    unsafe { READY = true };
    true
}

/// The shell's 800x600x16 raised to the desktop, as Menus.asi does (menus/README.md,
/// item 1); measured at DllMain, before the engine changes any mode.
fn raise_shell_mode() {
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    if w < DESIGN_W || h < DESIGN_H {
        crate::log!("menus: desktop {}x{} is smaller than the shell: its display mode is left at 800x600", w, h);
        return;
    }
    let mut new = MODE_STOCK;
    new[1] = 32;
    new[3..7].copy_from_slice(&(h as u32).to_le_bytes());
    new[8..12].copy_from_slice(&(w as u32).to_le_bytes());
    let n = MODE_SITES.iter().filter(|&&at| hook::patch_bytes(at, &MODE_STOCK, &new)).count();
    crate::log!("menus: shell display mode -> {}x{}x32 ({} of 2 sites)", w, h, n);
}

// ---- calling user32 through the originals --------------------------------------------

macro_rules! orig {
    ($o:ident, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {
        unsafe { core::mem::transmute::<usize, unsafe extern "system" fn($($t),*) -> $r>($o)($($a),*) }
    };
}

fn real_move(h: HWND, x: i32, y: i32, w: i32, ht: i32, rp: i32) -> i32 {
    orig!(O_MOVE, fn(HWND, i32, i32, i32, i32, i32) -> i32, h, x, y, w, ht, rp)
}

// ---- the dialogs -------------------------------------------------------------------

/// A copy of the dialog template with its pop-up styling replaced by a hidden child's.
/// None: create it stock.
fn child_template(inst: HANDLE, name: *const u8) -> Option<Vec<u8>> {
    unsafe {
        let rs = FindResourceA(inst, name, RT_DIALOG as *const u8);
        if rs.is_null() {
            return None;
        }
        let g = LoadResource(inst, rs);
        let src = if g.is_null() { core::ptr::null() } else { LockResource(g) };
        let n = SizeofResource(inst, rs) as usize;
        if src.is_null() || n < 18 {
            return None;
        }
        let mut v = core::slice::from_raw_parts(src, n).to_vec();
        let ex_first = u16::from_le_bytes([v[0], v[1]]) == 1 && u16::from_le_bytes([v[2], v[3]]) == 0xffff;
        let (so, eo) = if ex_first { (12, 8) } else { (0, 4) };
        let mut style = u32::from_le_bytes(v[so..so + 4].try_into().ok()?);
        let mut ex = u32::from_le_bytes(v[eo..eo + 4].try_into().ok()?);
        style &= !(WS_POPUP | WS_VISIBLE | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | DS_MODALFRAME | DS_SYSMODAL | DS_SETFOREGROUND);
        style |= WS_CHILD | WS_CLIPSIBLINGS;
        ex &= !(WS_EX_TOPMOST | WS_EX_APPWINDOW);
        v[so..so + 4].copy_from_slice(&style.to_le_bytes());
        v[eo..eo + 4].copy_from_slice(&ex.to_le_bytes());
        Some(v)
    }
}

fn template_id(name: *const u8) -> usize {
    if (name as usize) < 0x10000 { name as usize } else { 0 }
}

/// `Trace=1`: a dialog's messages of note, with what its procedure answered.
fn trace(h: HWND, msg: u32, wp: usize, lp: isize, r: isize) {
    // Not the frequent ones: paint, timers, the pointer, colours, hit-testing.
    if matches!(msg, 0x0f | 0x14 | 0x113 | 0x200 | 0x20 | 0x84 | 0x133..=0x138 | 0x2b | 0x317 | 0x318 | 0x0d | 0x0e | 0x7f) {
        return;
    }
    static mut N: u32 = 0;
    unsafe {
        N += 1;
        if N > 400 {
            return;
        }
    }
    crate::log!("menus: trace {:p} msg {:#x} wp {:#x} lp {:#x} -> {:#x}", h, msg, wp, lp, r);
}

/// The stock dialogs' procedures while `Host=0` traces them, innermost last.
static mut OBSERVED: Vec<(usize, usize)> = Vec::new();

/// `Host=0`: the game's procedure, traced, in a stock dialog.
unsafe extern "system" fn observe_proc(h: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    let v = unsafe { &mut *core::ptr::addr_of_mut!(OBSERVED) };
    let p = match v.iter().find(|o| o.0 == h as usize) {
        Some(o) => o.1,
        None => match v.iter_mut().rev().find(|o| o.0 == 0) {
            Some(o) => {
                o.0 = h as usize;
                o.1
            }
            None => return 0,
        },
    };
    let r = unsafe { core::mem::transmute::<usize, DlgProc>(p)(h, msg, wp, lp) };
    trace(h, msg, wp, lp, r);
    r
}

/// The dialog procedure of every hosted dialog: the game's, after the window is known.
unsafe extern "system" fn host_proc(h: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    let s = match slot_of(h) {
        Some(s) => s,
        None => {
            // The first message (WM_SETFONT or WM_INITDIALOG) of the dialog being made:
            // creation is synchronous, so the pending one is it.
            let Some(mut p) = (unsafe { (*core::ptr::addr_of_mut!(PENDING)).take() }) else {
                return 0;
            };
            let Some(free) = slots().iter_mut().find(|s| s.hwnd.is_null()) else {
                crate::log!("menus: no room for another hosted dialog");
                return 0;
            };
            p.hwnd = h;
            p.surface = Surface::make();
            if p.depth > 0 {
                p.frame = Surface::make();
            }
            p.paint = true;
            p.erase = true;
            let mut r = WinRect::default();
            unsafe { GetClientRect(h, &mut r) };
            p.rect = WinRect { left: 0, top: 0, right: r.right.min(DESIGN_W), bottom: r.bottom.min(DESIGN_H) };
            *free = p;
            free
        }
    };
    let proc_ = s.proc_;
    if msg == WM_NCDESTROY {
        let r = if proc_ != 0 { unsafe { core::mem::transmute::<usize, DlgProc>(proc_)(h, msg, wp, lp) } } else { 0 };
        if let Some(s) = slot_of(h) {
            s.surface.free();
            s.frame.free();
            *s = Slot::EMPTY;
        }
        return r;
    }
    if proc_ == 0 {
        return 0;
    }
    let r = unsafe { core::mem::transmute::<usize, DlgProc>(proc_)(h, msg, wp, lp) };
    if crate::config::get().menus_trace {
        trace(h, msg, wp, lp, r);
    }
    r
}

/// Why a dialog cannot be hosted, or None.
fn refuse(proc_: usize, owner: HWND) -> Option<&'static str> {
    if !ready() || !crate::config::get().menus {
        return Some("menus off");
    }
    if proc_ < SHELL_CODE.0 || proc_ >= SHELL_CODE.1 {
        return Some("not a shell screen");
    }
    let game = crate::input::window();
    if game.is_null() {
        return Some("no 3D window");
    }
    if crate::engine::d3d8_device().is_null() || crate::d3d::Device::from_d3d8(crate::engine::d3d8_device()).is_none() {
        return Some("no Direct3D 9 device to draw with");
    }
    if !owner.is_null() && owner != game && !is_hosted(owner) {
        return Some("its owner is a window the UI does not host");
    }
    if unsafe { DEPTH } >= 8 {
        return Some("nested too deep");
    }
    None
}

unsafe extern "system" fn my_dialog_box(inst: HANDLE, name: *const u8, owner: HWND, proc_: usize, lp: isize) -> isize {
    let real = |inst, name, owner, proc_, lp| orig!(O_DIALOG_BOX, fn(HANDLE, *const u8, HWND, usize, isize) -> isize, inst, name, owner, proc_, lp);
    let tpl = template_id(name);
    if ready() && !crate::config::get().menus_host {
        crate::log!("menus: observing dialog {} \"{}\"", tpl, screen_name(proc_));
        unsafe { (*core::ptr::addr_of_mut!(OBSERVED)).push((0, proc_)) };
        let r = real(inst, name, owner, observe_proc as *const () as usize, lp);
        unsafe { (*core::ptr::addr_of_mut!(OBSERVED)).pop() };
        crate::log!("menus: observed dialog {} returned {}", tpl, r);
        return r;
    }
    if let Some(why) = refuse(proc_, owner) {
        if why != "menus off" {
            crate::log!("menus: dialog {} (proc {:#x}) runs stock: {}", tpl, proc_, why);
        }
        return real(inst, name, owner, proc_, lp);
    }
    let Some(t) = child_template(inst, name) else {
        crate::log!("menus: dialog {} (proc {:#x}) runs stock: its template cannot be read", tpl, proc_);
        return real(inst, name, owner, proc_, lp);
    };
    let depth = unsafe { DEPTH } + 1;
    unsafe {
        PENDING = Some(Slot { proc_, owner, template: tpl, proc_addr: proc_, depth, ..Slot::EMPTY });
        DEPTH = depth;
    }
    let game = crate::input::window();
    crate::input::set_hosting(true);
    let h = unsafe { CreateDialogIndirectParamA(inst, t.as_ptr(), game, host_proc, lp) };
    unsafe { PENDING = None };
    if h.is_null() || slot_of(h).is_none() {
        unsafe { DEPTH = depth - 1 };
        if unsafe { DEPTH } == 0 {
            crate::input::set_hosting(false);
        }
        crate::log!("menus: dialog {} (proc {:#x}) could not be created hidden; running it stock", tpl, proc_);
        return real(inst, name, owner, proc_, lp);
    }
    log_dialog("open", h);
    let r = run_modal(h);
    unsafe { DEPTH = depth - 1 };
    if unsafe { DEPTH } == 0 {
        crate::input::set_hosting(false);
    }
    r
}

unsafe extern "system" fn my_create_dialog(inst: HANDLE, name: *const u8, parent: HWND, proc_: usize, lp: isize) -> HWND {
    let real = |inst, name, parent, proc_, lp| orig!(O_CREATE_DIALOG, fn(HANDLE, *const u8, HWND, usize, isize) -> HWND, inst, name, parent, proc_, lp);
    if !is_hosted(parent) {
        return real(inst, name, parent, proc_, lp);
    }
    let Some(mut t) = child_template(inst, name) else { return real(inst, name, parent, proc_, lp) };
    // A pane shows as its template says (the shell shows and hides them itself).
    let _ = &mut t;
    unsafe { PENDING = Some(Slot { proc_, owner: parent, template: template_id(name), proc_addr: proc_, depth: 0, ..Slot::EMPTY }) };
    let h = unsafe { CreateDialogIndirectParamA(inst, t.as_ptr(), parent, host_proc, lp) };
    unsafe { PENDING = None };
    if !h.is_null() {
        log_dialog("pane", h);
    }
    h
}

unsafe extern "system" fn my_end_dialog(h: HWND, r: isize) -> i32 {
    if let Some(s) = slot_of(h) {
        if s.depth > 0 {
            s.ended = true;
            s.result = r;
            return 1;
        }
    }
    orig!(O_END_DIALOG, fn(HWND, isize) -> i32, h, r)
}

fn log_dialog(what: &str, h: HWND) {
    if let Some(s) = slot_of(h) {
        crate::log!(
            "menus: {} dialog {} \"{}\" (proc {:#x}) depth {} hwnd {:p} {}x{} @{},{}",
            what, s.template, screen_name(s.proc_addr), s.proc_addr, s.depth, h, s.w(), s.h(), s.rect.left, s.rect.top
        );
    }
}

// ---- painting ---------------------------------------------------------------------

unsafe extern "system" fn my_get_dc(h: HWND) -> HDC {
    if let Some(s) = slot_of(h) {
        if s.surface.ok() {
            s.drawn = s.drawn.wrapping_add(1);
            // Whatever a stock helper left selected in it, the surface is what draws.
            s.surface.reselect();
            return s.surface.dc;
        }
    }
    if hosting() && !h.is_null() && h == crate::input::window() {
        unsafe {
            if !SCRATCH.ok() {
                SCRATCH = Surface::make();
            }
            if SCRATCH.ok() {
                return SCRATCH.dc;
            }
        }
    }
    orig!(O_GET_DC, fn(HWND) -> HDC, h)
}

unsafe extern "system" fn my_release_dc(h: HWND, dc: HDC) -> i32 {
    if is_surface(dc) {
        return 1;
    }
    orig!(O_RELEASE_DC, fn(HWND, HDC) -> i32, h, dc)
}

unsafe extern "system" fn my_begin_paint(h: HWND, ps: *mut PaintStruct) -> HDC {
    if let Some(s) = slot_of(h) {
        if s.surface.ok() && !ps.is_null() {
            unsafe {
                ValidateRect(h, core::ptr::null());
                core::ptr::write_bytes(ps, 0, 1);
                (*ps).hdc = s.surface.dc;
                (*ps).rc = WinRect { left: 0, top: 0, right: s.w().max(1), bottom: s.h().max(1) };
            }
            s.drawn = s.drawn.wrapping_add(1);
            // Whatever a stock helper left selected in it, the surface is what draws.
            s.surface.reselect();
            return s.surface.dc;
        }
    }
    orig!(O_BEGIN_PAINT, fn(HWND, *mut PaintStruct) -> HDC, h, ps)
}

unsafe extern "system" fn my_end_paint(h: HWND, ps: *const PaintStruct) -> i32 {
    if !ps.is_null() && is_surface(unsafe { (*ps).hdc }) {
        return 1;
    }
    orig!(O_END_PAINT, fn(HWND, *const PaintStruct) -> i32, h, ps)
}

unsafe extern "system" fn my_invalidate(h: HWND, r: *const WinRect, erase: i32) -> i32 {
    if let Some(s) = slot_of(h) {
        s.paint = true;
        s.erase |= erase != 0;
    }
    orig!(O_INVALIDATE, fn(HWND, *const WinRect, i32) -> i32, h, r, erase)
}

unsafe extern "system" fn my_redraw(h: HWND, r: *const WinRect, rgn: HANDLE, flags: u32) -> i32 {
    // RDW_INVALIDATE 1, RDW_ERASE 4.
    if let Some(s) = slot_of(h) {
        if flags & 1 != 0 {
            s.paint = true;
            s.erase |= flags & 4 != 0;
        }
    }
    orig!(O_REDRAW, fn(HWND, *const WinRect, HANDLE, u32) -> i32, h, r, rgn, flags)
}

/// Sends the paint the shell asked for to every hosted dialog that wants one: the
/// erase first if it was asked for, as BeginPaint would.
///
/// Only what shows is painted, as Windows sends `WM_PAINT` only to a window with
/// something visible: a dialog under a full-screen one waits until it is uncovered.
/// This matters: the shell keeps its screen's background in globals (the
/// `SetCurrentBackground` copy), and a covered main menu painting itself under Game
/// Setup put its own back, and Game Setup's player rows were never drawn again.
fn paint_pending() {
    let lowest = covered_below();
    for s in slots().iter_mut() {
        if s.hwnd.is_null() || !s.paint || !s.surface.ok() {
            continue;
        }
        let modal = if s.depth > 0 { s.depth } else { owner_depth(s.hwnd) };
        if modal < lowest {
            continue;
        }
        let (h, dc, erase) = (s.hwnd, s.surface.dc, s.erase);
        s.paint = false;
        s.erase = false;
        unsafe {
            if erase {
                SendMessageA(h, WM_ERASEBKGND, dc as usize, 0);
            }
            SendMessageA(h, WM_PAINT, 0, 0);
        }
        if let Some(s) = slot_of(h) {
            s.surface.reselect();
        }
    }
}

/// The depth of the innermost full-screen dialog: everything below it is hidden.
pub fn covered_below() -> u32 {
    stack().iter().filter(|s| s.w() >= DESIGN_W && s.h() >= DESIGN_H).map(|s| s.depth).max().unwrap_or(0)
}

/// The depth of the modal dialog a pane is in.
fn owner_depth(h: HWND) -> u32 {
    let mut w = h;
    for _ in 0..8 {
        match slot_of(w) {
            Some(s) if s.depth > 0 => return s.depth,
            _ => {}
        }
        w = unsafe { GetParent(w) };
        if w.is_null() {
            break;
        }
    }
    0
}

// ---- geometry ----------------------------------------------------------------------

/// The geometry a hosted dialog gets for what the shell asked (`x`, `y` in screen
/// coordinates): its design rectangle in its parent's client area. A dialog the shell
/// makes bigger than the design space is a full-screen one, which the shell sizes to
/// the 3D window (`init_screen_pos`): it is the design space.
fn place(h: HWND, x: i32, y: i32, w: i32, ht: i32, have_pos: bool, have_size: bool) -> WinRect {
    let s = slot_of(h).map(|s| s.rect).unwrap_or_default();
    let (mut nx, mut ny, mut nw, mut nh) = (s.left, s.top, s.right - s.left, s.bottom - s.top);
    if have_size {
        nw = w;
        nh = ht;
    }
    if have_pos {
        let mut p = Point { x, y };
        unsafe { ScreenToClient(GetParent(h), &mut p) };
        nx = p.x;
        ny = p.y;
    }
    if have_size && (w > DESIGN_W || ht > DESIGN_H) {
        return WinRect { left: 0, top: 0, right: DESIGN_W, bottom: DESIGN_H };
    }
    WinRect { left: nx, top: ny, right: nx + nw, bottom: ny + nh }
}

fn moved(h: HWND, r: WinRect) {
    if let Some(s) = slot_of(h) {
        let changed = s.rect.left != r.left || s.rect.top != r.top || s.rect.right != r.right || s.rect.bottom != r.bottom;
        s.rect = r;
        if changed {
            s.paint = true;
            s.erase = true;
            log_dialog("placed", h);
        }
    }
}

unsafe extern "system" fn my_move(h: HWND, x: i32, y: i32, w: i32, ht: i32, rp: i32) -> i32 {
    if is_hosted(h) {
        let r = place(h, x, y, w, ht, true, true);
        let ok = real_move(h, r.left, r.top, r.right - r.left, r.bottom - r.top, 0);
        moved(h, r);
        return ok;
    }
    real_move(h, x, y, w, ht, rp)
}

unsafe extern "system" fn my_set_pos(h: HWND, after: HWND, x: i32, y: i32, w: i32, ht: i32, flags: u32) -> i32 {
    let real = |after, x, y, w, ht, flags| orig!(O_SET_POS, fn(HWND, HWND, i32, i32, i32, i32, u32) -> i32, h, after, x, y, w, ht, flags);
    if let Some(s) = slot_of(h) {
        let modal = s.depth > 0;
        let r = place(h, x, y, w, ht, flags & SWP_NOMOVE == 0, flags & SWP_NOSIZE == 0);
        let mut f = flags & !(SWP_NOMOVE | SWP_NOSIZE);
        if modal {
            f &= !SWP_SHOWWINDOW;
        }
        let ok = real(after, r.left, r.top, r.right - r.left, r.bottom - r.top, f | SWP_NOACTIVATE);
        moved(h, r);
        return ok;
    }
    real(after, x, y, w, ht, flags)
}

unsafe extern "system" fn my_show(h: HWND, cmd: i32) -> i32 {
    if let Some(s) = slot_of(h) {
        if s.depth > 0 {
            // A hosted dialog is never shown: the UI draws it.
            return 0;
        }
        s.paint = true;
    }
    orig!(O_SHOW, fn(HWND, i32) -> i32, h, cmd)
}

unsafe extern "system" fn my_get_window(h: HWND, cmd: u32) -> HWND {
    if cmd == GW_OWNER {
        if let Some(s) = slot_of(h) {
            if s.depth > 0 {
                return s.owner;
            }
        }
    }
    orig!(O_GET_WINDOW, fn(HWND, u32) -> HWND, h, cmd)
}

// ---- composing ------------------------------------------------------------------

/// The child windows of `h`, bottom of the z-order first.
pub fn children(h: HWND) -> Vec<HWND> {
    let mut v = Vec::new();
    let mut c = unsafe { GetWindow(h, GW_CHILD) };
    while !c.is_null() && v.len() < 256 {
        v.push(c);
        c = unsafe { GetWindow(c, GW_HWNDNEXT) };
    }
    v.reverse();
    v
}

pub fn style(h: HWND) -> u32 {
    unsafe { GetWindowLongA(h, GWL_STYLE) as u32 }
}

/// A window's rectangle in `in_`'s client coordinates.
pub fn rect_in(h: HWND, in_: HWND) -> WinRect {
    let mut r = WinRect::default();
    unsafe {
        GetWindowRect(h, &mut r);
        let mut a = Point { x: r.left, y: r.top };
        let mut b = Point { x: r.right, y: r.bottom };
        ScreenToClient(in_, &mut a);
        ScreenToClient(in_, &mut b);
        WinRect { left: a.x, top: a.y, right: b.x, bottom: b.y }
    }
}

/// Draws `h`'s panes and controls over a copy of its own surface into `dst` (which it
/// sits in at `ox`, `oy`).
fn compose_into(h: HWND, dst: &Surface, ox: i32, oy: i32) {
    for c in children(h) {
        let st = style(c);
        if st & WS_VISIBLE == 0 {
            continue;
        }
        let r = rect_in(c, h);
        if let Some(p) = slot_of(c) {
            // A pane: its own surface, then its controls.
            if p.surface.ok() {
                copy_bits(dst, ox + r.left, oy + r.top, &p.surface, r.right - r.left, r.bottom - r.top);
            }
            compose_into(c, dst, ox + r.left, oy + r.top);
            continue;
        }
        unsafe {
            let mut old = Point::default();
            SetViewportOrgEx(dst.dc, ox + r.left, oy + r.top, &mut old);
            SendMessageA(c, WM_PRINT, dst.dc as usize, (PRF_CLIENT | PRF_NONCLIENT | PRF_CHILDREN) as isize);
            SetViewportOrgEx(dst.dc, old.x, old.y, core::ptr::null_mut());
        }
        dst.reselect();
    }
}

/// Composes every hosted modal dialog's frame: its surface, its panes and its
/// controls, as the screen would show them.
fn compose_all() {
    let lowest = covered_below();
    for s in slots().iter() {
        if s.hwnd.is_null() || s.depth == 0 || s.depth < lowest || !s.surface.ok() || !s.frame.ok() {
            continue;
        }
        // The surface is copied by its bits, not through its DC: the shell selects its
        // own bitmaps into the DCs it is handed, and Game Setup's paint left our
        // surface's bitmap selected in a DC of its own, so a `BitBlt` from our DC read
        // a stale bitmap while the surface itself was current.
        copy_bits(&s.frame, 0, 0, &s.surface, DESIGN_W, DESIGN_H);
        s.frame.reselect();
        compose_into(s.hwnd, &s.frame, 0, 0);
    }
    unsafe { GdiFlush() };
}

/// Copies `w` x `h` pixels of `src` (from its top-left) into `dst` at `x`, `y`, by
/// their bits.
fn copy_bits(dst: &Surface, x: i32, y: i32, src: &Surface, w: i32, h: i32) {
    unsafe { GdiFlush() };
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w).min(DESIGN_W), (y + h).min(DESIGN_H));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let row = (x1 - x0) as usize * 4;
    for yy in y0..y1 {
        let s = ((yy - y) * DESIGN_W + (x0 - x)) as usize * 4;
        let d = (yy * DESIGN_W + x0) as usize * 4;
        unsafe { core::ptr::copy(src.bits.add(s), dst.bits.add(d), row) };
    }
}

// ---- driving -----------------------------------------------------------------------

/// Sends a hosted dialog (or a control in it) a mouse message at `x`, `y` in its own
/// client coordinates.
pub fn send_mouse(h: HWND, msg: u32, keys: usize, x: i32, y: i32) {
    let lp = ((y as u32 & 0xffff) << 16 | (x as u32 & 0xffff)) as isize;
    unsafe { SendMessageA(h, msg, keys, lp) };
    // A stock control that took the mouse (a list box while its button is down) would
    // get the real pointer's messages in its own place: the UI sends them instead.
    let cap = unsafe { GetCapture() };
    if !cap.is_null() && in_hosted(cap) && msg != WM_LBUTTONDOWN && msg != WM_RBUTTONDOWN {
        unsafe { ReleaseCapture() };
    }
}

/// The deepest visible, enabled control of `dlg` (or a pane in it) under `x`, `y`
/// (`dlg`'s client coordinates), with the point in its own coordinates; `dlg` itself
/// when there is none.
pub fn hit(dlg: HWND, x: i32, y: i32) -> (HWND, i32, i32) {
    let mut cur = dlg;
    let (mut lx, mut ly) = (x, y);
    'down: for _ in 0..8 {
        // Topmost first.
        let mut kids = children(cur);
        kids.reverse();
        for c in kids {
            let st = style(c);
            if st & WS_VISIBLE == 0 {
                continue;
            }
            let r = rect_in(c, cur);
            if lx >= r.left && lx < r.right && ly >= r.top && ly < r.bottom {
                if st & WS_DISABLED != 0 {
                    break 'down;
                }
                lx -= r.left;
                ly -= r.top;
                cur = c;
                continue 'down;
            }
        }
        break;
    }
    (cur, lx, ly)
}

/// A dialog's place in the design space (its parents' offsets added).
pub fn design_origin(h: HWND) -> (i32, i32) {
    let game = crate::input::window();
    let r = rect_in(h, game);
    (r.left, r.top)
}

// ---- the loop ----------------------------------------------------------------------

/// One drain of the queue. Input for a hosted window is the UI's (noted for it, and
/// keys go through `IsDialogMessageA` as user32's modal loop would); input for the 3D
/// window reaches its procedure, whose hooks note it and keep it from the engine
/// (`input::set_hosting`). Some(code) on `WM_QUIT`.
fn pump(top: HWND) -> Option<i32> {
    let mut m = Msg { hwnd: core::ptr::null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, pt: Point::default() };
    let game = crate::input::window();
    let cfg = crate::config::get();
    unsafe {
        while PeekMessageA(&mut m, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if m.message == WM_QUIT {
                return Some(m.wparam as i32);
            }
            let key = (WM_KEYDOWN..=WM_KEYLAST).contains(&m.message);
            let mouse = (WM_MOUSEMOVE..=WM_MOUSELAST).contains(&m.message);
            if (key || mouse) && in_hosted(m.hwnd) {
                if key {
                    crate::input::note_message(m.message, m.wparam);
                    // IsDialogMessageA translates the key itself: a TranslateMessage here
                    // too typed every character twice.
                    if crate::menus::wants_keys() {
                        // A text field of ours has the keyboard: it sends the stock edit
                        // what changes (setup_kit::edit). The key reaches no stock control,
                        // or every character was typed twice (the stock edit holding the
                        // focus, as Multiplayer Connection's name edit does) or went to an
                        // edit of a dialog under the top one; it is only translated, so
                        // its WM_CHAR comes back here for our field.
                        TranslateMessage(&m);
                    } else if !escape(&m, top, cfg.menus_escape) {
                        IsDialogMessageA(top, &mut m);
                    }
                }
                // The real pointer's messages to a hosted window are not its: the UI
                // sends each dialog the pointer in its own design place.
                continue;
            }
            if key && m.hwnd == game && m.message != WM_SYSKEYDOWN && m.message != WM_SYSKEYUP && m.message != WM_SYSCHAR {
                // The 3D window holds the focus: its procedure notes the key for the UI
                // and keeps it from the engine. The dialog gets it as user32's modal loop
                // gives a dialog its keys: Enter and Esc as IDOK and IDCANCEL.
                TranslateMessage(&m);
                DispatchMessageA(&m);
                if m.message == WM_KEYDOWN && !crate::menus::wants_keys() && !escape(&m, top, cfg.menus_escape) {
                    let vk = m.wparam as u8;
                    if (vk == VK_RETURN || vk == VK_ESCAPE) && m.lparam & (1 << 30) == 0 {
                        let mut d = m;
                        d.hwnd = top;
                        IsDialogMessageA(top, &mut d);
                    }
                }
                continue;
            }
            if escape(&m, top, cfg.menus_escape) {
                continue;
            }
            if m.message == WM_SYSKEYDOWN && (m.wparam == 0x09 || m.wparam == 0x1b) && rd::<u8>(shell::PROGRAM) == 0 {
                continue;
            }
            TranslateMessage(&m);
            DispatchMessageA(&m);
        }
    }
    None
}

/// Esc in the in-mission menu: a click on Return to Game, through the menu's own
/// procedure (the same sound, result and EndDialog as the mouse; menus/README.md,
/// "Esc"). True when it was that.
fn escape(m: &Msg, top: HWND, on: bool) -> bool {
    // With no window holding the focus, keys come as WM_SYSKEYDOWN; Esc without Alt
    // (lParam bit 29) is still Esc.
    let down = m.message == WM_KEYDOWN || (m.message == WM_SYSKEYDOWN && m.lparam & (1 << 29) == 0);
    if !on || !down || m.wparam as u8 != VK_ESCAPE || m.lparam & (1 << 30) != 0 {
        return false;
    }
    let Some(s) = slot_of(top) else { return false };
    if s.proc_addr != ESCAPE_MENU_PROC {
        return false;
    }
    crate::log!("menus: Esc -> Return to Game");
    crate::menus::actions::click_design(top, ESCAPE_RETURN_AT.0, ESCAPE_RETURN_AT.1);
    true
}

/// Runs a hosted modal dialog until it ends, drawing a frame through the device every
/// pass, and returns what it passed to `EndDialog`.
fn run_modal(h: HWND) -> isize {
    let game = crate::input::window();
    shell::arrow_cursor(game, true);
    // Wine leaves the focus nowhere after some menus (menus/README.md, "Keyboard
    // focus"); then keys arrive as system keys and the dialog never sees Enter or Esc.
    unsafe {
        if GetFocus().is_null() && !game.is_null() {
            SetFocus(game);
        }
        // A dialog opened by a click on another (Game Setup's SELECT MAP) opens while
        // the outer one may still hold the mouse capture its button press took; every
        // press would then go to that hidden window and never reach the UI.
        let cap = GetCapture();
        if !cap.is_null() && in_hosted(cap) {
            crate::log!("menus: dialog {:p} opens with the capture held by {:p}: released", h, cap);
            ReleaseCapture();
        }
    }
    // A click that opened this dialog was a button down that a hosted window took the
    // mouse on (the campaign selection opens the mission list on the down); its up is
    // sent after this loop ends. Meanwhile the real pointer's messages would all go to
    // that hidden window and be dropped, so the dialog would take no click.
    unsafe {
        let cap = GetCapture();
        if !cap.is_null() && in_hosted(cap) {
            ReleaseCapture();
        }
    }
    let mut freq = 0i64;
    unsafe { QueryPerformanceFrequency(&mut freq) };
    let period = (freq / 60).max(1);
    let (mut frames, mut lost) = (0u32, 0u32);
    let started = crate::rt::now();
    let mut last_report = started;
    let depth = slot_of(h).map_or(0, |s| s.depth);
    if depth == 1 {
        crate::menus::backdrop::begin(slot_of(h).map_or("", |s| screen_name(s.proc_addr)));
    }
    let result;
    loop {
        let mut t0 = 0i64;
        unsafe { QueryPerformanceCounter(&mut t0) };
        if let Some(code) = pump(h) {
            unsafe {
                core::ptr::write_unaligned(shell::EXIT_CODE as *mut i32, code);
                PostQuitMessage(code);
            }
            crate::log!("menus: WM_QUIT ({}) while dialog {:p} was up", code, h);
            result = slot_of(h).map_or(0, |s| s.result);
            break;
        }
        match slot_of(h) {
            None => {
                result = 0;
                break;
            }
            Some(s) if s.ended => {
                result = s.result;
                break;
            }
            _ => {}
        }
        paint_pending();
        crate::menus::actions::run();
        if let Some(s) = slot_of(h) {
            if s.ended {
                result = s.result;
                break;
            }
        }
        compose_all();
        let now = crate::rt::now();
        if now - last_report >= 5.0 {
            last_report = now;
            if crate::config::get().menus_trace {
                for s in stack() {
                    dump(&s.surface, &crate::sfmt!("UI-dlg{}-surface.bmp", s.depth));
                    dump(&s.frame, &crate::sfmt!("UI-dlg{}-frame.bmp", s.depth));
                }
            }
            crate::log!("menus: dialog {:p} up, {} frames so far, top {:?}", h, frames, top().map(|t| screen_name(t.proc_addr)));
        }
        match frame() {
            Some(true) => frames += 1,
            Some(false) => lost += 1,
            None => {}
        }
        crate::menus::actions::run();
        if let Some(s) = slot_of(h) {
            if s.ended {
                result = s.result;
                break;
            }
        }
        let mut t1 = 0i64;
        unsafe { QueryPerformanceCounter(&mut t1) };
        let left = period - (t1 - t0);
        if left > 0 {
            unsafe { Sleep((left * 1000 / freq) as u32) };
        }
    }
    crate::log!(
        "menus: dialog {:p} returned {} after {} frames ({} lost) in {:.1} s",
        h, result, frames, lost, crate::rt::now() - started
    );
    crate::menus::closed(h);
    unsafe { DestroyWindow(h) };
    if let Some(s) = slot_of(h) {
        s.surface.free();
        s.frame.free();
        *s = Slot::EMPTY;
    }
    if depth == 1 {
        crate::menus::backdrop::end();
        shell::arrow_cursor(game, false);
    }
    result
}

/// `Trace=1`: a surface written to the game directory as a BMP, to see what a hidden
/// dialog drew.
fn dump(s: &Surface, name: &str) {
    if !s.ok() {
        return;
    }
    let (w, h) = (DESIGN_W as u32, DESIGN_H as u32);
    let size = w * h * 4;
    let mut v: Vec<u8> = Vec::with_capacity(54 + size as usize);
    v.extend_from_slice(b"BM");
    v.extend_from_slice(&(54 + size).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&54u32.to_le_bytes());
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(w as i32).to_le_bytes());
    v.extend_from_slice(&(-(h as i32)).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(&[0u8; 24]);
    v.extend_from_slice(unsafe { core::slice::from_raw_parts(s.bits, size as usize) });
    crate::rt::write_file(&crate::rt::game_path(name), &v);
}

/// One frame through the game's device: Some(true) presented, Some(false) the device is
/// lost, None no device.
fn frame() -> Option<bool> {
    let dev = crate::engine::d3d8_device();
    let st = shell::st3d_device();
    if dev.is_null() || st == 0 {
        unsafe { Sleep(20) };
        return None;
    }
    shell::test_cooperative(st);
    if super::dev8::test_cooperative_level(dev) < 0 {
        unsafe { Sleep(50) };
        return Some(false);
    }
    let bb = super::dev8::back_buffer_desc(dev)?;
    super::dev8::begin_scene(dev);
    super::dev8::set_viewport(dev, bb.width, bb.height);
    super::dev8::clear(dev, super::dev8::CLEAR_TARGET, 0xff000000);
    // The main menu's and the campaign selection's 3D scene, under everything of ours.
    crate::menus::scene::draw(bb.width, bb.height);
    crate::surface::frame(Context::Menus);
    super::dev8::end_scene(dev);
    super::dev8::present(dev);
    Some(true)
}
