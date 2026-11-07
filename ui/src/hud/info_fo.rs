//! The info panel on Fleet Operations: Roots, ours in Roots' layout (HUD.md, "The info panel on
//! Fleet Operations: Roots"; PLAN-FLEETOPS.md, FO-6, decisions 440 to 459).
//!
//! FleetOpsHook's `ShipDisplay` keeps simulating and its `Render` keeps running, unseen
//! (`engine::info_fo`); ours draws what its children say in the places of Roots' own layout,
//! which is data (`[InfoPanel]` in the content set: `content::InfoLayout`): the name, the crew and
//! special energy, the shield and hull bars (ours), the ship image (`<class>_si`, Roots' art) with
//! FleetOpsHook's weapon and buff icons where FleetOpsHook put them, or the wireframe picture, the
//! rank, the subsystems and the race icon, a builder's queue on Roots' plates, the construction,
//! research or store bar, and a group's tiles. The frame is the theme's.
//!
//! **The pointer.** Where ours keeps Roots' left edge (`row::Plan::native`), the panel keeps the
//! pointer over ours (`hud::set_info_area`), and nothing of ours takes a click: the engine's own
//! hover flags drive the tooltips and FleetOpsHook's range rings (a weapon icon's red ring, the
//! sensors' and the weapons' subsystem rings), and the clicks are the engine's (the camera, a
//! queue slot's cancel, a group icon's selection). The right-anchored children (the queue's
//! slots, the bar) are moved to follow ours when the panel is narrower or wider. Where ours is
//! moved off Roots' left edge (`BottomRow` with the bar before the info panel), FleetOpsHook's
//! icons cannot follow (their rectangles are its own); ours then takes the pointer and does the
//! clicks itself, as on Armada II, and the rings of the weapon icons are not had there.
use super::info::{hull_colour, tooltip, Tip};
use super::row;
use crate::content::{Anchor, Element, InfoLayout};
use crate::engine::api::{hud, info, info_fo, sprite, CANVAS};
use crate::gfx::{fade, mix, Color, Corners, Weight, R};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;

/// The panel's rectangle in the last frame it drew, and when.
static mut RECT: Option<(R, f64)> = None;
static mut ON: bool = false;
static mut LAST_LOG: String = String::new();

/// Whether ours draws Roots' panel (the build's info panel is FleetOpsHook's and its gate is in).
pub fn on() -> bool {
    unsafe { ON }
}

/// Switches ours on or off: the panel's `Render` runs unseen under ours, or as Roots draws it.
pub fn set(on: bool) -> bool {
    if on && !info_fo::install() {
        crate::log!("info: Roots' panel stays Roots' (its draw gate or reads are not ready)");
        return false;
    }
    hud::set_info_unseen(on);
    if !on {
        info_fo::restore();
    }
    unsafe { ON = on };
    true
}

/// The rectangle ours drew in, while it draws.
pub fn rect() -> Option<R> {
    unsafe {
        match RECT {
            Some((r, at)) if crate::rt::now() - at < 0.5 => Some(r),
            _ => None,
        }
    }
}

/// Where the table's elements land this frame: the panel's own rectangle (the engine children's
/// origin) and ours, the table's units in pixels, how far the right edge moved (table units) and
/// the scale of the right-anchored elements when they would run into the left ones.
#[derive(Clone, Copy)]
struct Frame {
    ours: R,
    native: R,
    kx: f32,
    ky: f32,
    d: f32,
    /// (factor, the right-anchored elements' right edge in table units after the move).
    squeeze: Option<(f32, f32)>,
    aligned: bool,
}

