// Generates the kernel32 and user32 import libraries from their .def files (llvm-dlltool --kill-at: the
// .def carries decorated names, the real DLL exports undecorated ones; menus/build.sh).
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
    version_resource(&here, &out, "d3d8.dll", "Armada II Refit: Direct3D 8 display");
}

// The DLL's version resource, compiled by platform/versioninfo/build-res.sh with the
// layer's and the project's versions from their changelogs, and linked like an object
// (platform/versioninfo/README.md).
fn version_resource(here: &std::path::Path, out: &std::path::Path, file: &str, desc: &str) {
    let vi = here.join("../platform/versioninfo");
    let changelogs = [here.join("CHANGELOG.md"), here.join("../CHANGELOG.md")];
    for p in [vi.join("build-res.sh"), vi.join("versioninfo.rc.in"), here.join("../LICENSE")]
        .iter()
        .chain(&changelogs)
    {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let [layer, project] = changelogs.map(|p| {
        let text = std::fs::read_to_string(&p).expect("changelog not readable");
        text.lines()
            .find_map(|l| {
                let v = l.strip_prefix("## ")?.split(' ').next()?;
                v.starts_with(|c: char| c.is_ascii_digit()).then(|| v.to_owned())
            })
            .expect("changelog has no '## X.Y.Z' heading")
    });
    let res = out.join("version.res");
    let st = Command::new("bash")
        .arg(vi.join("build-res.sh"))
        .arg(&res)
        .args([file, desc, &layer, &project])
        .status()
        .expect("bash not found");
    assert!(st.success(), "build-res.sh failed (llvm-rc is in llvm)");
    println!("cargo:rustc-link-arg={}", res.display());
}
