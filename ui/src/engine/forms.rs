//! Fleet Operations: Roots' front-end forms (PLAN-FLEETOPS.md, FO-6 step 7, option C).
//!
//! On Roots the main menu, Options and six more screens are not the executable's dialogs
//! but VCL forms of `FleetOpsHook.dll` (a Delphi library): owned pop-ups over the engine's
//! device window, each created when its screen opens, shown modal and freed when it
//! closes. Their buttons are graphic controls with no window of their own. This file
//! finds such a form by its window, reads its object through the property the VCL sets
//! on every window it makes, and reads the form's components from the live objects; it
//! writes nothing of FleetOpsHook's and patches none of its code (decisions 141-146).
//!
//! **Step 0, the probe** (`[Menus] Forms=probe`, the test bench only): a thread CBT hook
//! sees each form activate; the form gets an empty window region (Roots' drawing hidden);
//! a thread timer of ours, dispatched by the form's own modal loop, presents a plain frame
//! of ours on the engine's device with the controls read; on a fixed walk it posts one
//! press per screen (main menu -> Options -> Graphics -> Back -> Back -> Instant Action)
//! and changes one combo box through its window. Everything it learns goes to UI.log as
//! `forms:` lines.
use crate::rt::win::*;
use alloc::string::String;
use alloc::vec::Vec;

// ---- the VCL's objects (Delphi 2007, as FleetOpsHook links it) --------------------
//
// Offsets into the VCL's own classes, read from the library's run-time type information
// and its accessors (the private notes, `fo6-optionC.md`). They are the VCL's, not a
// Roots structure; step 2 moves them into the profile, checked per build.

/// A class's name (a pointer to a length-prefixed string), relative to its class pointer.
const VMT_CLASS_NAME: isize = -0x2c;
/// A class's parent (a pointer to the parent's class pointer).
const VMT_PARENT: isize = -0x24;
/// A component's name (a counted string), the components it owns (a list).
const COMP_NAME: usize = 0x08;
const COMP_COMPONENTS: usize = 0x10;
/// A list's items and count.
const LIST_ITEMS: usize = 0x04;
const LIST_COUNT: usize = 0x08;
/// A control's rectangle (left, top, width, height), relative to its parent.
const CTL_LEFT: usize = 0x40;
const CTL_VISIBLE: usize = 0x57;
const CTL_ENABLED: usize = 0x58;
/// A control's text (a null-terminated string, null when empty).
const CTL_TEXT: usize = 0x64;
/// A windowed control's window.
const WIN_HANDLE: usize = 0x1b4;

const WH_CBT: i32 = 5;
const HCBT_CREATEWND: i32 = 3;
const HCBT_DESTROYWND: i32 = 4;
const HCBT_ACTIVATE: i32 = 5;
const WM_LBUTTONDOWN: u32 = 0x201;
const CB_GETCOUNT: u32 = 0x146;
const CB_GETCURSEL: u32 = 0x147;
const CB_SETCURSEL: u32 = 0x14e;
const CBN_SELCHANGE: usize = 1;
const PRF_ERASEBKGND: usize = 8;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowsHookExA(id: i32, f: unsafe extern "system" fn(i32, usize, isize) -> isize, module: HANDLE, thread: u32) -> HANDLE;
    fn CallNextHookEx(h: HANDLE, code: i32, wp: usize, lp: isize) -> isize;
    fn SetTimer(w: HWND, id: usize, ms: u32, f: unsafe extern "system" fn(HWND, u32, usize, u32)) -> usize;
    fn KillTimer(w: HWND, id: usize) -> i32;
    fn SetWindowRgn(w: HWND, rgn: HANDLE, redraw: i32) -> i32;
    fn EnumPropsExA(w: HWND, f: unsafe extern "system" fn(HWND, *const u8, HANDLE, usize) -> i32, lp: usize) -> i32;
    fn WindowFromPoint(p: Point) -> HWND;
    fn GetCursorInfo(ci: *mut CursorInfo) -> i32;
    fn SetLayeredWindowAttributes(w: HWND, key: u32, alpha: u8, flags: u32) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateRectRgn(l: i32, t: i32, r: i32, b: i32) -> HANDLE;
}

