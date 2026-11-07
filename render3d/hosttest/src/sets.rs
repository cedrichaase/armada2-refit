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
            profile_by_race: false,
            profile_races: &[],
            race_key: "race",
            bump_maps_normal: false,
        };
        assert_eq!(c, old);
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn hulls_fleetops_keys_the_profile_by_race() {
        use hulls::*;
        let c = parse(include_str!("../../content/sets/fleetops/hulls.ini"));
        assert!(c.profile_by_race);
        assert!(c.bump_maps_normal);
        assert_eq!(c.profile_races, ["borg", "freeborg"]);
        assert_eq!(c.race_key, "race");
        assert_eq!(c.bump_profile, "Borg");
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
        assert_eq!(parse(stock("overlays")), Content { selection_model: "select", engine_moves_all: false });
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn overlays_fleetops_draws_every_engine_move() {
        use overlays::*;
        assert_eq!(parse(fleetops("overlays")), Content { selection_model: "select", engine_moves_all: true });
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

    /// The fleetops set's table for a module.
    fn fleetops(module: &str) -> &'static str {
        content::file(&format!("sets/fleetops/{module}.ini")).unwrap_or_else(|| panic!("no fleetops {module}.ini"))
    }

    /// Every module has a table in the fleetops set, and every file one names is built in.
    #[test]
    fn fleetops_has_every_module_and_only_files_that_exist() {
        for m in ["effects", "hulls", "ordnance", "overlays", "particles", "sky", "specials", "world"] {
            let t = a2ini::Table::parse(fleetops(m));
            for (sec, es) in &t.sections {
                if ["Recipes", "Surfaces", "Nebulae"].contains(sec) {
                    for (n, f) in es {
                        assert!(content::file(f).is_some(), "fleetops {m}.ini [{sec}] {n} names {f}, which is not in render3d/content/");
                    }
                }
            }
        }
    }

    #[test]
    fn fleetops_effects_key_factions_by_race() {
        use effects::*;
        let c = parse(fleetops("effects"));
        assert!(c.by_race());
        let key = |race: &str| c.faction_of_race(race).map(|i| c.factions[i].key);
        assert_eq!(key("unitedfederation"), Some("Federation"));
        assert_eq!(key("FreeBorg"), Some("Borg"));
        assert_eq!(key("species8472"), Some("Species8472"));
        assert_eq!(key("dominion"), Some("Other"));
        assert_eq!(key(""), Some("Other"));
        // The stock set keys by initial, as before.
        assert!(!parse(stock("effects")).by_race());
        assert_eq!(parse(stock("effects")).faction_of_race("borg"), Some(6));
    }

    /// Every ODF the fleetops ordnance table names maps to a section of the built-in
    /// recipes, and Roots' ionstm (another weapon than stock's) is not named.
    #[test]
    fn fleetops_ordnance_names_name_recipes() {
        use ordnance::*;
        let c = parse(fleetops("ordnance"));
        assert_eq!(c.recipes, parse(stock("ordnance")).recipes);
        assert!(!c.names.is_empty());
        let recipes = a2ini::Table::parse(c.recipes[0].1);
        for &(odf, sec) in c.names {
            let prefix = if odf.starts_with("Ordnance.") { "Ordnance." } else if odf.starts_with("Wave.") { "Wave." } else { panic!("{odf}: neither Ordnance. nor Wave.") };
            let want = format!("{prefix}{sec}");
            assert!(recipes.sections.iter().any(|(s, _)| s.eq_ignore_ascii_case(&want)), "{odf} = {sec}: no [{want}] in the recipes");
        }
        assert!(!c.names.iter().any(|(o, _)| o.eq_ignore_ascii_case("Ordnance.ionstm")));
        assert!(parse(stock("ordnance")).names.is_empty());
    }

    /// Roots as installed, when it is: every ODF the fleetops set names is in its
    /// `ODF.fpq` with the classLabel the set's comments give it, and the nebula classes'
    /// stock recipes are the stock nebula of the same nebulaType.
    #[test]
    fn fleetops_names_are_roots_odfs() {
        let Some(fpq) = roots_odf_fpq() else { return };
        let a = a2fmt::fpq::Archive::parse(fpq).unwrap();
        let class = |name: &str| -> Option<String> {
            let d = a.open(&format!("{name}.odf"))?;
            a2fmt::odf::Odf::parse_single(name, &d).class_label()
        };
        let want = [
            ("npc_tractorbeamo", "tractor_bm"), ("borg_ships_holo", "holding_bm"), ("borg_stations_holo", "holding_bm"),
            ("clairvoyance", "cvoyantlink"), ("gdisso", "micro"), ("hologen", "hologenerator"), ("shieldi", "shieldinv"),
            ("overrid", "override"), ("all_mine_dilithiumo", "minebm"), ("all_mine_dilithium2o", "minebm"),
            ("all_mine_tritaniumo", "minebm"), ("all_mine_tritanium2o", "minebm"), ("fed_repair_beamo", "minebm"),
            ("gcloakenhancew", "shockwavexplode"), ("weaponenhancerwave", "shockwavexplode"), ("ionstm", "utribeam"),
        ];
        for (odf, cl) in want {
            assert_eq!(class(odf).as_deref(), Some(cl), "{odf}");
        }
        let c = ordnance::parse(fleetops("ordnance"));
        for &(odf, _) in c.names {
            let n = odf.split_once('.').unwrap().1;
            assert!(want.iter().any(|(w, _)| w.eq_ignore_ascii_case(n)), "{n} is named but not checked here");
        }
        let w = world::parse(fleetops("world"));
        assert!(!w.nebulae.is_empty());
        let t = a2ini::Table::parse(fleetops("world"));
        for (odf, file) in t.entries("Nebulae") {
            let d = a.open(&format!("{odf}.odf")).unwrap_or_else(|| panic!("{odf} is not in ODF.fpq"));
            let o = a2fmt::odf::Odf::parse_single(odf, &d);
            assert_eq!(o.class_label().as_deref(), Some("nebula_obj"), "{odf}");
            let ty = o.get_i32("nebulaType").unwrap();
            let stock_ty = match *file {
                "nebulae/mnebula7.ini" => 2,
                "nebulae/mnebula8.ini" => 3,
                "nebulae/mnebula9.ini" => 4,
                "nebulae/mnebula10.ini" => 5,
                f => panic!("{odf}: {f} is not checked here"),
            };
            assert_eq!(ty, stock_ty, "{odf}: nebulaType");
        }
    }

    /// Roots' `Data/ODF.fpq`, when Roots is installed (`a2env.py --game fo-roots`).
    fn roots_odf_fpq() -> Option<Vec<u8>> {
        let out = std::process::Command::new("python3")
            .args([concat!(env!("CARGO_MANIFEST_DIR"), "/../../a2env.py"), "--game", "fo-roots", "--sh"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let game = text.lines().find_map(|l| l.strip_prefix("export A2_GAME="))?.trim_matches('\'').to_string();
        let d = std::fs::read(std::path::Path::new(&game).join("Data/ODF.fpq")).ok();
        if d.is_none() {
            eprintln!("no Roots install: skipped");
        }
        d
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
                menu_title: "",
                gui_sprites: "",
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
        // Stock's fonts draw Windows-1252 throughout: no glyph of their own.
        assert!(c.glyphs.is_empty());
        assert_eq!(parse(""), NONE);
    }

    #[test]
    fn ui_fleetops_info_layout() {
        use ui::*;
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/content/sets/fleetops/ui.ini")).unwrap();
        let c = parse(Box::leak(text.into_boxed_str()));
        let l = &c.info;
        // Roots' panel (infoPanelArea_1) and the places its children were measured at on the bench.
        assert_eq!(l.size, [1249.0, 163.0]);
        assert!(l.lamps);
        let e = |n: &str| l.element(n).unwrap_or_else(|| panic!("no {n}"));
        assert_eq!((e("Name").anchor, e("Name").rect), (Anchor::Left, [29.0, 28.0, 240.0, 20.0]));
        assert_eq!(e("Image").rect, [10.0, 18.0, 512.0, 128.0]);
        let systems: Vec<[f32; 2]> = (0..5).map(|i| { let r = e(&format!("System{i}")).rect; [r[0], r[1]] }).collect();
        assert_eq!(systems, [[537.0, 42.0], [508.0, 71.0], [508.0, 42.0], [537.0, 71.0], [508.0, 100.0]]);
        assert_eq!((e("Queue0").anchor, e("Queue0").rect), (Anchor::Right, [570.0, 31.0, 96.0, 96.0]));
        assert_eq!(e("Progress").anchor, Anchor::Flex);
        assert_eq!(l.number("QueueStep", 0.0), 60.0);
        assert_eq!(l.numbers("SmallStep"), [120.0, 39.0]);
        assert_eq!(l.numbers("WeaponTint"), [153.0, 153.0, 242.0]);
        assert_eq!(l.name("ImageSuffix"), "_si");
        assert_eq!(l.name("WireSuffix"), "w1");
        // Every element lies inside the panel.
        for el in l.elements.iter().filter(|e| !e.name.starts_with("Group") && !e.name.starts_with("Small")) {
            let r = el.rect;
            assert!(r[0] >= 0.0 && r[1] >= 0.0 && r[0] + r[2] <= l.size[0] && r[1] + r[3] <= l.size[1], "{} is outside the panel", el.name);
        }
        // Armada II's set lays its panel out itself.
        let stock = parse(Box::leak(std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/content/sets/stock/ui.ini")).unwrap().into_boxed_str()));
        assert!(stock.info.is_empty());
    }

    #[test]
    fn ui_fleetops_glyphs() {
        use ui::*;
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/content/sets/fleetops/ui.ini")).unwrap();
        let c = parse(Box::leak(text.into_boxed_str()));
        let g = |b: u8| c.glyphs.iter().find(|x| x.0 == b).map(|x| x.1);
        // Roots' red figures, in order, the cost lines' separator (a space, quoted), a
        // resource icon and a unit class (ui/README.md, "The engine's text").
        let figures: Vec<&str> = [0x7f, 0x81, 0x8d, 0x8f, 0x90, 0x9d, 0xa0, 0xad, 0xbe, 0xd7].iter().map(|&b| g(b).unwrap()).collect();
        assert_eq!(figures, ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]);
        assert_eq!(g(b'$'), Some(" "));
        assert_eq!(g(0x85), Some("Dilithium"));
        assert_eq!(g(0xf8), Some("Station"));
        assert_eq!(g(b'A'), None);
        // One line per code.
        let mut codes: Vec<u8> = c.glyphs.iter().map(|x| x.0).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), c.glyphs.len());
        // Tooltips from a player's UI.log on Roots (2026-11-06), as the UI now draws them.
        let cost = "350$\u{85}$/$175$\u{86}$/$8$\u{87}$/$150$\u{89}$/$50$\u{80}";
        assert_eq!(c.display(cost), "350 Dilithium / 175 Tritanium / 8 Supply / 150 Crew / 50 s");
        assert_eq!(c.display("Buy \u{85} for:\n\n -1000 \u{85} / 1000 \u{86} / 100 \u{87}"), "Buy Dilithium for:\n\n -1000 Dilithium / 1000 Tritanium / 100 Supply");
        assert_eq!(c.display(" \u{81}\u{9d}\u{7f}\u{7f} Hitpoints\n\t  \u{aa}"), " 1500 Hitpoints\n   Construction Ship");
        assert_eq!(c.display("Weak vs: \u{a9}, \u{a8}, \u{a3}, \n\t  \u{a5}"), "Weak vs: Artillery, Berserker, Battleship, \n   Dreadnought");
        assert_eq!(c.display("\u{95} Tachyon Detection Grid"), "\u{2022} Tachyon Detection Grid");
        // Twice is once.
        let once = c.display(cost);
        assert_eq!(c.display(&once), once);
        // Stock reads the code page only: 0x95 is the bullet, '$' and 0x85 its own.
        let stock = parse(Box::leak(std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/content/sets/stock/ui.ini")).unwrap().into_boxed_str()));
        assert_eq!(stock.display("\u{95} 5$ \u{85}\t"), "\u{2022} 5$ \u{2026} ");
    }
}
