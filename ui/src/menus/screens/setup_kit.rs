//! What the setup and multiplayer specs share: the stock controls of a hosted dialog
//! drawn as `crate::widgets` in the shell theme, each acting only through the stock
//! control it stands for (MENUS.md, "Writing a screen spec").
//!
//! - `button`: a shell button or a window button, as our push button; a click is the
//!   stock control's own click.
//! - `cycle`: a `CycleButton` as a button showing its current choice; a click steps it
//!   as the stock click does.
//! - `edit`: a stock `Edit` as our text field. The stock edit keeps the text: what the
//!   player types is sent to it as keys (End, Backspaces to where the two differ, then
//!   the new characters), so its own limit and its parent's `EN_CHANGE` handling apply,
//!   and the field shows the edit's text again on the next frame.
//! - `list`: a stock list box as our list; a choice is the stock list's selection and
//!   its parent's notification.
//! - `form`: any small dialog (labels, edits, buttons) as one window of ours.
use crate::engine::shellui::Kind;
use crate::gfx::{Weight, R};
use crate::menus::model::{Class, Control, What};
use crate::menus::{actions, Screen};
use crate::rt::win::{WM_CHAR, WM_KEYDOWN};
use crate::widgets::{theme, Align, FieldEvent, ListEvent, State, Ui};
use alloc::string::String;
use alloc::vec::Vec;

pub fn is_label(c: &Control) -> bool {
    matches!(c.what, What::Shell(Kind::Label))
}

pub fn is_button(c: &Control) -> bool {
    matches!(c.what, What::Shell(k) if k != Kind::Label && k != Kind::Scroll) || matches!(c.what, What::Window(Class::Button | Class::OwnerDrawn))
}

pub fn is_edit(c: &Control) -> bool {
    matches!(c.what, What::Window(Class::Edit))
}

pub fn is_list(c: &Control) -> bool {
    matches!(c.what, What::Window(Class::List))
}

/// The visible controls of the dialog that `f` accepts, in the order it made them.
pub fn visible<'a>(s: &'a Screen, f: fn(&Control) -> bool) -> Vec<&'a Control> {
    s.model.controls.iter().filter(|c| c.visible && c.rect[2] > 0 && c.rect[3] > 0 && f(c)).collect()
}

/// A labelled control that shows, by its label (any case).
pub fn shown<'a>(s: &'a Screen, label: &str) -> Option<&'a Control> {
    s.model.controls.iter().find(|c| c.visible && c.text.eq_ignore_ascii_case(label))
}

/// The first visible control whose label starts with `prefix` (any case).
pub fn starting<'a>(s: &'a Screen, prefix: &str) -> Option<&'a Control> {
    let p = prefix.as_bytes();
    s.model.controls.iter().find(|c| c.visible && c.text.len() >= p.len() && c.text.as_bytes()[..p.len()].eq_ignore_ascii_case(p))
}

/// The backdrop every full-screen spec draws: the menu plate, or the shell's gradient.
pub fn backdrop(ui: &mut Ui) {
    let r = ui.screen();
    if crate::menus::backdrop::over_mission() {
        ui.draw.rect(r, ui.theme.scrim);
    } else {
        theme::backdrop(ui, r);
    }
}

/// The box a full-screen setup screen (the lobby, Game Setup) is laid out in: the
/// screen, no wider than 16:9 and centred, so the windows do not stretch across a
/// wider monitor.
pub fn wide_box(ui: &Ui) -> R {
    let r = ui.screen();
    let w = r.w.min(libm::floorf(r.h * 16.0 / 9.0));
    R::new(libm::floorf(r.x + (r.w - w) * 0.5), r.y, w, r.h)
}

/// A widget id unique to the control (its object or window), so two controls with the
/// same label (two "Computer AI" rows) are two widgets.
fn key(c: &Control, what: &str) -> alloc::string::String {
    crate::sfmt!("{}{:x}", what, c.key)
}

/// A stock button as our push button, labelled `label` (the control's own when None).
/// True on the frame it is clicked (the click is already queued).
pub fn button(ui: &mut Ui, s: &Screen, c: &Control, r: R, label: Option<&str>) -> bool {
    let text = label.unwrap_or(c.text.as_str());
    let live = s.top && c.enabled;
    if !live {
        theme::button(ui, r, text, State { off: !c.enabled, ..State::default() });
        return false;
    }
    let id = ui.id(&key(c, "b"));
    let (hot, held, clicked) = ui.interact(id, r);
    let st = State::of(ui, id, hot, held);
    theme::button(ui, r, text, st);
    if clicked {
        press(s, c);
    }
    clicked
}

