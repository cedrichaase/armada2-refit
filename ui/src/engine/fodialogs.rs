//! FleetOpsHook's dialogs adopted (PLAN-FLEETOPS.md, FO-6 step 7, decisions 230 onwards;
//! ui/MENUS.md, "Roots' forms", "The nine dialogs, adopted").
//!
//! On Roots nine shell screens (Single Player, Game Setup, Game Settings, Load, Save, the
//! licence, the Technology Tree, the Admiral's Log, Internet) are still the executable's
//! dialogs, but FleetOpsHook opens them through its own import of `DialogBoxParamA`, with a
//! procedure of its own in front of the stock one: the dialog host never sees them
//! (decision 202). They are `#32770` pop-ups over the device window, which Wine makes
//! windows of their own that the window manager tiles and focuses (the user's report,
//! 2026-11-07). With the forms on, `engine/forms.rs` finds each one as it activates and
//! hands it here, where it is **adopted**:
//!
//! - **cut to the design size and moved off screen**, as the forms are (decision 170): the
//!   window manager no longer sees it (a window wholly off every output is not mapped);
//!   it keeps the activation, the keyboard focus and its modal loop, which stays user32's
//!   (FleetOpsHook's `DialogBoxParamA`);
//! - **painted into a surface of ours**: the stock procedures draw through the
//!   executable's imports of `GetDC`, `BeginPaint` and the rest, exactly as under the
//!   dialog host. Those import slots are pointed at functions of ours that answer an
//!   adopted dialog with a memory DC and pass every other window to user32. An off-screen
//!   window is never sent `WM_PAINT` and its own DC draws nothing, so `InvalidateRect` and
//!   `RedrawWindow` of an adopted dialog are noted and the pump sends the paint itself;
//! - **composed** into a picture each time something drew (its surface, then each visible
//!   child window printed with `WM_PRINT`, bottom of the z-order first), which the UI
//!   draws in its frame, fitted to the screen with the forms' design space;
//! - **read** as a control model: its child windows (class, id, text, rectangle, style),
//!   as the host's model reads a hosted dialog's (a spec sits on that; no spec yet);
//! - **driven** as a player would: the pointer the UI maps into the dialog's client
//!   coordinates is posted to the window under it (moves for the hover, a press and a
//!   release), the keys reach it as they would since it keeps the focus. The real
//!   pointer's own messages to an off-screen window can only come through a capture the
//!   dialog took and lie outside its client area: a message hook turns those into
//!   `WM_NULL`, so a held button never sees the pointer leave.
//!
//! Nothing of FleetOpsHook's memory is written; its record of the stock procedure is read
//! (once, logged). The executable's import slots are written only after each is checked
//! to hold user32's own function, and only while the dialog host is not installed (it
//! owns them on Armada II; on Roots it stands down).
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateEnhMetaFileA(reference: HDC, file: *const u8, frame: *const WinRect, desc: *const u8) -> HDC;
    fn CloseEnhMetaFile(dc: HDC) -> HANDLE;
    fn PlayEnhMetaFile(dc: HDC, emf: HANDLE, r: *const WinRect) -> i32;
    fn DeleteEnhMetaFile(emf: HANDLE) -> i32;
    fn GetEnhMetaFileHeader(emf: HANDLE, size: u32, hdr: *mut u8) -> u32;
}

const BS_OWNERDRAW: u32 = 0xb;

#[link(name = "user32")]
unsafe extern "system" {
    fn WindowFromDC(dc: HDC) -> HWND;
    fn SetWindowsHookExA(id: i32, f: unsafe extern "system" fn(i32, usize, isize) -> isize, module: HANDLE, thread: u32) -> HANDLE;
    fn CallNextHookEx(h: HANDLE, code: i32, wp: usize, lp: isize) -> isize;
}

