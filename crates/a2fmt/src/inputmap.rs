//! The key map: `Input.map` in the game directory, which binds the engine's actions to
//! keys and mouse buttons (`README.md`, "Input.map").
//!
//! Plain text. A line whose first non-blank character is `#` is a comment; a block is
//! `name {`, one binding line per input, and `}`. A binding line is a sign, a device and
//! an input: `+` means the input must be held, `-` that it must not be, and a modifier
//! the block does not name may be either. The device is `keyboard` or `mouse`; a
//! keyboard input is a key by its position (a set-1 scancode, [`KEYS`]), so `A` is the
//! key where A is on a US keyboard, whatever the layout says.
//!
//! [`InputMap::parse`] reads every block with the comment above it (what the action
//! does, in the file's own words) and the headings it sits under; [`InputMap::rewrite`]
//! gives a new text with some blocks' bindings replaced and every other byte kept.
//! Nothing here panics on bad input: a line it cannot read is skipped.

use alloc::string::String;
use alloc::vec::Vec;

/// A modifier key: Control, Shift, Alt, in that order everywhere here.
pub const MODIFIERS: [&str; 3] = ["Control", "Shift", "Alt"];

/// What a block asks of a modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mod {
    /// Not named: held or not.
    Any,
    /// `+`: held.
    On,
    /// `-`: not held.
    Off,
}

/// One binding line of a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// `+` (true) or `-`.
    pub plus: bool,
    /// `keyboard`, `mouse`, or whatever else the line names.
    pub device: String,
    /// The input's name as written (`F1`, `GreyHome`, `*Esc`, `LeftBtn`).
    pub input: String,
}

/// One action: a block of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    /// The comment lines directly above the block (no blank line between), `#` and
    /// surrounding blanks removed, in order. Empty when it has none.
    pub comment: Vec<String>,
    /// The last banner heading above it (a comment line framed by `#`s, under a line of
    /// `#`s): `Camera navigation`.
    pub section: String,
    /// The last `### ` (or `## `) heading after that banner, or "".
    pub group: String,
    pub lines: Vec<Line>,
    /// Byte range of the block's body in the text: from the start of the line after
    /// `name {` to the start of the `}` line.
    pub body: (usize, usize),
}

/// What a keyboard action is bound to, when it is one key and modifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The key's name in the file (`F1`, `GreyHome`).
    pub key: String,
    /// Control, Shift, Alt.
    pub mods: [Mod; 3],
}

/// A key the engine names: its name in the file, its set-1 scancode (`0x100` added for
/// an extended key, as Windows reports it in a key message's lParam bit 24), and a
/// name for people.
pub struct Key {
    pub name: &'static str,
    pub scan: u16,
    pub label: &'static str,
}

const fn k(name: &'static str, scan: u16, label: &'static str) -> Key {
    Key { name, scan, label }
}

