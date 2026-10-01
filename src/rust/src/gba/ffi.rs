//! C ABI boundary for GBAEUX11.
//!
//! ABI rules (v2.0 plan section 4.1):
//!   * every function is simulation-thread only -- in particular
//!     `gba_render_audio` must never be called from an SDL audio callback
//!     thread; the SDL side reads through the bridge in section 4.3 instead;
//!   * paths are UTF-8.
//!
//! S0' exposes the probe surface only: enough to prove the whole
//! CMake -> cargo -> staticlib -> header -> core chain, and nothing that
//! could perturb NES behaviour. The lifecycle / run / audio / savestate
//! surface in section 4.1 lands in S2.

use crate::gba::swi::Swi;

/// A cartridge image with nothing in it, for probes that only need the core
/// to construct successfully.
///
/// It has to be longer than the header: [`gba_core::cartridge_header::CartridgeHeader`]
/// slices up to `0x0E4`, and in a release build a short slice is a panic
/// rather than a `Result` -- an `extern "C"` entry point that aborts the
/// whole emulator is worse than one that returns a wrong number.
const ZERO_CARTRIDGE: [u8; 0x200] = [0; 0x200];

/// Revision of the GBAEUX11 C ABI.
///
/// Bumped whenever the declarations in this module change shape.
#[unsafe(no_mangle)]
pub extern "C" fn gba_abi_revision() -> u32 {
    0
}

/// S0 probe: the entry-point address the vendored core's cartridge-header
/// parser reports for an all-zero 0xC0-byte header. Referencing `gba-core`
/// from here is what proves the vendored core reaches the staticlib.
#[unsafe(no_mangle)]
pub extern "C" fn gba_core_probe() -> u32 {
    gba_core::cartridge_header::CartridgeHeader::new(&ZERO_CARTRIDGE).entry_point_address()
}

/// Number of SWI numbers the dispatch table recognises.
#[unsafe(no_mangle)]
pub extern "C" fn gba_swi_count() -> u32 {
    (0x00..=0x2A).filter(|n| Swi::from_raw(*n).is_some()).count() as u32
}

/// S0' probe: install the SWI hook into a fresh core and report whether it
/// took.
///
/// This is the end-to-end check for the seam. Returning 1 means the core
/// accepted our function pointer; the dispatch itself is exercised in S1,
/// when there is something to dispatch.
#[unsafe(no_mangle)]
pub extern "C" fn gba_swi_probe() -> u32 {
    // A zeroed BIOS and an empty cartridge are enough for `Gba::new`: the
    // header parses, the entry point is the default, and we never step.
    let bios = [0u8; 0x4000];
    let mut gba = gba_core::gba::Gba::new(bios, &ZERO_CARTRIDGE);
    crate::gba::install_swi_hook(&mut gba);
    u32::from(gba.cpu.swi_hook.is_some())
}

/// Decode an LZ77 header the way `Lz77UnCompWram` will. Returns the output
/// length in bytes, or 0 if the header is rejected.
#[unsafe(no_mangle)]
pub extern "C" fn gba_probe_lz77_header(raw: u32) -> u32 {
    use crate::gba::swi::decompress::{check_output_len, lz77_header};
    let h = lz77_header(raw);
    if check_output_len(h).is_err() {
        return 0;
    }
    h.output_len
}

/// S2 probe: how big a machine state actually is, in bytes.
///
/// Not a diagnostic to be removed. The savestate ABI in r31 is shaped around
/// this number — a state that is nearly a megabyte is why `gba_savestate_load`
/// writes into state we own rather than handing a value back across `extern
/// "C"`, where the caller's stack would have to hold it. Measured rather than
/// estimated, because the whole design rests on it.
#[unsafe(no_mangle)]
pub extern "C" fn gba_probe_cpu_size() -> u64 {
    std::mem::size_of::<gba_core::cpu::arm7tdmi::Arm7tdmi>() as u64
}

#[cfg(test)]
mod drift_guard {
    //! The GBA C ABI is declared by hand in `build.rs`, not generated.
    //!
    //! `build.rs` runs cbindgen against each *member* crate; the GBA ABI lives
    //! in the root crate, which cbindgen is never pointed at. So the list in
    //! `build.rs` is written out by a person, and a function added to Rust but
    //! forgotten there produces a library the C++ side cannot call — with no
    //! error anywhere. That is exactly what happened to the whole of S2-a:
    //! `gba_load_rom`, `gba_frame_buffer` and the rest compiled, tested green,
    //! and were absent from the staticlib and the header.
    //!
    //! This module is the guard. It reads the merged header that `build.rs`
    //! just wrote and asserts that every `gba_` function this crate exports is
    //! declared in it. Adding an export without declaring it fails the build.

