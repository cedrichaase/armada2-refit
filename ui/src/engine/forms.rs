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
//!   rectangle on Wine under XWayland: decision 170), from the moment it is shown, so the
//!   window manager never sees it (decision 310);
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
use super::forms_props;
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

/// The forms the UI draws. Any other (Tunngle's wait) stays Roots'
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
    "TMapEditForm",
    "TMapEditCreateForm",
    "TMapSaveForm",
    "TMessageForm",
];

const WH_CBT: i32 = 5;
const WH_CALLWNDPROC: i32 = 4;
const WM_WINDOWPOSCHANGING: u32 = 0x46;
const SWP_SHOWWINDOW_: u32 = 0x40;
const SWP_NOMOVE_: u32 = 0x2;
const SWP_NOSIZE_: u32 = 0x1;
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
    fn FindWindowExA(parent: HWND, after: HWND, class: *const u8, name: *const u8) -> HWND;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateRectRgn(l: i32, t: i32, r: i32, b: i32) -> HANDLE;
}

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// The adopted dialogs that belong to the front end, never to a mission's menus.
const FRONT_END_SCREENS: [&str; 3] = ["single player", "mission select", "game setup"];

/// An adopted dialog's screen, by its stock procedure (`Dialog::stock_proc`), named as
/// `engine::dialogs::screen_name` names Armada II's: "" when unknown, or when the build's
/// procedures are not verified for it (`forms.dialog_specs`; decision 250).
pub fn dialog_screen(stock_proc: usize) -> &'static str {
    use crate::syms_forms::sym;
    if stock_proc == 0 || eng().ready(hook::FORMS_DIALOG_SPECS).is_err() {
        return "";
    }
    const SCREENS: [(a2engine::Sym, &str); 13] = [
        (sym::FORMS_PROC_SINGLE_PLAYER, "single player"),
        (sym::FORMS_PROC_MISSION_SELECT, "mission select"),
        (sym::FORMS_PROC_LOAD_GAME, "load game"),
        (sym::FORMS_PROC_SAVE_GAME, "save game"),
        (sym::FORMS_PROC_GAME_SETTINGS, "game settings"),
        (sym::FORMS_PROC_GAME_SETUP, "game setup"),
        (sym::FORMS_PROC_ADMIRALS_LOG, "admiral's log"),
        (sym::FORMS_PROC_MESSAGE, "message"),
        (sym::FORMS_PROC_LICENCE, "licence agreement"),
        (sym::FORMS_PROC_INTERNET_GAME, "internet game"),
        (sym::FORMS_PROC_NET_WAITING, "net waiting"),
        (sym::FORMS_PROC_HELP, "help"),
        (sym::FORMS_PROC_GAME_NAME, "game name"),
    ];
    SCREENS.iter().find(|s| eng().addr(s.0) == stock_proc).map_or("", |s| s.1)
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
    /// A check box's state, read from its published `Checked` (None: not a check box, or
    /// its class reads it through a method).
    pub checked: Option<bool>,
    /// A slider's published `Value`, `MinValue`, `MaxValue` and `Increment` (None as above).
    pub value: Option<(i32, i32, i32, i32)>,
    /// A grid's cells, read from its memory (Map Select's list, the Mods list).
    pub grid: Option<Grid>,
}

/// A NextSuite grid's rows as read at the last tick: plain copies of the cells that hold
/// text or a number, row by row.
#[derive(Clone, Default)]
pub struct Grid {
    pub rows: Vec<Vec<String>>,
    /// The selected row (-1: none) and the first row the grid shows.
    pub selected: i32,
    pub first: i32,
    /// The grid's row and header heights in the form's pixels (its published `RowSize` and
    /// `HeaderSize`; 0 when the class does not hold them in a field).
    pub row_h: i32,
    pub head_h: i32,
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
    /// The window's caption (a VCL message box's title: "Warning", "Confirm").
    pub caption: String,
    /// Whether this is the modal form now (the others are under it, drawn inert).
    pub top: bool,
    /// Whether this form opened over a mission (the in-mission menu).
    pub in_mission: bool,
    /// Its place among the forms and dialogs, outermost first (the frame draws in this order).
    pub z: usize,
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
    /// The mouse at a point of the form or adopted dialog (forwarded to the window under
    /// it): down, move or up, and the buttons held (`MK_LBUTTON` or 0).
    Mouse(usize, u32, usize, i32, i32),
    /// A combo's entry chosen, with the parent's notification.
    Select(usize, i32),
    /// The keyboard to a windowed control (a text field clicked).
    Focus(usize),
    /// A key or a character for a windowed control (`WM_KEYDOWN` or `WM_CHAR`, its
    /// wParam), as the player's typing would send it: an adopted dialog's edit rewritten
    /// by a text field of ours (decision 250).
    Key(usize, u32, usize),
    /// A mouse message sent (not posted) to the control at a point of the form, with the
    /// form moved for that moment so the point lies under the real pointer: the library's
    /// check boxes decide where a click fell from the real pointer's place, which a form
    /// kept off screen never has under it. (form, message, buttons, x, y)
    Poke(usize, u32, usize, i32, i32),
    /// A key pressed and released at a windowed control (a grid's arrow keys), sent to its
    /// window. (window, virtual key)
    GridKey(usize, u32),
}

pub const MOUSE_DOWN: u32 = WM_LBUTTONDOWN;
pub const MOUSE_MOVE: u32 = WM_MOUSEMOVE;
pub const MOUSE_UP: u32 = WM_LBUTTONUP;
pub const MOUSE_HELD: usize = MK_LBUTTON;

