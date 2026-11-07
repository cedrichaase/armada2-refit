//! The status line about the craft under the pointer (HUD.md, "The status line under
//! the pointer"): the strip `ShipDisplay` draws about the craft the pointer is over in the
//! 3D view (`SDInfoBar`), recorded where the engine would draw it and drawn by the UI
//! (`hud::hover`, `[HUD] Hover=1`).
//!
//! `ShipDisplay::AlwaysSimulate` puts the craft under the pointer at `ShipDisplay+0x1ec`;
//! `ShipDisplay::Render` draws the strip for it with one call, `SDInfoBar::Render(craft, x,
//! y)`, at the strip's place for the panel's face (a pair of `+0x1b8` picked by `+0x1f4`,
//! from `infoBarOffset_<face>` in the HUD's layout), and clears `+0x1ec`. Stock puts the
//! place inside the info panel; Fleet Operations: Roots moves it to the top of the screen
//! (its layouts' `infoBarOffset_1`), where the strip reads as a status line for the craft
//! the pointer is over. The call is the same on Roots: FleetOpsHook leaves both
//! `SDInfoBar::Render` and its call in `ShipDisplay::Render` stock, in the file and in
//! memory.
//!
//! **The hook** takes that one call (`hud.hover`). Ours asks each of the strip's children
//! for its answer, as the strip's `Render` would draw it, and records it; the strip is
//! not drawn. The children (`SDInfoBar` members): three `TextAndSprite`s, shields
//! (`+0xc`), hull (`+0x10`) and crew (`+0x14`), each an icon and a text; the special
//! energy (`+0x18`, an `EnergyText`, drawn only while the craft's special energy
//! `+0x168` is at least 1); and the craft's name (`NameText`, `+0x20`) when its class has
//! the byte `+0x3c4` set, else its class (`ClassText`, `+0x1c`). Each is handed the craft
//! (`SetCurrentObject`, vtable `+0x20`), then answers its colour and string
//! (`GetInformation`, `+0x24`) and its icon's tint (`GetCurrentColor`, `+0x2c`). Where:
//! a `TextAndSprite` draws its icon at `+0x10c` and its text at `+0x11c`, a
//! `ClassText` or `NameText` its text at `+0x104` (each a RECT relative to the place),
//! all plus the origin its parent answers (`DisplayInterface` vtable `+0x64`, a POINT,
//! read through the child's `+0x4`: the info panel's corner for the group faces, none for
//! the one-unit face, so the sum is the same place); the text is justified by `+0x68`.
//! Everything is on the HUD's canvas (1600 by 1200 units, scaled per axis to the screen),
//! as the engine converts it when it draws.
//!
//! While the key is off (or the HUD stands aside) the call goes to the strip as before;
//! with `[HUD] LayoutLog=1` and the key off, the hook is installed only to log what the
//! engine draws (the bench's reference).
use super::{field as peek, hook as rthook, rd};
use crate::syms_hover::{field, hook, sym, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// Which child a part of the readout comes from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Name,
    Class,
    Shields,
    Hull,
    Crew,
    Energy,
}

/// One child's answer: its string (the game's text, decoded), colour (0..1), its text
/// rectangle (left, top, right, bottom, canvas units, origin included) and justification, and
/// its icon (an `ST3D_Sprite`, or 0), the icon's rectangle and tint.
#[derive(Clone)]
pub struct Part {
    pub kind: Kind,
    pub text: String,
    pub rgb: [f32; 3],
    pub rect: [i32; 4],
    pub justify: i32,
    pub font: usize,
    /// The font's line height (`ST3D_Font::GetHeight`), 0 without a font.
    pub font_h: f32,
    pub icon: usize,
    pub icon_rect: [i32; 4],
    pub icon_rgb: [f32; 3],
}

/// The strip for one frame: the craft, the place it was asked at, and its parts.
#[derive(Clone)]
pub struct Readout {
    pub craft: usize,
    pub at: (i32, i32),
    pub parts: Vec<Part>,
}

static mut INSTALLED: bool = false;
static mut OURS: bool = false;
static mut ASIDE: bool = false;
static mut LOG: bool = false;
static mut FRAME: Option<Readout> = None;

/// Puts the hook in (once), the first time it is wanted: `ours` draws the strip as the
/// UI's, `log` only logs what the engine draws. False when the site is not stock.
pub fn set(ours: bool, log: bool) -> bool {
    unsafe {
        if !INSTALLED && (ours || log) {
            if eng().ready(hook::HUD_HOVER).is_err() {
                return false;
            }
            let (site, render) = (eng().addr(sym::HOVER_INFOBAR_CALL), eng().addr(sym::HOVER_INFOBAR_RENDER));
            if !rthook::patch_call(site, render, infobar as *const () as usize) {
                crate::log!("hover: ShipDisplay::Render's strip call is not stock, the strip stays the engine's");
                return false;
            }
            INSTALLED = true;
        }
        OURS = ours && INSTALLED;
        LOG = log && INSTALLED;
        INSTALLED
    }
}

