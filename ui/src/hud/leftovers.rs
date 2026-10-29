//! The engine's last draws in a mission, drawn by the UI (`engine::leftovers`; HUD.md,
//! "The engine's last draws"). Each keeps the engine's look: the drag box four white
//! lines a pixel wide, the height posts green or red lines a pixel wide, the fade a
//! rectangle of its colour at its alpha, the unit view's video between two black bars;
//! all but the placement markers, which are drawn in the theme (`placement`, `[HUD]
//! Placement=1`). They are drawn first in the UI's frame, under every panel of ours,
//! where the engine drew them (in the 3D view, under its HUD).
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
    let ours = crate::config::get().hud.placement;
    for p in &f.places {
        super::placement::run(ui, p, ours);
    }
    if f.places.is_empty() {
        super::placement::none();
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
    if let Some(fr) = leftovers::frame_rate() {
        frame_rate(ui, fr);
    }
}

/// What `FRAMERATE_DISPLAY` prints (the engine's top-left lines: the triangles, the
/// frame rate, the total frame rate, each timing slot), in a small panel of the theme
/// at the top left: a label and its value a line, the frame rate first and largest.
fn frame_rate(ui: &mut Ui, fr: &leftovers::FrameRate) {
    use crate::gfx::Weight;
    use crate::widgets::Align;
    let t = ui.theme;
    let mut rows: alloc::vec::Vec<(alloc::string::String, alloc::string::String)> = alloc::vec::Vec::new();
    rows.push((alloc::string::String::from("Total FPS"), crate::sfmt!("{:.1}", fr.total_fps)));
    rows.push((alloc::string::String::from("Triangles"), crate::sfmt!("{}", fr.triangles)));
    for (name, ms) in &fr.slots {
        rows.push((crate::sfmt!("{} (ms)", name.trim()), crate::sfmt!("{:.1}", ms)));
    }
    let (line, big) = (ui.u(16.0), ui.u(26.0));
    let h = big + line * rows.len() as f32;
    let (outer, inner) = super::panels::fit(ui, crate::widgets::Anchor::TopLeft, ui.u(220.0), h, ui.u(6.0));
    ui.panel(outer);
    let head = R::new(inner.x, inner.y, inner.w, big);
    ui.text_in(head, "FPS", t.micro_size, Align::Left, t.text_dim, Weight::Regular);
    let fps = fr.fps.map_or(alloc::string::String::from("--"), |v| crate::sfmt!("{:.1}", v));
    ui.text_in(head, &fps, t.heading_size, Align::Right, t.text, Weight::Bold);
    for (i, (k, v)) in rows.iter().enumerate() {
        let r = R::new(inner.x, inner.y + big + i as f32 * line, inner.w, line);
        ui.text_in(r, k, t.micro_size, Align::Left, t.text_dim, Weight::Regular);
        ui.text_in(r, v, t.small_size, Align::Right, t.text, Weight::Regular);
    }
}

/// The external textures the rings and footprints draw from, one per engine texture
/// (`ST3D_Texture`): a slot is read when the frame is drawn, so two textures in one
/// frame need two.
static mut RING_TEX: [(usize, Option<crate::gfx::Tex>); 4] = [(0, None); 4];

#[allow(static_mut_refs)]
fn ring_tex(texture: usize) -> crate::gfx::Tex {
    let slots = unsafe { &mut RING_TEX };
    let i = slots.iter().position(|s| s.0 == texture)
        .or_else(|| slots.iter().position(|s| s.0 == 0))
        .unwrap_or(slots.len() - 1);
    slots[i].0 = texture;
    *slots[i].1.get_or_insert_with(crate::gfx::texture_external)
}

/// The order ring or a build footprint: its sprite's rectangle of its texture on the
/// four projected corners, added to the frame as the engine's z-sorted sprites are.
fn ring(ui: &mut Ui, r: &leftovers::Ring) {
    ui.draw.push_clip(clip_rect(r.clip));
    engine_sprite(ui, r.corners, r.texture, r.uv, r.rgb);
    ui.draw.pop_clip();
}

/// An engine sprite on four corners on the screen: its rectangle (`uv`: u0, v0, u1, v1)
/// of the engine's texture (`ST3D_Texture`), tinted, added to the frame.
pub fn engine_sprite(ui: &mut Ui, corners: [(f32, f32); 4], texture: usize, uv: [f32; 4], col: [f32; 3]) {
    let tex = ring_tex(texture);
    let Some((c, tw, th)) = crate::engine::radar::engine_texture(texture) else { return };
    crate::gfx::tex_set_external(tex, Some(c), tw, th);
    let [u0, v0, u1, v1] = uv;
    let uv = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
    ui.draw.quad_additive(tex, corners, uv, rgb(col, 1.0));
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
