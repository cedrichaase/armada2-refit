//! Screen specs for the dialogs FleetOpsHook opens on Roots (PLAN-FLEETOPS.md, FO-6 step 7,
//! decisions 250 onwards; MENUS.md, "Roots' dialogs, laid out"). Each one is adopted
//! (`engine::fodialogs`, decision 230): it runs hidden off screen in user32's modal loop,
//! FleetOpsHook's procedure in front of the stock one, and its painting is captured. A
//! spec here draws it our way, as Armada II's specs (`menus::screens`) draw the hosted
//! dialogs, from the same two things:
//!
//! - **its model** (`model::read_adopted`): the shell's own widgets that draw into it
//!   (buttons, labels, rows, read through `engine::shellui`, whose reader the forms install
//!   on Roots) and its child windows, in Roots' design space (1200x900);
//! - **its picture** (the mirror), for what a theme cannot draw (a campaign's art).
//!
//! A spec acts only as a player would (`Screen::click`): the pointer is posted to the
//! dialog at the stock control's point, a move first and the press a moment later, since
//! Roots' screens take a click only on a control already under the pointer (decision
//! 190); nothing writes a setting. The spec is chosen by the dialog's stock procedure
//! (`forms::dialog_screen`), and a dialog with none, or one a spec does not recognise
//! (it returns false), stays the mirror (`menus::roots`).
use crate::engine::api::forms::{self, Action, Dialog};
use crate::engine::api::shellui::Kind;
use crate::gfx::{Tex, R};
use crate::menus::model::{self, Control, Model, What};
use crate::rt::win::HWND;
use crate::widgets::Ui;
use alloc::vec::Vec;

pub mod admirals_log;
pub mod game_settings;
pub mod game_setup;
pub mod mission_select;
pub mod notice;
pub mod saves;
pub mod single_player;

/// Where Roots' design space lands on the back buffer (`menus::roots`' fit).
#[derive(Clone, Copy)]
pub struct Fit {
    pub x: f32,
    pub y: f32,
    pub s: f32,
}

impl Fit {
    /// A design rectangle (x, y, w, h) on the screen.
    pub fn rect(&self, r: [i32; 4]) -> R {
        R::new(self.x + r[0] as f32 * self.s, self.y + r[1] as f32 * self.s, r[2] as f32 * self.s, r[3] as f32 * self.s)
    }
}

/// What a spec draws an adopted dialog from.
pub struct Screen<'a> {
    pub d: &'a Dialog,
    pub model: &'a Model,
    pub fit: Fit,
    /// The dialog's picture as it paints it (`menus::roots`' texture).
    pub picture: Option<Tex>,
    /// It takes input (the top dialog).
    pub top: bool,
}

impl Screen<'_> {
    pub fn hwnd(&self) -> usize {
        self.d.hwnd
    }
    /// The shell buttons that show, in the order the screen made them.
    pub fn buttons(&self) -> Vec<&Control> {
        self.model.controls.iter().filter(|c| matches!(c.what, What::Shell(k) if k != Kind::Label) && c.visible).collect()
    }
    /// The labels, hidden ones too, in the order the screen made them.
    pub fn labels(&self) -> Vec<&Control> {
        self.model.controls.iter().filter(|c| matches!(c.what, What::Shell(Kind::Label))).collect()
    }
    /// Draws the part `r` (design coordinates) of the dialog's picture into `dst`.
    pub fn mirror_part(&self, ui: &mut Ui, r: [i32; 4], dst: R, tint: u32) {
        let Some(t) = self.picture else { return };
        let [ox, oy, w, h] = self.model.rect;
        if w <= 0 || h <= 0 {
            return;
        }
        let (u0, v0) = ((r[0] - ox) as f32 / w as f32, (r[1] - oy) as f32 / h as f32);
        ui.draw.image_uv(t, dst, [u0, v0, u0 + r[2] as f32 / w as f32, v0 + r[3] as f32 / h as f32], tint);
    }
    /// The pointer over the design point `x`, `y` of the dialog, as the player's would be
    /// (its hover picture and sound); None: over none of its controls.
    pub fn hover(&self, at: Option<(i32, i32)>) {
        // A click under way keeps the pointer where it is.
        if unsafe { (*core::ptr::addr_of!(CLICKS)).iter().any(|c| c.0 == self.d.hwnd) || ACTIVE.is_some_and(|c| c.0 == self.d.hwnd) } {
            return;
        }
        let (x, y) = match at {
            Some((x, y)) => (x - self.model.rect[0], y - self.model.rect[1]),
            None => (2, 2),
        };
        forms::act(Action::Mouse(self.d.hwnd, forms::MOUSE_MOVE, 0, x, y));
    }
    /// A click on the design point `x`, `y`: the pointer moved there now, the press and
    /// the release a moment later (decision 190).
    pub fn click(&self, x: i32, y: i32) {
        let (cx, cy) = (x - self.model.rect[0], y - self.model.rect[1]);
        unsafe { (*core::ptr::addr_of_mut!(CLICKS)).push((self.d.hwnd, cx, cy)) };
        flush(self.d.hwnd);
        crate::log!("menus: roots dialog {:#x} ({}): click at {},{}", self.d.hwnd, self.model.screen, x, y);
    }
    /// A click on the middle of a control.
    pub fn click_control(&self, c: &Control) {
        self.click(c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2);
    }
}

