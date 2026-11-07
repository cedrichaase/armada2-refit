//! The smaller HUD panels (HUD.md, steps 3 and 4 of "The order to take them over"):
//! each behind its own `[HUD]` key, taken over through the panel switch, and drawn
//! here in the order that keeps the boxes on top: the chat under everything, then the
//! resources, the top-right buttons and the replay controls, the overlay messages over
//! them, and last the boxes that pause the game (pause, message, objectives).
//!
//! Shared by them: `game_button`, a button of ours that shows a stock
//! `StandardButton`'s picture, its state and the game's tooltip, and presses it.
use super::{buttons, chat, messages, objectives, overlay_text, pause, replay, resources, trade};
use crate::config::Hud;
use crate::engine::api::{self, hud, popups, sound, sprite, topbar};
use crate::gfx::R;
use crate::widgets::{theme, Align, Ui};
use crate::gfx::Weight;
use alloc::string::String;

/// A HUD panel of ours (HUD.md, "Adding a panel"): a stock component it claims from
/// the engine through the panel switch, its `[HUD]` key, and how it draws. Every panel
/// is one entry of `PANELS`; `install` claims each one switched on, and `run` draws
/// them in the table's order. The widgets are immediate mode, so a panel's input is
/// read in its `draw`; `modal` is the one thing about input the table needs to know.
pub trait Panel: Sync {
    /// Its name, as the layout log notes its frames (`layout::at`).
    fn name(&self) -> &'static str;
    /// Its widget scope (`Ui::scope`): "hud.<name>".
    fn scope(&self) -> &'static str;
    /// Its `[HUD]` key: whether it is switched on in `c`.
    fn key(&self, c: &Hud) -> bool;
    /// The stock component it claims (`engine::api::hud::set_taken`), 0 for none, and
    /// that component's name for the log.
    fn component(&self) -> (usize, &'static str) {
        (0, "")
    }
    /// Claims what the panel takes over from the engine; false when it stands down
    /// (the stock part stays). By default the panel switch takes `component`.
    fn claim(&self) -> bool {
        let (at, name) = self.component();
        hud::set_taken(at, false, true);
        crate::log!("panels: {} ours", name);
        true
    }
    /// The claimed component's rectangle in canvas units (`api::CANVAS`): left, top,
    /// right, bottom, right and bottom exclusive; None without one, or while it has
    /// none.
    fn rect(&self) -> Option<[f32; 4]> {
        hud::panel_rect(self.component().0, api::CANVAS.0, api::CANVAS.1)
    }
    /// Drawn only while the engine shows its HUD (`ingame::hud_hidden`).
    fn with_hud(&self) -> bool {
        false
    }
    /// Input: it takes the whole screen's mouse this frame (a box that pauses the
    /// game), so nothing under it does.
    fn modal(&self) -> bool {
        false
    }
    /// Draws the panel and reads its input.
    fn draw(&self, ui: &mut Ui);
}

/// Every panel of these, in the order they draw, which keeps the boxes on top.
pub static PANELS: &[&dyn Panel] = &[
    &chat::Chat,
    &resources::Resources,
    &buttons::Buttons,
    &replay::Replay,
    &trade::Trade,
    // Over the top bar, as stock's (OverlayText follows ResourcePanel in the list).
    &overlay_text::Overlay,
    &pause::Pause,
    &messages::Messages,
    &objectives::Objectives,
];

/// What `install` took over: bit i for `PANELS[i]`.
static mut ON: Option<u32> = None;

fn on() -> Option<u32> {
    unsafe { ON }
}

/// Any of these panels switched on.
pub fn any(c: &Hud) -> bool {
    PANELS.iter().any(|p| p.key(c))
}

/// Takes over each panel switched on (after the panel switch is installed).
pub fn install(c: &Hud) {
    if !any(c) {
        return;
    }
    if !api::buttons::functions_stock() || !topbar::functions_stock() || !popups::functions_stock() {
        crate::log!("panels: an engine function is not stock: these panels stay stock");
        return;
    }
    api::buttons::install_presses();
    let mut on = 0u32;
    for (i, p) in PANELS.iter().enumerate() {
        if p.key(c) && p.claim() {
            on |= 1 << i;
        }
    }
    unsafe { ON = Some(on) };
}

