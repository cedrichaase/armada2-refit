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
use crate::engine::{self, hud, popups, sound, sprite, topbar};
use crate::gfx::R;
use crate::widgets::{theme, Align, Ui};
use crate::gfx::Weight;
use alloc::string::String;

/// What `install` took over (a copy of `[HUD]` with the panels that stood down off).
static mut ON: Option<Hud> = None;

#[allow(static_mut_refs)]
fn on() -> Option<&'static Hud> {
    unsafe { ON.as_ref() }
}

/// Any of these panels switched on.
pub fn any(c: &Hud) -> bool {
    c.resources || c.buttons || c.messages || c.pause || c.replay || c.overlay || c.chat || c.team_texts
}

/// Takes over each panel switched on (after the panel switch is installed).
pub fn install(c: &Hud) {
    if !any(c) {
        return;
    }
    if !engine::buttons::functions_stock() || !topbar::functions_stock() || !popups::functions_stock() {
        crate::log!("panels: an engine function is not stock: these panels stay stock");
        return;
    }
    let mut o = c.clone();
    let take = |flag: bool, at: usize, name: &str| {
        if flag {
            hud::set_taken(at, false, true);
            crate::log!("panels: {} ours", name);
        }
        flag
    };
    engine::buttons::install_presses();
    o.resources = take(c.resources, topbar::RESOURCE_PANEL, "ResourcePanel");
    o.buttons = take(c.buttons, topbar::BUTTON_PANEL, "ButtonPanel");
    o.messages = take(c.messages, popups::MESSAGE, "MessageDisplay") | take(c.messages, popups::OBJECTIVES, "ObjectivesDisplay");
    if o.messages {
        engine::buttons::set_each_pass(popups::objectives_keys);
    }
    o.pause = take(c.pause, popups::PAUSE, "PauseBox");
    o.replay = take(c.replay, popups::REPLAY, "ReplayPanel");
    o.overlay = take(c.overlay, engine::overlay_text::OVERLAY, "OverlayText");
    if c.chat {
        if engine::chat::install() {
            engine::chat::set_taken(true);
            crate::log!("panels: chat lines and text entry ours");
        } else {
            crate::log!("panels: chat stays stock");
            o.chat = false;
        }
    }
    if c.team_texts {
        if engine::trade::install() {
            engine::trade::set_taken(true);
            crate::log!("panels: team texts ours");
        } else {
            o.team_texts = false;
        }
    }
    unsafe { ON = Some(o) };
}

/// A mission's frame: every panel of these that is on.
static mut BOX_SHOWN: bool = false;

/// Whether a box of ours that pauses the game (message, objectives, pause) showed in
/// the last frame: it wants the pointer, so the cursor shows over it even when the
/// engine names none (`cursor::draw`).
pub fn box_shown() -> bool {
    unsafe { BOX_SHOWN }
}

pub fn run(ui: &mut Ui) {
    let Some(o) = on() else { return };
    let hud_hidden = engine::ingame::hud_hidden();
    // A box that pauses the game covers everything: nothing under it takes the mouse
    // (its own widgets are drawn in the overlay layer, above the occluder).
    let blocking = (o.messages && (popups::message().is_some() || popups::objectives_shown())) || (o.pause && popups::pause_shown());
    unsafe { BOX_SHOWN = blocking };
    if blocking {
        let screen = ui.screen();
        ui.claim(screen);
        ui.occlude(screen);
    }
    if o.chat {
        ui.scope("hud.chat", chat::run);
    }
    if !hud_hidden {
        if o.resources {
            ui.scope("hud.resources", resources::run);
        }
        if o.buttons {
            ui.scope("hud.buttons", buttons::run);
        }
    }
    if o.replay {
        ui.scope("hud.replay", replay::run);
    }
    if o.team_texts {
        ui.scope("hud.trade", trade::run);
    }
    // Over the top bar, as stock's (OverlayText follows ResourcePanel in the list).
    if o.overlay {
        ui.scope("hud.overlay", overlay_text::run);
    }
    if o.pause {
        ui.scope("hud.pause", pause::run);
    }
    if o.messages {
        ui.scope("hud.messages", messages::run);
        ui.scope("hud.objectives", objectives::run);
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
    let enabled = engine::buttons::enabled(b);
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !enabled;
    st.on = on || engine::buttons::state(b) == 2;
    let col = theme::button_frame(ui, r, st);
    let e = ui.u(ui.theme.edge).max(1.0);
    let img = r.inset(e + ui.u(4.0));
    let sp = engine::buttons::sprite(b);
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
        let verbose = now - h.since > 1.5;
        if h.tip.as_ref().map(|t| t.0) != Some(verbose) {
            let text = engine::buttons::tooltip(b, verbose).or_else(|| engine::buttons::tooltip(b, false)).unwrap_or_default();
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
        engine::buttons::queue(b);
        return true;
    }
    false
}

/// A text button of ours for the stock button `b`, labelled with its own label (or
/// `fallback`); presses `b` when clicked or when `key` (a virtual key) is pressed.
pub fn game_text_button(ui: &mut Ui, r: R, b: usize, fallback: &str, keys: &[u8]) -> bool {
    let label = engine::buttons::label(b);
    let label = if label.is_empty() { String::from(fallback) } else { label };
    let id = ui.id(&crate::sfmt!("btn:{:x}", b));
    let (hot, held, clicked) = ui.interact(id, r);
    let mut st = theme::State::of(ui, id, hot, held);
    st.off = !engine::buttons::enabled(b);
    theme::button(ui, r, &label, st);
    let key = keys.iter().any(|&k| ui.input.key_pressed(k));
    if (clicked || key) && !st.off {
        crate::log!("panels: {} {}", if clicked { "click" } else { "key" }, label);
        engine::buttons::queue(b);
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
