//! The game's assets, read by us (`a2fmt`) from the game directory: SODs, textures
//! with their mip chains, the sprite table and ODFs. Parsed data is kept for the
//! session; textures live on the device in the default pool (no address space held:
//! the process is not large-address-aware) and are dropped before a `Reset`.
//!
//! Names are looked up case-insensitively, as the engine (on Windows) does, through
//! an index of each directory made once.
use crate::d3d::*;
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
use core::ffi::c_void;

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
    sprites: Option<Rc<SpriteTable>>,
    widths: BTreeMap<String, Option<u32>>,
    textures: BTreeMap<String, Option<Com>>,
    dev: Option<Device>,
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
                sprites: None,
                widths: BTreeMap::new(),
                textures: BTreeMap::new(),
                dev: None,
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

/// Whether a texture of that name exists (`Textures/RGB/<name>.tga`).
pub fn texture_exists(name: &str) -> bool {
    store().tex_index.find(name).is_some()
}

/// A texture's base level decoded (BGRA8, rows top-down), by name; not cached.
pub fn image(name: &str) -> Option<tga::Image> {
    let s = store();
    let base = String::from(s.tex_index.find(name)?);
    tga::decode(&s.tex_dir.read(&base)?).ok()
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

/// A texture by name (no extension), on `dev`, with its whole chain: stock's own
/// levels where it has them (`mip_name()`'s rule), the rest box-filtered. None if
/// there is no such texture. The pointer is valid until the next `reset()`.
pub fn texture(dev: Device, name: &str) -> Option<*mut c_void> {
    let s = store();
    if s.dev != Some(dev) {
        s.textures.clear();
        s.dev = Some(dev);
    }
    let key = rt::lower(name);
    if let Some(t) = s.textures.get(&key) {
        return t.as_ref().map(|c| c.ptr());
    }
    let t = load_texture(s, dev, &key);
    if t.is_none() {
        log!("assets: texture {} not loaded", key);
    }
    let p = t.as_ref().map(|c| c.ptr());
    s.textures.insert(key, t);
    p
}

fn load_texture(s: &mut Store, dev: Device, name: &str) -> Option<Com> {
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
    let refs: Vec<&[u8]> = levels.iter().map(|l| l.as_slice()).collect();
    upload_texture(dev, w, h, FMT_A8R8G8B8, &refs)
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
