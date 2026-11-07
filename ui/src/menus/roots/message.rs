//! Roots' Warning prompt (`TYesNoForm`, PLAN-FLEETOPS.md, decisions 540 and 541): the one
//! form every confirmation uses (Exit to Desktop, End Mission, and the rest), as a dialog of
//! ours. The words are the executable's own buffers (`forms::message`), wrapped to the
//! dialog; the icon is a cut of the form's own picture, where Roots paints it, taken at run
//! time and cut down by area to the size we draw it at (never kept, never written); the
//! buttons are the form's, pressed as a player's click would press them. Enter is the first
//! button (Yes), Escape the last (No).
use crate::engine::api::forms::{self, Control, Form, Kind};
use crate::gfx::{self, Tex};
use crate::menus::screens::notice::{notice_window_icon, ICON};
use crate::widgets::Ui;
use alloc::string::String;
use alloc::vec::Vec;

const VK_RETURN: u8 = 0x0d;
const VK_ESCAPE: u8 = 0x1b;

/// The icon cut down for one form: (form, size in pixels, texture).
static mut ICON_TEX: Option<(usize, u32, Tex)> = None;
/// Where the icon lies in the printed picture, kept per form: x, y, w, h (0 w: none found).
static mut ICON_BOX: (usize, [i32; 4]) = (0, [0; 4]);

/// The icon's box in the picture: the bounding box of what is lit in the band left of the
/// question, between the title and the buttons, when it is about as wide as it is tall.
fn find_icon(bits: *const u8, w: i32, h: i32) -> Option<[i32; 4]> {
    // Left of x0 is the form's frame, a thin lit line.
    let (x0, x1, y0, y1) = (w * 7 / 100, w * 22 / 100, h * 17 / 100, h * 78 / 100);
    let (mut lx, mut ly, mut hx, mut hy) = (i32::MAX, i32::MAX, -1, -1);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = unsafe { core::slice::from_raw_parts(bits.add(((y * w + x) * 4) as usize), 3) };
            if p[0].max(p[1]).max(p[2]) > 24 {
                lx = lx.min(x);
                hx = hx.max(x);
                ly = ly.min(y);
                hy = hy.max(y);
            }
        }
    }
    let (bw, bh) = (hx - lx + 1, hy - ly + 1);
    if hx < 0 || bw < 16 || bh < 16 || bw * 10 < bh * 6 || bw * 6 > bh * 10 {
        return None;
    }
    Some([lx, ly, bw, bh])
}

/// The icon as a texture of `px` pixels square: the cut of the picture, averaged by area
/// (each target pixel is the mean of the source it covers, fractions weighted), its alpha
/// the brightness of the pixel (Roots paints the icon on black), colour kept straight.
fn icon_texture(form: usize, px: u32) -> Option<Tex> {
    if let Some((f, p, t)) = unsafe { ICON_TEX } {
        if f == form && p == px {
            return Some(t);
        }
    }
    let (bits, w, h, _) = forms::picture()?;
    let b = unsafe {
        if ICON_BOX.0 != form {
            ICON_BOX = (form, find_icon(bits, w, h).unwrap_or([0; 4]));
        }
        ICON_BOX.1
    };
    if b[2] == 0 {
        return None;
    }
    // A square cut around the box's middle, so the icon is not stretched.
    let side = b[2].max(b[3]);
    let (cx, cy) = (b[0] + b[2] / 2, b[1] + b[3] / 2);
    let (sx0, sy0) = (cx - side / 2, cy - side / 2);
    let mut out: Vec<u8> = alloc::vec![0; (px * px * 4) as usize];
    let k = side as f32 / px as f32;
    for ty in 0..px {
        for tx in 0..px {
            let (fx0, fx1) = (tx as f32 * k, (tx + 1) as f32 * k);
            let (fy0, fy1) = (ty as f32 * k, (ty + 1) as f32 * k);
            let (mut acc, mut wsum) = ([0f32; 3], 0f32);
            let mut y = libm::floorf(fy0) as i32;
            while (y as f32) < fy1 {
                let wy = (fy1.min(y as f32 + 1.0) - fy0.max(y as f32)).max(0.0);
                let mut x = libm::floorf(fx0) as i32;
                while (x as f32) < fx1 {
                    let wx = (fx1.min(x as f32 + 1.0) - fx0.max(x as f32)).max(0.0);
                    let (px_, py_) = (sx0 + x, sy0 + y);
                    if (0..w).contains(&px_) && (0..h).contains(&py_) {
                        let p = unsafe { core::slice::from_raw_parts(bits.add(((py_ * w + px_) * 4) as usize), 3) };
                        let wt = wx * wy;
                        for c in 0..3 {
                            acc[c] += p[c] as f32 * wt;
                        }
                        wsum += wt;
                    }
                    x += 1;
                }
                y += 1;
            }
            let o = ((ty * px + tx) * 4) as usize;
            if wsum > 0.0 {
                let m = [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum];
                let a = (m[0].max(m[1]).max(m[2]) * 3.0).min(255.0);
                for c in 0..3 {
                    out[o + c] = m[c].min(255.0) as u8;
                }
                out[o + 3] = a as u8;
            }
        }
    }
    let t = match unsafe { ICON_TEX } {
        Some((_, p0, t)) if p0 == px => t,
        _ => gfx::texture_dynamic_argb(px, px),
    };
    gfx::tex_update(t, out.as_ptr(), (px * 4) as usize, px, px);
    unsafe { ICON_TEX = Some((form, px, t)) };
    Some(t)
}

/// The prompt: false (the form's own picture then stands in) when the executable's words
/// are not there.
pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let Some(m) = forms::message() else { return false };
    let mut buttons: Vec<&Control> = f.controls.iter().filter(|c| c.visible && c.kind == Kind::Button && c.rect[2] > 0 && c.rect[3] > 0).collect();
    buttons.sort_by_key(|c| (c.rect[0], c.rect[1]));
    if buttons.is_empty() {
        return false;
    }
    static mut LOGGED: usize = 0;
    if unsafe { LOGGED } != f.hwnd {
        unsafe { LOGGED = f.hwnd };
        crate::log!("forms: prompt {:?}: {:?}, {} buttons", m.title, m.text, buttons.len());
    }
    let px = libm::ceilf(ui.u(ICON)) as u32;
    let icon = if f.top { icon_texture(f.hwnd, px.max(16)) } else { None };
    let title = if m.title.trim().is_empty() { String::from("WARNING") } else { m.title.to_uppercase() };
    let first = ui.input.key_pressed(VK_RETURN);
    let last = ui.input.key_pressed(VK_ESCAPE);
    let n = buttons.len();
    ui.draw.rect(ui.screen(), ui.theme.scrim);
    let mut press: Option<usize> = None;
    notice_window_icon(ui, &title, &m.text, icon, 560.0, n, &mut |ui, i, r| {
        let c = buttons[i];
        if i == 0 {
            ui.focus_ring(r, ui.u(4.0));
        }
        if ui.button(r, &c.text) && c.enabled {
            press = Some(i);
        }
    });
    if f.top && press.is_none() {
        if first {
            press = Some(0);
        } else if last {
            press = Some(n - 1);
        }
    }
    if let Some(i) = press {
        if buttons[i].enabled {
            forms::press(f, buttons[i]);
        }
    }
    true
}