/// A dialog over the device window (one of the nine FleetOpsHook opens through its own
/// import, or one they open), as the frame draws it.
#[derive(Clone, Default)]
pub struct Dialog {
    pub hwnd: usize,
    /// Adopted (decision 230): its place in the design space, where the frame draws its
    /// picture. None: framed as a Win32 window (decision 200), at `framed`.
    pub design: Option<[i32; 4]>,
    /// Where the window stands on the device window's client area (a framed window), and
    /// that area's size.
    pub framed: [i32; 4],
    pub client: (i32, i32),
    /// Whether it is the modal one now (an adopted dialog under another is inert).
    pub top: bool,
    /// Its child windows (an adopted dialog's model; empty for a framed one).
    pub children: Vec<crate::engine::fodialogs::Child>,
    /// FleetOpsHook's record of the stock procedure (0: not read): what a spec will be
    /// chosen by.
    pub stock_proc: usize,
    /// Its place among the forms and dialogs, outermost first (the frame draws in this order).
    pub z: usize,
    /// A pane of the dialog under it (drawn as part of it, with no frame of its own).
    pub pane: bool,
    /// Whether the window is shown (a pane the Admiral's Log hides is not drawn).
    pub visible: bool,
}

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
    /// A dialog over the device window (one of the nine FleetOpsHook opens, or one they
    /// open), framed by the UI: where it was put, in the device window's client
    /// coordinates (decision 200).
    framed: Option<[i32; 4]>,
    /// Adopted (decision 230): drawn by the UI from its picture, its window off screen;
    /// its place in the design space.
    adopted: Option<[i32; 4]>,
    /// Where the window stood on the screen when it activated, before it was placed: a
    /// pane keeps its place against its owner's (decision 235).
    orig: (i32, i32),
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
/// The classes `activated` saw over the forms and does not take, each logged once.
static mut UNTAKEN: Vec<String> = Vec::new();
static mut FO: (usize, usize) = (0, 0);
static mut PROP: String = String::new();
static mut TICKS: u32 = 0;
static mut DIALOG_TICKS: u32 = 0;
static mut FRAMES: u32 = 0;
static mut LAST_DIAG: f64 = 0.0;
static mut ACTIONS: Vec<Action> = Vec::new();
/// The forms as the frame draws them, outermost first.
static mut VIEW: Vec<Form> = Vec::new();
/// The framed dialogs over them, outermost first: each one's rectangle in the device
/// window's client coordinates, and that client area's size.
static mut DIALOGS: Vec<Dialog> = Vec::new();
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
/// Whether Roots' forms are the UI's (`[Menus] Forms=1`, installed on a FleetOpsHook of
/// the family), not the probe's.
pub fn forms_ours() -> bool {
    unsafe { MODE == 1 }
}

pub fn active() -> bool {
    unsafe { MODE != 0 && TIMER != 0 && !((*core::ptr::addr_of!(VIEW)).is_empty() && (*core::ptr::addr_of!(DIALOGS)).is_empty()) }
}

/// Whether the main menu's form is the screen shown (the menu scene's main view).
pub fn main_menu_shown() -> bool {
    let v = unsafe { &*core::ptr::addr_of!(VIEW) };
    let no_dialog = unsafe { (*core::ptr::addr_of!(DIALOGS)).is_empty() };
    no_dialog && v.last().is_some_and(|f| f.class == "TMainForm")
}

/// The dialogs the UI frames (decision 200), outermost first: each one's rectangle on
/// the device window's client area, and that area's size.
pub fn dialogs() -> Vec<Dialog> {
    unsafe { (*core::ptr::addr_of!(DIALOGS)).clone() }
}

/// Whether a tracked window keeps the pump going: a form of ours, or a framed dialog.
fn ours(t: &Tracked) -> bool {
    t.class != "#32770" || t.framed.is_some() || t.adopted.is_some()
}

/// Whether `h` is an adopted pane of `top` (the Admiral's Log's pop-up panes).
fn pane_of(h: HWND, top: HWND) -> bool {
    crate::engine::fodialogs::is_adopted(h) && unsafe { GetWindow(h, GW_OWNER) } == top
}

/// Whether the window on top of the ones tracked is an adopted dialog.
fn dialog_adopted_on_top() -> bool {
    tracked().last().is_some_and(|t| t.adopted.is_some())
}

