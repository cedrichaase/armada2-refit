//! The widget gallery: every widget of `widgets/` on one screen, switched on with
//! `[Demo] Shell=1`, to check them on the bench (testbench/scenarios/ui-widgets.md).
use crate::gfx::{Weight, R};
use crate::widgets::{Align, Anchor, FieldEvent, ListEvent, Stack, Ui};
use alloc::string::String;

struct Gallery {
    tab: usize,
    name: String,
    code: String,
    submitted: String,
    count: f32,
    gain: f32,
    ship: Option<usize>,
    activated: String,
    faction: usize,
    speed: usize,
    checks: [bool; 2],
    radio: usize,
    picks: u32,
    modal: bool,
    closed: u32,
}

static mut G: Gallery = Gallery {
    tab: 0,
    name: String::new(),
    code: String::new(),
    submitted: String::new(),
    count: 3.0,
    gain: 0.5,
    ship: None,
    activated: String::new(),
    faction: 0,
    speed: 1,
    checks: [true, false],
    radio: 0,
    picks: 0,
    modal: false,
    closed: 0,
};

const SHIPS: [&str; 30] = [
    "Galaxy", "Sovereign", "Defiant", "Akira", "Steamrunner", "Intrepid", "Nova", "Saber", "Norway", "Prometheus",
    "Excelsior", "Constitution", "D'Deridex", "Valdore", "Scimitar", "Hawk", "Falcon", "Venture", "Negh'Var", "Vor'cha",
    "Bird of Prey", "Raptor", "Cube", "Sphere", "Diamond", "Scout", "Assimilator", "Queen", "Probe", "Tactical",
];
const FACTIONS: [&str; 6] = ["Federation", "Klingon", "Romulan", "Borg", "Cardassian", "Species 8472"];
const SPEEDS: [&str; 3] = ["Slow", "Normal", "Fast"];
const TABS: [&str; 4] = ["Text", "Lists", "Choice", "Misc"];

