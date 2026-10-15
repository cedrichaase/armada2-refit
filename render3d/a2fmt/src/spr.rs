//! Sprite tables: the text files in `Sprites/` (`*.spr`).
//!
//! A table defines sprites (a rectangle of a texture), and around them:
//! `@sprite_node`s (the look of a SOD sprite node or of a particle: a sprite, an
//! animation, a size and a colour), `@animation`s (keyframed colour, visibility or
//! texture-offset tracks, also the texture flipbooks SOD meshes bind to), and
//! `@emitter`s (particle emitter templates). Directives such as `@reference=` and
//! `@tmaterial=` set state that applies to every sprite line after them.
//!
//! `sprites.spr` is the root of the 3D tables and `@include`s the rest; the GUI tables
//! and the fonts are loaded on their own. [`SpriteTable::load`] follows includes
//! through a caller-supplied closure; [`SpriteTable::parse`] reads one file alone.
//!
//! The parser is lenient, as the engine's is: a line it cannot read is recorded in
//! [`SpriteTable::warnings`] and skipped, never fatal.

use alloc::string::String;
use alloc::vec::Vec;

use crate::text::{lines, parse_f32, parse_i32, parse_tuple, strip_comment, tokens};

/// Colour-key transparency for the sprites after it (`@skip=`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    /// No `@skip` seen yet.
    Unset,
    /// `@skip=off`.
    Off,
    /// `@skip=(r,g,b)`: texels of this colour are transparent.
    Colour([u8; 3]),
}

/// One sprite: a rectangle of a texture.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub name: String,
    /// Texture name, no extension.
    pub texture: String,
    /// `u v w h` in texels of `reference`, not of the texture: a bigger texture
    /// keeps the same rectangle. Divide by `reference` for 0..1 coordinates.
    pub rect: [f32; 4],
    /// The texel space of `rect` (`@reference=`, `@referenceWidth=`,
    /// `@referenceHeight=`). `None` where the file has not set it.
    pub reference: [Option<f32>; 2],
    /// `@tmaterial=` in force: `interface`, `additive`, `default`, `font`,
    /// `alphathreshold`, `add_nomipf`, or empty when unset. Lower case.
    pub material: String,
    pub skip: Skip,
    /// `@anim=<id>` flags (up to two: a U and a V track).
    pub anims: Vec<String>,
    /// `@origin=(x,y)`: the hotspot, in texels.
    pub origin: Option<[f32; 2]>,
    pub cursor: bool,
    pub chromakey: bool,
    pub snap_to_pixel: bool,
    pub tile_u: bool,
    /// Any other `@flag` on the line, as written.
    pub other_flags: Vec<String>,
}

impl Sprite {
    /// `rect` as 0..1 texture coordinates, when the reference size is known.
    pub fn uv_rect(&self) -> Option<[f32; 4]> {
        let (rw, rh) = (self.reference[0]?, self.reference[1]?);
        if rw <= 0.0 || rh <= 0.0 {
            return None;
        }
        Some([self.rect[0] / rw, self.rect[1] / rh, self.rect[2] / rw, self.rect[3] / rh])
    }
}

/// `@sprite_node <id> <sprite> <animation|const> (w,h) (r,g,b) [billboard]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteNodeDef {
    pub id: String,
    pub sprite: String,
    /// `None` for `const`.
    pub animation: Option<String>,
    /// For a billboard a half extent (the quad is 2w by 2h); for an oriented quad the
    /// full extent in the node's XY plane.
    pub size: [f32; 2],
    pub colour: [f32; 3],
    /// The last word is exactly `billboard`; otherwise the quad is oriented.
    pub billboard: bool,
}

/// What an animation drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// Visibility; keys are integers (`0.8` reads as off).
    Draw,
    Colour,
    /// The texture offset (both axes); keys are texels of the current reference.
    Offset,
    UOffset,
    VOffset,
    /// A sprite's UV width (the `@tile_u` sprites).
    Width,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interp {
    Step,
    Linear,
    /// Linear, wrapping from the last key back to the first.
    TiledLinear,
}

