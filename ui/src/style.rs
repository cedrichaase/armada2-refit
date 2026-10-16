//! The style gallery (`[Demo] Style=1`, ui/STYLE.md): every theme side by side with
//! every widget state, and the type scale, to judge the look on the bench. In the
//! front end it is a screen of ours in the Credits' place, over the menu plate; in a
//! mission it is drawn over the game, to judge the panels against a real scene; on the
//! loading screen it is the loading look.
use crate::gfx::{Corners, Weight, R};
use crate::surface::Context;
use crate::widgets::theme::{self, Form, State};
use crate::widgets::{Align, Stack, Theme, Ui};

struct Live {
    on: [bool; 7],
    v: [f32; 7],
}

static mut LIVE: Live = Live { on: [true; 7], v: [0.6; 7] };

pub fn run(ctx: Context, ui: &mut Ui) {
    match ctx {
        Context::Loading => loading(ui),
        _ => gallery(ctx, ui),
    }
}

fn gallery(ctx: Context, ui: &mut Ui) {
    let screen = ui.screen();
    if ctx == Context::Shell {
        theme::backdrop(ui, screen);
    }
    let shell = Theme::shell();
    let m = ui.u(24.0);
    let gap = ui.u(16.0);
    // The page's title.
    ui.theme = shell;
    let head = R::new(m, m * 0.5, screen.w - 2.0 * m, ui.u(56.0));
    ui.text_in(head, "STYLE", shell.display_size, Align::Left, shell.title, Weight::Bold);
    let sub = R::new(head.x + ui.u(140.0), head.y, head.w - ui.u(140.0), head.h);
    let note = crate::sfmt!(
        "Every theme and widget state.  Units of a 1080-line screen.  {}x{}  ·  Theme for this context: {}",
        ui.w as u32,
        ui.h as u32,
        Theme::for_context(ctx).name
    );
    ui.text_in(sub, &note, shell.small_size, Align::Left, shell.text_dim, Weight::Regular);

    let top = head.bottom() + ui.u(8.0);
    let cols = 4;
    let cw = (screen.w - 2.0 * m - gap * (cols as f32 - 1.0)) / cols as f32;
    let ch = (screen.h - top - m - gap) / 2.0;
    let themes = Theme::all();
    for (i, t) in themes.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let cell = R::new(m + c as f32 * (cw + gap), top + r as f32 * (ch + gap), cw, ch);
        ui.theme = *t;
        ui.scope(t.name, |ui| card(ui, cell, i));
    }
    let cell = R::new(m + 3.0 * (cw + gap), top + (ch + gap), cw, ch);
    ui.theme = shell;
    ui.scope("type", |ui| type_card(ui, cell));
}

/// One theme's card: a window with every widget, in fixed states (the first button,
/// the check boxes and the slider are live).
fn card(ui: &mut Ui, r: R, i: usize) {
    let live = unsafe { &mut *core::ptr::addr_of_mut!(LIVE) };
    let t = ui.theme;
    let inner = ui.window(r, t.name);
    let gap = ui.u(8.0);
    let mut s = Stack::new(inner, gap);
    let bh = ui.u(34.0);
    let [a, b, c] = s.cells::<3>(bh);
    ui.button(a, "Normal");
    theme::button(ui, b, "Hover", State::fixed(1.0, false, false, false));
    theme::button(ui, c, "Pressed", State::fixed(1.0, true, false, false));
    let [a, b, c] = s.cells::<3>(bh);
    theme::button(ui, a, "Active", State::fixed(0.0, false, true, false));
    theme::button(ui, b, "Disabled", State::fixed(0.0, false, false, true));
    theme::button(ui, c, "Focus", State { focus: true, ..State::default() });
    let [a, b] = s.cells::<2>(ui.u(28.0));
    let mut on = live.on[i];
    ui.toggle(a, "Toggle", &mut on);
    live.on[i] = on;
    let w = theme::check(ui, b, false, State::fixed(1.0, false, false, false));
    let lb = R::new(b.x + w + ui.u(t.pad * 0.7), b.y, b.w - w, b.h);
    ui.text_in(lb, "Hover", t.text_size, Align::Left, t.text, Weight::Regular);
    ui.slider(s.row(ui.u(24.0)), "slider", &mut live.v[i], 0.0, 1.0);
    let v = live.v[i];
    ui.progress(s.row(ui.u(12.0)), v);
    // A list: three rows, one hovered, one selected, with its scroll bar.
    let rh = ui.u(26.0);
    let list = s.row(rh * 3.0 + ui.u(4.0));
    theme::field(ui, list, State::default());
    let li = list.inset(ui.u(2.0));
    let sb_w = ui.u(10.0);
    let names = ["Sovereign", "Akira", "Defiant"];
    for (k, n) in names.iter().enumerate() {
        let rr = R::new(li.x, li.y + k as f32 * rh, li.w - sb_w - ui.u(2.0), rh);
        let st = match k {
            1 => State::fixed(1.0, false, false, false),
            2 => State::fixed(0.0, false, true, false),
            _ => State::default(),
        };
        theme::row(ui, rr, k, st);
        let col = theme::row_text(ui, st);
        ui.text_in(R::new(rr.x + ui.u(10.0), rr.y, rr.w - ui.u(10.0), rr.h), n, t.text_size, Align::Left, col, Weight::Regular);
    }
    let track = R::new(li.right() - sb_w, li.y, sb_w, li.h);
    theme::scrollbar(ui, track, R::new(track.x, track.y + li.h * 0.15, sb_w, li.h * 0.45), State::default());
    // A text field with the focus, and a tab strip.
    let f = s.row(ui.u(32.0));
    theme::field(ui, f, State { focus: true, ..State::default() });
    let px = ui.u(t.text_size);
    let tx = f.x + ui.u(t.pad * 0.7);
    let ty = libm::roundf(f.y + (f.h - px * 1.2) * 0.5);
    let tw = ui.draw.text("Sector 001", px, libm::roundf(tx), ty, t.text);
    theme::caret(ui, tx + tw + ui.u(2.0), f.y + ui.u(6.0), f.h - ui.u(12.0), 0.0);
    theme::dropdown_arrow(ui, f, State::default());
    let tabs = s.row(ui.u(30.0));
    let tw3 = (tabs.w - ui.u(8.0)) / 3.0;
    for (k, n) in ["Ships", "Stations", "Tech"].iter().enumerate() {
        let tr = R::new(tabs.x + k as f32 * (tw3 + ui.u(4.0)), tabs.y, tw3, tabs.h);
        let st = match k {
            0 => State::fixed(0.0, false, true, false),
            1 => State::fixed(1.0, false, false, false),
            _ => State::default(),
        };
        theme::tab(ui, tr, n, st);
    }
    // A tooltip, at the card's bottom right.
    let tip = "Tooltip: hull 100%";
    let (w, h) = theme::tooltip_size(ui, tip, inner.w);
    let rest = s.rest();
    if rest.h >= h {
        let tr = R::new(inner.right() - w, rest.y + (rest.h - h) * 0.5, w, h);
        theme::tooltip(ui, tr, tip, 1.0);
        let swatch = ui.u(18.0);
        let mut x = inner.x;
        for c in [t.accent, t.accent2, t.accent3, t.good, t.warn, t.bad] {
            let sr = R::new(x, rest.y + (rest.h - swatch) * 0.5, swatch, swatch);
            ui.draw.fill(sr, if t.form == Form::Lcars { Corners::pill(sr) } else { t.small.scaled(ui.scale) }, c);
            x += swatch + ui.u(5.0);
        }
    }
}

