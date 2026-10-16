//! The module's settings: `render3d/ordnance.ini` (the special weapons' recipes and the
//! warp flash, ported from Specials.ini) is built into the plugin, and Render3D.ini
//! overrides it key by key, section by section. Built in, because an installed copy
//! would have to be merged with the player's edits section by section, which
//! `install.sh`'s key merge does not do.
use crate::rt::{self, Ini};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

const BUILT_IN: &str = include_str!("../../../ordnance.ini");

pub struct Settings {
    /// lower-case section -> lower-case key -> value
    base: BTreeMap<String, BTreeMap<String, String>>,
    /// The section names in the order the built-in file has them (as written).
    order: Vec<String>,
    user: Ini,
    /// The sections Render3D.ini has, lower case (only those are asked).
    user_secs: Vec<String>,
}

impl Settings {
    pub fn load() -> Settings {
        let mut base: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut order = Vec::new();
        let mut cur = String::new();
        for line in BUILT_IN.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with(';') || l.starts_with('#') {
                continue;
            }
            if l.starts_with('[') && l.ends_with(']') {
                let name = &l[1..l.len() - 1];
                cur = rt::lower(name);
                if !base.contains_key(&cur) {
                    order.push(String::from(name));
                }
                base.entry(cur.clone()).or_default();
                continue;
            }
            if let Some(eq) = l.find('=') {
                let (k, v) = (l[..eq].trim(), l[eq + 1..].trim());
                let v = v.split(';').next().unwrap_or("").trim();
                // The first of a repeated key wins, as GetPrivateProfileString reads it.
                base.entry(cur.clone()).or_default().entry(rt::lower(k)).or_insert_with(|| String::from(v));
            }
        }
        let user = crate::config::ini();
        let user_secs = user.sections().iter().map(|s| rt::lower(s)).collect();
        Settings { base, order, user, user_secs }
    }

    pub fn get(&self, sec: &str, key: &str) -> Option<String> {
        let ls = rt::lower(sec);
        if self.user_secs.contains(&ls) {
            if let Some(v) = self.user.string(sec, key) {
                return Some(v);
            }
        }
        self.base.get(&ls)?.get(&rt::lower(key)).cloned()
    }

    pub fn f(&self, sec: &str, key: &str, def: f32) -> f32 {
        self.get(sec, key).and_then(|s| rt::parse_f32(&s)).unwrap_or(def)
    }

    pub fn i(&self, sec: &str, key: &str, def: i32) -> i32 {
        self.get(sec, key).and_then(|s| rt::parse_i32(&s)).unwrap_or(def)
    }

    pub fn s(&self, sec: &str, key: &str, def: &str) -> String {
        self.get(sec, key).unwrap_or_else(|| String::from(def))
    }

    /// Up to N numbers separated by commas or spaces; the rest keep `def`.
    pub fn n<const N: usize>(&self, sec: &str, key: &str, def: [f32; N]) -> [f32; N] {
        let mut out = def;
        if let Some(s) = self.get(sec, key) {
            for (i, part) in s.split([',', ' ', '\t']).filter(|p| !p.is_empty()).take(N).enumerate() {
                if let Some(v) = rt::parse_f32(part) {
                    out[i] = v;
                }
            }
        }
        out
    }

    /// A colour given 0..255, as 0..1.
    pub fn rgb(&self, sec: &str, key: &str, def: [f32; 3]) -> [f32; 3] {
        let v = self.n(sec, key, def);
        [v[0] / 255.0, v[1] / 255.0, v[2] / 255.0]
    }

    /// Every section name, built-in first, then the ones only Render3D.ini has.
    pub fn sections(&self) -> Vec<String> {
        let mut v = self.order.clone();
        for s in self.user.sections() {
            if !self.base.contains_key(&rt::lower(&s)) {
                v.push(s);
            }
        }
        let _ = &self.user_secs;
        v
    }
}
