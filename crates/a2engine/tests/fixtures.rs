//! The resolver fixtures (tools/engine/fixtures/), run by `tools/engine/fixtures.py run
//! rs`, which generates the tables, the images and the expected transcripts and names
//! the module in `A2E_FIXTURES_RS`. The same transcripts the Python and C resolvers
//! print.
#![cfg(feature = "fixtures")]
use a2engine::pe::Image;
use std::sync::Mutex;

pub struct Case {
    pub name: &'static str,
    pub table: &'static a2engine::Table,
    pub image: &'static str,
    pub base: usize,
    /// The module file name the image stands for (`Image::name`).
    pub module: &'static str,
    /// The companion modules the process holds: name and the CRC-32 of its `.text`
    /// (None: its file cannot be read).
    pub modules: &'static [(&'static str, Option<u32>)],
    pub test_ini: Option<&'static str>,
    pub addr: &'static [&'static str],
    /// Field and value ids whose offset or value in the matched build is printed.
    pub field: &'static [&'static str],
    pub note_for: &'static [&'static str],
    /// The content set expected ("none" for no set), "?" when the fixture does not say.
    pub content: &'static str,
    pub expect: &'static [&'static str],
}

include!(env!("A2E_FIXTURES_RS"));

static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn log(s: &str) {
    LINES.lock().unwrap().push(s.to_string());
}

/// A PE file mapped as the loader would: headers, then each section's raw data at its
/// virtual address.
struct Mapped {
    mem: Vec<u8>,
    file: Vec<u8>,
    base: usize,
    module: &'static str,
    modules: &'static [(&'static str, Option<u32>)],
}

impl Image for Mapped {
    fn read(&self, at: usize, out: &mut [u8]) -> bool {
        match at.checked_add(out.len()) {
            Some(end) if end <= self.mem.len() => {
                out.copy_from_slice(&self.mem[at..end]);
                true
            }
            _ => false,
        }
    }
    fn base(&self) -> usize {
        self.base
    }
    fn file_crc(&self, section: &str) -> Option<u32> {
        a2engine::pe::section_crc_file(
            &mut |at, out| match at.checked_add(out.len()) {
                Some(end) if end <= self.file.len() => {
                    out.copy_from_slice(&self.file[at..end]);
                    true
                }
                _ => false,
            },
            section,
        )
    }
    fn companion(&self, name: &str) -> a2engine::pe::Companion {
        use a2engine::pe::Companion;
        match self.modules.iter().find(|m| m.0 == name) {
            None => Companion::NotLoaded,
            Some((_, None)) => Companion::Unreadable,
            Some((_, Some(c))) => Companion::Crc(*c),
        }
    }
    fn name(&self, out: &mut [u8]) -> usize {
        let n = self.module.len().min(out.len());
        out[..n].copy_from_slice(&self.module.as_bytes()[..n]);
        n
    }
}

fn u16_at(b: &[u8], o: usize) -> usize {
    u16::from_le_bytes([b[o], b[o + 1]]) as usize
}
fn u32_at(b: &[u8], o: usize) -> usize {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize
}

fn map(file: &[u8], base: usize, module: &'static str, modules: &'static [(&'static str, Option<u32>)]) -> Mapped {
    let nt = u32_at(file, 0x3c);
    let count = u16_at(file, nt + 6);
    let opt = nt + 24;
    let size = u32_at(file, opt + 56);
    let headers = u32_at(file, opt + 60);
    let mut mem = vec![0u8; size];
    mem[..headers].copy_from_slice(&file[..headers]);
    let table = opt + u16_at(file, nt + 20);
    for i in 0..count {
        let s = table + i * 40;
        let (va, raw, ptr) = (u32_at(file, s + 12), u32_at(file, s + 16), u32_at(file, s + 20));
        mem[va..va + raw].copy_from_slice(&file[ptr..ptr + raw]);
    }
    Mapped { mem, file: file.to_vec(), base, module, modules }
}

fn transcript(c: &Case) -> Vec<String> {
    LINES.lock().unwrap().clear();
    let file = std::fs::read(c.image).expect("the fixture image");
    let mem: &'static Mapped = Box::leak(Box::new(map(&file, c.base, c.module, c.modules)));
    let e = a2engine::init_with(c.table, log, mem, c.test_ini);
    let t = c.table;
    for (k, h) in t.hooks.iter().enumerate() {
        let ok = e.ready(a2engine::HookId(k as u16)).is_ok();
        log(&format!("ready {} {}", h.id, if ok { "ok" } else { "fail" }));
    }
    for id in c.addr {
        let k = t.syms.iter().position(|s| s.id == *id).expect("an addr query names a symbol");
        let v = e.addr(a2engine::Sym(k as u16));
        log(&format!("addr {} {:#x}", id, v));
    }
    for id in c.field {
        let k = t.syms.iter().position(|s| s.id == *id).expect("a field query names a field");
        let v = e.value(a2engine::Sym(k as u16)) as u32;
        assert_eq!(e.field(a2engine::Sym(k as u16)), v as usize, "{}: field and value agree", c.name);
        log(&format!("field {} {:#x}", id, v));
    }
    for (k, h) in t.hooks.iter().enumerate() {
        if let Some(n) = e.note(a2engine::HookId(k as u16)) {
            log(&format!("note {} {}", h.id, n));
        }
    }
    for row in c.note_for {
        let (s, k) = row.split_once('.').unwrap();
        log(&format!("note_for {} {}", row, e.note_for(s, k).unwrap_or("-")));
    }
    log(&format!("note_plugin {}", e.note_plugin().unwrap_or("-")));
    if c.content != "?" {
        assert_eq!(e.content().unwrap_or("none"), c.content, "{}: content set", c.name);
    }
    LINES.lock().unwrap().clone()
}

#[test]
fn fixtures() {
    let mut bad = 0;
    for c in CASES {
        let got = transcript(c);
        if got != c.expect {
            bad += 1;
            let i = (0..got.len().max(c.expect.len()))
                .find(|&i| got.get(i).map(|s| s.as_str()) != c.expect.get(i).copied())
                .unwrap();
            println!("FAIL {}: line {}\n  want {:?}\n  got  {:?}", c.name, i + 1, c.expect.get(i), got.get(i));
        }
    }
    assert_eq!(bad, 0, "{} of {} fixture tables differ", bad, CASES.len());
}