#[repr(C)]
#[derive(Default)]
struct CursorInfo {
    size: u32,
    flags: u32,
    cursor: usize,
    pos: Point,
}

/// A control of a form, as read this tick.
#[derive(Clone, Default)]
pub struct Control {
    pub name: String,
    pub class: String,
    /// In the form's client coordinates.
    pub rect: [i32; 4],
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
    /// The control's own window (0 for a graphic control).
    pub hwnd: usize,
}

/// A form seen by the hook.
struct Form {
    hwnd: HWND,
    class: String,
    /// Our walk's state on this form (the probe's script).
    opened: f64,
    obj: usize,
    emptied: bool,
    printed: bool,
    pressed: u32,
    combo: u32,
    visit: u32,
}

static mut CBT: HANDLE = core::ptr::null_mut();
static mut TIMER: usize = 0;
static mut FORMS: Vec<Form> = Vec::new();
static mut VISITS: Vec<(String, u32)> = Vec::new();
static mut FO: (usize, usize) = (0, 0);
static mut PROBE: bool = false;
static mut TICKS: u32 = 0;
static mut TICKS_IN_DIALOG: u32 = 0;
static mut FRAMES: u32 = 0;
static mut LAST_DIAG: f64 = 0.0;
static mut LOG_CREATES: u32 = 0;
/// The top form's controls and size, for the frame (`menus::forms`).
static mut SNAPSHOT: Vec<Control> = Vec::new();
static mut SIZE: (i32, i32) = (0, 0);
static mut PENDING_PRESS: Option<(usize, i32, i32)> = None;

fn forms() -> &'static mut Vec<Form> {
    unsafe { &mut *core::ptr::addr_of_mut!(FORMS) }
}

/// Whether the forms part is running (the probe, for now).
pub fn active() -> bool {
    unsafe { PROBE && TIMER != 0 && forms().iter().any(|f| f.obj != 0) }
}

/// The top form's design size (its client area) and controls, as read at the last tick.
pub fn snapshot() -> ((i32, i32), Vec<Control>) {
    unsafe { (SIZE, (*core::ptr::addr_of!(SNAPSHOT)).clone()) }
}

/// A press of ours on a control of the top form (its centre, posted to the form's window),
/// run at the next tick, outside the frame.
pub fn press(c: &Control) {
    let f = forms().iter().rev().find(|f| f.obj != 0);
    if let Some(f) = f {
        unsafe { PENDING_PRESS = Some((f.hwnd as usize, c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2)) };
        crate::log!("forms: press of ours on {} ({}) of {}", c.name, c.text, f.class);
    }
}

/// `[Menus] Forms=probe`: installs the probe's hook on the game's thread (DllMain's).
pub fn install_probe() -> bool {
    let m = unsafe { GetModuleHandleA(c"FleetOpsHook.dll".as_ptr() as *const u8) } as usize;
    if m == 0 {
        crate::log!("forms: no FleetOpsHook.dll in the process: the probe stands down");
        return false;
    }
    let size = unsafe {
        let nt = m + super::rd::<u32>(m + 0x3c) as usize;
        super::rd::<u32>(nt + 0x50) as usize
    };
    unsafe { FO = (m, m + size) };
    let tid = unsafe { GetCurrentThreadId() };
    let h = unsafe { SetWindowsHookExA(WH_CBT, cbt, core::ptr::null_mut(), tid) };
    crate::log!("forms: probe on: FleetOpsHook at {:#x}..{:#x}, CBT hook {:p} on thread {}", m, m + size, h, tid);
    unsafe {
        CBT = h;
        PROBE = !h.is_null();
    }
    !h.is_null()
}

