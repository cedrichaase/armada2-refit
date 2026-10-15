//! Every stock file of the installed game through every parser.
//!
//! The game directory comes from `A2_GAME`, else from `a2env.sh` at the repository
//! root. Without a game (CI) each test prints a note and passes. Nothing read here is
//! written anywhere.
//!
//! "Stock" means: where a `<file>.<suffix>-backup` sits beside an installed file, the
//! backup is the stock original. Both the backup and the installed (modded) file are
//! parsed; both must parse.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use a2fmt::odf::Odf;
use a2fmt::sod::{ChannelKeys, Error as SodError, NodeKind, Role, Sod};
use a2fmt::spr::SpriteTable;
use a2fmt::tga::{self, Header, TexIndex};

fn game_dir() -> Option<PathBuf> {
    let from_env = std::env::var_os("A2_GAME").map(PathBuf::from);
    let dir = from_env.or_else(|| {
        let env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../a2env.sh");
        let out = Command::new("bash").arg(env).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.lines().find_map(|l| l.strip_prefix("A2_GAME=").map(PathBuf::from))
    })?;
    if dir.join("SOD").is_dir() {
        Some(dir)
    } else {
        None
    }
}

macro_rules! game_or_skip {
    () => {
        match game_dir() {
            Some(d) => d,
            None => {
                eprintln!("NOTE: no Armada II install found (A2_GAME / a2env.sh); skipping stock test");
                return;
            }
        }
    };
}

/// A file to parse: its path, and whether it is the stock original.
#[derive(Debug)]
struct Item {
    path: PathBuf,
    /// The installed name (for a backup, without its backup suffix).
    name: String,
    stock: bool,
}

fn backup_base(name: &str) -> Option<&str> {
    // "<name>.<ext>.<tag>-backup" -> "<name>.<ext>"
    if !name.ends_with("-backup") {
        return None;
    }
    let at = name.rfind('.')?;
    Some(&name[..at])
}

