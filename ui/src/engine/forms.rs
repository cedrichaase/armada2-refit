//! Fleet Operations: Roots' front-end forms (PLAN-FLEETOPS.md, FO-6 step 7, option C;
//! ui/MENUS.md, "Roots' forms").
//!
//! On Roots the main menu, Options (the in-mission menu too) and six more screens are not
//! the executable's dialogs but VCL forms of `FleetOpsHook.dll` (a Delphi library): owned
//! pop-ups over the engine's device window, each created when its screen opens, shown
//! modal and freed when it closes. Their buttons are graphic controls with no window of
//! their own. The UI:
//!
//! - **finds a form** by its window: a thread CBT hook on the game's thread sees each window
//!   activate; the form's object comes from the property the VCL sets on every window it
//!   makes (named for the library's module and its thread), and counts only when its class
//!   lies in FleetOpsHook's image and is named as the window's class (decision 142);
//! - **reads its controls** from the live objects, by the VCL's own fields (the profile's
//!   `forms_*` fields, `profiles/gog-1.1/ui.forms.toml`), every pointer checked, only on
//!   the game thread, only while the form is modal and not being destroyed, and keeps
//!   nothing but plain copies (decision 143);
//! - **hides the form** by moving it off screen (an empty region alone leaves a black
//!   rectangle on Wine under XWayland: decision 170);
//! - **draws** from a thread timer of its own, which the form's modal loop dispatches like
//!   any message: a whole frame of ours on the engine's device (decision 144);
//! - **acts** only by posting what a player's mouse would (a left press and release at a
//!   control's centre, to the form's window) or by the messages a windowed control takes
//!   (a combo's selection and its notification); it calls no handler and writes no
//!   setting, so Roots' own code runs and writes what it writes (decision 145).
//!
//! Nothing of FleetOpsHook's memory is written and none of its code patched. The forms
//! part runs only on a FleetOpsHook of the measured family (`FAMILY`, decision 148), and
//! only with `[Menus] Forms=1` (Roots' default is 0, decision 149). `Forms=probe` is the
//! step 0 probe: the same, plus a fixed walk of presses and a log of what it finds.
use crate::rt::win::*;
use crate::syms_forms::{field, hook, value, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

/// The FleetOpsHook builds the forms part accepts: the CRC-32 of the file's `.text` (as
/// `[[module]] text_crc` names a companion) and the release, from `tools/engine/fo-forms.py`
/// (decision 147). One build edited in place across these three releases: the forms, their
/// classes and the VCL's layout are the same in each.
const FAMILY: &[(u32, &str)] = &[
    (0x0785_8db1, "Fleet Operations: Roots 4.0.93"),
    (0x28c9_0d19, "Fleet Operations: Roots 5.0Z11"),
    (0xa95a_b2d7, "Tactical Maneuvers 5.2.3"),
];

/// The forms the UI draws. Any other (the map editor's, Tunngle's wait) stays Roots'
/// (decision 152).
const TAKEN: &[&str] = &[
    "TMainForm",
    "TOptionsMenuForm",
    "TYesNoForm",
    "TManualIPForm",
    "TMultiplayerConnectionForm",
    "TMapSelectForm",
    "TGraphicOptionsForm",
    "TAdvancedOptionsForm",
    "TSoundSettingsForm",
    "TModSettingsForm",
    "TAdvSettingsForm",
];

const WH_CBT: i32 = 5;
const HCBT_DESTROYWND: i32 = 4;
const HCBT_ACTIVATE: i32 = 5;
const WM_LBUTTONDOWN: u32 = 0x201;
const CB_GETCOUNT: u32 = 0x146;
const CB_GETCURSEL: u32 = 0x147;
const CB_GETLBTEXT: u32 = 0x148;
const CB_GETLBTEXTLEN: u32 = 0x149;
const CB_SETCURSEL: u32 = 0x14e;
const CBN_SELCHANGE: usize = 1;
const PRF_ERASEBKGND: usize = 8;
const CWP_SKIPINVISIBLE: u32 = 1;
/// Off screen: far to the left of and above every output (decision 170).
const OFF: i32 = -20000;
/// The arrow (`IDC_ARROW`).
const IDC_ARROW: usize = 32512;
/// `FormState`'s and `ComponentState`'s bits the walk checks (the VCL's sets).
const FS_MODAL: u8 = 1 << 3;
const CS_DESTROYING: u8 = 1 << 3;
/// The picture of the top form is printed again this often (seconds).
const PRINT_EVERY: f64 = 0.1;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowsHookExA(id: i32, f: unsafe extern "system" fn(i32, usize, isize) -> isize, module: HANDLE, thread: u32) -> HANDLE;
    fn CallNextHookEx(h: HANDLE, code: i32, wp: usize, lp: isize) -> isize;
    fn SetTimer(w: HWND, id: usize, ms: u32, f: unsafe extern "system" fn(HWND, u32, usize, u32)) -> usize;
    fn KillTimer(w: HWND, id: usize) -> i32;
    fn SetWindowRgn(w: HWND, rgn: HANDLE, redraw: i32) -> i32;
    fn EnumPropsExA(w: HWND, f: unsafe extern "system" fn(HWND, *const u8, HANDLE, usize) -> i32, lp: usize) -> i32;
    fn WindowFromPoint(p: Point) -> HWND;
    fn ChildWindowFromPointEx(w: HWND, p: Point, flags: u32) -> HWND;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateRectRgn(l: i32, t: i32, r: i32, b: i32) -> HANDLE;
}

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