/// Every key a binding may name here. The names are the ones stock's file uses, and
/// the letters, digits and function keys it leaves out follow the same pattern. The
/// keypad's keys are named by what they do with Num Lock off (`Home` is keypad 7,
/// `UpArrow` keypad 8) and the separate cluster's by `Grey` (`GreyHome`, `GreyUpArrow`):
/// stock's AI menu puts its three-by-three grid on the keypad that way. Keys stock's file
/// never names and whose name cannot be told from the pattern (Page Up and Page Down of
/// the cluster, `;`, `'`, the right-hand modifiers) are not offered.
pub const KEYS: &[Key] = &[
    k("Esc", 0x01, "Esc"),
    k("One", 0x02, "1"),
    k("Two", 0x03, "2"),
    k("Three", 0x04, "3"),
    k("Four", 0x05, "4"),
    k("Five", 0x06, "5"),
    k("Six", 0x07, "6"),
    k("Seven", 0x08, "7"),
    k("Eight", 0x09, "8"),
    k("Nine", 0x0a, "9"),
    k("Zero", 0x0b, "0"),
    k("Minus", 0x0c, "-"),
    k("Equal", 0x0d, "="),
    k("Backspace", 0x0e, "Backspace"),
    k("Tab", 0x0f, "Tab"),
    k("Q", 0x10, "Q"),
    k("W", 0x11, "W"),
    k("E", 0x12, "E"),
    k("R", 0x13, "R"),
    k("T", 0x14, "T"),
    k("Y", 0x15, "Y"),
    k("U", 0x16, "U"),
    k("I", 0x17, "I"),
    k("O", 0x18, "O"),
    k("P", 0x19, "P"),
    k("LeftBracket", 0x1a, "["),
    k("RightBracket", 0x1b, "]"),
    k("Enter", 0x1c, "Enter"),
    k("A", 0x1e, "A"),
    k("S", 0x1f, "S"),
    k("D", 0x20, "D"),
    k("F", 0x21, "F"),
    k("G", 0x22, "G"),
    k("H", 0x23, "H"),
    k("J", 0x24, "J"),
    k("K", 0x25, "K"),
    k("L", 0x26, "L"),
    k("BackQuote", 0x29, "`"),
    k("BackSlash", 0x2b, "\\"),
    k("Z", 0x2c, "Z"),
    k("X", 0x2d, "X"),
    k("C", 0x2e, "C"),
    k("V", 0x2f, "V"),
    k("B", 0x30, "B"),
    k("N", 0x31, "N"),
    k("M", 0x32, "M"),
    k("Comma", 0x33, ","),
    k("Period", 0x34, "."),
    k("Slash", 0x35, "/"),
    k("Space", 0x39, "Space"),
    k("CAPSLock", 0x3a, "Caps Lock"),
    k("F1", 0x3b, "F1"),
    k("F2", 0x3c, "F2"),
    k("F3", 0x3d, "F3"),
    k("F4", 0x3e, "F4"),
    k("F5", 0x3f, "F5"),
    k("F6", 0x40, "F6"),
    k("F7", 0x41, "F7"),
    k("F8", 0x42, "F8"),
    k("F9", 0x43, "F9"),
    k("F10", 0x44, "F10"),
    k("Pause", 0x45, "Pause"),
    k("Home", 0x47, "Num 7"),
    k("UpArrow", 0x48, "Num 8"),
    k("PageUp", 0x49, "Num 9"),
    k("GreyMinus", 0x4a, "Num -"),
    k("LeftArrow", 0x4b, "Num 4"),
    k("Keypad5", 0x4c, "Num 5"),
    k("RightArrow", 0x4d, "Num 6"),
    k("GreyPlus", 0x4e, "Num +"),
    k("End", 0x4f, "Num 1"),
    k("DownArrow", 0x50, "Num 2"),
    k("PageDown", 0x51, "Num 3"),
    k("F11", 0x57, "F11"),
    k("F12", 0x58, "F12"),
    k("KeypadEnter", 0x11c, "Num Enter"),
    k("GreyHome", 0x147, "Home"),
    k("GreyUpArrow", 0x148, "Up"),
    k("GreyLeftArrow", 0x14b, "Left"),
    k("GreyRightArrow", 0x14d, "Right"),
    k("GreyEnd", 0x14f, "End"),
    k("GreyDownArrow", 0x150, "Down"),
    k("GreyInsert", 0x152, "Insert"),
    k("GreyDelete", 0x153, "Delete"),
];

/// The key named `name` (without regard to ASCII case).
pub fn key_by_name(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.name.eq_ignore_ascii_case(name))
}

/// The key of scancode `scan` (`0x100` added when extended).
pub fn key_by_scan(scan: u16) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.scan == scan)
}

/// Whether `name` is one of the three modifiers.
pub fn is_modifier(name: &str) -> bool {
    MODIFIERS.iter().any(|m| m.eq_ignore_ascii_case(name))
}