/// An adopted dialog's picture, for the mirror: (bits, width, height, serial).
pub fn dialog_picture(hwnd: usize) -> Option<(*const u8, i32, i32, u32)> {
    crate::engine::fodialogs::picture(hwnd as HWND)
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

/// The words of the Yes/No prompt: its title and question, read from the executable's
/// buffers (`forms.message_text`, decision 540). None while the hook is not ready or the
/// question is empty; the frame then draws the form's own picture instead.
pub struct Message {
    pub title: String,
    pub text: String,
}

pub fn message() -> Option<Message> {
    use crate::syms_forms::sym;
    if eng().ready(hook::FORMS_MESSAGE_TEXT).is_err() {
        return None;
    }
    let read = |s: a2engine::Sym| {
        let p = eng().addr(s);
        if p == 0 { String::new() } else { c_string(p).chars().take(127).collect::<String>() }
    };
    let text = read(sym::FORMS_YES_NO_MESSAGE);
    if text.trim().is_empty() {
        return None;
    }
    Some(Message { title: read(sym::FORMS_YES_NO_TITLE), text })
}

/// Queues an action on the top form, run at the next tick.
pub fn act(a: Action) {
    unsafe { (*core::ptr::addr_of_mut!(ACTIONS)).push(a) };
}

/// A press of ours on a control.
pub fn press(form: &Form, c: &Control) {
    crate::log!("forms: press on {} ({:?}) of {}", c.name, c.text, form.class);
    if c.hwnd != 0 {
        // A windowed button (the VCL's `TButton`, on the map editor's Save Map form) is a
        // window of its own: the press goes to it, down and up at its centre, as a
        // player's click would reach it. A graphic button has no window; the form finds it.
        let (x, y) = (c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2);
        act(Action::Mouse(form.hwnd, MOUSE_DOWN, MOUSE_HELD, x, y));
        act(Action::Mouse(form.hwnd, MOUSE_UP, 0, x, y));
        return;
    }
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
    // Shown off screen from the start (decision 310): a window a form or a dialog of ours
    // would show is moved off before it is first mapped, not after it activated.
    if !h.is_null() && mode == 1 {
        let s = unsafe { SetWindowsHookExA(WH_CALLWNDPROC, call_wnd, core::ptr::null_mut(), tid) };
        unsafe { SHOW_HOOK = s };
        crate::log!(
            "forms: the windows of ours are shown off screen from the start (window-procedure hook {})",
            if s.is_null() { "refused: they show until they activate" } else { "on" }
        );
    }
    if !h.is_null() {
        // FleetOpsHook's dialogs drawn by the UI (decision 230), or framed windows when
        // the executable's paint imports cannot be ours (decision 200).
        crate::engine::fodialogs::install(tid);
    }
    !h.is_null()
}

static mut SHOW_HOOK: HANDLE = core::ptr::null_mut();
/// Where a window shown off screen by `call_wnd` would have stood (screen coordinates),
/// for `activated` to place it from.
static mut WOULD: Vec<(HWND, WinRect)> = Vec::new();

/// `WINDOWPOS` (user32's).
#[repr(C)]
struct WindowPos {
    hwnd: HWND,
    after: HWND,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: u32,
}

/// `CWPSTRUCT` (user32's).
#[repr(C)]
struct CwpStruct {
    lp: isize,
    wp: usize,
    msg: u32,
    hwnd: HWND,
}

/// Windows `call_wnd` subclassed: the window and the procedure before ours.
static mut SUBCLASSED: Vec<(HWND, isize)> = Vec::new();
const WM_NCDESTROY: u32 = 0x82;

/// Decision 310: a window the forms would take (a form of `TAKEN` whose object is the
/// modal form, a `#32770` over the device window while the dialogs are adopted) is moved
/// off screen in the `WM_WINDOWPOSCHANGING` that shows it, before Wine maps it: hidden
/// only once it activated (decision 170), it stood on the screen for a moment as a window
/// of its own, which the window manager managed (a floating dialog that took the focus and
/// the full screen from the game). A thread `WH_CALLWNDPROC` hook sees each window of the
/// game's thread a message is sent to while it is still hidden and subclasses the ones the
/// forms may take (a VCL form once the VCL put its own procedure in, so that it is ours the
/// VCL chains to, not the other way round); `show_proc` then moves the window. The hook's
/// own copy of the message cannot change it (under Proton's WoW64 it is a copy: the first
/// take, which rewrote it, changed nothing). The position it was given is kept for
/// `activated`.
unsafe extern "system" fn call_wnd(code: i32, wp: usize, lp: isize) -> isize {
    if code == 0 && lp != 0 {
        let h = unsafe { (*(lp as *const CwpStruct)).hwnd };
        if !h.is_null() && unsafe { IsWindowVisible(h) } == 0 {
            let done = unsafe { &mut *core::ptr::addr_of_mut!(SUBCLASSED) };
            if !done.iter().any(|e| e.0 == h) && may_take(h) {
                let old = unsafe { SetWindowLongA(h, GWL_WNDPROC, show_proc as *const () as usize as isize) };
                if old != 0 {
                    done.retain(|e| unsafe { IsWindow(e.0) } != 0);
                    done.push((h, old));
                }
            }
        }
    }
    unsafe { CallNextHookEx(SHOW_HOOK, code, wp, lp) }
}

/// Whether `h` is a window the forms may take when it is shown, and its procedure is
/// settled: a `#32770` over the device window, or a form of `TAKEN` whose window procedure
/// is no longer the VCL's creation procedure (which lies in FleetOpsHook's image and puts
/// the form's own procedure in at the first message).
fn may_take(h: HWND) -> bool {
    let class = class_of(h);
    if class == "#32770" {
        return crate::engine::fodialogs::ready() && !device_of(h).is_null();
    }
    TAKEN.contains(&class.as_str()) && !in_fo(unsafe { GetWindowLongA(h, GWL_WNDPROC) } as usize)
}

unsafe extern "system" fn show_proc(h: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    let done = unsafe { &mut *core::ptr::addr_of_mut!(SUBCLASSED) };
    let Some(&(_, old)) = done.iter().find(|e| e.0 == h) else { return 0 };
    if msg == WM_WINDOWPOSCHANGING && lp != 0 {
        let pos = unsafe { &mut *(lp as *mut WindowPos) };
        if pos.flags & SWP_SHOWWINDOW_ != 0 && takes_at_show(h) {
            let mut r = WinRect::default();
            unsafe { GetWindowRect(h, &mut r) };
            let (w, ht) = (r.right - r.left, r.bottom - r.top);
            if pos.flags & SWP_NOMOVE_ == 0 {
                (r.left, r.top) = (pos.x, pos.y);
            }
            let (w, ht) = if pos.flags & SWP_NOSIZE_ == 0 { (pos.cx, pos.cy) } else { (w, ht) };
            (r.right, r.bottom) = (r.left + w, r.top + ht);
            let would = unsafe { &mut *core::ptr::addr_of_mut!(WOULD) };
            would.retain(|e| e.0 != h && unsafe { IsWindow(e.0) } != 0);
            would.push((h, r));
            pos.x = OFF;
            pos.y = OFF;
            pos.flags &= !SWP_NOMOVE_;
            crate::log!("forms: {} {:p} shown off screen (it was to stand at {},{} {}x{})", class_of(h), h, r.left, r.top, w, ht);
            // The next tick puts back one that did not activate (`restore_unactivated`).
            unsafe {
                if TIMER == 0 {
                    TIMER = SetTimer(core::ptr::null_mut(), 0, 16, tick);
                }
            }
        }
    }
    if msg == WM_NCDESTROY {
        unsafe { SetWindowLongA(h, GWL_WNDPROC, old) };
        done.retain(|e| e.0 != h);
    }
    unsafe { CallWindowProcA(old, h, msg, wp, lp) }
}

/// Whether `activated` will take `h` off screen: a dialog it adopts, or a form whose
/// object the pump will find and read (the checks of `object_of` and `live`, quietly).
fn takes_at_show(h: HWND) -> bool {
    let class = class_of(h);
    if class == "#32770" {
        return crate::engine::fodialogs::ready() && !device_of(h).is_null();
    }
    if !TAKEN.contains(&class.as_str()) {
        return false;
    }
    unsafe {
        FOUND = 0;
        EnumPropsExA(h, prop, 0);
    }
    let obj = unsafe { FOUND };
    if obj == 0 || !ptr_ok(obj, field::FORMS_FORM_STATE() + 4) {
        return false;
    }
    let vmt = rd::<usize>(obj);
    class_name(vmt).as_deref() == Some(class.as_str())
        && derives(vmt, "TCustomForm")
        && rd::<u8>(obj + field::FORMS_FORM_STATE()) & FS_MODAL != 0
        && rd::<u8>(obj + field::FORMS_COMP_STATE()) & CS_DESTROYING == 0
}

/// Where `h` stands, or where it would have stood when `call_wnd` showed it off screen
/// (taken: asked once, at its activation).
fn placed_rect(h: HWND) -> WinRect {
    let would = unsafe { &mut *core::ptr::addr_of_mut!(WOULD) };
    if let Some(i) = would.iter().position(|e| e.0 == h) {
        return would.remove(i).1;
    }
    let mut r = WinRect::default();
    unsafe { GetWindowRect(h, &mut r) };
    r
}

/// A window `call_wnd` showed off screen that did not then activate (so `activated` never
/// took it) is put back where it was to stand: only what the forms take stays off.
fn restore_unactivated() {
    let would = unsafe { &mut *core::ptr::addr_of_mut!(WOULD) };
    for (h, r) in core::mem::take(would) {
        if unsafe { IsWindow(h) } == 0 || tracked().iter().any(|t| t.hwnd == h) {
            continue;
        }
        crate::log!("forms: {} {:p} was shown off screen but did not activate: put back at {},{}", class_of(h), h, r.left, r.top);
        unsafe { SetWindowPos(h, core::ptr::null_mut(), r.left, r.top, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }
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
        // A window that activates over the forms and is not one of them (nor the device
        // window, `TMainWindowForm`, which a click on our frame activates) stands on the
        // screen as a window of its own: named once per class, for the next one to take.
        if !tracked().is_empty() && class != "TMainWindowForm" {
            let seen = unsafe { &mut *core::ptr::addr_of_mut!(UNTAKEN) };
            if !seen.contains(&class) {
                crate::log!("forms: {} {:p} activated over the forms: not one the forms take", class, h);
                seen.push(class);
            }
        }
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
    let r = placed_rect(h);
    let mut o = Point::default();
    unsafe {
        ClientToScreen(device_window(owner), &mut o);
    }
    // A form centred on one of ours (a Yes/No over Options) sits where that one is drawn,
    // not where it was moved to.
    let mut origin = (r.left - o.x, r.top - o.y);
    // Over an adopted dialog (Map Select over Game Setup) the same, from where the frame
    // draws that dialog.
    if let Some(p) = tracked().iter().find(|t| t.hwnd == owner && (t.hidden || t.adopted.is_some())) {
        let mut pr = WinRect::default();
        unsafe { GetWindowRect(owner, &mut pr) };
        let at = p.adopted.map_or(p.origin, |a| (a[0], a[1]));
        origin = (at.0 + r.left - pr.left, at.1 + r.top - pr.top);
    }
    // FleetOpsHook places a form opened over an adopted dialog (Map Select over Game Setup)
    // against the dialog's off-screen place: a form that would stand outside the design
    // space is centred in it.
    let (dw, dh) = design();
    let (fw, fh) = (r.right - r.left, r.bottom - r.top);
    if dialog_adopted_on_top() && dw > 0 && (origin.0 < 0 || origin.1 < 0 || origin.0 + fw > dw || origin.1 + fh > dh) {
        origin = (((dw - fw) / 2).max(0), ((dh - fh) / 2).max(0));
    }
    let first = !tracked().iter().any(ours);
    let mut in_mission = crate::engine::api::ingame::recent();
    let framed = if dialog { placement(h, r) } else { None };
    // Adopted (decision 230): cut and placed as above, then off screen, drawn by the UI.
    let adopted = match framed {
        Some(p) if crate::engine::fodialogs::ready() => {
            let d = device_of(h);
            let (cw, ch) = client_size(h);
            let (dw, dh) = design();
            let (ox, oy) = (((cw - dw) / 2).max(0), ((ch - dh) / 2).max(0));
            hide(h);
            crate::engine::fodialogs::adopt(h);
            let mut dr = WinRect::default();
            unsafe { GetWindowRect(d, &mut dr) };
            crate::log!(
                "forms: #32770 {:p} moved off screen; device window at {},{} {}x{}, active {:p}, focus {:p}",
                h, dr.left, dr.top, dr.right - dr.left, dr.bottom - dr.top, unsafe { GetActiveWindow() }, unsafe { GetFocus() }
            );
            // A screen of the front end (the campaign selection, the mission list, Game
            // Setup) is never over a mission, even straight after one: a mission won hands
            // over to the campaign selection with the mission's frames still recent, and it
            // was drawn over the mission's last frame, HUD and all, without the scene
            // (decision 316).
            let screen = dialog_screen(crate::engine::fodialogs::stock_proc_of(h));
            if in_mission && FRONT_END_SCREENS.contains(&screen) {
                in_mission = false;
                crate::log!("forms: #32770 {:p} is the front end's {}: not over the mission that just ended", h, screen);
            }
            let (mut ax, mut ay) = (p[0] - ox, p[1] - oy);
            // A dialog the executable opens at the device window's origin and never
            // moves (the editor's licence, `LicenseAgreementWindowDlgProc`) would stand
            // across the design space's top left corner, cut by the frame: one that does
            // not fit in the design space is centred in it, as a form over an adopted
            // dialog is above (PLAN-FLEETOPS.md, decision 503).
            if dw > 0 && dh > 0 && p[2] <= dw && p[3] <= dh && (ax < 0 || ay < 0 || ax + p[2] > dw || ay + p[3] > dh) {
                crate::log!("forms: #32770 {:p} would stand at {},{} {}x{} across the design space's edge: centred in it", h, ax, ay, p[2], p[3]);
                (ax, ay) = ((dw - p[2]) / 2, (dh - p[3]) / 2);
            }
            Some([ax, ay, p[2], p[3]])
        }
        Some(p) => {
            put(h, p);
            None
        }
        _ => None,
    };
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
        framed: if adopted.is_some() { None } else { framed },
        adopted,
        orig: (r.left, r.top),
        visit,
        pressed: false,
        combo: false,
    });
    crate::log!(
        "forms: {} {:p} up at {},{} {}x{}{}",
        class, h, origin.0, origin.1, r.right - r.left, r.bottom - r.top, if in_mission { ", over a mission" } else { "" }
    );
    if let Some(f) = adopted {
        crate::log!("forms: #32770 {:p} adopted at {},{} {}x{} in the design space", h, f[0], f[1], f[2], f[3]);
    } else if let Some(f) = framed {
        crate::log!("forms: #32770 {:p} framed at {},{} {}x{} on the device window", h, f[0], f[1], f[2], f[3]);
    }
    if first && (!dialog || framed.is_some()) {
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
        if let Some(d) = inside_host(w) {
            return d;
        }
        w = unsafe { GetWindow(w, GW_OWNER) };
    }
    owner
}

/// Display8's one window (`[Window] OneWindow`, PLAN-FLEETOPS.md decision 314): the device
/// window is a child of Display8's host, so a form's owner (a window's owner is always a
/// top-level one) is the host: the device window is the `TMainWindowForm` inside it.
fn inside_host(w: HWND) -> Option<HWND> {
    if class_of(w) != "Display8Host" {
        return None;
    }
    let d = unsafe { FindWindowExA(w, core::ptr::null_mut(), c"TMainWindowForm".as_ptr() as *const u8, core::ptr::null()) };
    (!d.is_null()).then_some(d)
}

fn destroyed(h: HWND) {
    let v = tracked();
    let Some(i) = v.iter().position(|t| t.hwnd == h) else { return };
    let t = v.remove(i);
    if t.adopted.is_some() {
        crate::engine::fodialogs::forget(h);
    }
    crate::log!("forms: {} {:p} closed after {:.1} s ({} frames of ours so far)", t.class, h, crate::rt::now() - t.opened, unsafe { FRAMES });
    if !v.iter().any(ours) {
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
        (*core::ptr::addr_of_mut!(DIALOGS)).clear();
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

/// A Delphi wide string (UTF-16, the length in bytes in the dword before): what a grid's
/// string cells hold.
fn wide_string(p: usize) -> String {
    if p < 4 || !super::readable(p - 4, 4) {
        return String::new();
    }
    let n = (rd::<u32>(p - 4) as usize / 2).min(256);
    if !super::readable(p, n * 2) {
        return String::new();
    }
    let w = unsafe { core::slice::from_raw_parts(p as *const u16, n) };
    char::decode_utf16(w.iter().copied()).map(|c| c.unwrap_or('?')).collect()
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
                // A windowed button keeps its caption in its window, not the VCL's field.
                Kind::Other if text.is_empty() && derives(vmt, "TButtonControl") => text = window_text(hwnd as HWND),
                Kind::Combo => (items, selected) = combo_items(hwnd as HWND),
                _ => {}
            }
        }
        let (checked, value) = state_of(c, vmt, &class);
        let grid = if class == "TNextGrid" { grid_of(c, vmt, &class) } else { None };
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
            checked,
            value,
            grid,
        });
    }
    out
}

