//! NSF "no PPU" idle stub.
//!
//! When the system type is NSF (Nintendo Sound Format), the PPU is
//! disabled — only the CPU and APU run.  The PPU master clock still
//! advances so that APU timing aligns with the rest of the system, but
//! no rendering happens.
//!
//! Reference: `src/fceu.cpp` NSF branch of `FCEU_LoadNSF` +
//! `src/nsf.cpp::NSFGI::FDS` etc.  Phase 3 implements the minimal
//! stub: PPU advances dots but produces no output.

use crate::ppu::dot_clock::{DOTS_PER_SCANLINE, SCANLINES_PER_FRAME};
use crate::ppu::registers::PpuRegisters;

/// NSF-mode PPU tick.
///
/// Returns `true` when a full frame has elapsed (for the audio buffer
/// to drain).  `frame_ready` is set internally for the SoC to read.
///
/// # Why this is here
///
/// NSF players don't render anything, but they still need the PPU's
/// clock to drive APU timing (frame counter, DMC reads, etc.).  By
/// keeping the dot clock ticking with no rendering, we preserve APU
/// behavior without paying the cost of full PPU emulation.
///
/// Reference: `src/fceu.cpp::FCEUI_LoadNSF` (NSF mode skips PPU).
pub fn tick_nsf_ppu(
    ppuctrl: &mut u8,
    ppumask: &mut u8,
    status: &mut u8,
    scanline: &mut i16,
    dot: &mut u16,
    odd_frame: &mut bool,
    frame_ready: &mut bool,
) -> bool {
    // In NSF mode, all rendering registers stay at 0.
    *ppuctrl = 0;
    *ppumask = 0;
    *status = 0;
    let _ = PpuRegisters::new(); // suppress unused-import for regs
    *dot += DOTS_PER_SCANLINE;
    if *dot >= DOTS_PER_SCANLINE {
        *dot = 0;
        *scanline += 1;
        if *scanline >= SCANLINES_PER_FRAME - 1 {
            *scanline = -1;
            *odd_frame = !*odd_frame;
            *frame_ready = true;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nsf_tick_advances_scanline() {
        let mut ppuctrl = 0xFF;
        let mut ppumask = 0xFF;
        let mut status = 0xFF;
        let mut scanline = -1i16;
        let mut dot = 0u16;
        let mut odd = false;
        let mut ready = false;

        // One tick: dot overflows past scanline boundary and wraps to 0;
        // scanline advances from -1 to 0.
        let _ = tick_nsf_ppu(
            &mut ppuctrl,
            &mut ppumask,
            &mut status,
            &mut scanline,
            &mut dot,
            &mut odd,
            &mut ready,
        );
        assert_eq!(dot, 0); // wrapped to 0 after exceeding boundary
        assert_eq!(scanline, 0);

        // Second tick: scanline 0 → 1.
        let _ = tick_nsf_ppu(
            &mut ppuctrl,
            &mut ppumask,
            &mut status,
            &mut scanline,
            &mut dot,
            &mut odd,
            &mut ready,
        );
        assert_eq!(scanline, 1);

        // After 262 ticks, frame_ready should be set.
        for _ in 0..262 {
            let _ = tick_nsf_ppu(
                &mut ppuctrl,
                &mut ppumask,
                &mut status,
                &mut scanline,
                &mut dot,
                &mut odd,
                &mut ready,
            );
        }
        assert!(ready);
    }

    #[test]
    fn nsf_tick_clears_rendering_registers() {
        let mut ppuctrl = 0xFF;
        let mut ppumask = 0xFF;
        let mut status = 0xFF;
        let mut scanline = -1i16;
        let mut dot = 0u16;
        let mut odd = false;
        let mut ready = false;

        let _ = tick_nsf_ppu(
            &mut ppuctrl,
            &mut ppumask,
            &mut status,
            &mut scanline,
            &mut dot,
            &mut odd,
            &mut ready,
        );
        // After one tick, all rendering registers must be zero.
        assert_eq!(ppuctrl, 0);
        assert_eq!(ppumask, 0);
        assert_eq!(status, 0);
    }
}