fn trim_start(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

/// Whether a comment line is a frame of `#`s (and blanks) only.
fn is_rule(c: &str) -> bool {
    !c.is_empty() && c.bytes().all(|b| b == b'#' || b == b' ' || b == b'\t')
}

/// A banner title, `# Camera navigation #`, without its `#`s.
fn banner(line: &str) -> Option<String> {
    let t = line.trim();
    if t.len() < 3 || !t.starts_with('#') || !t.ends_with('#') || is_rule(t) {
        return None;
    }
    let inner = t.trim_matches('#').trim();
    if inner.is_empty() { None } else { Some(String::from(inner)) }
}

/// The text of a comment line without its leading `#`s.
fn comment_text(line: &str) -> String {
    String::from(trim_start(line).trim_start_matches('#').trim())
}

/// `name {` at the start of a line: the name.
fn block_open(line: &str) -> Option<&str> {
    let t = line.trim();
    let name = t.strip_suffix('{')?.trim_end();
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(name)
}

/// `+ keyboard F1`.
fn binding_line(line: &str) -> Option<Line> {
    let t = line.trim();
    let plus = match t.as_bytes().first()? {
        b'+' => true,
        b'-' => false,
        _ => return None,
    };
    let mut it = t[1..].split([' ', '\t']).filter(|w| !w.is_empty());
    let device = String::from(it.next()?);
    let input = String::from(it.next()?);
    if it.next().is_some() {
        return None;
    }
    Some(Line { plus, device, input })
}

/// The lines of `text` with the byte offset each starts at (`\n`, `\r\n`).
fn line_starts(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut at = 0;
    for l in text.split('\n') {
        out.push((at, l.strip_suffix('\r').unwrap_or(l)));
        at += l.len() + 1;
    }
    out
}

/// Every block of an `Input.map`, in file order.
#[derive(Clone, Debug, Default)]
pub struct InputMap {
    pub actions: Vec<Action>,
}

impl InputMap {
    pub fn parse(text: &str) -> InputMap {
        let lines = line_starts(text);
        let mut actions = Vec::new();
        let mut section = String::new();
        let mut group = String::new();
        // The comment paragraph since the last blank line, block or heading.
        let mut para: Vec<String> = Vec::new();
        let mut prev_rule = false;
        let mut i = 0;
        while i < lines.len() {
            let (_, l) = lines[i];
            let t = l.trim();
            i += 1;
            if t.is_empty() {
                para.clear();
                prev_rule = false;
                continue;
            }
            if t.starts_with('#') {
                if is_rule(t) {
                    prev_rule = true;
                    para.clear();
                    continue;
                }
                if prev_rule && let Some(b) = banner(t) {
                    section = b;
                    group.clear();
                    para.clear();
                    continue;
                }
                prev_rule = false;
                let c = comment_text(t);
                if t.starts_with("##") && !t.starts_with("####") {
                    group = c;
                    para.clear();
                    continue;
                }
                para.push(c);
                continue;
            }
            prev_rule = false;
            let Some(name) = block_open(l) else {
                para.clear();
                continue;
            };
            let start = lines.get(i).map_or(text.len(), |x| x.0);
            let mut binds = Vec::new();
            let mut end = text.len();
            while i < lines.len() {
                let (at, b) = lines[i];
                i += 1;
                if b.trim() == "}" {
                    end = at;
                    break;
                }
                if let Some(x) = binding_line(b) {
                    binds.push(x);
                }
            }
            actions.push(Action { name: String::from(name), comment: core::mem::take(&mut para), section: section.clone(), group: group.clone(), lines: binds, body: (start, end.max(start)) });
        }
        InputMap { actions }
    }

    /// The first block named `name`.
    pub fn find(&self, name: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.name == name)
    }

    /// `text` (the text this map was parsed from) with the body of each named block
    /// replaced by `binding`'s lines ([`Action::body_for`]); every other byte is kept.
    /// Names with no block are skipped.
    pub fn rewrite(&self, text: &str, changes: &[(&str, &Binding)]) -> String {
        let crlf = text.contains("\r\n");
        let mut edits: Vec<(usize, usize, String)> = Vec::new();
        for (name, b) in changes {
            if let Some(a) = self.find(name)
                && a.body.1 <= text.len()
                && !edits.iter().any(|e| e.0 == a.body.0)
            {
                edits.push((a.body.0, a.body.1, a.body_for(b, crlf)));
            }
        }
        edits.sort_by_key(|e| e.0);
        let mut out = String::with_capacity(text.len() + 64);
        let mut at = 0;
        for (s, e, body) in edits {
            if s < at {
                continue;
            }
            out.push_str(&text[at..s]);
            out.push_str(&body);
            at = e;
        }
        out.push_str(&text[at..]);
        out
    }
}

