//! What the Vulkan backend reads out of dxc's SPIR-V, and the two changes it makes to
//! a pixel shader before a pipeline takes it. Plain words in, plain words out: no
//! Vulkan here, so the file also builds on the host for its self-test
//! (`gpu/spirv-selftest.sh`).
//!
//! - **Reflection** (`reflect`): each stage's inputs and outputs by location and
//!   semantic (dxc names them `in.var.TEXCOORD3`, `out.var.COLOR0`), the bytes of its
//!   uniform block (set 0: the stage's register file, so the highest member's end), and
//!   the combined image samplers it reads (set 1, binding = the Direct3D 9 stage) with
//!   their dimension. The pipeline's vertex input, the uniform bytes uploaded per draw
//!   and the placeholder bound to an empty slot come from it.
//! - **Interface locations** (`link`): dxc numbers a stage's interface in the order of
//!   its entry point's struct, so a pixel shader whose input struct is not its vertex
//!   shader's output struct would read the wrong varyings. Each input is given the
//!   location of the vertex shader's output of the same semantic.
//! - **The alpha test** (`alpha_test`): Direct3D 9's fixed-function state, which
//!   Vulkan does not have. After the store of the colour output, the fragment is
//!   discarded unless its alpha, as Direct3D 9 compares it (0..255, rounded), passes.
use alloc::string::String;
use alloc::vec::Vec;

const OP_NAME: u32 = 5;
const OP_ENTRY_POINT: u32 = 15;
const OP_TYPE_BOOL: u32 = 20;
const OP_TYPE_INT: u32 = 21;
const OP_TYPE_FLOAT: u32 = 22;
const OP_TYPE_VECTOR: u32 = 23;
const OP_TYPE_MATRIX: u32 = 24;
const OP_TYPE_IMAGE: u32 = 25;
const OP_TYPE_SAMPLED_IMAGE: u32 = 27;
const OP_TYPE_ARRAY: u32 = 28;
const OP_TYPE_STRUCT: u32 = 30;
const OP_TYPE_POINTER: u32 = 32;
const OP_CONSTANT: u32 = 43;
const OP_FUNCTION: u32 = 54;
const OP_VARIABLE: u32 = 59;
const OP_LOAD: u32 = 61;
const OP_STORE: u32 = 62;
const OP_DECORATE: u32 = 71;
const OP_MEMBER_DECORATE: u32 = 72;
const OP_COMPOSITE_EXTRACT: u32 = 81;
const OP_FMUL: u32 = 133;
const OP_LOGICAL_AND: u32 = 167;
const OP_LOGICAL_NOT: u32 = 168;
const OP_FORD_LESS_THAN: u32 = 184;
const OP_FORD_GREATER_THAN_EQUAL: u32 = 190;
const OP_LOOP_MERGE: u32 = 246;
const OP_SELECTION_MERGE: u32 = 247;
const OP_LABEL: u32 = 248;
const OP_BRANCH_CONDITIONAL: u32 = 250;
const OP_KILL: u32 = 252;
const OP_RETURN: u32 = 253;

const DEC_ARRAY_STRIDE: u32 = 6;
const DEC_MATRIX_STRIDE: u32 = 7;
const DEC_BUILTIN: u32 = 11;
const DEC_LOCATION: u32 = 30;
const DEC_BINDING: u32 = 33;
const DEC_SET: u32 = 34;
const DEC_OFFSET: u32 = 35;

const SC_UNIFORM_CONSTANT: u32 = 0;
const SC_INPUT: u32 = 1;
const SC_UNIFORM: u32 = 2;
const SC_OUTPUT: u32 = 3;

/// A semantic: `TEXCOORD` and 3 for `TEXCOORD3` (upper case; no digits: index 0).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sem {
    pub name: String,
    pub index: u32,
}