fn class_of(h: HWND) -> String {
    let mut b = [0u8; 64];
    let n = unsafe { GetClassNameA(h, b.as_mut_ptr(), 64) }.max(0) as usize;
    String::from(core::str::from_utf8(&b[..n]).unwrap_or("?"))
}

unsafe extern "system" fn cbt(code: i32, wp: usize, lp: isize) -> isize {
    let h = wp as HWND;
    match code {
        HCBT_CREATEWND => unsafe {
            if LOG_CREATES < 80 {
                LOG_CREATES += 1;
                let owner = GetWindow(h, GW_OWNER);
                crate::log!("forms: window {:p} created, class {}, owner {:p}", h, class_of(h), owner);
            }
        },
        HCBT_ACTIVATE => activated(h),
        HCBT_DESTROYWND => destroyed(h),
        _ => {}
    }
    unsafe { CallNextHookEx(CBT, code, wp, lp) }
}

fn is_form_class(c: &str) -> bool {
    c.starts_with('T') && c.ends_with("Form") && c != "TMainWindowForm"
}

fn activated(h: HWND) {
    let class = class_of(h);
    let owner = unsafe { GetWindow(h, GW_OWNER) };
    let dialog = class == "#32770";
    crate::log!("forms: {:p} activated, class {}, owner {:p} ({}), {} form(s) known", h, class, owner, class_of(owner), forms().len());
    if !is_form_class(&class) && !dialog {
        return;
    }
    let visits = unsafe { &mut *core::ptr::addr_of_mut!(VISITS) };
    let visit = match visits.iter_mut().find(|v| v.0 == class) {
        Some(v) => {
            v.1 += 1;
            v.1
        }
        None => {
            visits.push((class.clone(), 1));
            1
        }
    };
    // A form active again (the one under a screen that closed) starts its walk anew.
    if let Some(f) = forms().iter_mut().find(|f| f.hwnd == h) {
        f.visit = visit;
        f.opened = crate::rt::now();
        f.pressed = 0;
        f.printed = false;
        f.emptied = false;
        return;
    }
    forms().push(Form { hwnd: h, class, opened: crate::rt::now(), obj: 0, emptied: false, printed: false, pressed: 0, combo: 0, visit });
    unsafe {
        if TIMER == 0 {
            TIMER = SetTimer(core::ptr::null_mut(), 0, 16, tick);
            crate::log!("forms: timer {} started", TIMER);
        }
    }
}

fn destroyed(h: HWND) {
    let v = forms();
    if let Some(i) = v.iter().position(|f| f.hwnd == h) {
        let f = v.remove(i);
        crate::log!(
            "forms: {} {:p} destroyed after {:.1} s; {} ticks, {} frames so far, {} ticks inside a dialog",
            f.class, h, crate::rt::now() - f.opened, unsafe { TICKS }, unsafe { FRAMES }, unsafe { TICKS_IN_DIALOG }
        );
        unsafe { (*core::ptr::addr_of_mut!(SNAPSHOT)).clear() };
    }
    if v.is_empty() {
        unsafe {
            if TIMER != 0 {
                KillTimer(core::ptr::null_mut(), TIMER);
                crate::log!("forms: timer {} stopped", TIMER);
                TIMER = 0;
            }
        }
    }
}

// ---- reading the objects ------------------------------------------------------------

fn ptr_ok(p: usize, n: usize) -> bool {
    p >= 0x10000 && p < 0x8000_0000 && p % 4 == 0 && super::readable(p, n)
}

fn in_fo(p: usize) -> bool {
    let (a, b) = unsafe { FO };
    p >= a && p < b
}

/// A counted string (length byte first).
fn short_string(p: usize) -> Option<String> {
    if p == 0 || !super::readable(p, 1) {
        return None;
    }
    let n = unsafe { super::rd::<u8>(p) } as usize;
    if !super::readable(p + 1, n) {
        return None;
    }
    let b = unsafe { core::slice::from_raw_parts((p + 1) as *const u8, n) };
    core::str::from_utf8(b).ok().map(String::from)
}