/// The shared layouts (`screens::setup_kit`, Game Setup) act on an adopted dialog through
/// this: a click is `Screen::click` (the pointer resting first), typing is posted to the
/// edit as the keyboard would send it, Enter too (the dialog's modal loop, user32's, makes
/// it the dialog's OK, decision 253), a combo box's choice is its selection and its
/// parent's notification.
impl crate::menus::Host for Screen<'_> {
    fn model(&self) -> &Model {
        self.model
    }
    fn top(&self) -> bool {
        self.top
    }
    fn click(&self, x: i32, y: i32) {
        Screen::click(self, x, y);
    }
    fn mirror_part(&self, ui: &mut Ui, r: [i32; 4], dst: crate::gfx::R, tint: u32) {
        Screen::mirror_part(self, ui, r, dst, tint);
    }
    fn key(&self, h: HWND, msg: u32, wp: usize) {
        forms::act(Action::Key(h as usize, msg, wp));
    }
    fn enter(&self, h: HWND) {
        forms::act(Action::Key(h as usize, crate::rt::win::WM_KEYDOWN, 0x0d));
    }
    fn select(&self, h: HWND, combo: bool, index: i32, _double: bool) {
        if combo {
            forms::act(Action::Select(h as usize, index));
        } else {
            crate::log!("menus: roots dialog {:#x}: a list's selection is not sent (no list on Roots' dialogs yet)", self.d.hwnd);
        }
    }
    fn full_screen(&self) -> bool {
        self.model.rect[2] >= forms::design().0
    }
}

/// Clicks waiting their turn, in order: (dialog, client point).
static mut CLICKS: Vec<(usize, i32, i32)> = Vec::new();
/// The click under way: (dialog, client point, when its last step was sent, step: 0 the
/// pointer moved there, 1 the button down).
static mut ACTIVE: Option<(usize, i32, i32, f64, u8)> = None;
/// How long the pointer rests on a control before the press (seconds).
const PRESS_AFTER: f64 = 0.12;
/// How long the button is held (seconds).
const RELEASE_AFTER: f64 = 0.06;

