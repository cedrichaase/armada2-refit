//! The content sets (README.md, "Content sets"): each module's `content.rs`, built here
//! as it is in the module, parses the stock set's table (`render3d/content/sets/stock/`)
//! into exactly the tables the modules held as constants before the sets were data
//! (render3d 1.2.0), and the empty text into `NONE`; the UI's `content.rs` likewise
//! (`ui/content/sets/stock/ui.ini`, ui 2.4.0), with the names that were literals in its
//! screens.
//!
//! The modules' `content.rs` name `r3d_core::content` and `crate::syms` (the UI's
//! `crate::content_sets` and `crate::engine::api`); this crate
//! stands in for both (lib.rs: `extern crate self as r3d_core`), with the built-in
//! files read from `render3d/content/`.

/// What `r3d_core::content` offers a module's `content.rs`, on the host.
pub mod content {
    /// A file of `render3d/content/` by its path there, read once and kept.
    pub fn file(path: &str) -> Option<&'static str> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content").join(path);
        std::fs::read_to_string(p).ok().map(|s| &*Box::leak(s.into_boxed_str()))
    }

    pub struct Cache<T: 'static>(core::marker::PhantomData<T>);

    impl<T> Cache<T> {
        pub const fn new() -> Self {
            Cache(core::marker::PhantomData)
        }
        /// No engine on the host: no set.
        pub fn get<X>(&self, _: &'static str, _: &X, _: fn(&'static str) -> T, none: &'static T) -> &'static T {
            none
        }
    }
}

/// The modules' symbol tables (unused on the host).
pub mod syms {
    pub static TABLE: () = ();
}

/// What the UI's `crate::content_sets` and `crate::engine::api` offer its `content.rs`,
/// on the host.
pub mod content_sets {
    pub struct Cache<T: 'static>(core::marker::PhantomData<T>);

    impl<T> Cache<T> {
        pub const fn new() -> Self {
            Cache(core::marker::PhantomData)
        }
        /// No engine on the host: no set.
        pub fn get<X>(&self, _: X, _: fn(&'static str) -> T, none: &'static T) -> &'static T {
            none
        }
    }
}
pub mod engine {
    pub mod api {
        /// No engine on the host: no set.
        pub fn content_set() -> Option<&'static str> {
            None
        }
    }
}

#[path = "../../../ui/src/content.rs"]
pub mod ui;

#[path = "../../mods/effects/src/content.rs"]
pub mod effects;
#[path = "../../mods/hulls/src/content.rs"]
pub mod hulls;
#[path = "../../mods/ordnance/src/content.rs"]
pub mod ordnance;
#[path = "../../mods/overlays/src/content.rs"]
pub mod overlays;
#[path = "../../mods/particles/src/content.rs"]
pub mod particles;
#[path = "../../mods/sky/src/content.rs"]
pub mod sky;
#[path = "../../mods/specials/src/content.rs"]
pub mod specials;
#[path = "../../mods/world/src/content.rs"]
pub mod world;