/// Whether the strip is the UI's this frame (the key on and the hook in).
pub fn ours() -> bool {
    unsafe { OURS }
}

/// The HUD of ours stands aside (under the engine's box): the strip goes back to the
/// engine until it comes back.
pub fn stand_aside(aside: bool) {
    unsafe { ASIDE = aside };
}

/// This frame's strip, taken once (None when the engine drew none: no craft under the
/// pointer, the info panel hidden, or the strip the engine's).
#[allow(static_mut_refs)]
pub fn take() -> Option<Readout> {
    unsafe { FRAME.take() }
}

#[inline(always)]
fn vcall(c: usize, slot: usize) -> usize {
    unsafe { rd::<usize>(rd::<usize>(c) + slot) }
}

fn rect_at(a: usize) -> [i32; 4] {
    unsafe { rd::<[i32; 4]>(a) }
}

/// The HUD's origin the children add to their places (`DisplayInterface` vtable +0x64).
fn origin(c: usize) -> (i32, i32) {
    let di = peek(c, field::HOVER_ST_DISPLAY());
    if di == 0 {
        return (0, 0);
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const i32 = core::mem::transmute(vcall(di, field::HOVER_DI_ORIGIN()));
        let p = f(di);
        if p.is_null() { (0, 0) } else { (*p, *p.add(1)) }
    }
}

/// A text child's answer for `craft`: its string and colour, and (with an icon) the
/// icon's tint.
fn answer(c: usize, craft: usize, icon: bool) -> (String, [f32; 3], [f32; 3]) {
    unsafe {
        let set: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(vcall(c, field::HOVER_VT_SET_OBJECT()));
        set(c, craft);
        let mut out = [0u32; 4];
        let info: unsafe extern "thiscall" fn(usize, *mut u32) -> usize = core::mem::transmute(vcall(c, field::HOVER_VT_INFORMATION()));
        info(c, out.as_mut_ptr());
        let rgb = [f32::from_bits(out[0]), f32::from_bits(out[1]), f32::from_bits(out[2])];
        let text = crate::text::game::display(&super::buttons::latin1(out[3] as usize, 256));
        let mut tint = [1.0f32; 3];
        if icon && peek(c, field::HOVER_TS_SPRITE()) != 0 {
            let cur: unsafe extern "thiscall" fn(usize, *mut f32) -> usize = core::mem::transmute(vcall(c, field::HOVER_VT_CURRENT_COLOR()));
            cur(c, tint.as_mut_ptr());
        }
        (text, rgb, tint)
    }
}

fn shift(r: [i32; 4], dx: i32, dy: i32) -> [i32; 4] {
    [r[0] + dx, r[1] + dy, r[2] + dx, r[3] + dy]
}

fn font(c: usize) -> usize {
    let p = peek(c, field::HOVER_ST_FONT());
    if p == 0 { 0 } else { unsafe { rd::<usize>(p) } }
}

fn font_height(f: usize) -> f32 {
    if f == 0 {
        return 0.0;
    }
    let g: unsafe extern "thiscall" fn(usize) -> f32 = unsafe { core::mem::transmute(eng().addr(sym::HOVER_FONT_HEIGHT)) };
    let h = unsafe { g(f) };
    if h.is_finite() && h > 0.0 { h } else { 0.0 }
}

/// A `TextAndSprite` child (icon and text) at the place `x`, `y`.
fn with_icon(kind: Kind, c: usize, craft: usize, x: i32, y: i32) -> Option<Part> {
    if c == 0 {
        return None;
    }
    let (ox, oy) = origin(c);
    let (text, rgb, icon_rgb) = answer(c, craft, true);
    Some(Part {
        kind,
        text,
        rgb,
        rect: shift(rect_at(c + field::HOVER_TS_TEXT_RECT()), x + ox, y + oy),
        justify: unsafe { rd::<i32>(c + field::HOVER_ST_JUSTIFY()) },
        font: font(c),
        font_h: font_height(font(c)),
        icon: peek(c, field::HOVER_TS_SPRITE()),
        icon_rect: shift(rect_at(c + field::HOVER_TS_SPRITE_RECT()), x + ox, y + oy),
        icon_rgb,
    })
}