impl Sem {
    fn parse(s: &str) -> Sem {
        let digits = s.bytes().rev().take_while(|b| b.is_ascii_digit()).count();
        let (n, d) = s.split_at(s.len() - digits);
        let mut name = String::with_capacity(n.len());
        for c in n.chars() {
            name.push(c.to_ascii_uppercase());
        }
        let mut index = 0u32;
        for b in d.bytes() {
            index = index * 10 + (b - b'0') as u32;
        }
        Sem { name, index }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dim {
    D2,
    D3,
    Cube,
    Other,
}

/// A combined image sampler a stage reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TexUse {
    pub binding: u32,
    pub dim: Dim,
    pub ms: bool,
}

#[derive(Default, Debug)]
pub struct Reflect {
    /// (location, semantic) of each input that is not a built-in. Without names the
    /// semantic is empty.
    pub inputs: Vec<(u32, Option<Sem>)>,
    pub outputs: Vec<(u32, Option<Sem>)>,
    /// The bytes of the uniform blocks at set 0 (several alias one binding).
    pub ubo_bytes: u32,
    pub textures: Vec<TexUse>,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Ty {
    Scalar(u32),
    Vector(u32, u32),
    Matrix(u32, u32),
    Array(u32, u32),
    Struct(usize, usize),
    Pointer(u32, u32),
    Image(Dim, bool),
    SampledImage(u32),
    Other,
}

#[allow(dead_code)]
struct Module<'a> {
    w: &'a [u32],
    names: Vec<(u32, String)>,
    /// (id, decoration, first operand)
    decos: Vec<(u32, u32, u32)>,
    /// (struct, member, decoration, operand)
    mdecos: Vec<(u32, u32, u32, u32)>,
    types: Vec<(u32, Ty)>,
    members: Vec<u32>,
    consts: Vec<(u32, u32, u32)>,
    /// (id, pointer type, storage class)
    vars: Vec<(u32, u32, u32)>,
}

fn string_at(w: &[u32]) -> String {
    let mut s = String::new();
    'o: for &x in w {
        for k in 0..4 {
            let b = (x >> (8 * k)) as u8;
            if b == 0 {
                break 'o;
            }
            s.push(b as char);
        }
    }
    s
}

/// The instructions after the header: (offset of each, opcode, word count).
fn instrs(w: &[u32]) -> Vec<(usize, u32, usize)> {
    let mut v = Vec::new();
    let mut i = 5;
    while i < w.len() {
        let n = (w[i] >> 16) as usize;
        if n == 0 || i + n > w.len() {
            break;
        }
        v.push((i, w[i] & 0xffff, n));
        i += n;
    }
    v
}

