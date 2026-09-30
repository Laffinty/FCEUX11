//! SWI 0x0E `BgAffineSet` and SWI 0x0F `ObjAffineSet`.
//!
//! Both build the 2x2 affine matrix a rotation/scaling background or sprite
//! needs, from a scale ratio and an angle. This is the backbone of mode 7 and
//! of any scaled or rotated sprite, so a sign error here does not crash --
//! it turns the picture upside down.
//!
//! # The matrix, and the sign that GBATEK gets wrong
//!
//! ```text
//!     [ pa  pb ]     [ cos   -sin ] * [ 1/sx   0  ]
//! P = [          ]  =  [          ]   [  0   1/sy ]
//!     [ pc  pd ]     [ sin    cos ]
//! ```
//!
//! So `pa = cos/sx`, **`pb = -sin/sx`**, `pc = +sin/sy`, `pd = cos/sy`.
//!
//! GBATEK's own A-D table lists `B = Sin(alpha) / xMag` and
//! `C = Sin(alpha) / yMag` -- **both positive**, which is not a rotation
//! matrix at all. Two independent sources put the minus on `pb` where it
//! belongs: tonc (the coranac/libgba reference, in matrix form) and mGBA's
//! `_BgAffineSet`, which computes `(a,d) = cos`, `(b,c) = sin` and then
//! applies `b *= -sx`, `c *= sy`. Two different authors, same side, against a
//! prose table -- and tonc independently warns that GBATEK's element
//! descriptions lose information when translated into matrix form. Recorded as
//! r26 in the v2.0 plan.
//!
//! # Truncation, not rounding
//!
//! The results are truncated toward zero (`f32` semantics: `cosf(theta) *
//! 256.0` then `as i16` truncates). Rounding instead would put nearly every
//! angle one unit off from the hardware, and there is no way to tell from a
//! picture.
//!
//! # Layout
//!
//! `BgAffineSet` source entries are **20 bytes** -- 18 bytes of fields plus
//! **2 bytes of alignment padding** that no documentation mentions. GBATEK
//! and CowBite both describe the entry as a plain struct summing to 18
//! bytes, so a literal reading walks off the end of the array from the second
//! entry onwards. Only mGBA records the stride (`offset += 20`). This is a
//! single source, hence [`BG_SOURCE_STRIDE`] is pinned by a dedicated test.
//!
//! The 2-byte gap is the nastiest kind of mistake available here: entry 0
//! lands correctly, so anything that unpacks a single entry passes, and every
//! later entry is skewed. Nothing crashes and nothing goes out of bounds.
//!
//! Destination entries are 16 bytes: four `s16` matrix elements followed by
//! two `s32` reference points.
//!
//! `ObjAffineSet` source entries are 8 bytes (three fields plus 4 bytes of
//! padding), and its destination has no fixed stride -- `r3` gives the step
//! in bytes, 2 for contiguous and 8 to match OAM.

use core::f32::consts::PI;

/// Bytes between `BgAffineSet` source entries.
///
/// 18 bytes of fields (`s32 x`, `s32 y`, `s16 screen_x`, `s16 screen_y`,
/// `s16 scale_x`, `s16 scale_y`, `u16 angle`) plus 2 bytes of alignment
/// padding. The padding is the whole reason this constant exists.
pub const BG_SOURCE_STRIDE: usize = 20;

/// Bytes between `ObjAffineSet` source entries: three fields plus padding.
pub const OBJ_SOURCE_STRIDE: usize = 8;

/// Bytes written per `BgAffineSet` destination entry.
pub const BG_DEST_STRIDE: usize = 16;

/// `r3` value that makes `ObjAffineSet` write its four elements back to back.
pub const OBJ_CONTIGUOUS_OFFSET: u32 = 2;

/// `r3` value that makes `ObjAffineSet` write into OAM's 32-byte groups.
pub const OBJ_OAM_OFFSET: u32 = 8;

