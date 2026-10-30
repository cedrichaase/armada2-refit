//! SOD: the engine's binary model format (`SOD/*.sod`).
//!
//! One file is one model database: a table of lighting materials, a tree of nodes
//! (transforms, meshes, sprite nodes, particle emitters, lights), animation channels
//! that key node transforms or scales, and references that bind a mesh to a texture
//! flipbook from the sprite tables. `README.md` describes the layout field by field.
//!
//! The engine reads two containers, `Storm3D_SW` and the older `StarTrekDB`, with the
//! same reader, and accepts versions 1.4 to 1.93 inclusive: older and newer files are
//! refused at load, so a `StarTrekDB` 1.0 or 1.2 file never draws in the game. Which
//! fields are present depends on the version, compared as an `f32` exactly as stored.

use alloc::string::String;
use alloc::vec::Vec;

use crate::bytes::{Eof, Reader};
use crate::math::Matrix34;

/// Why a file did not parse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    /// Neither `Storm3D_SW` nor `StarTrekDB`.
    BadMagic,
    /// Outside 1.4..=1.93, which the engine refuses too.
    UnsupportedVersion(f32),
    /// A read ran past the end of the file, at this offset.
    Truncated(usize),
    /// Bytes left over after the last section: the layout was misread.
    TrailingBytes { offset: usize, count: usize },
    /// A face corner indexes past its mesh's positions or UVs.
    IndexOutOfRange { node: usize },
    /// A node's parent chain loops.
    Cycle { node: usize },
    /// An animation channel whose key data cannot fit in the file.
    BadChannel { offset: usize },
}

impl From<Eof> for Error {
    fn from(e: Eof) -> Self {
        Error::Truncated(e.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Storm3D,
    StarTrekDb,
}

// The version gates, as the f32 values the engine compares against.
const V1_4: f32 = 1.4;
const V1_6: f32 = 1.6;
const V1_7: f32 = 1.7;
const V1_8: f32 = 1.8;
const V1_9: f32 = 1.9;
const V1_91: f32 = 1.91;
const V1_92: f32 = 1.92;
const V1_93: f32 = 1.93;

/// An entry in the per-file texture-material table that files before 1.7 carry. A
/// mesh in such a file names its texture material by `ident`.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureMaterialDef {
    pub ident: String,
    pub texture: String,
    /// Passed to the material's constructor; its meaning is not established.
    pub param: u32,
    /// Three bytes: two addressing or wrap modes (when both are 2 the engine clears a
    /// flag of the material) and one more, unexplained.
    pub modes: [u8; 3],
}

/// A lighting material: named by the face groups that use it.
#[derive(Clone, Debug, PartialEq)]
pub struct LightingMaterial {
    pub name: String,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub power: f32,
    /// The lighting model, already corrected for files of version 1.4 and older
    /// (which store it one higher).
    pub model: u8,
    /// Version 1.9+: the group is self-illuminating; any such group sets bit 2 of its
    /// mesh's flags in the engine.
    pub self_illuminating: bool,
}

/// One corner of a triangle: positions and UVs are indexed separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corner {
    pub position: u16,
    pub uv: u16,
}

/// The faces of a mesh that share one lighting material.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceGroup {
    /// A [`LightingMaterial`] name (`team`, `base_fed`, `glow_white`, ...).
    pub material: String,
    pub faces: Vec<[Corner; 3]>,
}

/// A texture of a mesh, with the load flags its file gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshTexture {
    pub name: String,
    /// Version 1.93: or-ed into the texture's load flags. `0x200` marks a height map
    /// the engine turns into a normal map (the Borg bump path). 0 before 1.93.
    pub flags: u32,
}