/// A `ClassText` or `NameText` child at the place `x`, `y`.
fn plain(kind: Kind, c: usize, craft: usize, x: i32, y: i32) -> Option<Part> {
    if c == 0 {
        return None;
    }
    let (ox, oy) = origin(c);
    let (text, rgb, _) = answer(c, craft, false);
    Some(Part {
        kind,
        text,
        rgb,
        rect: shift(rect_at(c + field::HOVER_CT_RECT()), x + ox, y + oy),
        justify: unsafe { rd::<i32>(c + field::HOVER_ST_JUSTIFY()) },
        font: font(c),
        font_h: font_height(font(c)),
        icon: 0,
        icon_rect: [0; 4],
        icon_rgb: [1.0; 3],
    })
}

/// What the strip would draw for `craft` at `x`, `y`, in its order: shields, hull, crew,
/// the special energy while the craft has some, then the name or the class.
fn read(ib: usize, craft: usize, x: i32, y: i32) -> Readout {
    let mut parts = Vec::new();
    for (k, off) in [(Kind::Shields, field::HOVER_IB_SHIELD()), (Kind::Hull, field::HOVER_IB_HULL()), (Kind::Crew, field::HOVER_IB_CREW())] {
        parts.extend(with_icon(k, peek(ib, off), craft, x, y));
    }
    let energy = unsafe { rd::<f32>(craft + field::HOVER_OBJ_ENERGY()) };
    if energy as i32 > 0 {
        parts.extend(with_icon(Kind::Energy, peek(ib, field::HOVER_IB_ENERGY()), craft, x, y));
    }
    let cls = peek(craft, field::HOVER_OBJ_CLASS());
    let named = cls != 0 && unsafe { rd::<u8>(cls + field::HOVER_CLASS_NAMED()) } != 0;
    if named {
        parts.extend(plain(Kind::Name, peek(ib, field::HOVER_IB_NAME()), craft, x, y));
    } else {
        parts.extend(plain(Kind::Class, peek(ib, field::HOVER_IB_CLASS()), craft, x, y));
    }
    Readout { craft, at: (x, y), parts }
}

/// `SDInfoBar::Render(Craft *, int, int)`, thiscall, as `ShipDisplay::Render` calls it.
unsafe extern "thiscall" fn infobar(ib: usize, craft: usize, x: i32, y: i32) {
    unsafe {
        let ours = OURS && !ASIDE;
        if craft != 0 && ib != 0 && (ours || LOG) {
            let r = read(ib, craft, x, y);
            if LOG || ours {
                log(&r);
            }
            if ours {
                FRAME = Some(r);
                return;
            }
        }
        let f: unsafe extern "thiscall" fn(usize, usize, i32, i32) = core::mem::transmute(eng().addr(sym::HOVER_INFOBAR_RENDER));
        f(ib, craft, x, y);
    }
}

/// UI.log has each strip once while it changes (the craft, a value or a place), at
/// most 80 lines: "hover: craft 0x.. at 625,50 (drawn by the engine): Shields "100"
/// 0.40,0.60,1.00 text 30,60..80,80 j0 icon 8,60..28,80 | ...".
fn log(r: &Readout) {
    static mut LAST: u64 = 0;
    static mut LINES: u32 = 0;
    let mut key: u64 = r.craft as u64 ^ ((r.at.0 as u64) << 32) ^ ((r.at.1 as u64) << 48);
    for p in &r.parts {
        for b in p.text.bytes() {
            key = key.wrapping_mul(131).wrapping_add(b as u64);
        }
        key = key.wrapping_mul(31).wrapping_add((p.rgb[0] * 255.0) as u64 + ((p.rgb[1] * 255.0) as u64) * 256 + ((p.rgb[2] * 255.0) as u64) * 65536);
    }
    unsafe {
        if key == LAST || LINES >= 80 {
            return;
        }
        LAST = key;
        LINES += 1;
    }
    let mut line = crate::sfmt!("hover: craft {:#x} at {},{} ({}):", r.craft, r.at.0, r.at.1, if unsafe { OURS && !ASIDE } { "ours" } else { "drawn by the engine" });
    for p in &r.parts {
        line.push_str(&crate::sfmt!(
            " | {:?} {:?} {:.2},{:.2},{:.2} text {},{}..{},{} j{} font {:#x} h {:.1}",
            p.kind,
            p.text,
            p.rgb[0],
            p.rgb[1],
            p.rgb[2],
            p.rect[0],
            p.rect[1],
            p.rect[2],
            p.rect[3],
            p.justify,
            p.font,
            p.font_h
        ));
        if p.icon != 0 {
            let r = p.icon_rect;
            line.push_str(&crate::sfmt!(" icon {} {},{}..{},{} tint {:.2},{:.2},{:.2}", super::sprite::name(p.icon), r[0], r[1], r[2], r[3], p.icon_rgb[0], p.icon_rgb[1], p.icon_rgb[2]));
        }
    }
    crate::log!("{}", line);
}
