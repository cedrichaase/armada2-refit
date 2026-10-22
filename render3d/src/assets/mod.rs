//! The game's assets, read by us (`a2fmt`) from the game directory: SODs, textures
//! with their mip chains, the sprite table and ODFs. Parsed data is kept for the
//! session; textures live on the device in the default pool (no address space held:
//! the process is not large-address-aware) and are dropped before a `Reset`.
//!
//! Names are looked up case-insensitively, as the engine (on Windows) does, through
//! an index of each directory made once.

use crate::log;
use crate::rt;
use a2fmt::odf::Odf;
use a2fmt::sod::Sod;
use a2fmt::spr::SpriteTable;
use a2fmt::tga::{self, TexIndex};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;


/// Every file under a game directory (recursively), by lower-case file name, to its
/// path relative to the game directory.
struct DirIndex {
    files: BTreeMap<String, String>,
}

impl DirIndex {
    fn build(dir: &str) -> DirIndex {
        let mut files = BTreeMap::new();
        walk(dir, &mut files, 0);
        DirIndex { files }
    }
    fn path(&self, name: &str) -> Option<&str> {
        self.files.get(&rt::lower(name)).map(|s| s.as_str())
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        rt::read_file(&rt::game_path(self.path(name)?))
    }
}

fn walk(rel: &str, out: &mut BTreeMap<String, String>, depth: u32) {
    if depth > 6 {
        return;
    }
    let mut pat = rt::game_path(rel);
    pat.push_str("\\*");
    for (name, dir) in rt::list_entries(&pat) {
        if name == "." || name == ".." {
            continue;
        }
        let mut p = String::from(rel);
        p.push('\\');
        p.push_str(&name);
        if dir {
            walk(&p, out, depth + 1);
        } else {
            // The first found wins: a top-level file over one in a subdirectory.
            out.entry(rt::lower(&name)).or_insert(p);
        }
    }
}

struct Store {
    sod_dir: DirIndex,
    tex_dir: DirIndex,
    tex_index: TexIndex,
    spr_dir: DirIndex,
    odf_dir: DirIndex,
    sods: BTreeMap<String, Option<Rc<Sod>>>,
    odfs: BTreeMap<String, Option<Rc<Odf>>>,
    bases: BTreeMap<String, String>,
    sprites: Option<Rc<SpriteTable>>,
    widths: BTreeMap<String, Option<u32>>,
    textures: BTreeMap<String, Option<gpu::Texture>>,
}

static mut STORE: Option<Store> = None;

#[allow(static_mut_refs)]
fn store() -> &'static mut Store {
    unsafe {
        if STORE.is_none() {
            let t0 = rt::now();
            let tex_dir = DirIndex::build("Textures\\RGB");
            let tex_index = TexIndex::new(tex_dir.files.keys().cloned());
            let s = Store {
                sod_dir: DirIndex::build("SOD"),
                tex_dir,
                tex_index,
                spr_dir: DirIndex::build("Sprites"),
                odf_dir: DirIndex::build("odf"),
                sods: BTreeMap::new(),
                odfs: BTreeMap::new(),
                bases: BTreeMap::new(),
                sprites: None,
                widths: BTreeMap::new(),
                textures: BTreeMap::new(),
            };
            log!(
                "assets: {} SODs, {} textures, {} sprite files, {} ODFs indexed in {:.0} ms",
                s.sod_dir.files.len(), s.tex_dir.files.len(), s.spr_dir.files.len(), s.odf_dir.files.len(),
                (rt::now() - t0) * 1000.0
            );
            STORE = Some(s);
        }
        STORE.as_mut().unwrap()
    }
}

/// Reads a SOD by name (`fgalaxy` or `fgalaxy.sod`), parsed once.
pub fn sod(name: &str) -> Option<Rc<Sod>> {
    let s = store();
    let mut key = rt::lower(name);
    if !key.ends_with(".sod") {
        key.push_str(".sod");
    }
    if let Some(v) = s.sods.get(&key) {
        return v.clone();
    }
    let v = match s.sod_dir.read(&key) {
        Some(d) => match Sod::parse(&d) {
            Ok(sod) => Some(Rc::new(sod)),
            Err(e) => {
                log!("assets: {} does not parse: {:?}", key, e);
                None
            }
        },
        None => {
            log!("assets: no {}", key);
            None
        }
    };
    s.sods.insert(key, v.clone());
    v
}

/// An ODF and everything it includes, by name (`fgalaxy` or `fgalaxy.odf`).
pub fn odf(name: &str) -> Option<Rc<Odf>> {
    let s = store();
    let mut key = rt::lower(name);
    if !key.ends_with(".odf") {
        key.push_str(".odf");
    }
    if let Some(v) = s.odfs.get(&key) {
        return v.clone();
    }
    let dir = &s.odf_dir;
    let v = Odf::load(&key, |n| dir.read(n)).ok().map(Rc::new);
    s.odfs.insert(key, v.clone());
    v
}

