//! Sky recipes (sky/README.md, "Recipes"): a handful of numbers per stock sky, turned
//! into the pixel shader's constants c0..c21 exactly as `Sky.asi`'s `recipe_read` does.
//!
//! The recipes are `sky/skies/<name>.ini`, built into the plugin, so the sky needs no
//! file in the game directory. A file `Sky\<name>.ini` in the game directory (where
//! `sky/install.sh` puts them) takes precedence, and is re-read while the game runs
//! when it changes (`Reload=1`), so a recipe can still be tuned live.
use crate::math::{cosf, fmodf, sinf};
use crate::rt;
use alloc::string::String;
use alloc::vec::Vec;

/// The constants c0..c21 (`sky.hlsl`).
pub const NCONST: usize = 22;

/// The recipes of every stock sky with one, by background name (no extension).
static BUILT_IN: &[(&str, &str)] = &[
    ("mbg02", include_str!("../../../../sky/skies/mbg02.ini")),
    ("mbgaqu", include_str!("../../../../sky/skies/mbgaqu.ini")),
    ("mbgbaku", include_str!("../../../../sky/skies/mbgbaku.ini")),
    ("mbgblue", include_str!("../../../../sky/skies/mbgblue.ini")),
    ("mbgborg", include_str!("../../../../sky/skies/mbgborg.ini")),
    ("mbgdk", include_str!("../../../../sky/skies/mbgdk.ini")),
    ("mbgdom1", include_str!("../../../../sky/skies/mbgdom1.ini")),
    ("mbgflu", include_str!("../../../../sky/skies/mbgflu.ini")),
    ("mbggb", include_str!("../../../../sky/skies/mbggb.ini")),
    ("mbgkl", include_str!("../../../../sky/skies/mbgkl.ini")),
    ("mbgkling", include_str!("../../../../sky/skies/mbgkling.ini")),
    ("mbgpur", include_str!("../../../../sky/skies/mbgpur.ini")),
    ("mbgred", include_str!("../../../../sky/skies/mbgred.ini")),
    ("mbgrg", include_str!("../../../../sky/skies/mbgrg.ini")),
];

/// The fine detail's noise volume: one feature of its noise spans about three texels
/// of a 64-texel tile (`bake::FINE_N`).
const FINE_FEATURE: f32 = 3.0 / 64.0;
const DEG: f32 = 0.017_453_293;

pub struct Recipe {
    /// The base name, `mbgkling` for `mbgkling.sod`.
    pub name: String,
    pub k: [[f32; 4]; NCONST],
    /// The text it was read from, to notice an edit.
    pub text: String,
    /// Where it came from, for the log.
    pub source: &'static str,
}

/// The recipe's base name for a background name: "mbgkling.sod" and "mbgaqu" become
/// "mbgkling" and "mbgaqu".
pub fn base_name(bg: &str) -> String {
    let end = bg.find(['.', '\\', '/']).unwrap_or(bg.len());
    rt::lower(&bg[..end])
}

/// The recipe text for a base name: the game directory's file first, then the built-in.
pub fn text_for(base: &str) -> Option<(String, &'static str)> {
    let mut path = String::from("Sky\\");
    path.push_str(base);
    path.push_str(".ini");
    if let Some(d) = rt::read_file(&rt::game_path(&path)) {
        return Some((String::from_utf8_lossy(&d).into_owned(), "Sky\\ in the game directory"));
    }
    BUILT_IN.iter().find(|(n, _)| *n == base).map(|(_, t)| (String::from(*t), "built in"))
}

/// The `[Sky]` section's keys, lower-cased, with their values (comments stripped).
fn keys(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_sky = false;
    for line in text.lines() {
        let l = line.split(';').next().unwrap_or("").trim();
        if l.starts_with('[') {
            in_sky = l.eq_ignore_ascii_case("[sky]");
            continue;
        }
        if !in_sky {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            out.push((rt::lower(k.trim()), String::from(v.trim())));
        }
    }
    out
}

/// `n` numbers, separated by commas or spaces; None unless all are there (as
/// `Sky.asi`'s `parsen`).
fn nums<const N: usize>(kv: &[(String, String)], key: &str) -> Option<[f32; N]> {
    let key = rt::lower(key);
    let v = &kv.iter().rev().find(|(k, _)| *k == key)?.1;
    let mut out = [0.0f32; N];
    let mut it = v.split([',', ' ', '\t']).filter(|p| !p.is_empty());
    for o in out.iter_mut() {
        *o = rt::parse_f32(it.next()?)?;
    }
    Some(out)
}

