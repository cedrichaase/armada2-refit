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
//! panel to read. No vtable is changed.
//!
//! With no owner, `CursorInterface::Render` gives the device no cursor at all (each
//! owner's `Render` names its cursor in `iCursor+0x2c`). So while the pointer is over a
//! panel of ours, `render_component` names the bar's own arrow there just before the
//! cursor draws, as a stock panel would.
use super::{field as peek, hook as rthook, rd};
use crate::syms_hud::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// The address of `s` when hook `h` is ready, else 0 (a read of a part that stood down
/// finds nothing, and the stock game runs untouched).
fn at(h: a2engine::HookId, s: a2engine::Sym) -> usize {
    if eng().ready(h).is_ok() { eng().addr(s) } else { 0 }
}

// ---- the components ---------------------------------------------------------------

/// `g_pPopupPalette`: the button bar (`PopupPaletteImp *`), the address of the global.
pub fn palette_ptr() -> usize {
    at(hook::HUD_READS_COMPONENTS, sym::HUD_PALETTE_PTR)
}
/// `shipDisplay`: the info panel (an instance, not a pointer).
pub fn ship_display() -> usize {
    at(hook::HUD_READS_COMPONENTS, sym::HUD_SHIP_DISPLAY)
}
/// `mapRadar`: the minimap.
pub fn map_radar() -> usize {
    at(hook::HUD_READS_COMPONENTS, sym::HUD_MAP_RADAR)
}
/// `iCursor`: the `CursorInterface`.
fn cursor() -> usize {
    eng().addr(sym::HUD_CURSOR)
}

// `DisplayInterface+0x04` is the panel's rectangle, `{left, top, right, bottom}`,
// inclusive, in the HUD canvas (`canvas::CANVAS`) space every HUD rectangle is converted into
// (`cfgSCREEN_WIDTH` x `cfgSCREEN_HEIGHT`), which the engine scales to the back buffer
// per axis; `+0x16` is hidden (`SetIsHidden`; `RenderAll` calls `HiddenRender`); `+0x24`
// the cursor a panel shows while it owns the mouse. Vtable slots: `GetName` (+0x18),
// `Render` (+0x58), `CursorOver` (+0x70). `iCursor+0x24` is the component that owns the
// mouse; `+0x2c` the cursor sprite its `Render` named for this frame (cleared once
// handed to the device). `DisplayInterface::s_pRenderList` is one `std::list *` per
// view; `View_Record` is the view in use.

/// The button bar, or 0 outside a mission.
pub fn palette() -> usize {
    let g = palette_ptr();
    if g == 0 { 0 } else { unsafe { rd::<usize>(g) } }
}

/// A panel's rectangle in back-buffer pixels: the HUD canvas (`canvas::CANVAS`) rectangle scaled per
/// axis, as the engine draws it. (left, top, right, bottom), right and bottom exclusive.
pub fn panel_rect(c: usize, w: f32, h: f32) -> Option<[f32; 4]> {
    if c == 0 {
        return None;
    }
    let r: [i32; 4] = unsafe { rd(c + field::HUD_DI_RECT) };
    if r[2] <= r[0] || r[3] <= r[1] {
        return None;
    }
    let (sx, sy) = (w / super::canvas::CANVAS.0, h / super::canvas::CANVAS.1);
    Some([r[0] as f32 * sx, r[1] as f32 * sy, (r[2] + 1) as f32 * sx, (r[3] + 1) as f32 * sy])
}

/// Whether the panel is hidden: its own flag (`SetIsHidden`; the interface modes hide
/// them), or it is not in the render list of the view in use. The cinematic view
/// (`View_Record` 2, a mission's opening and the scripts' cutscenes) has a list of its
/// own with only the unit view, the overlay messages, the fade and the multiplayer and
/// pause boxes in it: seen on the bench, the first Federation mission's opening
/// showed the resources, the buttons and the minimap of ours over it until this.
pub fn hidden(c: usize) -> bool {
    c == 0 || flag_hidden(c) || !in_view(c)
}

/// The panel's own hidden flag alone (`+0x16`), whatever the view: what a toggle such
/// as `TogglePalette` flips.
pub fn flag_hidden(c: usize) -> bool {
    c != 0 && unsafe { rd::<u8>(c + field::HUD_DI_HIDDEN) } != 0
}

/// Whether `RenderAll` walks this component in the view in use (true when the list
/// cannot be read, so a misread hides nothing).
pub fn in_view(c: usize) -> bool {
    let list = components();
    list.is_empty() || list.contains(&c)
}

/// The component's name (`GetName`), e.g. "PopupPalette".
pub fn name(c: usize) -> &'static str {
    if c == 0 || eng().ready(hook::HUD_READS_COMPONENTS).is_err() {
        return "";
    }
    let f: unsafe extern "thiscall" fn(usize) -> *const u8 = unsafe { core::mem::transmute(rd::<usize>(rd::<usize>(c) + field::HUD_VT_NAME)) };
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