/// The type scale and spacing, in the shell's theme.
fn type_card(ui: &mut Ui, r: R) {
    let t = ui.theme;
    let inner = ui.window(r, "Type and space");
    let mut y = inner.y;
    for (name, size, w, col) in [
        ("Display 44", t.display_size, Weight::Bold, t.title),
        ("Title 26", t.title_size, Weight::Bold, t.title),
        ("Heading 22", t.heading_size, Weight::Bold, t.text),
        ("Body 20: Engage at warp 9", t.text_size, Weight::Regular, t.text),
        ("Small 17: Crew 120/120, shields 87%", t.small_size, Weight::Regular, t.text_dim),
        ("Micro 14: 3500  2000  500", t.micro_size, Weight::Regular, t.text_dim),
    ] {
        let px = ui.u(size);
        ui.draw.text_w(name, px, inner.x, libm::roundf(y), col, w);
        y += px * 1.3;
    }
    y += ui.u(6.0);
    theme::separator(ui, R::new(inner.x, y, inner.w, ui.u(8.0)));
    y += ui.u(14.0);
    let mut x = inner.x;
    for sp in [4.0, 8.0, 12.0, 16.0, 24.0, 32.0] {
        let b = ui.u(sp);
        ui.draw.rect(R::new(x, y + ui.u(32.0) - b, b, b), t.accent2);
        ui.draw.text(&crate::sfmt!("{}", sp as u32), ui.u(t.micro_size), x, y + ui.u(36.0), t.text_dim);
        x += b.max(ui.u(18.0)) + ui.u(14.0);
    }
    theme::heading(ui, R::new(x + ui.u(8.0), y, inner.right() - x, ui.u(32.0)), "Spacing");
}

/// The loading screen's look: a band across the bottom with the mission's name, a
/// line of text and the bar, in the shell's theme; the engine's logo scene stays
/// behind it.
fn loading(ui: &mut Ui) {
    let t = ui.theme;
    let screen = ui.screen();
    let bh = ui.u(180.0);
    let band = R::new(0.0, screen.h - bh, screen.w, bh);
    ui.draw.rect_vgrad(band, 0x00000000, 0xe0000000);
    let m = ui.u(64.0);
    let p = crate::engine::loading::progress().unwrap_or(0.0);
    let w = (screen.w - 2.0 * m).min(ui.u(1400.0));
    let x = (screen.w - w) * 0.5;
    let title = R::new(x, band.y + ui.u(30.0), w, ui.u(40.0));
    ui.text_in(title, "INSTANT ACTION", t.title_size, Align::Left, t.title, Weight::Bold);
    ui.text_in(title, &crate::sfmt!("{:.0}%", p * 100.0), t.title_size, Align::Right, t.text, Weight::Bold);
    ui.text_in(R::new(x, title.bottom(), w, ui.u(26.0)), "Loading the map and its ships", t.small_size, Align::Left, t.text_dim, Weight::Regular);
    // Above the engine's own bar, which runs along the screen's last 40 units until
    // the loading screen is ours (PLAN.md, step 5).
    theme::bar(ui, R::new(x, band.bottom() - ui.u(64.0), w, ui.u(10.0)), p, None);
}