/// A grid's cells, read from its memory (read only, decision 260): the grid's cells
/// object holds one list of cells for each column, each cell its value. Only a string
/// cell's text and an integer cell's number are read, each cell by its class's name; every
/// pointer is checked and any shape that is not the expected one ends the read with `None`
/// (the form's picture then stays).
fn grid_of(g: usize, vmt: usize, class: &str) -> Option<Grid> {
    let cols = rd::<usize>(g + field::FORMS_GRID_COLUMNS());
    if !ptr_ok(cols, field::FORMS_COLUMNS_LIST() + 4) {
        return None;
    }
    let list = rd::<usize>(cols + field::FORMS_COLUMNS_LIST());
    if !ptr_ok(list, field::FORMS_LIST_COUNT() + 4) {
        return None;
    }
    let ncols = rd::<i32>(list + field::FORMS_LIST_COUNT());
    let cells = rd::<usize>(g + field::FORMS_GRID_CELLS());
    if !(1..=32).contains(&ncols) || !ptr_ok(cells, field::FORMS_CELLS_ROWS() + 4) {
        return None;
    }
    let nrows = rd::<i32>(cells + field::FORMS_CELLS_ROWS());
    if !(0..=4096).contains(&nrows) || !ptr_ok(cells + field::FORMS_CELLS_LISTS(), ncols as usize * 4) {
        return None;
    }
    let mut lists = Vec::new();
    for c in 0..ncols as usize {
        let l = rd::<usize>(cells + field::FORMS_CELLS_LISTS() + c * 4);
        if !ptr_ok(l, field::FORMS_LIST_COUNT() + 4) {
            return None;
        }
        let (items, n) = (rd::<usize>(l + field::FORMS_LIST_ITEMS()), rd::<i32>(l + field::FORMS_LIST_COUNT()));
        if n < nrows || (nrows > 0 && !ptr_ok(items, nrows as usize * 4)) {
            return None;
        }
        lists.push(items);
    }
    let mut rows = Vec::with_capacity(nrows as usize);
    for r in 0..nrows as usize {
        let mut row = Vec::with_capacity(lists.len());
        for items in &lists {
            let cell = rd::<usize>(items + r * 4);
            let mut text = String::new();
            if ptr_ok(cell, field::FORMS_CELL_VALUE() + 4) {
                match class_name(rd::<usize>(cell)).as_deref() {
                    Some("TStringCell") => text = wide_string(rd::<usize>(cell + field::FORMS_CELL_VALUE())),
                    Some("TIntegerCell") => text = crate::sfmt!("{}", rd::<i32>(cell + field::FORMS_CELL_VALUE())),
                    _ => {}
                }
            }
            row.push(text);
        }
        rows.push(row);
    }
    static mut LAST: (usize, i32) = (0, -1);
    unsafe {
        if LAST != (g, nrows) {
            LAST = (g, nrows);
            crate::log!("forms: grid {:#x} read: {} rows of {} columns, first row {:?}", g, nrows, ncols, rows.first());
        }
    }
    let props = forms_props::published(vmt, in_fo);
    let logged = unsafe { &mut *core::ptr::addr_of_mut!(PROPS_LOGGED) };
    if !logged.iter().any(|n| n == class) {
        logged.push(String::from(class));
        let list: Vec<String> = props
            .iter()
            .map(|p| match p.getter {
                forms_props::Getter::Field(o) => crate::sfmt!("{}@{:#x}", p.name, o),
                forms_props::Getter::Method => crate::sfmt!("{}()", p.name),
            })
            .collect();
        crate::log!("forms: published properties of {}: {}", class, list.join(" "));
    }
    Some(Grid {
        rows,
        selected: rd::<i32>(g + field::FORMS_GRID_SELECTED()),
        first: rd::<i32>(g + field::FORMS_GRID_FIRST()),
        row_h: forms_props::field_of(g, &props, "RowSize", 4).unwrap_or(0),
        head_h: forms_props::field_of(g, &props, "HeaderSize", 4).unwrap_or(0),
    })
}

