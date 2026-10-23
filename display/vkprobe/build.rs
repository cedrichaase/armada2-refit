// Generates the kernel32 and user32 import libraries from their .def files
// (llvm-dlltool --kill-at: the .def carries decorated names, the real DLLs export
// undecorated ones; menus/build.sh), and links the version resource, as ../build.rs
// does for d3d8.dll.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let here = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let tool = env::var("LLVM_DLLTOOL").unwrap_or_else(|_| "llvm-dlltool".into());
    for lib in ["kernel32", "user32"] {
        let def = here.join(format!("{lib}.def"));
        println!("cargo:rerun-if-changed={}", def.display());
        let st = Command::new(&tool)
            .args(["-m", "i386", "--kill-at", "-d"])
            .arg(&def)
            .arg("-l")
            .arg(out.join(format!("{lib}.lib")))
            .status()
            .expect("llvm-dlltool not found (install llvm)");
        assert!(st.success(), "llvm-dlltool failed on {lib}.def");
        println!("cargo:rustc-link-lib={lib}");
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    version_resource(&here, &out);
}

// The version resource (platform/versioninfo/README.md): the probe's own version from
// Cargo.toml (a tool, not a layer: it has no changelog of its own) and the project's
// from the root changelog.
fn version_resource(here: &std::path::Path, out: &std::path::Path) {
    let vi = here.join("../../platform/versioninfo");
    let root = here.join("../../CHANGELOG.md");
    for p in [vi.join("build-res.sh"), vi.join("versioninfo.rc.in"), here.join("../../LICENSE"), root.clone()] {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let text = std::fs::read_to_string(&root).expect("changelog not readable");
    let project = text
        .lines()
        .find_map(|l| {
            let v = l.strip_prefix("## ")?.split(' ').next()?;
            v.starts_with(|c: char| c.is_ascii_digit()).then(|| v.to_owned())
        })
        .expect("changelog has no '## X.Y.Z' heading");
    let res = out.join("version.res");
    let st = Command::new("bash")
        .arg(vi.join("build-res.sh"))
        .arg(&res)
        .args(["vkprobe.exe", "Armada II Refit: Vulkan probe", env!("CARGO_PKG_VERSION"), &project])
        .status()
        .expect("bash not found");
    assert!(st.success(), "build-res.sh failed (llvm-rc is in llvm)");
    println!("cargo:rustc-link-arg={}", res.display());
}
