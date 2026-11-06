//! `.fpq` archives and LZO1X: synthetic fixtures built here in code (nothing from a
//! game), hostile input, the archive as a `Vfs`, and an opt-in test on a real Roots
//! install.

use a2fmt::fpq::{self, Archive, Hash};
use a2fmt::lzo;
use a2fmt::odf::Odf;
use a2fmt::vfs::{Entry, Fpq, Kind, Layered, Status, Vfs};

// ---- LZO1X ----

/// Literals-only stream of at most 238 bytes, with the end marker.
fn lits(data: &[u8]) -> Vec<u8> {
    assert!(data.len() > 0 && data.len() <= 238);
    let mut v = vec![17 + data.len() as u8];
    v.extend_from_slice(data);
    v.extend_from_slice(&[17, 0, 0]);
    v
}

/// The bytes of a length extension: zeros for each 255, then the rest (non-zero).
fn ext(mut n: usize) -> Vec<u8> {
    assert!(n < 1 << 20);
    let mut v = vec![];
    while n > 255 {
        v.push(0);
        n -= 255;
    }
    v.push(n as u8);
    v
}

/// A long M3 match (`dist` back, `len` bytes, no trailing literals).
fn m3(dist: usize, len: usize) -> Vec<u8> {
    assert!(len >= 33 && dist >= 1);
    let mut v = vec![32];
    v.extend(ext(len - 2 - 31));
    let w = ((dist - 1) << 2) as u16;
    v.extend_from_slice(&w.to_le_bytes());
    v
}

fn naive(mut out: Vec<u8>, dist: usize, len: usize) -> Vec<u8> {
    for _ in 0..len {
        let b = out[out.len() - dist];
        out.push(b);
    }
    out
}

#[test]
fn lzo_literals_and_each_match_kind() {
    assert_eq!(lzo::decompress(&lits(b"hello"), 5).unwrap(), b"hello");
    // Without the end marker the stream ends with its input.
    assert_eq!(lzo::decompress(&lits(b"hello")[..6], 5).unwrap(), b"hello");

    // M2: 3 literals, then a 6-byte match 3 back.
    let s = [20, b'a', b'b', b'c', 168, 0, 17, 0, 0];
    assert_eq!(lzo::decompress(&s, 9).unwrap(), b"abcabcabc");

    // M1 after 1..3 literals: 2 bytes, 2 back.
    let s = [20, b'a', b'b', b'c', 4, 0, 17, 0, 0];
    assert_eq!(lzo::decompress(&s, 5).unwrap(), b"abcbc");

    // M3 with an extended length, trailing literals taken from its low bits, then a
    // run of literals (state 0), then M1 beyond 2048 (state 4).
    let mut s = vec![18, b'a'];
    s.extend(m3(1, 3000));
    s.extend_from_slice(&[1, b'w', b'x', b'y', b'z', 0, 0]);
    s.extend_from_slice(&[17, 0, 0]);
    let mut want = vec![b'a'; 3001];
    want.extend_from_slice(b"wxyz");
    want.extend_from_slice(b"aaa");
    assert_eq!(lzo::decompress(&s, want.len()).unwrap(), want);

    // M3 with trailing literals in the distance word.
    let mut s = vec![19, b'x', b'y'];
    s.extend_from_slice(&[33, (1 << 2 | 2) as u8, 0, b'p', b'q', 17, 0, 0]);
    assert_eq!(lzo::decompress(&s, 2 + 3 + 2).unwrap(), b"xyxyxpq");

    // M4 (far), with an extended length.
    let mut s = vec![19, b'x', b'y'];
    s.extend(m3(2, 20000));
    s.extend_from_slice(&[16, 3 + 0, 0, 0]); // placeholder, replaced below
    s.truncate(s.len() - 4);
    s.push(16);
    s.extend(ext(1)); // length 7 + 1 + 2 = 10
    s.extend_from_slice(&((4u16 << 2).to_le_bytes())); // distance 16384 + 4
    s.extend_from_slice(&[17, 0, 0]);
    let mut want = naive(b"xy".to_vec(), 2, 20000);
    want = naive(want, 16388, 10);
    assert_eq!(lzo::decompress(&s, want.len()).unwrap(), want);

    // A zero run before a long literal run (state 0, first byte 0).
    let mut s = vec![18, b'a'];
    s.extend(m3(1, 40));
    s.push(0);
    s.extend(ext(0 + 5)); // 15 + 5 + 3 literals
    let run: Vec<u8> = (0..23).collect();
    s.extend_from_slice(&run);
    let mut want = vec![b'a'; 41];
    want.extend_from_slice(&run);
    assert_eq!(lzo::decompress(&s, want.len()).unwrap(), want);
}

