//! The ordnance module's built-in tables (README.md, "Content"): its recipes, as data.

/// The special weapons' and waves' recipes and the warp flash
/// (`render3d/content/ordnance/`), as `recipes` takes them: `Render3D.ini` overrides
/// them section by section, key by key (`ini.rs`), so there is no copy to install and merge.
pub static RECIPES: [(&str, &str); 1] = [("ordnance", include_str!("../../../content/ordnance/ordnance.ini"))];

/// A SOD name as an ODF's `shotGeometry` writes it, in either case.
pub const SOD_EXTS: [&str; 2] = [".sod", ".SOD"];