// The two sites: in `DisplayInterface::RenderAll`, the virtual call (vtable slot 0x58) of a
// visible component's `Render`; in `CursorInterface::RefreshOwner`, the virtual call (slot
// 0x70) of each component's `CursorOver` (profiles/gog-1.1/ui.hud.toml, `hud.panels`).

const MAX_TAKEN: usize = 16;
/// The components taken over: each by its address, or (`by_pointer`) by the address
/// of the global that points to it, read when it is asked about (the bar's
/// `g_pPopupPalette`).
static mut TAKEN: [(usize, bool); MAX_TAKEN] = [(0, false); MAX_TAKEN];
static mut OVER_UI: bool = false;
static mut INSTALLED: bool = false;

/// Patches both sites, each seven bytes of a register move and a call, replaced by a call of ours. False,
/// with nothing written, when either is not stock (`hud.panels` stands down).
pub fn install() -> bool {
    if eng().ready(hook::HUD_PANELS).is_err() {
        return false;
    }
    let (render, owner) = (eng().addr(sym::HUD_RENDER_CALL), eng().addr(sym::HUD_OWNER_CALL));
    let ok = patch(render, render_component as *const () as usize) && patch(owner, cursor_over as *const () as usize);
    unsafe { INSTALLED = ok };
    ok
}

fn patch(site: usize, to: usize) -> bool {
    // The seven bytes are stock: `ready` checked them against the profile.
    let stock: [u8; 7] = unsafe { rd(site) };
    let rel = (to as i64 - (site as i64 + 7)) as i32;
    let mut b = [0x8b, 0xce, 0xe8, 0, 0, 0, 0];
    b[3..7].copy_from_slice(&rel.to_le_bytes());
    rthook::patch_bytes(site, &stock, &b)
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

/// While the UI cannot draw (`gfx::drawing`: no device to draw with, so a panel of
/// ours would not show), nothing is taken: the stock components draw, and on Display8's
/// stub reach the screen through passthrough.
#[allow(static_mut_refs)]
fn taken(c: usize) -> bool {
    c != 0 && unsafe { INSTALLED } && crate::gfx::drawing() && unsafe { TAKEN.iter().any(|&(a, p)| a != 0 && if p { rd::<usize>(a) == c } else { a == c }) }
}

/// Whether the pointer is over a panel of ours (the last UI frame's claim): then no
/// stock component owns the mouse.
pub fn set_pointer_over_ui(on: bool) {
    unsafe { OVER_UI = on };
}

unsafe extern "thiscall" fn render_component(c: usize) {
    unsafe {
        if c == cursor() && rd::<usize>(cursor() + field::HUD_CURSOR_SPRITE) == 0 {
            // No cursor named this frame: no owner (the pointer is over a panel of
            // ours), or an owner we have taken, whose stock `Render`, which names it,
            // is skipped (the objectives box, which stays the owner while it shows: at
            // a mission's start nothing else showed a cursor). Show the owner's own
            // cursor, else the bar's arrow, as a stock panel would.
            let owner = rd::<usize>(cursor() + field::HUD_CURSOR_OWNER);
            if (owner == 0 && OVER_UI) || taken(owner) {
                let mut sprite = peek(owner, field::HUD_DI_CURSOR);
                if sprite == 0 {
                    sprite = peek(palette(), field::HUD_DI_CURSOR);
                }
                if sprite != 0 {
                    *((cursor() + field::HUD_CURSOR_SPRITE) as *mut usize) = sprite;
                }
            }
        }
        if c == cursor() && TOOLTIPS {
            return render_cursor_without_tooltip(c);
        }
        if taken(c) {
            return;
        }
        if c == cinematic_view() && super::unitview::enabled() {
            return super::unitview::render(c);
        }
        let f: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(rd::<usize>(c) + field::HUD_VT_RENDER));
        f(c)
    }
}

// ---- the tooltips at the pointer ---------------------------------------------------

// `iCursor+0x34`, `+0x38`: the short and the verbose tooltip, each a
/// `std::stringstream *` that `CursorInterface::DoTooltip` makes afresh for the
/// owner's tooltip once it has waited `s_tooltipDelay` / `s_verboseTooltipDelay`, NULL
/// when there is none. `CursorInterface::Render` draws the verbose one while
/// `m_tooltipState` is 0, else the short one, with `mDrawTooltip` at the pointer, 550
/// units wide on the HUD canvas (`canvas::CANVAS`) (stretched on a wide screen), and the verbose
/// one's framing (`mDrawFramingElement`).
// (`hud.tooltips`; `CursorInterface::m_tooltipState` is what the help button cycles, 0..2.)

static mut TOOLTIPS: bool = false;
static mut TIP: Option<alloc::string::String> = None;

/// Takes the engine's tooltips at the pointer (`[HUD] Tooltips=1`): each frame their
/// text is read for `tooltip()`, and the stock `Render` draws the cursor without them.
pub fn take_tooltips(on: bool) {
    unsafe { TOOLTIPS = on && eng().ready(hook::HUD_TOOLTIPS).is_ok() };
}