impl Frame {
    /// An element's rectangle in table units, moved by its anchor.
    fn units(&self, e: &Element, dx: f32, dy: f32) -> [f32; 4] {
        let [x, y, w, h] = e.rect;
        let (x, y) = (x + dx, y + dy);
        match e.anchor {
            Anchor::Left => [x, y, w, h],
            Anchor::Centre => [x + self.d * 0.5, y, w, h],
            Anchor::Flex => [x, y, (w + self.d).max(w * 0.25), h],
            Anchor::Right => match self.squeeze {
                Some((f, right)) => {
                    let nx = right - (right - (x + self.d)) * f;
                    [nx, y + h * (1.0 - f) * 0.5, w * f, h * f]
                }
                None => [x + self.d, y, w, h],
            },
        }
    }
    /// An element's rectangle in pixels.
    fn px(&self, e: &Element, dx: f32, dy: f32) -> R {
        let [x, y, w, h] = self.units(e, dx, dy);
        R::new(self.ours.x + x * self.kx, self.ours.y + y * self.ky, w * self.kx, h * self.ky)
    }
    /// The same in the engine's canvas units, local to the panel's own origin (a child's
    /// rectangle, inclusive).
    fn canvas(&self, ui: &Ui, e: &Element, dx: f32, dy: f32) -> [i32; 4] {
        let r = self.px(e, dx, dy);
        let (cx, cy) = (CANVAS.0 / ui.w, CANVAS.1 / ui.h);
        let l = libm::roundf((r.x - self.native.x) * cx) as i32;
        let t = libm::roundf((r.y - self.native.y) * cy) as i32;
        [l, t, (libm::roundf((r.right() - self.native.x) * cx) as i32 - 1).max(l), (libm::roundf((r.bottom() - self.native.y) * cy) as i32 - 1).max(t)]
    }
    /// A child's own rectangle (canvas units, local to the panel's origin) in pixels on ours.
    fn child_px(&self, ui: &Ui, r: [i32; 4]) -> R {
        let (sx, sy) = (ui.w / CANVAS.0, ui.h / CANVAS.1);
        R::new(self.ours.x + r[0] as f32 * sx, self.ours.y + r[1] as f32 * sy, (r[2] - r[0] + 1) as f32 * sx, (r[3] - r[1] + 1) as f32 * sy)
    }
}

/// The right-anchored elements' squeeze: when, moved with the right edge, they would start left
/// of where the left-anchored ones end, they are scaled down about the right edge to fit.
fn squeeze(l: &InfoLayout, d: f32) -> Option<(f32, f32)> {
    let right: alloc::vec::Vec<&Element> = l.elements.iter().filter(|e| e.anchor == Anchor::Right).collect();
    if right.is_empty() || d >= 0.0 {
        return None;
    }
    let lo = right.iter().map(|e| e.rect[0]).fold(f32::MAX, f32::min);
    let hi = right.iter().map(|e| e.rect[0] + e.rect[2]).fold(f32::MIN, f32::max);
    // The queue runs QueueStep further than its second slot: its last slot is its right end.
    let steps = l.number("QueueStep", 0.0) * (info::queue_len().saturating_sub(2)) as f32;
    let hi = hi.max(l.element("Queue1").map_or(hi, |q| q.rect[0] + q.rect[2] + steps));
    let left_end = l.elements.iter().filter(|e| e.anchor == Anchor::Left && !e.name.starts_with("Group") && !e.name.starts_with("Small")).map(|e| e.rect[0] + e.rect[2]).fold(0.0, f32::max);
    let start = lo + d;
    if start >= left_end + 4.0 {
        return None;
    }
    let right_end = hi + d;
    let f = ((right_end - left_end - 4.0) / (hi - lo)).clamp(0.2, 1.0);
    Some((f, right_end))
}

/// A sprite stretched over `r`, as Roots draws its pictures; false when it has no art.
fn sprite_over(ui: &mut Ui, sp: usize, r: R, tint: Color) -> bool {
    match sprite::art(sp) {
        Some((tex, uv)) => {
            ui.draw.image_uv(tex, r, uv, tint);
            true
        }
        None => false,
    }
}

/// A sprite fitted into `r`, its aspect kept, centred.
fn sprite_in(ui: &mut Ui, sp: usize, r: R, tint: Color) -> bool {
    let Some((tex, uv)) = sprite::art(sp) else { return false };
    let (tw, th) = crate::gfx::tex_size(tex);
    let (pw, ph) = ((uv[2] - uv[0]).abs() * tw as f32, (uv[3] - uv[1]).abs() * th as f32);
    if pw <= 0.0 || ph <= 0.0 {
        return false;
    }
    let s = (r.w / pw).min(r.h / ph);
    let (w, h) = (pw * s, ph * s);
    ui.draw.image_uv(tex, R::new(r.x + (r.w - w) * 0.5, r.y + (r.h - h) * 0.5, w, h), uv, tint);
    true
}

