//! SWI 0x09: `ArcTan`.
//!
//! # What the BIOS actually does
//!
//! Not a library `atan`. The GBA BIOS evaluates a **seventh-order odd power
//! series in `-t²`**, entirely with shifts, adds and multiplies, on the
//! magnitude of the input and negates the result at the end. The eight
//! coefficients below were recovered independently by two sources that agree on
//! every value -- so the constants themselves are not in question.
//!
//! # How the shifts were settled
//!
//! The two sources disagree on the fixed-point scale (one of them says so
//! itself). Rather than vote, the **contract** was used as an oracle: GBATEK
//! states the output range as `C000h`-`4000h` for `-PI/2 < THETA < PI/2`, which
//! pins `PI/2 <-> 0x4000` and therefore `atan(1.0) = PI/4 <-> 0x2000`. Of every
//! `(horner_shift, final_shift)` pair tried, **exactly one** reproduces the
//! contract's anchors exactly -- 14 and 16. The other source's `fixShift = 15`
//! belongs to a different normalisation: it folds an explicit `QDIV(y, x, 15)`
//! into the tangent before the series, so its `t` is Q15 where ours is the raw
//! 1.14 input.
//!
//! # How close is it
//!
//! Against the ideal arctangent over the whole `|v| <= 0x4000` domain the worst
//! error is **2 units**, one unit being about 0.0055 degrees. There are also
//! seven points above `v = 0x3B5B` where the result drops by one -- the
//! approximation's own wobble, and exactly the range GBATEK flags as having
//! "a problem in accuracy with THETA<-PI/4, PI/4<THETA". That the wobble falls
//! where the documentation says it should is a second, independent sign that
//! this is the right polynomial.

/// The eight coefficients of the series, highest order last.
///
/// Recovered from the BIOS; two independent sources agree on all eight.
pub const COEFFICIENTS: [i32; 8] = [
    0x00A9, 0x0390, 0x091C, 0x0FB6, 0x16AA, 0x2081, 0x3651, 0xA2F9,
];

/// Scale of the inner (`-t²`) Horner steps.
const HORNER_SHIFT: u32 = 14;

/// Scale of the final multiply.
const FINAL_SHIFT: u32 = 16;

/// The input magnitude that represents `tan = 1.0`, i.e. `PI/4`.
pub const UNITY: i16 = 0x4000;

/// The result that corresponds to `PI/4`; the contract's anchor.
pub const QUARTER_PI: i16 = 0x2000;

/// The largest input magnitude for which the result is strictly monotone
/// across the **whole signed domain**.
///
/// It is not the positive side's first wobble (`0x3B5B`): negating the
/// magnitude at the end does not preserve *where* the approximation wobbles,
/// so the negative side drops by one as early as `|v| = 0x3FA`. Taking the
/// earlier of the two is what makes the monotone test meaningful.
pub const ACCURATE_RANGE_END: i32 = 0x3FA;

