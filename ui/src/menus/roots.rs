//! Roots' front-end forms drawn by the UI (PLAN-FLEETOPS.md, FO-6 step 7, option C;
//! MENUS.md, "Roots' forms"): each form FleetOpsHook shows is read from its live objects
//! (`engine::api::forms`), hidden, and drawn here in our style, in its design space (Roots'
//! 1200x900 shell) fitted to the back buffer.
//!
//! - **Specs**: the main menu (a column of our buttons in the form's order, its version
//!   line) and Yes/No (the form's own picture for its question, our two buttons).
//! - **Generic** (`[Menus] Generic`): `widgets` draws each control the UI knows as one of
//!   ours where the form has it (buttons, labels, text fields, combo boxes) and the rest
//!   (check boxes, sliders, grids) as crops of the form's own picture, with the mouse
//!   forwarded to them; `mirror` draws the whole picture and forwards the mouse.
//!
//! Every action goes back to Roots' form as a player's would (`forms::act`): a press of
//! ours is a posted click at the control's centre; nothing writes a setting.
mod graphics;
mod kit;
mod sound;
mod specs;

use crate::engine::api::forms::{self, Action, Control, Dialog, Form, Kind};
use crate::gfx::{self, Tex, R};
use crate::gfx::Weight;
use crate::widgets::{Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

/// Where the design space lands on the back buffer.
#[derive(Clone, Copy)]
struct Fit {
    x: f32,
    y: f32,
    s: f32,
}

impl Fit {
    fn rect(&self, f: &Form, r: [i32; 4]) -> R {
        R::new(
            self.x + (f.origin.0 + r[0]) as f32 * self.s,
            self.y + (f.origin.1 + r[1]) as f32 * self.s,
            r[2] as f32 * self.s,
            r[3] as f32 * self.s,
        )
    }
    fn form(&self, f: &Form) -> R {
        self.rect(f, [0, 0, f.size.0, f.size.1])
    }
    /// A screen point in the form's client coordinates.
    fn point(&self, f: &Form, p: (f32, f32)) -> (i32, i32) {
        (
            libm::floorf((p.0 - self.x) / self.s) as i32 - f.origin.0,
            libm::floorf((p.1 - self.y) / self.s) as i32 - f.origin.1,
        )
    }
}

/// The top form's picture as a texture, and the print it holds.
static mut PICTURE: Option<(Tex, i32, i32, u32)> = None;
/// The mouse held on a forwarded area: the form's window.
static mut HELD: usize = 0;
static mut LAST_FIT: (i32, i32, i32, i32) = (0, 0, 0, 0);

fn picture() -> Option<(Tex, i32, i32)> {
    let (bits, w, h, serial) = forms::picture()?;
    let slot = unsafe { &mut *core::ptr::addr_of_mut!(PICTURE) };
    let tex = match *slot {
        Some((t, pw, ph, s)) if pw == w && ph == h => {
            if s != serial {
                gfx::tex_update(t, bits, w as usize * 4, w as u32, h as u32);
                *slot = Some((t, w, h, serial));
            }
            t
        }
        _ => {
            let t = gfx::texture_dynamic(w as u32, h as u32);
            gfx::tex_update(t, bits, w as usize * 4, w as u32, h as u32);
            *slot = Some((t, w, h, serial));
            t
        }
    };
    Some((tex, w, h))
}

/// The screen's name in our title bar.
fn title(class: &str) -> &'static str {
    match class {
        "TMainForm" => "MAIN MENU",
        "TOptionsMenuForm" => "OPTIONS",
        "TYesNoForm" => "CONFIRM",
        "TManualIPForm" => "MANUAL IP",
        "TMultiplayerConnectionForm" => "MULTIPLAYER",
        "TMapSelectForm" => "SELECT MAP",
        "TGraphicOptionsForm" => "GRAPHICS",
        "TAdvancedOptionsForm" => "ADVANCED OPTIONS",
        "TSoundSettingsForm" => "SOUND",
        "TModSettingsForm" => "MODS",
        "TAdvSettingsForm" => "ADVANCED SETTINGS",
        _ => "MENU",
    }
}