/// This frame's tooltip at the pointer, as the engine would have drawn it; taken once.
#[allow(static_mut_refs)]
pub fn tooltip() -> Option<alloc::string::String> {
    unsafe { TIP.take() }
}

/// `CursorInterface::Render` with its tooltips read and hidden from it: both fields are
/// emptied for the call and put back, so `DoTooltip` still owns and frees the streams.
#[allow(static_mut_refs)]
unsafe fn render_cursor_without_tooltip(c: usize) {
    unsafe {
        let (tip, tip_verbose) = (cursor() + field::HUD_CURSOR_TIP, cursor() + field::HUD_CURSOR_TIP_VERBOSE);
        let (short, verbose) = (rd::<usize>(tip), rd::<usize>(tip_verbose));
        let ss = if verbose != 0 && rd::<u32>(eng().addr(sym::HUD_TOOLTIP_STATE)) == 0 { verbose } else { short };
        TIP = if ss != 0 && !OVER_UI { super::msvc::stream_text(ss) } else { None };
        *(tip as *mut usize) = 0;
        *(tip_verbose as *mut usize) = 0;
        let f: unsafe extern "thiscall" fn(usize) = core::mem::transmute(rd::<usize>(rd::<usize>(c) + field::HUD_VT_RENDER));
        f(c);
        *(tip as *mut usize) = short;
        *(tip_verbose as *mut usize) = verbose;
    }
}

unsafe extern "thiscall" fn cursor_over(c: usize) -> bool {
    unsafe {
        // The objectives box stays the mouse's owner while it shows, ours or not, as
        // stock's modal box is: `GameKeysProcessInput` opens the chat line on Return
        // unless the owner is this box, and Return is also what closes it.
        if c == super::popups::objectives_box() && rd::<u8>(c + field::HUD_DI_HIDDEN) == 0 && taken(c) {
            let f: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd::<usize>(c) + field::HUD_VT_CURSOR_OVER));
            return f(c);
        }
        if OVER_UI || taken(c) {
            return false;
        }
        let f: unsafe extern "thiscall" fn(usize) -> bool = core::mem::transmute(rd::<usize>(rd::<usize>(c) + field::HUD_VT_CURSOR_OVER));
        f(c)
    }
}

/// The components of the view in use, in the order `RenderAll` draws them.
pub fn components() -> alloc::vec::Vec<usize> {
    let mut v = alloc::vec::Vec::new();
    let (record, lists) = (at(hook::HUD_READS_COMPONENTS, sym::HUD_VIEW_RECORD), at(hook::HUD_READS_COMPONENTS, sym::HUD_RENDER_LISTS));
    if record == 0 || lists == 0 {
        return v;
    }
    unsafe {
        let view = rd::<u32>(record) as usize;
        if view > 8 {
            return v;
        }
        let list = rd::<usize>(lists + 4 * view);
        let head = peek(list, 4);
        if head == 0 {
            return v;
        }
        let mut node = rd::<usize>(head);
        while node != head && node != 0 && v.len() < 64 {
            let c = peek(peek(node, 8), 4);
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
        let r: [i32; 4] = unsafe { rd(c + field::HUD_DI_RECT) };
        s.push_str(&crate::sfmt!(" {}{}[{},{},{},{}]", name(*c), if hidden(*c) { "(hidden)" } else { "" }, r[0], r[1], r[2], r[3]));
    }
    crate::log!("hud: {} components, in drawing order:{}", list.len(), s);
}

// ---- small reads every panel needs -------------------------------------------------

/// `g_cinematicView`: the unit view (an instance); 0 when the engine is not known.
pub fn cinematic_view() -> usize {
    at(hook::HUD_READS_CALLS, sym::HUD_CINEMATIC_VIEW)
}

/// `TextInput_IsActive`: the chat line has the keyboard. The engine's `InputNext`
/// skips every key binding then, and so do our keys.
pub fn typing() -> bool {
    let a = at(hook::HUD_READS_CALLS, sym::HUD_TEXT_INPUT_ACTIVE);
    if a == 0 {
        return false;
    }
    let f: unsafe extern "C" fn() -> usize = unsafe { core::mem::transmute(a) };
    unsafe { f() != 0 }
}

/// `CinematicView::IsPlayingVideo` on `g_cinematicView`: a button's click sound is
/// skipped while a video plays (`StandardButton::Simulate`).
pub fn video_playing() -> bool {
    let (a, c) = (at(hook::HUD_READS_CALLS, sym::HUD_IS_PLAYING_VIDEO), cinematic_view());
    if a == 0 || c == 0 {
        return false;
    }
    let f: unsafe extern "thiscall" fn(usize) -> bool = unsafe { core::mem::transmute(a) };
    unsafe { f(c) }
}

/// Every function above that is called, checked by its first bytes (the profile's
/// `hud.reads_calls`) before the HUD part patches anything.
pub fn functions_stock() -> bool {
    eng().ready(hook::HUD_READS_CALLS).is_ok() && eng().ready(hook::HUD_READS_COMPONENTS).is_ok()
}