const WH_GETMESSAGE: i32 = 3;
const GWL_EXSTYLE: i32 = -20;
const GWL_USERDATA: i32 = -21;
const DWL_DLGPROC: i32 = 4;
const WM_ERASEBKGND_: u32 = 0x14;
const WM_MOUSEWHEEL: u32 = 0x20a;
const WS_CHILD_: u32 = 0x4000_0000;
/// The top dialog is painted at least this often (seconds; decision 233).
const REPAINT_EVERY: f64 = 0.25;

/// A 32-bit top-down DIB section with its own memory DC.
#[derive(Clone, Copy)]
struct Dib {
    dc: HDC,
    bmp: HGDIOBJ,
    old: HGDIOBJ,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl Dib {
    const NONE: Dib = Dib { dc: core::ptr::null_mut(), bmp: core::ptr::null_mut(), old: core::ptr::null_mut(), bits: core::ptr::null_mut(), w: 0, h: 0 };

    fn make(w: i32, h: i32) -> Dib {
        if w <= 0 || h <= 0 {
            return Dib::NONE;
        }
        let bi = BitmapInfoHeader { size: 40, width: w, height: -h, planes: 1, bit_count: 32, ..Default::default() };
        unsafe {
            let dc = CreateCompatibleDC(core::ptr::null_mut());
            if dc.is_null() {
                return Dib::NONE;
            }
            let mut bits = core::ptr::null_mut();
            let bmp = CreateDIBSection(dc, &bi, 0, &mut bits, core::ptr::null_mut(), 0);
            if bmp.is_null() || bits.is_null() {
                DeleteDC(dc);
                return Dib::NONE;
            }
            let old = SelectObject(dc, bmp);
            PatBlt(dc, 0, 0, w, h, BLACKNESS);
            Dib { dc, bmp, old, bits, w, h }
        }
    }

    fn ok(&self) -> bool {
        !self.dc.is_null()
    }

    fn free(&mut self) {
        if self.ok() {
            unsafe {
                SelectObject(self.dc, self.old);
                DeleteObject(self.bmp);
                DeleteDC(self.dc);
            }
        }
        *self = Dib::NONE;
    }

    /// The stock code selects bitmaps of its own into the DCs it is handed and does not
    /// always put ours back (MENUS.md, "Its DC is not trusted to hold its bitmap").
    fn reselect(&self) {
        if self.ok() {
            unsafe { SelectObject(self.dc, self.bmp) };
        }
    }

    fn len(&self) -> usize {
        self.w as usize * self.h as usize * 4
    }
}

/// A child window of an adopted dialog, as read at the last compose.
#[derive(Clone, Default)]
pub struct Child {
    pub hwnd: usize,
    pub class: String,
    pub id: i32,
    pub text: String,
    /// In the dialog's client coordinates.
    pub rect: [i32; 4],
    pub style: u32,
    pub visible: bool,
    pub enabled: bool,
}

struct Adopted {
    hwnd: HWND,
    /// FleetOpsHook's record in the window's user data: the stock procedure and its
    /// parameter (read once, for the log and a spec's choice).
    stock_proc: usize,
    surface: Dib,
    picture: Dib,
    paint: bool,
    erase: bool,
    /// Bumped whenever the dialog draws (a DC of ours handed out).
    drawn: u32,
    composed: u32,
    composed_at: f64,
    painted_at: f64,
    serial: u32,
    children: Vec<Child>,
    logged: bool,
}

static mut LIST: Vec<Adopted> = Vec::new();
static mut READY: bool = false;
static mut MSG_HOOK: HANDLE = core::ptr::null_mut();
static mut DROPPED: u32 = 0;

static mut O_GET_DC: usize = 0;
static mut O_RELEASE_DC: usize = 0;
static mut O_BEGIN_PAINT: usize = 0;
static mut O_END_PAINT: usize = 0;
static mut O_INVALIDATE: usize = 0;
static mut O_REDRAW: usize = 0;

fn list() -> &'static mut Vec<Adopted> {
    unsafe { &mut *core::ptr::addr_of_mut!(LIST) }
}

