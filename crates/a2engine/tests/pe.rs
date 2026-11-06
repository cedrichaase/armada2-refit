//! The identity reader against synthetic PE images built here (no game file is read or
//! kept): the numbers, the three-part sensitivity, and the refusals.
use a2engine::pe::{Identity, identify_bytes as identify};

/// (name, virtual address, virtual size, raw size, characteristics)
type Sec = (&'static [u8], u32, u32, u32, u32);

/// A PE32 image: DOS header, `PE\0\0`, file header, a 224-byte optional header, the
/// section table.
fn image(timestamp: u32, image_size: u32, sections: &[Sec], magic: u16) -> Vec<u8> {
    let nt = 0x80usize;
    let mut v = vec![0u8; nt + 24 + 224 + sections.len() * 40];
    v[0..2].copy_from_slice(b"MZ");
    v[0x3c..0x40].copy_from_slice(&(nt as u32).to_le_bytes());
    v[nt..nt + 4].copy_from_slice(b"PE\0\0");
    v[nt + 6..nt + 8].copy_from_slice(&(sections.len() as u16).to_le_bytes());
    v[nt + 8..nt + 12].copy_from_slice(&timestamp.to_le_bytes());
    v[nt + 20..nt + 22].copy_from_slice(&224u16.to_le_bytes());
    let opt = nt + 24;
    v[opt..opt + 2].copy_from_slice(&magic.to_le_bytes());
    v[opt + 56..opt + 60].copy_from_slice(&image_size.to_le_bytes());
    for (i, (name, va, vs, raw, ch)) in sections.iter().enumerate() {
        let s = opt + 224 + i * 40;
        v[s..s + name.len()].copy_from_slice(name);
        v[s + 8..s + 12].copy_from_slice(&vs.to_le_bytes());
        v[s + 12..s + 16].copy_from_slice(&va.to_le_bytes());
        v[s + 16..s + 20].copy_from_slice(&raw.to_le_bytes());
        v[s + 36..s + 40].copy_from_slice(&ch.to_le_bytes());
    }
    v
}

const CODE: u32 = 0x6000_0020;
const DATA: u32 = 0xc000_0040;

fn stock_like() -> Vec<Sec> {
    vec![
        (b".text", 0x1000, 0x2ad000, 0x2ac000, CODE),
        (b".rdata", 0x2ae000, 0x40000, 0x3f000, 0x4000_0040),
        (b".data", 0x2ee000, 0x10000, 0xc000, DATA),
        (b".rsrc", 0x3f8000, 0x2000, 0x2000, 0x4000_0040),
    ]
}

/// The same hash, spelled out byte by byte, independent of the reader.
fn fnv_of(sections: &[Sec]) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for (name, va, vs, raw, _) in sections {
        let mut bytes = [0u8; 8];
        bytes[..name.len()].copy_from_slice(name);
        let mut all = bytes.to_vec();
        all.extend_from_slice(&va.to_le_bytes());
        all.extend_from_slice(&vs.to_le_bytes());
        all.extend_from_slice(&raw.to_le_bytes());
        for b in all {
            h = (h ^ b as u32).wrapping_mul(0x0100_0193);
        }
    }
    h
}

#[test]
fn reads_the_three_parts() {
    let s = stock_like();
    let img = image(0x3c4c76bd, 0x3fa000, &s, 0x10b);
    let id = identify(&img).expect("a PE32");
    assert_eq!(id.timestamp, 0x3c4c76bd);
    assert_eq!(id.image_size, 0x3fa000);
    assert_eq!(id.sections, fnv_of(&s));
    assert_eq!(id.base, 0);
    assert_eq!(id.code, (0x1000, 0x1000 + 0x2ad000));
}

/// A pinned value, so a change to the hash's spelling is a deliberate edit here and in
/// csdk/a2engine.h's host test, which pins the same number.
#[test]
fn golden_fingerprint() {
    let img = image(0x3c4c76bd, 0x3fa000, &stock_like(), 0x10b);
    assert_eq!(identify(&img).unwrap().sections, GOLDEN);
}
const GOLDEN: u32 = 0xa6b2c1cf;

