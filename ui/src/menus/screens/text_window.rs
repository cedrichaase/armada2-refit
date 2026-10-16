//! The text windows: the lobby's help (`do_helpWindow`) and the licence agreement
//! (`do_licenseAgreement`, shown on a first start only). A window with the text in our
//! scroll area and the stock buttons below it. The text is read from the stock control
//! that holds it (the longest edit, static or label of the screen), whole: the model
//! keeps only the first 4 KB of a control's text, which would cut a licence short.
use super::common::*;
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Class, Control, What};
use crate::menus::Screen;
use crate::rt::win::*;
use crate::widgets::{Anchor, Ui};
use alloc::string::String;
use alloc::vec::Vec;

fn full_text(h: HWND) -> String {
    let n = unsafe { SendMessageA(h, WM_GETTEXTLENGTH, 0, 0) }.clamp(0, 200_000) as usize;
    let mut b = alloc::vec![0u8; n + 1];
    let got = unsafe { SendMessageA(h, WM_GETTEXT, n + 1, b.as_mut_ptr() as isize) }.clamp(0, n as isize) as usize;
    b[..got].iter().filter(|&&c| c != b'\r').map(|&c| char::from(c)).collect()
}

pub fn draw(ui: &mut Ui, s: &Screen) -> bool {
    // The control that holds the text.
    let holder = s
        .model
        .controls
        .iter()
        .filter(|c| c.visible && matches!(c.what, What::Window(Class::Edit) | What::Window(Class::Static) | What::Shell(Kind::Label)) && c.text.len() > 40)
        .max_by_key(|c| c.rect[2] * c.rect[3]);
    let Some(holder) = holder else { return false };
    let text = match holder.what {
        What::Window(_) => full_text(holder.hwnd),
        _ => holder.text.clone(),
    };
    let mut btns: Vec<&Control> = s.model.controls.iter().filter(|c| c.visible && !c.text.is_empty() && (is_button(c) || matches!(c.what, What::Window(Class::Button))) && c.key != holder.key).collect();
    btns.sort_by_key(|c| (c.rect[0], c.rect[1]));

    backdrop_for(ui, s);
    let t = ui.theme;
    let r = Anchor::Centre.place(ui.screen(), ui.u(1000.0), ui.u(900.0).min(ui.screen().h * 0.94), 0.0);
    let name = title(s).map_or("", |c| if c.key != holder.key { c.text.as_str() } else { "" });
    let inner = ui.window(r, name);
    let (body, foot) = inner.take_top(inner.h - ui.u(44.0 + 16.0));
    let size = ui.u(t.text_size);
    let view_w = body.w - ui.u(12.0 + 4.0) - ui.u(16.0);
    let (_, th) = crate::text::measure_wrapped(crate::text::font(), &text, size, view_w);
    ui.scroll_area(body, "text", th + ui.u(16.0), |ui, c| {
        ui.draw.text_wrapped(&text, size, view_w, c.x + ui.u(8.0), c.y + ui.u(8.0), t.text, Weight::Regular);
    });
    let mut hot = None;
    let n = btns.len().max(1) as f32;
    let (bw, gap) = (ui.u(200.0), ui.u(20.0));
    let mut x = foot.x + (foot.w - (n * bw + (n - 1.0) * gap)) * 0.5;
    for c in btns {
        stock_button(ui, s, c, R::new(x, foot.bottom() - ui.u(44.0), bw, ui.u(44.0)), &mut hot);
        x += bw + gap;
    }
    hover_stock(s, hot);
    true
}