static mut PROPS_LOGGED: Vec<String> = Vec::new();

/// A check box's and a slider's state, from their published properties (read only,
/// `forms_props.rs`); the class's table is logged once.
fn state_of(c: usize, vmt: usize, class: &str) -> (Option<bool>, Option<(i32, i32, i32, i32)>) {
    let check = derives(vmt, "TJvgCheckBox");
    let slider = derives(vmt, "TJvxSlider");
    if !check && !slider {
        return (None, None);
    }
    let props = forms_props::published(vmt, in_fo);
    let logged = unsafe { &mut *core::ptr::addr_of_mut!(PROPS_LOGGED) };
    if !logged.iter().any(|n| n == class) {
        logged.push(String::from(class));
        let list: Vec<String> = props
            .iter()
            .map(|p| match p.getter {
                forms_props::Getter::Field(o) => crate::sfmt!("{}@{:#x}", p.name, o),
                forms_props::Getter::Method => crate::sfmt!("{}()", p.name),
            })
            .collect();
        crate::log!("forms: published properties of {}: {}", class, list.join(" "));
    }
    let checked = if check { forms_props::field_of(c, &props, "Checked", 1).map(|v| v != 0) } else { None };
    let value = if slider {
        let v = forms_props::field_of(c, &props, "Value", 4);
        let lo = forms_props::field_of(c, &props, "MinValue", 4);
        let hi = forms_props::field_of(c, &props, "MaxValue", 4);
        let inc = forms_props::field_of(c, &props, "Increment", 4).unwrap_or(1).max(1);
        match (v, lo, hi) {
            (Some(v), Some(lo), Some(hi)) => Some((v, lo, hi, inc)),
            _ => None,
        }
    } else {
        None
    };
    (checked, value)
}