/// One `BgAffineSet` source entry, already read out of the bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgSource {
    /// Centre of the original data, 8.8 fixed point, in texture space.
    pub origin_x: i32,
    /// Centre of the original data, 8.8 fixed point, in texture space.
    pub origin_y: i32,
    /// Centre on screen, in whole pixels.
    pub screen_x: i16,
    /// Centre on screen, in whole pixels.
    pub screen_y: i16,
    /// Scale ratio in x, 8.8 fixed point.
    pub scale_x: i16,
    /// Scale ratio in y, 8.8 fixed point.
    pub scale_y: i16,
    /// Rotation angle. Only the upper 8 bits are used -- see [`angle_theta`].
    pub angle: u16,
}

/// One `BgAffineSet` destination entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgDest {
    /// Texture x-increment per pixel.
    pub pa: i16,
    /// Texture x-increment per scanline.
    pub pb: i16,
    /// Texture y-increment per pixel.
    pub pc: i16,
    /// Texture y-increment per scanline.
    pub pd: i16,
    /// Texture coordinate of screen pixel (0, 0), 8.8 fixed point.
    pub start_x: i32,
    /// Texture coordinate of screen pixel (0, 0), 8.8 fixed point.
    pub start_y: i32,
}

/// One `ObjAffineSet` source entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjSource {
    /// Scale ratio in x, 8.8 fixed point.
    pub scale_x: i16,
    /// Scale ratio in y, 8.8 fixed point.
    pub scale_y: i16,
    /// Rotation angle. Only the upper 8 bits are used.
    pub angle: u16,
}

/// The four matrix elements, shared by both calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Matrix {
    pub pa: i16,
    pub pb: i16,
    pub pc: i16,
    pub pd: i16,
}

/// Turn a 16-bit angle into radians.
///
/// **The BIOS reads only the upper 8 bits.** GBATEK: *"Rotation angles are
/// specified as 0-FFFFh (covering a range of 360 degrees), however, the GBA
/// BIOS recurses only the upper 8bit; the lower 8bit may contain a fractional
/// portion, but it is ignored by the BIOS."* mGBA writes the same arithmetic
/// (`(angle >> 8) / 128.f * M_PI`), so the full 16-bit value would give a
/// rotation up to 256x too small.
pub fn angle_theta(angle: u16) -> f32 {
    f32::from((angle >> 8) as u8) / 128.0 * PI
}

/// Build the matrix for one scale-and-rotate transformation.
///
/// Truncates toward zero at both the matrix and the reference point, matching
/// the `as i16` / `as i32` casts the hardware performs. Note the asymmetry
/// that makes this more than a sign flip: `pb` negates *before* truncating,
/// so `-0.5` truncates to `0` and not to `-1`.
pub fn matrix(scale_x: i16, scale_y: i16, angle: u16) -> Matrix {
    let theta = angle_theta(angle);
    let (sin, cos) = theta.sin_cos();
    let sx = f32::from(scale_x) / 256.0;
    let sy = f32::from(scale_y) / 256.0;
    Matrix {
        pa: (cos * sx * 256.0) as i16,
        pb: (-sin * sx * 256.0) as i16,
        pc: (sin * sy * 256.0) as i16,
        pd: (cos * sy * 256.0) as i16,
    }
}

