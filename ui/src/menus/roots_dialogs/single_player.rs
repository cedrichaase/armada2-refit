//! Roots' Single Player (`SingleDlgProc`, adopted): the campaign selection, laid out as
//! Armada II's (`screens::single_player`, whose column and cards it shares).
//!
//! Roots' screen has four campaigns where Armada II has the tutorials and three: the
//! Allied, the Dominion, Species 8472 against the Borg and the Free Borg. As in Armada II,
//! the two top cards are hit areas of the procedure, the two bottom ones picture
//! `ShellButton`s, and no card has a label: the words appear in the middle of the screen on
//! hover, from four hidden `TextLabel`s, which are where the names come from (matched to
//! the content set's `[Campaign.*]` by their keys, `card_names`). A click is a click on a
//! point inside the card, after the pointer has rested there (decision 190).
//!
//! Over render3d's map the screen is the column: each campaign a cut of its own card's
//! art as its emblem (Roots' picture, captured from the dialog at run time and never
//! stored: the α, γ, V and B are the only marks Roots gives its campaigns, and a theme
//! cannot draw them; decision 251) and its name, and pointing at one moves the camera to
//! its view (the fleetops map's `federation`, `dominion`, `species8472`, `freeborg`).
//! Without the scene, the cards (2x2) with the same art. Load Game and Previous Menu are
//! the dialog's own buttons, by their labels.
use super::Screen;
use crate::menus::screens::single_player::{self as a2, ColumnRow};
use crate::widgets::Ui;
use alloc::string::String;
use alloc::vec::Vec;

/// Each card's art on Roots' screen (design rectangle, measured on the bench's picture
/// of the dialog), and a point inside it to click: the top two are the procedure's hit
/// areas; the bottom two are buttons, whose own rectangles the model gives (`point`).
const ART: [[i32; 4]; 4] = [[118, 78, 334, 334], [750, 78, 334, 334], [118, 487, 334, 334], [750, 487, 334, 334]];
const AT: [(i32, i32); 4] = [(285, 245), (916, 245), (285, 654), (916, 654)];

/// The card under the pointer last frame and the foot entry (to tell the stock screen
/// when it changes).
static mut HOVER: (usize, i32) = (0, -1);

/// Where card `i` is clicked: the stock button's centre when the model has one over the
/// card's art, else the measured point.
fn point(s: &Screen, i: usize) -> (i32, i32) {
    let a = ART[i];
    s.buttons()
        .into_iter()
        .filter(|c| c.text.is_empty() && c.rect[2] >= 200 && c.rect[3] >= 200)
        .map(|c| (c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2))
        .find(|&(x, y)| x >= a[0] && y >= a[1] && x < a[0] + a[2] && y < a[1] + a[3])
        .unwrap_or(AT[i])
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    let labels = s.labels();
    let camps = crate::content::get().campaigns;
    // A set without Roots' four campaigns leaves the screen to its picture.
    if labels.len() < 4 || camps.len() != ART.len() {
        return false;
    }
    let mut foot: Vec<_> = s.buttons().into_iter().filter(|c| !c.text.trim().is_empty() && c.rect[1] > 800).collect();
    foot.sort_by_key(|c| c.rect[0]);
    if foot.len() < 2 {
        return false;
    }
    let names = a2::card_names(&labels);
    let hints: [Option<String>; 4] = core::array::from_fn(|i| a2::lock_hint(i, &names));
    let at: [(i32, i32); 4] = core::array::from_fn(|i| point(s, i));
    let live = |i: usize| s.top && s.model.button_at(at[i].0, at[i].1).is_none_or(|c| c.enabled);
    let last = foot.len() - 1;
    let art = &mut |ui: &mut Ui, i: usize, dst: crate::gfx::R, tint: u32| s.mirror_part(ui, ART[i], dst, tint);
    let (out, foot_controls) = if crate::menus::scene::shown() {
        let shown = crate::menus::scene::chosen().or(crate::content::get().first_campaign_scene);
        let rows: [ColumnRow; 4] = core::array::from_fn(|i| ColumnRow {
            name: names[i],
            hint: hints[i].as_deref(),
            live: live(i),
            here: shown.is_some_and(|v| v == camps[i].scene),
        });
        let foot_entries = [(foot[last].text.trim(), s.top && foot[last].enabled), (foot[0].text.trim(), s.top && foot[0].enabled)];
        // The emblem: the card's own art, a rounded square, tinted as the name.
        let mut emblem = |ui: &mut Ui, i: usize, r: crate::gfx::R, c: crate::gfx::Color| {
            let k = 0.55 + 0.45 * (((c >> 8) & 0xff) as f32 / 255.0);
            let g = (255.0 * k) as u32;
            art(ui, i, r, 0xff00_0000 | g << 16 | g << 8 | g);
            ui.draw.frame(r, 1.0, crate::gfx::fade(ui.theme.panel_edge, 0.8));
        };
        let out = a2::campaign_column(ui, "FLEET OPERATIONS", "SINGLE PLAYER", &rows, &mut emblem, &foot_entries);
        if out.hover >= 0 {
            crate::menus::scene::choose(camps[out.hover as usize].scene);
        }
        (out, [foot[last], foot[0]])
    } else {
        crate::menus::screens::kit::backdrop(ui);
        let rows: [ColumnRow; 4] = core::array::from_fn(|i| ColumnRow { name: names[i], hint: hints[i].as_deref(), live: live(i), here: false });
        let foot_entries = [(foot[0].text.trim(), foot[0].enabled), (foot[last].text.trim(), foot[last].enabled)];
        let out = a2::campaign_cards(ui, "SINGLE PLAYER", &rows, &ART, art, foot_entries, s.top);
        (out, [foot[0], foot[last]])
    };
    if let Some(i) = out.clicked {
        if hints[i].is_some() {
            a2::locked_click(i);
        }
        s.click(at[i].0, at[i].1);
    }
    if let Some(i) = out.foot_clicked {
        s.click_control(foot_controls[i]);
    }
    // The stock screen hears of the hover (its label and its sound), and of the foot's
    // entry under the pointer, so its own button is lit when the press comes.
    let hover = match (out.hover, out.foot_hover) {
        (h, _) if h >= 0 => h,
        (_, Some(i)) => 10 + i as i32,
        _ => -1,
    };
    unsafe {
        if s.top && HOVER != (s.hwnd(), hover) {
            HOVER = (s.hwnd(), hover);
            let p = match hover {
                h if (0..4).contains(&h) => Some(at[h as usize]),
                h if h >= 10 => {
                    let c = foot_controls[(h - 10) as usize];
                    Some((c.rect[0] + c.rect[2] / 2, c.rect[1] + c.rect[3] / 2))
                }
                _ => None,
            };
            s.hover(p);
        }
    }
    true
}