impl<'a> Module<'a> {
    fn parse(w: &'a [u32]) -> Option<Module<'a>> {
        if w.len() < 5 || w[0] != 0x0723_0203 {
            return None;
        }
        let mut m = Module { w, names: Vec::new(), decos: Vec::new(), mdecos: Vec::new(), types: Vec::new(), members: Vec::new(), consts: Vec::new(), vars: Vec::new() };
        for (i, op, n) in instrs(w) {
            let a = &w[i + 1..i + n];
            match op {
                OP_NAME if a.len() >= 2 => m.names.push((a[0], string_at(&a[1..]))),
                OP_DECORATE if a.len() >= 2 => m.decos.push((a[0], a[1], a.get(2).copied().unwrap_or(0))),
                OP_MEMBER_DECORATE if a.len() >= 3 => m.mdecos.push((a[0], a[1], a[2], a.get(3).copied().unwrap_or(0))),
                OP_TYPE_BOOL => m.types.push((a[0], Ty::Scalar(4))),
                OP_TYPE_INT | OP_TYPE_FLOAT if a.len() >= 2 => m.types.push((a[0], Ty::Scalar(a[1] / 8))),
                OP_TYPE_VECTOR if a.len() >= 3 => m.types.push((a[0], Ty::Vector(a[1], a[2]))),
                OP_TYPE_MATRIX if a.len() >= 3 => m.types.push((a[0], Ty::Matrix(a[1], a[2]))),
                OP_TYPE_ARRAY if a.len() >= 3 => m.types.push((a[0], Ty::Array(a[1], a[2]))),
                OP_TYPE_STRUCT => {
                    let at = m.members.len();
                    m.members.extend_from_slice(&a[1..]);
                    m.types.push((a[0], Ty::Struct(at, a.len() - 1)));
                }
                OP_TYPE_POINTER if a.len() >= 3 => m.types.push((a[0], Ty::Pointer(a[1], a[2]))),
                OP_TYPE_IMAGE if a.len() >= 7 => {
                    let dim = match a[2] {
                        1 => Dim::D2,
                        2 => Dim::D3,
                        3 => Dim::Cube,
                        _ => Dim::Other,
                    };
                    m.types.push((a[0], Ty::Image(dim, a[5] != 0)));
                }
                OP_TYPE_SAMPLED_IMAGE if a.len() >= 2 => m.types.push((a[0], Ty::SampledImage(a[1]))),
                OP_CONSTANT if a.len() >= 3 => m.consts.push((a[0], a[1], a[2])),
                OP_VARIABLE if a.len() >= 3 => m.vars.push((a[1], a[0], a[2])),
                _ => {
                    if op != OP_ENTRY_POINT && op >= OP_TYPE_BOOL && op <= OP_TYPE_POINTER {
                        m.types.push((a.first().copied().unwrap_or(0), Ty::Other));
                    }
                }
            }
        }
        Some(m)
    }

    fn ty(&self, id: u32) -> Ty {
        self.types.iter().find(|t| t.0 == id).map_or(Ty::Other, |t| t.1)
    }

    fn deco(&self, id: u32, d: u32) -> Option<u32> {
        self.decos.iter().find(|x| x.0 == id && x.1 == d).map(|x| x.2)
    }

    fn mdeco(&self, s: u32, member: u32, d: u32) -> Option<u32> {
        self.mdecos.iter().find(|x| x.0 == s && x.1 == member && x.2 == d).map(|x| x.3)
    }

    fn name(&self, id: u32) -> Option<&str> {
        self.names.iter().find(|n| n.0 == id).map(|n| n.1.as_str())
    }

    fn constant(&self, id: u32) -> Option<u32> {
        self.consts.iter().find(|c| c.1 == id).map(|c| c.2)
    }

    /// Bytes a value of `id` takes in a block; `mstride` the matrix stride its member
    /// was decorated with.
    fn size(&self, id: u32, mstride: Option<u32>, depth: u32) -> u32 {
        if depth > 16 {
            return 0;
        }
        match self.ty(id) {
            Ty::Scalar(b) => b,
            Ty::Vector(e, n) => self.size(e, None, depth + 1) * n,
            Ty::Matrix(c, n) => {
                let col = self.size(c, None, depth + 1);
                match mstride {
                    // The last column (or row) need not take a whole stride.
                    Some(s) => s * (n - 1) + col,
                    None => col * n,
                }
            }
            Ty::Array(e, len) => {
                let n = self.constant(len).unwrap_or(1);
                let stride = self.deco(id, DEC_ARRAY_STRIDE).unwrap_or_else(|| self.size(e, mstride, depth + 1).div_ceil(16) * 16);
                if n == 0 { 0 } else { stride * (n - 1) + self.size(e, mstride, depth + 1) }
            }
            Ty::Struct(at, n) => {
                let mut end = 0;
                for k in 0..n {
                    let mt = self.members[at + k];
                    let off = self.mdeco(id, k as u32, DEC_OFFSET).unwrap_or(0);
                    let ms = self.mdeco(id, k as u32, DEC_MATRIX_STRIDE);
                    end = end.max(off + self.size(mt, ms, depth + 1));
                }
                end
            }
            _ => 0,
        }
    }
}

fn sem_of(name: Option<&str>, prefix: &str) -> Option<Sem> {
    let n = name?.strip_prefix(prefix)?;
    Some(Sem::parse(n))
}

/// What a stage reads and writes (the module doc). None: not SPIR-V.
pub fn reflect(w: &[u32]) -> Option<Reflect> {
    let m = Module::parse(w)?;
    let mut r = Reflect::default();
    for &(id, ptr, sc) in &m.vars {
        let Ty::Pointer(_, pointee) = m.ty(ptr) else { continue };
        match sc {
            SC_INPUT | SC_OUTPUT => {
                if m.deco(id, DEC_BUILTIN).is_some() {
                    continue;
                }
                let Some(loc) = m.deco(id, DEC_LOCATION) else { continue };
                if sc == SC_INPUT {
                    r.inputs.push((loc, sem_of(m.name(id), "in.var.")));
                } else {
                    r.outputs.push((loc, sem_of(m.name(id), "out.var.")));
                }
            }
            SC_UNIFORM if m.deco(id, DEC_SET) == Some(0) => {
                r.ubo_bytes = r.ubo_bytes.max(m.size(pointee, None, 0));
            }
            SC_UNIFORM_CONSTANT if m.deco(id, DEC_SET) == Some(1) => {
                let binding = m.deco(id, DEC_BINDING).unwrap_or(0);
                let img = match m.ty(pointee) {
                    Ty::SampledImage(i) => m.ty(i),
                    t => t,
                };
                if let Ty::Image(dim, ms) = img {
                    r.textures.push(TexUse { binding, dim, ms });
                }
            }
            _ => {}
        }
    }
    r.inputs.sort_by_key(|x| x.0);
    r.outputs.sort_by_key(|x| x.0);
    Some(r)
}

/// `ps` with each input at the location of `vs_outputs`' output of the same semantic
/// (the module doc). Inputs without a name, or with no such output, keep theirs.
pub fn link(ps: &[u32], vs_outputs: &[(u32, Option<Sem>)]) -> Vec<u32> {
    let mut out = ps.to_vec();
    let Some(m) = Module::parse(ps) else { return out };
    let mut moves: Vec<(u32, u32)> = Vec::new();
    // An input the vertex shader does not write reads nothing defined either way; it
    // goes past every output so that it collides with none.
    let mut spare = vs_outputs.iter().map(|o| o.0 + 1).max().unwrap_or(0);
    for &(id, _, sc) in &m.vars {
        if sc != SC_INPUT || m.deco(id, DEC_BUILTIN).is_some() || m.deco(id, DEC_LOCATION).is_none() {
            continue;
        }
        let sem = sem_of(m.name(id), "in.var.");
        match vs_outputs.iter().find(|o| sem.is_some() && o.1 == sem) {
            Some(&(loc, _)) => moves.push((id, loc)),
            None if sem.is_some() => {
                moves.push((id, spare));
                spare += 1;
            }
            None => {}
        }
    }
    for (i, op, n) in instrs(ps) {
        if op == OP_DECORATE && n >= 4 && ps[i + 2] == DEC_LOCATION {
            if let Some(&(_, loc)) = moves.iter().find(|x| x.0 == ps[i + 1]) {
                out[i + 3] = loc;
            }
        }
    }
    out
}

fn ins(v: &mut Vec<u32>, op: u32, args: &[u32]) {
    v.push(((args.len() as u32 + 1) << 16) | op);
    v.extend_from_slice(args);
}

/// `ps` discarding every fragment whose colour output's alpha, times 255, is not in
/// `lo <= a < hi` (`invert`: discarding those that are). None when the shader is not
/// shaped as expected (no store of the output at location 0 in a block that returns):
/// the caller then draws without the test, and says so.
pub fn alpha_test(ps: &[u32], lo: f32, hi: f32, invert: bool) -> Option<Vec<u32>> {
    let m = Module::parse(ps)?;
    // The colour output: an Output variable at location 0.
    let out_var = m.vars.iter().find(|v| v.2 == SC_OUTPUT && m.deco(v.0, DEC_LOCATION) == Some(0) && m.deco(v.0, DEC_BUILTIN).is_none())?.0;
    let float = m.types.iter().find(|t| matches!(t.1, Ty::Scalar(4)) && ps_op_of(ps, t.0) == Some(OP_TYPE_FLOAT))?.0;
    let bool_ty = m.types.iter().find(|t| ps_op_of(ps, t.0) == Some(OP_TYPE_BOOL)).map(|t| t.0);
    let list = instrs(ps);
    // Each store to it, in a block that ends with OpReturn and is no loop header.
    let mut stores = Vec::new();
    for (k, &(i, op, _)) in list.iter().enumerate() {
        if op == OP_STORE && ps[i + 1] == out_var {
            let mut j = k + 1;
            let mut ok = false;
            while j < list.len() {
                match list[j].1 {
                    OP_RETURN => {
                        ok = true;
                        break;
                    }
                    OP_LABEL | OP_LOOP_MERGE | OP_SELECTION_MERGE => break,
                    o if (246..=255).contains(&o) => break,
                    _ => j += 1,
                }
            }
            if !ok {
                return None;
            }
            stores.push(i);
        }
    }
    if stores.is_empty() {
        return None;
    }
    let mut bound = ps[3];
    let mut id = || {
        bound += 1;
        bound - 1
    };
    let mut decls = Vec::new();
    let b = match bool_ty {
        Some(b) => b,
        None => {
            let b = id();
            ins(&mut decls, OP_TYPE_BOOL, &[b]);
            b
        }
    };
    let (c255, clo, chi) = (id(), id(), id());
    ins(&mut decls, OP_CONSTANT, &[float, c255, 255f32.to_bits()]);
    ins(&mut decls, OP_CONSTANT, &[float, clo, lo.to_bits()]);
    ins(&mut decls, OP_CONSTANT, &[float, chi, hi.to_bits()]);
    let first_fn = list.iter().find(|x| x.1 == OP_FUNCTION)?.0;
    let mut out = Vec::with_capacity(ps.len() + decls.len() + 32 * stores.len());
    out.extend_from_slice(&ps[..first_fn]);
    out.extend_from_slice(&decls);
    let mut at = first_fn;
    for &s in &stores {
        let n = (ps[s] >> 16) as usize;
        out.extend_from_slice(&ps[at..s + n]);
        at = s + n;
        let value = ps[s + 2];
        let (a, v, ge, lt, keep, kill, merge) = (id(), id(), id(), id(), id(), id(), id());
        ins(&mut out, OP_COMPOSITE_EXTRACT, &[float, a, value, 3]);
        ins(&mut out, OP_FMUL, &[float, v, a, c255]);
        ins(&mut out, OP_FORD_GREATER_THAN_EQUAL, &[b, ge, v, clo]);
        ins(&mut out, OP_FORD_LESS_THAN, &[b, lt, v, chi]);
        ins(&mut out, OP_LOGICAL_AND, &[b, keep, ge, lt]);
        let cond = if invert {
            keep
        } else {
            let d = id();
            ins(&mut out, OP_LOGICAL_NOT, &[b, d, keep]);
            d
        };
        ins(&mut out, OP_SELECTION_MERGE, &[merge, 0]);
        ins(&mut out, OP_BRANCH_CONDITIONAL, &[cond, kill, merge]);
        ins(&mut out, OP_LABEL, &[kill]);
        ins(&mut out, OP_KILL, &[]);
        ins(&mut out, OP_LABEL, &[merge]);
    }
    out.extend_from_slice(&ps[at..]);
    out[3] = bound;
    let _ = OP_LOAD;
    Some(out)
}

/// The opcode of the instruction that defines `id` among the types.
fn ps_op_of(w: &[u32], id: u32) -> Option<u32> {
    for (i, op, n) in instrs(w) {
        if (OP_TYPE_BOOL..=OP_TYPE_POINTER).contains(&op) && n >= 2 && w[i + 1] == id {
            return Some(op);
        }
        if op == OP_FUNCTION {
            break;
        }
    }
    None
}

/// The interval `lo <= a * 255 < hi` an alpha passes `compare` against `reference` in
/// (Direct3D 9 compares the alpha as a whole number of 255ths, rounded), and whether
/// the test keeps the outside of it instead. None: the test keeps every fragment.
pub fn alpha_interval(compare: u32, reference: u8) -> Option<(f32, f32, bool)> {
    let r = reference as f32;
    let (inf, ninf) = (1.0e30f32, -1.0e30f32);
    // compare: Never, Less, Equal, LessEqual, Greater, NotEqual, GreaterEqual, Always.
    Some(match compare {
        0 => (inf, inf, false),
        1 => (ninf, r - 0.5, false),
        2 => (r - 0.5, r + 0.5, false),
        3 => (ninf, r + 0.5, false),
        4 => (r + 0.5, inf, false),
        5 => (r - 0.5, r + 0.5, true),
        6 => (r - 0.5, inf, false),
        _ => return None,
    })
}