/// A thin bar of ours: its track and its fill, rounded.
fn strip(ui: &mut Ui, r: R, f: f32, col: Color) {
    let t = ui.theme;
    let h = r.h.max(2.0);
    ui.draw.fill(r, Corners::round(h * 0.5), t.track);
    let f = f.clamp(0.0, 1.0);
    if f > 0.0 {
        ui.draw.fill(R::new(r.x, r.y, (r.w * f).max(h), h), Corners::round(h * 0.5), col);
    }
}

/// One line of text of ours in `r`, sized to the rectangle's height.
fn text(ui: &mut Ui, r: R, s: &str, c: Color, w: Weight, align: Align) {
    let size = (r.h / ui.u(1.0) * 0.9).clamp(8.0, 40.0);
    ui.draw.push_clip(R::new(r.x - 2.0, r.y - r.h, r.w + 4.0, r.h * 3.0));
    ui.text_in(r, s, size, align, c, w);
    ui.draw.pop_clip();
}

/// The hull bar's colour: the theme's blue while over half, toward warn, then bad.
fn hull_bar_colour(ui: &Ui, hull: f32) -> Color {
    let t = &ui.theme;
    if hull > 0.5 { t.fill } else if hull > 0.25 { mix(t.warn, t.fill, (hull - 0.25) / 0.25) } else { t.bad }
}

/// A text's colour as Roots' panel gives it (the race's colours, through the text's own
/// answer), or `or` where it gives none (black or plain white).
fn text_colour(ui: &Ui, c: [f32; 3], or: Color) -> Color {
    let _ = ui;
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    if max <= 0.05 || min >= 0.95 { or } else { info::colour(c) }
}

/// A colour of the layout table for the HUD's race (`<key>.<race>`, the race's name without
/// spaces), else `<key>`.
fn race_tint(l: &InfoLayout, key: &str) -> Option<Color> {
    let race = theme::player_race().map(|r| r.name()).unwrap_or("");
    let mut name = String::from(key);
    name.push('.');
    name.extend(race.chars().filter(|c| !c.is_whitespace()));
    let v = if race.is_empty() { &[][..] } else { l.numbers(&name) };
    let v = if v.len() >= 3 { v } else { l.numbers(key) };
    (v.len() >= 3).then(|| 0xff000000 | (v[0] as u32).min(255) << 16 | (v[1] as u32).min(255) << 8 | (v[2] as u32).min(255))
}

/// A subsystem icon's tint: Roots' healthy colour of the race (`SystemTint.<race>`, else the
/// theme's accent), toward `bad` with the damage, `warn` on the flash while it is repaired.
fn system_tint(ui: &Ui, l: &InfoLayout, st: &info::SystemState, flash: bool) -> Color {
    let t = &ui.theme;
    let healthy = race_tint(l, "SystemTint").unwrap_or(mix(t.accent, 0xffffffff, 0.15));
    if st.repairing && flash {
        t.warn
    } else if st.repair_left > 0.01 {
        mix(healthy, t.bad, 0.45 + 0.55 * st.repair_left)
    } else {
        healthy
    }
}