// ---- what the rest of the UI sees ---------------------------------------------------

/// What a control is, for drawing it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Kind {
    /// A graphic button (pressed by a posted click at its centre).
    Button,
    /// Text.
    Label,
    /// A picture (a form's background).
    Image,
    /// A text field with a window of its own.
    Edit,
    /// A combo box with a window of its own.
    Combo,
    /// Anything else: drawn from the form's own picture, clicks forwarded.
    #[default]
    Other,
}

/// A control of a form, as read at the last tick: plain copies, no pointer into the form.
#[derive(Clone, Default)]
pub struct Control {
    pub name: String,
    pub class: String,
    pub kind: Kind,
    /// In the form's client coordinates.
    pub rect: [i32; 4],
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
    /// The control's own window (0 for a graphic control).
    pub hwnd: usize,
    /// A combo's entries and the selected one (-1: none).
    pub items: Vec<String>,
    pub selected: i32,
}

/// A form as the UI draws it: its class, where it sits in the design space (the
/// full-screen forms' client size), its size, and its controls.
#[derive(Clone, Default)]
pub struct Form {
    pub hwnd: usize,
    pub class: String,
    pub origin: (i32, i32),
    pub size: (i32, i32),
    pub controls: Vec<Control>,
    /// Whether this is the modal form now (the others are under it, drawn inert).
    pub top: bool,
    /// Whether this form opened over a mission (the in-mission menu).
    pub in_mission: bool,
}

impl Form {
    pub fn control(&self, name: &str) -> Option<&Control> {
        self.controls.iter().find(|c| c.name == name)
    }
}

/// What the UI asks of the top form, run at the next tick before it is read again.
#[derive(Clone)]
pub enum Action {
    /// A left press and release at a control's centre.
    Press(usize, [i32; 4]),
    /// The mouse at a point of the form (forwarded to the window under it): down, move or
    /// up.
    Mouse(usize, u32, i32, i32),
    /// A combo's entry chosen, with the parent's notification.
    Select(usize, i32),
    /// The keyboard to a windowed control (a text field clicked).
    Focus(usize),
}

pub const MOUSE_DOWN: u32 = WM_LBUTTONDOWN;
pub const MOUSE_MOVE: u32 = WM_MOUSEMOVE;
pub const MOUSE_UP: u32 = WM_LBUTTONUP;

// ---- state ----------------------------------------------------------------------------

struct Tracked {
    hwnd: HWND,
    class: String,
    obj: usize,
    opened: f64,
    origin: (i32, i32),
    size: (i32, i32),
    hidden: bool,
    last: Vec<Control>,
    in_mission: bool,
    // the probe's walk
    visit: u32,
    pressed: bool,
    combo: bool,
}

static mut MODE: u8 = 0;
static mut CBT: HANDLE = core::ptr::null_mut();
static mut TIMER: usize = 0;
static mut TRACKED: Vec<Tracked> = Vec::new();
static mut VISITS: Vec<(String, u32)> = Vec::new();
static mut FO: (usize, usize) = (0, 0);
static mut PROP: String = String::new();
static mut TICKS: u32 = 0;
static mut DIALOG_TICKS: u32 = 0;
static mut FRAMES: u32 = 0;
static mut LAST_DIAG: f64 = 0.0;
static mut ACTIONS: Vec<Action> = Vec::new();
/// The forms as the frame draws them, outermost first.
static mut VIEW: Vec<Form> = Vec::new();
/// The top form's picture: DC, bitmap, the DC's old bitmap, the bits (32-bit top-down),
/// width and height.
static mut PICTURE: (HDC, HGDIOBJ, HGDIOBJ, *mut u8, i32, i32) =
    (core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), 0, 0);
static mut PRINTED_AT: f64 = 0.0;
static mut PRINTS: u32 = 0;
static mut HOSTING: bool = false;

fn tracked() -> &'static mut Vec<Tracked> {
    unsafe { &mut *core::ptr::addr_of_mut!(TRACKED) }
}

/// Whether a form of Roots' is drawn by the UI now: the frame of `menus::forms` is the
/// menus' frame.
pub fn active() -> bool {
    unsafe { MODE != 0 && TIMER != 0 && !(*core::ptr::addr_of!(VIEW)).is_empty() }
}