impl Action {
    /// The keyboard keys the block holds down that are not modifiers, as written.
    pub fn keys(&self) -> Vec<&str> {
        self.lines.iter().filter(|l| l.plus && l.device.eq_ignore_ascii_case("keyboard") && !is_modifier(&l.input)).map(|l| l.input.as_str()).collect()
    }

    /// Whether any line names the mouse (or a device other than the keyboard).
    pub fn uses_mouse(&self) -> bool {
        self.lines.iter().any(|l| !l.device.eq_ignore_ascii_case("keyboard"))
    }

    /// What the block asks of each modifier.
    pub fn mods(&self) -> [Mod; 3] {
        let mut m = [Mod::Any; 3];
        for l in self.lines.iter().filter(|l| l.device.eq_ignore_ascii_case("keyboard")) {
            if let Some(i) = MODIFIERS.iter().position(|x| x.eq_ignore_ascii_case(&l.input)) {
                m[i] = if l.plus { Mod::On } else { Mod::Off };
            }
        }
        m
    }

    /// The block as one key and modifiers, when it is that: one held key that is not a
    /// modifier, no mouse, nothing held to be released (`-` on a key that is not a
    /// modifier). The key may be one this module does not know.
    pub fn binding(&self) -> Option<Binding> {
        if self.uses_mouse() {
            return None;
        }
        let keys = self.keys();
        if keys.len() != 1 || self.lines.iter().any(|l| !l.plus && !is_modifier(&l.input)) {
            return None;
        }
        Some(Binding { key: String::from(keys[0]), mods: self.mods() })
    }

    /// Whether a binding of this block may be changed here: it is one key this module
    /// knows, without a mark (`*Esc`, whose `*` is the engine's and not understood
    /// here), and modifiers.
    pub fn rebindable(&self) -> bool {
        self.binding().is_some_and(|b| key_by_name(&b.key).is_some())
    }

    /// What the block reads as once bound to `b` ([`Action::body_for`]): `b`'s key,
    /// and each modifier as the new body names it.
    pub fn bound_to(&self, b: &Binding) -> Binding {
        let now = self.mods();
        let mut mods = [Mod::Any; 3];
        for i in 0..3 {
            mods[i] = match (b.mods[i], now[i]) {
                (Mod::On, _) => Mod::On,
                (Mod::Off | Mod::Any, Mod::Any) => Mod::Any,
                (Mod::Off, _) => Mod::Off,
                (Mod::Any, m) => m,
            };
        }
        Binding { key: b.key.clone(), mods }
    }

    /// The body that binds this block to `b`: the key, then each modifier `b` holds
    /// (`+`), and each modifier it does not hold that the block names now (`-`). A
    /// modifier the block leaves open and `b` does not hold stays open, so the number
    /// keys' group block keeps answering Control+number.
    pub fn body_for(&self, b: &Binding, crlf: bool) -> String {
        let nl = if crlf { "\r\n" } else { "\n" };
        let mut s = String::new();
        s.push_str("\t+ keyboard\t");
        s.push_str(&b.key);
        s.push_str(nl);
        let now = self.mods();
        for (i, m) in MODIFIERS.iter().enumerate() {
            let sign = match (b.mods[i], now[i]) {
                (Mod::On, _) => "+",
                // Not held, where the block leaves it open: it stays open.
                (Mod::Off | Mod::Any, Mod::Any) => continue,
                (Mod::Off, _) => "-",
                // Any where the block named it: keep what the block said.
                (Mod::Any, Mod::On) => "+",
                (Mod::Any, Mod::Off) => "-",
            };
            s.push('\t');
            s.push_str(sign);
            s.push_str(" keyboard\t");
            s.push_str(m);
            s.push_str(nl);
        }
        s
    }
}

impl Binding {
    /// A binding as the overlay spells it: `Control+Shift+F1`, modifiers that must be held
    /// first (a modifier left open, or one that must not be held, is not written).
    pub fn spell(&self) -> String {
        let mut s = String::new();
        for (i, m) in MODIFIERS.iter().enumerate() {
            if self.mods[i] == Mod::On {
                s.push_str(m);
                s.push('+');
            }
        }
        s.push_str(&self.key);
        s
    }