#[test]
fn lzo_refuses_bad_streams() {
    // Size wrong either way.
    assert!(lzo::decompress(&lits(b"hello"), 4).is_err());
    assert!(lzo::decompress(&lits(b"hello"), 6).is_err());
    // A match before the start, and one of distance 0 in effect.
    assert_eq!(lzo::decompress(&[18, b'a', 168, 0], 7), Err(lzo::Error::Distance));
    // Literals past the input; a length extension past the input.
    assert_eq!(lzo::decompress(&[30, b'a'], 13), Err(lzo::Error::Input));
    assert_eq!(lzo::decompress(&[18, b'a', 32, 0, 0], 100), Err(lzo::Error::Input));
    // A match that would pass the output size.
    assert_eq!(lzo::decompress(&[18, b'a', 160, 0], 3), Err(lzo::Error::Overrun));
    // A size no stream of this length could make.
    assert!(lzo::decompress(&[17, 0, 0], 1 << 30).is_err());
    // Empty input makes an empty output only.
    assert_eq!(lzo::decompress(&[], 0).unwrap(), b"");
    assert!(lzo::decompress(&[], 1).is_err());
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn lzo_never_panics_on_truncation_or_flips() {
    let mut s = vec![19, b'x', b'y'];
    s.extend(m3(2, 700));
    s.extend_from_slice(&[168, 0, 1, 1, 2, 3, 4, 0, 0, 17, 0, 0]);
    for n in 0..s.len() {
        let _ = lzo::decompress(&s[..n], 720);
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20000 {
        let mut t = s.clone();
        for _ in 0..1 + rng.next() % 3 {
            let i = (rng.next() as usize) % t.len();
            t[i] = rng.next() as u8;
        }
        let size = (rng.next() % 2000) as usize;
        let _ = lzo::decompress(&t, size);
    }
    for _ in 0..5000 {
        let t: Vec<u8> = (0..rng.next() % 64).map(|_| rng.next() as u8).collect();
        let _ = lzo::decompress(&t, (rng.next() % 5000) as usize);
    }
}

// ---- the archive ----

struct F {
    folder: Option<u32>,
    name: &'static str,
    unpacked: Vec<u8>,
    /// None: stored; Some: the compressed stream.
    packed: Option<Vec<u8>>,
}

fn put(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Builds an archive with `hash_size` slots, folders `odf` (0) and `odf/system` (1).
fn build(files: &[F], hash_size: u32) -> Vec<u8> {
    let mut names = Vec::new();
    let mut folders = Vec::new();
    for (parent, name) in [(u32::MAX, "odf"), (0, "system")] {
        put(&mut folders, parent);
        put(&mut folders, names.len() as u32);
        put(&mut folders, name.len() as u32);
        names.extend_from_slice(name.as_bytes());
    }
    let mut table = Vec::new();
    let mut blob = Vec::new();
    for f in files {
        let packed = f.packed.clone().unwrap_or_else(|| f.unpacked.clone());
        put(&mut table, blob.len() as u32);
        put(&mut table, f.unpacked.len() as u32);
        put(&mut table, packed.len() as u32);
        put(&mut table, names.len() as u32);
        put(&mut table, f.name.len() as u32);
        table.push(f.packed.is_some() as u8);
        put(&mut table, f.folder.unwrap_or(u32::MAX));
        names.extend_from_slice(f.name.as_bytes());
        blob.extend_from_slice(&packed);
    }
    let mut slots = vec![(0u32, 0u32, 0u32); hash_size as usize];
    for (i, f) in files.iter().enumerate() {
        let mut j = fpq::hash(f.name, Hash::Slot) as usize & (hash_size as usize - 1);
        while slots[j] != (0, 0, 0) {
            j = (j + 1) & (hash_size as usize - 1);
        }
        slots[j] = (fpq::hash(f.name, Hash::CheckA), fpq::hash(f.name, Hash::CheckB), i as u32);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"FPQ\0");
    for x in [2, 2, files.len() as u32, hash_size, blob.len() as u32, names.len() as u32] {
        put(&mut out, x);
    }
    for (a, b, c) in slots {
        put(&mut out, a);
        put(&mut out, b);
        put(&mut out, c);
    }
    out.extend(folders);
    out.extend(table);
    out.extend(names);
    out.extend(blob);
    out
}

fn sample() -> Vec<u8> {
    let long: Vec<u8> = b"ab".iter().cycle().take(102).cloned().collect();
    let mut s = vec![19, b'a', b'b'];
    s.extend(m3(2, 100));
    s.extend_from_slice(&[17, 0, 0]);
    build(
        &[
            F { folder: Some(1), name: "Ship.odf", unpacked: b"class = \"ship\"\n".to_vec(), packed: None },
            F { folder: Some(1), name: "weapon.ODF", unpacked: b"#include \"ship.odf\"\nx = 1\n".to_vec(), packed: Some(lits(b"#include \"ship.odf\"\nx = 1\n")) },
            F { folder: Some(1), name: "long.odf", unpacked: long, packed: Some(s) },
            F { folder: Some(0), name: "top.odf", unpacked: b"t".to_vec(), packed: None },
            F { folder: None, name: "bare.odf", unpacked: b"b".to_vec(), packed: None },
        ],
        16,
    )
}

#[test]
fn archive_lists_finds_and_reads() {
    let a = Archive::parse(sample()).unwrap();
    assert_eq!(a.entries().len(), 5);
    assert_eq!(a.folders().len(), 2);
    assert_eq!(a.path(0).as_deref(), Some("odf/system/Ship.odf"));
    assert_eq!(a.path(3).as_deref(), Some("odf/top.odf"));
    assert_eq!(a.path(4).as_deref(), Some("bare.odf"));
    // Case, slashes and folders.
    for p in ["Ship.odf", "SHIP.ODF", "system/ship.odf", "odf\\system\\Ship.odf", "/odf/./system/ship.odf"] {
        assert_eq!(a.find(p), Some(0), "{p}");
    }
    assert_eq!(a.find("other/ship.odf"), None);
    assert_eq!(a.find("odf/ship.odf"), None);
    assert_eq!(a.find("nothing.odf"), None);
    assert_eq!(a.find(""), None);
    assert_eq!(a.find("../ship.odf"), None);
    assert_eq!(a.open("ship.odf").unwrap(), b"class = \"ship\"\n");
    assert_eq!(a.open("WEAPON.odf").unwrap(), b"#include \"ship.odf\"\nx = 1\n");
    let long = a.open("long.odf").unwrap();
    assert_eq!(long.len(), 102);
    assert!(long.chunks(2).all(|c| c == b"ab"));
    assert_eq!(a.open("top.odf").unwrap(), b"t");
    assert_eq!(a.open("bare.odf").unwrap(), b"b");
    // The ODF reader through the archive's own includes.
    let o = Odf::load("weapon.odf", |n| a.open(n)).unwrap();
    assert_eq!(o.get_str("class"), Some("ship"));
    assert_eq!(o.get_i32("x"), Some(1));
}

#[test]
fn probing_survives_a_full_table() {
    // More files than a quarter of the slots, in a table with no spare slot at all.
    let files: Vec<F> = (0..8)
        .map(|i| F {
            folder: Some(1),
            name: ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"][i],
            unpacked: vec![i as u8],
            packed: None,
        })
        .collect();
    let a = Archive::parse(build(&files, 8)).unwrap();
    for i in 0..8 {
        assert_eq!(a.open(&format!("A{i}")).unwrap(), [i as u8]);
        assert_eq!(a.open(&format!("a{i}")).unwrap(), [i as u8]);
    }
    assert_eq!(a.find("a9"), None);
}

#[test]
fn archive_refuses_damage() {
    let good = sample();
    assert!(Archive::parse(good.clone()).is_ok());
    assert!(Archive::parse(vec![]).is_err());
    let mut t = good.clone();
    t[0] = b'X';
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Header));
    let mut t = good.clone();
    t[4] = 1;
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Header));
    // Any truncation or extension breaks the exact length.
    assert_eq!(Archive::parse(good[..good.len() - 1].to_vec()).err(), Some(fpq::Error::Size));
    let mut t = good.clone();
    t.push(0);
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Size));
    // A hash size that is not a power of two; counts that overflow every sum.
    let mut t = good.clone();
    t[16] = 15;
    assert!(Archive::parse(t).is_err());
    let mut t = good.clone();
    t[8..24].fill(0xFF);
    assert!(Archive::parse(t).is_err());
    // Hostile tables: a file past the data blob; a name past the names blob; a folder
    // that is its own parent; a folder index that is not there; a slot to no file.
    let hash_size = 16;
    let folder_at = 28 + hash_size * 12;
    let file_at = folder_at + 2 * 12;
    let mut t = good.clone();
    t[file_at..file_at + 4].copy_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Table));
    let mut t = good.clone();
    t[file_at + 12..file_at + 16].copy_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Table));
    let mut t = good.clone();
    t[folder_at + 12..folder_at + 16].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Table));
    let mut t = good.clone();
    t[file_at + 21..file_at + 25].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Table));
    let mut t = good.clone();
    let slot = (0..hash_size).find(|i| t[28 + i * 12 + 8] == 1).unwrap_or(0);
    t[28 + slot * 12 + 8..28 + slot * 12 + 12].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(Archive::parse(t).err(), Some(fpq::Error::Table));
    // An unknown method and a stored file whose sizes disagree are errors on read, and
    // a compressed file that does not decode is one too.
    let mut t = good.clone();
    t[file_at + 20] = 7;
    let a = Archive::parse(t).unwrap();
    assert_eq!(a.read(0), Err(fpq::Error::Method(7)));
    let mut t = good.clone();
    t[file_at + 4..file_at + 8].copy_from_slice(&3u32.to_le_bytes());
    assert!(Archive::parse(t).unwrap().read(0).is_err());
    assert!(Archive::parse(good.clone()).unwrap().read(99).is_err());
}