/// A class's name, when its class pointer lies in FleetOpsHook's image.
pub fn class_name(vmt: usize) -> Option<String> {
    if !in_fo(vmt) {
        return None;
    }
    let p = unsafe { super::rd::<usize>((vmt as isize + VMT_CLASS_NAME) as usize) };
    if !in_fo(p) {
        return None;
    }
    short_string(p)
}

/// Whether the object's class is `name` or derives from it.
fn derives(vmt: usize, name: &str) -> bool {
    let mut v = vmt;
    for _ in 0..40 {
        match class_name(v) {
            Some(n) if n == name => return true,
            Some(_) => {}
            None => return false,
        }
        let pp = unsafe { super::rd::<usize>((v as isize + VMT_PARENT) as usize) };
        if !in_fo(pp) {
            return false;
        }
        v = unsafe { super::rd::<usize>(pp) };
    }
    false
}

/// A Delphi long string (characters, length in the dword before).
fn ansi_string(p: usize) -> String {
    if p < 4 || !super::readable(p - 4, 4) {
        return String::new();
    }
    let n = (unsafe { super::rd::<u32>(p - 4) } as usize).min(256);
    if !super::readable(p, n) {
        return String::new();
    }
    let b = unsafe { core::slice::from_raw_parts(p as *const u8, n) };
    b.iter().map(|&c| if c.is_ascii() { c as char } else { '?' }).collect()
}

fn c_string(p: usize) -> String {
    let mut s = String::new();
    if p == 0 {
        return s;
    }
    for i in 0..256 {
        if !super::readable(p + i, 1) {
            break;
        }
        let c = unsafe { super::rd::<u8>(p + i) };
        if c == 0 {
            break;
        }
        s.push(if c.is_ascii() { c as char } else { '?' });
    }
    s
}

static mut FOUND_PROP: usize = 0;
static mut PROP_NAME: String = String::new();

unsafe extern "system" fn prop(_h: HWND, name: *const u8, data: HANDLE, _lp: usize) -> i32 {
    // An atom (a small integer) is not a name.
    if (name as usize) < 0x10000 {
        return 1;
    }
    let s = c_string(name as usize);
    if s.starts_with("ControlOfs") {
        unsafe {
            FOUND_PROP = data as usize;
            PROP_NAME = s;
        }
        return 0;
    }
    1
}

/// The form's object, through the VCL's window property ("ControlOfs" + the library's
/// module handle and its thread's id), checked: its class pointer in FleetOpsHook's
/// image, its class name the window's class.
fn object_of(h: HWND, class: &str) -> usize {
    unsafe {
        FOUND_PROP = 0;
        EnumPropsExA(h, prop, 0);
    }
    let obj = unsafe { FOUND_PROP };
    let name = unsafe { (*core::ptr::addr_of!(PROP_NAME)).clone() };
    if obj == 0 || !ptr_ok(obj, 4) {
        crate::log!("forms: {:p} ({}) has no object property", h, class);
        return 0;
    }
    let vmt = unsafe { super::rd::<usize>(obj) };
    let cn = class_name(vmt);
    let ok = cn.as_deref() == Some(class);
    crate::log!(
        "forms: {:p} property {} -> object {:#x}, class pointer {:#x} {} FleetOpsHook, class name {:?}: {}",
        h, name, obj, vmt, if in_fo(vmt) { "in" } else { "outside" }, cn, if ok { "accepted" } else { "refused" }
    );
    if ok { obj } else { 0 }
}