pub fn run(ui: &mut Ui) {
    let g = unsafe { &mut *core::ptr::addr_of_mut!(G) };
    let screen = ui.screen();
    ui.keyboard_nav();
    let r = Anchor::Right.place(screen, ui.u(800.0), ui.u(720.0), ui.u(40.0));
    let inner = ui.window(r, "WIDGET GALLERY");
    let t = ui.theme;
    let row = ui.u(36.0);
    let mut s = Stack::new(inner, ui.u(10.0));
    ui.tabs(s.row(ui.u(44.0)), "gallery tabs", &TABS, &mut g.tab);
    let body = s.rest();
    let mut b = Stack::new(R::new(body.x, body.y + ui.u(8.0), body.w, body.h - ui.u(8.0)), ui.u(10.0));
    match g.tab {
        0 => {
            ui.note(b.row(ui.u(22.0)), "Name (24 characters, Enter submits)");
            let ev = ui.text_field(b.row(row), "name", &mut g.name, 24);
            if ev == FieldEvent::Submit {
                g.submitted = g.name.clone();
            }
            ui.note(b.row(ui.u(22.0)), "Code (8 characters)");
            ui.text_field(b.row(row), "code", &mut g.code, 8);
            ui.label(b.row(row), &crate::sfmt!("Typed: {} / {}", g.name, g.code));
            ui.label(b.row(row), &crate::sfmt!("Submitted: {}", g.submitted));
            let [a, c] = b.cells::<2>(row);
            ui.spinner(a, "count", &mut g.count, 0.0, 20.0, 1.0, 0);
            ui.spinner(c, "gain", &mut g.gain, 0.0, 1.0, 0.05, 2);
            ui.label(b.row(row), &crate::sfmt!("Count {:.0}   Gain {:.2}", g.count, g.gain));
        }
        1 => {
            let h = ui.u(380.0);
            let [a, c] = b.cells::<2>(h);
            let ev = ui.list_text(a, "ships", &SHIPS, &mut g.ship);
            if ev == ListEvent::Activated {
                g.activated = String::from(g.ship.map_or("", |i| SHIPS[i]));
            }
            let line = ui.u(30.0);
            let content = 40.0 * line;
            ui.scroll_area(c, "log", content, |ui, cr| {
                let t = ui.theme;
                for i in 0..40 {
                    let rr = R::new(cr.x, cr.y + i as f32 * line, cr.w, line);
                    ui.text_in(rr, &crate::sfmt!("Scrolled line {}", i + 1), t.text_size, Align::Left, t.text, Weight::Regular);
                }
            });
            ui.label(b.row(row), &crate::sfmt!("Selected: {}", g.ship.map_or("none", |i| SHIPS[i])));
            ui.label(b.row(row), &crate::sfmt!("Activated: {}", if g.activated.is_empty() { "none" } else { &g.activated }));
        }
        2 => {
            let [a, c] = b.cells::<2>(row);
            ui.dropdown(a, "faction", &FACTIONS, &mut g.faction);
            ui.dropdown(c, "speed", &SPEEDS, &mut g.speed);
            ui.label(b.row(row), &crate::sfmt!("Faction: {}   Speed: {}", FACTIONS[g.faction], SPEEDS[g.speed]));
            ui.checkbox(b.row(row), "Shields", &mut g.checks[0]);
            ui.checkbox(b.row(row), "Cloak", &mut g.checks[1]);
            ui.radio_group(b.row(ui.u(3.0 * 34.0)), "stance", &["Aggressive", "Defensive", "Hold fire"], &mut g.radio);
            ui.label(b.row(row), &crate::sfmt!("Shields {}  Cloak {}  Stance {}", g.checks[0], g.checks[1], g.radio));
            let br = b.row(ui.u(56.0));
            if let Some((tex, uv)) = crate::cursor::sprite("c_attack") {
                let ib = R::new(br.x, br.y, ui.u(56.0), ui.u(56.0));
                if ui.image_button(ib, "attack art", tex, uv) {
                    g.picks += 1;
                }
                ui.tooltip(ib, "An image button: the attack cursor from the game's own art.");
                ui.label(R::new(ib.right() + ui.u(12.0), br.y, br.w, br.h), &crate::sfmt!("Image button clicks: {}", g.picks));
            } else {
                ui.note(br, "No cursor art found");
            }
        }
        _ => {
            let [a, c] = b.cells::<2>(row);
            ui.button(a, "Hover me");
            ui.tooltip(a, "A tooltip: it shows after the pointer rests, follows it, and stays on the screen.");
            if ui.button(c, "Open modal") {
                g.modal = true;
            }
            ui.label(b.row(row), &crate::sfmt!("Modal closed {} times", g.closed));
            ui.note(b.row(ui.u(22.0)), "Tab / Shift+Tab move the focus; Enter or Space activates; Esc backs out.");
            ui.progress(b.row(ui.u(12.0)), g.gain);
            ui.text_in(b.row(row), "Focus a widget on another tab, then come back.", t.text_size * 0.8, Align::Left, t.text_dim, Weight::Regular);
        }
    }
}

/// The modal, drawn last in the frame so the dimming lands on everything.
pub fn modal(ui: &mut Ui) {
    let g = unsafe { &mut *core::ptr::addr_of_mut!(G) };
    if g.modal {
        let esc = ui.modal("A MODAL", 520.0, 260.0, |ui, inner| {
            let mut m = Stack::new(inner, ui.u(10.0));
            ui.label(m.row(ui.u(32.0)), "Everything below is dimmed and takes no input.");
            ui.note(m.row(ui.u(26.0)), "Esc or the button closes it.");
            let q = m.rest();
            if ui.button(R::new(q.x, q.y + ui.u(20.0), ui.u(200.0), ui.u(44.0)), "Close") {
                g.modal = false;
                g.closed += 1;
            }
        });
        if esc && g.modal {
            g.modal = false;
            g.closed += 1;
        }
    }
}