pub fn frame(ui: &mut Ui) {
    let view = forms::view();
    let dialogs = forms::dialogs();
    if view.is_empty() && dialogs.is_empty() {
        return;
    }
    let (mut dw, mut dh) = forms::design();
    if dw <= 0 || dh <= 0 {
        let Some(f) = view.last() else { return };
        (dw, dh) = f.size;
    }
    let s = (ui.w / dw as f32).min(ui.h / dh as f32);
    let fit = Fit { x: libm::floorf((ui.w - dw as f32 * s) * 0.5), y: libm::floorf((ui.h - dh as f32 * s) * 0.5), s };
    let area = R::new(fit.x, fit.y, dw as f32 * s, dh as f32 * s);
    unsafe {
        let k = (area.x as i32, area.y as i32, area.w as i32, area.h as i32);
        if LAST_FIT != k {
            LAST_FIT = k;
            // The bench maps the design space to the screen with this line
            // (testbench/bench/scenario.py, design_to_screen; "at forms X,Y").
            crate::log!("menus: forms {}x{} @0,0 -> {}x{} @{},{} (the design space on the screen)", dw, dh, k.2, k.3, k.0, k.1);
        }
    }
    // Over the mission it interrupts (the in-mission menu), else the theme's backdrop:
    // render3d's menu scene, dimmed, when it drew under this frame.
    // The main menu over the scene shades only where its column stands, as Armada II's.
    let main_over_scene = crate::menus::scene::shown() && forms::main_menu_shown() && crate::config::get().menus_specs;
    if !main_over_scene && !crate::menus::backdrop::draw(ui) {
        let full = ui.screen();
        crate::widgets::theme::backdrop(ui, full);
    }
    if forms::probe() {
        ui.note(R::new(area.x + ui.u(8.0), area.bottom() - ui.u(28.0), area.w, ui.u(24.0)), "forms probe");
    }
    let cfg = crate::config::get();
    // Forms and dialogs in their order, outermost first; a full-screen one (a form or an
    // adopted dialog the design space's size) hides everything under it.
    let full_dialog = |d: &Dialog| d.design.is_some_and(|p| p[2] >= dw && p[3] >= dh);
    let first = view
        .iter()
        .filter(|f| f.size.0 >= dw && f.size.1 >= dh)
        .map(|f| f.z)
        .chain(dialogs.iter().filter(|d| d.visible && full_dialog(d)).map(|d| d.z))
        .max()
        .unwrap_or(0);
    let mut order: Vec<(usize, Option<&Form>, Option<&Dialog>)> = view
        .iter()
        .filter(|f| f.z >= first)
        .map(|f| (f.z, Some(f), None))
        .chain(dialogs.iter().filter(|d| d.visible && (d.z >= first || d.design.is_none())).map(|d| (d.z, None, Some(d))))
        .collect();
    order.sort_by_key(|o| o.0);
    // The pointer goes to the live dialog it is over, the uppermost (a pane over its
    // owner), or to the one that holds the button.
    let rect_of = |p: [i32; 4]| R::new(fit.x + p[0] as f32 * fit.s, fit.y + p[1] as f32 * fit.s, p[2] as f32 * fit.s, p[3] as f32 * fit.s);
    let held = unsafe { HELD };
    let target = if held != 0 && dialogs.iter().any(|d| d.hwnd == held && d.top) {
        held
    } else {
        let mut live: Vec<&Dialog> = dialogs.iter().filter(|d| d.top && d.visible && d.design.is_some()).collect();
        live.sort_by_key(|d| core::cmp::Reverse(d.z));
        match ui.input.mouse {
            Some(m) => live.iter().find(|d| rect_of(d.design.unwrap_or_default()).contains(m)).map_or(0, |d| d.hwnd),
            None => 0,
        }
    };
    for (_, f, d) in order {
        if let Some(f) = f {
            let live = f.top;
            ui.scope(&crate::sfmt!("form{}", f.hwnd), |ui| {
                ui.inert_unless(live, |ui| match (cfg.menus_specs, f.class.as_str()) {
                    (true, "TMainForm") => main_menu(ui, f),
                    (true, "TOptionsMenuForm") => column(ui, &fit, f, area, Some("buttonBack")),
                    (true, "TYesNoForm") => yes_no(ui, &fit, f),
                    (true, _) if specs::draw(ui, f) => {}
                    _ => generic(ui, &fit, f, cfg.menus_widgets, live),
                })
            });
        }
        let Some(d) = d else { continue };
        // An adopted dialog (decision 230) is drawn here from its picture, in the design
        // space like the forms, scaled to fit (decision 232), and takes the pointer while
        // it is the top one.
        if let Some(p) = d.design {
            let r = rect_of(p);
            if !d.pane {
                ui.panel(r.inset(-ui.u(6.0)));
            }
            if let Some(t) = dialog_texture(d) {
                ui.draw.image(t, r, 0xff_ff_ff_ff);
            }
            if d.hwnd == target {
                ui.scope(&crate::sfmt!("dialog{}", d.hwnd), |ui| forward_dialog(ui, d, r, fit.s));
            }
            continue;
        }
        // A framed one (decision 200: the executable's paint imports could not be ours)
        // stands over this frame as Roots draws it, cut to the design size and centred: a
        // panel of ours frames it, in the device window's client pixels.
        let (cw, ch) = d.client;
        if cw <= 0 || ch <= 0 {
            continue;
        }
        let r = d.framed;
        let (sx, sy) = (ui.w / cw as f32, ui.h / ch as f32);
        let dr = R::new(r[0] as f32 * sx, r[1] as f32 * sy, r[2] as f32 * sx, r[3] as f32 * sy);
        ui.panel(dr.inset(-ui.u(6.0)));
    }
}