fn find(h: HWND) -> Option<&'static mut Adopted> {
    if h.is_null() {
        return None;
    }
    list().iter_mut().find(|a| a.hwnd == h)
}

fn is_ours(dc: HDC) -> bool {
    !dc.is_null() && list().iter().any(|a| a.surface.dc == dc)
}

/// Whether the dialogs can be adopted: the paint imports are ours.
pub fn ready() -> bool {
    unsafe { READY }
}

/// An adopted dialog's stock procedure (0 when `h` is not adopted).
pub fn stock_proc_of(h: HWND) -> usize {
    find(h).map_or(0, |a| a.stock_proc)
}

pub fn is_adopted(h: HWND) -> bool {
    find(h).is_some()
}

/// Whether `h` is an adopted dialog or a window in one.
fn in_adopted(h: HWND) -> Option<HWND> {
    let mut w = h;
    for _ in 0..8 {
        if w.is_null() {
            return None;
        }
        if is_adopted(w) {
            return Some(w);
        }
        if unsafe { GetWindowLongA(w, GWL_STYLE) } as u32 & WS_CHILD_ == 0 {
            return None;
        }
        w = unsafe { GetParent(w) };
    }
    None
}

// ---- installing ------------------------------------------------------------------------

/// Points the executable's paint imports at ours (each slot checked to hold user32's own
/// function before any is written) and puts the message hook on the game's thread. False,
/// nothing written, when the dialog host has them (Armada II) or any slot is not user32's:
/// the dialogs are then framed as Win32 windows (decision 200).
pub fn install(thread: u32) -> bool {
    if super::dialogs::ready() {
        crate::log!("forms: the dialog host has the executable's imports: FleetOpsHook's dialogs stay framed windows");
        return false;
    }
    install_background();
    let hooks: [(&str, usize, *mut usize); 6] = [
        ("GetDC", my_get_dc as *const () as usize, core::ptr::addr_of_mut!(O_GET_DC)),
        ("ReleaseDC", my_release_dc as *const () as usize, core::ptr::addr_of_mut!(O_RELEASE_DC)),
        ("BeginPaint", my_begin_paint as *const () as usize, core::ptr::addr_of_mut!(O_BEGIN_PAINT)),
        ("EndPaint", my_end_paint as *const () as usize, core::ptr::addr_of_mut!(O_END_PAINT)),
        ("InvalidateRect", my_invalidate as *const () as usize, core::ptr::addr_of_mut!(O_INVALIDATE)),
        ("RedrawWindow", my_redraw as *const () as usize, core::ptr::addr_of_mut!(O_REDRAW)),
    ];
    let user32 = unsafe { GetModuleHandleA(c"user32.dll".as_ptr() as *const u8) };
    let mut slots = [0usize; 6];
    for (i, &(name, _, _)) in hooks.iter().enumerate() {
        let Some(slot) = super::dialogs::iat_entry("USER32.dll", name) else {
            crate::log!("forms: the executable does not import {}: FleetOpsHook's dialogs stay framed windows", name);
            return false;
        };
        let real = unsafe { GetProcAddress(user32, crate::rt::cstr(name).as_ptr()) };
        let cur = unsafe { super::rd::<usize>(slot) };
        if real == 0 || cur != real {
            crate::log!(
                "forms: the executable's import of {} is not user32's ({:#x}, user32 {:#x}): FleetOpsHook's dialogs stay framed windows",
                name, cur, real
            );
            return false;
        }
        slots[i] = slot;
    }
    for (i, &(name, ours, orig)) in hooks.iter().enumerate() {
        match super::hook::patch_slot(slots[i], 0, ours) {
            Some(o) => unsafe { *orig = o },
            None => {
                crate::log!("forms: could not point the import of {} at ours", name);
                return false;
            }
        }
    }
    unsafe {
        MSG_HOOK = SetWindowsHookExA(WH_GETMESSAGE, get_message, core::ptr::null_mut(), thread);
        READY = true;
    }
    crate::log!("forms: the executable's paint imports are ours for FleetOpsHook's dialogs (message hook {})", if unsafe { MSG_HOOK }.is_null() { "refused" } else { "on" });
    // The shell's own widgets (buttons, labels, rows: painted, not windows) are what a
    // screen spec reads (decision 250); the dialog host installs the reader on Armada II.
    crate::log!(
        "forms: the shell's widgets {}",
        if super::shellui::install() { "are read (screen specs for the adopted dialogs)" } else { "are not read: the adopted dialogs are mirrored" }
    );
    true
}

