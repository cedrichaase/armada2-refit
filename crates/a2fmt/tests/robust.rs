//! Hostile-input regressions: none of these needs a game install.

use a2fmt::odf::Odf;
use a2fmt::spr::SpriteTable;
use a2fmt::tga;

fn hdr(t: u8, w: u16, h: u16, bpp: u8) -> Vec<u8> {
    let mut d = vec![0u8; 18];
    d[2] = t;
    d[12..14].copy_from_slice(&w.to_le_bytes());
    d[14..16].copy_from_slice(&h.to_le_bytes());
    d[16] = bpp;
    d
}

/// 32768x32768 at 32 bpp: `count * 4` is 2^32, which wraps to zero on a 32-bit build.
#[test]
fn tga_huge_header_without_data_is_an_error() {
    for t in [2u8, 10] {
        assert!(tga::decode(&hdr(t, 32768, 32768, 32)).is_err());
    }
    assert!(tga::decode(&hdr(2, 65535, 65535, 24)).is_err());
    assert!(tga::decode(&hdr(3, 65535, 65535, 8)).is_err());
}

#[test]
fn spr_huge_key_count_does_not_blow_up() {
    let t = SpriteTable::parse(b"@animation x\noffset 2000000000 1 step\n@auto=row\n@keyframes\n");
    let a = t.animation("x").unwrap();
    assert!(a.auto_offsets().len() <= 4096);
}

#[test]
fn spr_self_include_is_cheap() {
    let mut calls = 0u32;
    let _ = SpriteTable::load("a.spr", |_| {
        calls += 1;
        Some(b"@include a.spr\n@include a.spr\n@include a.spr\n@include a.spr\n".to_vec())
    });
    assert!(calls < 1000, "fetched {calls} times");
}

#[test]
fn spr_doubling_chain_is_cheap() {
    let mut calls = 0u32;
    let _ = SpriteTable::load("f0.spr", |n| {
        calls += 1;
        let k: u32 = n[1..n.len() - 4].parse().ok()?;
        Some(format!("@include f{0}.spr\n@include f{0}.spr\n", k + 1).into_bytes())
    });
    assert!(calls < 1000, "fetched {calls} times");
}

#[test]
fn odf_doubling_chain_is_cheap() {
    let mut calls = 0u32;
    let _ = Odf::load("f0.odf", |n| {
        calls += 1;
        let k: u32 = n[1..n.len() - 4].parse().ok()?;
        Some(format!("#include f{0}.odf\n#include f{0}.odf\n", k + 1).into_bytes())
    });
    assert!(calls < 5000, "fetched {calls} times");
}