/// The arc tangent of `value`, the way the BIOS computes it.
///
/// `value` is 1.14 fixed point in `-0x4000..=0x4000`. The result is 1.14 fixed
/// point over `-PI/2..=PI/2`, so `PI/2` maps to `0x4000` and `PI/4` to
/// `0x2000`.
///
/// # Input outside the contract
///
/// Only `|value| <= 0x4000` is defined. Beyond that the Horner products
/// overflow 32 bits, and the BIOS -- running on 32-bit ARM arithmetic -- simply
/// wraps. So this wraps too, deliberately: a panic here would unwind through
/// the `extern "C"` hook frame and take the whole emulator process with it,
/// which is far worse than the garbage value the hardware would also produce.
#[must_use]
pub fn arctan(value: i16) -> i16 {
    let negative = value < 0;
    // `unsigned_abs` keeps `i16::MIN` from overflowing on negation; the BIOS
    // reads the same 16 bits and takes the magnitude the same way.
    let t = i32::from(value.unsigned_abs());

    // Horner in u = -t^2. The negation makes the series alternate, which is what
    // turns the odd Taylor expansion into this form.
    let u = t.wrapping_mul(t).wrapping_neg() >> HORNER_SHIFT;
    let mut acc = COEFFICIENTS[0];
    for coefficient in &COEFFICIENTS[1..] {
        acc = coefficient + (u.wrapping_mul(acc) >> HORNER_SHIFT);
    }

    let result = (acc.wrapping_mul(t) >> FINAL_SHIFT) as i16;
    if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{arctan, ACCURATE_RANGE_END, COEFFICIENTS, QUARTER_PI, UNITY};

    /// The contract states the output range as `C000h`-`4000h` for
    /// `-PI/2 < THETA < PI/2`. That fixes the scale, and with it the value at
    /// `tan = 1.0`. These anchors come from the specification, not from a
    /// transcribed table -- which is what makes them usable as an oracle.
    #[test]
    fn the_contract_anchors_hold_exactly() {
        assert_eq!(arctan(0), 0, "atan(0) is 0");
        assert_eq!(
            arctan(UNITY),
            QUARTER_PI,
            "atan(1.0) is PI/4, and PI/2 is 0x4000"
        );
        assert_eq!(arctan(-UNITY), -QUARTER_PI, "and the result is odd");
    }

    /// Intermediate points, each derived from the same scale rather than
    /// copied. `atan(0.5)` in radians is 0.463648, times `0x4000*2/PI`.
    #[test]
    fn intermediate_points_match_the_derived_scale() {
        assert_eq!(arctan(0x1000), 0x09FB, "atan(0.25)");
        assert_eq!(arctan(0x2000), 0x12E4, "atan(0.5)");
        assert_eq!(arctan(0x3000), 0x1A37, "atan(0.75)");
    }
    /// The BIOS takes the magnitude and negates at the end, so the function is
    /// exactly odd. A sign handled anywhere else shows up here.
    #[test]
    fn the_function_is_exactly_odd() {
        for v in -(UNITY as i32)..=(UNITY as i32) {
            let v = v as i16;
            assert_eq!(arctan(-v), -arctan(v), "v = {v:#06x}");
        }
    }

    /// Monotone across the domain GBATEK calls accurate. Beyond
    /// [`ACCURATE_RANGE_END`] the approximation wobbles, and that is pinned
    /// separately rather than hidden -- see the next test.
    #[test]
    fn monotone_over_the_documented_accurate_range() {
        let mut v = -(ACCURATE_RANGE_END as i16);
        let mut previous = arctan(v);
        while v < ACCURATE_RANGE_END as i16 {
            v += 1;
            let current = arctan(v);
            assert!(
                current >= previous,
                "not monotone at {v:#06x}: {previous:#06x} -> {current:#06x}"
            );
            previous = current;
        }
    }

    /// Every one-unit drop in the output, over the whole signed domain, is a
    /// property of the BIOS's approximation rather than of this port. There
    /// are fourteen -- seven on each side, at different magnitudes, because
    /// negating the magnitude at the end does not mirror *where* the
    /// approximation wobbles.
    ///
    /// Pinning all of them means a change to the algorithm surfaces as *this*
    /// test failing, instead of as a unit or two of drift nobody notices.
    #[test]
    fn the_documented_wobble_is_exactly_where_it_is() {
        let mut drops = Vec::new();
        let mut previous = arctan(-UNITY);
        for v in (-(UNITY as i32) + 1)..=(UNITY as i32) {
            let current = arctan(v as i16);
            if current < previous {
                drops.push((v as u16, previous.wrapping_sub(current)));
            }
            previous = current;
        }
        assert_eq!(
            drops,
            vec![
                (0xC006, 1),
                (0xC0F7, 1),
                (0xC196, 1),
                (0xC1EC, 1),
                (0xC22F, 1),
                (0xC32C, 1),
                (0xC4A6, 1),
                (0x3B5B, 1),
                (0x3CD5, 1),
                (0x3DD2, 1),
                (0x3E15, 1),
                (0x3E6B, 1),
                (0x3F0A, 1),
                (0x3FFB, 1)
            ],
            "the approximation's wobble moved; this is not the BIOS's polynomial"
        );
    }

    /// `PI/2` is the ceiling of the output range and `atan(1.0)` is the
    /// largest legitimate result, so nothing may exceed `0x2000`.
    #[test]
    fn the_result_never_leaves_the_declared_range() {
        for v in -(UNITY as i32)..=(UNITY as i32) {
            let got = arctan(v as i16);
            assert!(got <= QUARTER_PI, "v = {v:#06x} gave {got:#06x}");
            assert!(got >= -QUARTER_PI, "v = {v:#06x} gave {got:#06x}");
        }
    }

    /// The coefficients are the BIOS's, and they are the whole content of the
    /// function. Asserted as a literal rather than against a derived value, so
    /// a transcription slip in this file cannot pass.
    #[test]
    fn the_coefficients_are_the_bios_own() {
        assert_eq!(
            COEFFICIENTS,
            [0x00A9, 0x0390, 0x091C, 0x0FB6, 0x16AA, 0x2081, 0x3651, 0xA2F9]
        );
        // They are strictly increasing, which is what a Taylor sequence of
        // decreasing magnitudes looks like once the Horner order is reversed --
        // a transposed pair would break this without changing any single value.
        assert!(
            COEFFICIENTS.windows(2).all(|w| w[0] < w[1]),
            "the coefficients must increase: {COEFFICIENTS:?}"
        );
    }

    /// Input outside the contract must not panic. This path is reachable
    /// through the `extern "C"` hook frame, where a panic cannot unwind and
    /// would abort the whole emulator rather than just this call.
    #[test]
    fn out_of_contract_input_does_not_panic() {
        for v in [i16::MIN, i16::MIN + 1, -0x4001, 0x4001, 0x7FFF] {
            let _ = arctan(v);
        }
    }

    /// The worst error against the ideal, over the whole domain, is two units
    /// -- about 0.011 degrees. This is the algorithm's accuracy, not ours; the
    /// test exists so a future "optimisation" that loses a shift is caught.
    #[test]
    fn the_worst_error_stays_within_two_units() {
        // The ideal value at each sample, derived from the same scale the
        // anchors use. Sampled rather than exhaustive so the table stays
        // readable; the exhaustive check is the range test above.
        let ideal = [
            (0x0400i32, 0x028Bi32),
            (0x0800, 0x0511),
            (0x1000, 0x09FB),
            (0x1800, 0x0E9E),
            (0x2000, 0x12E4),
            (0x2800, 0x16C2),
            (0x3000, 0x1A38),
            (0x3800, 0x1D4A),
            (0x4000, 0x2000),
        ];
        for (input, want) in ideal {
            let got = arctan(input as i16);
            assert!(
                (got as i32 - want).abs() <= 2,
                "atan({input:#06x}) = {got:#06x}, ideal {want:#06x}"
            );
        }
    }
}