/// Files in `dir` (not recursive) with extension `ext` (any case), plus their backups.
fn items(dir: &Path, ext: &str) -> Vec<Item> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return out };
    let names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let has_ext = |n: &str| {
        Path::new(n).extension().map(|x| x.to_string_lossy().eq_ignore_ascii_case(ext)).unwrap_or(false)
    };
    let backed: BTreeSet<String> = names.iter().filter_map(|n| backup_base(n)).map(String::from).collect();
    for n in &names {
        if let Some(base) = backup_base(n) {
            if has_ext(base) {
                out.push(Item { path: dir.join(n), name: base.to_string(), stock: true });
            }
        } else if has_ext(n) {
            out.push(Item { path: dir.join(n), name: n.clone(), stock: !backed.contains(n) });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn finite3(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}

#[test]
fn sod_every_file() {
    let game = game_or_skip!();
    let mut all = items(&game.join("SOD"), "sod");
    all.extend(items(&game.join("SOD/Software"), "sod"));
    assert!(all.len() > 400, "expected the stock SOD set, found {}", all.len());

    let mut parsed = 0;
    let mut stock_parsed = 0;
    let mut refused_old = Vec::new();
    let mut failures = Vec::new();
    let mut versions: BTreeMap<String, usize> = BTreeMap::new();
    let mut node_types: BTreeMap<u16, usize> = BTreeMap::new();
    let mut chan_types: BTreeMap<(u8, u8), usize> = BTreeMap::new();
    let mut mesh_flags: BTreeMap<u32, usize> = BTreeMap::new();
    let mut tex_flags: BTreeMap<u32, usize> = BTreeMap::new();
    let mut field_u16: BTreeMap<u16, usize> = BTreeMap::new();
    let mut texmats: BTreeMap<String, usize> = BTreeMap::new();
    let mut models: BTreeMap<u8, usize> = BTreeMap::new();
    let mut cull: BTreeMap<u8, usize> = BTreeMap::new();
    let mut unresolved_parents = BTreeSet::new();
    let mut missing_group_mats = 0usize;
    let mut texture2 = 0usize;
    let mut trailing = 0usize;
    let mut empty_meshes = 0usize;
    let mut hardpoints = 0usize;
    let mut lods = 0usize;
    let mut mirrored = 0usize;
    let mut sprite_names = BTreeSet::new();
    let mut emitter_names = BTreeSet::new();
    let mut refs = BTreeSet::new();
    let mut nonfinite = Vec::new();
    let mut max_keys = 0usize;
    let mut idents = (0usize, 0usize);

    for it in &all {
        let data = fs::read(&it.path).unwrap();
        let sod = match Sod::parse(&data) {
            Ok(s) => s,
            Err(SodError::UnsupportedVersion(v)) if v < 1.4 => {
                refused_old.push((it.path.file_name().unwrap().to_string_lossy().into_owned(), v));
                continue;
            }
            Err(e) => {
                failures.push(format!("{}: {:?}", it.path.display(), e));
                continue;
            }
        };
        parsed += 1;
        if it.stock {
            stock_parsed += 1;
        }
        *versions
            .entry(format!("{:?} {:.2}", sod.container, sod.version))
            .or_default() += 1;
        for m in &sod.lighting_materials {
            *models.entry(m.model).or_default() += 1;
            if !finite3(&m.ambient) || !finite3(&m.diffuse) || !finite3(&m.specular) || !m.power.is_finite() {
                nonfinite.push(format!("{} material {}", it.name, m.name));
            }
        }
        for (i, n) in sod.nodes.iter().enumerate() {
            *node_types.entry(n.node_type).or_default() += 1;
            if !n.transform.is_finite() {
                nonfinite.push(format!("{} node {}", it.name, n.name));
            }
            if n.transform.determinant() < 0.0 {
                mirrored += 1;
            }
            if !n.parent_name.is_empty() && n.parent.is_none() {
                unresolved_parents.insert(format!("{}: {} -> {}", it.name, n.name, n.parent_name));
            }
            // The tree must be acyclic: a walk to the root ends.
            let mut at = n.parent;
            let mut steps = 0;
            while let Some(p) = at {
                steps += 1;
                assert!(steps <= sod.nodes.len(), "cycle in {}", it.name);
                at = sod.nodes[p].parent;
            }
            for &c in &n.children {
                assert_eq!(sod.nodes[c].parent, Some(i));
            }
            match n.role() {
                Role::Hardpoint(_) => hardpoints += 1,
                Role::Geometry => {
                    if !sod.lod_levels(i).is_empty() {
                        lods += 1;
                    }
                }
                _ => {}
            }
            match &n.kind {
                NodeKind::Mesh(m) => {
                    *mesh_flags.entry(m.flags).or_default() += 1;
                    *field_u16.entry(m.field_u16).or_default() += 1;
                    *cull.entry(m.cull).or_default() += 1;
                    *texmats.entry(m.texture_material.to_ascii_lowercase()).or_default() += 1;
                    for t in &m.textures {
                        *tex_flags.entry(t.flags).or_default() += 1;
                    }
                    if !m.texture2.is_empty() {
                        texture2 += 1;
                    }
                    if !m.trailing.is_empty() {
                        trailing += 1;
                        println!("  trailing u16 list in {} ({}) mesh {}: {:?}", it.name, if it.stock { "stock" } else { "modded" }, n.name, m.trailing);
                    }
                    if sod.version < 1.7 {
                        if sod.texture_for_ident(&m.texture_material).is_some() {
                            idents.0 += 1;
                        } else {
                            idents.1 += 1;
                        }
                    }
                    if m.positions.is_empty() {
                        empty_meshes += 1;
                    }
                    for p in &m.positions {
                        if !finite3(p) {
                            nonfinite.push(format!("{} mesh {} position", it.name, n.name));
                            break;
                        }
                    }
                    for t in &m.uvs {
                        if !finite3(t) {
                            nonfinite.push(format!("{} mesh {} uv", it.name, n.name));
                            break;
                        }
                    }
                    for g in &m.groups {
                        if sod.lighting_material(&g.material).is_none() {
                            missing_group_mats += 1;
                        }
                        for f in &g.faces {
                            for c in f {
                                assert!((c.position as usize) < m.positions.len());
                                assert!((c.uv as usize) < m.uvs.len());
                            }
                        }
                    }
                    // De-indexing keeps every triangle and indexes in range.
                    let u = m.unindexed();
                    for (g, idx) in m.groups.iter().zip(&u.groups) {
                        assert_eq!(idx.len(), g.faces.len() * 3);
                        assert!(idx.iter().all(|&k| (k as usize) < u.vertices.len()));
                    }
                }
                NodeKind::Sprite => {
                    sprite_names.insert(n.sprite_node_id().to_ascii_lowercase());
                }
                NodeKind::Emitter { emitter } => {
                    emitter_names.insert(emitter.to_ascii_lowercase());
                }
                _ => {}
            }
        }
        for c in &sod.channels {
            *chan_types.entry((c.channel_type, c.ignored)).or_default() += 1;
            max_keys = max_keys.max(c.key_count());
            assert!(c.period.is_finite(), "{} channel period", it.name);
            match &c.keys {
                ChannelKeys::Matrix(k) => {
                    if k.iter().any(|m| !m.is_finite()) {
                        nonfinite.push(format!("{} channel {}", it.name, c.node));
                    }
                }
                ChannelKeys::Scale(k) => {
                    if k.iter().any(|v| !v.is_finite()) {
                        nonfinite.push(format!("{} channel {}", it.name, c.node));
                    }
                }
            }
        }
        for r in &sod.anim_refs {
            refs.insert((r.ref_type, r.animation.to_ascii_lowercase()));
            assert!(r.offset.is_finite());
        }
    }

    println!("SOD files: {} (+{} StarTrekDB older than 1.4, refused as the engine refuses them)", parsed, refused_old.len());
    println!("  stock originals parsed: {stock_parsed}");
    println!("  versions: {versions:?}");
    println!("  node types: {node_types:?}; hardpoints {hardpoints}, geometry nodes with LOD levels {lods}, mirrored nodes {mirrored}");
    println!("  channel (type, ignored byte): {chan_types:?}; most keys {max_keys}");
    println!("  mesh flags: {mesh_flags:?}; texture flags: {tex_flags:?}");
    println!("  1.91+ u16 field: {field_u16:?}; cull: {cull:?}; texture2 set: {texture2}; trailing lists: {trailing}; empty meshes: {empty_meshes}");
    println!("  texture materials: {texmats:?}");
    println!("  pre-1.7 meshes whose material ident is in the file's table: {} (not: {})", idents.0, idents.1);
    println!("  lighting models: {models:?}; face groups naming no material: {missing_group_mats}");
    println!("  sprite node names: {}, emitter names: {}, anim refs: {}", sprite_names.len(), emitter_names.len(), refs.len());
    println!("  refused (old): {refused_old:?}");
    println!("  unresolved parents ({}): {:?}", unresolved_parents.len(), unresolved_parents.iter().take(10).collect::<Vec<_>>());
    println!("  non-finite: {nonfinite:?}");
    assert!(failures.is_empty(), "SOD failures:\n{}", failures.join("\n"));
    assert!(nonfinite.is_empty());
    assert!(parsed > 400);
}

#[test]
fn sod_spot_checks() {
    let game = game_or_skip!();
    let read_stock = |name: &str| -> Vec<u8> {
        let dir = game.join("SOD");
        let found = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.to_ascii_lowercase().starts_with(&name.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        let pick = found
            .iter()
            .find(|n| n.ends_with("-backup"))
            .or_else(|| found.iter().find(|n| n.eq_ignore_ascii_case(name)))
            .unwrap_or_else(|| panic!("{name} not found"));
        fs::read(dir.join(pick)).unwrap()
    };

    // The Galaxy: a 1.93 ship with hardpoints, LOD groups, emitters and sprite nodes.
    let g = Sod::parse(&read_stock("fgalaxy.sod")).unwrap();
    assert_eq!(g.version, 1.93);
    for hp in ["hp01", "hp27", "hp28"] {
        let i = g.find(hp).unwrap_or_else(|| panic!("{hp}"));
        assert!(matches!(g.nodes[i].role(), Role::Hardpoint(_)));
        assert!(g.nodes[i].transform.is_finite());
    }
    let roots: Vec<&str> = g.roots().map(|r| g.nodes[r].name.as_str()).collect();
    println!("fgalaxy roots: {roots:?}");
    if let Some(geo) = g.nodes.iter().position(|n| n.role() == Role::Geometry) {
        let lv = g.lod_levels(geo);
        let faces: Vec<usize> = lv.iter().map(|&l| g.face_count_recursive(l)).collect();
        println!("fgalaxy LOD levels: {:?} faces {faces:?}", lv.iter().map(|&l| &g.nodes[l].name).collect::<Vec<_>>());
        assert!(faces.windows(2).all(|w| w[0] <= w[1]));
    }
    assert!(g.nodes.iter().any(|n| matches!(n.kind, NodeKind::Emitter { .. })));
    let meshes: Vec<_> = g.nodes.iter().filter_map(|n| n.mesh()).collect();
    assert!(!meshes.is_empty());
    assert!(meshes.iter().all(|m| m.diffuse_texture().is_some()));
    println!(
        "fgalaxy: {} nodes, {} meshes, {} faces; first mesh texture {:?} material {:?}",
        g.nodes.len(),
        meshes.len(),
        meshes.iter().map(|m| m.face_count()).sum::<usize>(),
        meshes[0].diffuse_texture(),
        meshes[0].texture_material
    );

    // A Borg hull: the bump path, a height map flagged 0x200 and the bump bit set.
    let b = Sod::parse(&read_stock("bassault.sod")).unwrap();
    let bump: Vec<_> = b.nodes.iter().filter_map(|n| n.mesh()).filter(|m| m.height_map().is_some()).collect();
    assert!(!bump.is_empty(), "bassault has a height-mapped mesh");
    assert!(bump.iter().all(|m| m.flags & a2fmt::sod::Mesh::FLAG_BUMP != 0));
    println!("bassault bump meshes: {}, flags {:#x}, height map {:?}", bump.len(), bump[0].flags, bump[0].height_map());

    // The loading-screen model: six quads carrying LOADING1..6.
    let logo = Sod::parse(&read_stock("logo.sod")).unwrap();
    let mut tex: Vec<String> = logo
        .nodes
        .iter()
        .filter_map(|n| n.mesh())
        .flat_map(|m| m.textures.iter().map(|t| t.name.to_ascii_uppercase()))
        .filter(|t| t.starts_with("LOADING"))
        .collect();
    tex.sort();
    tex.dedup();
    assert_eq!(tex, ["LOADING1", "LOADING2", "LOADING3", "LOADING4", "LOADING5", "LOADING6"]);
}

/// Texture files: name -> (installed path, stock path).
fn texture_files(game: &Path) -> BTreeMap<String, (Option<PathBuf>, PathBuf)> {
    let dir = game.join("Textures/RGB");
    let mut out: BTreeMap<String, (Option<PathBuf>, PathBuf)> = BTreeMap::new();
    for it in items(&dir, "tga") {
        let e = out.entry(it.name.clone()).or_insert((None, it.path.clone()));
        if it.path.file_name().unwrap().to_string_lossy() == it.name {
            e.0 = Some(it.path.clone());
            if it.stock {
                e.1 = it.path.clone();
            }
        } else {
            e.1 = it.path.clone();
        }
    }
    out
}

#[test]
fn tga_every_file() {
    let game = game_or_skip!();
    let files = texture_files(&game);
    let mut decoded = 0usize;
    let mut stock = 0usize;
    let mut kinds: BTreeMap<(u8, u8, u8, u8), usize> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut paths: Vec<(PathBuf, bool)> = Vec::new();
    for (inst, st) in files.values() {
        paths.push((st.clone(), true));
        if let Some(i) = inst {
            if i != st {
                paths.push((i.clone(), false));
            }
        }
    }
    for (p, is_stock) in &paths {
        let data = fs::read(p).unwrap();
        let h = match Header::parse(&data) {
            Ok(h) => h,
            Err(e) => {
                failures.push(format!("{}: {e:?}", p.display()));
                continue;
            }
        };
        if *is_stock {
            *kinds.entry((h.image_type, h.bits_per_pixel, h.descriptor, h.id_length)).or_default() += 1;
        }
        match tga::decode(&data) {
            Ok(img) => {
                assert_eq!(img.pixels.len(), (img.width * img.height * 4) as usize);
                assert_eq!((img.width, img.height), (h.width as u32, h.height as u32));
                if h.bits_per_pixel == 24 {
                    assert!(img.pixels.chunks(4).all(|p| p[3] == 255));
                }
                decoded += 1;
                if *is_stock {
                    stock += 1;
                }
            }
            Err(e) => failures.push(format!("{}: {e:?}", p.display())),
        }
    }
    println!("TGA: {} names, {} files decoded ({} stock originals)", files.len(), decoded, stock);
    println!("  stock (type, bpp, descriptor, id length): {kinds:?}");
    assert!(failures.is_empty(), "TGA failures:\n{}", failures.join("\n"));
    assert!(stock >= 2000, "expected the stock set, decoded {stock}");
    assert!(kinds.keys().all(|k| k.0 == 2), "a stock TGA is not type 2 (uncompressed)");
}

#[test]
fn tga_spot_check_orientation() {
    let game = game_or_skip!();
    let files = texture_files(&game);
    // A stock file decoded both ways must agree with a manual read of its first stored
    // row: the first row in the file is the image's bottom unless bit 5 is set.
    let mut checked = 0;
    for (_, st) in files.values().take(400) {
        let data = fs::read(st).unwrap();
        let h = Header::parse(&data).unwrap();
        let img = tga::decode(&data).unwrap();
        let bpp = (h.bits_per_pixel / 8) as usize;
        let off = h.data_offset();
        let first = &data[off..off + bpp];
        let y = if h.top_down() { 0 } else { img.height - 1 };
        let px = img.pixel(0, y);
        assert_eq!(&px[..3], &first[..3]);
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn tga_mip_chains() {
    let game = game_or_skip!();
    let files = texture_files(&game);
    let idx = TexIndex::new(files.keys());
    let width = |f: &str| -> Option<u32> {
        let (_, st) = files.get(f)?;
        let mut b = [0u8; 18];
        use std::io::Read;
        fs::File::open(st).ok()?.read_exact(&mut b).ok()?;
        Header::parse(&b).ok().map(|h| h.width as u32)
    };
    let mut chains = 0usize;
    let mut levels = 0usize;
    let mut level_names = BTreeSet::new();
    let mut strays = BTreeSet::new();
    for name in files.keys() {
        let chain = idx.mip_chain(name, width);
        if !chain.is_empty() {
            chains += 1;
            levels += chain.len();
            level_names.extend(chain.iter().map(|s| s.to_string()));
        }
        for s in idx.mip_strays(name, width) {
            strays.insert(s.to_string());
        }
    }
    // Some files are levels of one base and bases of a shorter chain themselves; count
    // the chains whose base is not itself a level.
    let roots = files
        .keys()
        .filter(|n| !level_names.contains(*n) && !idx.mip_chain(n, width).is_empty())
        .count();
    println!(
        "mip chains: {chains} bases with a level-1 sibling ({roots} not themselves levels), {levels} level links, {} distinct level files, {} named-like-a-level strays",
        level_names.len(),
        strays.len()
    );
    // Spellings the README names: an underscore chain and two bare ones.
    assert_eq!(idx.mip_chain("fcruise1_B", width).first().map(|s| s.to_ascii_lowercase()), Some("fcruise1_b1.tga".into()));
    assert_eq!(idx.mip_chain("fresearch", width).first().map(|s| s.to_ascii_lowercase()), Some("fresearch1.tga".into()));
    // Same-size placeholders are strays, not levels.
    assert!(idx.mip_chain("Fsensor_B", width).is_empty());
    assert!(!idx.mip_strays("Fsensor_B", width).is_empty());
    assert!(roots > 300);
}

fn ci_fetcher(dir: PathBuf) -> impl FnMut(&str) -> Option<Vec<u8>> {
    let map: BTreeMap<String, PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| (e.file_name().to_string_lossy().to_ascii_lowercase(), e.path()))
        .collect();
    move |n: &str| fs::read(map.get(&n.to_ascii_lowercase())?).ok()
}

#[test]
fn spr_every_file() {
    let game = game_or_skip!();
    let dir = game.join("Sprites");
    let mut files = items(&dir, "spr");
    files.retain(|i| i.stock);
    assert!(files.len() >= 30);
    let mut sprites = 0;
    let mut nodes = 0;
    let mut anims = 0;
    let mut emitters = 0;
    let mut warnings = Vec::new();
    for it in &files {
        let t = SpriteTable::parse(&fs::read(&it.path).unwrap());
        sprites += t.sprites.len();
        nodes += t.sprite_nodes.len();
        anims += t.animations.len();
        emitters += t.emitters.len();
        for w in &t.warnings {
            warnings.push(format!("{}:{} {}", it.name, w.line, w.message));
        }
        for s in &t.sprites {
            assert!(s.rect.iter().all(|v| v.is_finite()));
        }
    }
    println!("SPR: {} files, {sprites} sprites, {nodes} sprite nodes, {anims} animations, {emitters} emitters", files.len());
    println!("  warnings ({}): {warnings:#?}", warnings.len());

    // The 3D tables, includes followed.
    let t = SpriteTable::load("sprites.spr", ci_fetcher(dir.clone()));
    println!(
        "sprites.spr tree: {} sprites, {} nodes, {} animations, {} emitters; missing includes {:?}",
        t.sprites.len(),
        t.sprite_nodes.len(),
        t.animations.len(),
        t.emitters.len(),
        t.missing_includes
    );
    assert!(t.missing_includes.is_empty());
    let v = t.sprite_node("ventplas").expect("ventplas");
    assert_eq!(v.sprite, "plasma");
    assert!(v.billboard);
    let e = t.emitter("plasmaxl").expect("plasmaxl");
    assert_eq!(e.sprite.as_deref(), Some("ventplas"));
    let strobe = t.animation("strobe").expect("strobe");
    assert!(strobe.plays_evenly(), "stock flattens the strobe's double flash");
    let flip = t.animation("tex4x4_smoke").expect("tex4x4_smoke");
    assert_eq!(flip.auto_offsets().len(), 16);
    // Every animation a sprite node names exists, except the known stock gaps.
    let missing: Vec<_> = t
        .sprite_nodes
        .iter()
        .filter_map(|n| n.animation.as_deref())
        .filter(|a| t.animation(a).is_none())
        .collect();
    println!("  sprite-node animations not defined: {missing:?}");

    // Every SOD sprite node and emitter name against the tables.
    let mut sod_sprites = BTreeSet::new();
    let mut sod_emitters = BTreeSet::new();
    let mut sod_refs = BTreeSet::new();
    for it in items(&game.join("SOD"), "sod").iter().filter(|i| i.stock) {
        let Ok(s) = Sod::parse(&fs::read(&it.path).unwrap()) else { continue };
        for n in &s.nodes {
            match &n.kind {
                NodeKind::Sprite => {
                    sod_sprites.insert(n.sprite_node_id().to_string());
                }
                NodeKind::Emitter { emitter } => {
                    sod_emitters.insert(emitter.clone());
                }
                _ => {}
            }
        }
        for r in &s.anim_refs {
            sod_refs.insert(r.animation.clone());
        }
    }
    let unresolved_nodes: Vec<_> = sod_sprites.iter().filter(|n| t.sprite_node(n).is_none()).collect();
    let unresolved_emitters: Vec<_> = sod_emitters.iter().filter(|n| t.emitter(n).is_none()).collect();
    let unresolved_refs: Vec<_> = sod_refs.iter().filter(|n| t.animation(n).is_none()).collect();
    println!(
        "  SOD names: {} sprite nodes ({} unresolved: {:?}), {} emitters ({} unresolved), {} flipbooks ({} unresolved: {:?})",
        sod_sprites.len(),
        unresolved_nodes.len(),
        unresolved_nodes,
        sod_emitters.len(),
        unresolved_emitters.len(),
        sod_refs.len(),
        unresolved_refs.len(),
        unresolved_refs
    );
    assert!(unresolved_emitters.is_empty(), "{unresolved_emitters:?}");
    assert!(unresolved_nodes.len() <= 10, "{unresolved_nodes:?}");
}

#[test]
fn odf_every_file() {
    let game = game_or_skip!();
    let root = game.join("odf");
    let mut index: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x.eq_ignore_ascii_case("odf")).unwrap_or(false) {
                index.insert(e.file_name().to_string_lossy().to_ascii_lowercase(), p);
            }
        }
    }
    assert!(index.len() > 1000, "found {} ODFs", index.len());
    let fetch = |n: &str| fs::read(index.get(&n.to_ascii_lowercase())?).ok();
    let mut labels: BTreeMap<String, usize> = BTreeMap::new();
    let mut missing = BTreeSet::new();
    let mut strays = Vec::new();
    let mut dup_keys = 0usize;
    let mut entries = 0usize;
    for name in index.keys() {
        let odf = Odf::load(name, fetch).expect("loads");
        for m in &odf.missing {
            missing.insert(format!("{name} -> {m}"));
        }
        for l in &odf.file.stray_lines {
            strays.push(format!("{name}:{l}"));
        }
        entries += odf.file.entries.len();
        let mut seen = BTreeSet::new();
        for e in &odf.file.entries {
            if !seen.insert(e.key.to_ascii_lowercase()) {
                dup_keys += 1;
            }
        }
        *labels.entry(odf.class_label().unwrap_or_default()).or_default() += 1;
    }
    println!(
        "ODF: {} files, {entries} own entries, {} class labels ({} without), {dup_keys} keys written twice in one file",
        index.len(),
        labels.len(),
        labels.get("").copied().unwrap_or(0)
    );
    println!("  missing includes: {missing:?}");
    println!("  stray lines ({}): {:?}", strays.len(), strays.iter().take(20).collect::<Vec<_>>());
    assert!(missing.is_empty());

    // Spot checks.
    let g = Odf::load("fgalaxy.odf", fetch).unwrap();
    assert_eq!(g.class_label().as_deref(), Some("craft"));
    assert_eq!(g.get_str("weapon1"), Some("fgphas"));
    assert_eq!(g.get_strs("weaponHardpoints2"), ["hp27", "hp28"]);
    assert_eq!(g.get_i32("maxHealth"), Some(720));
    assert!(g.get_strs("possibleCraftNames").len() > 10);
    let a = Odf::load("aster.odf", fetch).unwrap();
    assert_eq!(a.get_str("baseName"), Some("sscr_1"));
    assert_eq!(a.file.entries[0].section, "GameObjectClass");
}

/// Damaged copies of real files: every parser must return, never panic.
#[test]
fn damaged_files_never_panic() {
    let game = game_or_skip!();
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut sods: Vec<Item> = items(&game.join("SOD"), "sod").into_iter().filter(|i| i.stock).collect();
    sods.truncate(40);
    let mut texs: Vec<Item> = items(&game.join("Textures/RGB"), "tga").into_iter().filter(|i| i.stock).collect();
    texs.truncate(40);
    let mut runs = 0;
    for it in sods.iter().chain(texs.iter()) {
        let data = fs::read(&it.path).unwrap();
        for _ in 0..50 {
            let mut d = data.clone();
            match rnd() % 3 {
                0 => d.truncate((rnd() as usize) % (d.len() + 1)),
                1 => {
                    for _ in 0..8 {
                        let i = (rnd() as usize) % d.len().max(1);
                        if i < d.len() {
                            d[i] = rnd() as u8;
                        }
                    }
                }
                _ => {
                    let i = (rnd() as usize) % d.len().max(1);
                    d.drain(i..(i + 7).min(d.len()));
                }
            }
            let _ = Sod::parse(&d);
            let _ = tga::decode(&d);
            let _ = SpriteTable::parse(&d);
            let _ = Odf::parse_single("x.odf", &d);
            runs += 1;
        }
    }
    println!("damaged inputs survived: {runs}");
}
