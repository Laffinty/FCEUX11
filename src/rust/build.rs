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

    // Generate f11qa header (P1 scaffold; exports nothing yet)
    let f11qa_out = format!("{}/target/fceux11_rust_f11qa.h", crate_dir);
    let status = Command::new(&cbindgen)
        .args([
            "--crate",
            "f11qa",
            "--config",
            &config,
            "--output",
            &f11qa_out,
        ])
        .current_dir(&crate_dir)
        .status()
        .expect("Failed to run cbindgen for f11qa");
    assert!(status.success(), "cbindgen for f11qa failed");

    // Merge into final header
    merge_headers(
        &crate_dir,
        &utils_out,
        &media_out,
        &formats_out,
        &debug_out,
        &lua_out,
        &core_out,
        &f11qa_out,
    );

    // Tell cargo to rerun if sources change
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=crates/fceux11-utils/src");
    println!("cargo:rerun-if-changed=crates/fceux11-media/src");
    println!("cargo:rerun-if-changed=crates/fceux11-formats/src");
    println!("cargo:rerun-if-changed=crates/fceux11-debug/src");
    println!("cargo:rerun-if-changed=crates/fceux11-lua/src");
    println!("cargo:rerun-if-changed=crates/fceux11-core/src");
    println!("cargo:rerun-if-changed=crates/f11qa/src");
    println!("cargo:rerun-if-changed=crates/gba-core/src");
    // The GBA ABI below is hand-written from the root crate's sources, so this
    // build script has to re-run when those change or the merged header goes
    // stale. It was missing from this list, which is why adding five audio
    // exports recompiled the library and left fceux11_rust.h untouched -- a
    // library the C++ side could link but not call. `src/gba` rather than
    // `src/lib.rs`, because it is the module tree that holds the exports.
    println!("cargo:rerun-if-changed=src/gba");
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
    f11qa_path: &str,
) {
    let mut output = String::new();
    output.push_str("/* Auto-generated by cbindgen. Do not edit. */\n\n");
    output.push_str("#ifndef FCEUX11_RUST_H\n");
    output.push_str("#define FCEUX11_RUST_H\n\n");
    output.push_str("#include <stdbool.h>\n");
    output.push_str("#include <stdint.h>\n\n");
    output.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
    output.push_str("/* === v0.2.13 Slice types (manually appended) === */\n");
    output.push_str("typedef struct FceuSlice {\n");
    output.push_str("  const uint8_t *ptr;\n");
    output.push_str("  size_t len;\n");
    output.push_str("} FceuSlice;\n\n");
    output.push_str("typedef struct FceuSliceMut {\n");
    output.push_str("  uint8_t *ptr;\n");
    output.push_str("  size_t len;\n");
    output.push_str("} FceuSliceMut;\n\n");

    let utils_body = extract_body(&fs::read_to_string(utils_path).unwrap());
    let media_body = extract_body(&fs::read_to_string(media_path).unwrap());
    let formats_body = extract_body(&fs::read_to_string(formats_path).unwrap());
    let debug_body = extract_body(&fs::read_to_string(debug_path).unwrap());
    let lua_body = extract_body(&fs::read_to_string(lua_path).unwrap());
    let core_body = extract_body(&fs::read_to_string(core_path).unwrap());
    let f11qa_body = extract_body(&fs::read_to_string(f11qa_path).unwrap());

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
    if !core_body.is_empty() && !f11qa_body.is_empty() {
        output.push('\n');
    }
    output.push_str(&f11qa_body);

    // Stage-2 §七 (C-1): the exported C-ABI symbol `kagami_qa_direct_main` now
    // lives in the root crate fceux11-rust (see src/lib.rs wrapper). It is
    // NOT part of any individual member crate's cbindgen output, so we append
    // its declaration here to keep fceux11_rust.h self-contained.
    // Follows the existing "manually appended" pattern (v0.2.13 Slice types).
    output.push_str("\n/* === Stage-2 C-1: in-process direct runner entry === */\n");
    output.push_str("/**\n");
    output.push_str(" * Main entry point called from C++ (f11qa_direct_main.cpp).\n");
    output.push_str(" * Parses CLI args and runs Hardware Consistency Check tests in-process.\n");
    output.push_str(" */\n");
    output.push_str("int32_t kagami_qa_direct_main(int32_t argc, const char *const *argv);\n");

    // v2.0 GBAEUX11 (S0). The GBA C ABI is declared by this root crate,
    // not by a separate crate: rustc 1.96 fat LTO cannot load the bitcode of
    // a tiny facade rlib in the dependency chain, so the ABI lives here
    // alongside kagami_qa_direct_main.
    //
    // These are written out by hand rather than generated: `build.rs` runs
    // cbindgen per *member* crate, and the GBA ABI is in the root crate,
    // which cbindgen is never pointed at. S0 left the five probes; S2-a adds
    // the lifecycle and frame surface from `src/gba/frame.rs`. S2-b adds the
    // savestate surface from `src/gba/save.rs`.
    //
    // A hand-written list is a real risk -- a function can exist in Rust and
    // be missing here, which is exactly what happened to S2-a's exports.
    // `the_gba_c_abi_is_declared_for_every_exported_function` in
    // `src/gba/ffi.rs` is the guard: it fails the build if the two drift.
    output.push_str("/* v2.0 GBAEUX11 error codes (plan section 4.1) */\n");
    output.push_str("#define GBA_OK 0\n");
    output.push_str("#define GBA_ERR_NO_ROM 1\n");
    output.push_str("#define GBA_ERR_BAD_ROM 2\n");
    output.push_str("#define GBA_ERR_BIOS 3\n");
    // r37 ⑤: the enum in plan section 4.1 has seven members, and these are
    // their values. `UNSUPPORTED` was added and `STATE` moved to 5 — before that
    // this block skipped 4 entirely, so every code from there up was off by one
    // and nothing noticed, because nothing on the C++ side compares against
    // them yet. `the_error_codes_reach_the_generated_header` in
    // `src/gba/ffi.rs` is what keeps the two lists from drifting again.
    output.push_str("#define GBA_ERR_UNSUPPORTED 4\n");
    output.push_str("#define GBA_ERR_STATE 5\n");
    output.push_str("#define GBA_ERR_CAPACITY 6\n\n");

    output.push_str("/* v2.0 GBAEUX11 C ABI */\n");
    output.push_str("uint32_t gba_abi_revision(void);\n");
    output.push_str("uint32_t gba_core_probe(void);\n");
    output.push_str("uint32_t gba_swi_probe(void);\n");
    output.push_str("uint32_t gba_swi_count(void);\n");
    output.push_str("uint32_t gba_probe_lz77_header(uint32_t raw);\n");
    output.push_str("uint64_t gba_probe_cpu_size(void);\n\n");

    output.push_str("/* Lifecycle and frame (S2-a) */\n");
    output.push_str("int32_t gba_init(void);\n");
    output.push_str("int32_t gba_rom_loaded(void);\n");
    output.push_str("int32_t gba_unload_rom(void);\n");
    output.push_str("int32_t gba_last_error(uint8_t *dst, uint32_t cap);\n");
    output.push_str("int32_t gba_load_rom(const char *path);\n");
    output.push_str("int32_t gba_reset(void);\n");
    output.push_str("int32_t gba_step_frame(void);\n");
    output.push_str("int32_t gba_frame_buffer_size(uint32_t *out_size);\n");
    output.push_str("int32_t gba_frame_buffer(uint8_t *dst, uint32_t cap);\n");
    output.push_str("int32_t gba_set_overlay(int32_t enable);\n");

    output.push_str("\n/* Audio (S2-b1) */\n");
    output.push_str("int32_t gba_set_output_rate(uint32_t rate);\n");
    output.push_str("int32_t gba_set_volume(uint32_t volume);\n");
    output.push_str("int32_t gba_render_audio(int32_t *dst, uint32_t cap, uint32_t *out_frames);\n");
    output.push_str("void gba_samples_per_frame_fixed(uint32_t *num, uint32_t *den);\n");
    output.push_str("uint32_t gba_audio_underruns(void);\n");

    output.push_str("\n/* Savestates (S2-b2). The caller owns the buffer: there is no */\n");
    output.push_str("/* gba_savestate_free, because gba_savestate_save writes into one. */\n");
    output.push_str("int32_t gba_savestate_size(uint32_t *out_size);\n");
    output.push_str(
        "int32_t gba_savestate_save(uint8_t *dst, uint32_t cap, uint32_t *out_written);\n",
    );
    output.push_str("int32_t gba_savestate_load(const uint8_t *src, uint32_t len);\n");

    output.push_str("\n/* Cartridge real-time clock (S2-b3). A pinned clock does not */\n");
    output.push_str("/* advance; enable == 0 releases the pin and restores the host. */\n");
    output.push_str("int32_t gba_rtc_set_time(int64_t unix_secs, int32_t enable);\n");
    output.push_str("int32_t gba_rtc_time(int64_t *out_unix_secs);\n");

    output.push_str("\n/* Input (S2-b4 stage 1). A set bit means held. Bit layout is  */\n");
    output.push_str("/* the core's GbaButton, in order: A B Select Start Right Left  */\n");
    output.push_str("/* Up Down R L. */\n");
    output.push_str("void     gba_set_buttons(uint16_t mask);\n");
    output.push_str("int32_t  gba_buttons(uint16_t *out_mask);\n");

    output.push_str("\n/* Cartridge battery save (S3-1). gba_battery_write refuses on a */\n");
    output.push_str("/* cartridge with no save hardware -- see plan section 7.3. */\n");
    output.push_str("int32_t  gba_battery_save_type(void);\n");
    output.push_str("int32_t  gba_set_save_type(int32_t save_type);\n");
    output.push_str("int32_t  gba_battery_size(uint32_t *out_size);\n");
    output.push_str("int32_t  gba_battery_read(uint8_t *dst, uint32_t cap);\n");
    output.push_str("int32_t  gba_battery_write(const uint8_t *src, uint32_t len);\n");
    output.push_str("int32_t  gba_battery_take_dirty(void);\n");

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