impl MeshTexture {
    pub const HEIGHT_MAP: u32 = 0x200;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The texture material, which is a blend mode (`opaque`, `default`, `additive`,
    /// `alphathreshold`, `wormhole`, ...) from 1.7 on, and an `ident` in the file's
    /// [`TextureMaterialDef`] table before.
    pub texture_material: String,
    /// Version 1.93: the mesh's flags. Bit 1 (`2`) selects the bump (dot3) path, set
    /// with a height-map texture; `4` is the ordinary value. 0 before 1.93.
    pub flags: u32,
    /// At least one. Before 1.7 the single texture is named by the material ident.
    pub textures: Vec<MeshTexture>,
    /// Version 1.92+: a second texture name, usually empty.
    pub texture2: String,
    /// Version 1.91+: a 16-bit field of unknown use (0 in every stock file read).
    pub field_u16: u16,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub groups: Vec<FaceGroup>,
    /// 1 = back faces culled, 0 = two-sided.
    pub cull: u8,
    /// A trailing list of 16-bit values, empty in stock.
    pub trailing: Vec<u16>,
}

impl Mesh {
    pub const FLAG_BUMP: u32 = 2;

    pub fn face_count(&self) -> usize {
        self.groups.iter().map(|g| g.faces.len()).sum()
    }

    /// The bump-path height map, if the mesh has one.
    pub fn height_map(&self) -> Option<&str> {
        self.textures
            .iter()
            .find(|t| t.flags & MeshTexture::HEIGHT_MAP != 0)
            .map(|t| t.name.as_str())
    }

    /// The diffuse texture: the first one that is not a height map.
    pub fn diffuse_texture(&self) -> Option<&str> {
        self.textures
            .iter()
            .find(|t| t.flags & MeshTexture::HEIGHT_MAP == 0 && !t.name.is_empty())
            .map(|t| t.name.as_str())
    }

    /// The mesh as a GPU wants it: one vertex per distinct (position, UV) pair, and a
    /// triangle list per face group indexing them. Corners keep the file's winding.
    pub fn unindexed(&self) -> UnindexedMesh {
        let mut vertices: Vec<Corner> = Vec::new();
        // A sorted map from (position, uv) to vertex index, without std's HashMap.
        let mut keys: Vec<(u32, u32)> = Vec::new();
        let mut groups = Vec::with_capacity(self.groups.len());
        for g in &self.groups {
            let mut idx = Vec::with_capacity(g.faces.len() * 3);
            for f in &g.faces {
                for c in f {
                    let k = ((c.position as u32) << 16) | c.uv as u32;
                    let i = match keys.binary_search_by_key(&k, |e| e.0) {
                        Ok(at) => keys[at].1,
                        Err(at) => {
                            let i = vertices.len() as u32;
                            vertices.push(*c);
                            keys.insert(at, (k, i));
                            i
                        }
                    };
                    idx.push(i);
                }
            }
            groups.push(idx);
        }
        UnindexedMesh { vertices, groups }
    }

    /// Smooth normals per position, area-weighted, from the faces as wound in the
    /// file. **The files store no normals**; this is a reasonable reconstruction, not
    /// the engine's own (stock windings are not consistent, so some point inward).
    pub fn smooth_normals(&self) -> Vec<[f32; 3]> {
        let mut n = alloc::vec![[0.0f32; 3]; self.positions.len()];
        for g in &self.groups {
            for f in &g.faces {
                let p = |i: usize| self.positions[f[i].position as usize];
                let (a, b, c) = (p(0), p(1), p(2));
                let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let x = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for c in f {
                    let t = &mut n[c.position as usize];
                    t[0] += x[0];
                    t[1] += x[1];
                    t[2] += x[2];
                }
            }
        }
        for t in n.iter_mut() {
            let l = libm::sqrtf(t[0] * t[0] + t[1] * t[1] + t[2] * t[2]);
            if l > 0.0 {
                t[0] /= l;
                t[1] /= l;
                t[2] /= l;
            }
        }
        n
    }