/// `@auto=` key generation for flipbooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auto {
    Square,
    Row,
    Column,
    Scalar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyValue {
    Bool(bool),
    Colour([f32; 3]),
    /// Texels of the animation's reference size.
    Offset([f32; 2]),
    Scalar(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// Absolute time in seconds (a `+t` time is already added to the previous key's).
    /// `None` when the animation's length is 0 (then the keys carry no times).
    pub time: Option<f32>,
    pub value: KeyValue,
}

/// `@animation <id>`, its `<field> <nKeys> <length> <interp>` line, optional
/// `@auto=`, `@origin`, `@size`, and `@keyframes` with its rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub id: String,
    pub field: Field,
    /// The declared key count.
    pub key_count: u32,
    /// Seconds.
    pub length: f32,
    pub interp: Interp,
    pub auto: Option<Auto>,
    /// `@origin` for `@auto`, in 0..1 texture units.
    pub auto_origin: [f32; 2],
    /// `@size` for `@auto`, in 0..1 texture units, when given.
    pub auto_size: Option<[f32; 2]>,
    /// False when the `@keyframes` line is missing: the engine then leaves the
    /// animation with no keys, so it never animates (stock has two such).
    pub has_keyframes_line: bool,
    /// Explicit keys as written (empty for `@auto`).
    pub keys: Vec<Key>,
    /// The reference size in force where the animation was defined (offset keys are
    /// texels of it).
    pub reference: [Option<f32>; 2],
}

impl Animation {
    /// Offsets in 0..1 texture units for an `@auto` animation, as the engine
    /// generates them. `square` uses a grid of floor(sqrt(n)) and ignores the origin's
    /// y (keys past the grid stay at zero); `scalar` scrolls one axis (in `[0]`) and has
    /// one extra key for `tiled_linear`.
    pub fn auto_offsets(&self) -> Vec<[f32; 2]> {
        let Some(auto) = self.auto else { return Vec::new() };
        let n = self.key_count as usize;
        let o = self.auto_origin;
        let mut out = alloc::vec![[0.0f32; 2]; n];
        match auto {
            Auto::Square => {
                let g = (libm::floorf(libm::sqrtf(n as f32)) as usize).max(1);
                let s = self.auto_size.unwrap_or([1.0 / g as f32; 2]);
                for r in 0..g {
                    for c in 0..g {
                        let k = r * g + c;
                        if k < n {
                            out[k] = [o[0] + c as f32 * s[0], r as f32 * s[1]];
                        }
                    }
                }
            }
            Auto::Row => {
                let s = self.auto_size.unwrap_or([1.0 / n.max(1) as f32; 2]);
                for (i, v) in out.iter_mut().enumerate() {
                    *v = [o[0] + i as f32 * s[0], o[1]];
                }
            }
            Auto::Column => {
                let s = self.auto_size.unwrap_or([1.0 / n.max(1) as f32; 2]);
                for (i, v) in out.iter_mut().enumerate() {
                    *v = [o[0], o[1] + i as f32 * s[1]];
                }
            }
            Auto::Scalar => {
                let s = self.auto_size.unwrap_or([1.0 / n.max(1) as f32; 2]);
                let m = if self.interp == Interp::TiledLinear { n + 1 } else { n };
                out = (0..m).map(|i| [o[0] - i as f32 * s[0], 0.0]).collect();
            }
        }
        out
    }

