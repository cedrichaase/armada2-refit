// Generates the kernel32 import library from kernel32.def (llvm-dlltool --kill-at: the
// .def carries decorated names, the real DLL exports undecorated ones; menus/build.sh).
use std::{env, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let here = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let def = here.join("kernel32.def");
    println!("cargo:rerun-if-changed={}", def.display());
    let tool = env::var("LLVM_DLLTOOL").unwrap_or_else(|_| "llvm-dlltool".into());
    let st = Command::new(&tool)
        .args(["-m", "i386", "--kill-at", "-d"])
        .arg(&def)
        .arg("-l")
        .arg(out.join("kernel32.lib"))
        .status()
        .expect("llvm-dlltool not found (install llvm)");
    assert!(st.success(), "llvm-dlltool failed");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=kernel32");
}