#[test]
fn archive_never_panics_on_flips() {
    let good = sample();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for _ in 0..20000 {
        let mut t = good.clone();
        for _ in 0..1 + rng.next() % 3 {
            let i = (rng.next() as usize) % t.len();
            t[i] = rng.next() as u8;
        }
        if let Ok(a) = Archive::parse(t) {
            for i in 0..a.entries().len() {
                let _ = a.read(i);
                let _ = a.path(i);
            }
            let _ = a.find("ship.odf");
            let _ = a.find("long.odf");
        }
    }
}

// ---- as a Vfs ----

fn names(v: &dyn Vfs, dir: &str) -> Vec<(String, bool)> {
    let mut l: Vec<(String, bool)> = v.list(dir).into_iter().map(|Entry { name, dir }| (name, dir)).collect();
    l.sort();
    l
}

#[test]
fn fpq_as_a_vfs_mounted_beside_loose_files() {
    let f = Fpq::new("ODF.fpq", sample()).unwrap();
    assert_eq!(f.kind(), Kind::Fpq);
    assert_eq!(f.status(), Status::Ready);
    assert_eq!(f.label(), "fpq ODF.fpq");
    // The top folder is the mount's name.
    assert_eq!(f.open("system/ship.odf").unwrap(), b"class = \"ship\"\n");
    assert_eq!(f.open("ODF/System/Ship.odf").unwrap(), b"class = \"ship\"\n");
    assert!(f.exists("ship.odf"));
    assert!(!f.exists("system"));
    assert_eq!(
        names(&f, ""),
        [("bare.odf".into(), false), ("system".into(), true), ("top.odf".into(), false)]
    );
    assert_eq!(names(&f, "odf"), names(&f, ""));
    assert_eq!(
        names(&f, "system"),
        [("Ship.odf".into(), false), ("long.odf".into(), false), ("weapon.ODF".into(), false)]
    );
    assert!(names(&f, "nowhere").is_empty());
    assert!(names(&f, "../system").is_empty());

    // A loose file replaces a packed one; the rest still comes from the archive.
    struct One;
    impl Vfs for One {
        fn open(&self, p: &str) -> Option<Vec<u8>> {
            p.eq_ignore_ascii_case("system/ship.odf").then(|| b"loose".to_vec())
        }
        fn exists(&self, p: &str) -> bool {
            self.open(p).is_some()
        }
        fn list(&self, _: &str) -> Vec<Entry> {
            Vec::new()
        }
        fn label(&self) -> String {
            "one".into()
        }
    }
    let mut v = Layered::new();
    v.mount("odf", Box::new(f));
    v.mount("odf", Box::new(One));
    assert_eq!(v.open("odf/system/Ship.odf").unwrap(), b"loose");
    assert_eq!(v.open("odf/system/top.odf"), None);
    assert_eq!(v.open("odf/top.odf").unwrap(), b"t");
    assert_eq!(v.mounts()[0].1, "one");
}