    /// Whether the engine discards the explicit key times and plays the keys evenly
    /// spaced over `length`. Its test is one-sided: times are dropped when every gap
    /// is shorter than `length / n + 1e-5`, so authored gaps that are *shorter* than
    /// even (the double-flash strobes) are flattened too. This is the stock look of
    /// every running light.
    pub fn plays_evenly(&self) -> bool {
        let n = self.keys.len();
        if n < 2 || self.keys.iter().any(|k| k.time.is_none()) {
            return true;
        }
        let even = self.length / n as f32;
        self.keys
            .windows(2)
            .all(|w| (w[1].time.unwrap_or(0.0) - w[0].time.unwrap_or(0.0)) < even + 1e-5)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterType {
    Spray,
    Cloud,
    Beam,
}

/// `@emitter <id>` ... `@end_emitter`. Every number defaults to 0.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterDef {
    pub id: String,
    pub emitter_type: EmitterType,
    /// A `@sprite_node` id. Without one the engine creates no emitter.
    pub sprite: Option<String>,
    /// Stored by the engine, never used.
    pub pixel: [f32; 3],
    pub velocity: f32,
    pub expansion_rate: f32,
    pub lifespan: f32,
    pub firing_period: f32,
    /// Degrees.
    pub spread_angle: f32,
    pub particle_count: i32,
    pub cloud_radius: f32,
    /// Keys the engine does not know, as written.
    pub unknown: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    /// The file it came from, as named to the loader (empty for [`SpriteTable::parse`]).
    pub file: String,
    /// 1-based.
    pub line: usize,
    pub message: &'static str,
}

/// Directive state that applies to the sprite lines after it.
#[derive(Clone, Debug)]
struct State {
    reference: [Option<f32>; 2],
    material: String,
    skip: Skip,
}

impl Default for State {
    fn default() -> Self {
        State { reference: [None, None], material: String::new(), skip: Skip::Unset }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpriteTable {
    pub sprites: Vec<Sprite>,
    pub sprite_nodes: Vec<SpriteNodeDef>,
    pub animations: Vec<Animation>,
    pub emitters: Vec<EmitterDef>,
    /// `@include`d names, in the order met (as written).
    pub includes: Vec<String>,
    /// Files an include named that the loader could not supply.
    pub missing_includes: Vec<String>,
    pub warnings: Vec<Warning>,
}

impl SpriteTable {
    /// One file, includes not followed (they are listed in `includes`).
    pub fn parse(data: &[u8]) -> SpriteTable {
        let mut t = SpriteTable::default();
        let mut st = State::default();
        let mut none = |_: &str| None;
        t.parse_into(data, "", &mut st, &mut none, 0);
        t
    }

    /// A file and everything it includes, depth first in place, with directive state
    /// carried across includes as one parser would. `fetch` returns a file's bytes by
    /// the name written after `@include` (look it up case-insensitively).
    pub fn load<F>(root: &str, mut fetch: F) -> SpriteTable
    where
        F: FnMut(&str) -> Option<Vec<u8>>,
    {
        let mut t = SpriteTable::default();
        let mut st = State::default();
        match fetch(root) {
            Some(d) => t.parse_into(&d, root, &mut st, &mut fetch, 0),
            None => t.missing_includes.push(String::from(root)),
        }
        t
    }

    pub fn sprite(&self, name: &str) -> Option<&Sprite> {
        // A later definition of the same name replaces the earlier.
        self.sprites.iter().rev().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn sprite_node(&self, id: &str) -> Option<&SpriteNodeDef> {
        self.sprite_nodes.iter().rev().find(|s| s.id.eq_ignore_ascii_case(id))
    }

    pub fn animation(&self, id: &str) -> Option<&Animation> {
        self.animations.iter().rev().find(|s| s.id.eq_ignore_ascii_case(id))
    }

    pub fn emitter(&self, id: &str) -> Option<&EmitterDef> {
        self.emitters.iter().rev().find(|s| s.id.eq_ignore_ascii_case(id))
    }

    fn warn(&mut self, file: &str, line: usize, message: &'static str) {
        self.warnings.push(Warning { file: String::from(file), line, message });
    }

    fn parse_into(
        &mut self,
        data: &[u8],
        file: &str,
        st: &mut State,
        fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
        depth: u32,
    ) {
        let all = lines(data);
        let mut i = 0;
        while i < all.len() {
            let lno = i + 1;
            let line = strip_comment(&all[i], true).trim();
            i += 1;
            if line.is_empty() {
                continue;
            }
            let toks = tokens(line);
            let head = toks[0].as_str();
            let lower = head.to_ascii_lowercase();
            if lower == "sprite_table" {
                continue;
            }
            if let Some(d) = lower.strip_prefix('@') {
                let (key, val) = match d.find('=') {
                    Some(at) => (&d[..at], Some(&head[at + 2..])),
                    None => (d, None),
                };
                match key {
                    "include" => {
                        let Some(name) = toks.get(1) else {
                            self.warn(file, lno, "@include without a name");
                            continue;
                        };
                        self.includes.push(name.clone());
                        if depth >= 16 {
                            self.warn(file, lno, "@include nested too deep");
                            continue;
                        }
                        match fetch(name) {
                            Some(d) => self.parse_into(&d, name, st, fetch, depth + 1),
                            None => self.missing_includes.push(name.clone()),
                        }
                    }
                    "reference" | "referencewidth" | "referenceheight" => {
                        match val.and_then(parse_f32) {
                            Some(v) => {
                                if key != "referenceheight" {
                                    st.reference[0] = Some(v);
                                }
                                if key != "referencewidth" {
                                    st.reference[1] = Some(v);
                                }
                            }
                            None => self.warn(file, lno, "unreadable reference size"),
                        }
                    }
                    "tmaterial" => match val {
                        Some(v) => st.material = v.to_ascii_lowercase(),
                        None => self.warn(file, lno, "@tmaterial without a value"),
                    },
                    "skip" => match val {
                        Some(v) if v.eq_ignore_ascii_case("off") => st.skip = Skip::Off,
                        Some(v) => match parse_tuple::<3>(v) {
                            Some(c) => {
                                st.skip = Skip::Colour(c.map(|x| x.clamp(0.0, 255.0) as u8))
                            }
                            None => self.warn(file, lno, "unreadable @skip"),
                        },
                        None => self.warn(file, lno, "@skip without a value"),
                    },
                    "sprite_node" => self.sprite_node_line(&toks, file, lno),
                    "animation" => i = self.animation_block(&all, i, &toks, file, lno, st),
                    "emitter" => i = self.emitter_block(&all, i, &toks, file, lno),
                    // Notes the engine reads past.
                    "define" | "todo" => {}
                    _ => self.warn(file, lno, "unknown directive"),
                }
                continue;
            }
            self.sprite_line(&toks, file, lno, st);
        }
    }

    fn sprite_line(&mut self, toks: &[String], file: &str, lno: usize, st: &State) {
        if toks.len() < 6 {
            self.warn(file, lno, "sprite line with fewer than six fields");
            return;
        }
        let mut rect = [0.0f32; 4];
        for k in 0..4 {
            match parse_f32(&toks[2 + k]) {
                Some(v) => rect[k] = v,
                None => {
                    self.warn(file, lno, "sprite rectangle is not numeric");
                    return;
                }
            }
        }
        let mut s = Sprite {
            name: toks[0].clone(),
            texture: toks[1].clone(),
            rect,
            reference: st.reference,
            material: st.material.clone(),
            skip: st.skip,
            anims: Vec::new(),
            origin: None,
            cursor: false,
            chromakey: false,
            snap_to_pixel: false,
            tile_u: false,
            other_flags: Vec::new(),
        };
        for t in &toks[6..] {
            let l = t.to_ascii_lowercase();
            if let Some(a) = l.strip_prefix("@anim=") {
                let _ = a;
                s.anims.push(String::from(&t[6..]));
            } else if let Some(o) = l.strip_prefix("@origin=") {
                s.origin = parse_tuple::<2>(o);
                if s.origin.is_none() {
                    self.warn(file, lno, "unreadable @origin");
                }
            } else {
                match l.as_str() {
                    "@cursor" => s.cursor = true,
                    "@chromakey" => s.chromakey = true,
                    "@snap_to_pixel" => s.snap_to_pixel = true,
                    "@tile_u" => s.tile_u = true,
                    _ => s.other_flags.push(t.clone()),
                }
            }
        }
        self.sprites.push(s);
    }

    fn sprite_node_line(&mut self, toks: &[String], file: &str, lno: usize) {
        if toks.len() < 6 {
            self.warn(file, lno, "@sprite_node with fewer than five fields");
            return;
        }
        let size = parse_tuple::<2>(&toks[4]);
        let colour = parse_tuple::<3>(&toks[5]);
        if size.is_none() || colour.is_none() {
            self.warn(file, lno, "@sprite_node size or colour unreadable");
        }
        let anim = &toks[3];
        self.sprite_nodes.push(SpriteNodeDef {
            id: toks[1].clone(),
            sprite: toks[2].clone(),
            animation: if anim.eq_ignore_ascii_case("const") { None } else { Some(anim.clone()) },
            size: size.unwrap_or([5.0, 5.0]),
            colour: colour.unwrap_or([1.0, 1.0, 1.0]),
            billboard: toks.get(6).map(|t| t == "billboard").unwrap_or(false),
        });
    }

    /// Reads an `@animation` block starting after its header line; returns the index of
    /// the next line to read.
    fn animation_block(
        &mut self,
        all: &[String],
        mut i: usize,
        head: &[String],
        file: &str,
        lno: usize,
        st: &State,
    ) -> usize {
        let Some(id) = head.get(1) else {
            self.warn(file, lno, "@animation without an id");
            return i;
        };
        // The definition line: the next non-empty, non-comment line.
        let mut def: Option<Vec<String>> = None;
        while i < all.len() {
            let l = strip_comment(&all[i], true).trim();
            i += 1;
            if !l.is_empty() {
                def = Some(tokens(l));
                break;
            }
        }
        let Some(def) = def else {
            self.warn(file, lno, "@animation at end of file");
            return i;
        };
        let field = match def[0].to_ascii_lowercase().as_str() {
            "draw" => Field::Draw,
            "colour" | "color" => Field::Colour,
            "offset" => Field::Offset,
            "u_offset" => Field::UOffset,
            "v_offset" => Field::VOffset,
            "width" => Field::Width,
            _ => {
                self.warn(file, lno, "@animation with an unknown field");
                return i - 1;
            }
        };
        let key_count = def.get(1).and_then(|t| parse_i32(t)).unwrap_or(0).max(0) as u32;
        let length = def.get(2).and_then(|t| parse_f32(t)).unwrap_or(0.0);
        let interp = match def.get(3).map(|t| t.to_ascii_lowercase()) {
            Some(t) if t == "linear" => Interp::Linear,
            Some(t) if t == "tiled_linear" => Interp::TiledLinear,
            _ => Interp::Step,
        };
        if key_count < 2 {
            self.warn(file, lno, "@animation with fewer than two keys (the engine refuses it)");
        }
        let mut anim = Animation {
            id: id.clone(),
            field,
            key_count,
            length,
            interp,
            auto: None,
            auto_origin: [0.0, 0.0],
            auto_size: None,
            has_keyframes_line: false,
            keys: Vec::new(),
            reference: st.reference,
        };
        // Options up to @keyframes. Anything else ends the block without keys.
        while i < all.len() {
            let l = strip_comment(&all[i], true).trim();
            if l.is_empty() {
                i += 1;
                continue;
            }
            let lt = l.to_ascii_lowercase();
            if lt.starts_with("@keyframes") {
                anim.has_keyframes_line = true;
                i += 1;
                break;
            } else if let Some(a) = lt.strip_prefix("@auto=") {
                anim.auto = match a.trim() {
                    "square" => Some(Auto::Square),
                    "row" => Some(Auto::Row),
                    "column" => Some(Auto::Column),
                    "scalar" => Some(Auto::Scalar),
                    _ => {
                        self.warn(file, i + 1, "unknown @auto");
                        None
                    }
                };
                i += 1;
            } else if lt.starts_with("@origin") || lt.starts_with("@size") {
                let rest = l[if lt.starts_with("@origin") { 7 } else { 5 }..]
                    .trim_start_matches(['=', ' ', '\t']);
                let v = parse_tuple::<2>(&rest.replace([' ', '\t'], ""));
                match (lt.starts_with("@origin"), v) {
                    (true, Some(v)) => anim.auto_origin = v,
                    (false, Some(v)) => anim.auto_size = Some(v),
                    _ => self.warn(file, i + 1, "unreadable @origin/@size"),
                }
                i += 1;
            } else {
                break;
            }
        }
        if !anim.has_keyframes_line {
            self.warn(file, lno, "@animation without @keyframes (it never animates)");
            self.animations.push(anim);
            return i;
        }
        if anim.auto.is_none() {
            let timed = length > 0.0;
            let mut last_t = 0.0f32;
            while (anim.keys.len() as u32) < key_count && i < all.len() {
                let l = strip_comment(&all[i], true).trim();
                if l.starts_with('@') {
                    break;
                }
                i += 1;
                if l.is_empty() {
                    continue;
                }
                let t = tokens(l);
                let mut at = 0;
                let time = if timed {
                    let tt = &t[0];
                    let rel = tt.starts_with('+');
                    let Some(v) = parse_f32(tt.trim_start_matches('+')) else {
                        self.warn(file, i, "keyframe time unreadable");
                        continue;
                    };
                    at = 1;
                    last_t = if rel { last_t + v } else { v };
                    Some(last_t)
                } else {
                    None
                };
                let arg = |k: usize| t.get(at + k).map(|s| s.as_str());
                let value = match field {
                    // An integer read: `0.8` and `.8` are 0, so off.
                    Field::Draw => arg(0).and_then(parse_f32).map(|v| KeyValue::Bool(v as i32 != 0)),
                    Field::Colour => arg(0).and_then(parse_tuple::<3>).map(KeyValue::Colour),
                    Field::Offset => match (arg(0).and_then(parse_i32), arg(1).and_then(parse_i32)) {
                        (Some(u), Some(v)) => Some(KeyValue::Offset([u as f32, v as f32])),
                        _ => None,
                    },
                    Field::UOffset | Field::VOffset | Field::Width => {
                        arg(0).and_then(parse_f32).map(KeyValue::Scalar)
                    }
                };
                match value {
                    Some(value) => anim.keys.push(Key { time, value }),
                    None => self.warn(file, i, "keyframe value unreadable"),
                }
            }
            if (anim.keys.len() as u32) < key_count {
                self.warn(file, lno, "@animation has fewer keys than declared");
            }
        }
        self.animations.push(anim);
        i
    }

    fn emitter_block(
        &mut self,
        all: &[String],
        mut i: usize,
        head: &[String],
        file: &str,
        lno: usize,
    ) -> usize {
        let mut e = EmitterDef {
            id: head.get(1).cloned().unwrap_or_default(),
            emitter_type: EmitterType::Spray,
            sprite: None,
            pixel: [1.0, 1.0, 1.0],
            velocity: 0.0,
            expansion_rate: 0.0,
            lifespan: 0.0,
            firing_period: 0.0,
            spread_angle: 0.0,
            particle_count: 0,
            cloud_radius: 0.0,
            unknown: Vec::new(),
        };
        if e.id.is_empty() {
            self.warn(file, lno, "@emitter without an id");
        }
        let mut closed = false;
        while i < all.len() {
            let l = strip_comment(&all[i], true).trim();
            i += 1;
            if l.is_empty() {
                continue;
            }
            if l.to_ascii_lowercase().starts_with("@end_emitter") {
                closed = true;
                break;
            }
            let Some(eq) = l.find('=') else {
                self.warn(file, i, "emitter line without '='");
                continue;
            };
            let key = l[..eq].trim().to_ascii_lowercase();
            let val = l[eq + 1..].trim();
            let f = || parse_f32(val).unwrap_or(0.0);
            match key.as_str() {
                "type" => {
                    e.emitter_type = match val.to_ascii_lowercase().as_str() {
                        "cloud" => EmitterType::Cloud,
                        "beam" => EmitterType::Beam,
                        _ => EmitterType::Spray,
                    }
                }
                "sprite" => e.sprite = Some(String::from(val)),
                "pixel" => e.pixel = parse_tuple::<3>(&val.replace([' ', '\t'], "")).unwrap_or(e.pixel),
                "velocity" => e.velocity = f(),
                "expansion_rate" => e.expansion_rate = f(),
                "lifespan" => e.lifespan = f(),
                "firing_period" => e.firing_period = f(),
                "spread_angle" => e.spread_angle = f(),
                "particle_count" => e.particle_count = parse_i32(val).unwrap_or(0),
                "cloud_radius" => e.cloud_radius = f(),
                _ => e.unknown.push((key, String::from(val))),
            }
        }
        if !closed {
            self.warn(file, lno, "@emitter without @end_emitter");
        }
        self.emitters.push(e);
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "sprite_table\r\n\
@reference=64\r\n\
@tmaterial=additive\r\n\
softdot\t\txparticle\t0\t0\t32\t32\r\n\
@referenceWidth=128\r\n\
plasma xsmoke 0 32 32 32 @anim=tex4x4 @origin=(16,16) @cursor\r\n\
@sprite_node ventplas plasma fade3 (2,2) (1,1,1) billboard\r\n\
@sprite_node crew1 plasma const (2,2) (1,1,1) //billboard\r\n\
@animation strobe\r\n\
draw 4 2.0 step\r\n\
@keyframes\r\n\
0.0\t1\r\n\
0.1 0\r\n\
+0.1\t0.8 # reads as off\r\n\
0.3\t0\r\n\
@animation tex4x4\r\n\
offset 16 4.0 step\r\n\
@auto=square\r\n\
@keyframes\r\n\
@animation broken\r\n\
colour 2 1 linear\r\n\
@emitter rift\r\n\
type=cloud\r\n\
sprite=ventplas\r\n\
spread_angle=3.0\r\n\
particle_count=30\r\n\
@end_emitter\r\n";

    #[test]
    fn sample() {
        let t = SpriteTable::parse(SAMPLE.as_bytes());
        assert_eq!(t.sprites.len(), 2);
        assert_eq!(t.sprites[0].reference, [Some(64.0), Some(64.0)]);
        assert_eq!(t.sprites[1].reference, [Some(128.0), Some(64.0)]);
        assert_eq!(t.sprites[1].origin, Some([16.0, 16.0]));
        assert!(t.sprites[1].cursor);
        assert_eq!(t.sprites[0].material, "additive");
        assert!(t.sprite_nodes[0].billboard);
        assert!(!t.sprite_nodes[1].billboard);
        let s = t.animation("strobe").unwrap();
        assert_eq!(s.keys.len(), 4);
        assert_eq!(s.keys[2].time, Some(0.2));
        assert_eq!(s.keys[2].value, KeyValue::Bool(false));
        assert!(s.plays_evenly());
        let f = t.animation("tex4x4").unwrap();
        assert_eq!(f.auto_offsets()[5], [0.25, 0.25]);
        assert!(!t.animation("broken").unwrap().has_keyframes_line);
        let e = t.emitter("rift").unwrap();
        assert_eq!(e.emitter_type, EmitterType::Cloud);
        assert_eq!(e.particle_count, 30);
        assert_eq!(e.sprite.as_deref(), Some("ventplas"));
    }

    #[test]
    fn includes() {
        let t = SpriteTable::load("root.spr", |n| match n {
            "root.spr" => Some(b"@reference=32\n@include a.spr\nb x 0 0 1 1\n".to_vec()),
            "a.spr" => Some(b"a y 0 0 1 1\n@include nope.spr\n".to_vec()),
            _ => None,
        });
        assert_eq!(t.sprites.len(), 2);
        assert_eq!(t.sprites[0].name, "a");
        assert_eq!(t.sprites[0].reference[0], Some(32.0));
        assert_eq!(t.missing_includes, ["nope.spr"]);
    }
}