// ---- the shell background (decision 300) -----------------------------------------------

/// The trampoline `DrawTransparentBitmap` runs through when the hook lets a draw pass: its
/// first instructions, then a jump back. 0 until installed.
static mut BG_TRAMP: usize = 0;
#[repr(align(16))]
struct Tramp([u8; 32]);
static mut BG_CODE: Tramp = Tramp([0xcc; 32]);
static mut BG_SKIPPED: u32 = 0;

/// Game Setup's paint has the shell set its background, which on Roots is FleetOpsHook's
/// `SetCurrentBackgroundNew`: it keeps the picture in `hCurrentBackground`, then draws it
/// with the executable's `DrawTransparentBitmap` onto the game window (the window
/// `ST3D_GraphicsEngine::GetWindowHandle` names), through a DC from its own `GetDC`, which
/// no import of the executable passes. That GDI draw lands on the device window at 1:1
/// and shows over the frame until the swap chain's next image covers it (decision 258,
/// measured with Wine's relay in decision 300). The function is taken at its entry: a draw
/// onto the device window while the forms' frame covers it is skipped, every other draw
/// (the stock procedures' own, into the surfaces of ours) runs as it was.
fn install_background() {
    use crate::syms_forms::{hook, sym, TABLE};
    let e = TABLE.engine();
    if let Err(why) = e.ready(hook::FORMS_SHELL_BACKGROUND) {
        crate::log!("forms: the shell background is not kept off the game window ({}): Game Setup may flash its background", why);
        return;
    }
    let at = e.addr(sym::FORMS_DRAW_TRANSPARENT);
    let check = e.check_bytes(sym::FORMS_DRAW_TRANSPARENT);
    let code = unsafe { &mut (*core::ptr::addr_of_mut!(BG_CODE)).0 };
    if !super::hook::patch_entry(at, check, check.len(), draw_transparent as *const () as usize, code) {
        crate::log!("forms: DrawTransparentBitmap's entry ({:#x}) is not stock: Game Setup may flash its background", at);
        return;
    }
    unsafe { BG_TRAMP = code.as_ptr() as usize };
    crate::log!("forms: DrawTransparentBitmap taken at its entry ({:#x}): the shell background stays off the game window under the frame", at);
}

/// `DrawTransparentBitmap(dc, bitmap, x, y, key, dc)`, cdecl (the two shorts pushed as
/// words), at its entry.
unsafe extern "C" fn draw_transparent(dc: HDC, bmp: HGDIOBJ, x: i32, y: i32, key: u32, other: HDC) {
    if super::forms::covers(unsafe { WindowFromDC(dc) }) {
        let n = unsafe {
            BG_SKIPPED = BG_SKIPPED.wrapping_add(1);
            BG_SKIPPED
        };
        if n == 1 || (crate::config::get().menus_trace && n % 50 == 0) {
            crate::log!("forms: the shell background's draw onto the game window skipped ({} so far)", n);
        }
        return;
    }
    let f: unsafe extern "C" fn(HDC, HGDIOBJ, i32, i32, u32, HDC) = unsafe { core::mem::transmute(BG_TRAMP) };
    unsafe { f(dc, bmp, x, y, key, other) }
}

