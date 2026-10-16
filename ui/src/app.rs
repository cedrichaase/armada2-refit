//! What the UI shows in each context. For now the runtime's own demonstration
//! overlays, switched on per context in UI.ini's `[Demo]`; nothing is drawn otherwise,
//! so an install with the defaults looks exactly like the game without the plugin.
use crate::gfx::{fade, Weight, R};
use crate::surface::Context;
use crate::widgets::{Align, Anchor, Stack, Ui};

struct Demo {
    on: bool,
    slider: f32,
    clicks: u32,
}

static mut DEMO: Demo = Demo { on: true, slider: 0.5, clicks: 0 };

pub fn run(ctx: Context, ui: &mut Ui) {
    let cfg = crate::config::get();
    if ctx == Context::Menus {
        crate::menus::frame(ui);
        return;
    }
    if ctx == Context::Loading && cfg.loading {
        crate::loading::run(ui);
    }
    if cfg.demo_style {
        crate::style::run(ctx, ui);
        return;
    }
    let on = match ctx {
        Context::Mission => cfg.demo_mission,
        Context::Shell => cfg.demo_shell,
        Context::Loading => cfg.demo_loading,
        Context::Menus => false,
    };
    if ctx == Context::Mission {
        crate::hud::run(ui);
    }
    if on {
        demo(ctx, ui);
    }
}

/// A panel showing what the runtime draws: text in both weights, every widget, the
/// pointer as the UI sees it, and the context it is in.
fn demo(ctx: Context, ui: &mut Ui) {
    let d = unsafe { &mut *core::ptr::addr_of_mut!(DEMO) };
    let screen = ui.screen();
    if ctx == Context::Shell {
        // A screen of our own: fill it.
        ui.draw.rect_vgrad(screen, 0xff050a14, 0xff0c1a2e);
    }
    let (w, h) = (ui.u(460.0), ui.u(400.0));
    let anchor = match ctx {
        Context::Mission => Anchor::TopRight,
        Context::Shell => Anchor::Left,
        Context::Loading | Context::Menus => Anchor::BottomLeft,
    };
    if ctx == Context::Shell {
        crate::gallery::run(ui);
    }
    let r = anchor.place(screen, w, h, ui.u(if ctx == Context::Mission { 96.0 } else { 40.0 }));
    let inner = ui.window(r, "UI RUNTIME");
    let row = ui.u(30.0);
    let mut s = Stack::new(inner, ui.u(8.0));
    let name = match ctx {
        Context::Mission => "Mission",
        Context::Shell => "Shell",
        Context::Loading => "Loading",
        Context::Menus => "Menus",
    };
    ui.label(s.row(row), &crate::sfmt!("Context: {}", name));
    ui.note(s.row(row), &crate::sfmt!("Back buffer {}x{}", ui.w as u32, ui.h as u32));
    let m = match ui.input.mouse {
        Some((x, y)) => crate::sfmt!("Pointer {:.0}, {:.0}{}", x, y, if ui.input.down[0] { "  (held)" } else { "" }),
        None => crate::sfmt!("Pointer outside the window"),
    };
    ui.note(s.row(row), &m);
    if ctx == Context::Loading {
        let p = crate::engine::loading::progress();
        ui.note(s.row(row), &match p {
            Some(f) => crate::sfmt!("Loading {:.0}%", f * 100.0),
            None => crate::sfmt!("No load in progress"),
        });
        ui.progress(s.row(ui.u(12.0)), p.unwrap_or(0.0));
    } else {
        let [a, b] = s.cells::<2>(ui.u(40.0));
        if ui.button(a, "Click me") {
            d.clicks += 1;
        }
        let t = ui.theme;
        ui.text_in(b, &crate::sfmt!("{} clicks", d.clicks), t.text_size, Align::Centre, t.accent, Weight::Bold);
        let mut on = d.on;
        ui.toggle(s.row(row), "A toggle", &mut on);
        d.on = on;
        ui.slider(s.row(row), "A slider", &mut d.slider, 0.0, 1.0);
        ui.progress(s.row(ui.u(12.0)), d.slider);
    }
    // Every size of text, and the time, to see it animate.
    let t = ui.time();
    let rest = s.rest();
    let sizes = [14.0, 18.0, 28.0];
    let mut y = rest.y;
    for sz in sizes {
        let px = ui.u(sz);
        ui.draw.text(&crate::sfmt!("{:.0} units: Sphinx of black quartz", sz), px, rest.x, y, fade(ui.theme.text, 0.9));
        y += px * 1.25;
    }
    let pulse = 0.5 + 0.5 * libm::sinf(t as f32 * 3.0);
    let rad = ui.u(ui.theme.radius);
    ui.draw.rect(R::new(r.x + rad, r.bottom() - ui.u(4.0), (r.w - 2.0 * rad) * pulse, ui.u(2.0)), ui.theme.accent);
    if ctx == Context::Shell {
        crate::gallery::modal(ui);
    }
}
