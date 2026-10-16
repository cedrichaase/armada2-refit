//! The HUD's panels and the panel switch (HUD.md, "The panel switch").
//!
//! The in-mission HUD is a list of `DisplayInterface` components. Two walks over that
//! list make a panel visible and give it the mouse, and each is patched at the one
//! instruction where it calls into a component:
//!
//! - **Drawing.** `DisplayInterface::RenderAll` walks the render list of the view
//!   (`s_pRenderList[view]`, a `std::list` of `DisplayViewInfo *`, the component at
//!   `+0x4`) and calls each visible component's `Render` (vtable `+0x58`). That call is
//!   re-pointed at `render_component`, which skips a component we have taken over.
//! - **The mouse.** Which component the mouse belongs to is `iCursor+0x24`, the
//!   cursor's owner, set each frame by `CursorInterface::RefreshOwner` (from its
//!   `AlwaysSimulate`): unless a component has locked the cursor (`+0x28`), the last
//!   visible component in the render list whose `CursorOver` (vtable `+0x70`) says
//!   the pointer is over it. Only the owner gets its `FocusSimulate` (where a panel
//!   reads clicks), its buttons' hover and clicks (`StandardButton::Simulate` checks
//!   the owner), its tooltips, and supplies the cursor shape. That call is re-pointed
//!   at `cursor_over`, which says no for a component we have taken over, and no for
//!   every component while the pointer is over a panel of ours, so no stock panel
//!   (nor the 3D view, `cOverViewImp`, the owner of last resort) reacts under it.
//!
//! The component's simulate steps are not touched, so its state stays current for our
//! panel to read. No vtable is changed (HUD.asi recognises panels by their vtables).
//!
//! With no owner, `CursorInterface::Render` gives the device no cursor at all (each
//! owner's `Render` names its cursor in `iCursor+0x2c`). So while the pointer is over a
//! panel of ours, `render_component` names the bar's own arrow there just before the
//! cursor draws, as a stock panel would.
use super::{field, hook, rd};

// ---- the components ---------------------------------------------------------------

/// `g_pPopupPalette`: the button bar (`PopupPaletteImp *`).
pub const PALETTE_PTR: usize = 0x763c00;
/// `shipDisplay`: the info panel (an instance, not a pointer).
pub const SHIP_DISPLAY: usize = 0x763968;
/// `mapRadar`: the minimap.
pub const MAP_RADAR: usize = 0x763810;
/// `iCursor`: the `CursorInterface`.
pub const CURSOR: usize = 0x7643a8;

/// `DisplayInterface+0x04`: the panel's rectangle, `{left, top, right, bottom}`,
/// inclusive, in the 1600 x 1200 space every HUD rectangle is converted into
/// (`cfgSCREEN_WIDTH` x `cfgSCREEN_HEIGHT`), which the engine scales to the back
/// buffer per axis.
const DI_RECT: usize = 0x04;
/// `DisplayInterface+0x16`: hidden (`SetIsHidden`); `RenderAll` calls `HiddenRender`.
const DI_HIDDEN: usize = 0x16;
/// `DisplayInterface+0x24`: the cursor a panel shows while it owns the mouse.
const DI_CURSOR: usize = 0x24;
/// Vtable slots: `GetName` (+0x18), `Render` (+0x58), `CursorOver` (+0x70).
const VT_NAME: usize = 0x18;
const VT_RENDER: usize = 0x58;
const VT_CURSOR_OVER: usize = 0x70;

/// `iCursor+0x24`: the component that owns the mouse; `+0x2c`: the cursor sprite its
/// `Render` named for this frame (cleared once handed to the device).
const CURSOR_OWNER: usize = CURSOR + 0x24;
const CURSOR_SPRITE: usize = CURSOR + 0x2c;

/// `DisplayInterface::s_pRenderList`: one `std::list *` per view; `View_Record` is the
/// view in use.
const RENDER_LISTS: usize = 0x764fd8;
const VIEW_RECORD: usize = 0x76b610;

/// The button bar, or 0 outside a mission.
pub fn palette() -> usize {
    unsafe { rd::<usize>(PALETTE_PTR) }
}