/// The form's components that are controls, read now.
fn controls(obj: usize) -> Vec<Control> {
    let mut out = Vec::new();
    let list = unsafe { super::rd::<usize>(obj + COMP_COMPONENTS) };
    if !ptr_ok(list, 12) {
        return out;
    }
    let (items, n) = unsafe { (super::rd::<usize>(list + LIST_ITEMS), super::rd::<i32>(list + LIST_COUNT)) };
    if n <= 0 || n > 512 || !ptr_ok(items, n as usize * 4) {
        return out;
    }
    for i in 0..n as usize {
        let c = unsafe { super::rd::<usize>(items + i * 4) };
        if !ptr_ok(c, WIN_HANDLE + 4) {
            continue;
        }
        let vmt = unsafe { super::rd::<usize>(c) };
        let Some(class) = class_name(vmt) else { continue };
        if !derives(vmt, "TControl") {
            continue;
        }
        let r: [i32; 4] = unsafe { core::array::from_fn(|k| super::rd::<i32>(c + CTL_LEFT + k * 4)) };
        let windowed = derives(vmt, "TWinControl");
        out.push(Control {
            name: ansi_string(unsafe { super::rd::<usize>(c + COMP_NAME) }),
            class,
            rect: r,
            text: c_string(unsafe { super::rd::<usize>(c + CTL_TEXT) }),
            visible: unsafe { super::rd::<u8>(c + CTL_VISIBLE) } != 0,
            enabled: unsafe { super::rd::<u8>(c + CTL_ENABLED) } != 0,
            hwnd: if windowed { unsafe { super::rd::<usize>(c + WIN_HANDLE) } } else { 0 },
        });
    }
    out
}

fn log_controls(f: &Form, v: &[Control]) {
    crate::log!("forms: {} (visit {}) has {} controls:", f.class, f.visit, v.len());
    for c in v {
        crate::log!(
            "forms:   {} {} {},{} {}x{} {}{} text {:?} window {:#x}",
            c.class, c.name, c.rect[0], c.rect[1], c.rect[2], c.rect[3],
            if c.visible { "visible" } else { "hidden" }, if c.enabled { "" } else { " disabled" }, c.text, c.hwnd
        );
    }
}

// ---- the pump -------------------------------------------------------------------------

unsafe extern "system" fn tick(_h: HWND, _m: u32, _id: usize, _t: u32) {
    unsafe { TICKS += 1 };
    let now = crate::rt::now();
    let v = forms();
    v.retain(|f| unsafe { IsWindow(f.hwnd) } != 0);
    let Some(f) = v.last_mut() else { return };
    if f.class == "#32770" {
        unsafe { TICKS_IN_DIALOG += 1 };
        if now - unsafe { LAST_DIAG } > 2.0 {
            unsafe { LAST_DIAG = now };
            crate::log!("forms: tick {} inside the dialog {:p} ({} there)", unsafe { TICKS }, f.hwnd, unsafe { TICKS_IN_DIALOG });
        }
        return;
    }
    if f.obj == 0 {
        f.obj = object_of(f.hwnd, &f.class);
        if f.obj == 0 {
            f.class.push_str(" (refused)");
            return;
        }
    }
    let ctl = controls(f.obj);
    let mut c = WinRect::default();
    unsafe { GetClientRect(f.hwnd, &mut c) };
    let age = now - f.opened;
    if !f.emptied {
        f.emptied = true;
        let how = hide(f.hwnd, &f.class, f.visit, c.right, c.bottom);
        log_controls(f, &ctl);
        crate::log!("forms: {} client {}x{}: hidden by {}, active {:p}, focus {:p}", f.class, c.right, c.bottom, how,
            unsafe { GetActiveWindow() }, unsafe { GetFocus() });
    }
    keep_off(f.hwnd);
    // Q8: with the form hidden nothing sets the pointer's shape: the pump does.
    unsafe { SetCursor(LoadCursorA(core::ptr::null_mut(), 32512)) };
    if !f.printed && age > 3.0 {
        f.printed = true;
        print(f.hwnd, &f.class, c.right, c.bottom);
        log_controls(f, &ctl);
    }
    unsafe {
        SIZE = (c.right, c.bottom);
        *core::ptr::addr_of_mut!(SNAPSHOT) = ctl.clone();
    }
    if now - unsafe { LAST_DIAG } > 2.0 {
        unsafe { LAST_DIAG = now };
        diag(f.hwnd, &f.class);
    }
    if let Some((h, x, y)) = unsafe { PENDING_PRESS.take() } {
        post_press(h as HWND, x, y);
    }
    script(f, &ctl, age);
    frame();
}

