// Only for the preview test (feature `preview`): bakes the vendored fonts into OUT_DIR so
// the test can include ui/src/text and render through the runtime itself.
#[allow(dead_code)]
#[path = "src/lib.rs"]
mod fontbake;

fn main() {
    if std::env::var_os("CARGO_FEATURE_PREVIEW").is_none() {
        return;
    }
    let here = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fontbake::bake_fonts(&here.join("../../platform/vendor"), &out);
    println!("cargo:rerun-if-changed=src/lib.rs");
}
