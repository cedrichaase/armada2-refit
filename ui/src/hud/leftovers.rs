//! The engine's last draws in a mission, drawn by the UI (`engine::leftovers`; HUD.md,
//! "The engine's last draws"). Each keeps the engine's look: the drag box four white
//! lines a pixel wide, the height posts green or red lines a pixel wide, the fade a
//! rectangle of its colour at its alpha, the unit view's video between two black bars.
//! They are drawn first in the UI's frame, under every panel of ours, where the engine
//! drew them (in the 3D view, under its HUD).
use crate::engine::leftovers;
use crate::gfx::{Color, R};
use crate::widgets::Ui;

static mut VIDEO_TEX: Option<crate::gfx::Tex> = None;

fn rgb(c: [f32; 3], a: f32) -> Color {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    (b(a) << 24) | (b(c[0]) << 16) | (b(c[1]) << 8) | b(c[2])
}

fn clip_rect(c: [f32; 4]) -> R {
    R::new(c[0], c[1], c[2], c[3])
}

/// Draws what the engine would have drawn this frame. Called first in the mission
/// frame, before every panel of ours.
pub fn run(ui: &mut Ui) {
    let f = leftovers::take();
    for r in &f.rings {
        ring(ui, r);
    }
    for v in &f.video {
        video(ui, v);
    }
    for p in &f.posts {
        ui.draw.push_clip(clip_rect(p.clip));
        ui.draw.line((p.a.0 + 0.5, p.a.1 + 0.5), (p.b.0 + 0.5, p.b.1 + 0.5), 1.0, rgb(p.rgb, 1.0));
        ui.draw.pop_clip();
    }
    if let Some(d) = f.drag {
        // DrawWireframeRectangle: lines along the four edges, from (x, y) to (x + w, y + h).
        let [x, y, w, h] = d.rect;
        let (l, r) = (libm::floorf(x.min(x + w)), libm::floorf(x.max(x + w)));
        let (t, b) = (libm::floorf(y.min(y + h)), libm::floorf(y.max(y + h)));
        let c = rgb(d.rgb, 1.0);
        ui.draw.push_clip(clip_rect(d.clip));
        ui.draw.rect(R::new(l, t, r - l + 1.0, 1.0), c);
        ui.draw.rect(R::new(l, b, r - l + 1.0, 1.0), c);
        ui.draw.rect(R::new(l, t, 1.0, b - t + 1.0), c);
        ui.draw.rect(R::new(r, t, 1.0, b - t + 1.0), c);
        ui.draw.pop_clip();
    }
    if let Some(fd) = f.fade {
        if fd.alpha > 0.0 {
            ui.draw.rect(clip_rect(fd.rect), rgb(fd.rgb, fd.alpha));
        }
    }
}

static mut RING_TEX: Option<crate::gfx::Tex> = None;

/// The order ring: its sprite's texture on the four projected corners, added to the
/// frame as the engine's z-sorted sprites are.
fn ring(ui: &mut Ui, r: &leftovers::Ring) {
    let tex = unsafe { *RING_TEX.get_or_insert_with(crate::gfx::texture_external) };
    let Some((c, tw, th)) = crate::engine::radar::engine_texture(r.texture) else { return };
    crate::gfx::tex_set_external(tex, Some(c), tw, th);
    let uv = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    ui.draw.push_clip(clip_rect(r.clip));
    ui.draw.quad_additive(tex, r.corners, uv, rgb(r.rgb, 1.0));
    ui.draw.pop_clip();
}

/// The video as `BlitToBlackBackground` lays it out: as wide as the rectangle, at the
/// video's aspect, centred, between black bars.
fn video(ui: &mut Ui, v: &leftovers::Video) {
    let [x, y, w, h] = v.rect;
    ui.draw.rect(R::new(x, y, w, h), 0xff000000);
    let tex = unsafe { *VIDEO_TEX.get_or_insert_with(crate::gfx::texture_external) };
    let Some((c, tw, th)) = crate::engine::radar::engine_texture(v.texture) else {
        crate::gfx::tex_set_external(tex, None, 1, 1);
        return;
    };
    crate::gfx::tex_set_external(tex, Some(c), tw, th);
    if tw == 0 || th == 0 {
        return;
    }
    let vh = libm::floorf(w * v.size.1 as f32 / v.size.0 as f32);
    let top = libm::floorf((h - vh) * 0.5);
    let uv = [0.0, 0.0, (v.size.0 as f32 / tw as f32).min(1.0), (v.size.1 as f32 / th as f32).min(1.0)];
    ui.draw.image_uv(tex, R::new(x, y + top, w, vh), uv, 0xffffffff);
}