const GWL_EXSTYLE: i32 = -20;
const WS_EX_LAYERED: isize = 0x80000;
const LWA_ALPHA: u32 = 2;
/// Off screen: far to the left and above of every output.
const OFF: i32 = -20000;
static mut OFFSCREEN: Vec<usize> = Vec::new();

/// Q1 and its fallbacks, one per screen on the walk so one bench pass answers each: the
/// main menu's first visit an empty region and a nearly transparent layered window (the first
/// pass tried the region alone on every screen), every other screen moved off screen (and its region
/// emptied, so nothing of it can show on the way).
fn hide(h: HWND, class: &str, visit: u32, w: i32, ht: i32) -> &'static str {
    let rgn = unsafe { SetWindowRgn(h, CreateRectRgn(0, 0, 0, 0), 1) } != 0;
    match (class, visit) {
        ("TMainForm", 1) => unsafe {
            let ex = GetWindowLongA(h, GWL_EXSTYLE);
            SetWindowLongA(h, GWL_EXSTYLE, ex | WS_EX_LAYERED);
            let ok = SetLayeredWindowAttributes(h, 0, 1, LWA_ALPHA) != 0;
            if ok && rgn { "an empty region and a layered window at alpha 1" } else { "an empty region and a layered window (one of them refused)" }
        },
        _ => {
            let _ = (w, ht);
            unsafe {
                SetWindowPos(h, core::ptr::null_mut(), OFF, OFF, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                (*core::ptr::addr_of_mut!(OFFSCREEN)).push(h as usize);
            }
            "a move off screen (and an empty region)"
        }
    }
}

/// A form moved off screen stays there: FleetOpsHook re-places its screens when the device
/// window moves.
fn keep_off(h: HWND) {
    if !unsafe { (*core::ptr::addr_of!(OFFSCREEN)).contains(&(h as usize)) } {
        return;
    }
    let mut r = WinRect::default();
    unsafe { GetWindowRect(h, &mut r) };
    if r.left != OFF || r.top != OFF {
        crate::log!("forms: {:p} was moved back to {},{}: off screen again", h, r.left, r.top);
        unsafe { SetWindowPos(h, core::ptr::null_mut(), OFF, OFF, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }
}

fn diag(form: HWND, class: &str) {
    let mut p = Point::default();
    let mut ci = CursorInfo { size: core::mem::size_of::<CursorInfo>() as u32, ..Default::default() };
    unsafe {
        GetCursorPos(&mut p);
        GetCursorInfo(&mut ci);
    }
    let under = unsafe { WindowFromPoint(p) };
    let owner = unsafe { GetWindow(form, GW_OWNER) };
    let mut q = p;
    unsafe { ScreenToClient(owner, &mut q) };
    let inp = crate::input::state();
    crate::log!(
        "forms: {} up; tick {} frame {}; pointer {},{} (owner client {},{}; UI {:?}) over {:p} {}; owner enabled {}; cursor {:#x} flags {}; active {:p} focus {:p}",
        class, unsafe { TICKS }, unsafe { FRAMES }, p.x, p.y, q.x, q.y, inp.mouse, under, class_of(under),
        unsafe { IsWindowEnabled(owner) }, ci.cursor, ci.flags, unsafe { GetActiveWindow() }, unsafe { GetFocus() }
    );
}

fn post_press(h: HWND, x: i32, y: i32) {
    let lp = ((y as u32) << 16 | (x as u32 & 0xffff)) as isize;
    unsafe {
        PostMessageA(h, WM_LBUTTONDOWN, MK_LBUTTON, lp);
        PostMessageA(h, WM_LBUTTONUP, 0, lp);
    }
    crate::log!("forms: press posted to {:p} at {},{}", h, x, y);
}

fn find<'a>(v: &'a [Control], name: &str) -> Option<&'a Control> {
    v.iter().find(|c| c.name == name)
}