// ---- adopting --------------------------------------------------------------------------

/// Takes a dialog (already cut to its place and moved off screen by `forms`): a surface of
/// its client size, a paint and a compose at the next tick, its record and children logged.
pub fn adopt(h: HWND) {
    if find(h).is_some() {
        return;
    }
    let (w, ht) = client_of(h);
    // FleetOpsHook keeps {stock procedure, stock parameter} in the window's user data.
    let rec = unsafe { GetWindowLongA(h, GWL_USERDATA) } as usize;
    let mut stock_proc = if super::readable(rec, 8) && rec % 4 == 0 { unsafe { super::rd::<usize>(rec) } } else { 0 };
    // One the executable opens through its own import (the mission list, the one-button
    // message) has no record: its procedure is the dialog's own (decision 250).
    if stock_proc == 0 {
        stock_proc = unsafe { GetWindowLongA(h, DWL_DLGPROC) } as usize;
    }
    crate::log!(
        "forms: #32770 {:p} adopted: client {}x{}, style {:#010x} ex {:#010x}, stock procedure {:#x}",
        h, w, ht, unsafe { GetWindowLongA(h, GWL_STYLE) } as u32, unsafe { GetWindowLongA(h, GWL_EXSTYLE) } as u32, stock_proc
    );
    list().push(Adopted {
        hwnd: h,
        stock_proc,
        surface: Dib::make(w, ht),
        picture: Dib::make(w, ht),
        paint: true,
        erase: true,
        drawn: 0,
        composed: u32::MAX,
        composed_at: 0.0,
        painted_at: 0.0,
        serial: 0,
        children: Vec::new(),
        logged: false,
    });
}

/// The dialog is gone (destroyed).
pub fn forget(h: HWND) {
    let v = list();
    if let Some(i) = v.iter().position(|a| a.hwnd == h) {
        let mut a = v.remove(i);
        crate::log!("forms: #32770 {:p} released ({} paints, {} pictures)", h, a.drawn, a.serial);
        a.surface.free();
        a.picture.free();
    }
}

fn client_of(h: HWND) -> (i32, i32) {
    let mut r = WinRect::default();
    unsafe { GetClientRect(h, &mut r) };
    (r.right - r.left, r.bottom - r.top)
}

/// Each tick, for every adopted dialog: its surface follows its size, the paint it asked
/// for is sent, and its picture is composed again when it drew (or every `every` seconds,
/// for what draws without asking: a caret, a control's own timer).
pub fn tick(every: f64, top: HWND) {
    let now = crate::rt::now();
    let hs: Vec<HWND> = list().iter().map(|a| a.hwnd).collect();
    for h in hs {
        if unsafe { IsWindow(h) } == 0 {
            forget(h);
            continue;
        }
        let (w, ht) = client_of(h);
        if let Some(a) = find(h) {
            if a.surface.w != w || a.surface.h != ht {
                a.surface.free();
                a.picture.free();
                a.surface = Dib::make(w, ht);
                a.picture = Dib::make(w, ht);
                a.paint = true;
                a.erase = true;
            }
        }
        // The top one is painted again when it has not asked for a while: an invalidation
        // that does not come through the executable's imports (FleetOpsHook's own, user32's)
        // never reaches an off-screen window as a paint (decision 233).
        if let Some(a) = find(h) {
            if (h == top || unsafe { GetWindow(h, GW_OWNER) } == top) && now - a.painted_at >= REPAINT_EVERY {
                a.paint = true;
            }
        }
        paint(h);
        let Some(a) = find(h) else { continue };
        if a.drawn != a.composed || now - a.composed_at >= every {
            a.composed = a.drawn;
            a.composed_at = now;
            compose(a);
        }
        if !a.logged && a.serial > 0 {
            a.logged = true;
            log_children(a);
        }
    }
}