/// A set's table as the module builds it in.
pub fn stock(module: &str) -> &'static str {
    content::file(&format!("sets/stock/{module}.ini")).unwrap_or_else(|| panic!("no stock {module}.ini"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recipes the modules built in before (render3d 1.2.0): (name, file under
    /// render3d/content/).
    fn recipes(list: &[(&'static str, &str)]) -> Vec<(&'static str, &'static str)> {
        list.iter().map(|&(n, f)| (n, content::file(f).expect(f))).collect()
    }

    #[test]
    fn effects_stock_is_the_old_table() {
        use effects::*;
        let c = parse(stock("effects"));
        assert_eq!(c.site_names, ["beam", "shot", "mine", "missile"]);
        let old = [
            Faction { key: "Federation", colour: [90.0, 170.0, 255.0], initial: b'f' },
            Faction { key: "Klingon", colour: [12.0, 255.0, 6.0], initial: b'k' },
            Faction { key: "Romulan", colour: [90.0, 255.0, 130.0], initial: b'r' },
            Faction { key: "Borg", colour: [110.0, 255.0, 120.0], initial: b'b' },
            Faction { key: "Cardassian", colour: [255.0, 200.0, 80.0], initial: b'c' },
            Faction { key: "Species8472", colour: [255.0, 140.0, 40.0], initial: b'8' },
            Faction { key: "Other", colour: [140.0, 200.0, 255.0], initial: 0 },
        ];
        assert_eq!(c.factions, old);
        // Were literals in chunks.rs and shields.rs.
        assert_eq!(c.cocoon_sod, "8472_cocoon");
        assert_eq!(c.ray_factions, "Klingon");
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn hulls_stock_is_the_old_table() {
        use hulls::*;
        let c = parse(stock("hulls"));
        let old = Content {
            cocoon_sod: "8472_cocoon",
            odf_ext: ".odf",
            hardpoints_node: "hardpoints",
            lights_nodes: &["lights", "light"],
            damage_parent_nodes: &["life", "engines", "weapons", "shield", "shields", "sensors", "crew", "damage"],
            borg_node: "borg",
            ore_node: "ore",
            scrap_glow_sprite: "glowdmoon",
            resource_colours: &[[0.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.6, 0.9, 1.0], [1.0, 1.0, 0.0], [0.8, 0.8, 0.8], [1.0, 0.0, 0.0]],
            // The prefix of the `Borg*` keys look.rs read by name.
            bump_profile: "Borg",
        };
        assert_eq!(c, old);
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn ordnance_stock_is_the_old_table() {
        use ordnance::*;
        let c = parse(stock("ordnance"));
        assert_eq!(c.recipes, recipes(&[("ordnance", "ordnance/ordnance.ini")]));
        assert_eq!(c.sod_exts, [".sod", ".SOD"]);
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn overlays_stock_is_the_old_table() {
        use overlays::*;
        assert_eq!(parse(stock("overlays")), Content { selection_model: "select" });
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn particles_stock_is_the_old_table() {
        use particles::*;
        let old = Content {
            systems: &["shields", "engines", "target", "sensors", "life", "borg"],
            vent_sprites: ["ventplas", "ventgas"],
            chain_emitter: "crpulsar",
            manheim_emitter: "gemini",
            sod_ext: ".sod",
        };
        assert_eq!(parse(stock("particles")), old);
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn sky_stock_is_the_old_table() {
        use sky::*;
        let c = parse(stock("sky"));
        let names = ["mbg02", "mbgaqu", "mbgbaku", "mbgblue", "mbgborg", "mbgdk", "mbgdom1", "mbgflu", "mbggb", "mbgkl", "mbgkling", "mbgpur", "mbgred", "mbgrg"];
        let files: Vec<String> = names.iter().map(|n| format!("sky/{n}.ini")).collect();
        let old: Vec<(&str, &str)> = names.iter().zip(&files).map(|(n, f)| (*n, f.as_str())).collect();
        assert_eq!(c.recipes, recipes(&old));
        assert_eq!(c.point_stars, "mstars");
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn specials_stock_is_the_old_table() {
        use specials::*;
        let c = parse(stock("specials"));
        assert_eq!(c.names, ["Override", "Drain", "Psionic"]);
        let old: [Defaults; 3] = [
            ([60.0, 255.0, 110.0], 1.6, 1.0, 2.5, 14.0, 0.15, [80.0, 255.0, 120.0], 1.6, 3.0),
            ([235.0, 80.0, 190.0], 1.4, 0.9, 1.5, 10.0, 0.3, [235.0, 90.0, 200.0], 1.0, 2.5),
            ([150.0, 110.0, 255.0], 1.6, 1.0, 0.8, 3.0, 0.6, [160.0, 110.0, 255.0], 1.4, 3.0),
        ];
        assert_eq!(c.defaults, old);
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn world_stock_is_the_old_table() {
        use world::*;
        let c = parse(stock("world"));
        assert_eq!(
            c.surfaces,
            recipes(&[
                ("master1", "surfaces/Master1.ini"),
                ("master2", "surfaces/Master2.ini"),
                ("mdmoon", "surfaces/Mdmoon.ini"),
                ("pb_clssd", "surfaces/PB_CLSSD.ini"),
                ("pb_clssh", "surfaces/PB_CLSSH.ini"),
                ("pb_clssj", "surfaces/PB_CLSSJ.ini"),
                ("pb_clssk", "surfaces/PB_CLSSK.ini"),
                ("pb_clssl", "surfaces/PB_CLSSL.ini"),
                ("pb_clssm", "surfaces/PB_CLSSM.ini"),
            ])
        );
        assert_eq!(
            c.nebulae,
            recipes(&[
                ("fluidicnebula", "nebulae/fluidicnebula.ini"),
                ("generatednebula", "nebulae/generatednebula.ini"),
                ("latinum", "nebulae/latinum.ini"),
                ("mnebula10", "nebulae/mnebula10.ini"),
                ("mnebula12", "nebulae/mnebula12.ini"),
                ("mnebula6", "nebulae/mnebula6.ini"),
                ("mnebula7", "nebulae/mnebula7.ini"),
                ("mnebula8", "nebulae/mnebula8.ini"),
                ("mnebula9", "nebulae/mnebula9.ini"),
            ])
        );
        assert_eq!(c.moon_glow_material, "glowdmoon");
        assert_eq!(parse(""), NONE);
    }

    /// Every file a stock table names is in `render3d/content/` (so it is built in).
    #[test]
    fn stock_tables_name_only_files_that_exist() {
        for m in ["ordnance", "sky", "world"] {
            let t = a2ini::Table::parse(stock(m));
            for (sec, es) in &t.sections {
                if ["Recipes", "Surfaces", "Nebulae"].contains(sec) {
                    for (n, f) in es {
                        assert!(content::file(f).is_some(), "{m}.ini [{sec}] {n} names {f}, which is not in render3d/content/");
                    }
                }
            }
        }
    }

    #[test]
    fn ui_stock_is_the_old_table() {
        use ui::*;
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/content/sets/stock/ui.ini")).unwrap();
        let c = parse(Box::leak(text.into_boxed_str()));
        // ui/src/content.rs before (with each race's look, which theme.rs's match gave).
        let races: [(&str, &str, &[&str]); 6] = [
            ("Federation", "lcars", &["federation", "fed"]),
            ("Klingon", "blade", &["klingon", "kli", "kling"]),
            ("Romulan", "wing", &["romulan", "rom"]),
            ("Cardassian", "arch", &["cardassian", "card", "car"]),
            ("Borg", "cube", &["borg", "bor"]),
            ("Species 8472", "organic", &["species8472", "species 8472", "8472", "species"]),
        ];
        assert_eq!(c.races.len(), 6);
        for (r, (name, theme, aliases)) in c.races.iter().zip(races) {
            assert_eq!((r.name, r.theme, r.aliases), (name, theme, aliases));
        }
        assert_eq!(c.resources, ["Crew", "Officers", "Dilithium", "Latinum", "Metal", "Biomatter"]);
        // (name, key, scene, emblem) as before, then loading.rs's map prefix and kicker.
        let camps = [
            ("Tutorials", "tutorial", "tutorials", None, "a2_tutorial", "Tutorial", false),
            ("Federation Campaign", "federation", "federation", Some((0, 0)), "a2_fed", "Federation campaign", true),
            ("Klingon Campaign", "klingon", "klingon", Some((84, 0)), "a2_kling", "Klingon campaign", true),
            ("Borg Campaign", "borg", "borg", Some((84, 84)), "a2_borg", "Borg campaign", true),
        ];
        assert_eq!(c.campaigns.len(), 4);
        for (k, o) in c.campaigns.iter().zip(camps) {
            assert_eq!((k.name, k.key, k.scene.to_str().unwrap(), k.emblem, k.map, k.kicker, k.numbered), o);
        }
        let tour: Vec<&str> = c.tour.iter().map(|s| s.to_str().unwrap()).collect();
        assert_eq!(tour, ["main", "federation", "klingon", "tutorials", "borg"]);
        assert_eq!(c.main_scene.map(|s| s.to_str().unwrap()), Some("main"));
        assert_eq!(c.first_campaign_scene.map(|s| s.to_str().unwrap()), Some("federation"));
        // Literals in single_player.rs, credits.rs and cursor.rs before.
        assert_eq!(
            c.files,
            Files {
                race_icons: "textures\\allrace.tga",
                credits: "Credits.txt",
                credits_logos: "textures\\logos.tga",
                cursor_sprites: "Sprites/cursor.spr",
            }
        );
        // gallery.rs's lists before.
        let ships = [
            "Galaxy", "Sovereign", "Defiant", "Akira", "Steamrunner", "Intrepid", "Nova", "Saber", "Norway", "Prometheus",
            "Excelsior", "Constitution", "D'Deridex", "Valdore", "Scimitar", "Hawk", "Falcon", "Venture", "Negh'Var", "Vor'cha",
            "Bird of Prey", "Raptor", "Cube", "Sphere", "Diamond", "Scout", "Assimilator", "Queen", "Probe", "Tactical",
        ];
        assert_eq!(c.gallery_ships, ships);
        assert_eq!(c.gallery_factions, ["Federation", "Klingon", "Romulan", "Borg", "Cardassian", "Species 8472"]);
        assert_eq!(parse(""), NONE);
    }
}