    /// The overlay's spelling read back ([`Binding::spell`]): every modifier it names is
    /// held, every other one must not be. Also takes `Ctrl` for Control. None for a key
    /// this module does not know.
    pub fn read(s: &str) -> Option<Binding> {
        let mut mods = [Mod::Off; 3];
        let mut parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        let key = parts.pop()?;
        for p in parts {
            let p = if p.eq_ignore_ascii_case("ctrl") { "Control" } else { p };
            let i = MODIFIERS.iter().position(|m| m.eq_ignore_ascii_case(p))?;
            mods[i] = Mod::On;
        }
        let k = key_by_name(key)?;
        Some(Binding { key: String::from(k.name), mods })
    }

    /// For people: `Ctrl+Shift+Home`, with the keys' labels; a modifier left open is
    /// not shown.
    pub fn label(&self) -> String {
        let mut s = String::new();
        for (i, m) in ["Ctrl", "Shift", "Alt"].iter().enumerate() {
            if self.mods[i] == Mod::On {
                s.push_str(m);
                s.push('+');
            }
        }
        let name = self.key.trim_start_matches('*');
        s.push_str(key_by_name(name).map_or(name, |k| k.label));
        s
    }

    /// Whether a press of `key` with the modifiers `held` (Control, Shift, Alt) is this
    /// binding.
    pub fn matches(&self, key: &str, held: [bool; 3]) -> bool {
        self.key.trim_start_matches('*').eq_ignore_ascii_case(key.trim_start_matches('*'))
            && (0..3).all(|i| match self.mods[i] {
                Mod::Any => true,
                Mod::On => held[i],
                Mod::Off => !held[i],
            })
    }