pub fn run(ui: &mut Ui) {
    let panel = hud::ship_display();
    let l = &crate::content::get().info;
    if panel == 0 || l.is_empty() || hud::hidden(panel) || crate::engine::api::ingame::hud_hidden() {
        hud::set_info_area(None);
        unsafe { RECT = None };
        return;
    }
    // A new mission's panel: the rectangles moved for the last one went with it.
    static mut PANEL: usize = 0;
    unsafe {
        if PANEL != panel {
            if PANEL != 0 {
                info_fo::forget();
            }
            PANEL = panel;
        }
    }
    // Roots' own rectangle for the panel this frame (empty while it shows nothing).
    if let Some(r) = hud::panel_rect(panel, ui.w, ui.h) {
        row::learn_native(ui, R::new(r[0], r[1], r[2] - r[0], r[3] - r[1]));
    }
    let face = info::face();
    let obj = info::current();
    let Some(plan) = row::current() else {
        hud::set_info_area(None);
        return;
    };
    let show = match face {
        info::Face::Single | info::Face::Builder => obj != 0,
        info::Face::Group => true,
        _ => false,
    };
    if !show || plan.info.w < 8.0 {
        hud::set_info_area(None);
        unsafe { RECT = None };
        return;
    }
    info_fo::log_children(panel);
    let (kx, ky) = (plan.native.w / l.size[0], plan.native.h / l.size[1]);
    let min_w = l.min_width * kx;
    let ours = R::new(plan.info.x, plan.info.y, plan.info.w.max(min_w.min(ui.w - plan.info.x)), plan.info.h);
    let d = ours.w / kx - l.size[0];
    // Within a pixel of Roots' own width is Roots' own width (no move, no squeeze).
    let d = if d.abs() * kx < 1.5 { 0.0 } else { d };
    let aligned = (ours.x - plan.native.x).abs() < 1.0 && (ours.y - plan.native.y).abs() < 1.0;
    let fr = Frame { ours, native: plan.native, kx, ky, d, squeeze: squeeze(l, d), aligned };

    // The frame: the theme's (with its lamps where Roots' panel has them), never a claimed panel
    // while the panel keeps the pointer.
    theme::panel_as(ui, ours, if l.lamps { theme::Lamps::On } else { theme::Lamps::Off });
    super::layout::note(ours);
    if aligned {
        hud::set_info_area(Some([ours.x, ours.y, ours.right(), ours.bottom()]));
    } else {
        hud::set_info_area(None);
        ui.claim(ours);
    }
    let mut tip = Tip::None;
    match face {
        info::Face::Single | info::Face::Builder => single(ui, l, &fr, obj, face == info::Face::Builder, &mut tip),
        _ => group(ui, l, &fr, &mut tip),
    }
    unsafe { RECT = Some((ours, crate::rt::now())) };
    if !aligned {
        // Ours takes the pointer here: a click elsewhere on the panel takes the camera there.
        let id = ui.id("info.fo.panel");
        if ui.interact(id, ours).2 && obj != 0 && face != info::Face::Group {
            info::look_at(obj);
        }
        tooltip(ui, tip, Some(ours));
    }
}

/// Logs the child the engine has the pointer over, once each time it changes (the range rings
/// follow it; the bench waits on it).
fn note_hover(comp: usize, what: &str, sprite: usize) {
    static mut LAST: usize = 0;
    unsafe {
        if LAST != comp {
            LAST = comp;
            crate::log!("info: the engine's hover on Roots' {} {:#x} ({})", what, comp, sprite::name(sprite));
        }
    }
}

fn log_once(face: &str, obj: usize, line: String) {
    #[allow(static_mut_refs)]
    let last = unsafe { &mut LAST_LOG };
    let key = crate::sfmt!("{}{:#x}", face, obj);
    if *last != key {
        crate::log!("{}", line);
        *last = key;
    }
}