/// The probe's walk: one press per screen, a combo changed on the graphics screen.
fn script(f: &mut Form, v: &[Control], age: f64) {
    let target = match (f.class.as_str(), f.visit) {
        // The main menu's field names are swapped against their actions on Roots: the
        // one named for the map editor runs Options (the private notes).
        ("TMainForm", 1) => Some("buttonMapEdit"),
        ("TMainForm", 2) => Some("buttonInstantAction"),
        ("TOptionsMenuForm", 1) => Some("btnGraphicSettings"),
        ("TOptionsMenuForm", 2) => Some("buttonBack"),
        ("TGraphicOptionsForm", _) => Some("buttonBack"),
        _ => None,
    };
    if f.class == "TGraphicOptionsForm" && f.combo == 0 && age > 6.0 {
        f.combo = 1;
        combo_probe(f.hwnd, v);
    }
    let Some(t) = target else { return };
    if f.pressed == 0 && age > if f.class == "TMainForm" && f.visit == 1 { 25.0 } else { 12.0 } {
        f.pressed = 1;
        match find(v, t) {
            Some(c) if c.visible => {
                crate::log!("forms: the walk presses {} ({:?}) of {}", c.name, c.text, f.class);
                post_press(f.hwnd, c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2);
            }
            _ => crate::log!("forms: the walk finds no visible {} on {}", t, f.class),
        }
    }
}

/// Q9: a combo box changed through its window. The device combo's change handler
/// refills the resolution combo and selects the shell's mode in it, so: move the
/// resolution combo's selection quietly, then select the device combo's own entry with
/// the parent's notification, and read the resolution combo again.
fn combo_probe(form: HWND, v: &[Control]) {
    let (Some(dev), Some(res)) = (find(v, "comboDevicePrimary"), find(v, "comboDevicePrimaryResolutions")) else {
        crate::log!("forms: combo probe: the two combos are not there");
        return;
    };
    let (dh, rh) = (dev.hwnd as HWND, res.hwnd as HWND);
    unsafe {
        let r0 = SendMessageA(rh, CB_GETCURSEL, 0, 0);
        let rn = SendMessageA(rh, CB_GETCOUNT, 0, 0);
        let moved = if r0 == 0 { 1 } else { 0 };
        SendMessageA(rh, CB_SETCURSEL, moved, 0);
        let r1 = SendMessageA(rh, CB_GETCURSEL, 0, 0);
        // Probe only: a counting subclass on the resolution combo for the length of the
        // notification (the device combo's handler refills that list).
        COMBO_SEEN = 0;
        COMBO_PREV = SetWindowLongA(rh, GWL_WNDPROC, combo_count as unsafe extern "system" fn(HWND, u32, usize, isize) -> isize as usize as isize);
        let d0 = SendMessageA(dh, CB_GETCURSEL, 0, 0);
        SendMessageA(dh, CB_SETCURSEL, d0.max(0) as usize, 0);
        let id = GetDlgCtrlID(dh) as usize;
        let parent = GetParent(dh);
        SendMessageA(parent, WM_COMMAND, (CBN_SELCHANGE << 16) | (id & 0xffff), dh as isize);
        let r2 = SendMessageA(rh, CB_GETCURSEL, 0, 0);
        let rn2 = SendMessageA(rh, CB_GETCOUNT, 0, 0);
        let seen = COMBO_SEEN;
        SetWindowLongA(rh, GWL_WNDPROC, COMBO_PREV);
        crate::log!(
            "forms: combo probe on {:p}: resolutions {} of {}, moved quietly to {}, device {} re-selected with CBN_SELCHANGE to {:p}: resolutions {} of {}, {} list messages to it meanwhile -> the change handler {}",
            form, r0, rn, r1, d0, parent, r2, rn2, seen,
            if r2 != r1 || seen > 0 { "ran" } else { "did not run" }
        );
    }
}

