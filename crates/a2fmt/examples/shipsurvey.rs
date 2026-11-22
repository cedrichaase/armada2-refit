//! Host tool: a survey of the craft in Fleet Operations: Roots, for agents.
//!
//!     cargo run --release --example shipsurvey -- <Roots Data dir> [out-dir]
//!
//! Reads `ODF.fpq` (through `a2fmt::fpq`), resolves every ODF that names a `race` and a
//! `basename`, finds its SOD (`sod/`, then `addon/`, ASCII case-insensitive), and writes
//! tab-separated tables: `ships.tsv` (ODF, race, class label, ship class, SOD, mesh and
//! bump-mapped counts), `meshes.tsv` (per mesh: blend material, flags, bump, textures
//! with their load words), `textures.tsv` (each texture, the races that use it),
//! `races.tsv` (the race ODFs' own keys), `cfg.tsv` (lines of `RTS_CFG.h`/`ART_CFG.h`
//! that mention race, colour or team) and `summary.txt`.
//!
//! A mesh counts as bump-mapped when its flags carry the dot3 bit or one of its textures
//! carries the height-map word `0x200` (`Mesh::FLAG_BUMP`, `Mesh::height_map`).
//!
//! The output is derived from the game's files: it goes to `$A2_DATA/research/fo-roots/
//! ship-survey/` by default and the tool refuses a directory inside a git checkout (hard
//! rule 8). Every read is bounded: files over 64 MiB, SODs over 16 MiB and directories of
//! more than 100000 entries are skipped (and counted or ignored).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::exit;

use a2fmt::fpq::Archive;
use a2fmt::odf::Odf;
use a2fmt::sod::{Mesh, Sod};

const MAX_FILE: u64 = 64 << 20;
const MAX_ARCHIVE: u64 = 1 << 30;
const MAX_SOD: u64 = 16 << 20;
const MAX_DIR: usize = 100_000;
const MAX_ODFS: usize = 20_000;

fn die(msg: &str) -> ! {
    eprintln!("shipsurvey: {msg}");
    exit(1)
}

fn in_checkout(dir: &Path) -> bool {
    dir.ancestors().any(|a| a.join(".git").exists())
}

fn default_out() -> PathBuf {
    let base = std::env::var("A2_DATA").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| die("no A2_DATA and no HOME"));
        PathBuf::from(home).join(".local/share/armada2-refit")
    });
    base.join("research/fo-roots/ship-survey")
}

/// Lower-cased file name -> path, for one directory (not recursive).
fn index(dir: &Path) -> BTreeMap<String, PathBuf> {
    let mut m = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.take(MAX_DIR).flatten() {
            if let Some(n) = e.file_name().to_str() {
                m.insert(n.to_ascii_lowercase(), e.path());
            }
        }
    }
    m
}

fn read_bounded(p: &Path, max: u64) -> Option<Vec<u8>> {
    if std::fs::metadata(p).ok()?.len() > max {
        return None;
    }
    std::fs::read(p).ok()
}

fn clean(s: &str) -> String {
    s.chars().map(|c| if c == '\t' || c == '\n' || c == '\r' { ' ' } else { c }).collect()
}

struct Row {
    odf: String,
    race: String,
    class: String,
    shipclass: String,
    basename: String,
    sod: String, // file name when found, else ""
    meshes: Vec<Mesh>,
}