/// The probe of step 0 (`Forms=probe`).
pub fn probe() -> bool {
    unsafe { MODE == 2 }
}

/// The forms to draw, outermost first (a copy).
pub fn view() -> Vec<Form> {
    unsafe { (*core::ptr::addr_of!(VIEW)).clone() }
}

/// The design space: the full-screen forms' size (the profile's, Roots' shell size).
pub fn design() -> (i32, i32) {
    (value::FORMS_DESIGN_WIDTH(), value::FORMS_DESIGN_HEIGHT())
}

/// The top form's picture as last printed: (bits, width, height, serial), for the mirror.
pub fn picture() -> Option<(*const u8, i32, i32, u32)> {
    let p = unsafe { PICTURE };
    if p.3.is_null() { None } else { Some((p.3 as *const u8, p.4, p.5, unsafe { PRINTS })) }
}

/// Queues an action on the top form, run at the next tick.
pub fn act(a: Action) {
    unsafe { (*core::ptr::addr_of_mut!(ACTIONS)).push(a) };
}

/// A press of ours on a control.
pub fn press(form: &Form, c: &Control) {
    crate::log!("forms: press on {} ({:?}) of {}", c.name, c.text, form.class);
    act(Action::Press(form.hwnd, c.rect));
}

// ---- install --------------------------------------------------------------------------

/// Whether this game has forms of FleetOpsHook's the UI knows: the build has them and the
/// loaded FleetOpsHook is one of the family. Decides whether the Refit screen shows the
/// `Forms` row at all; settled once, as neither changes while the game runs.
pub fn available() -> bool {
    use core::sync::atomic::{AtomicU8, Ordering};
    static A: AtomicU8 = AtomicU8::new(0);
    match A.load(Ordering::Relaxed) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    let ok = eng().ready(hook::FORMS_VCL).is_ok()
        && value::FORMS_PRESENT() != 0
        && a2engine::companion_crc("FleetOpsHook.dll").is_some_and(|c| FAMILY.iter().any(|f| f.0 == c));
    A.store(if ok { 2 } else { 1 }, Ordering::Relaxed);
    ok
}

/// `[Menus] Forms` 1 (ours) or 2 (the probe): the CBT hook on the game's thread (DllMain's),
/// when the build has forms and FleetOpsHook is one of the family.
pub fn install(mode: u8) -> bool {
    if let Err(why) = eng().ready(hook::FORMS_VCL) {
        crate::log!("forms: standing down: {:?}", why);
        return false;
    }
    if value::FORMS_PRESENT() == 0 {
        crate::log!("forms: this build has no forms of FleetOpsHook's: standing down");
        return false;
    }
    let m = unsafe { GetModuleHandleA(c"FleetOpsHook.dll".as_ptr() as *const u8) } as usize;
    if m == 0 {
        crate::log!("forms: FleetOpsHook.dll is not loaded: standing down");
        return false;
    }
    let crc = a2engine::companion_crc("FleetOpsHook.dll");
    let Some(&(_, release)) = FAMILY.iter().find(|f| Some(f.0) == crc) else {
        crate::log!(
            "forms: FleetOpsHook.dll .text crc {:#010x} is not a measured build: Roots draws its own menus",
            crc.unwrap_or(0)
        );
        return false;
    };
    let size = unsafe {
        let nt = m + super::rd::<u32>(m + 0x3c) as usize; // PE header
        super::rd::<u32>(nt + 0x50) as usize // PE header: SizeOfImage
    };
    let tid = unsafe { GetCurrentThreadId() };
    unsafe {
        FO = (m, m + size);
        // The VCL's window property: "ControlOfs", the library's module handle and its
        // thread's id, eight hex digits each (the thread is the game's: step 0).
        PROP = crate::sfmt!("ControlOfs{:08X}{:08X}", m, tid);
    }
    let h = unsafe { SetWindowsHookExA(WH_CBT, cbt, core::ptr::null_mut(), tid) };
    crate::log!(
        "forms: {} (FleetOpsHook of {}), CBT hook on thread {}: {}",
        if mode == 2 { "the probe" } else { "Roots' forms are the UI's" }, release, tid, if h.is_null() { "refused" } else { "on" }
    );
    unsafe {
        CBT = h;
        MODE = if h.is_null() { 0 } else { mode };
    }
    !h.is_null()
}

fn class_of(h: HWND) -> String {
    let mut b = [0u8; 64];
    let n = unsafe { GetClassNameA(h, b.as_mut_ptr(), 64) }.max(0) as usize;
    String::from(core::str::from_utf8(&b[..n]).unwrap_or("?"))
}

unsafe extern "system" fn cbt(code: i32, wp: usize, lp: isize) -> isize {
    match code {
        HCBT_ACTIVATE => activated(wp as HWND),
        HCBT_DESTROYWND => destroyed(wp as HWND),
        _ => {}
    }
    unsafe { CallNextHookEx(CBT, code, wp, lp) }
}