static mut COMBO_PREV: isize = 0;
static mut COMBO_SEEN: u32 = 0;

unsafe extern "system" fn combo_count(h: HWND, m: u32, wp: usize, lp: isize) -> isize {
    // CB_RESETCONTENT, CB_ADDSTRING, CB_INSERTSTRING, CB_SETCURSEL
    if matches!(m, 0x14b | 0x143 | 0x14a | 0x14e) {
        unsafe { COMBO_SEEN += 1 };
    }
    unsafe { CallWindowProcA(COMBO_PREV, h, m, wp, lp) }
}

/// Q6: the form printed into a memory DC, whatever its region; written as a BMP beside
/// the logs on the bench (a picture of Roots' art: never copied out of the clone).
fn print(h: HWND, class: &str, w: i32, ht: i32) {
    if w <= 0 || ht <= 0 {
        return;
    }
    let bi = BitmapInfoHeader { size: 40, width: w, height: -ht, planes: 1, bit_count: 32, ..Default::default() };
    unsafe {
        let dc = CreateCompatibleDC(core::ptr::null_mut());
        let mut bits = core::ptr::null_mut();
        let bmp = CreateDIBSection(dc, &bi, 0, &mut bits, core::ptr::null_mut(), 0);
        if bmp.is_null() {
            DeleteDC(dc);
            return;
        }
        let old = SelectObject(dc, bmp);
        PatBlt(dc, 0, 0, w, ht, BLACKNESS);
        let r = SendMessageA(h, WM_PRINT, dc as usize, (PRF_CLIENT | PRF_CHILDREN | PRF_ERASEBKGND | PRF_NONCLIENT) as isize);
        GdiFlush();
        let n = (w * ht) as usize;
        let px = core::slice::from_raw_parts(bits as *const u32, n);
        let lit = px.iter().filter(|&&p| p & 0xffffff != 0).count();
        crate::log!("forms: {} printed ({}): {} of {} pixels not black ({:.1}%)", class, r, lit, n, lit as f64 * 100.0 / n as f64);
        let size = (n * 4) as u32;
        let mut v: Vec<u8> = Vec::with_capacity(54 + n * 4);
        v.extend_from_slice(b"BM");
        v.extend_from_slice(&(54 + size).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&54u32.to_le_bytes());
        v.extend_from_slice(&40u32.to_le_bytes());
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&(-ht).to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&32u16.to_le_bytes());
        v.extend_from_slice(&[0u8; 24]);
        v.extend_from_slice(core::slice::from_raw_parts(bits as *const u8, n * 4));
        let dir = crate::rt::log_dir().unwrap_or_else(|| crate::rt::game_path(""));
        crate::rt::write_file(&crate::sfmt!("{}\\UI-form-{}.bmp", dir, class), &v);
        SelectObject(dc, old);
        DeleteObject(bmp);
        DeleteDC(dc);
    }
}

/// Q2: one frame of ours on the engine's device, presented from the timer.
fn frame() {
    use super::dev8;
    let dev = super::d3d8_device();
    if dev.is_null() || dev8::test_cooperative_level(dev) < 0 {
        return;
    }
    let Some(bb) = dev8::back_buffer_desc(dev) else { return };
    if dev8::begin_scene(dev) < 0 {
        return;
    }
    dev8::set_viewport(dev, bb.width, bb.height);
    dev8::clear(dev, dev8::CLEAR_TARGET, 0xff000000);
    crate::surface::frame(crate::surface::Context::Menus);
    dev8::end_scene(dev);
    let r = dev8::present(dev);
    unsafe {
        FRAMES += 1;
        if FRAMES == 1 || FRAMES % 600 == 0 {
            crate::log!("forms: frame {} presented ({:#x}), {}x{}", FRAMES, r, bb.width, bb.height);
        }
    }
}