fn el(l: &'static InfoLayout, name: &str) -> Option<&'static Element> {
    l.element(name)
}

fn single(ui: &mut Ui, l: &'static InfoLayout, fr: &Frame, obj: usize, builder: bool, tip: &mut Tip) {
    let t = ui.theme;
    let (name, crew, energy) = info_fo::texts(obj);
    let (hull, shields) = info_fo::hull_shields(obj);
    let craft = info_fo::is_craft(obj);
    let art = info_fo::class_art_name(obj);
    let image = if art.is_empty() { 0 } else { crate::engine::api::palette::find_sprite(&crate::sfmt!("{}{}", art, l.name("ImageSuffix"))) };
    let weapons = if image != 0 { info_fo::weapon_icons(obj) } else { alloc::vec::Vec::new() };
    let buffs = if image != 0 { info_fo::buff_icons(obj) } else { alloc::vec::Vec::new() };

    // The picture: the ship image with FleetOpsHook's icons on it, or the wireframe picture.
    let mut drew_image = false;
    if image != 0 {
        if let Some(e) = el(l, "Image") {
            let tint = match info_fo::cloak(obj) {
                1 | 3 => 0xffb3b3cc,
                2 => 0xff8080ff,
                _ => 0xffffffff,
            };
            drew_image = sprite_over(ui, image, fr.px(e, 0.0, 0.0), tint);
        }
    }
    if !drew_image {
        if let (Some(e), Some(w)) = (el(l, "Wireframe"), info::wire(obj)) {
            let r = fr.px(e, 0.0, 0.0);
            let c = if info::whole_damaged(obj) { t.bad } else { 0xffffffff };
            for &sp in w.sprites.iter().filter(|&&s| s != 0).take(1) {
                sprite_in(ui, sp, r, c);
            }
        }
    }
    // Roots tints its weapon icons with the race's `weaponIconColor` (`WeaponTint.<race>`).
    let weapon_tint = race_tint(l, "WeaponTint").unwrap_or(0xffffffff);
    if drew_image {
        for ic in weapons.iter().chain(buffs.iter()) {
            let r = fr.child_px(ui, ic.rect);
            let hot = info_fo::comp_hover(ic.comp);
            if hot {
                ui.draw.stroke(r.inset(-ui.u(1.0)), Corners::round(r.w * 0.16), ui.u(1.5).max(1.0), t.accent);
                note_hover(ic.comp, "weapon icon", ic.sprite);
            }
            sprite_over(ui, ic.sprite, r, weapon_tint);
            if !fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)) {
                *tip = Tip::Comp(ic.comp, r);
            }
        }
        let rank = info_fo::rank(obj);
        if rank > 0 {
            if let Some(e) = el(l, "Rank") {
                let sp = crate::engine::api::palette::find_sprite(&crate::sfmt!("{}{}", l.name("RankSprite"), rank));
                sprite_in(ui, sp, fr.px(e, 0.0, 0.0), 0xffffffff);
            }
        }
    }

    // The words and the bars, at the left.
    if let (Some(e), Some(n)) = (el(l, "Name"), name.as_ref()) {
        let r = fr.px(e, 0.0, 0.0);
        // The name in the colour Roots gives it (the race's shipNameColor, through the text).
        text(ui, r, n.text.trim(), text_colour(ui, n.colour, t.title), Weight::Bold, Align::Left);
        if !fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)) {
            *tip = Tip::Comp(n.comp, r);
        }
    }
    for (txt, icon_el, text_el) in [(&crew, "CrewIcon", "Crew"), (&energy, "EnergyIcon", "Energy")] {
        let Some(tx) = txt else { continue };
        if let Some(e) = el(l, icon_el) {
            if tx.icon != 0 {
                sprite_in(ui, tx.icon, fr.px(e, 0.0, 0.0), mix(info::colour(tx.icon_colour), 0xffffffff, 0.1));
            }
        }
        if let Some(e) = el(l, text_el) {
            let r = fr.px(e, 0.0, 0.0);
            let r = R::new(r.x, r.y, r.w, r.h.min(fr.ky * 22.0));
            text(ui, r, tx.text.trim(), text_colour(ui, tx.colour, t.text), Weight::Bold, Align::Left);
            if !fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)) {
                *tip = Tip::Comp(tx.comp, r);
            }
        }
    }
    // Roots' two bars: the shields above (green when up, through to red), the hull below (blue,
    // toward warn and bad as it goes); a moon or a resource has the hull's alone.
    if craft {
        if let (Some(e), Some((now, max))) = (el(l, "Shield"), shields) {
            if max > 0.0 {
                let f = (now / max).clamp(0.0, 1.0);
                strip(ui, fr.px(e, 0.0, 0.0), f, hull_colour(ui, f));
            }
        }
    }
    if let Some(e) = el(l, "Hull") {
        strip(ui, fr.px(e, 0.0, 0.0), hull, hull_bar_colour(ui, hull));
    }

    // The subsystems and the race icon.
    let mut sys_line = String::new();
    if craft {
        for i in 0..5 {
            let Some((comp, sp, st, flash, present)) = info::repair_icon(obj, i) else { continue };
            let Some(e) = el(l, &crate::sfmt!("System{}", i)) else { continue };
            let r = fr.px(e, 0.0, 0.0);
            let hot = info_fo::comp_hover(comp);
            if hot {
                ui.draw.stroke(r.inset(-ui.u(1.0)), Corners::round(r.w * 0.16), ui.u(1.5).max(1.0), t.accent);
                note_hover(comp, if i == 0 { "system 0 (sensors)" } else if i == 2 { "system 2 (weapons)" } else { "system" }, sp);
            }
            // A system the craft does not have (a station's engines) is greyed, as Roots does.
            let tint = if present { system_tint(ui, l, &st, flash) } else { 0xff5a5a5a };
            if !sprite_over(ui, sp, r, tint) {
                ui.draw.fill(r.inset(ui.u(2.0)), Corners::round(ui.u(3.0)), tint);
            }
            if st.repair_left > 0.01 {
                sys_line.push_str(&crate::sfmt!(" sys{}:{:.0}%", i, (1.0 - st.repair_left) * 100.0));
            }
            if !fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)) {
                *tip = Tip::Comp(comp, r);
            }
        }
        // Roots shows the race icon while a builder's queue has room, and always for a craft.
        if let Some(e) = el(l, "Race") {
            let sp = info::race_icon(obj);
            sprite_in(ui, sp, fr.px(e, 0.0, 0.0), 0xffffffff);
        }
    }

    // The right: a builder's queue on Roots' plates, the construction, research or store bar.
    let (bar, bar_text) = info::right_bar(obj);
    let mut queued = 0;
    if builder {
        let classes = info_fo::queue_classes(obj);
        let step = l.number("QueueStep", 60.0);
        for (i, &class) in classes.iter().enumerate() {
            let (e, dx) = if i == 0 { (el(l, "Queue0"), 0.0) } else { (el(l, "Queue1"), step * (i - 1) as f32) };
            let Some(e) = e else { continue };
            let r = fr.px(e, dx, 0.0);
            let (comp, plate) = info_fo::queue_slot(i);
            if fr.aligned && (fr.d != 0.0 || fr.squeeze.is_some()) {
                info_fo::move_rect(comp, fr.canvas(ui, e, dx, 0.0));
            }
            let hot = info_fo::comp_hover(comp) || (!fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)));
            if plate == 0 || !sprite_over(ui, plate, r, if hot { 0xffffffff } else { 0xe0ffffff }) {
                ui.draw.fill(r.inset(ui.u(1.0)), Corners::round(ui.u(6.0)), fade(t.track, 0.6));
            }
            if class != 0 {
                queued += 1;
                let name = info_fo::class_art_name_of(class);
                let sp = crate::engine::api::palette::find_sprite(&crate::sfmt!("{}{}", name, l.name("WireSuffix")));
                sprite_in(ui, sp, r.inset(r.w * 0.08), 0xffffffff);
                if !fr.aligned {
                    let id = ui.id(&crate::sfmt!("info.fo.queue{}", i));
                    if ui.interact(id, r).2 {
                        crate::engine::api::sound::click();
                        info::cancel(obj, i);
                    }
                    if hot {
                        *tip = Tip::Comp(comp, r);
                    }
                }
            }
            if hot && class != 0 {
                ui.draw.frame(r, ui.u(1.5), t.accent);
            }
        }
    }
    if let Some((kind, b)) = bar {
        let e = match kind {
            info::BarKind::Construction => el(l, "Progress"),
            _ => el(l, "Store"),
        };
        if let Some(e) = e {
            let r = fr.px(e, 0.0, 0.0);
            let col = match kind {
                info::BarKind::Construction => t.accent,
                _ => {
                    let c = info::colour(b.colour);
                    if c & 0x00ffffff == 0 { t.fill } else { mix(c, t.fill, 0.2) }
                }
            };
            // A bar of the theme's, at Roots' place, a little lower than Roots' tall sprite bar.
            let h = (r.h * 0.45).max(ui.u(6.0));
            theme::bar(ui, R::new(r.x, r.y + (r.h - h) * 0.5, r.w, h), b.fraction(), Some(col));
            if !fr.aligned && ui.input.mouse.is_some_and(|m| r.contains(m)) {
                *tip = Tip::Comp(b.comp, r);
            }
        }
    }
    if let Some(bt) = bar_text.filter(|b| !b.text.trim().is_empty()) {
        if let Some(e) = el(l, "StoreText") {
            let r = fr.px(e, 0.0, 0.0);
            let s = crate::sfmt!("{}{}", bt.label, bt.text);
            text(ui, r, s.trim(), text_colour(ui, bt.colour, t.text), Weight::Regular, Align::Left);
        }
    }

    let face = if builder { "Builder" } else { "Single" };
    log_once(
        face,
        obj,
        crate::sfmt!(
            "info: Roots' panel ours: {} {} | image {} weapons {} buffs {} | hull {:.0}% shields {} | queued {} | {} | {}{}",
            face,
            name.as_ref().map_or("", |n| n.text.trim()),
            if drew_image { "yes" } else { "no (wireframe)" },
            weapons.len(),
            buffs.len(),
            hull * 100.0,
            shields.map_or(String::from("-"), |(n, m)| crate::sfmt!("{:.0}/{:.0}", n, m)),
            queued,
            if fr.aligned { "aligned: the panel keeps the pointer" } else { "moved: ours takes the pointer" },
            crate::sfmt!("width {:.0} of {:.0}{}", fr.ours.w / fr.kx, l.size[0], if fr.squeeze.is_some() { ", squeezed" } else { "" }),
            sys_line
        ),
    );
}

