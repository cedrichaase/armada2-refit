//! The button bar of ours on a build whose bar is not the stock layout (`palette_slots_indirect`:
//! Fleet Operations: Roots), as a skin (PLAN-FLEETOPS.md, FO-6 step 6, decisions 94, 95 and 128).
//!
//! The stock bar keeps everything: it simulates, owns the pointer (so each button's hover and
//! pressed bytes are the engine's own, and FleetOpsHook's range ring and the tooltip follow them),
//! and a click presses its button. Only its drawing is skipped (`engine::hud::set_bar_skin`). Here
//! each shown button is drawn in the theme over its own rectangle: the palette's origin plus the
//! button's local rectangle, both read from the engine's objects, in slot order. Nothing of
//! FleetOpsHook's memory is read and nothing is claimed or pressed: no `interact`, no panel.
//! The icon is read from the player's install at run time, as on stock (the HUD's sprite
//! database, `icons`' flat icon by name, else the game's own art).
use crate::engine::api::palette;
use crate::engine::api::{hud, sprite};
use crate::gfx::{fade, R};
use crate::widgets::{theme, Align, Ui};

pub fn run(ui: &mut Ui) {
    let p = palette::palette();
    if p == 0 || hud::hidden(p) || crate::engine::api::ingame::hud_hidden() {
        return;
    }
    let Some((ox, oy)) = hud::panel_origin(p, ui.w, ui.h) else { return };
    let (sx, sy) = (ui.w / crate::engine::api::CANVAS.0, ui.h / crate::engine::api::CANVAS.1);
    let cfg = crate::config::get();
    let t = ui.theme;
    for i in 0..palette::n_slots() {
        let b = palette::slot(p, i);
        if !palette::valid(b) {
            continue;
        }
        let Some(lr) = palette::local_rect(b) else { continue };
        let r = R::new(
            ox + lr[0] as f32 * sx,
            oy + lr[1] as f32 * sy,
            (lr[2] - lr[0] + 1) as f32 * sx,
            (lr[3] - lr[1] + 1) as f32 * sy,
        )
        .inset(ui.u(2.0));
        let enabled = palette::state(b) != 0;
        let id = ui.id(&crate::sfmt!("bar.skin.{}", i));
        let mut st = theme::State::of(ui, id, palette::hovered(b) && enabled, palette::pressed(b) && enabled);
        st.on = palette::state(b) == 2;
        st.off = !enabled;
        let content = theme::button_frame(ui, r, st);
        let icon = r.inset(ui.u(5.0));
        let mut sp = palette::sprite(b);
        if sp == 0 {
            sp = palette::own_sprite(b);
        }
        let ours = crate::icons::ours_for_sprite(cfg.hud.icons, sp, |s| sprite::art(s).is_some()) && {
            let ink = if enabled { content } else { fade(content, 0.45) };
            let ir = icon.inset(ui.u(0.5));
            crate::icons::draw(&mut ui.draw, sprite::name(sp), ir, 1.0, crate::icons::theme_colour(&t, ink))
        };
        if ours {
            continue;
        }
        match sprite::art(sp) {
            Some((tex, uv)) => {
                let tint = if enabled { 0xffffffff } else { fade(0xffffffff, 0.4) };
                ui.draw.image_uv(tex, icon, uv, tint);
            }
            None => {
                // No art: the name without a Roots prefix (`npc_tractorbeam`, `all_bastion`).
                let d = palette::describe(b);
                let name = d.name.strip_prefix("npc_").or_else(|| d.name.strip_prefix("all_")).unwrap_or(&d.name);
                ui.text_in(icon, name, t.micro_size, Align::Centre, content, crate::gfx::Weight::Regular);
            }
        }
    }
}