fn yaw_pitch(yaw: f32, pitch: f32) -> [f32; 4] {
    let (y, p) = (yaw * DEG, pitch * DEG);
    [cosf(p) * sinf(y), sinf(p), cosf(p) * cosf(y), 0.0]
}

impl Recipe {
    pub fn parse(base: &str, text: String, source: &'static str) -> Recipe {
        let kv = keys(&text);
        let mut k = [[0.0f32; 4]; NCONST];
        let rgb = |key: &str| -> [f32; 4] {
            let v = nums::<3>(&kv, key).unwrap_or([0.0; 3]);
            [v[0] / 255.0, v[1] / 255.0, v[2] / 255.0, 0.0]
        };
        let one = |key: &str, def: f32| nums::<1>(&kv, key).map_or(def, |v| v[0]);
        k[0] = rgb("Deep");
        k[1] = rgb("GasA");
        k[2] = rgb("GasB");
        k[3] = rgb("Glow");
        k[4] = [one("Scale", 2.0), one("Warp", 0.4), one("Coverage", 1.0), one("Softness", 2.0).max(0.01)];
        let seed = one("Seed", 1.0).abs();
        k[5] = [fmodf(seed * 12.9898, 97.0), fmodf(seed * 78.233, 89.0), fmodf(seed * 37.719, 83.0), one("HueScale", 2.0)];
        for i in 0..2 {
            let (key, col) = if i == 0 { ("Core1", "Core1Colour") } else { ("Core2", "Core2Colour") };
            let c = nums::<4>(&kv, key).unwrap_or([0.0, 0.0, 10.0, 0.0]);
            k[6 + 2 * i] = yaw_pitch(c[0], c[1]);
            k[6 + 2 * i][3] = c[2] * DEG;
            k[7 + 2 * i] = rgb(col);
            k[7 + 2 * i][3] = c[3];
        }
        let band = nums::<4>(&kv, "Band").unwrap_or([0.0, 90.0, 20.0, 0.0]);
        k[13] = yaw_pitch(band[0], band[1]);
        k[13][3] = band[2] * DEG;
        let st = nums::<3>(&kv, "Stretch").unwrap_or([0.0, 90.0, 1.0]);
        k[10] = yaw_pitch(st[0], st[1]);
        k[10][3] = st[2].max(1.0);
        k[11] = [one("Brightness", 1.0), one("Ridge", 0.0), one("Patchiness", 0.5), one("Detail", 0.5)];
        k[12] = [band[3], one("Dither", 1.0), one("Gamma", 1.0).max(0.05), one("HueMix", 0.5)];
        let grain = nums::<3>(&kv, "Grain").unwrap_or([0.0, 8.0, 0.6]);
        let dust = nums::<3>(&kv, "Dust").unwrap_or([0.0, 6.0, 0.5]);
        let haze = nums::<2>(&kv, "Haze").unwrap_or([0.0, 0.0]);
        k[14] = [grain[0], grain[1], grain[2], haze[0]];
        k[15] = [dust[0], dust[1], dust[2], haze[1]];
        let stars = nums::<4>(&kv, "Stars")
            .or_else(|| nums::<3>(&kv, "Stars").map(|s| [s[0], s[1], s[2], 0.0]))
            .unwrap_or([0.0, 1.0, 0.5, 0.0]);
        k[16] = [stars[0] / 314.0, stars[1], stars[2] * DEG, stars[3]];
        k[17] = rgb("StarA");
        k[18] = rgb("StarB");
        let gain = nums::<3>(&kv, "GradeGain").unwrap_or([1.0; 3]);
        let curve = nums::<3>(&kv, "GradeCurve").unwrap_or([1.0; 3]);
        k[19] = [gain[0], gain[1], gain[2], 0.0];
        k[20] = [curve[0].max(0.2), curve[1].max(0.2), curve[2].max(0.2), 0.0];
        let fine = nums::<3>(&kv, "Fine").unwrap_or([0.1, 0.3, 0.55]);
        let size = fine[1].max(0.01);
        k[21] = [fine[0], FINE_FEATURE / (size * DEG), fine[2], 0.5 * fine[0] * fine[0]];
        Recipe { name: String::from(base), k, text, source }
    }

    pub fn floats(&self) -> &[f32] {
        unsafe { core::slice::from_raw_parts(self.k.as_ptr() as *const f32, NCONST * 4) }
    }
}