    /// Centre and radius of the positions' bounding box sphere (a loose bound).
    pub fn bounds(&self) -> ([f32; 3], f32) {
        if self.positions.is_empty() {
            return ([0.0; 3], 0.0);
        }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let c = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5, (lo[2] + hi[2]) * 0.5];
        let mut r2 = 0.0f32;
        for p in &self.positions {
            let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
            r2 = r2.max(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
        }
        (c, libm::sqrtf(r2))
    }
}

/// [`Mesh::unindexed`]'s result.
#[derive(Clone, Debug, PartialEq)]
pub struct UnindexedMesh {
    /// Each GPU vertex: which position and which UV it takes.
    pub vertices: Vec<Corner>,
    /// Per face group (same order as [`Mesh::groups`]), three indices per triangle.
    pub groups: Vec<Vec<u32>>,
}

/// A light node (type 2). None ships in stock; the engine reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    /// 2 and 3 select two light classes, anything else a third.
    pub kind: u8,
    pub colour: [f32; 3],
    pub falloff_start: f32,
    pub falloff_range: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    /// Type 0 (and every type the engine does not know): a transform only. Group
    /// nodes, hardpoints, LOD switches and damage points are all null nodes.
    Null,
    /// Type 1.
    Mesh(Mesh),
    /// Type 2.
    Light(Light),
    /// Type 3: the node's NAME is a `@sprite_node` id in the sprite tables.
    Sprite,
    /// Type 12: an `@emitter` id in the sprite tables.
    Emitter { emitter: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The type word as stored.
    pub node_type: u16,
    pub name: String,
    /// The parent's name as stored; empty for a root.
    pub parent_name: String,
    /// The parent, resolved by name (ASCII case-insensitive, first node of that name).
    /// `None` for a root, or when the name matches no node.
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// Local transform in the parent's space.
    pub transform: Matrix34,
    pub kind: NodeKind,
}

/// What a node is for, by its name. The engine finds these by name too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// `hp` + digits: a hardpoint (weapons, repair and build sites, venting). Hardpoints
    /// never animate: the engine refuses a channel on one.
    Hardpoint(u16),
    /// A name starting `geometry` (any case): the engine makes this node the LOD
    /// switch over its direct children named `lod*`.
    Geometry,
    /// A name starting `lod` (any case). Only a direct child of a [`Role::Geometry`]
    /// node is a level; see [`Sod::lod_levels`] for their order.
    Lod,
    /// `damage`, `crew`, `lights`, `hardpoints`, `shield`, `sensors`, `life`, `engines`,
    /// `target`, `build`, `repair`, `dock`: the group nodes ships carry.
    Group,
    Other,
}

impl Node {
    pub fn mesh(&self) -> Option<&Mesh> {
        match &self.kind {
            NodeKind::Mesh(m) => Some(m),
            _ => None,
        }
    }

    pub fn role(&self) -> Role {
        let n = self.name.to_ascii_lowercase();
        if let Some(d) = n.strip_prefix("hp") {
            if !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit()) {
                return Role::Hardpoint(d.parse().unwrap_or(0));
            }
        }
        if n.starts_with("geometry") {
            return Role::Geometry;
        }
        if n.starts_with("lod") {
            return Role::Lod;
        }
        match n.as_str() {
            "damage" | "crew" | "lights" | "hardpoints" | "shield" | "sensors" | "life"
            | "engines" | "target" | "build" | "repair" | "dock" => Role::Group,
            _ => Role::Other,
        }
    }

    /// For a sprite node: the `@sprite_node` id it draws. The engine takes the name up
    /// to its first underscore (C `strtok` on `_`, so leading underscores are skipped):
    /// `redstrobe_12` and `greenstrobe_1_2` draw `redstrobe` and `greenstrobe`, which is
    /// how many nodes share one look under unique names.
    pub fn sprite_node_id(&self) -> &str {
        let n = self.name.trim_start_matches('_');
        match n.find('_') {
            Some(i) => &n[..i],
            None => n,
        }
    }
}