/// The clicks, one at a time, as a player's: the pointer moved to the point, once it has
/// rested there the button down, and a moment later up, each in a pass of its own through
/// the dialog's messages (decision 255: a press and its release sent together left Load
/// Game's question answered Yes without the load). A click for a dialog no longer on top
/// is dropped.
fn flush(top: usize) {
    let now = crate::rt::now();
    let q = unsafe { &mut *core::ptr::addr_of_mut!(CLICKS) };
    q.retain(|c| c.0 == top);
    if let Some((h, x, y, at, step)) = unsafe { ACTIVE } {
        if h != top {
            unsafe { ACTIVE = None };
        } else if step == 0 && now - at >= PRESS_AFTER {
            forms::act(Action::Mouse(h, forms::MOUSE_MOVE, 0, x, y));
            forms::act(Action::Mouse(h, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
            if crate::config::get().menus_trace {
                crate::log!("menus: trace: roots dialog {:#x}: pressed at {},{} (client)", h, x, y);
            }
            unsafe { ACTIVE = Some((h, x, y, now, 1)) };
        } else if step == 1 && now - at >= RELEASE_AFTER {
            forms::act(Action::Mouse(h, forms::MOUSE_UP, 0, x, y));
            unsafe { ACTIVE = None };
        }
        return;
    }
    if !q.is_empty() {
        let (h, x, y) = q.remove(0);
        forms::act(Action::Mouse(h, forms::MOUSE_MOVE, 0, x, y));
        unsafe { ACTIVE = Some((h, x, y, now, 0)) };
    }
}

/// The dialogs whose model was logged, and how many visible controls it had then.
static mut LOGGED: Vec<(usize, usize)> = Vec::new();

fn log_model(m: &Model, hwnd: usize) {
    let logged = unsafe { &mut *core::ptr::addr_of_mut!(LOGGED) };
    logged.retain(|x| forms::dialogs().iter().any(|d| d.hwnd == x.0));
    let shown = m.controls.iter().filter(|c| c.visible).count();
    match logged.iter_mut().find(|x| x.0 == hwnd) {
        Some(x) if x.1 == shown => {}
        Some(x) => {
            x.1 = shown;
            model::log(m);
        }
        None => {
            logged.push((hwnd, shown));
            model::log(m);
            crate::engine::api::shellui::log_unread(hwnd as HWND);
        }
    }
}

type Draw = fn(&mut Ui, &Screen) -> bool;

fn spec(screen: &str) -> Option<Draw> {
    Some(match screen {
        "single player" => single_player::draw,
        "mission select" => mission_select::draw,
        "load game" => saves::draw_load,
        "save game" => saves::draw_save,
        "message" => notice::draw,
        "game settings" => game_settings::draw,
        "game setup" => game_setup::draw,
        "admiral's log" => admirals_log::draw,
        _ => return None,
    })
}

/// An adopted dialog drawn by its spec: false when it has none (or the spec does not
/// recognise it this frame), and the frame then draws its picture (`menus::roots`).
pub fn draw(ui: &mut Ui, d: &Dialog, fit: Fit, picture: Option<Tex>) -> bool {
    let Some(design) = d.design else { return false };
    if d.pane {
        // The Admiral's Log's panes are drawn by the log's spec, in its page (decision 259).
        let owner = forms::dialogs().into_iter().find(|o| !o.pane && o.z + 1 == d.z);
        return crate::config::get().menus_specs && owner.is_some_and(|o| forms::dialog_screen(o.stock_proc) == "admiral's log");
    }
    let name = forms::dialog_screen(d.stock_proc);
    let m = model::read_adopted(d.hwnd as HWND, design, name);
    log_model(&m, d.hwnd);
    if !crate::config::get().menus_specs {
        return false;
    }
    let Some(f) = spec(name) else { return false };
    if d.top {
        flush(d.hwnd);
    } else {
        // Something opened over it (Load's question, after the stock row's double click):
        // the clicks still waiting are a player's that the question took, not this
        // dialog's; sent after it, one more Load undid the answer (decision 255).
        unsafe {
            if ACTIVE.is_some_and(|c| c.0 == d.hwnd) {
                ACTIVE = None;
            }
            (*core::ptr::addr_of_mut!(CLICKS)).retain(|c| c.0 != d.hwnd);
        }
    }
    let s = Screen { d, model: &m, fit, picture, top: d.top };
    let drawn = ui.scope(&crate::sfmt!("rdlg{}", d.hwnd), |ui| ui.inert_unless(d.top, |ui| f(ui, &s)));
    // The bench waits on this line (once per dialog and outcome).
    unsafe {
        let said = &mut *core::ptr::addr_of_mut!(SAID);
        if !said.contains(&(d.hwnd, drawn)) {
            said.retain(|x| forms::dialogs().iter().any(|o| o.hwnd == x.0));
            said.push((d.hwnd, drawn));
            crate::log!("menus: roots dialog {:#x}: {} {}", d.hwnd, name, if drawn { "drawn by its spec" } else { "not recognised by its spec: the picture" });
        }
    }
    drawn
}

/// The dialogs whose spec's outcome was logged.
static mut SAID: Vec<(usize, bool)> = Vec::new();

/// The screen of the full-screen adopted dialog on top of what shows ("" for none): the
/// menu scene chooses its view by it (`menus::scene`).
pub fn full_screen() -> &'static str {
    let (dw, dh) = forms::design();
    forms::dialogs()
        .iter()
        .filter(|d| d.visible && !d.pane && d.design.is_some_and(|p| p[2] >= dw && p[3] >= dh))
        .max_by_key(|d| d.z)
        .map_or("", |d| forms::dialog_screen(d.stock_proc))
}

/// Whether the campaign selection stands over the scene with its spec (the frame then
/// leaves the scene undimmed, as under the main menu).
pub fn campaigns_over_scene() -> bool {
    crate::config::get().menus_specs && crate::menus::scene::shown() && full_screen() == "single player"
}
