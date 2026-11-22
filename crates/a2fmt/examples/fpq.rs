//! Host tool for Fleet Operations: Roots' `.fpq` archives, for agents.
//!
//!     cargo run --release --example fpq -- list <archive.fpq>
//!     cargo run --release --example fpq -- cat <archive.fpq> <name>
//!     cargo run --release --example fpq -- extract <archive.fpq> <out-dir> [name ...]
//!
//! `extract` writes every file (or the named ones) under `<out-dir>/<archive path>`, and
//! refuses a directory inside a git checkout: what comes out of a game's archive is an
//! asset and belongs in `$A2_DATA` or a scratch directory (hard rule 8).

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::exit;

use a2fmt::fpq::Archive;

fn die(msg: &str) -> ! {
    eprintln!("fpq: {msg}");
    exit(1)
}

fn in_checkout(dir: &Path) -> bool {
    dir.ancestors().any(|a| a.join(".git").exists())
}

/// `out` plus the archive path's components, each of which must be a plain name: an
/// absolute path, a `..` or a drive in an archive's name never leaves `out`.
fn safe_join(out: &Path, path: &str) -> Option<PathBuf> {
    let mut to = out.to_path_buf();
    for c in path.split('/') {
        let mut it = Path::new(c).components();
        match (it.next(), it.next()) {
            (Some(Component::Normal(n)), None) => to.push(n),
            _ => return None,
        }
    }
    assert!(to.starts_with(out));
    Some(to)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(file)) = (args.first(), args.get(1)) else {
        die("usage: fpq list|cat|extract <archive.fpq> [out-dir] [name ...]")
    };
    let data = std::fs::read(file).unwrap_or_else(|e| die(&format!("{file}: {e}")));
    let arc = Archive::parse(data).unwrap_or_else(|e| die(&format!("{file}: {e:?}")));
    match cmd.as_str() {
        "list" => {
            for (i, e) in arc.entries().iter().enumerate() {
                println!("{}\t{}\t{}\tmethod {}", arc.path(i).unwrap_or_default(), e.unpacked, e.packed, e.method);
            }
            eprintln!("{} files, {} folders", arc.entries().len(), arc.folders().len());
        }
        "cat" => {
            let name = args.get(2).unwrap_or_else(|| die("cat needs a name"));
            let i = arc.find(name).unwrap_or_else(|| die(&format!("{name}: not in the archive")));
            let bytes = arc.read(i).unwrap_or_else(|e| die(&format!("{name}: {e:?}")));
            std::io::stdout().write_all(&bytes).unwrap();
        }
        "extract" => {
            let out = Path::new(args.get(2).unwrap_or_else(|| die("extract needs an out-dir")));
            std::fs::create_dir_all(out).unwrap_or_else(|e| die(&format!("{}: {e}", out.display())));
            let out = out.canonicalize().unwrap_or_else(|e| die(&format!("{}: {e}", out.display())));
            if in_checkout(&out) {
                die("refusing to extract inside a git checkout: use $A2_DATA or a scratch directory");
            }
            let wanted: Vec<usize> = if args.len() > 3 {
                args[3..]
                    .iter()
                    .map(|n| arc.find(n).unwrap_or_else(|| die(&format!("{n}: not in the archive"))))
                    .collect()
            } else {
                (0..arc.entries().len()).collect()
            };
            for i in &wanted {
                let path = arc.path(*i).unwrap_or_default();
                let bytes = arc.read(*i).unwrap_or_else(|e| die(&format!("{path}: {e:?}")));
                let to = safe_join(&out, &path).unwrap_or_else(|| die(&format!("{path}: not a safe path")));
                std::fs::create_dir_all(to.parent().unwrap()).unwrap();
                std::fs::write(&to, bytes).unwrap_or_else(|e| die(&format!("{}: {e}", to.display())));
            }
            eprintln!("{} files extracted to {}", wanted.len(), out.display());
        }
        _ => die("commands: list, cat, extract"),
    }
}