/// Visible controls of a kind, top to bottom.
fn shown(f: &Form, k: Kind) -> Vec<&Control> {
    let mut v: Vec<&Control> = f.controls.iter().filter(|c| c.visible && c.kind == k && c.rect[2] > 0 && c.rect[3] > 0).collect();
    v.sort_by_key(|c| (c.rect[1], c.rect[0]));
    v
}

fn caption(c: &Control) -> &str {
    if c.text.is_empty() || c.text == "*" { c.name.as_str() } else { c.text.as_str() }
}

/// The main menu: Armada II's main menu over the scene (`screens::main_menu::scene_column`,
/// the same layout and look), titled for Roots, its entries the form's visible buttons in
/// the order Roots shows them and with the captions Roots set (decision 205); a press goes
/// to the button at its own rectangle, so two buttons whose names are swapped against
/// their actions still do what their captions say. Roots' version line stands at the
/// bottom right, as Armada II's does.
fn main_menu(ui: &mut Ui, f: &Form) {
    let buttons = shown(f, Kind::Button);
    let labels: Vec<String> = buttons.iter().map(|c| caption(c).to_uppercase()).collect();
    let entries: Vec<(&str, bool)> = labels.iter().zip(buttons.iter()).map(|(l, c)| (l.as_str(), f.top && c.enabled)).collect();
    let (clicked, _) = crate::menus::screens::main_menu::scene_column(ui, "FLEET OPERATIONS", "ROOTS", &entries);
    if let Some(i) = clicked {
        forms::press(f, buttons[i]);
    }
    if let Some(v) = f.control("labelVersion").filter(|c| c.visible && !c.text.is_empty()) {
        let t = ui.theme;
        let s = ui.screen();
        let m = ui.u(24.0);
        let r = R::new(s.w - m - ui.u(400.0), s.h - m - ui.u(24.0), ui.u(400.0), ui.u(24.0));
        ui.text_in(r, &v.text, t.small_size, Align::Right, t.text_dim, Weight::Regular);
    }
}