/// A panel's rectangle in back-buffer pixels: the 1600 x 1200 rectangle scaled per
/// axis, as the engine draws it. (left, top, right, bottom), right and bottom exclusive.
pub fn panel_rect(c: usize, w: f32, h: f32) -> Option<[f32; 4]> {
    if c == 0 {
        return None;
    }
    let r: [i32; 4] = unsafe { rd(c + DI_RECT) };
    if r[2] <= r[0] || r[3] <= r[1] {
        return None;
    }
    let (sx, sy) = (w / 1600.0, h / 1200.0);
    Some([r[0] as f32 * sx, r[1] as f32 * sy, (r[2] + 1) as f32 * sx, (r[3] + 1) as f32 * sy])
}

/// Whether the panel is hidden (`SetIsHidden`; a cinematic hides them all).
pub fn hidden(c: usize) -> bool {
    c == 0 || unsafe { rd::<u8>(c + DI_HIDDEN) } != 0
}

/// The component's name (`GetName`), e.g. "PopupPalette".
pub fn name(c: usize) -> &'static str {
    if c == 0 {
        return "";
    }
    let f: unsafe extern "thiscall" fn(usize) -> *const u8 = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(c) + VT_NAME)) };
    unsafe { cstr_static(f(c)) }
}

/// A NUL-terminated string in the game's memory, read as Latin-1 if it is not UTF-8.
///
/// # Safety
/// `p` is NULL or points to a NUL-terminated string that outlives the use.
pub unsafe fn cstr_static(p: *const u8) -> &'static str {
    if p.is_null() {
        return "";
    }
    let mut n = 0;
    while n < 256 && unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, n) }).unwrap_or("?")
}

// ---- the panel switch -------------------------------------------------------------

/// In `DisplayInterface::RenderAll`: `mov edx,[esi]; mov ecx,esi; call [edx+0x58]`,
/// the call of a visible component's `Render`.
const SITE_RENDER: usize = 0x51abf7;
const RENDER_STOCK: [u8; 7] = [0x8b, 0x16, 0x8b, 0xce, 0xff, 0x52, 0x58];
/// In `CursorInterface::RefreshOwner`: `mov ecx,esi; mov edx,[esi]; call [edx+0x70]`,
/// the call of each component's `CursorOver`.
const SITE_OWNER: usize = 0x505316;
const OWNER_STOCK: [u8; 7] = [0x8b, 0xce, 0x8b, 0x16, 0xff, 0x52, 0x70];

const MAX_TAKEN: usize = 16;
/// The components taken over: each by its address, or (`by_pointer`) by the address
/// of the global that points to it, read when it is asked about (the bar's
/// `g_pPopupPalette`).
static mut TAKEN: [(usize, bool); MAX_TAKEN] = [(0, false); MAX_TAKEN];
static mut OVER_UI: bool = false;
static mut INSTALLED: bool = false;

/// Patches both sites, each `mov ecx,esi; call <ours>` in the same seven bytes. False,
/// with nothing written, when either is not stock.
pub fn install() -> bool {
    let at = |site: usize| unsafe { core::slice::from_raw_parts(site as *const u8, 7) };
    if at(SITE_RENDER) != RENDER_STOCK || at(SITE_OWNER) != OWNER_STOCK {
        crate::log!("hud: RenderAll's or RefreshOwner's call into the components is not stock: the HUD stays stock");
        return false;
    }
    let ok = patch(SITE_RENDER, &RENDER_STOCK, render_component as *const () as usize)
        && patch(SITE_OWNER, &OWNER_STOCK, cursor_over as *const () as usize);
    unsafe { INSTALLED = ok };
    ok
}

fn patch(site: usize, stock: &[u8; 7], to: usize) -> bool {
    let rel = (to as i64 - (site as i64 + 7)) as i32;
    let mut b = [0x8b, 0xce, 0xe8, 0, 0, 0, 0];
    b[3..7].copy_from_slice(&rel.to_le_bytes());
    hook::patch_bytes(site, stock, &b)
}

pub fn installed() -> bool {
    unsafe { INSTALLED }
}

/// Takes a component over (or gives it back): while taken, it is neither drawn nor
/// given the mouse. `at` is the component, or with `by_pointer` a global holding it.
#[allow(static_mut_refs)]
pub fn set_taken(at: usize, by_pointer: bool, on: bool) {
    if at == 0 {
        return;
    }
    let t = unsafe { &mut TAKEN };
    let have = t.iter().position(|&x| x == (at, by_pointer));
    match (on, have) {
        (true, None) => {
            if let Some(slot) = t.iter().position(|&x| x.0 == 0) {
                t[slot] = (at, by_pointer);
            }
        }
        (false, Some(i)) => t[i] = (0, false),
        _ => {}
    }
}