/// Keys of an animation channel. SOD channels are evenly spaced over the period.
#[derive(Clone, Debug, PartialEq)]
pub enum ChannelKeys {
    /// Every type but 5: whole local matrices, played as a step flipbook.
    Matrix(Vec<Matrix34>),
    /// Type 5: the node's uniform scale.
    Scale(Vec<f32>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimChannel {
    /// The node it drives, by name.
    pub node: String,
    pub period: f32,
    pub channel_type: u8,
    /// A byte stored after the type that the engine reads and ignores.
    pub ignored: u8,
    pub keys: ChannelKeys,
}

impl AnimChannel {
    pub fn key_count(&self) -> usize {
        match &self.keys {
            ChannelKeys::Matrix(k) => k.len(),
            ChannelKeys::Scale(k) => k.len(),
        }
    }
}

/// Binds a mesh to an `@animation` of the sprite tables (version 1.6+). Type 4, the
/// only one stock uses, is a texture flipbook on the mesh's UV offset.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimRef {
    pub ref_type: u8,
    pub node: String,
    pub animation: String,
    /// Version 1.8+: time offset in seconds. 0 before.
    pub offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sod {
    pub container: Container,
    pub version: f32,
    /// Before 1.7 only.
    pub texture_materials: Vec<TextureMaterialDef>,
    pub lighting_materials: Vec<LightingMaterial>,
    /// In file order; parents are linked by index.
    pub nodes: Vec<Node>,
    pub channels: Vec<AnimChannel>,
    pub anim_refs: Vec<AnimRef>,
}

impl Sod {
    pub fn parse(data: &[u8]) -> Result<Sod, Error> {
        parse(data)
    }

    /// The nodes with no parent, in file order.
    pub fn roots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].parent.is_none())
    }

    /// The first node of that name, ASCII case-insensitive.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(name))
    }

    pub fn lighting_material(&self, name: &str) -> Option<&LightingMaterial> {
        self.lighting_materials.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// A node's transform composed through all its ancestors into model space.
    /// (The engine's own debris code skips the parents; for the ordinary identity
    /// group nodes the two agree.)
    pub fn model_transform(&self, node: usize) -> Matrix34 {
        let mut m = self.nodes[node].transform;
        let mut at = self.nodes[node].parent;
        let mut guard = 0;
        while let Some(p) = at {
            m = m.then(&self.nodes[p].transform);
            at = self.nodes[p].parent;
            guard += 1;
            if guard > self.nodes.len() {
                break;
            }
        }
        m
    }

    /// Faces of every mesh in a node's subtree.
    pub fn face_count_recursive(&self, node: usize) -> usize {
        let mut total = 0;
        let mut stack = alloc::vec![node];
        let mut guard = 0;
        while let Some(i) = stack.pop() {
            guard += 1;
            if guard > self.nodes.len() {
                break;
            }
            if let Some(m) = self.nodes[i].mesh() {
                total += m.face_count();
            }
            stack.extend_from_slice(&self.nodes[i].children);
        }
        total
    }

    /// The levels of detail under a `geometry*` node: its direct children named `lod*`,
    /// ordered as the engine orders them, by face count ascending (index 0 is the
    /// coarsest; ties keep file order). The engine shows index min(k, len - 1), where k
    /// rises from 0 as the camera nears (`DETAIL_n_LOD_*_DISTANCE` in `ART_CFG.h`).
    /// The digit in the name plays no part.
    pub fn lod_levels(&self, geometry: usize) -> Vec<usize> {
        let mut lv: Vec<(usize, usize)> = self.nodes[geometry]
            .children
            .iter()
            .copied()
            .filter(|&c| self.nodes[c].role() == Role::Lod)
            .map(|c| (self.face_count_recursive(c), c))
            .collect();
        lv.sort_by_key(|e| e.0);
        lv.into_iter().map(|e| e.1).collect()
    }

    /// The LOD level a node is drawn in: `(geometry node, level node)` for the
    /// nearest ancestor (or itself) that is a `lod*` child of a `geometry*` node, or
    /// `None` when it is drawn at every distance.
    pub fn lod_group(&self, node: usize) -> Option<(usize, usize)> {
        let mut at = Some(node);
        let mut guard = 0;
        while let Some(i) = at {
            if self.nodes[i].role() == Role::Lod {
                if let Some(p) = self.nodes[i].parent {
                    if self.nodes[p].role() == Role::Geometry {
                        return Some((p, i));
                    }
                }
            }
            at = self.nodes[i].parent;
            guard += 1;
            if guard > self.nodes.len() {
                break;
            }
        }
        None
    }

    /// For a file before 1.7: the texture file a material ident names, from the
    /// file's table. For later files the mesh names its texture directly.
    pub fn texture_for_ident(&self, ident: &str) -> Option<&str> {
        self.texture_materials
            .iter()
            .find(|t| t.ident.eq_ignore_ascii_case(ident))
            .map(|t| t.texture.as_str())
    }
}

