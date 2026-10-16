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
use super::{buttons, field, hook, rd};
use alloc::string::String;
use alloc::vec::Vec;

/// `g_chat_display`: `ChatDisplay *`.
const CHAT: usize = 0x768de8;
const CHAT_LINES: usize = 0x04;
const NODE_TEXT: usize = 0x0c;
const NODE_COLOUR: usize = 0x18;
/// `ChatDisplay::RemoveExpiredText`, thiscall.
const REMOVE_EXPIRED: usize = 0x51d190;

/// The head of the text-input regions' list.
const REGIONS: usize = 0x7b0fbc;
const R_X: usize = 0x0;
const R_Y: usize = 0x4;
const R_BUF: usize = 0x10;
const R_PROMPT: usize = 0x130;
const R_END: usize = 0x134;
const R_CARET: usize = 0x138;
const R_FLAGS: usize = 0x144;
const F_SHOWN: u32 = 0x1;
const F_ACTIVE: u32 = 0x100;
const F_CHANGED: u32 = 0x200;
const R_BUF_MAX: usize = 0x118;

/// In `Armada_RenderFrame`: `call TextInput_Update`, just before the chat lines.
const SITE_TEXT_INPUT: usize = 0x598a07;
const TEXT_INPUT_UPDATE: usize = 0x65f150;

static mut TAKEN: bool = false;
static mut PATCHED: bool = false;

/// Patches the text entry's call in the mission frame, so that with the chat taken its
/// stock drawing is skipped. False (nothing patched) when the site is not stock.
pub fn install() -> bool {
    if !super::starts(REMOVE_EXPIRED, &[0x53, 0x56, 0x57, 0x8b, 0xf1]) {
        return false;
    }
    if !hook::patch_call(SITE_TEXT_INPUT, TEXT_INPUT_UPDATE, text_input_update as *const () as usize) {
        crate::log!("chat: the call at {:#x} is not to TextInput_Update: the chat stays stock", SITE_TEXT_INPUT);
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
            unsafe { *((r + R_FLAGS) as *mut u32) &= !F_CHANGED };
        }
        return;
    }
    let f: unsafe extern "C" fn() = unsafe { core::mem::transmute(TEXT_INPUT_UPDATE) };
    unsafe { f() }
}

/// `ChatDisplay::Render` as the mission frame calls it (`engine::ingame`): stock, or
/// with the chat taken only the expired lines dropped.
pub fn render_stock(chat: usize, stock: unsafe extern "thiscall" fn(usize)) {
    if unsafe { TAKEN } {
        let f: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(REMOVE_EXPIRED) };
        unsafe { f(chat) };
    } else {
        unsafe { stock(chat) };
    }
}

/// The chat lines, oldest first, each with its colour (0xAARRGGBB).
pub fn lines() -> Vec<(String, u32)> {
    let mut v = Vec::new();
    let chat = unsafe { rd::<usize>(CHAT) };
    let head = field(chat, CHAT_LINES);
    if head == 0 {
        return v;
    }
    let mut n = field(head, 0);
    while n != 0 && n != head && v.len() < 16 {
        let text = buttons::latin1(field(n, NODE_TEXT), 512);
        let c = field(n, NODE_COLOUR);
        let col = if c == 0 { 0xff00ff00 } else { buttons::colour(unsafe { rd(c) }) };
        v.push((text, col));
        n = field(n, 0);
    }
    v
}

fn regions() -> Vec<usize> {
    let mut v = Vec::new();
    let mut l = unsafe { rd::<usize>(REGIONS) };
    while l != 0 && v.len() < 16 {
        let r = field(l, 8);
        if r != 0 {
            v.push(r);
        }
        l = field(l, 0);
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
        let flags: u32 = unsafe { rd(r + R_FLAGS) };
        if flags & F_SHOWN == 0 {
            continue;
        }
        let (p, e, c): (usize, usize, usize) = unsafe { (rd(r + R_PROMPT), rd(r + R_END), rd(r + R_CARET)) };
        if p > R_BUF_MAX || e > R_BUF_MAX {
            continue;
        }
        let buf = buttons::latin1(r + R_BUF, R_BUF_MAX);
        let chars: Vec<char> = buf.chars().collect();
        let p = p.min(chars.len());
        let e = e.clamp(p, chars.len());
        v.push(Entry {
            prompt: chars[..p].iter().collect(),
            text: chars[p..e].iter().collect(),
            caret: c.clamp(p, e) - p,
            active: flags & F_ACTIVE != 0,
            x: unsafe { rd(r + R_X) },
            y: unsafe { rd(r + R_Y) },
        });
    }
    v
}