    /// Whether some press answers both bindings (the same key, and no modifier one
    /// needs held that the other needs released).
    pub fn overlaps(&self, o: &Binding) -> bool {
        self.key.trim_start_matches('*').eq_ignore_ascii_case(o.key.trim_start_matches('*'))
            && (0..3).all(|i| !matches!((self.mods[i], o.mods[i]), (Mod::On, Mod::Off) | (Mod::Off, Mod::On)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // Written for these tests in the file's syntax; not a copy of the game's file.
    const SAMPLE: &str = "####\n# Top #\n####\n\n# The first modifier\ncontrol {\n\t+ keyboard\tControl\n}\n\n\
######\n# Moving #\n######\n\n### Turning\n# Turn left\n# (only in one view)\nturn_left {\n\t+ keyboard\tGreyLeftArrow\n\t+ keyboard \tControl\n\t- keyboard\tShift\n}\n\
group_one {\n\t+ keyboard\tOne\n}\n#hidden {\n#\t+ keyboard\tO\n#}\nback {\n\t+ keyboard\t*Esc\n\t- keyboard\tAlt\n}\nclick {\n\t+ mouse\tLeftBtn\n}\ntwo_keys {\n\t+ keyboard  A\n\t+ keyboard\tI\n}\n";

    #[test]
    fn parses_blocks_headings_and_comments() {
        let m = InputMap::parse(SAMPLE);
        let names: Vec<&str> = m.actions.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["control", "turn_left", "group_one", "back", "click", "two_keys"]);
        let t = m.find("turn_left").unwrap();
        assert_eq!(t.section, "Moving");
        assert_eq!(t.group, "Turning");
        assert_eq!(t.comment, vec![String::from("Turn left"), String::from("(only in one view)")]);
        assert_eq!(t.mods(), [Mod::On, Mod::Off, Mod::Any]);
        assert_eq!(t.binding().unwrap().label(), "Ctrl+Left");
        assert_eq!(m.find("control").unwrap().section, "Top");
        assert!(m.find("group_one").unwrap().comment.is_empty());
        assert!(m.find("hidden").is_none());
    }

    #[test]
    fn what_can_be_rebound() {
        let m = InputMap::parse(SAMPLE);
        assert!(m.find("turn_left").unwrap().rebindable());
        assert!(m.find("group_one").unwrap().rebindable());
        assert!(!m.find("back").unwrap().rebindable(), "a marked key is kept");
        assert!(!m.find("click").unwrap().rebindable());
        assert!(!m.find("two_keys").unwrap().rebindable());
        assert!(m.find("control").unwrap().binding().is_none());
    }

    #[test]
    fn rewrite_keeps_every_other_byte() {
        let m = InputMap::parse(SAMPLE);
        let b = Binding::read("Shift+F5").unwrap();
        let g = Binding::read("Alt+Two").unwrap();
        let out = m.rewrite(SAMPLE, &[("turn_left", &b), ("group_one", &g), ("nothing", &b)]);
        let n = InputMap::parse(&out);
        assert_eq!(n.actions.len(), m.actions.len());
        let t = n.find("turn_left").unwrap();
        assert_eq!(t.binding().unwrap(), m.find("turn_left").unwrap().bound_to(&b));
        assert_eq!(n.find("group_one").unwrap().binding().unwrap(), m.find("group_one").unwrap().bound_to(&g));
        // Control was named, so not holding it is written as `-`; Alt was open and stays so.
        assert_eq!(t.binding().unwrap(), Binding { key: String::from("F5"), mods: [Mod::Off, Mod::On, Mod::Any] });
        assert_eq!(n.find("group_one").unwrap().binding().unwrap().mods, [Mod::Any, Mod::Any, Mod::On]);
        assert_eq!(t.comment, m.find("turn_left").unwrap().comment);
        // Unchanged blocks are byte for byte the same.
        let a = &m.find("back").unwrap().body;
        let c = &n.find("back").unwrap().body;
        assert_eq!(&SAMPLE[a.0..a.1], &out[c.0..c.1]);
        assert!(out.ends_with("\t+ keyboard\tI\n}\n"));
        // Back to the first binding: the parse matches the original's.
        let first = m.find("turn_left").unwrap().binding().unwrap();
        let back = n.rewrite(&out, &[("turn_left", &first)]);
        assert_eq!(InputMap::parse(&back).find("turn_left").unwrap().binding(), Some(first));
    }

    #[test]
    fn crlf_is_kept() {
        let text = SAMPLE.replace('\n', "\r\n");
        let m = InputMap::parse(&text);
        assert_eq!(m.actions.len(), 6);
        assert_eq!(m.find("turn_left").unwrap().group, "Turning");
        let out = m.rewrite(&text, &[("group_one", &Binding::read("F1").unwrap())]);
        assert!(!out.replace("\r\n", "").contains('\n'));
        assert_eq!(InputMap::parse(&out).find("group_one").unwrap().keys(), vec!["F1"]);
    }

    #[test]
    fn spellings_and_matching() {
        let b = Binding::read("ctrl+shift+greyhome").unwrap();
        assert_eq!(b.spell(), "Control+Shift+GreyHome");
        assert_eq!(b.label(), "Ctrl+Shift+Home");
        assert_eq!(Binding::read(&b.spell()), Some(b.clone()));
        assert!(Binding::read("Hyper+A").is_none());
        assert!(Binding::read("NoSuchKey").is_none());
        assert!(b.matches("GreyHome", [true, true, false]));
        assert!(!b.matches("GreyHome", [true, true, true]));
        let any = Binding { key: String::from("One"), mods: [Mod::Any; 3] };
        assert!(any.overlaps(&Binding::read("Control+One").unwrap()));
        assert!(!Binding::read("One").unwrap().overlaps(&Binding::read("Control+One").unwrap()));
    }

    #[test]
    fn keys_are_unique() {
        for (i, a) in KEYS.iter().enumerate() {
            for b in &KEYS[i + 1..] {
                assert!(a.scan != b.scan && !a.name.eq_ignore_ascii_case(b.name), "{} / {}", a.name, b.name);
            }
        }
        assert_eq!(key_by_scan(0x148).unwrap().name, "GreyUpArrow");
        assert_eq!(key_by_scan(0x48).unwrap().name, "UpArrow");
    }

    #[test]
    fn bad_input_does_not_panic() {
        for cut in 0..SAMPLE.len() {
            if SAMPLE.is_char_boundary(cut) {
                let m = InputMap::parse(&SAMPLE[..cut]);
                let _ = m.rewrite(&SAMPLE[..cut], &[("turn_left", &Binding::read("A").unwrap())]);
            }
        }
        let _ = InputMap::parse("{\n}\n+ keyboard\n x {\n");
    }
}
