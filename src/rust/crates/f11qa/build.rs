// Stage-2 S-4: stamp the git revision into the binary at COMPILE time.
//
// `report/matrix.rs` reads the revision with `option_env!("FCEUX11_GIT_REV")`,
// which is evaluated by the compiler, not at run time. `scripts/run_matrix.ps1`
// used to set `$env:FCEUX11_GIT_REV` just before *launching* the runner — far
// too late to have any effect. Every locally produced migration matrix was
// therefore stamped `git_rev: "unknown"`, which is exactly what made stale
// artifacts indistinguishable from fresh ones.
//
// This build script closes that gap: the revision is resolved here and handed
// to rustc via `cargo:rustc-env`, so it cannot be forgotten by a caller.
// An externally supplied FCEUX11_GIT_REV still wins (CI may know a better
// answer than `git` does inside a shallow checkout).

use std::process::Command;

fn main() {
    let rev = std::env::var("FCEUX11_GIT_REV")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(git_short_rev)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=FCEUX11_GIT_REV={}", rev.trim());

    // Re-stamp when HEAD moves (branch switch / new commit) or when the caller
    // overrides the value. Paths are relative to this crate's directory:
    // src/rust/crates/kagami-qa -> repo root is four levels up.
    println!("cargo:rerun-if-env-changed=FCEUX11_GIT_REV");
    println!("cargo:rerun-if-changed=../../../../.git/HEAD");
}

fn git_short_rev() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let rev = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if rev.is_empty() { None } else { Some(rev) }
}
