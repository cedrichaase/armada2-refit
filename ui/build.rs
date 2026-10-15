// Generates the import libraries from the .def files beside this crate (llvm-dlltool
// --kill-at: the .defs carry decorated names, the real DLLs export undecorated ones;
// menus/build.sh).
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
}