/// Compute one `BgAffineSet` destination entry.
///
/// The reference point is the texture coordinate that lands on screen pixel
/// `(0, 0)`: the texture-space centre, pulled back through the matrix.
pub fn bg_compute(source: &BgSource) -> BgDest {
    let m = matrix(source.scale_x, source.scale_y, source.angle);
    BgDest {
        pa: m.pa,
        pb: m.pb,
        pc: m.pc,
        pd: m.pd,
        start_x: ((source.origin_x as f32) - f32::from(m.pa) * f32::from(source.screen_x)
            - f32::from(m.pb) * f32::from(source.screen_y))
            as i32,
        start_y: ((source.origin_y as f32) - f32::from(m.pc) * f32::from(source.screen_x)
            - f32::from(m.pd) * f32::from(source.screen_y))
            as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BG_DEST_STRIDE, BG_SOURCE_STRIDE, BgDest, BgSource, Matrix, OBJ_CONTIGUOUS_OFFSET,
        OBJ_OAM_OFFSET, OBJ_SOURCE_STRIDE, angle_theta, bg_compute, matrix,
    };
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    /// Assert a matrix element, naming the element so a failure says which one.
    fn assert_elem(m: &Matrix, pa: i16, pb: i16, pc: i16, pd: i16) {
        assert_eq!(
            (m.pa, m.pb, m.pc, m.pd),
            (pa, pb, pc, pd),
            "matrix is (pa, pb, pc, pd) = ({}, {}, {}, {}), expected ({pa}, {pb}, {pc}, {pd})",
            m.pa,
            m.pb,
            m.pc,
            m.pd
        );
    }

    /// Unit scale, no rotation: the identity.
    ///
    /// One fixed point unit is 1.0, so 8.8 fixed "one" is `0x0100`.
    #[test]
    fn unit_scale_and_no_angle_gives_the_identity() {
        let m = matrix(0x0100, 0x0100, 0);
        assert_elem(&m, 0x0100, 0, 0, 0x0100);
    }

    /// **The sign of `pb` is the whole point of this file.**
    ///
    /// A quarter turn (angle `0x4000`, of which the BIOS uses the top byte
    /// `0x40`) gives `cos = 0`, `sin = 1`. GBATEK's table would make `pb`
    /// positive; the correct rotation matrix has it negative while `pc` stays
    /// positive. Getting this backwards mirrors the image top to bottom.
    #[test]
    fn a_quarter_turn_puts_the_minus_on_pb_only() {
        let m = matrix(0x0100, 0x0100, 0x4000);
        assert_elem(&m, 0, -0x0100, 0x0100, 0);
    }

    /// The opposite quarter turn flips both off-diagonal terms.
    #[test]
    fn a_negative_quarter_turn_flips_both_off_diagonals() {
        let m = matrix(0x0100, 0x0100, 0xC000);
        assert_elem(&m, 0, 0x0100, -0x0100, 0);
    }

    /// A half turn is the identity with both diagonals negated; both
    /// off-diagonals land on zero, which also exercises the sign cancellation.
    #[test]
    fn a_half_turn_negates_the_diagonal() {
        let m = matrix(0x0100, 0x0100, 0x8000);
        assert_elem(&m, -0x0100, 0, 0, -0x0100);
    }

    /// An eighth turn puts equal magnitude on both off-diagonals, with `pb`
    /// negative and `pc` positive.
    #[test]
    fn an_eighth_turn_keeps_the_off_diagonals_opposite() {
        let m = matrix(0x0100, 0x0100, 0x2000);
        assert!(m.pb < 0, "pb must stay negative, got {}", m.pb);
        assert!(m.pc > 0, "pc must stay positive, got {}", m.pc);
        assert_eq!(m.pb, -m.pc, "same scale means equal magnitude");
    }

    /// Only the upper 8 bits of the angle are read.
    ///
    /// Two angles that differ only in the low byte must produce the *same*
    /// matrix -- that is what "the BIOS ignores the lower 8bit" means. Written
    /// as a property over all 256 low-byte values rather than one example,
    /// because a single example would pass just as well under the wrong
    /// implementation if the sample happened to land on a boundary.
    #[test]
    fn the_low_byte_of_the_angle_is_ignored() {
        for high in 0u16..=255 {
            let base = high << 8;
            let expected = matrix(0x0100, 0x0100, base);
            for low in 0u16..=255 {
                let m = matrix(0x0100, 0x0100, base | low);
                assert_eq!(
                    (m.pa, m.pb, m.pc, m.pd),
                    (expected.pa, expected.pb, expected.pc, expected.pd),
                    "angle {:#06x} must match {:#06x}",
                    base | low,
                    base
                );
            }
        }
    }

    /// `angle_theta` spans exactly one full turn across the 8-bit range.
    #[test]
    fn the_angle_range_spans_one_turn() {
        assert!(angle_theta(0x0000).abs() < 1e-6, "zero is zero");
        let quarter = angle_theta(0x4000);
        assert!((quarter - FRAC_PI_2).abs() < 1e-5, "{quarter} vs PI/2");
        let eighth = angle_theta(0x2000);
        assert!((eighth - FRAC_PI_4).abs() < 1e-5, "{eighth} vs PI/4");
        let three_quarters = angle_theta(0xC000);
        // 0xC0 / 128 = 1.5 turns is 270 degrees = 1.5*PI, not -PI/2. The
        // angle runs counter-clockwise from 0, so nothing here goes negative;
        // only `sin` does, which is where the sign on `pc` comes from.
        assert!((three_quarters - 1.5 * PI).abs() < 1e-5, "{three_quarters}");
        // The top of the 8-bit range is 255/256 of a turn, **not** a full
        // turn: `0xFFFF` has high byte `0xFF`, and a full turn is `0x0000`.
        // Asserting the top against PI would encode a one-eighth-turn error.
        let top = angle_theta(0xFF00);
        assert!((top - (255.0 / 256.0) * 2.0 * PI).abs() < 1e-4, "{top}");
        assert!(top < 2.0 * PI, "the angle never reaches a whole turn");
    }

    /// Scale multiplies the matrix, and x and y scale independently.
    #[test]
    fn scale_multiplies_the_matrix_independently_per_axis() {
        let half_x = matrix(0x0080, 0x0100, 0x0000);
        assert_elem(&half_x, 0x0080, 0, 0, 0x0100);
        let half_y = matrix(0x0100, 0x0080, 0x0000);
        assert_elem(&half_y, 0x0100, 0, 0, 0x0080);
        // A scale of 2.0 is 0x0200.
        let double = matrix(0x0200, 0x0200, 0x0000);
        assert_elem(&double, 0x0200, 0, 0, 0x0200);
    }

    /// Truncation goes toward zero, not toward negative infinity.
    ///
    /// A negative `pb` that lands between -1 and 0 becomes 0, not -1. The
    /// hardware's cast truncates the same way, and rounding instead would put
    /// most angles a unit off with no visible way to tell.
    #[test]
    fn results_truncate_toward_zero() {
        // 45 degrees: cos = sin = 1/sqrt(2) = 0.7071, times 256 = 181.0.
        // 181.019 rounds to 181 and truncates to 181, so use a scale that
        // lands strictly between two integers instead: 0.5 * 0.7071 * 256
        // = 90.5, which rounds to 91 and truncates to 90.
        let m = matrix(0x0080, 0x0080, 0x2000);
        assert_eq!(m.pa, 90, "0.5 * cos45 * 256 = 90.5, truncated");
        // pb is negated before truncation: -90.5 truncates to -90, not -91.
        assert_eq!(m.pb, -90, "the negation happens before the truncation");
        assert_eq!(m.pc, 90);
        assert_eq!(m.pd, 90);
    }

    /// With the texture and screen centres aligned, the reference point is
    /// the texture centre itself.
    #[test]
    fn an_aligned_centre_leaves_the_reference_point_where_it_started() {
        let source = BgSource {
            origin_x: 0x1000,
            origin_y: 0x0800,
            screen_x: 0,
            screen_y: 0,
            scale_x: 0x0100,
            scale_y: 0x0100,
            angle: 0x2000,
        };
        let out = bg_compute(&source);
        assert_eq!(out.start_x, 0x1000);
        assert_eq!(out.start_y, 0x0800);
        assert_elem(
            &Matrix {
                pa: out.pa,
                pb: out.pb,
                pc: out.pc,
                pd: out.pd,
            },
            181,
            -181,
            181,
            181,
        );
    }

    /// The reference point is pulled back through the matrix by the screen
    /// offset, which is the whole reason `BgAffineSet` exists.
    ///
    /// `screen_x`/`screen_y` count whole pixels, and the matrix is 8.8 fixed
    /// point, so the correction is `screen * 0x0100` per pixel.
    #[test]
    fn the_reference_point_follows_the_screen_offset() {
        // No rotation, unit scale: the matrix is the identity, so the
        // reference point is the texture centre minus the screen offset.
        let source = BgSource {
            origin_x: 0x2000,
            origin_y: 0x2000,
            screen_x: 16,
            screen_y: 32,
            scale_x: 0x0100,
            scale_y: 0x0100,
            angle: 0x0000,
        };
        let out = bg_compute(&source);
        // 0x2000 = 32.0; 32.0 - 16.0 = 16.0 = 0x1000, 32.0 - 32.0 = 0.
        assert_eq!(out.start_x, 0x1000);
        assert_eq!(out.start_y, 0);
    }

    /// A rotated matrix moves the reference point along *both* axes.
    ///
    /// `screen_x`/`screen_y` are whole pixels, but the reference point is in
    /// 8.8 fixed point, so the correction is `screen * pa` without any
    /// fractional part -- 16 * 181 = 2896, not 11.
    #[test]
    fn a_rotation_off_centre_moves_both_reference_coordinates() {
        let centred = bg_compute(&BgSource {
            origin_x: 0,
            origin_y: 0,
            screen_x: 0,
            screen_y: 0,
            scale_x: 0x0100,
            scale_y: 0x0100,
            angle: 0x2000,
        });
        let offset = bg_compute(&BgSource {
            origin_x: 0,
            origin_y: 0,
            screen_x: 16,
            screen_y: 0,
            scale_x: 0x0100,
            scale_y: 0x0100,
            angle: 0x2000,
        });
        // Under rotation the x offset bleeds into y through `pc`.
        assert_ne!(offset.start_x, centred.start_x);
        assert_ne!(offset.start_y, centred.start_y);
        // 16 * 181 = 2896, and the reference point moves the other way.
        assert_eq!(offset.start_x, -2896);
        assert_eq!(offset.start_y, -2896);
    }

    /// The source stride is 20 bytes, not the 18 the field list adds up to.
    ///
    /// Pinned as a literal rather than derived from [`BG_SOURCE_STRIDE`], on
    /// purpose: this constant has exactly one source (mGBA) and every
    /// documentation that describes the entry as a struct implies 18. A test
    /// comparing the constant to itself would pass under either answer.
    ///
    /// The 2-byte gap is **alignment padding**, and getting it wrong is the
    /// worst kind of bug here: entry 0 lands correctly, so a single-entry test
    /// passes, and every later entry is skewed by two bytes. Nothing crashes.
    #[test]
    fn the_bg_source_entry_is_twenty_bytes_with_two_bytes_of_alignment() {
        assert_eq!(BG_SOURCE_STRIDE, 20);
        // Two s32 centres...
        let centres = 4 + 4;
        // ...and five halfwords: screen_x, screen_y, scale_x, scale_y, angle.
        let halfwords = 2 + 2 + 2 + 2 + 2;
        assert_eq!(centres + halfwords, 18, "the documented fields are 18 bytes");
        // The stride is 2 more than that, to reach a 4-byte boundary.
        assert_eq!(
            BG_SOURCE_STRIDE - (centres + halfwords),
            2,
            "the padding aligns each entry to a word"
        );
    }

    /// The strides this module walks with.
    #[test]
    fn the_other_strides_match_the_documented_layouts() {
        assert_eq!(BG_DEST_STRIDE, 16, "4 s16 + 2 s32");
        assert_eq!(OBJ_SOURCE_STRIDE, 8, "3 fields + 4 bytes padding");
        assert_eq!(OBJ_CONTIGUOUS_OFFSET, 2);
        assert_eq!(OBJ_OAM_OFFSET, 8);
    }

    /// A destination entry is four halfwords then two words -- nothing more.
    #[test]
    fn a_bg_destination_entry_is_its_six_fields() {
        let out = bg_compute(&BgSource {
            origin_x: 0x1234,
            origin_y: 0x5678,
            screen_x: 1,
            screen_y: 2,
            scale_x: 0x0100,
            scale_y: 0x0100,
            angle: 0x0000,
        });
        let size = core::mem::size_of::<BgDest>();
        assert_eq!(size, BG_DEST_STRIDE);
        let _ = out;
    }
}