/// The paint the dialog asked for, through our surface: the erase first if asked, as
/// `BeginPaint` would.
fn paint(h: HWND) {
    let Some(a) = find(h) else { return };
    if !a.paint || !a.surface.ok() {
        return;
    }
    let (dc, erase) = (a.surface.dc, a.erase);
    a.paint = false;
    a.painted_at = crate::rt::now();
    a.erase = false;
    unsafe {
        if erase {
            SendMessageA(h, WM_ERASEBKGND_, dc as usize, 0);
        }
        SendMessageA(h, WM_PAINT, 0, 0);
    }
    if let Some(a) = find(h) {
        a.surface.reselect();
    }
}

/// The picture: the surface's bits, then each visible child printed at its place.
fn compose(a: &mut Adopted) {
    if !a.surface.ok() || !a.picture.ok() {
        return;
    }
    unsafe {
        GdiFlush();
        core::ptr::copy_nonoverlapping(a.surface.bits, a.picture.bits, a.picture.len());
    }
    a.picture.reselect();
    let mut kids = Vec::new();
    for c in super::dialogs::children(a.hwnd) {
        let st = super::dialogs::style(c);
        let r = super::dialogs::rect_in(c, a.hwnd);
        kids.push(Child {
            hwnd: c as usize,
            class: class_of(c),
            id: unsafe { GetDlgCtrlID(c) },
            text: text_of(c),
            rect: [r.left, r.top, r.right - r.left, r.bottom - r.top],
            style: st,
            visible: st & WS_VISIBLE != 0,
            enabled: st & WS_DISABLED == 0,
        });
        if st & WS_VISIBLE == 0 {
            continue;
        }
        if st & 0xf == BS_OWNERDRAW && print_recorded(c, a.picture.dc, r.left, r.top) {
            a.picture.reselect();
            continue;
        }
        unsafe {
            let mut old = Point::default();
            SetViewportOrgEx(a.picture.dc, r.left, r.top, &mut old);
            SendMessageA(c, WM_PRINT, a.picture.dc as usize, (PRF_CLIENT | PRF_NONCLIENT | PRF_CHILDREN) as isize);
            SetViewportOrgEx(a.picture.dc, old.x, old.y, core::ptr::null_mut());
        }
        a.picture.reselect();
    }
    unsafe { GdiFlush() };
    a.children = kids;
    a.serial = a.serial.wrapping_add(1);
    if crate::config::get().menus_trace && (a.serial == 2 || a.serial == 40) {
        dump(a);
    }
}

/// An owner-drawn button printed through a recording (decision 237). The Admiral's Log's
/// drawing helper selects a bitmap of its own into whatever DC its `WM_DRAWITEM` is handed
/// (MENUS.md, "What the shell is, in the engine"): into a memory DC that replaces the
/// picture itself and everything it draws is lost, while on a window's DC (the stock
/// screen) the selection fails and the drawing lands. A metafile DC refuses the selection
/// as a window's DC does; what it recorded is played into the picture at the button's
/// place, at its own size.
fn print_recorded(c: HWND, dst: HDC, x: i32, y: i32) -> bool {
    unsafe {
        let rec = CreateEnhMetaFileA(dst, core::ptr::null(), core::ptr::null(), core::ptr::null());
        if rec.is_null() {
            return false;
        }
        SendMessageA(c, WM_PRINT, rec as usize, (PRF_CLIENT | PRF_NONCLIENT | PRF_CHILDREN) as isize);
        let emf = CloseEnhMetaFile(rec);
        if emf.is_null() {
            return false;
        }
        // ENHMETAHEADER: rclBounds (inclusive device pixels) at +8.
        let mut hdr = [0i32; 22];
        let ok = GetEnhMetaFileHeader(emf, 88, hdr.as_mut_ptr() as *mut u8) != 0;
        let (l, t, rr, b) = (hdr[2], hdr[3], hdr[4], hdr[5]);
        if ok && rr >= l && b >= t {
            let to = WinRect { left: x + l, top: y + t, right: x + rr + 1, bottom: y + b + 1 };
            PlayEnhMetaFile(dst, emf, &to);
        }
        DeleteEnhMetaFile(emf);
        true
    }
}