/// A mission's frame: every panel of these that is on.
static mut BOX_SHOWN: bool = false;
/// `IsGamePausedAtAll` at the last frame, for the log.
static mut PAUSED: Option<bool> = None;

/// Whether a box of ours that pauses the game (message, objectives, pause) showed in
/// the last frame: it wants the pointer, so the cursor shows over it even when the
/// engine names none (`cursor::draw`).
pub fn box_shown() -> bool {
    unsafe { BOX_SHOWN }
}

/// Whether a box that pauses the game and is not ours shows: the engine's briefing and
/// objectives (`ObjectivesDisplay`) or its message box (`MessageDisplay`) with
/// `[HUD] Messages=0` (Roots' default, decision 125). The engine draws it in
/// `RenderAll`, before the UI's frame, and stock's box owns the mouse wherever the
/// pointer is (`CursorOver` always true); so while it shows the HUD of ours stands
/// aside, as stock's HUD sits under the box (HUD.md, "Under the engine's boxes").
pub fn stock_box_up() -> bool {
    let on = on().unwrap_or(0);
    let ours = |name: &str| PANELS.iter().enumerate().any(|(i, p)| on & (1 << i) != 0 && p.name() == name);
    (!ours("objectives") && popups::objectives_shown()) || (!ours("messages") && popups::message().is_some())
}

pub fn run(ui: &mut Ui) {
    let Some(on) = on() else { return };
    let taken = |i: usize| on & (1 << i) != 0;
    let hud_hidden = api::ingame::hud_hidden();
    // Under the engine's own box only the boxes of ours (which pause the game too) draw.
    let under_box = stock_box_up();
    // Whether the engine counts the game paused, logged as it changes: a box of ours
    // leaves the pause to the engine, as the stock box does (fo-ui-briefing-ours).
    let paused = popups::game_paused();
    unsafe {
        if PAUSED != Some(paused) {
            if PAUSED.is_some() || paused {
                crate::log!("panels: the game {}", if paused { "paused" } else { "runs" });
            }
            PAUSED = Some(paused);
        }
    }
    // A box that pauses the game covers everything: nothing under it takes the mouse
    // (its own widgets are drawn in the overlay layer, above the occluder).
    let blocking = PANELS.iter().enumerate().any(|(i, p)| taken(i) && p.modal());
    unsafe { BOX_SHOWN = blocking };
    if blocking {
        let screen = ui.screen();
        ui.claim(screen);
        ui.occlude(screen);
    }
    for (i, p) in PANELS.iter().enumerate() {
        if !taken(i) || (hud_hidden && p.with_hud()) || (under_box && !p.modal()) {
            continue;
        }
        crate::hud::layout::at(p.name());
        ui.scope(p.scope(), |ui| p.draw(ui));
    }
}

// ---- shared ---------------------------------------------------------------------------

struct Hover {
    btn: usize,
    since: f64,
    tip: Option<(bool, String)>,
}

static mut HOVER: Hover = Hover { btn: 0, since: 0.0, tip: None };

