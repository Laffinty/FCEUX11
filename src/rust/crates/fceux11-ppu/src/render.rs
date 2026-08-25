//! NROM BG/sprite/palette renderer (Phase 2 stub; full pipeline in Phase 4).
//!
//! Phase 2 ships a deterministic placeholder so the bridge wiring is
//! end-to-end testable. The full pipeline that produces bit-exact
//! `tests/fixtures/golden_frames/nrom.xbuf` lives here in Phase 4
//! (PPU precision suite).

use crate::state::PpuState;

/// Run the (Phase 2 placeholder) renderer on the current frame's
/// state. Writes into `framebuffer[sl*256 + dot]` for `sl in 0..240`,
/// `dot in 0..256`. The visible area is the first 240×256 bytes
/// (XBuf layout matches `src/video.h::XBuf`).
///
/// Phase 4 replaces this with the full `pputile.inc`-equivalent BG
/// fetch + sprite render + palette MUX.
pub fn render_nrom(state: &PpuState, framebuffer: &mut [u8; 256 * 256]) {
    // Phase 2 placeholder: fill with PAL[0] when rendering is on,
    // otherwise 0x00. The real pipeline lands in Phase 4.
    let rendering_on = state.rendering_enabled();
    framebuffer.fill(0x00);
    if !rendering_on {
        return;
    }
    let col: u8 = 0x00;
    let row = [col; 256];
    for sl in 0..240 {
        let off = sl * 256;
        framebuffer[off..off + 256].copy_from_slice(&row);
    }
}