fn parse(data: &[u8]) -> Result<Sod, Error> {
    if data.len() < 14 {
        return Err(Error::Truncated(data.len()));
    }
    let container = match &data[..10] {
        b"Storm3D_SW" => Container::Storm3D,
        b"StarTrekDB" => Container::StarTrekDb,
        _ => return Err(Error::BadMagic),
    };
    let mut r = Reader::new(data, 10);
    let v = r.f32()?;
    if !(v >= V1_4 && v <= V1_93) {
        return Err(Error::UnsupportedVersion(v));
    }

    let mut texture_materials = Vec::new();
    if v < V1_7 {
        let n = r.u16()?;
        for _ in 0..n {
            let ident = r.string()?;
            let texture = r.string()?;
            let param = r.u32()?;
            let modes = [r.u8()?, r.u8()?, r.u8()?];
            texture_materials.push(TextureMaterialDef { ident, texture, param, modes });
        }
    }

    let n = r.u16()?;
    let mut lighting_materials = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let name = r.string()?;
        let ambient = r.f32s::<3>()?;
        let diffuse = r.f32s::<3>()?;
        let specular = r.f32s::<3>()?;
        let power = r.f32()?;
        let mut model = r.u8()?;
        if v <= V1_4 {
            model = model.wrapping_sub(1);
        }
        let self_illuminating = if v >= V1_9 { r.u8()? != 0 } else { false };
        lighting_materials.push(LightingMaterial {
            name,
            ambient,
            diffuse,
            specular,
            power,
            model,
            self_illuminating,
        });
    }

    let n = r.u16()?;
    let mut nodes = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let node_type = r.u16()?;
        let name = r.string()?;
        let parent_name = r.string()?;
        let transform = Matrix34 { m: r.f32s::<12>()? };
        let kind = match node_type {
            1 => NodeKind::Mesh(read_mesh(&mut r, v, nodes.len())?),
            2 => {
                let kind = r.u8()?;
                let colour = r.f32s::<3>()?;
                let falloff_start = r.f32()?;
                let falloff_range = r.f32()?;
                NodeKind::Light(Light { kind, colour, falloff_start, falloff_range })
            }
            3 => NodeKind::Sprite,
            12 => NodeKind::Emitter { emitter: r.string()? },
            _ => NodeKind::Null,
        };
        nodes.push(Node {
            node_type,
            name,
            parent_name,
            parent: None,
            children: Vec::new(),
            transform,
            kind,
        });
    }

    let n = r.u16()?;
    let mut channels = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let at = r.pos();
        let node = r.string()?;
        let nkeys = r.u16()? as usize;
        let period = r.f32()?;
        let channel_type = r.u8()?;
        let ignored = r.u8()?;
        let size = if channel_type == 5 { 4 } else { 48 };
        if nkeys * size > r.remaining() {
            return Err(Error::BadChannel { offset: at });
        }
        let keys = if channel_type == 5 {
            let mut k = Vec::with_capacity(nkeys);
            for _ in 0..nkeys {
                k.push(r.f32()?);
            }
            ChannelKeys::Scale(k)
        } else {
            let mut k = Vec::with_capacity(nkeys);
            for _ in 0..nkeys {
                k.push(Matrix34 { m: r.f32s::<12>()? });
            }
            ChannelKeys::Matrix(k)
        };
        channels.push(AnimChannel { node, period, channel_type, ignored, keys });
    }

    let mut anim_refs = Vec::new();
    if v >= V1_6 {
        let n = r.u16()?;
        for _ in 0..n {
            let ref_type = r.u8()?;
            let node = r.string()?;
            let animation = r.string()?;
            let offset = if v >= V1_8 { r.f32()? } else { 0.0 };
            anim_refs.push(AnimRef { ref_type, node, animation, offset });
        }
    }

    if r.remaining() != 0 {
        return Err(Error::TrailingBytes { offset: r.pos(), count: r.remaining() });
    }

    // Link the tree by name. A parent that comes later in the file is allowed.
    let lowered: Vec<String> = nodes.iter().map(|n| n.name.to_ascii_lowercase()).collect();
    let mut sorted: Vec<(usize, &str)> =
        lowered.iter().enumerate().map(|(i, s)| (i, s.as_str())).collect();
    // Stable sort keeps the first node of each name first.
    sorted.sort_by(|a, b| a.1.cmp(b.1));
    for i in 0..nodes.len() {
        if nodes[i].parent_name.is_empty() {
            continue;
        }
        let want = nodes[i].parent_name.to_ascii_lowercase();
        if let Ok(mut at) = sorted.binary_search_by(|e| e.1.cmp(want.as_str())) {
            while at > 0 && sorted[at - 1].1 == want.as_str() {
                at -= 1;
            }
            let p = sorted[at].0;
            if p != i {
                nodes[i].parent = Some(p);
            }
        }
    }
    for i in 0..nodes.len() {
        // A loop in the parent chain would make every walk endless.
        let mut at = nodes[i].parent;
        let mut steps = 0;
        while let Some(p) = at {
            steps += 1;
            if p == i || steps > nodes.len() {
                return Err(Error::Cycle { node: i });
            }
            at = nodes[p].parent;
        }
    }
    for i in 0..nodes.len() {
        if let Some(p) = nodes[i].parent {
            nodes[p].children.push(i);
        }
    }

    Ok(Sod {
        container,
        version: v,
        texture_materials,
        lighting_materials,
        nodes,
        channels,
        anim_refs,
    })
}