fn log_controls(t: &Tracked, v: &[Control]) {
    crate::log!("forms: {} has {} controls:", t.class, v.len());
    for c in v {
        crate::log!(
            "forms:   {:?} {} {} {},{} {}x{} {}{} text {:?}{}{}{}",
            c.kind, c.class, c.name, c.rect[0], c.rect[1], c.rect[2], c.rect[3],
            if c.visible { "visible" } else { "hidden" }, if c.enabled { "" } else { " disabled" }, c.text,
            if c.items.is_empty() { String::new() } else { crate::sfmt!(" ({} items, {} chosen)", c.items.len(), c.selected) },
            match (c.checked, c.value) {
                (Some(k), _) => crate::sfmt!(" checked {}", k),
                (_, Some((v, lo, hi, inc))) => crate::sfmt!(" value {} of {}..{} step {}", v, lo, hi, inc),
                _ => String::new(),
            },
            if c.hwnd != 0 { " windowed" } else { "" }
        );
    }
}

/// A check box's, a slider's or a combo's state that changed since the last read: logged,
/// for the bench to check what a press of ours did (`forms: <name> of <class> now ...`).
fn log_changes(t: &Tracked, now: &[Control]) {
    if t.last.is_empty() {
        return;
    }
    for c in now {
        let Some(o) = t.last.iter().find(|o| o.name == c.name) else { continue };
        if c.checked != o.checked {
            crate::log!("forms: {} of {} now {}", c.name, t.class, if c.checked == Some(true) { "checked" } else { "unchecked" });
        }
        if c.value != o.value {
            crate::log!("forms: {} of {} now at {}", c.name, t.class, c.value.map_or(-1, |v| v.0));
        }
        if c.selected != o.selected {
            crate::log!("forms: {} of {} now chose {}", c.name, t.class, c.selected);
        }
    }
}

// ---- the pump (decision 144) ------------------------------------------------------------