#[test]
fn fpq_new_reports_a_bad_archive() {
    assert!(Fpq::new("x", vec![1, 2, 3]).is_err());
}

// ---- a real Roots install (opt-in) ----

fn real_archive() -> Option<(String, Vec<u8>)> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("A2_FO_ODF_FPQ") {
        paths.push(p);
    }
    if let Ok(h) = std::env::var("HOME") {
        paths.push(format!("{h}/Games/Heroic/Star Trek Armada II Fleet Operations Roots/Data/ODF.fpq"));
    }
    paths.into_iter().find_map(|p| std::fs::read(&p).ok().map(|d| (p, d)))
}

/// `cargo test --release --test fpq -- --ignored --nocapture` with Roots installed
/// (or `A2_FO_ODF_FPQ=<file>`): reports counts only.
#[test]
#[ignore]
fn real_odf_fpq_decompresses_and_parses() {
    let Some((path, data)) = real_archive() else {
        eprintln!("no Roots ODF.fpq found; skipping");
        return;
    };
    let a = Archive::parse(data).unwrap();
    let n = a.entries().len();
    let mut ok = 0;
    let mut by_method = [0usize; 2];
    let mut found = 0;
    let mut bytes = 0usize;
    for (i, e) in a.entries().iter().enumerate() {
        let d = a.read(i).unwrap_or_else(|err| panic!("entry {i}: {err:?}"));
        assert_eq!(d.len(), e.unpacked as usize, "entry {i}");
        ok += 1;
        bytes += d.len();
        by_method[e.method as usize] += 1;
        // The hash finds every file, by bare name and by its full path.
        if a.find(&e.name) == Some(i) && a.find(&a.path(i).unwrap()) == Some(i) {
            found += 1;
        }
    }
    assert_eq!(found, n);
    // Every `.odf` parses and reads at least one key.
    let (mut odfs, mut with_keys, mut includes_missing) = (0, 0, 0);
    for (i, e) in a.entries().iter().enumerate() {
        if !e.name.to_ascii_lowercase().ends_with(".odf") {
            continue;
        }
        odfs += 1;
        let o = Odf::load(&e.name, |n| a.open(n)).unwrap();
        if !o.all().is_empty() {
            with_keys += 1;
        }
        includes_missing += o.missing.len();
        let _ = i;
    }
    assert_eq!(odfs, with_keys);
    // The Vfs sees the same files.
    let v = Fpq::new(&path, std::fs::read(&path).unwrap()).unwrap();
    let listed: usize = v.list("system").iter().filter(|e| !e.dir).count();
    eprintln!(
        "{n} files ({} stored, {} lzo), {ok} decompressed to their size ({bytes} bytes), \
         {found} found by hash, {odfs} odfs parsed, {includes_missing} unresolved includes, \
         {listed} listed under system",
        by_method[0], by_method[1]
    );
}
