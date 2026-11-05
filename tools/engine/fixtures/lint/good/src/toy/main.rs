// Uses the table: sym::COMMENTED_OUT is in a comment and must be ignored.
const FNV: u32 = 0x811c9dc5;
const NOT_ADDR: u32 = 0x401000; // not an address
const NOT_OFFSET: usize = 0x10; // not an offset: a flag
use crate::syms::{field as off};
fn pe(base: usize) -> u32 {
    unsafe { *((base + 0x3c) as *const u32) } // PE header
}
fn run() {
    let _ = (sym::UPDATE_FN, sym::TICK_CALL, sym::TICK_JMP, sym::SPEED_PATCH, sym::SPEED_LABEL, sym::NEW_FN);
    let _ = (sym::TOY_VT, sym::TOY_GLOBAL, sym::TOY_SLOT, sym::TOY_SCAN);
    let _ = (hook::SPEED, hook::LOOK, hook::FIND);
    let _ = (field::TOY_HP, value::TOY_WIDTH, off::TOY_HP);
    let s = "// not a comment";
}
