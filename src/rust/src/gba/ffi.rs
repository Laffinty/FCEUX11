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

#[cfg(test)]
mod tests {
    use super::{
        ZERO_CARTRIDGE, gba_abi_revision, gba_core_probe, gba_probe_lz77_header, gba_swi_count,
        gba_swi_probe,
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

    /// The LZ77 probe answers in output bytes and rejects a corrupt header
    /// with zero, which is the only value a caller can tell apart from a
    /// one-byte result.
    #[test]
    fn lz77_probe_reports_output_length_or_rejects() {
        // Bit 31 set: the second field counts 8-byte units. 0x100 units.
        assert_eq!(
            gba_probe_lz77_header(0x8000_0100),
            0x100 * 8,
            "unit-flagged headers are scaled to bytes"
        );
        assert_eq!(
            gba_probe_lz77_header(0x200),
            0x200,
            "byte counts pass through"
        );
        assert_eq!(
            gba_probe_lz77_header(0),
            0,
            "a zero output length is corrupt"
        );
    }
}
