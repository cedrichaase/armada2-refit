//! Fields and values resolved per build at run time (README.md, "Fields and values"):
//! the same hand-made table and image as tests/identity.c's `fields()`, so the Rust and
//! C resolvers are held to one answer. A child build moves a field (0x1c to 0x20) and
//! adds one the root lacks; the hook that uses the added field stands down in the root.
use a2engine::pe::Image;
use a2engine::{Build, Entry, HookDef, HookId, Kind, Note, Runtime, Sym, SymDef, Table};
use core::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize};

const fn field(value: i32) -> Entry {
    Entry { present: true, value, ..Entry::ABSENT }
}

const SYMS: &[SymDef] = &[
    SymDef { id: "fd", symbol: "One", kind: Kind::Field },
    SymDef { id: "v1", symbol: "One", kind: Kind::Value },
    SymDef { id: "fx", symbol: "One", kind: Kind::Field },
];
const HOOKS: &[HookDef] = &[
    HookDef { id: "t.both", run_unknown: false, rows: &[], syms: &[0, 1] },
    HookDef { id: "t.child", run_unknown: false, rows: &[], syms: &[2] },
];

macro_rules! table {
    ($name:ident, $rt:ident) => {
        static $rt: Runtime = {
            static H: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];
            static F: [AtomicU8; 3] = [const { AtomicU8::new(0) }; 3];
            static A: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
            static N: [Note; 2] = [const { Note::new() }; 2];
            Runtime::new(&H, &F, &A, &N)
        };
        static $name: Table = Table {
            game: "test",
            plugin: "t",
            syms: SYMS,
            hooks: HOOKS,
            builds: &[
                Build {
                    name: "b1",
                    key: "test/b1",
                    timestamp: 0x3c4c76bd,
                    image_size: 0x3fa000,
                    sections: 0x1111_1111,
                    image_base: 0x400000,
                    n_resolved: 2,
                    content: "",
                    entries: &[field(0x1c), field(-40), Entry::ABSENT],
                    blocked: &[],
                },
                Build {
                    name: "b2",
                    key: "test/b2",
                    timestamp: 0x3c4c76bd,
                    image_size: 0x3fa000,
                    sections: 0xa6b2_c1cf,
                    image_base: 0x400000,
                    n_resolved: 3,
                    content: "",
                    entries: &[field(0x20), field(-40), field(0x30)],
                    blocked: &[],
                },
            ],
            n_scan: 0,
            rt: &$rt,
        };
    };
}

table!(CHILD, RT_CHILD);
table!(UNKNOWN, RT_UNKNOWN);

/// tests/identity.c's synthetic image (the fingerprint tests/pe.rs pins), headers only.
struct Img(Vec<u8>);

impl Image for Img {
    fn read(&self, at: usize, out: &mut [u8]) -> bool {
        match self.0.get(at..at + out.len()) {
            Some(b) => {
                out.copy_from_slice(b);
                true
            }
            None => false,
        }
    }
    fn base(&self) -> usize {
        0x400000
    }
}

fn image(fingerprint_ok: bool) -> &'static Img {
    let secs: [(&[u8], u32, u32, u32, u32); 4] = [
        (b".text", 0x1000, 0x2ad000, 0x2ac000, 0x6000_0020),
        (b".rdata", 0x2ae000, 0x40000, 0x3f000, 0x4000_0040),
        (b".data", 0x2ee000, 0x10000, 0xc000, 0xc000_0040),
        (b".rsrc", 0x3f8000, 0x2000, 0x2000, 0x4000_0040),
    ];
    let nt = 0x80usize;
    let mut v = vec![0u8; nt + 24 + 224 + secs.len() * 40];
    v[0..2].copy_from_slice(b"MZ");
    v[0x3c..0x40].copy_from_slice(&(nt as u32).to_le_bytes());
    v[nt..nt + 4].copy_from_slice(b"PE\0\0");
    v[nt + 6] = secs.len() as u8;
    v[nt + 8..nt + 12].copy_from_slice(&0x3c4c76bdu32.to_le_bytes());
    v[nt + 20] = 224;
    let opt = nt + 24;
    v[opt..opt + 2].copy_from_slice(&0x10bu16.to_le_bytes());
    v[opt + 56..opt + 60].copy_from_slice(&0x3fa000u32.to_le_bytes());
    for (i, (name, va, vs, raw, ch)) in secs.iter().enumerate() {
        let s = opt + 224 + i * 40;
        v[s..s + name.len()].copy_from_slice(name);
        v[s + 8..s + 12].copy_from_slice(&vs.to_le_bytes());
        v[s + 12..s + 16].copy_from_slice(&va.to_le_bytes());
        v[s + 16..s + 20].copy_from_slice(&raw.to_le_bytes());
        v[s + 36..s + 40].copy_from_slice(&ch.to_le_bytes());
    }
    if !fingerprint_ok {
        v[opt + 224] ^= 1;
    }
    Box::leak(Box::new(Img(v)))
}

fn quiet(_: &str) {}

#[test]
fn a_child_build_moves_a_field() {
    let e = CHILD.engine();
    // Before init: the root build's.
    assert_eq!(e.field(Sym(0)), 0x1c);
    assert_eq!(e.value(Sym(1)), -40);
    let e = a2engine::init_with(&CHILD, quiet, image(true), None);
    assert_eq!(e.build(), Some("test/b2"));
    assert_eq!(e.field(Sym(0)), 0x20);
    assert_eq!(e.value(Sym(1)), -40);
    assert_eq!(e.field(Sym(2)), 0x30);
    assert!(e.ready(HookId(0)).is_ok() && e.ready(HookId(1)).is_ok());
    assert_eq!(e.addr(Sym(0)), 0x20);
}

#[test]
fn an_unknown_build_reads_the_root() {
    let e = a2engine::init_with(&UNKNOWN, quiet, image(false), None);
    assert_eq!(e.build(), None);
    assert_eq!(e.field(Sym(0)), 0x1c);
    assert_eq!(e.field(Sym(2)), 0, "a field the root lacks");
    assert!(e.ready(HookId(1)).is_err());
}