/// The stock click on `c`. A shell button whose label is justified (Game Setup's
/// "Minimap Hidden", "Resources: Normal"; the Advanced panel's settings) is a small
/// square picture with its label beside it, and takes the pointer on the square only (a
/// click in the middle of "Minimap Hidden" did nothing). The square is at the end the
/// label is justified to: the left, or the right for a right-justified label (the
/// Advanced panel's left column). A button with a centred label is clicked in its
/// middle.
pub fn press(s: &Screen, c: &Control) {
    let half = c.rect[3] / 2;
    match (&c.what, c.align) {
        (What::Shell(Kind::Toggle | Kind::Button | Kind::Cycle), -1) => s.click(c.rect[0] + half, c.rect[1] + half),
        (What::Shell(Kind::Toggle | Kind::Button | Kind::Cycle), 1) => s.click(c.rect[0] + c.rect[2] - half, c.rect[1] + half),
        _ => s.click_control(c),
    }
}

/// A plain button of ours that clicks a design point of the dialog (a hit area the
/// screen's procedure tests itself).
pub fn point_button(ui: &mut Ui, s: &Screen, r: R, label: &str, x: i32, y: i32, enabled: bool) -> bool {
    if !(s.top && enabled) {
        theme::button(ui, r, label, State { off: !enabled, ..State::default() });
        return false;
    }
    let id = ui.id(&crate::sfmt!("p{}:{}", x, y));
    let (hot, held, clicked) = ui.interact(id, r);
    let st = State::of(ui, id, hot, held);
    theme::button(ui, r, label, st);
    if clicked {
        s.click(x, y);
    }
    clicked
}

/// A cycle (or toggle) button: a button labelled with the stock label, which says the
/// current choice ("Resources: Lots", "Warp Disabled"). A click steps it, through the
/// stock click. Not a drop-down: the stock button has no list, only a next choice.
pub fn cycle(ui: &mut Ui, s: &Screen, c: &Control, r: R) -> bool {
    button(ui, s, c, r, None)
}

/// A heading and a value line over a control, as the setup screens' groups have.
pub fn caption(ui: &mut Ui, r: R, s: &str) {
    let t = ui.theme;
    ui.text_in(r, s, t.small_size, Align::Left, t.text_dim, Weight::Regular);
}

/// The characters typed into a field this frame, sent on to the stock edit `h`: back to
/// where the old and the new text first differ, then the rest of the new text.
fn send_text(h: crate::rt::win::HWND, old: &str, new: &str) {
    let o: Vec<char> = old.chars().collect();
    let n: Vec<char> = new.chars().collect();
    let common = o.iter().zip(n.iter()).take_while(|(a, b)| a == b).count();
    actions::push(actions::Action::Key { h, msg: WM_KEYDOWN, wp: 0x23 }); // End
    for _ in common..o.len() {
        actions::push(actions::Action::Key { h, msg: WM_CHAR, wp: 8 });
    }
    for &ch in &n[common..] {
        if (ch as u32) < 256 && ch != '\r' && ch != '\n' {
            actions::push(actions::Action::Key { h, msg: WM_CHAR, wp: ch as usize });
        }
    }
}

/// A stock edit as our text field. The field shows the stock edit's text; typing goes
/// to the stock edit. Returns what the field did (Submit on Enter: the stock dialog
/// gets the Enter itself, as `IDOK`, as the stock edit would give it).
pub fn edit(ui: &mut Ui, s: &Screen, c: &Control, r: R) -> FieldEvent {
    let mut text = c.text.clone();
    if !(s.top && c.enabled) {
        theme::field(ui, r, State { off: !c.enabled, ..State::default() });
        let t = ui.theme;
        ui.text_in(r.inset(ui.u(t.pad * 0.7)), &text, t.text_size, Align::Left, t.text_dim, Weight::Regular);
        return FieldEvent::None;
    }
    let ev = ui.text_field(r, &key(c, "e"), &mut text, 0);
    if ev == FieldEvent::Changed && text != c.text {
        send_text(c.hwnd, &c.text, &text);
    }
    if ev == FieldEvent::Submit {
        // Enter in our field is the stock edit's Enter, given to its dialog as the modal
        // loop gives it (`IsDialogMessageA`): the default button, `IDOK`, which is how
        // the chat lines send and the small forms are confirmed.
        actions::push(actions::Action::Enter { h: c.hwnd });
    }
    ev
}

