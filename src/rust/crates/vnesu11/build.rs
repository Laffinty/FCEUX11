// vNESU11 build.rs (Phase 0 §1.1).
//
// Generates `target/vnesu11_ffi.h` via cbindgen for C++ inclusion. Mirrors
// the pattern in `src/rust/build.rs` but is scoped to this single crate
// — Phase 6 will fold vNESU11's generated header into the merged
// `fceux11_rust.h` once the full FFI surface stabilizes.
//
// Phase 0: only `lifecycle + mapper per-range + peek/poke + savestate
// stubs` are exposed; everything else is `pub` but not yet wired.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| format!("{}/target", crate_dir));
    let _ = fs::create_dir_all(&out_dir);

    let cbindgen = find_cbindgen();
    let config = format!("{}/cbindgen.toml", crate_dir);
    let out_path = format!("{}/vnesu11_ffi.h", out_dir);

    let status = Command::new(&cbindgen)
        .args([
            "--crate", "vnesu11",
            "--config", &config,
            "--output", &out_path,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for vnesu11");
    assert!(status.success(), "cbindgen for vnesu11 failed");

    // Rerun triggers.
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/soc.rs");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/cpu/regs.rs");
    println!("cargo:rerun-if-changed=src/mapper.rs");
}

/// Locate `cbindgen.exe` — prefer `CARGO_HOME/bin`, then `PATH`.
fn find_cbindgen() -> String {
    if let Ok(cargo) = env::var("CARGO") {
        let cargo_path = Path::new(&cargo);
        if let Some(bin_dir) = cargo_path.parent() {
            let candidate = bin_dir.join("cbindgen.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(';') {
            let candidate = Path::new(dir).join("cbindgen.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    panic!("cbindgen.exe not found. Install with: cargo install cbindgen");
}