fn class_of(h: HWND) -> String {
    let mut b = [0u8; 64];
    let n = unsafe { GetClassNameA(h, b.as_mut_ptr(), 64) }.max(0) as usize;
    String::from(core::str::from_utf8(&b[..n]).unwrap_or("?"))
}

fn text_of(h: HWND) -> String {
    let mut b = [0u8; 128];
    let n = unsafe { GetWindowTextA(h, b.as_mut_ptr(), 128) }.clamp(0, 127) as usize;
    b[..n].iter().map(|&c| if c < 0x80 { c as char } else { char::from_u32(c as u32).unwrap_or('?') }).collect()
}

/// The children, once, when the dialog first composes: where a spec's author finds them.
fn log_children(a: &Adopted) {
    let lit = if a.picture.ok() {
        let px = unsafe { core::slice::from_raw_parts(a.picture.bits as *const u32, (a.picture.w * a.picture.h) as usize) };
        px.iter().filter(|&&p| p & 0x00ff_ffff != 0).count() * 100 / px.len().max(1)
    } else {
        0
    };
    crate::log!(
        "forms: #32770 {:p} (stock procedure {:#x}) has {} child windows; {} paints so far, {}% of its picture drawn:",
        a.hwnd, a.stock_proc, a.children.len(), a.drawn, lit
    );
    for c in &a.children {
        crate::log!(
            "forms:   {} id {} {},{} {}x{} style {:#010x} {}{} text {:?}",
            c.class, c.id, c.rect[0], c.rect[1], c.rect[2], c.rect[3], c.style,
            if c.visible { "visible" } else { "hidden" }, if c.enabled { "" } else { " disabled" }, c.text
        );
    }
}

/// `[Menus] Trace=1`: the picture written to the game directory as a BMP.
fn dump(a: &Adopted) {
    let (w, h) = (a.picture.w as u32, a.picture.h as u32);
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
    v.extend_from_slice(unsafe { core::slice::from_raw_parts(a.picture.bits, size as usize) });
    let name = crate::sfmt!("forms-dialog-{:x}-{}.bmp", a.hwnd as usize, a.serial);
    crate::rt::write_file(&crate::rt::game_path(&name), &v);
    crate::log!("forms: probe: picture of #32770 {:p} written to {}", a.hwnd, name);
}

// ---- what the frame sees ---------------------------------------------------------------

/// An adopted dialog's picture: (bits, width, height, serial).
pub fn picture(h: HWND) -> Option<(*const u8, i32, i32, u32)> {
    let a = find(h)?;
    if !a.picture.ok() || a.serial == 0 {
        return None;
    }
    Some((a.picture.bits as *const u8, a.picture.w, a.picture.h, a.serial))
}

/// An adopted dialog's child windows, as last composed.
pub fn children(h: HWND) -> Vec<Child> {
    find(h).map(|a| a.children.clone()).unwrap_or_default()
}

/// FleetOpsHook's record of the stock procedure (0: not read).
pub fn stock_proc(h: HWND) -> usize {
    find(h).map_or(0, |a| a.stock_proc)
}

// ---- the paint imports -----------------------------------------------------------------

