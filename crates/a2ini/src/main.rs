//! `a2ini`, the host tool for the overlay's edit (tools/settings/README.md): the same
//! `a2ini::edit` and `rename` the game runs, for the bench, the GUI installer and the
//! installers' key moves. The text is handled as latin-1, so every byte not touched is kept.
//!
//!   a2ini get FILE Section:Key             print a key's value (exit 1 when absent)
//!   a2ini edit FILE WORD... [--unset S:K]  set (and remove) keys; WORD is Section:Key=value
//!   a2ini unset FILE Section:Key...        remove keys
//!   a2ini rename FILE FromSection:Key ToSection:Key
//!
//! A file is written whole to `FILE.tmp` and renamed over FILE; unchanged, it is not written.
use std::process::ExitCode;

fn read(path: &str) -> String {
    std::fs::read(path).map(|b| a2ini::from_latin1(&b)).unwrap_or_default()
}

/// Writes `new` when it differs from `old`; true when it did.
fn write(path: &str, old: &str, new: &str) -> Result<bool, String> {
    if new == old {
        return Ok(false);
    }
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, a2ini::to_latin1(new)).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{path}: {e}"))?;
    Ok(true)
}

fn name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn sk(word: &str) -> Result<(&str, &str), String> {
    match word.split_once(':') {
        Some((s, k)) if !s.is_empty() && !k.is_empty() => Ok((s, k)),
        _ => Err(format!("{word}: want Section:Key")),
    }
}

fn run(a: &[String]) -> Result<ExitCode, String> {
    let usage = || "usage: a2ini get|edit|unset|rename FILE ...".to_string();
    let cmd = a.first().ok_or_else(usage)?;
    let path = a.get(1).ok_or_else(usage)?;
    let rest = &a[2..];
    match cmd.as_str() {
        "get" => {
            let (s, k) = sk(rest.first().ok_or_else(usage)?)?;
            match a2ini::parse(&read(path)).value(s, k) {
                Some(v) => {
                    println!("{v}");
                    Ok(ExitCode::SUCCESS)
                }
                None => Ok(ExitCode::from(1)),
            }
        }
        "edit" | "unset" => {
            let mut todo: Vec<(&str, &str, Option<&str>)> = Vec::new();
            let mut i = 0;
            while i < rest.len() {
                if cmd == "unset" || rest[i] == "--unset" {
                    if cmd == "edit" {
                        i += 1;
                    }
                    let (s, k) = sk(rest.get(i).ok_or_else(usage)?)?;
                    todo.push((s, k, None));
                } else {
                    let bad = || format!("{}: want Section:Key=value", rest[i]);
                    let (sec, kv) = rest[i].split_once(':').ok_or_else(bad)?;
                    let (k, v) = kv.split_once('=').ok_or_else(bad)?;
                    todo.push((sec, k, Some(v)));
                }
                i += 1;
            }
            for (s, k, v) in todo {
                if v.is_some_and(|v| !a2ini::valid_value(v)) || !a2ini::valid_section(s) || !a2ini::valid_key(k) {
                    return Err(format!("{s}:{k}: refused (CR, LF or surrounding whitespace in the value, or a name that cannot be written)"));
                }
                let old = read(path);
                let wrote = write(path, &old, &a2ini::edit(&old, s, k, v))?;
                let what = v.map_or(format!("{k} removed"), |v| format!("{k}={v}"));
                println!("{}: [{s}] {what}{}", name(path), if wrote { "" } else { " (unchanged)" });
            }
            Ok(ExitCode::SUCCESS)
        }
        "rename" => {
            let (fs, fk) = sk(rest.first().ok_or_else(usage)?)?;
            let (ts, tk) = sk(rest.get(1).ok_or_else(usage)?)?;
            let old = read(path);
            let wrote = write(path, &old, &a2ini::rename(&old, fs, fk, ts, tk))?;
            println!("{}: [{fs}] {fk} -> [{ts}] {tk}{}", name(path), if wrote { "" } else { " (unchanged)" });
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match run(&a) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("a2ini: {e}");
            ExitCode::from(1)
        }
    }
}
