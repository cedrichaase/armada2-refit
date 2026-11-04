//! The chat lines (`ChatDisplay`, `g_chat_display`) and the text-entry line
//! (`TextInput_*`): not HUD components, the mission frame draws them itself after
//! `DisplayInterface::RenderAll` (README.md, "The mission frame"). Read by `hud::chat`
//! (HUD.md, "Chat and the text entry").
//!
//! - **The lines.** `ChatDisplay` keeps a `std::list<ChatLine>` at `+0x04` (its head
//!   node; a node is next `+0`, prev `+4`, the `ChatLine` from `+8`: a `std::string`
//!   whose characters' pointer is at the node's `+0xc`, then the line's colour as an
//!   `ST3D_Colour *` at the node's `+0x18`), at most ten, and a parallel list of the
//!   times each expires (`+0x0c`, `+0x10`; `Add` gives each `SCROLL_INTERVAL` ms).
//!   `ChatDisplay::Render` first drops the expired ones and then prints the rest down
//!   from (10, 150), so skipping it would keep every line forever: with the chat
//!   taken, `RemoveExpiredText` (the same drop, on its own) runs in its place.
//! - **The text entry.** `TextInput_CreateRegion` makes a region and links it into a
//!   list (its head at `0x7b0fbc`, which the map names `g_random+0x10`; a link is next
//!   `+0`, prev `+4`, the region `+8`). A region: its point (`+0`, `+4`), the prompt
//!   and the text in one buffer at `+0x10` (the prompt's length `+0x130`, the end
//!   `+0x134`, the caret `+0x138`, all from the buffer's start), and its flags at
//!   `+0x144` (1 shown, 0x100 taking keys: `TextInput_IsActive`, 0x200 changed).
//!   `TextInput_Update`, called by the mission frame just before `ChatDisplay::Render`,
//!   prints each shown region at its point in the player's colour and clears 0x200;
//!   the keys are `TextInput_ProcessInput`'s, from the engine's input, untouched by us.
use super::{buttons, field as fld, hook as patch, rd};
use crate::syms_panels::{field, hook, sym, value, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

fn ok() -> bool {
    eng().ready(hook::PANELS_CHAT).is_ok()
}

const F_SHOWN: u32 = value::PANELS_REGION_SHOWN as u32;
const F_ACTIVE: u32 = value::PANELS_REGION_ACTIVE as u32;
const F_CHANGED: u32 = value::PANELS_REGION_CHANGED as u32;
const R_BUF_MAX: usize = value::PANELS_REGION_BUF_MAX as usize;

static mut TAKEN: bool = false;
static mut PATCHED: bool = false;

/// Patches the text entry's call in the mission frame, so that with the chat taken its
/// stock drawing is skipped. False (nothing patched) when the site is not stock.
pub fn install() -> bool {
    if !ok() {
        return false;
    }
    let site = addr(sym::PANELS_TEXT_INPUT_SITE);
    if !patch::patch_call(site, addr(sym::PANELS_TEXT_INPUT_UPDATE), text_input_update as *const () as usize) {
        crate::log!("chat: the call at {:#x} is not to TextInput_Update: the chat stays stock", site);
        return false;
    }
    unsafe { PATCHED = true };
    true
}

/// Takes the chat lines and the text entry over (or gives them back).
pub fn set_taken(on: bool) {
    unsafe { TAKEN = on && PATCHED };
}

pub fn taken() -> bool {
    unsafe { TAKEN }
}

unsafe extern "C" fn text_input_update() {
    if unsafe { TAKEN } {
        // What stock does besides printing: the regions' "changed" flag is cleared.
        for r in regions() {
            unsafe { *((r + field::PANELS_REGION_FLAGS) as *mut u32) &= !F_CHANGED };
        }
        return;
    }
    let f: unsafe extern "C" fn() = unsafe { core::mem::transmute(addr(sym::PANELS_TEXT_INPUT_UPDATE)) };
    unsafe { f() }
}

/// `ChatDisplay::Render` as the mission frame calls it (`engine::ingame`): stock, or
/// with the chat taken only the expired lines dropped.
pub fn render_stock(chat: usize, stock: unsafe extern "thiscall" fn(usize)) {
    if unsafe { TAKEN } {
        let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(addr(sym::PANELS_REMOVE_EXPIRED)) };
        unsafe { f(chat) };
    } else {
        unsafe { stock(chat) };
    }
}

/// The chat lines, oldest first, each with its colour (0xAARRGGBB).
pub fn lines() -> Vec<(String, u32)> {
    let mut v = Vec::new();
    if !ok() {
        return v;
    }
    let chat = unsafe { rd::<usize>(addr(sym::PANELS_CHAT)) };
    let head = fld(chat, field::PANELS_CHAT_LINES);
    if head == 0 {
        return v;
    }
    let mut n = fld(head, 0);
    while n != 0 && n != head && v.len() < 16 {
        let text = buttons::latin1(fld(n, field::PANELS_NODE_TEXT), 512);
        let c = fld(n, field::PANELS_NODE_COLOUR);
        let col = if c == 0 { 0xff00ff00 } else { buttons::colour(unsafe { rd(c) }) };
        v.push((text, col));
        n = fld(n, 0);
    }
    v
}

fn regions() -> Vec<usize> {
    let mut v = Vec::new();
    if !ok() {
        return v;
    }
    let mut l = unsafe { rd::<usize>(addr(sym::PANELS_REGIONS)) };
    while l != 0 && v.len() < 16 {
        let r = fld(l, 8);
        if r != 0 {
            v.push(r);
        }
        l = fld(l, 0);
    }
    v
}

/// A text-entry line being shown.
pub struct Entry {
    /// The prompt (who the line goes to, e.g. "To All:"), and what has been typed.
    pub prompt: String,
    pub text: String,
    /// The caret, in characters into `text`.
    pub caret: usize,
    /// Taking keys (`TextInput_IsActive`).
    pub active: bool,
    pub x: i32,
    pub y: i32,
}

/// The shown text-entry lines.
pub fn entries() -> Vec<Entry> {
    let mut v = Vec::new();
    for r in regions() {
        let flags: u32 = unsafe { rd(r + field::PANELS_REGION_FLAGS) };
        if flags & F_SHOWN == 0 {
            continue;
        }
        let (p, e, c): (usize, usize, usize) = unsafe { (rd(r + field::PANELS_REGION_PROMPT), rd(r + field::PANELS_REGION_END), rd(r + field::PANELS_REGION_CARET)) };
        if p > R_BUF_MAX || e > R_BUF_MAX {
            continue;
        }
        let buf = buttons::latin1(r + field::PANELS_REGION_BUF, R_BUF_MAX);
        let chars: Vec<char> = buf.chars().collect();
        let p = p.min(chars.len());
        let e = e.clamp(p, chars.len());
        v.push(Entry {
            prompt: chars[..p].iter().collect(),
            text: chars[p..e].iter().collect(),
            caret: c.clamp(p, e) - p,
            active: flags & F_ACTIVE != 0,
            x: unsafe { rd(r + field::PANELS_REGION_X) },
            y: unsafe { rd(r + field::PANELS_REGION_Y) },
        });
    }
    v
}