/// Options: our column of the form's visible buttons, in the form's
/// order, centred over the backdrop; Options' Back (bound by Roots when the form opens:
/// Previous Menu in the shell, the return to the mission in one) at the window's foot; the
/// main menu's version line at the bottom right. A press goes to Roots' own button, so the
/// action is the one Roots binds.
fn column(ui: &mut Ui, fit: &Fit, f: &Form, area: R, back: Option<&str>) {
    let mut buttons = shown(f, Kind::Button);
    let foot = back.and_then(|b| buttons.iter().position(|c| c.name == b)).map(|i| buttons.remove(i));
    let bw = ui.u(320.0).min(area.w * 0.4);
    let bh = ui.u(44.0);
    let gap = ui.u(12.0);
    let inner_h = buttons.len() as f32 * (bh + gap) - gap + if foot.is_some() { bh + gap * 2.0 } else { 0.0 };
    let pad = ui.u(24.0);
    let head = ui.u(56.0);
    let r = R::new(area.x + (area.w - bw) * 0.5 - pad, area.y + (area.h - inner_h - head) * 0.5 - pad, bw + pad * 2.0, inner_h + head + pad * 2.0);
    let inner = ui.window(r, title(&f.class));
    let mut y = inner.y + head - pad;
    for c in buttons {
        let b = R::new(inner.x + (inner.w - bw) * 0.5, y, bw, bh);
        if ui.button(b, caption(c)) && c.enabled {
            forms::press(f, c);
        }
        y += bh + gap;
    }
    if let Some(c) = foot {
        let b = R::new(inner.x + (inner.w - bw) * 0.5, y + gap, bw, bh);
        if ui.button(b, caption(c)) && c.enabled {
            forms::press(f, c);
        }
    }
    if let Some(v) = f.control("labelVersion").filter(|c| c.visible) {
        let vr = fit.rect(f, v.rect);
        let t = ui.theme;
        let line = R::new(area.x, vr.y, area.w - ui.u(16.0), vr.h.max(ui.u(20.0)));
        ui.text_in(line, &v.text, t.small_size, Align::Right, t.text_dim, Weight::Regular);
    }
}

/// Yes/No: Roots draws its question into the form's picture, so the picture stands
/// behind our two buttons.
fn yes_no(ui: &mut Ui, fit: &Fit, f: &Form) {
    let r = fit.form(f);
    ui.panel(r.inset(-ui.u(6.0)));
    if let Some((t, _, _)) = picture().filter(|_| f.top) {
        ui.draw.image(t, r, 0xff_ff_ff_ff);
    }
    for c in shown(f, Kind::Button) {
        if ui.button(fit.rect(f, c.rect), caption(c)) && c.enabled {
            forms::press(f, c);
        }
    }
}

/// Any form: its controls where the form has them.
fn generic(ui: &mut Ui, fit: &Fit, f: &Form, widgets: bool, live: bool) {
    let r = fit.form(f);
    let full = f.size.0 >= forms::design().0 && f.size.1 >= forms::design().1;
    let inner = if full { ui.window(r, title(&f.class)) } else {
        ui.panel(r);
        r
    };
    let _ = inner;
    let pic = if live { picture() } else { None };
    if !widgets {
        if let Some((t, _, _)) = pic {
            ui.draw.image(t, r, 0xff_ff_ff_ff);
        }
        forward(ui, fit, f, r);
        return;
    }
    // A small form (Manual IP and the like) has its title and labels painted into its
    // background by Roots, not as controls: its picture stands behind our widgets, as
    // Yes/No's does (decision 184).
    if !full {
        if let Some((t, _, _)) = pic {
            ui.draw.image(t, r, 0xff_ff_ff_ff);
        }
    }
    let mut controls: Vec<&Control> = f.controls.iter().filter(|c| c.visible && c.rect[2] > 0 && c.rect[3] > 0).collect();
    controls.sort_by_key(|c| (c.rect[1], c.rect[0]));
    for (i, c) in controls.iter().enumerate() {
        let cr = fit.rect(f, c.rect);
        ui.scope(&crate::sfmt!("c{}", i), |ui| match c.kind {
            Kind::Image => {}
            Kind::Button => {
                if ui.button(cr, caption(c)) && c.enabled {
                    forms::press(f, c);
                }
            }
            Kind::Label => ui.label(cr, &c.text),
            Kind::Edit => {
                // The text field is Roots' own: a click gives it the keyboard, the typing
                // goes to it, and ours shows what it holds.
                let shown_text: String = if c.text.is_empty() { String::from(" ") } else { c.text.clone() };
                if ui.button(cr, &shown_text) {
                    forms::act(Action::Focus(c.hwnd));
                }
            }
            Kind::Combo => {
                let items: Vec<&str> = c.items.iter().map(|s| s.as_str()).collect();
                let mut sel = c.selected.max(0) as usize;
                if ui.dropdown(cr, &c.name, &items, &mut sel) && sel as i32 != c.selected {
                    crate::log!("forms: {} of {} set to {:?}", c.name, f.class, items.get(sel));
                    forms::act(Action::Select(c.hwnd, sel as i32));
                }
            }
            Kind::Other => {
                if let Some((t, w, h)) = pic {
                    let uv = [
                        c.rect[0] as f32 / w as f32,
                        c.rect[1] as f32 / h as f32,
                        (c.rect[0] + c.rect[2]) as f32 / w as f32,
                        (c.rect[1] + c.rect[3]) as f32 / h as f32,
                    ];
                    ui.draw.image_uv(t, cr, uv, 0xff_ff_ff_ff);
                }
                forward(ui, fit, f, cr);
            }
        });
    }
}