fn activated(h: HWND) {
    let class = class_of(h);
    let dialog = class == "#32770";
    if !TAKEN.contains(&class.as_str()) && !dialog {
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
    if let Some(t) = tracked().iter_mut().find(|t| t.hwnd == h) {
        t.visit = visit;
        t.opened = crate::rt::now();
        t.pressed = false;
        return;
    }
    // Where it sits, before it is moved: relative to the device window's client area, the
    // design space's origin (Roots places its full-screen forms there).
    let owner = unsafe { GetWindow(h, GW_OWNER) };
    let mut r = WinRect::default();
    let mut o = Point::default();
    unsafe {
        GetWindowRect(h, &mut r);
        ClientToScreen(device_window(owner), &mut o);
    }
    // A form centred on one of ours (a Yes/No over Options) sits where that one is drawn,
    // not where it was moved to.
    let mut origin = (r.left - o.x, r.top - o.y);
    if let Some(p) = tracked().iter().find(|t| t.hwnd == owner && t.hidden) {
        let mut pr = WinRect::default();
        unsafe { GetWindowRect(owner, &mut pr) };
        origin = (p.origin.0 + r.left - pr.left, p.origin.1 + r.top - pr.top);
    }
    let first = !tracked().iter().any(|t| t.class != "#32770");
    let in_mission = crate::engine::api::ingame::recent();
    tracked().push(Tracked {
        hwnd: h,
        class: class.clone(),
        obj: 0,
        opened: crate::rt::now(),
        origin,
        size: (r.right - r.left, r.bottom - r.top),
        hidden: false,
        last: Vec::new(),
        in_mission,
        visit,
        pressed: false,
        combo: false,
    });
    crate::log!(
        "forms: {} {:p} up at {},{} {}x{}{}",
        class, h, origin.0, origin.1, r.right - r.left, r.bottom - r.top, if in_mission { ", over a mission" } else { "" }
    );
    if first && !dialog {
        crate::input::set_hosting(true);
        unsafe { HOSTING = true };
        crate::menus::backdrop::begin(if in_mission { "forms" } else { "main menu" });
    }
    unsafe {
        if TIMER == 0 {
            TIMER = SetTimer(core::ptr::null_mut(), 0, 16, tick);
        }
    }
}

/// The device window: the form's owner, or its owner's (a Yes/No is owned by the screen
/// that asked it).
fn device_window(owner: HWND) -> HWND {
    let mut w = owner;
    for _ in 0..4 {
        if w.is_null() || class_of(w) == "TMainWindowForm" {
            return w;
        }
        w = unsafe { GetWindow(w, GW_OWNER) };
    }
    owner
}

fn destroyed(h: HWND) {
    let v = tracked();
    let Some(i) = v.iter().position(|t| t.hwnd == h) else { return };
    let t = v.remove(i);
    crate::log!("forms: {} {:p} closed after {:.1} s ({} frames of ours so far)", t.class, h, crate::rt::now() - t.opened, unsafe { FRAMES });
    if v.iter().all(|t| t.class == "#32770") {
        stop();
    }
}

/// No form of ours is up: the timer stops, the engine has its input back.
fn stop() {
    unsafe {
        if TIMER != 0 {
            KillTimer(core::ptr::null_mut(), TIMER);
            TIMER = 0;
        }
        (*core::ptr::addr_of_mut!(VIEW)).clear();
        (*core::ptr::addr_of_mut!(ACTIONS)).clear();
        if HOSTING {
            HOSTING = false;
            crate::input::set_hosting(false);
            crate::menus::backdrop::end();
        }
    }
}

// ---- reading the objects (decision 143) -----------------------------------------------

fn ptr_ok(p: usize, n: usize) -> bool {
    (0x10000..0x8000_0000).contains(&p) && p % 4 == 0 && super::readable(p, n)
}

fn in_fo(p: usize) -> bool {
    let (a, b) = unsafe { FO };
    p >= a && p < b
}

fn rd<T: Copy>(p: usize) -> T {
    unsafe { super::rd::<T>(p) }
}

/// A counted string (length byte first).
fn short_string(p: usize) -> Option<String> {
    if p == 0 || !super::readable(p, 1) {
        return None;
    }
    let n = rd::<u8>(p) as usize;
    if !super::readable(p + 1, n) {
        return None;
    }
    let b = unsafe { core::slice::from_raw_parts((p + 1) as *const u8, n) };
    core::str::from_utf8(b).ok().map(String::from)
}

/// The VCL's class layout (Delphi 2007): a class's name and its parent sit at fixed
/// slots before the class pointer, the language runtime's, not FleetOpsHook's.
const VMT_CLASS_NAME: usize = 0x2c; // not an offset into game memory: a VCL class slot, before the class pointer
const VMT_PARENT: usize = 0x24; // not an offset into game memory: a VCL class slot, before the class pointer

/// A class's name, when its class pointer lies in FleetOpsHook's image.
fn class_name(vmt: usize) -> Option<String> {
    if !in_fo(vmt) {
        return None;
    }
    let p = rd::<usize>(vmt - VMT_CLASS_NAME);
    if !in_fo(p) { None } else { short_string(p) }
}

/// Whether the class `vmt` is `name` or derives from it.
fn derives(vmt: usize, name: &str) -> bool {
    let mut v = vmt;
    for _ in 0..40 {
        match class_name(v) {
            Some(n) if n == name => return true,
            Some(_) => {}
            None => return false,
        }
        let pp = rd::<usize>(v - VMT_PARENT);
        if !in_fo(pp) {
            return false;
        }
        v = rd::<usize>(pp);
    }
    false
}

fn latin1(c: u8) -> char {
    char::from_u32(c as u32).unwrap_or('?')
}

/// A Delphi long string (characters, length in the dword before).
fn ansi_string(p: usize) -> String {
    if p < 4 || !super::readable(p - 4, 4) {
        return String::new();
    }
    let n = (rd::<u32>(p - 4) as usize).min(256);
    if !super::readable(p, n) {
        return String::new();
    }
    let b = unsafe { core::slice::from_raw_parts(p as *const u8, n) };
    b.iter().map(|&c| latin1(c)).collect()
}

fn c_string(p: usize) -> String {
    let mut s = String::new();
    if p == 0 {
        return s;
    }
    for i in 0..512 {
        if !super::readable(p + i, 1) {
            break;
        }
        let c = rd::<u8>(p + i);
        if c == 0 {
            break;
        }
        s.push(latin1(c));
    }
    s
}

static mut FOUND: usize = 0;

unsafe extern "system" fn prop(_h: HWND, name: *const u8, data: HANDLE, _lp: usize) -> i32 {
    // An atom (a small integer) is not a name.
    if (name as usize) < 0x10000 {
        return 1;
    }
    if c_string(name as usize) == unsafe { (*core::ptr::addr_of!(PROP)).as_str() } {
        unsafe { FOUND = data as usize };
        return 0;
    }
    1
}

/// The form's object (decision 142).
fn object_of(h: HWND, class: &str) -> usize {
    unsafe {
        FOUND = 0;
        EnumPropsExA(h, prop, 0);
    }
    let obj = unsafe { FOUND };
    if obj == 0 || !ptr_ok(obj, 4) {
        crate::log!("forms: {:p} ({}) has no object of the VCL's: left to Roots", h, class);
        return 0;
    }
    let vmt = rd::<usize>(obj);
    let cn = class_name(vmt);
    if cn.as_deref() != Some(class) || !derives(vmt, "TCustomForm") {
        crate::log!("forms: {:p}: its object's class {:?} is not {} in FleetOpsHook: left to Roots", h, cn, class);
        return 0;
    }
    obj
}

/// Whether the form's object can be read now: still its window's, the modal form, not
/// being destroyed.
fn live(t: &Tracked) -> bool {
    if t.obj == 0 || unsafe { IsWindow(t.hwnd) } == 0 || !ptr_ok(t.obj, field::FORMS_FORM_STATE() + 4) {
        return false;
    }
    if !in_fo(rd::<usize>(t.obj)) {
        return false;
    }
    let modal = rd::<u8>(t.obj + field::FORMS_FORM_STATE()) & FS_MODAL != 0;
    let dying = rd::<u8>(t.obj + field::FORMS_COMP_STATE()) & CS_DESTROYING != 0;
    modal && !dying
}

fn kind_of(vmt: usize, class: &str, windowed: bool) -> Kind {
    if class == "TJetButton" {
        Kind::Button
    } else if derives(vmt, "TCustomLabel") {
        Kind::Label
    } else if derives(vmt, "TImage") {
        Kind::Image
    } else if windowed && derives(vmt, "TCustomComboBox") {
        Kind::Combo
    } else if windowed && derives(vmt, "TCustomEdit") && !derives(vmt, "TCustomMemo") {
        Kind::Edit
    } else {
        Kind::Other
    }
}

/// A windowed control's text, through its window.
fn window_text(h: HWND) -> String {
    let mut b = [0u8; 256];
    let n = unsafe { SendMessageA(h, WM_GETTEXT, b.len(), b.as_mut_ptr() as isize) }.clamp(0, 255) as usize;
    b[..n].iter().map(|&c| latin1(c)).collect()
}

/// A combo's entries, through its window. Roots' combos draw HTML: the tags are dropped.
fn combo_items(h: HWND) -> (Vec<String>, i32) {
    let n = unsafe { SendMessageA(h, CB_GETCOUNT, 0, 0) }.clamp(0, 256);
    let mut v = Vec::new();
    for i in 0..n {
        let len = unsafe { SendMessageA(h, CB_GETLBTEXTLEN, i as usize, 0) };
        if !(0..1024).contains(&len) {
            v.push(String::new());
            continue;
        }
        let mut b = alloc::vec![0u8; len as usize + 1];
        unsafe { SendMessageA(h, CB_GETLBTEXT, i as usize, b.as_mut_ptr() as isize) };
        v.push(strip_tags(&b[..len as usize]));
    }
    (v, unsafe { SendMessageA(h, CB_GETCURSEL, 0, 0) } as i32)
}

fn strip_tags(b: &[u8]) -> String {
    let mut s = String::new();
    let mut tag = false;
    for &c in b {
        match c {
            b'<' => tag = true,
            b'>' => tag = false,
            _ if !tag => s.push(latin1(c)),
            _ => {}
        }
    }
    String::from(s.trim())
}

/// The form's components that are controls, read now.
fn controls(obj: usize) -> Vec<Control> {
    let mut out = Vec::new();
    let list = rd::<usize>(obj + field::FORMS_COMP_COMPONENTS());
    if !ptr_ok(list, field::FORMS_LIST_COUNT() + 4) {
        return out;
    }
    let (items, n) = (rd::<usize>(list + field::FORMS_LIST_ITEMS()), rd::<i32>(list + field::FORMS_LIST_COUNT()));
    if n <= 0 || n > 512 || !ptr_ok(items, n as usize * 4) {
        return out;
    }
    for i in 0..n as usize {
        let c = rd::<usize>(items + i * 4);
        if !ptr_ok(c, field::FORMS_WIN_HANDLE() + 4) {
            continue;
        }
        let vmt = rd::<usize>(c);
        let Some(class) = class_name(vmt) else { continue };
        if !derives(vmt, "TControl") {
            continue;
        }
        let r: [i32; 4] = core::array::from_fn(|k| rd::<i32>(c + field::FORMS_CTL_LEFT() + k * 4));
        let windowed = derives(vmt, "TWinControl");
        let hwnd = if windowed { rd::<usize>(c + field::FORMS_WIN_HANDLE()) } else { 0 };
        let kind = kind_of(vmt, &class, windowed);
        let mut text = c_string(rd::<usize>(c + field::FORMS_CTL_TEXT()));
        let (mut items, mut selected) = (Vec::new(), -1);
        if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) } != 0 {
            match kind {
                Kind::Edit => text = window_text(hwnd as HWND),
                Kind::Combo => (items, selected) = combo_items(hwnd as HWND),
                _ => {}
            }
        }
        out.push(Control {
            name: ansi_string(rd::<usize>(c + field::FORMS_COMP_NAME())),
            class,
            kind,
            rect: r,
            text,
            visible: rd::<u8>(c + field::FORMS_CTL_VISIBLE()) != 0,
            enabled: rd::<u8>(c + field::FORMS_CTL_ENABLED()) != 0,
            hwnd,
            items,
            selected,
        });
    }
    out
}