fn read_mesh(r: &mut Reader, v: f32, node: usize) -> Result<Mesh, Error> {
    let texture_material = r.string()?;
    let mut flags = 0;
    let mut textures = Vec::new();
    if v >= V1_93 {
        flags = r.u32()?;
        let n = r.u32()? as usize;
        // Each entry takes at least six bytes; refuse a count the file cannot hold.
        if n > r.remaining() / 6 {
            return Err(Error::Truncated(r.pos()));
        }
        for _ in 0..n {
            let name = r.string()?;
            let tflags = r.u32()?;
            textures.push(MeshTexture { name, flags: tflags });
        }
    } else if v >= V1_7 {
        textures.push(MeshTexture { name: r.string()?, flags: 0 });
    } else {
        textures.push(MeshTexture { name: texture_material.clone(), flags: 0 });
    }
    let texture2 = if v >= V1_92 { r.string()? } else { String::new() };
    let field_u16 = if v >= V1_91 { r.u16()? } else { 0 };

    let mut mesh = Mesh {
        texture_material,
        flags,
        textures,
        texture2,
        field_u16,
        positions: Vec::new(),
        uvs: Vec::new(),
        groups: Vec::new(),
        cull: 1,
        trailing: Vec::new(),
    };
    // The engine stops reading a mesh at a zero vertex or UV count: nothing else of
    // it is stored then.
    let nv = r.u16()? as usize;
    if nv == 0 {
        return Ok(mesh);
    }
    let nuv = r.u16()? as usize;
    if nuv == 0 {
        return Ok(mesh);
    }
    let ng = r.u16()? as usize;
    if nv * 12 + nuv * 8 > r.remaining() {
        return Err(Error::Truncated(r.pos()));
    }
    mesh.positions.reserve(nv);
    for _ in 0..nv {
        mesh.positions.push(r.f32s::<3>()?);
    }
    mesh.uvs.reserve(nuv);
    for _ in 0..nuv {
        mesh.uvs.push(r.f32s::<2>()?);
    }
    for _ in 0..ng {
        let nf = r.u16()? as usize;
        let material = r.string()?;
        if nf * 12 > r.remaining() {
            return Err(Error::Truncated(r.pos()));
        }
        let mut faces = Vec::with_capacity(nf);
        for _ in 0..nf {
            let mut f = [Corner { position: 0, uv: 0 }; 3];
            for c in f.iter_mut() {
                c.position = r.u16()?;
                c.uv = r.u16()?;
                if c.position as usize >= nv || c.uv as usize >= nuv {
                    return Err(Error::IndexOutOfRange { node });
                }
            }
            faces.push(f);
        }
        mesh.groups.push(FaceGroup { material, faces });
    }
    mesh.cull = r.u8()?;
    let n = r.u16()? as usize;
    for _ in 0..n {
        mesh.trailing.push(r.u16()?);
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn s(out: &mut Vec<u8>, t: &str) {
        out.extend_from_slice(&(t.len() as u16).to_le_bytes());
        out.extend_from_slice(t.as_bytes());
    }
    fn f(out: &mut Vec<u8>, v: f32) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    fn h(out: &mut Vec<u8>, v: u16) {
        out.extend_from_slice(&v.to_le_bytes());
    }

    /// A minimal 1.93 file: one material, a root null node and a one-triangle mesh.
    fn tiny() -> Vec<u8> {
        let mut d = b"Storm3D_SW".to_vec();
        f(&mut d, 1.93);
        h(&mut d, 1);
        s(&mut d, "team");
        for _ in 0..10 {
            f(&mut d, 0.5);
        }
        d.push(2);
        d.push(1);
        h(&mut d, 2);
        // root
        h(&mut d, 0);
        s(&mut d, "root");
        s(&mut d, "");
        for v in Matrix34::IDENTITY.m {
            f(&mut d, v);
        }
        // mesh
        h(&mut d, 1);
        s(&mut d, "hull");
        s(&mut d, "ROOT");
        for v in Matrix34::IDENTITY.m {
            f(&mut d, v);
        }
        s(&mut d, "opaque");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        s(&mut d, "fhull");
        d.extend_from_slice(&0u32.to_le_bytes());
        s(&mut d, "");
        h(&mut d, 0);
        h(&mut d, 3);
        h(&mut d, 2);
        h(&mut d, 1);
        for p in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in p {
                f(&mut d, c);
            }
        }
        for t in [[0.0, 0.0], [1.0, 1.0]] {
            for c in t {
                f(&mut d, c);
            }
        }
        h(&mut d, 1);
        s(&mut d, "team");
        for (p, t) in [(0, 0), (1, 1), (2, 0)] {
            h(&mut d, p);
            h(&mut d, t);
        }
        d.push(1);
        h(&mut d, 0);
        h(&mut d, 0); // channels
        h(&mut d, 0); // anim refs
        d
    }

    #[test]
    fn parses_tiny() {
        let sod = Sod::parse(&tiny()).unwrap();
        assert_eq!(sod.nodes.len(), 2);
        assert_eq!(sod.nodes[1].parent, Some(0));
        assert_eq!(sod.nodes[0].children, vec![1]);
        let m = sod.nodes[1].mesh().unwrap();
        assert_eq!(m.face_count(), 1);
        assert_eq!(m.diffuse_texture(), Some("fhull"));
        assert!(sod.lighting_materials[0].self_illuminating);
        let u = m.unindexed();
        assert_eq!(u.vertices.len(), 3);
        assert_eq!(u.groups[0], vec![0, 1, 2]);
        let n = m.smooth_normals();
        assert_eq!(n[0], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn never_panics_on_truncation() {
        let d = tiny();
        for len in 0..d.len() {
            assert!(Sod::parse(&d[..len]).is_err());
        }
        let mut long = d.clone();
        long.push(0);
        assert!(matches!(Sod::parse(&long), Err(Error::TrailingBytes { .. })));
    }

    #[test]
    fn rejects_bad_index_and_version() {
        let mut d = tiny();
        let n = d.len();
        // The last face corner's position index sits 11 bytes from the end.
        d[n - 11] = 9;
        assert!(matches!(Sod::parse(&d), Err(Error::IndexOutOfRange { .. })));
        let mut d = tiny();
        d[10..14].copy_from_slice(&1.2f32.to_le_bytes());
        assert!(matches!(Sod::parse(&d), Err(Error::UnsupportedVersion(_))));
    }
}