/// The mouse over `r` goes to the form as it is: a press, the moves while held, the
/// release, at the point in the form's client coordinates.
fn forward(ui: &mut Ui, fit: &Fit, f: &Form, r: R) {
    let Some(m) = ui.input.mouse else { return };
    let over = r.contains(m);
    if over {
        ui.claim(r);
    }
    let (x, y) = fit.point(f, m);
    unsafe {
        if over && ui.input.pressed[0] && HELD == 0 {
            HELD = f.hwnd;
            forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
        } else if HELD == f.hwnd {
            if ui.input.released[0] || !ui.input.down[0] {
                HELD = 0;
                forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, 0, x, y));
            } else {
                forms::act(Action::Mouse(f.hwnd, forms::MOUSE_MOVE, forms::MOUSE_HELD, x, y));
            }
        }
    }
}

/// An adopted dialog's picture as a texture, by window (textures are kept and reused as
/// dialogs come and go, as the host's mirrors are).
static mut DIALOG_TEX: Vec<(usize, Tex, i32, i32, u32)> = Vec::new();

fn dialog_texture(d: &Dialog) -> Option<Tex> {
    let (bits, w, h, serial) = forms::dialog_picture(d.hwnd)?;
    let v = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG_TEX) };
    let i = match v.iter().position(|e| e.0 == d.hwnd && e.2 == w && e.3 == h) {
        Some(i) => i,
        None => match v.iter().position(|e| e.2 == w && e.3 == h && !forms::dialogs().iter().any(|o| o.hwnd == e.0)) {
            Some(i) => {
                v[i].0 = d.hwnd;
                v[i].4 = serial.wrapping_sub(1);
                i
            }
            None => {
                v.push((d.hwnd, gfx::texture_dynamic(w as u32, h as u32), w, h, serial.wrapping_sub(1)));
                v.len() - 1
            }
        },
    };
    if v[i].4 != serial {
        gfx::tex_update(v[i].1, bits, w as usize * 4, w as u32, h as u32);
        v[i].4 = serial;
    }
    Some(v[i].1)
}

/// The pointer over the dialog's picture goes to the dialog as the real pointer would:
/// every move (the stock buttons light up and take a click only under the pointer), the
/// press, the moves while held and the release, in its client coordinates.
static mut DIALOG_HOVER: (usize, i32, i32) = (0, i32::MIN, i32::MIN);

fn forward_dialog(ui: &mut Ui, d: &Dialog, r: R, s: f32) {
    let Some(m) = ui.input.mouse else { return };
    let over = r.contains(m);
    if over {
        ui.claim(r);
    }
    let x = libm::floorf((m.0 - r.x) / s) as i32;
    let y = libm::floorf((m.1 - r.y) / s) as i32;
    unsafe {
        let moved = DIALOG_HOVER != (d.hwnd, x, y);
        if HELD == d.hwnd {
            if ui.input.released[0] || !ui.input.down[0] {
                HELD = 0;
                forms::act(Action::Mouse(d.hwnd, forms::MOUSE_UP, 0, x, y));
            } else if moved {
                forms::act(Action::Mouse(d.hwnd, forms::MOUSE_MOVE, forms::MOUSE_HELD, x, y));
            }
        } else if over {
            if moved {
                forms::act(Action::Mouse(d.hwnd, forms::MOUSE_MOVE, 0, x, y));
            }
            if ui.input.pressed[0] && HELD == 0 {
                HELD = d.hwnd;
                forms::act(Action::Mouse(d.hwnd, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
            }
        }
        if over || HELD == d.hwnd {
            DIALOG_HOVER = (d.hwnd, x, y);
        }
    }
}
