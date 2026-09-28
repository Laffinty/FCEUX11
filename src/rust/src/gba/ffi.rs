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
    gba_core::cartridge_header::CartridgeHeader::new(&[0u8; 0xC0]).entry_point_address()
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
    // A zeroed BIOS and an all-zero 0xC0 cartridge are enough for `Gba::new`:
    // the header parses, the entry point is the default, and we never step.
    let bios = [0u8; 0x4000];
    let cart = [0u8; 0xC0];
    let mut gba = gba_core::gba::Gba::new(bios, &cart);
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