unsafe extern "system" fn tick(_h: HWND, _m: u32, _id: usize, _t: u32) {
    unsafe { TICKS += 1 };
    let now = crate::rt::now();
    restore_unactivated();
    let v = tracked();
    v.retain(|t| unsafe { IsWindow(t.hwnd) } != 0);
    if !v.iter().any(ours) {
        stop();
        return;
    }
    let top = v.len() - 1;
    if v[top].class == "#32770" {
        // One of the nine dialogs FleetOpsHook opens through its own import: Roots' own
        // until phase two (decision 150). A framed one (decision 200) stands at its place
        // over our frame, which goes on under it; any other leaves our last frame.
        unsafe { DIALOG_TICKS += 1 };
        if v[top].adopted.is_some() {
            // Adopted (decision 230): off screen, painted into our surface, drawn by the
            // frame from its picture, driven by what the frame queued.
            let h = v[top].hwnd;
            // Its panes too: the Admiral's Log shows one and hides the others as its tabs
            // are pressed, and a pane shown is put back where it was.
            for t in v.iter().filter(|t| t.adopted.is_some()) {
                keep_off(t.hwnd);
            }
            // A dialog with no control to take it leaves the keyboard focus nowhere, and
            // keys then come as system keys that no dialog sees (MENUS.md, "A focus left
            // nowhere"): the focus goes to the dialog (decision 236).
            if unsafe { GetFocus() }.is_null() && unsafe { GetActiveWindow() } == h {
                unsafe { SetFocus(h) };
                crate::log!("forms: #32770 {:p} had no keyboard focus: given the focus", h);
            }
            poll_button(h);
            run_actions(h, 0);
            crate::engine::fodialogs::tick(PRINT_EVERY, h);
            unsafe { SetCursor(LoadCursorA(core::ptr::null_mut(), IDC_ARROW)) };
            publish(v, None);
            frame();
            return;
        }
        if v[top].framed.is_none() {
            return;
        }
        keep_framed(&v[top]);
        publish(v, None);
        frame();
        return;
    }
    run_actions(v[top].hwnd, v[top].obj);
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
        crate::log!(
            "forms: {} is ours: moved off screen, active {:p}, focus {:p}, owner {}",
            t.class, unsafe { GetActiveWindow() }, unsafe { GetFocus() }, class_of(unsafe { GetWindow(t.hwnd, GW_OWNER) })
        );
    }
    keep_off(t.hwnd);
    poll_button(t.hwnd);
    if t.class == "TYesNoForm" {
        poll_keys(t.hwnd);
    }
    log_changes(t, &ctl);
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
    publish(v, Some(top));
    frame();
}

/// What the frame draws: the forms that are ours, outermost first, only `live` live
/// (None: a dialog is on top and every form under it is inert); the framed dialogs.
fn publish(v: &[Tracked], live: Option<usize>) {
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
            caption: window_text(t.hwnd),
            top: Some(i) == live,
            in_mission: t.in_mission,
            z: 2 * i,
        })
        .collect();
    let n = v.len();
    // A pane (a pop-up owned by an adopted dialog: the Admiral's Log's eight) opens before
    // its owner activates; it is drawn just above its owner, and takes the pointer while
    // its owner is the top one (decision 235).
    let owner_of = |t: &Tracked| -> Option<usize> {
        let o = unsafe { GetWindow(t.hwnd, GW_OWNER) };
        v.iter().position(|u| u.hwnd == o && u.adopted.is_some())
    };
    let dialogs: Vec<Dialog> = v
        .iter()
        .enumerate()
        .filter(|(_, t)| t.framed.is_some() || t.adopted.is_some())
        .map(|(i, t)| {
            let pane = owner_of(t).filter(|&j| j > i);
            // A pane stands where it stood against its owner before either was moved.
            let design = match (pane, t.adopted) {
                (Some(j), Some(a)) => v[j].adopted.map(|o| [o[0] + t.orig.0 - v[j].orig.0, o[1] + t.orig.1 - v[j].orig.1, a[2], a[3]]),
                _ => t.adopted,
            };
            Dialog {
                hwnd: t.hwnd as usize,
                design,
                framed: t.framed.unwrap_or_default(),
                client: client_size(t.hwnd),
                top: i + 1 == n || pane.is_some_and(|j| j + 1 == n),
                children: if t.adopted.is_some() { crate::engine::fodialogs::children(t.hwnd) } else { Vec::new() },
                stock_proc: crate::engine::fodialogs::stock_proc(t.hwnd),
                z: pane.map_or(2 * i, |j| 2 * j + 1),
                pane: pane.is_some(),
                visible: unsafe { IsWindowVisible(t.hwnd) } != 0,
            }
        })
        .collect();
    unsafe {
        *core::ptr::addr_of_mut!(VIEW) = view;
        *core::ptr::addr_of_mut!(DIALOGS) = dialogs;
    }
}

// ---- the dialogs over the device window (decision 200) ------------------------------------

/// Whether `w` is the device window (FleetOpsHook's `TMainWindowForm`) while the forms'
/// frame is drawn over it: nothing drawn on it through GDI is ours to show then
/// (`fodialogs`, the shell background; decision 300).
pub fn covers(w: HWND) -> bool {
    if w.is_null() || class_of(w) != "TMainWindowForm" {
        return false;
    }
    // Decision 312: also while a dialog the forms will take is being made over this window
    // and is not yet adopted (the forms' frame is not up between the in-mission menu's end
    // and Single Player's adoption, and FleetOpsHook draws Single Player's background onto
    // the window in that time, which stood on the screen until our next frame).
    let hosting = unsafe { HOSTING };
    let pending = unsafe { &*core::ptr::addr_of!(SUBCLASSED) };
    hosting || pending.iter().any(|&(h, _)| unsafe { IsWindow(h) } != 0 && class_of(h) == "#32770" && device_of(h) == w)
}

/// The device window a window is owned by (through its owners), or null.
fn device_of(h: HWND) -> HWND {
    let mut w = unsafe { GetWindow(h, GW_OWNER) };
    for _ in 0..6 {
        if w.is_null() || class_of(w) == "TMainWindowForm" {
            return w;
        }
        if let Some(d) = inside_host(w) {
            return d;
        }
        w = unsafe { GetWindow(w, GW_OWNER) };
    }
    core::ptr::null_mut()
}

/// The device window's client size, for a window over it.
fn client_size(h: HWND) -> (i32, i32) {
    let d = device_of(h);
    let mut c = WinRect::default();
    if !d.is_null() {
        unsafe { GetClientRect(d, &mut c) };
    }
    (c.right - c.left, c.bottom - c.top)
}

/// A window's rectangle in its device window's client coordinates.
fn on_device(h: HWND, d: HWND) -> [i32; 4] {
    let mut r = WinRect::default();
    unsafe { GetWindowRect(h, &mut r) };
    rect_on_device(r, d)
}

/// A rectangle in screen coordinates, in the device window's client coordinates.
fn rect_on_device(r: WinRect, d: HWND) -> [i32; 4] {
    let mut o = Point::default();
    unsafe { ClientToScreen(d, &mut o) };
    [r.left - o.x, r.top - o.y, r.right - r.left, r.bottom - r.top]
}