fn is_bump(m: &Mesh) -> bool {
    m.flags & Mesh::FLAG_BUMP != 0 || m.height_map().is_some()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(data_dir) = args.first() else { die("usage: shipsurvey <Roots Data dir> [out-dir]") };
    let data_dir = PathBuf::from(data_dir);
    let out = args.get(1).map(PathBuf::from).unwrap_or_else(default_out);
    std::fs::create_dir_all(&out).unwrap_or_else(|e| die(&format!("{}: {e}", out.display())));
    let out = out.canonicalize().unwrap_or_else(|e| die(&format!("{}: {e}", out.display())));
    if in_checkout(&out) {
        die("refusing to write inside a git checkout: the tables are derived from the game (use $A2_DATA)");
    }

    let fpq_path = data_dir.join("ODF.fpq");
    let bytes = read_bounded(&fpq_path, MAX_ARCHIVE).unwrap_or_else(|| die(&format!("{}: unreadable or too big", fpq_path.display())));
    let arc = Archive::parse(bytes).unwrap_or_else(|e| die(&format!("ODF.fpq: {e:?}")));

    let sod_dirs: Vec<_> = ["sod", "addon"].iter().map(|d| index(&data_dir.join(d))).collect();
    let mut tex_dir = index(&data_dir.join("Textures"));
    if tex_dir.is_empty() {
        tex_dir = index(&data_dir.join("Textures/RGB"));
    }

    // Every ODF in the archive.
    let mut names: Vec<String> = Vec::new();
    for i in 0..arc.entries().len().min(MAX_ODFS) {
        if let Some(p) = arc.path(i) {
            if p.to_ascii_lowercase().ends_with(".odf") {
                names.push(p.rsplit(['/', '\\']).next().unwrap_or(&p).to_string());
            }
        }
    }
    names.sort();
    let fetch = |n: &str| arc.find(n).and_then(|i| arc.read(i).ok());

    // The race ODFs (the ones with `displaykey` and `name`): all their keys.
    let mut races = String::from("odf\tkey\tvalue\n");
    let mut race_files: BTreeMap<String, String> = BTreeMap::new();
    for n in &names {
        let Ok(o) = Odf::load(n, fetch) else { continue };
        if o.contains("displaykey") {
            if let Some(name) = o.get_str("name") {
                race_files.insert(name.to_ascii_lowercase(), n.clone());
                for (k, v) in o.all() {
                    let _ = writeln!(races, "{n}\t{}\t{}", clean(k), clean(v));
                }
            }
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    let mut sod_cache: BTreeMap<String, Option<Sod>> = BTreeMap::new();
    let mut sods_unparsed = 0usize;
    for n in &names {
        let Ok(o) = Odf::load(n, fetch) else { continue };
        let (Some(race), Some(base)) = (o.get_str("race"), o.get_str("basename")) else { continue };
        let sodname = base.to_ascii_lowercase();
        let sodname = if sodname.ends_with(".sod") { sodname } else { format!("{sodname}.sod") };
        if !sod_cache.contains_key(&sodname) {
            let path = sod_dirs.iter().find_map(|d| d.get(&sodname));
            let sod = path.and_then(|p| read_bounded(p, MAX_SOD)).and_then(|b| Sod::parse(&b).ok());
            if sod.is_none() && path.is_some() {
                sods_unparsed += 1;
            }
            sod_cache.insert(sodname.clone(), sod);
        }
        let found = sod_cache[&sodname].as_ref();
        let meshes: Vec<Mesh> = found.map(|s| s.nodes.iter().filter_map(|nd| nd.mesh().cloned()).collect()).unwrap_or_default();
        rows.push(Row {
            odf: n.clone(),
            race: race.to_ascii_lowercase(),
            class: o.class_label().unwrap_or_default(),
            shipclass: o.get_str("shipclass").unwrap_or("").to_ascii_lowercase(),
            basename: base.to_string(),
            sod: if found.is_some() { sodname } else { String::new() },
            meshes,
        });
    }

    let mut ships = String::from("odf\trace\tclasslabel\tshipclass\tbasename\tsod\tmeshes\tbump_meshes\n");
    let mut meshes_tsv = String::from("odf\tsod\tmesh\tblend\tflags\tbump\theight_map\ttextures(name:word)\ttexture2\n");
    let mut tex_races: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut tex_bump: BTreeSet<String> = BTreeSet::new();
    // per race: all with race, craft, craft with SOD, craft all-bump, craft some-bump
    let mut per_race: BTreeMap<String, [usize; 5]> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        let nb = r.meshes.iter().filter(|m| is_bump(m)).count();
        let _ = writeln!(ships, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", r.odf, r.race, r.class, r.shipclass, r.basename, r.sod, r.meshes.len(), nb);
        for (i, m) in r.meshes.iter().enumerate() {
            let tex: Vec<String> = m.textures.iter().map(|t| format!("{}:{:#x}", t.name, t.flags)).collect();
            let _ = writeln!(
                meshes_tsv,
                "{}\t{}\t{}\t{}\t{:#x}\t{}\t{}\t{}\t{}",
                r.odf,
                r.sod,
                i,
                clean(&m.texture_material),
                m.flags,
                if is_bump(m) { "bump" } else { "" },
                m.height_map().unwrap_or(""),
                clean(&tex.join(" ")),
                clean(&m.texture2)
            );
            for t in m.textures.iter().map(|t| t.name.as_str()).chain(Some(m.texture2.as_str())).filter(|t| !t.is_empty()) {
                tex_races.entry(t.to_ascii_lowercase()).or_default().insert(r.race.clone());
                if is_bump(m) {
                    tex_bump.insert(t.to_ascii_lowercase());
                }
            }
        }
        *classes.entry(r.class.clone()).or_default() += 1;
        let c = per_race.entry(r.race.clone()).or_default();
        c[0] += 1;
        if r.class == "craft" {
            c[1] += 1;
            if !r.sod.is_empty() {
                c[2] += 1;
                if nb == r.meshes.len() && nb > 0 {
                    c[3] += 1;
                } else if nb > 0 {
                    c[4] += 1;
                }
            }
        }
    }

    let mut tex_tsv = String::from("texture\tin_data_textures\tbump_mesh\traces\n");
    let mut shared: Vec<(&String, &BTreeSet<String>)> = Vec::new();
    for (t, rs) in &tex_races {
        let stem = t.trim_end_matches(".tga").trim_end_matches(".dds");
        let have = ["", ".tga", ".dds"].iter().any(|e| tex_dir.contains_key(&format!("{stem}{e}")));
        let _ = writeln!(
            tex_tsv,
            "{t}\t{}\t{}\t{}",
            if have { "yes" } else { "no" },
            if tex_bump.contains(t) { "yes" } else { "" },
            rs.iter().cloned().collect::<Vec<_>>().join(",")
        );
        if rs.len() > 1 {
            shared.push((t, rs));
        }
    }

    // Lines of the cfg headers about race, colour or team.
    let mut cfg = String::from("file\tline\ttext\n");
    for f in ["RTS_CFG.h", "ART_CFG.h", "game_cfg.h"] {
        let Some(b) = read_bounded(&data_dir.join(f), MAX_FILE) else { continue };
        for (i, l) in String::from_utf8_lossy(&b).lines().enumerate() {
            let low = l.to_ascii_lowercase();
            if ["race", "colour", "color", "team", "species"].iter().any(|w| low.contains(w)) && !l.trim().is_empty() {
                let _ = writeln!(cfg, "{f}\t{}\t{}", i + 1, clean(l.trim()));
            }
        }
    }

    let mut s = String::new();
    let _ = writeln!(
        s,
        "ODFs read: {}; with race and basename: {}; distinct SODs: {} ({} not found, {} unparsed)",
        names.len(),
        rows.len(),
        sod_cache.len(),
        sod_cache.values().filter(|v| v.is_none()).count(),
        sods_unparsed
    );
    let _ = writeln!(s, "race ODFs (name -> file): {}", race_files.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "\nclass labels of the ODFs that name a race: {}", classes.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "\nper race, bump columns for class 'craft' with a SOD found:");
    let _ = writeln!(s, "race\tall_with_race\tcraft\tcraft_with_sod\tall_meshes_bump\tsome_meshes_bump\tno_bump");
    for (r, c) in &per_race {
        let _ = writeln!(s, "{r}\t{}\t{}\t{}\t{}\t{}\t{}", c[0], c[1], c[2], c[3], c[4], c[2] - c[3] - c[4]);
    }
    let mut mesh_race: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.class == "craft") {
        let e = mesh_race.entry(&r.race).or_default();
        e.0 += r.meshes.len();
        e.1 += r.meshes.iter().filter(|m| is_bump(m)).count();
    }
    let _ = writeln!(s, "\nmeshes of craft per race (total, bump-mapped):");
    for (r, (t, b)) in &mesh_race {
        let _ = writeln!(s, "{r}\t{t}\t{b}");
    }
    let mut sod_races: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for r in rows.iter().filter(|r| !r.sod.is_empty()) {
        sod_races.entry(&r.sod).or_default().insert(&r.race);
    }
    let mut per: BTreeMap<&str, usize> = BTreeMap::new();
    for rs in sod_races.values() {
        for r in rs {
            *per.entry(r).or_default() += 1;
        }
    }
    let _ = writeln!(s, "\ndistinct SODs per race: {}", per.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "\ntextures: {} distinct, {} used by more than one race, {} on a bump-mapped mesh", tex_races.len(), shared.len(), tex_bump.len());
    let _ = writeln!(s, "textures shared between races (first 60):");
    for (t, rs) in shared.iter().take(60) {
        let _ = writeln!(s, "  {t}\t{}", rs.iter().cloned().collect::<Vec<_>>().join(","));
    }
    let _ = writeln!(s, "\nSODs used by more than one race (first 60):");
    for (k, v) in sod_races.iter().filter(|(_, v)| v.len() > 1).take(60) {
        let _ = writeln!(s, "  {k}\t{}", v.iter().cloned().collect::<Vec<_>>().join(","));
    }

    for (f, c) in [("ships.tsv", &ships), ("meshes.tsv", &meshes_tsv), ("textures.tsv", &tex_tsv), ("races.tsv", &races), ("cfg.tsv", &cfg), ("summary.txt", &s)] {
        std::fs::write(out.join(f), c).unwrap_or_else(|e| die(&format!("{f}: {e}")));
    }
    print!("{s}");
    eprintln!("written to {}", out.display());
}