/// A stock list box (with string items) as our list. A click selects (and a double
/// click, or Enter, activates) the stock way.
pub fn list(ui: &mut Ui, s: &Screen, c: &Control, r: R) -> ListEvent {
    let items: Vec<&str> = c.items.iter().map(|x| x.as_str()).collect();
    let mut sel = if c.selected >= 0 { Some(c.selected as usize) } else { None };
    let ev = ui.list_text(r, &key(c, "l"), &items, &mut sel);
    if s.top && c.enabled && ev != ListEvent::None {
        if let Some(i) = sel {
            actions::push(actions::Action::Select { h: c.hwnd, combo: false, index: i as i32, double: ev == ListEvent::Activated });
        }
    }
    ev
}

/// A shell `ComboBox` (Game Setup's player rows, Map Select's filter) as our drop-down.
/// A choice is what the player's two clicks would be: on the stock button, which opens
/// its pull-down, and on the item in it (`engine::shellcombo::item_point`).
pub fn combo(ui: &mut Ui, s: &Screen, c: &Control, r: R) -> bool {
    if !(s.top && c.enabled) || c.items.is_empty() {
        theme::field(ui, r, State { off: !c.enabled, ..State::default() });
        let t = ui.theme;
        let pad = ui.u(t.pad * 0.7);
        ui.text_in(R::new(r.x + pad, r.y, r.w - 2.0 * pad, r.h), &c.text, t.text_size, Align::Left, if c.enabled { t.text } else { t.text_dim }, Weight::Regular);
        return false;
    }
    let items: Vec<&str> = c.items.iter().map(|x| x.as_str()).collect();
    let mut sel = c.selected.max(0) as usize;
    if !ui.dropdown(r, &key(c, "d"), &items, &mut sel) || sel as i32 == c.selected {
        return false;
    }
    choose(s, c, sel as i32)
}

/// Chooses item `i` of the shell `ComboBox` `c` with the stock clicks.
pub fn choose(s: &Screen, c: &Control, i: i32) -> bool {
    let Some((x, y)) = crate::engine::shellcombo::item_point(c.key, i) else { return false };
    // `item_point` is in the dialog's client space; clicks are in design space.
    let (ox, oy) = (s.model.rect[0], s.model.rect[1]);
    s.click_control(c);
    s.click(ox + x, oy + y);
    true
}

/// A shell toggle. Its label says its state ("Fixed Placement", "Warp Disabled"), and
/// its on flag does not always mean the setting is on (Warp Enabled is a toggle that
/// is off), so it is drawn as a button with that label, as `cycle`.
pub fn check(ui: &mut Ui, s: &Screen, c: &Control, r: R) -> bool {
    button(ui, s, c, r, None)
}

/// A piece of the stock dialog's picture (a colour swatch, a map preview) drawn in `r`,
/// passing a click on it to the stock dialog at the same place. True when clicked.
pub fn picture(ui: &mut Ui, s: &Screen, design: [i32; 4], r: R, live: bool) -> bool {
    s.mirror_part(ui, design, r, 0xffffffff);
    if !(live && s.top) {
        return false;
    }
    let id = ui.id(&crate::sfmt!("pic{},{}", design[0], design[1]));
    let (hot, held, clicked) = ui.interact(id, r);
    if hot || held {
        let t = ui.theme;
        ui.draw.frame(r.inset(-ui.u(2.0)), ui.u(2.0).max(1.0), if held { t.accent } else { crate::gfx::fade(t.accent, 0.7) });
    }
    if clicked {
        // The same place in the design, scaled from where the player clicked.
        let (px, py) = ui.input.mouse.unwrap_or((r.x, r.y));
        let fx = ((px - r.x) / r.w.max(1.0)).clamp(0.0, 0.999);
        let fy = ((py - r.y) / r.h.max(1.0)).clamp(0.0, 0.999);
        s.click(design[0] + (fx * design[2] as f32) as i32, design[1] + (fy * design[3] as f32) as i32);
    }
    clicked
}

/// A panel with no title: returns the room inside it.
pub fn panel(ui: &mut Ui, r: R) -> R {
    ui.panel(r);
    theme::panel_inner(ui, r)
}

/// Drops controls that sit on the same rectangle with the same label as one before
/// (the stock screens make some twice and show one).
pub fn dedupe(v: &mut Vec<&Control>) {
    let mut out: Vec<&Control> = Vec::new();
    for c in v.iter() {
        if !out.iter().any(|o| o.rect == c.rect && o.text == c.text) {
            out.push(c);
        }
    }
    *v = out;
}

/// A label's text, in the theme's text colour (the game's colour is GDI's on its art).
pub fn text(ui: &mut Ui, r: R, s: &str, dim: bool) {
    let t = ui.theme;
    ui.text_in(r, s, t.text_size, Align::Left, if dim { t.text_dim } else { t.text }, Weight::Regular);
}

