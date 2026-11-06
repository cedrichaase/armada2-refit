//! Host tool: which SODs have a level-of-detail switch, and which of them a database flag
//! on one level could empty (every mesh sits under the levels). For the hull sweep of
//! Fleet Operations: Roots.
//!
//!     cargo run --release --example lodsurvey -- <Data dir> [out.tsv]
//!
//! Reads `sod/` and `addon/` of the given folder; the table is derived from the game's
//! files, so it is written to `$A2_DATA` by default and refused inside a checkout.

use std::path::{Path, PathBuf};
use a2fmt::sod::{Role, Sod};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let Some(data) = a.first() else { eprintln!("usage: lodsurvey <Data dir> [out.tsv]"); std::process::exit(1) };
    let out = a.get(1).map(PathBuf::from).unwrap_or_else(|| {
        let base = std::env::var("A2_DATA").map(PathBuf::from).unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap()).join(".local/share/armada2-refit")
        });
        base.join("research/fo-roots/hull-sweep/lodsurvey.tsv")
    });
    if let Some(p) = out.parent() { std::fs::create_dir_all(p).unwrap(); }
    let abs = out.parent().unwrap().canonicalize().unwrap();
    if abs.ancestors().any(|d| d.join(".git").exists()) { eprintln!("refusing to write inside a checkout"); std::process::exit(1) }
    let mut rows = String::from("sod\tnodes\tmeshes\tmeshes_outside_lod\tgeometries\tlevels(faces)\tflags\n");
    let (mut total, mut with_lod, mut fragile) = (0, 0, 0);
    for d in ["sod", "addon"] {
        let dir = Path::new(data).join(d);
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for p in files {
            if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sod")) { continue }
            let Ok(b) = std::fs::read(&p) else { continue };
            let Ok(s) = Sod::parse(&b) else { rows += &format!("{}\t\t\t\t\t\tUNPARSED\n", p.display()); continue };
            total += 1;
            let meshes: Vec<usize> = (0..s.nodes.len()).filter(|&i| s.nodes[i].mesh().is_some()).collect();
            let outside = meshes.iter().filter(|&&i| s.lod_group(i).is_none()).count();
            let geoms: Vec<usize> = (0..s.nodes.len()).filter(|&i| s.nodes[i].role() == Role::Geometry).collect();
            let mut lv = String::new();
            let mut flags = Vec::new();
            for &g in &geoms {
                let l = s.lod_levels(g);
                if l.is_empty() { continue }
                let f: Vec<String> = l.iter().map(|&n| format!("{}:{}", s.nodes[n].name, s.face_count_recursive(n))).collect();
                lv += &format!("[{}] ", f.join(","));
                if l.iter().any(|&n| s.face_count_recursive(n) == 0) { flags.push("EMPTY_LEVEL"); }
                if l.len() == 1 { flags.push("ONE_LEVEL"); }
            }
            if !lv.is_empty() {
                with_lod += 1;
                if outside == 0 && !meshes.is_empty() { flags.push("ALL_MESHES_UNDER_LOD"); fragile += 1; }
            }
            if meshes.is_empty() { flags.push("NO_MESH"); }
            flags.sort(); flags.dedup();
            rows += &format!("{}/{}\t{}\t{}\t{}\t{}\t{}\t{}\n", d, p.file_name().unwrap().to_string_lossy(), s.nodes.len(), meshes.len(), outside, geoms.len(), lv, flags.join(","));
        }
    }
    std::fs::write(&out, rows).unwrap();
    println!("{total} SODs, {with_lod} with a LOD switch, {fragile} with every mesh under it -> {}", out.display());
}
