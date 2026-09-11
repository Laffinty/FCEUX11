//! Region / video-system timing table - FCEUX11 v2.1.1.7 Step B.5-2a.
//!
//! The Rust PPU has been NTSC-only: scanline count, VBL/NMI lines, the
//! frame budget and the CPU:PPU ratio were hard-coded. This module
//! introduces the data model that the PAL/Dendy work consumes -
//! Step B.5-2a (this batch) is **data only**: nothing here changes NTSC
//! behaviour; the state machine keeps using its existing constants.
//!
//! Values come from the cross-verified table in
//! `docs/plans/v2.1.1.7_cpp_ppu_removal.md` section B.5 D4.1
//! (primary source: Mesen2 `Core/NES/NesPpu.cpp::UpdateTimings`,
//! `Core/NES/NesCpu.cpp:119-136`, `Core/NES/NesConstants.h`; secondary:
//! Nestopia UE `source/core/NstPpu.cpp:551-670`):
//!
//! | system | scanlines | VBL set | VBL end | dots/frame | dots per CPU cycle |
//! |--------|-----------|---------|---------|------------|--------------------|
//! | NTSC   | 262       | 241     | 260     | 89342      | 3.0 (12:4)         |
//! | PAL    | 312       | 241     | 310     | 106392     | 3.2 (16:5)         |
//! | Dendy  | 312       | 291     | 310     | 106392     | 3.0 (15:5)         |
//!
//! Batch map: B.5-2a model (here) -> B.5-2b raster/VBL wiring ->
//! B.5-2c CPU:PPU interleave ratio -> B.5-2d remaining region details
//! (odd-frame dot skip, emphasis bit swap, sprite-eval window).

/// Which PPU/CPU timing set the emulated console uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoSystem {
    #[default]
    Ntsc,
    Pal,
    Dendy,
}

impl VideoSystem {
    /// Map the legacy boolean (`false` = NTSC, `true` = PAL) onto the
    /// model. Dendy has no boolean spelling; the C ABI gains one in
    /// Step B.5-2b.
    pub const fn from_pal_flag(pal: bool) -> Self {
        if pal { Self::Pal } else { Self::Ntsc }
    }

    /// `true` for the 50 Hz systems (PAL and Dendy).
    pub const fn is_50hz(self) -> bool {
        matches!(self, Self::Pal | Self::Dendy)
    }

    /// One row of the region timing table (see the module docs for sources).
    pub const fn timings(self) -> VideoSystemTimings {
        match self {
            Self::Ntsc => VideoSystemTimings {
                scanlines: 262,
                vbl_set_scanline: 241,
                vbl_end_scanline: 260,
                pre_render_scanline: -1,
                visible_scanlines: 240,
                dots_per_scanline: 341,
                dots_per_frame: 89342,
                ppu_dots_per_cpu_cycle_num: 3,
                ppu_dots_per_cpu_cycle_den: 1,
                odd_frame_skips_dot: true,
            },
            Self::Pal => VideoSystemTimings {
                scanlines: 312,
                vbl_set_scanline: 241,
                vbl_end_scanline: 310,
                pre_render_scanline: -1,
                visible_scanlines: 240,
                dots_per_scanline: 341,
                dots_per_frame: 106392,
                ppu_dots_per_cpu_cycle_num: 16,
                ppu_dots_per_cpu_cycle_den: 5,
                odd_frame_skips_dot: false,
            },
            Self::Dendy => VideoSystemTimings {
                scanlines: 312,
                vbl_set_scanline: 291,
                vbl_end_scanline: 310,
                pre_render_scanline: -1,
                visible_scanlines: 240,
                dots_per_scanline: 341,
                dots_per_frame: 106392,
                ppu_dots_per_cpu_cycle_num: 3,
                ppu_dots_per_cpu_cycle_den: 1,
                odd_frame_skips_dot: false,
            },
        }
    }
}