fn group(ui: &mut Ui, l: &'static InfoLayout, fr: &Frame, tip: &mut Tip) {
    let t = ui.theme;
    let units = info::group();
    if units.is_empty() {
        return;
    }
    let max = l.number("GroupMax", 16.0) as usize;
    let small = units.len() > max;
    let rows = l.number("SmallRows", 3.0).max(1.0) as usize;
    let step = l.numbers("SmallStep");
    let (sx, sy) = if step.len() >= 2 { (step[0], step[1]) } else { (120.0, 39.0) };
    let gstep = l.number("GroupStep", 74.0);
    let (tile_el, wire_el, name_el, shield_el, hull_el) = if small {
        (el(l, "SmallTile"), el(l, "SmallWire"), None, el(l, "SmallShield"), el(l, "SmallHull"))
    } else {
        (el(l, "GroupTile"), el(l, "GroupTile"), el(l, "GroupName"), el(l, "GroupShield"), el(l, "GroupHull"))
    };
    let Some(tile_el) = tile_el else { return };
    let shift = ui.input.shift || info::shift_held();
    let limit = if small { rows * 10 } else { max };
    for (i, &u) in units.iter().enumerate().take(limit) {
        let (dx, dy) = if small { (sx * (i / rows) as f32, sy * (i % rows) as f32) } else { (gstep * i as f32, 0.0) };
        let tile = fr.px(tile_el, dx, dy);
        // A part of the tile is placed from the tile's own top left.
        let part = |e: &Element| {
            let tl = tile_el.rect;
            fr.px(e, dx + tl[0], dy + tl[1])
        };
        let hot = ui.input.mouse.is_some_and(|m| tile.contains(m));
        if hot {
            ui.draw.fill(tile, Corners::round(ui.u(5.0)), fade(t.accent, 0.18));
        }
        let wr = match wire_el {
            Some(w) if small => part(w),
            _ => tile,
        };
        let art = info_fo::class_art_name(u);
        let sp = crate::engine::api::palette::find_sprite(&crate::sfmt!("{}{}", art, l.name("WireSuffix")));
        let pic = wr;
        sprite_in(ui, sp, pic, 0xffffffff);
        if let Some(e) = name_el {
            let (n, _) = info::name_and_class(u);
            if let Some(n) = n {
                text(ui, part(e), n.text.trim(), t.text, Weight::Regular, Align::Centre);
            }
        }
        // As the one-object face: the shields green to red above, the hull blue below.
        let (h, s) = info_fo::hull_shields(u);
        if let (Some(e), Some((now, mx))) = (shield_el, s) {
            if mx > 0.0 {
                let f = (now / mx).clamp(0.0, 1.0);
                strip(ui, part(e), f, hull_colour(ui, f));
            }
        }
        if let Some(e) = hull_el {
            strip(ui, part(e), h, hull_bar_colour(ui, h));
        }
        if !fr.aligned {
            let id = ui.id(&crate::sfmt!("info.fo.unit{}", u));
            let (hot2, _, clicked) = ui.interact(id, tile);
            if hot2 {
                *tip = Tip::Unit(u, tile);
            }
            if clicked {
                crate::engine::api::sound::click();
                info::select(u, shift);
            }
        }
    }
    log_once("Group", units.len(), crate::sfmt!("info: Roots' panel ours: Group of {} ({})", units.len(), if small { "three rows" } else { "one row" }));
}