macro_rules! orig {
    ($o:ident, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {
        unsafe { core::mem::transmute::<usize, unsafe extern "system" fn($($t),*) -> $r>($o)($($a),*) }
    };
}

unsafe extern "system" fn my_get_dc(h: HWND) -> HDC {
    if let Some(a) = find(h) {
        if a.surface.ok() {
            a.drawn = a.drawn.wrapping_add(1);
            a.surface.reselect();
            return a.surface.dc;
        }
    }
    if crate::config::get().menus_trace && in_adopted(h).is_some() {
        static mut SEEN: Vec<usize> = Vec::new();
        let seen = unsafe { &mut *core::ptr::addr_of_mut!(SEEN) };
        if !seen.contains(&(h as usize)) {
            seen.push(h as usize);
            crate::log!("forms: trace: GetDC of {:p} ({}), a control of an adopted dialog", h, class_of(h));
        }
    }
    orig!(O_GET_DC, fn(HWND) -> HDC, h)
}

unsafe extern "system" fn my_release_dc(h: HWND, dc: HDC) -> i32 {
    if is_ours(dc) {
        return 1;
    }
    orig!(O_RELEASE_DC, fn(HWND, HDC) -> i32, h, dc)
}

unsafe extern "system" fn my_begin_paint(h: HWND, ps: *mut PaintStruct) -> HDC {
    if let Some(a) = find(h) {
        if a.surface.ok() && !ps.is_null() {
            unsafe {
                ValidateRect(h, core::ptr::null());
                core::ptr::write_bytes(ps, 0, 1);
                (*ps).hdc = a.surface.dc;
                (*ps).rc = WinRect { left: 0, top: 0, right: a.surface.w, bottom: a.surface.h };
            }
            a.drawn = a.drawn.wrapping_add(1);
            a.surface.reselect();
            return a.surface.dc;
        }
    }
    orig!(O_BEGIN_PAINT, fn(HWND, *mut PaintStruct) -> HDC, h, ps)
}

unsafe extern "system" fn my_end_paint(h: HWND, ps: *const PaintStruct) -> i32 {
    if !ps.is_null() && is_ours(unsafe { (*ps).hdc }) {
        return 1;
    }
    orig!(O_END_PAINT, fn(HWND, *const PaintStruct) -> i32, h, ps)
}

unsafe extern "system" fn my_invalidate(h: HWND, r: *const WinRect, erase: i32) -> i32 {
    if let Some(a) = find(h) {
        a.paint = true;
        a.erase |= erase != 0;
    } else if let Some(d) = in_adopted(h) {
        // A control drew itself anew: compose again.
        if let Some(a) = find(d) {
            a.drawn = a.drawn.wrapping_add(1);
        }
    }
    orig!(O_INVALIDATE, fn(HWND, *const WinRect, i32) -> i32, h, r, erase)
}

unsafe extern "system" fn my_redraw(h: HWND, r: *const WinRect, rgn: HANDLE, flags: u32) -> i32 {
    // RDW_INVALIDATE 1, RDW_ERASE 4.
    if let Some(a) = find(h) {
        if flags & 1 != 0 {
            a.paint = true;
            a.erase |= flags & 4 != 0;
        }
    }
    orig!(O_REDRAW, fn(HWND, *const WinRect, HANDLE, u32) -> i32, h, r, rgn, flags)
}

// ---- the real pointer ------------------------------------------------------------------

/// The real pointer never lies over an off-screen dialog; a mouse message for one with
/// its point outside the window's client area came through a capture the dialog took
/// (a button held), and would tell it the pointer left: it becomes `WM_NULL`. What the
/// UI posts lies inside.
unsafe extern "system" fn get_message(code: i32, wp: usize, lp: isize) -> isize {
    if code >= 0 && lp != 0 {
        let m = unsafe { &mut *(lp as *mut Msg) };
        if (WM_MOUSEMOVE..=0x20d).contains(&m.message) && m.message != WM_MOUSEWHEEL && in_adopted(m.hwnd).is_some() {
            let x = (m.lparam & 0xffff) as i16 as i32;
            let y = ((m.lparam >> 16) & 0xffff) as i16 as i32;
            let (w, h) = client_of(m.hwnd);
            if x < 0 || y < 0 || x >= w || y >= h {
                m.message = 0;
                unsafe { DROPPED = DROPPED.wrapping_add(1) };
            }
        }
    }
    unsafe { CallNextHookEx(MSG_HOOK, code, wp, lp) }
}