fn log_controls(t: &Tracked, v: &[Control]) {
    crate::log!("forms: {} has {} controls:", t.class, v.len());
    for c in v {
        crate::log!(
            "forms:   {:?} {} {} {},{} {}x{} {}{} text {:?}{}",
            c.kind, c.class, c.name, c.rect[0], c.rect[1], c.rect[2], c.rect[3],
            if c.visible { "visible" } else { "hidden" }, if c.enabled { "" } else { " disabled" }, c.text,
            if c.items.is_empty() { String::new() } else { crate::sfmt!(" ({} items, {} chosen)", c.items.len(), c.selected) }
        );
    }
}

// ---- the pump (decision 144) ------------------------------------------------------------

unsafe extern "system" fn tick(_h: HWND, _m: u32, _id: usize, _t: u32) {
    unsafe { TICKS += 1 };
    let now = crate::rt::now();
    let v = tracked();
    v.retain(|t| unsafe { IsWindow(t.hwnd) } != 0);
    if !v.iter().any(|t| t.class != "#32770") {
        stop();
        return;
    }
    let top = v.len() - 1;
    if v[top].class == "#32770" {
        // One of the nine dialogs FleetOpsHook opens through its own import: Roots' own
        // until phase two (decision 150). Our last frame stays under it.
        unsafe { DIALOG_TICKS += 1 };
        return;
    }
    run_actions(v[top].hwnd);
    let t = &mut v[top];
    if t.obj == 0 {
        if t.class.ends_with(')') {
            return;
        }
        t.obj = object_of(t.hwnd, &t.class);
        if t.obj == 0 {
            // Not ours: left as Roots draws it (not moved, so it shows).
            t.class.push_str(" (Roots')");
            return;
        }
    }
    if !live(t) {
        return;
    }
    let ctl = controls(t.obj);
    if !t.hidden {
        t.hidden = true;
        hide(t.hwnd);
        log_controls(t, &ctl);
        crate::log!("forms: {} is ours: moved off screen, active {:p}, focus {:p}", t.class, unsafe { GetActiveWindow() }, unsafe { GetFocus() });
    }
    keep_off(t.hwnd);
    poll_button(t.hwnd);
    t.last = ctl;
    // Nothing else sets the pointer's shape once the form is away from it (decision 171).
    unsafe { SetCursor(LoadCursorA(core::ptr::null_mut(), IDC_ARROW)) };
    if now - unsafe { PRINTED_AT } >= PRINT_EVERY {
        unsafe { PRINTED_AT = now };
        print(t.hwnd, t.size.0, t.size.1);
    }
    if unsafe { MODE } == 2 {
        if now - unsafe { LAST_DIAG } > 2.0 {
            unsafe { LAST_DIAG = now };
            diag(t.hwnd, &t.class);
        }
        let age = now - t.opened;
        script(t, age);
    }
    // What the frame draws: the forms that are ours, outermost first; only the last is live.
    let view: Vec<Form> = v
        .iter()
        .enumerate()
        .filter(|(_, t)| t.obj != 0 && t.hidden)
        .map(|(i, t)| Form {
            hwnd: t.hwnd as usize,
            class: t.class.clone(),
            origin: t.origin,
            size: t.size,
            controls: t.last.clone(),
            top: i == top,
            in_mission: t.in_mission,
        })
        .collect();
    unsafe { *core::ptr::addr_of_mut!(VIEW) = view };
    frame();
}