/// Where a dialog over the device window stands. The shell sizes a full-screen screen to
/// its owner's client area, which on Display8 is the desktop's size, while Roots paints the
/// screen's design size (1200x900) at its top left and leaves the rest unpainted (white):
/// such a dialog is cut to the design size and centred. A smaller one stays where the
/// shell put it, unless that is off the client area (its owner a form of ours, moved off
/// screen): then it is centred. None: not over the device window, left alone.
fn placement(h: HWND, r: WinRect) -> Option<[i32; 4]> {
    let d = device_of(h);
    if d.is_null() {
        return None;
    }
    let (cw, ch) = client_size(h);
    let [x, y, w, ht] = rect_on_device(r, d);
    let (dw, dh) = design();
    if dw > 0 && dh > 0 && w >= dw && ht >= dh {
        return Some([((cw - dw) / 2).max(0), ((ch - dh) / 2).max(0), dw, dh]);
    }
    if x >= 0 && y >= 0 && x + w <= cw && y + ht <= ch {
        Some([x, y, w, ht])
    } else {
        Some([((cw - w) / 2).max(0), ((ch - ht) / 2).max(0), w, ht])
    }
}

fn put(h: HWND, p: [i32; 4]) {
    let mut o = Point::default();
    unsafe {
        ClientToScreen(device_of(h), &mut o);
        SetWindowPos(h, core::ptr::null_mut(), o.x + p[0], o.y + p[1], p[2], p[3], SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// The shell or FleetOpsHook may place the dialog again (FleetOpsHook re-places the
/// windows it registered when the device window moves): back where the frame has it.
fn keep_framed(t: &Tracked) {
    let Some(p) = t.framed else { return };
    let d = device_of(t.hwnd);
    if d.is_null() {
        return;
    }
    let now = on_device(t.hwnd, d);
    if now != p {
        crate::log!("forms: #32770 {:p} was placed at {},{} {}x{}: framed again", t.hwnd, now[0], now[1], now[2], now[3]);
        put(t.hwnd, p);
    }
}

static mut BUTTON: bool = false;

/// The real pointer falls on the device window, whose procedure hands its mouse to the
/// engine's message functions, where the UI records it; under a form that leaves the
/// device window disabled (Yes/No, Manual IP) no click arrives at all, so the pump reads
/// the left button itself and records its changes (decision 183).
fn poll_button(top: HWND) {
    let down = unsafe { GetAsyncKeyState(1) } as u16 & 0x8000 != 0;
    let was = unsafe { core::mem::replace(&mut BUTTON, down) };
    let dev = device_under(top);
    if dev.is_null() || unsafe { IsWindowEnabled(dev) } != 0 || down == was {
        return;
    }
    crate::input::note_message(if down { WM_LBUTTONDOWN } else { WM_LBUTTONUP }, if down { MK_LBUTTON } else { 0 });
}

/// The device window under the top form: its owner's, or, for a form whose owner is not
/// the device window (the VCL's message box, owned by the library's own window over the map
/// editor's Save Map form), the first form under it that has one.
fn device_under(top: HWND) -> HWND {
    let mut owners = alloc::vec![unsafe { GetWindow(top, GW_OWNER) }];
    owners.extend(tracked().iter().rev().map(|t| unsafe { GetWindow(t.hwnd, GW_OWNER) }));
    for o in owners {
        let d = device_window(o);
        if !d.is_null() && class_of(d) == "TMainWindowForm" {
            return d;
        }
    }
    core::ptr::null_mut()
}

/// The keys of a prompt, Enter and Escape: the form holds the keyboard focus, so a key goes
/// to its window and never to the device window's procedure, where the UI records keys. The
/// pump reads the two keys itself while the form is the active window of this thread and
/// records their downs (decision 541), as it does the left button.
static mut KEYS_WAS: [bool; 2] = [false; 2];

fn poll_keys(top: HWND) {
    const KEYS: [usize; 2] = [0x0d, 0x1b];
    let active = unsafe { GetActiveWindow() } == top;
    for (i, vk) in KEYS.iter().enumerate() {
        let down = active && unsafe { GetAsyncKeyState(*vk as i32) } as u16 & 0x8000 != 0;
        let was = unsafe { core::mem::replace(&mut (*core::ptr::addr_of_mut!(KEYS_WAS))[i], down) };
        if down && !was {
            crate::input::note_message(WM_KEYDOWN, *vk);
        }
    }
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
fn run_actions(top: HWND, _obj: usize) {
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
            Action::Mouse(h, msg, keys, x, y) if h == top as usize || pane_of(h as HWND, top) => {
                // To the deepest visible window at the point, in its own client coordinates.
                let mut target = h as HWND;
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
            Action::Poke(h, msg, keys, x, y) if h == top as usize => {
                // The form's window put where `x, y` lies under the real pointer.
                let mut cp = Point::default();
                unsafe { GetCursorPos(&mut cp) };
                unsafe { SetWindowPos(top, core::ptr::null_mut(), cp.x - x, cp.y - y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
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
                unsafe { SendMessageA(target, msg, keys, lparam(p.x, p.y)) };
                unsafe { SetWindowPos(top, core::ptr::null_mut(), OFF, OFF, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
            }
            Action::GridKey(h, vk) => unsafe {
                let mut p = h as HWND;
                for _ in 0..6 {
                    if p == top || p.is_null() {
                        break;
                    }
                    p = GetParent(p);
                }
                if IsWindow(h as HWND) != 0 && p == top {
                    SendMessageA(h as HWND, 0x100, vk as usize, 1);
                    SendMessageA(h as HWND, 0x101, vk as usize, 0xc000_0001u32 as isize);
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
            Action::Key(h, msg, wp) => unsafe {
                if IsWindow(h as HWND) != 0 {
                    PostMessageA(h as HWND, msg, wp, 1);
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
        // A grid paints its cells through its own window's device context, which a print
        // of the form does not reach (nor a print of the grid's window): the UI reads the
        // cells instead (`grid_of`).
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
    // render3d's menu scene under everything of ours, as the dialog host's frame has it.
    crate::menus::scene::draw(bb.width, bb.height);
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
