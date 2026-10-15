//! Shared helpers for the text formats (`.spr`, ODF): Latin-1 decoding, comment
//! stripping and tokenising.

use alloc::string::String;
use alloc::vec::Vec;

/// Bytes as Latin-1: every byte maps to the char of the same value.
pub fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The lines of a text file (`\n`, `\r\n` or a lone `\r`), as Latin-1.
pub fn lines(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut start = 0;
    while i < data.len() {
        match data[i] {
            b'\n' => {
                out.push(latin1(&data[start..i]));
                start = i + 1;
            }
            b'\r' => {
                out.push(latin1(&data[start..i]));
                if i + 1 < data.len() && data[i + 1] == b'\n' {
                    i += 1;
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < data.len() {
        out.push(latin1(&data[start..]));
    }
    out
}

/// Cuts a line at the first comment marker outside double quotes. `hash` says whether
/// `#` starts a comment as well as `//`.
pub fn strip_comment(line: &str, hash: bool) -> &str {
    let b = line.as_bytes();
    let mut quoted = false;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => quoted = !quoted,
            b'#' if hash && !quoted => return &line[..i],
            b'/' if !quoted && i + 1 < b.len() && b[i + 1] == b'/' => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

/// Splits on whitespace, but keeps a parenthesised group such as `( 1, 0.5 ,1)` as one
/// token (with its inner whitespace removed), and a double-quoted string as one token
/// (quotes kept).
pub fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0u32;
    let mut quoted = false;
    for c in line.chars() {
        if quoted {
            cur.push(c);
            if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            c if c.is_whitespace() => {
                if depth == 0 {
                    if !cur.is_empty() {
                        out.push(core::mem::take(&mut cur));
                    }
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Parses a float the way C's `atof` would for the inputs these files hold: an optional
/// sign, digits, an optional fraction and exponent. Trailing garbage ends the number
/// (`0.25f` reads 0.25); no digits at all is `None`.
pub fn parse_f32(s: &str) -> Option<f32> {
    let s = s.trim();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let ds = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i - ds;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let fs = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        digits += i - fs;
    }
    if digits == 0 {
        return None;
    }
    let mantissa_end = i;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let es = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > es {
            i = j;
        }
    }
    let num = &s[..i.max(mantissa_end)];
    // `str::parse` rejects a bare "5." or ".5"? It accepts both; and "+1".
    num.parse::<f32>().ok()
}

/// Parses an integer like C's `atoi`: sign, digits, stop at the first non-digit. A
/// float such as `0.8` reads 0, which is what the engine does with `draw` keys.
pub fn parse_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    let b = s.as_bytes();
    let mut i = 0;
    let neg = if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
        b[0] == b'-'
    } else {
        false
    };
    let ds = i;
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = (v * 10 + (b[i] - b'0') as i64).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    if i == ds {
        return None;
    }
    let v = if neg { -v } else { v };
    Some(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// `(a,b,c,...)` as floats. Missing or unreadable parts are `None` for the whole tuple.
/// A missing closing parenthesis is forgiven, as a `scanf` pattern would forgive it
/// (stock `damage.spr` has one).
pub fn parse_tuple<const N: usize>(s: &str) -> Option<[f32; N]> {
    let s = s.trim();
    let inner = s.strip_prefix('(')?;
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    let mut out = [0.0f32; N];
    let mut n = 0;
    for part in inner.split(',') {
        if n == N {
            return None;
        }
        out[n] = parse_f32(part)?;
        n += 1;
    }
    if n == N { Some(out) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        assert_eq!(parse_f32("0.25"), Some(0.25));
        assert_eq!(parse_f32("-3"), Some(-3.0));
        assert_eq!(parse_f32("1.5e2x"), Some(150.0));
        assert_eq!(parse_f32(".5"), Some(0.5));
        assert_eq!(parse_f32("abc"), None);
        assert_eq!(parse_i32("0.8"), Some(0));
        assert_eq!(parse_i32("-12a"), Some(-12));
        assert_eq!(parse_tuple::<3>("( 1, 0.5 ,1)".replace(' ', "").as_str()), Some([1.0, 0.5, 1.0]));
    }

    #[test]
    fn tokenising() {
        let t = tokens("@sprite_node a b c (2, 2) (1,1,1) billboard");
        assert_eq!(t.len(), 7);
        assert_eq!(t[4], "(2,2)");
        let t = tokens("names = \"USS A\" \"USS B\"");
        assert_eq!(t, ["names", "=", "\"USS A\"", "\"USS B\""]);
        assert_eq!(strip_comment("a = \"x//y\" // c", true), "a = \"x//y\" ");
    }
}