/// The SOD an ODF's objects draw, lower case and without extension: its `baseName`
/// when it sets one (`kmartok` draws `kbattle`, `kprison` draws `kbase`), else its own
/// name. Every SOD looked up for a game object goes through this; `ScaleSOD` and the
/// rest stay the ODF's.
pub fn sod_of(name: &str) -> String {
    let key = rt::lower(name.strip_suffix(".odf").or(name.strip_suffix(".ODF")).unwrap_or(name));
    if let Some(v) = store().bases.get(&key) {
        return v.clone();
    }
    let base = odf(&key).and_then(|o| o.get_str("baseName").map(rt::lower)).filter(|b| !b.is_empty());
    let v = match base {
        Some(b) => {
            let b = String::from(b.strip_suffix(".sod").unwrap_or(&b));
            log!("assets: {} draws {}.sod (baseName)", key, b);
            b
        }
        None => key.clone(),
    };
    store().bases.insert(key, v.clone());
    v
}

/// The sprite table: `sprites.spr` and everything it includes.
pub fn sprites() -> Rc<SpriteTable> {
    let s = store();
    if s.sprites.is_none() {
        let dir = &s.spr_dir;
        let t = SpriteTable::load("sprites.spr", |n| dir.read(n));
        log!(
            "assets: sprite table: {} sprites, {} sprite nodes, {} animations, {} emitters",
            t.sprites.len(), t.sprite_nodes.len(), t.animations.len(), t.emitters.len()
        );
        s.sprites = Some(Rc::new(t));
    }
    s.sprites.clone().unwrap()
}

/// A texture's base image decoded (BGRA8, top-down), by name: for a module that makes
/// something of its own from it (a normal map from a height map, a mean colour).
pub fn image(name: &str) -> Option<tga::Image> {
    let s = store();
    let base = String::from(s.tex_index.find(name)?);
    tga::decode(&s.tex_dir.read(&base)?).ok()
}

/// Whether a texture of that name exists (`Textures/RGB/<name>.tga`).
pub fn texture_exists(name: &str) -> bool {
    store().tex_index.find(name).is_some()
}

fn tex_width(s: &mut Store, file: &str) -> Option<u32> {
    let key = rt::lower(file);
    if let Some(w) = s.widths.get(&key) {
        return *w;
    }
    let w = s.tex_dir.path(&key).and_then(|p| {
        // The header is all we need: 18 bytes.
        let d = rt::read_file(&rt::game_path(p))?;
        tga::Header::parse(&d).ok().map(|h| h.width as u32)
    });
    s.widths.insert(key, w);
    w
}

/// A texture by name (no extension), with its whole chain: stock's own levels where it
/// has them (`mip_name()`'s rule), the rest box-filtered. None if there is no such
/// texture. Made on the device at once, holding no copy of its texels (`Transient`):
/// it is loaded again after the next `reset()`.
pub fn texture(name: &str) -> Option<gpu::Texture> {
    let s = store();
    let key = rt::lower(name);
    if let Some(t) = s.textures.get(&key) {
        return t.clone();
    }
    let t = load_texture(s, &key).filter(|t| t.ready(crate::gfx::gpu()));
    if t.is_none() {
        log!("assets: texture {} not loaded", key);
    }
    s.textures.insert(key, t.clone());
    t
}

fn load_texture(s: &mut Store, name: &str) -> Option<gpu::Texture> {
    let base = String::from(s.tex_index.find(name)?);
    let img = tga::decode(&s.tex_dir.read(&base)?).ok()?;
    let (w, h) = (img.width, img.height);
    if w == 0 || h == 0 || !w.is_power_of_two() || !h.is_power_of_two() {
        return None;
    }
    let chain: Vec<String> = {
        let idx = &s.tex_index as *const TexIndex;
        // The width lookup reads files through the store; the index is not touched.
        let names = unsafe { &*idx }.mip_chain(&base, |f| tex_width(s, f));
        names.into_iter().map(String::from).collect()
    };
    let mut levels: Vec<Vec<u8>> = Vec::new();
    levels.push(img.pixels);
    let (mut lw, mut lh) = (w, h);
    while lw > 1 || lh > 1 {
        let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
        let k = levels.len();
        let stock = chain.get(k - 1).and_then(|f| s.tex_dir.read(f)).and_then(|d| tga::decode(&d).ok());
        let next = match stock {
            Some(i) if i.width == nw && i.height == nh => i.pixels,
            _ => halve(&levels[k - 1], lw, lh),
        };
        levels.push(next);
        lw = nw;
        lh = nh;
    }
    let n = levels.len() as u32;
    let mut px = Vec::with_capacity(levels.iter().map(|l| l.len()).sum());
    for l in &levels {
        px.extend_from_slice(l);
    }
    drop(levels);
    let d = gpu::TextureDesc::d2(w, h, gpu::Format::Bgra8, gpu::Usage::SAMPLED).with_levels(n);
    Some(crate::gfx::gpu().texture(d, gpu::Contents::Transient(px)))
}

/// A 2x2 box filter of a BGRA8 image.
fn halve(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let mut sum = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    sum += src[((sy * w + sx) * 4 + c) as usize] as u32;
                }
                out.push(((sum + 2) / 4) as u8);
            }
        }
    }
    out
}

/// Drops everything on the device (before a `Reset`).
pub fn reset() {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(s) = STORE.as_mut() {
            s.textures.clear();
        }
    }
}
