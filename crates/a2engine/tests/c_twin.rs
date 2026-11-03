//! csdk/a2engine.h's identity part compiled for the host and run against the same
//! synthetic image as tests/pe.rs (tests/identity.c). Skipped, loudly, without a C
//! compiler; CI has one.
use std::process::Command;

#[test]
fn c_twin_agrees() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let exe = std::env::temp_dir().join(format!("a2e_identity_{}", std::process::id()));
    let mut built = false;
    for cc in ["cc", "clang", "gcc"] {
        if let Ok(s) = Command::new(cc).args(["-std=c99", "-Wall", "-Werror", "-o"]).arg(&exe).arg(format!("{dir}/tests/identity.c")).status() {
            if s.success() {
                built = true;
                break;
            }
            panic!("{cc} could not compile tests/identity.c");
        }
    }
    if !built {
        eprintln!("no C compiler: the C twin was not tested");
        return;
    }
    let out = Command::new(&exe).output().expect("run the C test");
    let _ = std::fs::remove_file(&exe);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}
