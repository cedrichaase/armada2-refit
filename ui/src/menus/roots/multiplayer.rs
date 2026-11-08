//! Roots' Multiplayer Connection (`TMultiplayerConnectionForm`) as Armada II's
//! (`screens::multiplayer`): the player's name as a field, one wide button for each way to
//! connect (Roots' captions, so a Tunngle build says Tunngle and the others GameSpy), and
//! Previous Menu. Roots binds Internet's action when the form opens, so the buttons are
//! pressed at their own rectangles and do whatever Roots bound.
//!
//! With `Online.asi` loaded and its entry free for a screen of ours (`Online_Available`),
//! the first button, the dead internet service's, is *Internet - Online*, as IPX's is on
//! Armada II (online/README.md, "On Fleet Operations: Roots"): it tells Online the entry is
//! chosen and presses Roots' own Manual IP button, whose address field then takes a join
//! code. Every other button tells Online it is not.
use super::kit;
use crate::engine::api::forms::Form;
use crate::menus::screens::multiplayer::connection_layout;
use crate::widgets::Ui;

const KINDS: &[&str] = &["buttonInternetConnect", "buttonManualIPConnect", "buttonLANTCPConnect"];

/// Online.asi's entry, when a screen of ours may choose it (`online/online.c`).
pub(super) mod online {
    static mut AVAILABLE: usize = 1;
    static mut CHOOSE: usize = 1;
    static mut CHOSEN: usize = 1;

    fn find(slot: &mut usize, name: &str) -> usize {
        if *slot == 1 {
            *slot = crate::settings::find_export(name);
        }
        *slot
    }

    /// Online is loaded and the connection screen is ours to give it a button.
    pub fn available() -> bool {
        let f = find(unsafe { &mut *core::ptr::addr_of_mut!(AVAILABLE) }, "Online_Available");
        f != 0 && unsafe { core::mem::transmute::<usize, extern "C" fn() -> i32>(f)() } != 0
    }

    /// The entry chosen (or not) for the connection about to be made.
    pub fn choose(on: bool) {
        let f = find(unsafe { &mut *core::ptr::addr_of_mut!(CHOOSE) }, "Online_Choose");
        if f != 0 {
            unsafe { core::mem::transmute::<usize, extern "C" fn(i32)>(f)(on as i32) };
        }
    }

    /// Internet - Online is the entry chosen.
    pub fn chosen() -> bool {
        let f = find(unsafe { &mut *core::ptr::addr_of_mut!(CHOSEN) }, "Online_Chosen");
        f != 0 && unsafe { core::mem::transmute::<usize, extern "C" fn() -> i32>(f)() } != 0
    }
}

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(name), Some(back)) = (kit::shown(f, "editGameUsername"), kit::shown(f, "buttonBack")) else { return false };
    let kinds: alloc::vec::Vec<_> = KINDS.iter().filter_map(|n| kit::shown(f, n)).collect();
    if kinds.is_empty() {
        return false;
    }
    let manual = kit::shown(f, "buttonManualIPConnect");
    let online = manual.is_some() && online::available();
    let (er, kind_rs, nav_rs) = connection_layout(ui, "MULTIPLAYER", "Name", kinds.len(), 1);
    kit::edit(ui, f, name, er);
    for (c, r) in kinds.into_iter().zip(kind_rs) {
        if online && c.name == "buttonInternetConnect" {
            if ui.scope("btnOnline", |ui| ui.button(r, "Internet - Online")) && f.top {
                if let Some(m) = manual.filter(|m| m.enabled) {
                    online::choose(true);
                    crate::log!("forms: Internet - Online: pressing {} of {}", m.name, f.class);
                    kit::press(f, m);
                }
            }
        } else if ui.scope(&crate::sfmt!("btn{}", c.name), |ui| ui.button(r, kit::caption(c))) && f.top && c.enabled {
            online::choose(false);
            kit::press(f, c);
        }
    }
    if kit::button(ui, f, back, nav_rs[0]) {
        online::choose(false);
    }
    true
}