/// A square button of ours showing the stock button `b`'s picture: hover, the
/// rollover sound, the game's tooltip (verbose after a while), `on` drawn active.
/// Presses `b` (its own press function) when clicked; true then.
pub fn game_button(ui: &mut Ui, r: R, b: usize, name: &str, on: bool) -> bool {
    let id = ui.id(name);
    let (hot, held, clicked) = ui.interact(id, r);
    let enabled = api::buttons::enabled(b);
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !enabled;
    st.on = on || api::buttons::state(b) == 2;
    let col = theme::button_frame(ui, r, st);
    let e = ui.u(ui.theme.edge).max(1.0);
    let img = r.inset(e + ui.u(4.0));
    let sp = api::buttons::sprite(b);
    let t = ui.theme;
    let ours = crate::config::get().hud.icons && sp != 0 && {
        let ink = if !enabled { crate::gfx::fade(col, 0.45) } else { col };
        let ir = img.inset(ui.u(1.0));
        crate::icons::draw(&mut ui.draw, sprite::name(sp), ir, 1.0, crate::icons::theme_colour(&t, ink))
    };
    match if ours { None } else { sprite::art(sp) } {
        _ if ours => {}
        Some((tex, uv)) => {
            let tint = if !enabled { 0xff5a5a5a } else if hot { 0xffffffff } else { 0xffd0d0d0 };
            ui.draw.image_uv(tex, img, uv, tint);
        }
        None => {
            ui.text_in(img, name, t.micro_size, Align::Centre, col, Weight::Bold);
        }
    }
    // Hover: the rollover sound on entering an enabled button, then the tooltip.
    let h = unsafe { &mut *core::ptr::addr_of_mut!(HOVER) };
    let now = ui.time();
    if hot && h.btn != b {
        if enabled {
            sound::rollover();
        }
        h.btn = b;
        h.since = now;
        h.tip = None;
    } else if !hot && h.btn == b {
        h.btn = 0;
        h.tip = None;
    }
    if hot && h.btn == b && !held {
        let verbose = ui.waited(h.since, 1.5);
        if h.tip.as_ref().map(|t| t.0) != Some(verbose) {
            let text = api::buttons::tooltip(b, verbose).or_else(|| api::buttons::tooltip(b, false)).unwrap_or_default();
            h.tip = Some((verbose, text));
        }
        if let Some((_, text)) = &h.tip {
            if !text.is_empty() {
                let text = text.clone();
                ui.tooltip(r, &text);
            }
        }
    }
    if clicked && enabled {
        crate::log!("panels: click {}", name);
        api::buttons::queue(b);
        return true;
    }
    false
}

/// A text button of ours for the stock button `b`, labelled with its own label (or
/// `fallback`); presses `b` when clicked or when `key` (a virtual key) is pressed.
pub fn game_text_button(ui: &mut Ui, r: R, b: usize, fallback: &str, keys: &[u8]) -> bool {
    let label = api::buttons::label(b);
    let label = if label.is_empty() { String::from(fallback) } else { label };
    let id = ui.id(&crate::sfmt!("btn:{:x}", b));
    let (hot, held, clicked) = ui.interact(id, r);
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !api::buttons::enabled(b);
    theme::button(ui, r, &label, st);
    let key = keys.iter().any(|&k| ui.input.key_pressed(k));
    if (clicked || key) && !st.off {
        crate::log!("panels: {} {}", if clicked { "click" } else { "key" }, label);
        api::buttons::queue(b);
        return true;
    }
    false
}

/// A panel holding `w` x `h` pixels of content, anchored in the screen: the panel's
/// rectangle and the content's, whatever room the theme's frame takes (the LCARS
/// elbows take more than an edge).
pub fn fit(ui: &Ui, at: crate::widgets::Anchor, w: f32, h: f32, margin: f32) -> (R, R) {
    let probe = R::new(0.0, 0.0, 4000.0, 4000.0);
    let i = theme::panel_inner(ui, probe);
    let (l, t, r, b) = (i.x, i.y, probe.right() - i.right(), probe.bottom() - i.bottom());
    let outer = at.place(ui.screen(), w + l + r, h + t + b, margin);
    (outer, R::new(outer.x + l, outer.y + t, w, h))
}

/// A box over a dimmed screen, in the overlay layer: a window titled `title`, `w` x
/// `h` units in the middle; the room inside it. Unlike `Ui::modal` it keeps the mouse
/// but not the keyboard: the stock box's keys are the engine's (`popups`).
pub fn boxed(ui: &mut Ui, title: &str, w: f32, h: f32, dim: f32) -> R {
    let screen = ui.screen();
    let scrim = crate::gfx::fade(ui.theme.scrim, dim);
    ui.draw.rect(screen, scrim);
    ui.claim(screen);
    let r = crate::widgets::Anchor::Centre.place(screen, ui.u(w), ui.u(h), 0.0);
    ui.window(r, title)
}