/// Whether a component is one we have taken over.
pub fn is_taken(c: usize) -> bool {
    taken(c)
}

#[allow(static_mut_refs)]
fn taken(c: usize) -> bool {
    c != 0 && unsafe { TAKEN.iter().any(|&(a, p)| a != 0 && if p { rd::<usize>(a) == c } else { a == c }) }
}

/// Whether the pointer is over a panel of ours (the last UI frame's claim): then no
/// stock component owns the mouse.
pub fn set_pointer_over_ui(on: bool) {
    unsafe { OVER_UI = on };
}

unsafe extern "thiscall" fn render_component(c: usize) {
    unsafe {
        if c == CURSOR && OVER_UI && rd::<usize>(CURSOR_OWNER) == 0 && rd::<usize>(CURSOR_SPRITE) == 0 {
            // No owner, so no cursor: show the bar's arrow, as a stock panel would.
            let arrow = field(palette(), DI_CURSOR);
            if arrow != 0 {
                *(CURSOR_SPRITE as *mut usize) = arrow;
            }
        }
        if taken(c) {
            return;
        }
        let f: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(rd::<usize>(c) + VT_RENDER));
        f(c)
    }
}

unsafe extern "thiscall" fn cursor_over(c: usize) -> bool {
    unsafe {
        if OVER_UI || taken(c) {
            return false;
        }
        let f: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd::<usize>(c) + VT_CURSOR_OVER));
        f(c)
    }
}

/// The components of the view in use, in the order `RenderAll` draws them.
pub fn components() -> alloc::vec::Vec<usize> {
    let mut v = alloc::vec::Vec::new();
    unsafe {
        let view = rd::<u32>(VIEW_RECORD) as usize;
        if view > 8 {
            return v;
        }
        let list = rd::<usize>(RENDER_LISTS + 4 * view);
        let head = field(list, 4);
        if head == 0 {
            return v;
        }
        let mut node = rd::<usize>(head);
        while node != head && node != 0 && v.len() < 64 {
            let c = field(field(node, 8), 4);
            if c != 0 {
                v.push(c);
            }
            node = rd::<usize>(node);
        }
    }
    v
}

/// Logs the components and their rectangles once per mission, for HUD.md's inventory.
pub fn log_components() {
    let list = components();
    let mut s = alloc::string::String::new();
    for c in &list {
        let r: [i32; 4] = unsafe { rd(c + DI_RECT) };
        s.push_str(&crate::sfmt!(" {}{}[{},{},{},{}]", name(*c), if hidden(*c) { "(hidden)" } else { "" }, r[0], r[1], r[2], r[3]));
    }
    crate::log!("hud: {} components, in drawing order:{}", list.len(), s);
}

// ---- small reads every panel needs -------------------------------------------------

/// `TextInput_IsActive`: the chat line has the keyboard. The engine's `InputNext`
/// skips every key binding then, and so do our keys.
const TEXT_INPUT_ACTIVE: usize = 0x65f120;
/// `CinematicView::IsPlayingVideo` on `g_cinematicView`: a button's click sound is
/// skipped while a video plays (`StandardButton::Simulate`).
const IS_PLAYING_VIDEO: usize = 0x4e6350;
const CINEMATIC_VIEW: usize = 0x763758;

pub fn typing() -> bool {
    let f: unsafe extern "C" fn() -> usize = unsafe { core::mem::transmute(TEXT_INPUT_ACTIVE) };
    unsafe { f() != 0 }
}

pub fn video_playing() -> bool {
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(IS_PLAYING_VIDEO) };
    unsafe { f(CINEMATIC_VIEW) }
}

/// Every function above that is called, checked by its first bytes before the HUD
/// part patches anything.
pub fn functions_stock() -> bool {
    super::starts(TEXT_INPUT_ACTIVE, &[0xa1, 0xbc, 0x0f, 0x7b, 0x00])
        && super::starts(IS_PLAYING_VIDEO, &[0x8b, 0x51, 0x24, 0x33, 0xc0])
}