static mut BUTTON: bool = false;

/// The real pointer falls on the device window, whose procedure hands its mouse to the
/// engine's message functions, where the UI records it; under a form that leaves the
/// device window disabled (Yes/No, Manual IP) no click arrives at all, so the pump reads
/// the left button itself and records its changes (decision 183).
fn poll_button(top: HWND) {
    let down = unsafe { GetAsyncKeyState(1) } as u16 & 0x8000 != 0;
    let was = unsafe { core::mem::replace(&mut BUTTON, down) };
    let dev = device_window(unsafe { GetWindow(top, GW_OWNER) });
    if dev.is_null() || unsafe { IsWindowEnabled(dev) } != 0 || down == was {
        return;
    }
    crate::input::note_message(if down { WM_LBUTTONDOWN } else { WM_LBUTTONUP }, if down { MK_LBUTTON } else { 0 });
}

/// Decision 170: off screen, its region emptied as well (on a Wine where the region
/// works, nothing of it shows on the way).
fn hide(h: HWND) {
    unsafe {
        SetWindowRgn(h, CreateRectRgn(0, 0, 0, 0), 1);
        SetWindowPos(h, core::ptr::null_mut(), OFF, OFF, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// FleetOpsHook re-places its full-screen forms when the device window moves: off again.
fn keep_off(h: HWND) {
    let mut r = WinRect::default();
    unsafe { GetWindowRect(h, &mut r) };
    if r.left != OFF || r.top != OFF {
        crate::log!("forms: {:p} was moved back to {},{}: off screen again", h, r.left, r.top);
        unsafe { SetWindowPos(h, core::ptr::null_mut(), OFF, OFF, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }
}

fn lparam(x: i32, y: i32) -> isize {
    ((y as u32) << 16 | (x as u32 & 0xffff)) as isize
}

/// The actions the frame queued, on the top form only (one meant for a form no longer on
/// top is dropped). Run before the form is read again, so the next frame shows what they
/// did.
fn run_actions(top: HWND) {
    let v = core::mem::take(unsafe { &mut *core::ptr::addr_of_mut!(ACTIONS) });
    for a in v {
        match a {
            Action::Press(h, r) if h == top as usize => {
                let lp = lparam(r[0] + r[2] / 2, r[1] + r[3] / 2);
                unsafe {
                    PostMessageA(top, WM_MOUSEMOVE, 0, lp);
                    PostMessageA(top, WM_LBUTTONDOWN, MK_LBUTTON, lp);
                    PostMessageA(top, WM_LBUTTONUP, 0, lp);
                }
            }
            Action::Mouse(h, msg, x, y) if h == top as usize => {
                // To the deepest visible window at the point, in its own client coordinates.
                let mut target = top;
                let mut p = Point { x, y };
                for _ in 0..8 {
                    let c = unsafe { ChildWindowFromPointEx(target, p, CWP_SKIPINVISIBLE) };
                    if c.is_null() || c == target {
                        break;
                    }
                    let mut r = WinRect::default();
                    let mut o = Point::default();
                    unsafe {
                        GetWindowRect(c, &mut r);
                        ClientToScreen(target, &mut o);
                    }
                    p = Point { x: p.x - (r.left - o.x), y: p.y - (r.top - o.y) };
                    target = c;
                }
                let keys = if msg == WM_LBUTTONDOWN || msg == WM_MOUSEMOVE { MK_LBUTTON } else { 0 };
                unsafe { PostMessageA(target, msg, keys, lparam(p.x, p.y)) };
            }
            Action::Select(h, i) => unsafe {
                let w = h as HWND;
                let parent = GetParent(w);
                if IsWindow(w) != 0 && !parent.is_null() {
                    SendMessageA(w, CB_SETCURSEL, i as usize, 0);
                    let id = GetDlgCtrlID(w) as usize & 0xffff;
                    SendMessageA(parent, WM_COMMAND, (CBN_SELCHANGE << 16) | id, w as isize);
                }
            },
            Action::Focus(h) => unsafe {
                if IsWindow(h as HWND) != 0 {
                    SetFocus(h as HWND);
                    // EM_SETSEL over the whole text: typing replaces it, as a click into a
                    // field and a select-all would.
                    SendMessageA(h as HWND, 0xb1, 0, -1);
                }
            },
            _ => {}
        }
    }
}

/// The top form printed into a memory DC (the VCL answers the print message wherever the
/// window is): the mirror's picture.
fn print(h: HWND, w: i32, ht: i32) {
    if w <= 0 || ht <= 0 {
        return;
    }
    unsafe {
        let p = &mut *core::ptr::addr_of_mut!(PICTURE);
        if p.4 != w || p.5 != ht {
            if !p.0.is_null() {
                SelectObject(p.0, p.2);
                DeleteObject(p.1);
                DeleteDC(p.0);
            }
            *p = (core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), 0, 0);
            let bi = BitmapInfoHeader { size: 40, width: w, height: -ht, planes: 1, bit_count: 32, ..Default::default() };
            let dc = CreateCompatibleDC(core::ptr::null_mut());
            let mut bits = core::ptr::null_mut();
            let bmp = CreateDIBSection(dc, &bi, 0, &mut bits, core::ptr::null_mut(), 0);
            if bmp.is_null() || bits.is_null() {
                DeleteDC(dc);
                return;
            }
            let old = SelectObject(dc, bmp);
            *p = (dc, bmp, old, bits, w, ht);
        }
        PatBlt(p.0, 0, 0, w, ht, BLACKNESS);
        SendMessageA(h, WM_PRINT, p.0 as usize, (PRF_CLIENT | PRF_CHILDREN | PRF_ERASEBKGND | PRF_NONCLIENT) as isize);
        GdiFlush();
        PRINTS = PRINTS.wrapping_add(1);
    }
}

/// One frame of ours on the engine's device, presented from the timer.
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
    dev8::present(dev);
    unsafe {
        FRAMES += 1;
        if FRAMES == 1 {
            crate::log!("forms: first frame of ours, {}x{}", bb.width, bb.height);
        }
    }
}

// ---- the probe (step 0) ---------------------------------------------------------------

fn diag(form: HWND, class: &str) {
    let mut p = Point::default();
    unsafe { GetCursorPos(&mut p) };
    let under = unsafe { WindowFromPoint(p) };
    crate::log!(
        "forms: {} {:p} up; tick {} frame {} ({} inside a dialog); pointer {},{} over {}; UI {:?}; active {:p} focus {:p}",
        class, form, unsafe { TICKS }, unsafe { FRAMES }, unsafe { DIALOG_TICKS }, p.x, p.y, class_of(under),
        crate::input::state().mouse, unsafe { GetActiveWindow() }, unsafe { GetFocus() }
    );
}

/// The probe's walk: one press per screen (main menu -> Options -> Graphics -> Back ->
/// Back -> Instant Action), a combo changed on the graphics screen.
fn script(t: &mut Tracked, age: f64) {
    let target = match (t.class.as_str(), t.visit) {
        // The main menu's field names are swapped against their actions on Roots: the
        // one named for the map editor runs Options.
        ("TMainForm", 1) => Some("buttonMapEdit"),
        ("TMainForm", 2) => Some("buttonInstantAction"),
        ("TOptionsMenuForm", 1) => Some("btnGraphicSettings"),
        ("TOptionsMenuForm", 2) => Some("buttonBack"),
        ("TGraphicOptionsForm", _) => Some("buttonBack"),
        _ => None,
    };
    if t.class == "TGraphicOptionsForm" && !t.combo && age > 6.0 {
        t.combo = true;
        if let Some(d) = t.last.iter().find(|c| c.name == "comboDevicePrimary") {
            crate::log!("forms: the walk re-selects {} entry {} of {}", d.name, d.selected, d.items.len());
            act(Action::Select(d.hwnd, d.selected.max(0)));
        }
    }
    let Some(name) = target else { return };
    let wait = if t.class == "TMainForm" && t.visit == 1 { 25.0 } else { 12.0 };
    if !t.pressed && age > wait {
        t.pressed = true;
        match t.last.iter().find(|c| c.name == name) {
            Some(c) if c.visible => {
                crate::log!("forms: the walk presses {} ({:?}) of {}", c.name, c.text, t.class);
                act(Action::Press(t.hwnd as usize, c.rect));
            }
            _ => crate::log!("forms: the walk finds no visible {} on {}", name, t.class),
        }
    }
}