/// Text wrapped to `r`'s width from its top; returns the height used.
pub fn para(ui: &mut Ui, r: R, s: &str, size: f32, c: u32) -> f32 {
    let px = ui.u(size);
    ui.draw.text_wrapped(s, px, r.w, r.x, r.y, c, Weight::Regular)
}

/// Any small dialog as one window of ours, centred: its labels as text, top to bottom
/// in their design order, its edits as fields, its lists as lists, and its buttons in a
/// row at the bottom. For the screens that are a few labels, edits and buttons (Manual
/// IP, the game's name, a password, the GameSpy account screens, waiting for the
/// network). `title` is the window's title when the dialog has no label to take it
/// from. False when the dialog has none of these (the generic screen draws it).
pub fn form(ui: &mut Ui, s: &Screen, title: &str) -> bool {
    let mut labels = visible(s, |c| is_label(c) && !c.text.trim().is_empty());
    let mut edits = visible(s, is_edit);
    let mut lists = visible(s, |c| is_list(c));
    let mut buttons = visible(s, |c| is_button(c) && !c.text.trim().is_empty() && !c.text.eq_ignore_ascii_case("NULL"));
    if labels.is_empty() && edits.is_empty() && buttons.is_empty() {
        return false;
    }
    let by_pos = |a: &&Control, b: &&Control| (a.rect[1], a.rect[0]).cmp(&(b.rect[1], b.rect[0]));
    labels.sort_by(by_pos);
    edits.sort_by(by_pos);
    lists.sort_by(by_pos);
    buttons.sort_by(|a, b| (a.rect[0], a.rect[1]).cmp(&(b.rect[0], b.rect[1])));
    // A full-screen dialog gets the backdrop; a small one shows over what is under it.
    if s.model.rect[2] >= crate::engine::dialogs::DESIGN_W {
        backdrop(ui);
    } else {
        ui.draw.rect(ui.screen(), ui.theme.scrim);
    }
    let t = ui.theme;
    // The topmost label is the title when it sits above everything else.
    let mut title_s = String::from(title);
    if let Some(first) = labels.first() {
        let top_other = edits.iter().chain(buttons.iter()).map(|c| c.rect[1]).min().unwrap_or(i32::MAX);
        if title.is_empty() && first.rect[1] < top_other && labels.len() > 1 {
            title_s = first.text.clone();
            labels.remove(0);
        }
    }
    let w = ui.u(640.0).min(ui.w - ui.u(32.0));
    let row = ui.u(t.row_h + 4.0);
    let list_h = ui.u(260.0);
    let mut h = ui.u(t.title_size + 2.0 * t.pad + 8.0) + ui.u(t.pad * 2.0) + ui.u(t.button_h + t.gap);
    // Labels may wrap: allow two lines each.
    h += labels.len() as f32 * row * 1.6 + edits.len() as f32 * (row + ui.u(t.gap)) + lists.len() as f32 * (list_h + ui.u(t.gap));
    let r = crate::widgets::Anchor::Centre.place(ui.screen(), w, h.min(ui.h - ui.u(32.0)), 0.0);
    let inner = ui.window(r, &title_s);
    let mut y = inner.y;
    // Labels and edits in the order they stand on the stock screen.
    let mut items: Vec<&Control> = labels.iter().chain(edits.iter()).chain(lists.iter()).copied().collect();
    items.sort_by(by_pos);
    for c in items {
        if is_label(c) {
            let used = para(ui, R::new(inner.x, y, inner.w, row), &c.text, t.text_size, t.text);
            y += used.max(ui.u(t.text_size)) + ui.u(t.gap);
        } else if is_edit(c) {
            let er = R::new(inner.x, y, inner.w, ui.u(t.row_h + 4.0));
            edit(ui, s, c, er);
            y += er.h + ui.u(t.gap);
        } else {
            let lr = R::new(inner.x, y, inner.w, list_h);
            list(ui, s, c, lr);
            y += lr.h + ui.u(t.gap);
        }
    }
    let _ = y;
    // The buttons, right-aligned along the bottom, in their stock order left to right.
    let bh = ui.u(t.button_h);
    let bw = ui.u(180.0);
    let gap = ui.u(t.gap);
    let n = buttons.len() as f32;
    let total = n * bw + (n - 1.0).max(0.0) * gap;
    let mut x = inner.right() - total.min(inner.w);
    let bw = if total > inner.w { (inner.w - (n - 1.0) * gap) / n } else { bw };
    for c in buttons {
        let br = R::new(x, inner.bottom() - bh, bw, bh);
        x += bw + gap;
        if matches!(c.what, What::Shell(Kind::Cycle)) {
            cycle(ui, s, c, br);
        } else {
            button(ui, s, c, br, None);
        }
    }
    true
}
