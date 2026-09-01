use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cbindgen = find_cbindgen();
    let config = format!("{}/cbindgen.toml", crate_dir);

    // Generate utils header
    let utils_out = format!("{}/target/fceux11_rust_utils.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-utils",
            "--config",
            &config,
            "--output",
            &utils_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for utils");
    assert!(status.success(), "cbindgen for utils failed");

    // Generate media header
    let media_out = format!("{}/target/fceux11_rust_media.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-media",
            "--config",
            &config,
            "--output",
            &media_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for media");
    assert!(status.success(), "cbindgen for media failed");

    // Generate formats header
    let formats_out = format!("{}/target/fceux11_rust_formats.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-formats",
            "--config",
            &config,
            "--output",
            &formats_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for formats");
    assert!(status.success(), "cbindgen for formats failed");

    // Generate debug header
    let debug_out = format!("{}/target/fceux11_rust_debug.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-debug",
            "--config",
            &config,
            "--output",
            &debug_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for debug");
    assert!(status.success(), "cbindgen for debug failed");

    // Generate lua header
    let lua_out = format!("{}/target/fceux11_rust_lua.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-lua",
            "--config",
            &config,
            "--output",
            &lua_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for lua");
    assert!(status.success(), "cbindgen for lua failed");

    // Generate core header
    let core_out = format!("{}/target/fceux11_rust_core.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-core",
            "--config",
            &config,
            "--output",
            &core_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for core");
    assert!(status.success(), "cbindgen for core failed");

    // Generate kagami-qa header (P1 scaffold; exports nothing yet)
    let kagami_out = format!("{}/target/fceux11_rust_kagami_qa.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "kagami-qa",
            "--config",
            &config,
            "--output",
            &kagami_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for kagami-qa");
    assert!(status.success(), "cbindgen for kagami-qa failed");

    // v2.1 PPU Refactor — Phase 2 (2026-08-25): cbindgen 0.29.3 does
    // NOT emit declarations for Rust-2024 `pub unsafe extern "C" fn`
    // or `Option<unsafe extern "C" fn(...)>` fields, so the fceux11-ppu
    // crate's FFI surface (functions + `fceux11_ppu_bus_callbacks`
    // struct) never makes it into cbindgen output. We therefore skip
    // the per-crate cbindgen call and hand-write the declarations in
    // `merge_headers` (same pattern as the existing kagami_qa_* manual
    // appends). The staticlib symbol exports still come from the root
    // crate wrappers (see src/rust/src/lib.rs).

    // Generate root-crate header. The root crate `fceux11-rust` is the
    // staticlib producer; its `#[unsafe(no_mangle)]` wrapper functions
    // (e.g. `fceux11_ppu_create`) are the only path by which C++ can
    // reach the workspace's Rust FFI surface, so we must cbindgen this
    // crate separately and merge the result into `fceux11_rust.h`.
    let root_out = format!("{}/target/fceux11_rust_root.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "fceux11-rust",
            "--config",
            &config,
            "--output",
            &root_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for fceux11-rust root crate");
    assert!(status.success(), "cbindgen for root crate failed");

    // Merge into final header
    merge_headers(
        &crate_dir,
        &utils_out,
        &media_out,
        &formats_out,
        &debug_out,
        &lua_out,
        &core_out,
        &kagami_out,
        &root_out,
    );

    // Tell cargo to rerun if sources change
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=crates/fceux11-utils/src");
    println!("cargo:rerun-if-changed=crates/fceux11-media/src");
    println!("cargo:rerun-if-changed=crates/fceux11-formats/src");
    println!("cargo:rerun-if-changed=crates/fceux11-debug/src");
    println!("cargo:rerun-if-changed=crates/fceux11-lua/src");
    println!("cargo:rerun-if-changed=crates/fceux11-core/src");
    println!("cargo:rerun-if-changed=crates/kagami-qa/src");
    println!("cargo:rerun-if-changed=crates/fceux11-ppu/src");
    println!("cargo:rerun-if-changed=crates/fceux11-ppu/cbindgen.toml");
}

fn find_cbindgen() -> String {
    // Try CARGO directory first
    if let Ok(cargo) = env::var("CARGO") {
        let cargo_path = Path::new(&cargo);
        if let Some(bin_dir) = cargo_path.parent() {
            let candidate = bin_dir.join("cbindgen.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Fallback: try PATH
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

fn merge_headers(
    crate_dir: &str,
    utils_path: &str,
    media_path: &str,
    formats_path: &str,
    debug_path: &str,
    lua_path: &str,
    core_path: &str,
    kagami_path: &str,
    root_path: &str,
) {
    let mut output = String::new();
    output.push_str("/* Auto-generated by cbindgen. Do not edit. */\n\n");
    output.push_str("#ifndef FCEUX11_RUST_H\n");
    output.push_str("#define FCEUX11_RUST_H\n\n");
    // v2.1 PPU Refactor — Phase 2: the merged header keeps
    // `#include <stdint.h>` / `<stdbool.h>` (C++ TUs resolve them via
    // MSVC tools; C TUs get them from the MSVC tools include dir which
    // src/CMakeLists.txt adds — see the "MSVC tools include" block).
    // cbindgen emits Rust `bool` as the C keyword `bool`, which MSVC's
    // C compiler treats as an identifier unless <stdbool.h> defines it
    // (Windows SDK 10.0.26100 ships no <stdbool.h>, but the MSVC tools
    // dir does). To be safe for C TUs that include the header before
    // <stdbool.h>, map `bool` → `_Bool` explicitly.
    output.push_str("#include <stdint.h>\n");
    output.push_str("#include <stdbool.h>\n\n");
    output.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

    output.push_str("#ifndef __cplusplus\n");
    output.push_str("#ifndef bool\n");
    output.push_str("#define bool _Bool\n");
    output.push_str("#endif\n");
    output.push_str("#ifndef true\n");
    output.push_str("#define true 1\n");
    output.push_str("#endif\n");
    output.push_str("#ifndef false\n");
    output.push_str("#define false 0\n");
    output.push_str("#endif\n");
    output.push_str("#endif\n\n");
    output.push_str("/* === v0.2.13 Slice types (manually appended) === */\n");
    output.push_str("typedef struct FceuSlice {\n");
    output.push_str("  const uint8_t *ptr;\n");
    output.push_str("  size_t len;\n");
    output.push_str("} FceuSlice;\n\n");
    output.push_str("typedef struct FceuSliceMut {\n");
    output.push_str("  uint8_t *ptr;\n");
    output.push_str("  size_t len;\n");
    output.push_str("} FceuSliceMut;\n\n");

    // Phase 2 of the v2.1 PPU refactor plan — forward declare PpuState
    // so the C/C++ side can hold `*PpuState` opaque handles. The Rust
    // `fceux11_ppu::PpuState` is an opaque struct; cbindgen on the root
    // crate emits `PpuState *` references without a forward declaration
    // (the type is only defined inside `fceux11-ppu/src/state.rs` which
    // isn't in the root crate's cbindgen output), so we manually inject
    // it here to keep the merged header self-contained.
    output.push_str("/* === Phase 2 — Rust PPU opaque type === */\n");
    output.push_str("typedef struct PpuState PpuState;\n\n");

    let mut utils_body = extract_body(&fs::read_to_string(utils_path).unwrap());
    let mut media_body = extract_body(&fs::read_to_string(media_path).unwrap());
    let mut formats_body = extract_body(&fs::read_to_string(formats_path).unwrap());
    let mut debug_body = extract_body(&fs::read_to_string(debug_path).unwrap());
    let mut lua_body = extract_body(&fs::read_to_string(lua_path).unwrap());
    let mut core_body = extract_body(&fs::read_to_string(core_path).unwrap());
    let mut kagami_body = extract_body(&fs::read_to_string(kagami_path).unwrap());
    let mut root_body = extract_body(&fs::read_to_string(root_path).unwrap());

    // v2.1 PPU Refactor — Phase 2 (2026-08-25): type aliases are kept
    // verbatim from cbindgen (`bool`, `uintN_t`, `uintptr_t`). C++ TUs
    // resolve them natively (`bool` is a C++ keyword; `<stdint.h>` is
    // in the MSVC tools include dir, added by src/CMakeLists.txt). C
    // TUs get `bool` → `_Bool` via the `#define bool _Bool` shim at the
    // top of this merged header, and `<stdint.h>`/`<stdbool.h>` from the
    // same MSVC tools include dir. No post-hoc rewriting is needed — a
    // naive `bool` → `unsigned char` replacement breaks C++ signature
    // matching (e.g. `fceux11_cpu_set_nmi_fresh_bridge(bool)` vs
    // `unsigned char` in debug.cpp:536).

    output.push_str(&utils_body);
    if !utils_body.is_empty() && !media_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&media_body);
    if !media_body.is_empty() && !formats_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&formats_body);
    if !formats_body.is_empty() && !debug_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&debug_body);
    if !debug_body.is_empty() && !lua_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&lua_body);
    if !lua_body.is_empty() && !core_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&core_body);
    if !core_body.is_empty() && !kagami_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&kagami_body);

    // v2.1 PPU Refactor — Phase 2 (2026-08-25): hand-written C ABI for
    // the fceux11-ppu crate's bus-callback vtable. cbindgen 0.29.3 cannot
    // emit Rust-2024 `Option<unsafe extern "C" fn>` fields, so the
    // `fceux11_ppu_bus_callbacks` struct never appears in the root crate's
    // cbindgen output — yet the root crate's `fceux11_ppu_install_bus_callbacks`
    // declaration (emitted below from the wrapper) references it by name.
    // We inject the struct + function-pointer typedefs here, BEFORE the
    // root crate body (so the reference resolves). Symbol implementations
    // live in the root crate wrappers (src/rust/src/lib.rs,
    // #[unsafe(no_mangle)]).
    if !kagami_body.is_empty() {
        output.push('\n');
    }
    output.push_str("/* === Phase 2: Rust PPU bus-callback vtable (manual) === */\n");
    output.push_str("typedef uint8_t (*fceux11_ppu_bus_read_fn)(uint32_t addr);\n");
    output.push_str("typedef void (*fceux11_ppu_bus_write_fn)(uint32_t addr, uint8_t value);\n");
    output.push_str("typedef void (*fceux11_ppu_notify_a12_fn)(void);\n");
    output.push_str("typedef void (*fceux11_ppu_notify_hblank_fn)(void);\n");
    output.push_str("typedef void (*fceux11_ppu_notify_hblank2_fn)(void);\n");
    output.push_str("typedef void (*fceux11_ppu_notify_scanline_fn)(int16_t sl);\n");
    output.push_str("typedef void (*fceux11_ppu_notify_vblank_fn)(bool asserted);\n");
    output.push_str("typedef struct fceux11_ppu_bus_callbacks {\n");
    output.push_str("  fceux11_ppu_bus_read_fn read;\n");
    output.push_str("  fceux11_ppu_bus_write_fn write;\n");
    output.push_str("  fceux11_ppu_bus_read_fn cpu_read;\n");
    output.push_str("  fceux11_ppu_notify_a12_fn notify_a12_rising;\n");
    output.push_str("  fceux11_ppu_notify_hblank_fn notify_hblank;\n");
    output.push_str("  fceux11_ppu_notify_hblank2_fn notify_hblank2;\n");
    output.push_str("  fceux11_ppu_notify_scanline_fn notify_scanline;\n");
    output.push_str("  fceux11_ppu_notify_vblank_fn notify_vblank;\n");
    output.push_str("} fceux11_ppu_bus_callbacks;\n\n");

    // v2.1 Phase 6.3.a — PPU internal data-bus open-bus FFI. The
    // fceux11-ppu crate's cbindgen output is empty (cbindgen 0.29.3
    // cannot emit Rust-2024 `pub unsafe extern "C" fn` declarations
    // — see block comment above). The root crate re-exports these via
    // `#[unsafe(no_mangle)]` wrappers in src/rust/src/lib.rs but those
    // wrappers are themselves invisible to cbindgen (root crate's
    // body extraction strips everything except what cbindgen emits,
    // and cbindgen doesn't follow `fceux11_ppu::ffi::f` calls). We
    // therefore inject the prototypes here, matching the existing
    // "manually appended" pattern below.
    output.push_str("/* === Phase 6.3.a: PPU data-bus open-bus + decay FFI === */\n");
    output.push_str("void fceux11_ppu_set_current_cpu_cycle(PpuState *state, uint64_t current_cpu_cycle);\n");
    output.push_str("void fceux11_ppu_refresh_data_bus(PpuState *state, uint8_t val, uint64_t current_cpu_cycle);\n");
    output.push_str("void fceux11_ppu_check_data_bus_decay(PpuState *state, uint64_t current_cpu_cycle);\n");
    output.push_str("/* === Phase 6.3.b: DMC DMA arbitration scaffolding FFI === */\n");
    output.push_str("void fceux11_ppu_dmc_dma_arbitration(PpuState *state, uint8_t stall_cycles);\n");
    output.push_str("/* === Phase 6.3.c.1: take-and-clear companion to 6.3.b === */\n");
    output.push_str("uint8_t fceux11_ppu_take_dmc_dma_stall(PpuState *state);\n\n");

    if !root_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&root_body);

    // Stage-2 §七 (C-1): the exported C-ABI symbol `kagami_qa_direct_main` now
    // lives in the root crate fceux11-rust (see src/lib.rs wrapper). It is
    // NOT part of any individual member crate's cbindgen output, so we append
    // its declaration here to keep fceux11_rust.h self-contained.
    // Follows the existing "manually appended" pattern (v0.2.13 Slice types).
    output.push_str("\n/* === Stage-2 C-1: in-process direct runner entry === */\n");
    output.push_str("/**\n");
    output.push_str(" * Main entry point called from C++ (kagami_direct_main.cpp).\n");
    output.push_str(" * Parses CLI args and runs Oracle B tests in-process.\n");
    output.push_str(" */\n");
    output.push_str("int32_t kagami_qa_direct_main(int32_t argc, const char *const *argv);\n");

    output.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n");
    output.push_str("#endif /* FCEUX11_RUST_H */\n");

    let final_path = format!("{}/fceux11_rust.h", crate_dir);
    let tmp_path = format!("{}/fceux11_rust.h.tmp", crate_dir);
    fs::write(&tmp_path, &output).expect("Failed to write merged header");

    // Windows: fs::rename cannot overwrite a file locked by another process
    // (parallel C++ compilation may hold a handle on fceux11_rust.h).
    // Retry with remove-then-rename, fall back to copy if all retries fail.
    let mut last_err = None;
    for attempt in 0..10 {
        match fs::rename(&tmp_path, &final_path) {
            Ok(()) => { last_err = None; break; }
            Err(e) => {
                last_err = Some(e);
                let _ = fs::remove_file(&final_path);
                std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
            }
        }
    }
    if let Some(_e) = last_err {
        fs::copy(&tmp_path, &final_path)
            .expect("Failed to copy merged header (rename and copy both failed)");
    }
    let _ = fs::remove_file(&tmp_path);
}

fn extract_body(content: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip includes (we provide our own)
        if trimmed.starts_with("#include") {
            continue;
        }

        // Skip cbindgen header/version comments (we provide our own)
        if trimmed == "/* Auto-generated by cbindgen. Do not edit. */"
            || trimmed.starts_with("/* Generated with cbindgen:")
        {
            continue;
        }

        // Skip header guard directives
        if trimmed.starts_with("#ifndef")
            || trimmed.starts_with("#define")
            || trimmed.starts_with("#endif")
        {
            continue;
        }

        // Empty lines at the start
        if result.is_empty() && trimmed.is_empty() {
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result.trim_end().to_string()
}