#[test]
fn each_part_tells_builds_apart() {
    let base = identify(&image(0x3c4c76bd, 0x3fa000, &stock_like(), 0x10b)).unwrap();
    let other_time = identify(&image(0x3c4c76be, 0x3fa000, &stock_like(), 0x10b)).unwrap();
    assert_ne!(base.timestamp, other_time.timestamp);
    assert_eq!(base.sections, other_time.sections);
    let other_size = identify(&image(0x3c4c76bd, 0x405000, &stock_like(), 0x10b)).unwrap();
    assert_ne!(base.image_size, other_size.image_size);
    // A Roots-like exe: same stamp, bigger image, one more section.
    let mut roots = stock_like();
    roots.push((b".test", 0x3fa000, 0x5000, 0x5000, DATA));
    let r = identify(&image(0x3c4c76bd, 0x405000, &roots, 0x10b)).unwrap();
    assert_eq!(r.timestamp, base.timestamp);
    assert_ne!(r.image_size, base.image_size);
    assert_ne!(r.sections, base.sections);
    // One section header field alone moves the fingerprint.
    let mut moved = stock_like();
    moved[2].3 += 0x1000;
    let m = identify(&image(0x3c4c76bd, 0x3fa000, &moved, 0x10b)).unwrap();
    assert_ne!(m.sections, base.sections);
    // Names count, and a name is its 8 bytes padded with zero.
    let mut renamed = stock_like();
    renamed[3].0 = b".rsr";
    let n = identify(&image(0x3c4c76bd, 0x3fa000, &renamed, 0x10b)).unwrap();
    assert_ne!(n.sections, base.sections);
}

#[test]
fn pe32_plus_reads_the_same_fields() {
    let s = stock_like();
    let id = identify(&image(1, 0x5000, &s, 0x20b)).unwrap();
    assert_eq!((id.timestamp, id.image_size, id.sections), (1, 0x5000, fnv_of(&s)));
}

#[test]
fn refuses_what_is_not_a_pe() {
    let good = image(1, 0x5000, &stock_like(), 0x10b);
    assert!(identify(&[0u8; 0]).is_none());
    assert!(identify(&good[..0x3f]).is_none(), "truncated DOS header");
    assert!(identify(&good[..0x90]).is_none(), "truncated NT headers");
    let last = good.len() - 1;
    assert!(identify(&good[..last]).is_none(), "truncated section table");
    let mut bad = good.clone();
    bad[0] = b'Z';
    assert!(identify(&bad).is_none(), "no MZ");
    let mut bad = good.clone();
    bad[0x80] = b'X';
    assert!(identify(&bad).is_none(), "no PE signature");
    let mut bad = good.clone();
    bad[0x3c..0x40].copy_from_slice(&0xffff_fff0u32.to_le_bytes());
    assert!(identify(&bad).is_none(), "e_lfanew out of reach");
    assert!(identify(&image(1, 0x5000, &[], 0x10b)).is_none(), "no sections");
    assert!(identify(&image(1, 0x5000, &stock_like(), 0x107)).is_none(), "ROM image");
}

#[test]
fn the_log_line() {
    let id = identify(&image(0x3c4c76bd, 0x3fa000, &stock_like(), 0x10b)).unwrap();
    assert_eq!(
        format!("engine: {} {id}", "Armada2.exe"),
        "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf"
    );
}

#[test]
fn identity_lines_of_any_executable() {
    use a2engine::is_identity_line;
    assert!(is_identity_line("engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf"));
    assert!(is_identity_line("engine: ArmadaL.exe timestamp 0x3c4c76bd, image size 0x3fb000, sections 0x00000001"));
    assert!(is_identity_line("engine: Armada2.exe identity unreadable"));
    assert!(!is_identity_line("engine: build armada2/gog-1.1"));
    assert!(!is_identity_line("engine: unknown build"));
    assert!(!is_identity_line("engine: symbols: 3/3 resolved; 0 hooks blocked"));
    assert!(!is_identity_line("engine: TEST: A2EngineTest.ini forces a mismatch for x"));
    assert!(!is_identity_line("engine: ui.hud stands down: X is not stock (bytes differ)"));
}

#[test]
fn no_executable_section_gives_an_empty_code_range() {
    let id: Identity = identify(&image(1, 0x5000, &[(b".data", 0x1000, 0x100, 0x100, DATA)], 0x10b)).unwrap();
    assert_eq!(id.code, (0, 0));
}
