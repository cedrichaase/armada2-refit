//! Host tests of render3d's pure parts, which build without the game's runtime:
//!
//!   cargo test --release --target x86_64-unknown-linux-gnu --manifest-path render3d/hosttest/Cargo.toml
//!
//! `scene::media` (light semantics as data, phase 7 S3): the word a point light packs
//! for the shaders is the one the old `Gas` enum and day-side flag gave, for every old
//! kind with and without the day side, and the gas's march still reads the slots the
//! table names.
#[path = "../../src/scene/media.rs"]
pub mod media;

#[cfg(test)]
mod tests {
    use super::media::*;

    /// The old packing, kept here only: `clusters::gather` before S3.
    mod old {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[repr(u32)]
        pub enum Gas {
            Plain = 0,
            Fireball = 1,
            Bolt = 2,
            OwnGlow = 3,
            Spark = 4,
        }
        pub const ALL: [Gas; 5] = [Gas::Plain, Gas::Fireball, Gas::Bolt, Gas::OwnGlow, Gas::Spark];
        const DAY_SIDE: u32 = 16;
        pub const KIND_LINE: u32 = 1;
        pub const KIND_SPOT: u32 = 2;
        pub fn point(day: bool, key: bool, g: u32, gas: Gas) -> [u32; 4] {
            [if day && key { DAY_SIDE } else { 0 }, g, gas as u32, 0]
        }
        pub fn shape(kind: u32) -> [u32; 4] {
            [kind, 0, 0, 0]
        }
    }

    /// The medium each module registers for what was each `Gas` kind.
    fn params(g: old::Gas) -> MediumParams {
        match g {
            old::Gas::Plain => PLAIN,
            old::Gas::Fireball => FIREBALL,
            old::Gas::Bolt => BOLT,
            old::Gas::OwnGlow => OWN_GLOW,
            old::Gas::Spark => SPARK,
        }
    }

    #[test]
    fn point_word_unchanged() {
        for g in old::ALL {
            let id = slot_of(params(g)).expect("every old kind has a slot");
            for day in [false, true] {
                for key in [false, true] {
                    for group in [0u32, 1, 7, 2047] {
                        assert_eq!(point_word(day && key, group, id), old::point(day, key, group, g), "{:?} day {} key {} group {}", g, day, key, group);
                    }
                }
            }
        }
        assert_eq!(MediumId::PLAIN, slot_of(PLAIN).unwrap());
    }

    #[test]
    fn shape_word_unchanged() {
        assert_eq!(shape_word(KIND_LINE), old::shape(old::KIND_LINE));
        assert_eq!(shape_word(KIND_SPOT), old::shape(old::KIND_SPOT));
    }

    #[test]
    fn slots_are_distinct_and_reserved_ones_empty() {
        for (i, a) in MEDIA.iter().enumerate() {
            for b in &MEDIA[i + 1..] {
                if a.is_some() {
                    assert_ne!(a, b);
                }
            }
        }
        assert!(MEDIA[5..].iter().all(|m| m.is_none()));
        // A response the shader has none for is no slot.
        assert_eq!(slot_of(MediumParams { response: Response::Fireball, share: Share::Bolt }), None);
    }

    fn fnv(s: &str) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for b in s.replace("\r\n", "\n").bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn shader(name: &str) -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../shaders/").to_string() + name).expect("shader source")
    }

    /// The text of a function, from its signature to the closing brace at column 0.
    fn function(src: &str, sig: &str) -> String {
        let a = src.find(sig).unwrap_or_else(|| panic!("{} not found", sig));
        let b = src[a..].find("\n}\n").expect("end of function") + a + 3;
        src[a..b].to_string()
    }

    /// The gas's march reads the slots `MEDIA` names, and its falloff is as it was:
    /// the literals by text, the functions by hash (as of S3; a change to them is a
    /// change to the look, which this test is here to catch).
    #[test]
    fn shader_reads_the_table() {
        let neb = shader("world_nebula.hlsl");
        let nl = function(&neb, "float3 nl_light(float3 p)");
        let at = |p: MediumParams| slot_of(p).unwrap().0;
        assert!(nl.contains(&format!("gk == {}u)", at(OWN_GLOW))), "own glow skipped");
        assert!(nl.contains(&format!("if (gk == {}u || gk == {}u) {{", at(BOLT), at(SPARK))), "bolt and spark coronas");
        assert!(nl.contains(&format!("if (gk == {}u) {{", at(FIREBALL))), "fireball core");
        assert!(nl.contains(&format!("c *= gk == {}u ? k_nlk.x : gk == {}u ? k_nlk.y : gk == {}u ? k_nlk.w", at(FIREBALL), at(BOLT), at(SPARK))), "shares");
        assert!(nl.contains(&format!("(l.t.x & {}u) != 0u", FLAG_SHARE)), "day side skipped in the gas");
        assert!(nl.contains("uint gk = l.t.z;"), "medium in t.z");
        let cl = shader("clustered.hlsli");
        assert!(cl.contains(&format!("(l.t.x & {}u) != 0u", FLAG_SHARE)));
        assert!(cl.contains(&format!("uint kind = l.t.x & 15u;")));
        let pinned = [
            ("nl_light", fnv(&nl)),
            ("cl_eval", fnv(&function(&cl, "float3 cl_eval(ClLight l, float3 wp, out float3 L)"))),
            ("cl_line", fnv(&function(&cl, "float3 cl_line(ClLight l, float3 wp, out float3 L)"))),
        ];
        let changed: Vec<String> =
            pinned.iter().zip(PINNED.iter()).filter(|(g, w)| g != w).map(|(g, _)| format!("{} is now {:#x}", g.0, g.1)).collect();
        assert!(changed.is_empty(), "the gas's falloff changed: {}", changed.join(", "));
    }

    /// As of render3d 0.37.2 (S3), the shaders as they were before it.
    const PINNED: [(&str, u64); 3] = [("nl_light", 0x608a65a830f13fd1), ("cl_eval", 0xe2bfabab8fe1ee39), ("cl_line", 0x4ae7440e5b7b9411)];
}
