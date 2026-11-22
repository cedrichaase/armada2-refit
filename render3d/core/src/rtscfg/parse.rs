//! The reader of `RTS_CFG.h`'s integer keys, pure (host-tested in `render3d/hosttest`).

/// `name`'s integer value in RTS_CFG.h-like text: a line `[type] NAME = value;`, comments
/// after `//` ignored; the last setting wins. None when the key is not there or its value
/// is not an integer.
pub fn parse_int(text: &[u8], name: &str) -> Option<i32> {
    let mut found = None;
    for raw in text.split(|&c| c == b'\n') {
        let Ok(line) = core::str::from_utf8(raw) else { continue };
        let line = match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        };
        let Some(eq) = line.find('=') else { continue };
        let left = line[..eq].trim();
        let key = left.rsplit(|c: char| c.is_whitespace()).next().unwrap_or("");
        if key != name {
            continue;
        }
        let right = line[eq + 1..].trim().trim_end_matches(';').trim();
        let (neg, digits) = match right.strip_prefix('-') {
            Some(d) => (true, d.trim()),
            None => (false, right),
        };
        if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let mut v: i64 = 0;
        for c in digits.bytes() {
            v = (v * 10 + (c - b'0') as i64).min(i32::MAX as i64);
        }
        found = Some(if neg { -(v as i32) } else { v as i32 });
    }
    found
}