/// One row of the region timing table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoSystemTimings {
    /// Scanlines per frame (262 / 312 / 312).
    pub scanlines: i16,
    /// Scanline on which the VBL flag is set and NMI is asserted
    /// (NTSC/PAL 241, Dendy 291).
    pub vbl_set_scanline: i16,
    /// Last scanline of the VBL block; `vbl_end_scanline + 1` is the
    /// hardware pre-render line.
    pub vbl_end_scanline: i16,
    /// Index the state machine uses for the pre-render line (hardware
    /// `vbl_end_scanline + 1`). All three systems alias it to -1.
    pub pre_render_scanline: i16,
    /// Visible scanlines (240 on all three systems).
    pub visible_scanlines: i16,
    /// PPU dots per scanline (341 everywhere).
    pub dots_per_scanline: u16,
    /// PPU dots per frame - the frame budget the C++ bridge passes in.
    pub dots_per_frame: u32,
    /// CPU:PPU ratio as an exact fraction: `num` PPU dots per `den`
    /// CPU cycles (NTSC 3/1, PAL 16/5, Dendy 3/1).
    pub ppu_dots_per_cpu_cycle_num: u32,
    /// Denominator of [`Self::ppu_dots_per_cpu_cycle_num`].
    pub ppu_dots_per_cpu_cycle_den: u32,
    /// `true` only on NTSC: with rendering enabled, odd frames are one
    /// PPU clock shorter ("This behavior is NTSC-specific - PAL frames
    /// are always the same number of cycles" - Mesen2).
    pub odd_frame_skips_dot: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntsc_row_matches_shipped_constants() {
        let t = VideoSystem::Ntsc.timings();
        assert_eq!(t.scanlines, 262);
        assert_eq!(t.vbl_set_scanline, 241);
        assert_eq!(t.vbl_end_scanline, 260);
        assert_eq!(t.pre_render_scanline, -1);
        assert_eq!(t.dots_per_frame, 89342);
        assert_eq!((t.ppu_dots_per_cpu_cycle_num, t.ppu_dots_per_cpu_cycle_den), (3, 1));
        assert!(t.odd_frame_skips_dot);
        assert!(!VideoSystem::Ntsc.is_50hz());
    }

    #[test]
    fn pal_row_matches_mesen2() {
        let t = VideoSystem::Pal.timings();
        assert_eq!(t.scanlines, 312);
        assert_eq!(t.vbl_set_scanline, 241);
        assert_eq!(t.vbl_end_scanline, 310);
        assert_eq!(t.dots_per_frame, 106392);
        assert_eq!((t.ppu_dots_per_cpu_cycle_num, t.ppu_dots_per_cpu_cycle_den), (16, 5));
        assert!(!t.odd_frame_skips_dot);
        assert!(VideoSystem::Pal.is_50hz());
    }

    #[test]
    fn dendy_row_matches_mesen2() {
        let t = VideoSystem::Dendy.timings();
        assert_eq!(t.scanlines, 312);
        assert_eq!(t.vbl_set_scanline, 291, "Dendy sets VBL/NMI 50 lines later");
        assert_eq!(t.vbl_end_scanline, 310);
        assert_eq!(t.dots_per_frame, 106392);
        assert_eq!((t.ppu_dots_per_cpu_cycle_num, t.ppu_dots_per_cpu_cycle_den), (3, 1));
        assert!(!t.odd_frame_skips_dot);
        assert!(VideoSystem::Dendy.is_50hz());
    }

    #[test]
    fn dots_per_frame_is_scanlines_times_dots_per_scanline() {
        for sys in [VideoSystem::Ntsc, VideoSystem::Pal, VideoSystem::Dendy] {
            let t = sys.timings();
            assert_eq!(
                t.dots_per_frame,
                t.scanlines as u32 * t.dots_per_scanline as u32,
                "{sys:?} frame budget must equal scanlines * dots/scanline"
            );
        }
    }

    #[test]
    fn only_pal_uses_the_3_2_ratio() {
        for sys in [VideoSystem::Ntsc, VideoSystem::Pal, VideoSystem::Dendy] {
            let t = sys.timings();
            let is_3_2 = t.ppu_dots_per_cpu_cycle_num * 5 == t.ppu_dots_per_cpu_cycle_den * 16;
            assert_eq!(is_3_2, sys == VideoSystem::Pal, "{sys:?}");
        }
    }

    #[test]
    fn pal_flag_maps_to_ntsc_or_pal() {
        assert_eq!(VideoSystem::from_pal_flag(false), VideoSystem::Ntsc);
        assert_eq!(VideoSystem::from_pal_flag(true), VideoSystem::Pal);
    }
}