    /// Every `gba_` function the root crate actually exports, discovered by
    /// reading the sources.
    ///
    /// This used to be a hand-written list, and that made the guard useless:
    /// it compared one hand-maintained list against a header built from
    /// another hand-maintained list, so adding an export to neither kept it
    /// green. That is the symmetric-bug shape -- two lists wrong in the same
    /// way cancel out, and any assertion that only checks self-consistency
    /// cannot see it. The whole of S2-a slipped through exactly that way:
    /// ten exports compiled, tested green, and were absent from the header.
    ///
    /// So the left side is discovered rather than written. The anchor is
    /// `#[no_mangle]` on the line above the function: a test helper or a `use`
    /// alias has no such attribute, and every real export has one.
    fn exported_names() -> Vec<String> {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    collect_from(&path, out);
                }
            }
        }

        fn collect_from(path: &std::path::Path, out: &mut Vec<String>) {
            let Ok(text) = std::fs::read_to_string(path) else {
                return;
            };
            let lines: Vec<&str> = text.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                // The attribute sits on the line before the signature.
                let anchored = index > 0 && lines[index - 1].contains("no_mangle");
                if !anchored {
                    continue;
                }
                if let Some(start) = line.find("fn gba_") {
                    let rest = &line[start + 3..];
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        out.push(name);
                    }
                }
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gba");
        let mut names = Vec::new();
        walk(&root, &mut names);
        names.sort();
        names.dedup();
        assert!(
            !names.is_empty(),
            "no gba_ exports were discovered under {} -- if the scan broke, this \
             guard would pass while checking nothing",
            root.display()
        );
        names
    }

    /// The generated header, as `build.rs` wrote it.
    fn header() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fceux11_rust.h");
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("the merged header at {path} must exist: {e}")
        })
    }

    /// Every export the sources declare is in the header the C++ side includes.
    ///
    /// The right side is the header *on disk*, which is what makes this more
    /// than a tautology: `build.rs` only rewrites it when one of its
    /// `rerun-if-changed` paths moved, so a stale header is caught here even
    /// when both sides agree on paper.
    #[test]
    fn the_gba_c_abi_is_declared_for_every_exported_function() {
        let header = header();
        let missing: Vec<String> = exported_names()
            .into_iter()
            .filter(|name| !header.contains(name.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "these are exported from Rust but not declared in fceux11_rust.h, \
             so C++ cannot call them: {missing:?}. Add them to the hand-written \
             list in build.rs. If the header is simply out of date, build.rs did \
             not re-run -- check that its rerun-if-changed list covers src/gba."
        );
    }

    /// And the error codes the functions return are named, not magic numbers.
    #[test]
    fn the_error_codes_are_named_in_the_header() {
        let header = header();
        for name in [
            "GBA_OK",
            "GBA_ERR_NO_ROM",
            "GBA_ERR_BAD_ROM",
            "GBA_ERR_STATE",
            "GBA_ERR_CAPACITY",
        ] {
            assert!(header.contains(name), "{name} is not declared in the header");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ZERO_CARTRIDGE, gba_abi_revision, gba_core_probe, gba_probe_cpu_size,
        gba_probe_lz77_header, gba_swi_count, gba_swi_probe,
    };

    /// Every symbol this module exports is called from the acceptance run, so
    /// every one of them has to survive being called. The release profile is
    /// `panic = "abort"`: a probe that panics does not return a wrong number,
    /// it takes the emulator down with it.
    #[test]
    fn every_exported_probe_answers() {
        assert_eq!(gba_abi_revision(), 0);
        // Asking the vendored header parser for the default entry point is
        // what proves the core is reachable from here -- and it is also the
        // call that used to panic on a too-short cartridge slice.
        assert_eq!(
            gba_core_probe(),
            gba_core::cartridge_header::CartridgeHeader::new(&ZERO_CARTRIDGE).entry_point_address()
        );
        assert_eq!(
            gba_swi_probe(),
            1,
            "the core must accept our hook function pointer"
        );
        assert!(
            gba_swi_count() > 0,
            "the SWI dispatch table must not be empty"
        );
    }

    /// The machine state's size, measured.
    ///
    /// Recorded because r31's ABI shape depends on it: this is why a state is
    /// never returned by value across `extern "C"`.
    ///
    /// The threshold is 64K rather than "big": the frame buffer alone is
    /// 240x160x2 = 76,800 bytes of it, which is over half the total, and the
    /// R12 caller's default 1 MB stack has to hold the state, the decoded
    /// state and serde's own frames at once. The variable-length stores
    /// (WRAM, VRAM, ROM) are heap allocations behind `Vec`, so they are *not*
    /// counted here -- r30 initially estimated the state at "close to a
    /// megabyte" and that was wrong by an order of magnitude.
    #[test]
    fn the_machine_state_is_too_large_to_return_by_value() {
        let size = gba_probe_cpu_size();
        assert!(
            size > 64 * 1024,
            "the state is {size} bytes; if that ever becomes small, r31's \
             by-value prohibition can be revisited"
        );
    }

    /// The LZ77 probe answers in output bytes and rejects a corrupt header
    /// with zero, which is the only value a caller can tell apart from a
    /// one-byte result.
    ///
    /// The headers here carry a non-zero compressed length, because a real one
    /// always does. The pre-S1b probe test used `0x8000_0100`, whose low 24
    /// bits are zero -- precisely the case that never occurs, and the reason
    /// the broken length decode survived.
    #[test]
    fn lz77_probe_reports_output_length_or_rejects() {
        // Composed from signature + size, the way a real block reads:
        // `10 00 20 00` is an LZ77 block of 0x2000 bytes. Written as a literal
        // it is easy to transpose the halves, which is how the two previous
        // versions of this test ended up certifying a wrong decode.
        let lz77_block: u32 = 0x10 | (0x2000 << 8);
        let rl_block: u32 = 0x30 | (0x1234 << 8);
        assert_eq!(
            gba_probe_lz77_header(lz77_block),
            0x2000,
            "the size is the top three bytes"
        );
        assert_eq!(
            gba_probe_lz77_header(rl_block),
            0x1234,
            "run-length blocks share the header layout"
        );
        assert_eq!(
            gba_probe_lz77_header(0x10),
            0,
            "a zero output length is corrupt"
        );
    }
}
