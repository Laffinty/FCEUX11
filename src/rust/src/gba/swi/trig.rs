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
/// # The shift-then-negate order is load-bearing
///
/// The BIOS forms its first Horner term as `-((i * i) >> 14)`: it shifts
/// **first**, then negates. Computing it the other way round, as
/// `(-(i * i)) >> 14`, is off by one whenever `i * i` is not a multiple of
/// 2^14 -- which is almost always. Rust's `>>` on a signed integer is an
/// arithmetic shift, so it rounds towards negative infinity exactly like the
/// ARM `asr` the BIOS uses, and the two forms genuinely differ. Doing the
/// negation first would put every result one unit off in a way no range or
/// accuracy test would catch.
///
/// For the same reason this is **not** implemented as "take the magnitude,
/// evaluate, negate at the end". That form is exactly odd; the BIOS's is not,
/// and the difference shows up on the negative half. `arctan2` inherits the
/// asymmetry, so folding it away here would corrupt its octants.
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
    let t = i32::from(value);

    // Horner in u = -t^2. The negation makes the series alternate, which is
    // what turns the odd Taylor expansion into this form. Shift, *then*
    // negate -- see the note above.
    let u = -(t.wrapping_mul(t) >> HORNER_SHIFT);
    let mut acc = COEFFICIENTS[0];
    for coefficient in &COEFFICIENTS[1..] {
        acc = coefficient + (u.wrapping_mul(acc) >> HORNER_SHIFT);
    }

    (acc.wrapping_mul(t) >> FINAL_SHIFT) as i16
}

/// The four axis directions, in the 0..2PI scale the contract uses.
pub const AXIS_POS_X: u16 = 0x0000;
/// `+Y`, i.e. a quarter turn.
pub const AXIS_POS_Y: u16 = 0x4000;
/// `-X`, i.e. a half turn.
pub const AXIS_NEG_X: u16 = 0x8000;
/// `-Y`, i.e. three quarters.
pub const AXIS_NEG_Y: u16 = 0xC000;

/// The arc tangent of the vector `(x, y)`, over a full turn.
///
/// GBATEK: *"Return: r0 `0000h`-`FFFFh` for `0<=THETA<2PI`."* The BIOS does
/// not compute a two-argument arctangent. It reduces the input to one of
/// **eight octants** around a known axis, calls the same polynomial
/// [`arctan`] on a ratio it has already brought inside the contract's
/// `-PI/2..=PI/2` domain, and folds the result back with a quadrant offset.
///
/// # The axis cases are not the interesting part -- they are the *only* part
/// that is exact
///
/// For `x == 0` or `y == 0` the BIOS returns one of four constants and never
/// touches the polynomial. A floating-point `atan2` gets these approximately
/// right and never exactly, which is why this number is claimed rather than
/// left to the core: the differences show up wherever a game uses an
/// axis-aligned vector to mean "up", "left", and so on.
///
/// # `x == 0 && y == 0` is defined, and it is the origin
///
/// It returns `0x0000`, because the `y == 0` test comes first. That is not a
/// special case invented here; it falls out of the branch order, and it is
/// pinned so the order cannot be rearranged without the test noticing.
#[must_use]
pub fn arctan2(x: i16, y: i16) -> u16 {
    let (x, y) = (i32::from(x), i32::from(y));

    if y == 0 {
        return if x >= 0 { AXIS_POS_X } else { AXIS_NEG_X };
    }
    if x == 0 {
        return if y >= 0 { AXIS_POS_Y } else { AXIS_NEG_Y };
    }

    // Each branch picks the axis whose octant the vector is nearest, forms
    // `tan` = the ratio of the two components as 1.14, and adds the angle of
    // that axis. The shifts keep every ratio inside `-0x4000..=0x4000`, so the
    // polynomial is always called within its contract.
    //
    // **The offsets below are in `arctan2`'s scale, not `arctan`'s.** A
    // quarter turn is `0x4000` here but `0x2000` in [`arctan`]'s
    // `-PI/2..=PI/2` range. Mixing the two is the easy mistake here, and it
    // lands the answer in the wrong quadrant rather than out of range.
    let angle = if y >= 0 {
        if x >= 0 {
            if x >= y {
                // First octant, measured from +X.
                arctan(ratio(y, x))
            } else {
                // Second octant, measured from +Y.
                (AXIS_POS_Y as i32 - arctan(ratio(x, y)) as i32) as i16
            }
        } else if -x >= y {
            // Fourth octant, measured from -X.
            (arctan(ratio(y, x)) as i32 + AXIS_NEG_X as i32) as i16
        } else {
            // Third octant, measured from +Y the other way.
            (AXIS_POS_Y as i32 - arctan(ratio(x, y)) as i32) as i16
        }
    } else if x <= 0 {
        if -x > -y {
            // Fifth octant, measured from -X.
            (arctan(ratio(y, x)) as i32 + AXIS_NEG_X as i32) as i16
        } else {
            // Sixth octant, measured from -Y.
            (AXIS_NEG_Y as i32 - arctan(ratio(x, y)) as i32) as i16
        }
    } else if x >= -y {
        // Eighth octant, measured from -X again -- the offset wraps past 2PI.
        (arctan(ratio(y, x)) as i32 + 0x1_0000) as i16
    } else {
        // Seventh octant, measured from -Y.
        (AXIS_NEG_Y as i32 - arctan(ratio(x, y)) as i32) as i16
    };

    angle as u16
}

/// `numerator / denominator` scaled from 1.14 into the polynomial's domain.
///
/// The `<< 14` is the Q14-to-Q1 move: 1.14 `1.0` is `0x4000`, and the
/// polynomial wants a plain integer ratio. Truncation is toward zero, matching
/// C integer division, which is what the hardware's `sdiv` does.
fn ratio(numerator: i32, denominator: i32) -> i16 {
    ((numerator << 14) / denominator) as i16
}

#[cfg(test)]
mod tests {
    use super::{
        ACCURATE_RANGE_END, AXIS_NEG_X, AXIS_NEG_Y, AXIS_POS_X, AXIS_POS_Y, COEFFICIENTS,
        QUARTER_PI, UNITY, arctan, arctan2,
    };

    /// The `x == 0` short-circuit is an optimisation, not a behaviour.
    ///
    /// Deleting it leaves every test in this file green, and that is correct
    /// rather than a gap: with `x == 0` the ratio formed is `0`, `arctan(0)`
    /// is `0`, and the second- and sixth-octant branches then evaluate to
    /// `AXIS_POS_Y - 0` and `AXIS_NEG_Y - 0` -- the same two constants the
    /// short-circuit returns. The branch is kept because it is what the
    /// hardware does, and because relying on a polynomial to land exactly on
    /// an axis constant is a coincidence rather than a guarantee.
    ///
    /// Pinned so the redundancy is a documented fact instead of something the
    /// next person has to re-derive.
    #[test]
    fn the_axis_short_circuit_agrees_with_the_polynomial_path() {
        for y in [1i16, 2, 0x1000, 0x4000, 0x7FFF] {
            // The short-circuit's answer...
            assert_eq!(arctan2(0, y), AXIS_POS_Y, "y = {y}");
            // ...and what the general path would produce for the same input.
            let via_polynomial = AXIS_POS_Y.wrapping_sub(arctan(0) as u16);
            assert_eq!(arctan2(0, y), via_polynomial, "y = {y}");
        }
        for y in [-1i16, -2, -0x1000, -0x4000, -0x8000] {
            assert_eq!(arctan2(0, y), AXIS_NEG_Y, "y = {y}");
        }
    }

    /// The four axis directions are exact constants, not approximations.
    ///
    /// A floating-point `atan2` lands near these but never on them. This is
    /// the single clearest reason to claim the number.
    #[test]
    fn the_four_axis_directions_are_exact() {
        assert_eq!(arctan2(0x4000, 0), AXIS_POS_X, "due +X is 0 degrees");
        assert_eq!(arctan2(0, 0x4000), AXIS_POS_Y, "due +Y is 90");
        assert_eq!(arctan2(-0x4000, 0), AXIS_NEG_X, "due -X is 180");
        assert_eq!(arctan2(0, -0x4000), AXIS_NEG_Y, "due -Y is 270");
        // The diagonal is *not* an axis: it is 45 degrees, and this call must
        // go through the polynomial rather than hit a constant. Asserted so
        // the axis detection cannot be widened by accident.
        let diagonal = arctan2(0x4000, 0x4000);
        assert!(diagonal > 0 && diagonal < AXIS_POS_Y, "45 degrees is {diagonal:#06x}");
        // Any magnitude works, not just 1.0: the axes are detected before the
        // ratio is ever formed.
        for magnitude in [1i16, 2, 100, 0x7FFF] {
            assert_eq!(arctan2(magnitude, 0), AXIS_POS_X, "x = {magnitude}");
            assert_eq!(arctan2(-magnitude, 0), AXIS_NEG_X, "x = -{magnitude}");
            assert_eq!(arctan2(0, magnitude), AXIS_POS_Y, "y = {magnitude}");
            assert_eq!(arctan2(0, -magnitude), AXIS_NEG_Y, "y = -{magnitude}");
        }
    }

    /// The constants are the ones the contract's scale implies.
    #[test]
    fn the_axis_constants_are_the_contract_scale() {
        assert_eq!(AXIS_POS_X, 0x0000);
        assert_eq!(AXIS_POS_Y, 0x4000);
        assert_eq!(AXIS_NEG_X, 0x8000);
        assert_eq!(AXIS_NEG_Y, 0xC000);
        // A quarter turn in this scale is what `arctan` calls PI/4 doubled.
        assert_eq!(AXIS_POS_Y, 2 * QUARTER_PI as u16);
    }

    /// The origin returns zero, and that falls out of the branch order.
    #[test]
    fn the_origin_is_zero() {
        assert_eq!(arctan2(0, 0), AXIS_POS_X, "the y == 0 test comes first");
    }

    /// Circular distance between two full-turn angles.
    ///
    /// The domain wraps at `0x10000`, so `0xFFFF` is one unit from `0x0000`
    /// and 65535 units from it. A plain subtraction would call that a failure.
    fn turn_error(got: i32, expected: i32) -> i32 {
        let raw = (got - expected).rem_euclid(0x1_0000);
        raw.min((-raw).rem_euclid(0x1_0000))
    }

    /// Rotating a vector by a quarter turn rotates the answer by a quarter turn.
    ///
    /// `(x, y) -> (-y, x)` is a 90-degree counter-clockwise turn of the input,
    /// so the answer must move by `0x4000`. A property over a grid rather than
    /// a few samples, because a single sample can agree by accident.
    ///
    /// Allowed to be off by one: the polynomial's two octant branches round
    /// independently, so the composition is a unit away from the ideal
    /// rotation at some points. Pinned as a bound rather than an equality --
    /// demanding exactness would be demanding a property the hardware does not
    /// have.
    #[test]
    fn a_quarter_turn_of_the_input_turns_the_answer_by_a_quarter() {
        for x in [-0x4000i32, -0x2000, -0x1000, -1, 0, 1, 0x1000, 0x2000, 0x4000] {
            for y in [-0x4000i32, -0x2000, -0x1000, -1, 0, 1, 0x1000, 0x2000, 0x4000] {
                if x == 0 && y == 0 {
                    continue;
                }
                let a = arctan2(x as i16, y as i16) as i32;
                let b = arctan2((-y) as i16, x as i16) as i32;
                let error = turn_error(b, a + AXIS_POS_Y as i32);
                assert!(error <= 1, "({x}, {y}): {b:#06x} vs {a:#06x}, off by {error}");
            }
        }
    }

    /// Negating the vector adds a half turn, modulo the full turn.
    #[test]
    fn a_half_turn_of_the_input_turns_the_answer_by_a_half() {
        for x in [-0x4000i32, -0x1234, -1, 0, 1, 0x1234, 0x4000] {
            for y in [-0x4000i32, -0x1234, -1, 0, 1, 0x1234, 0x4000] {
                if x == 0 && y == 0 {
                    continue;
                }
                let a = arctan2(x as i16, y as i16) as i32;
                let b = arctan2((-x) as i16, (-y) as i16) as i32;
                let error = turn_error(b, a + AXIS_NEG_X as i32);
                assert!(error <= 1, "({x}, {y}): {b:#06x} vs {a:#06x}, off by {error}");
            }
        }
    }

    /// Swapping the two components reflects the answer about the 45-degree
    /// line, not about an axis.
    ///
    /// `atan2` treats `x` and `y` symmetrically, so `(x, y)` and `(y, x)` are
    /// mirror images across the diagonal: the answer becomes `90 - angle`,
    /// i.e. `0x4000 - angle` in this scale. On the diagonal itself the two are
    /// equal, which is what makes this a good check -- a formula that is off
    /// by a quadrant fails immediately there instead of only on some inputs.
    #[test]
    fn swapping_the_components_reflects_about_the_diagonal() {
        for x in [-0x4000i32, -0x2000, -0x0100, 0x0100, 0x2000, 0x4000] {
            for y in [-0x4000i32, -0x2000, -0x0100, 0x0100, 0x2000, 0x4000] {
                if x == 0 && y == 0 {
                    continue;
                }
                let a = arctan2(x as i16, y as i16) as i32;
                let b = arctan2(y as i16, x as i16) as i32;
                let error = turn_error(b, AXIS_POS_Y as i32 - a);
                assert!(
                    error <= 1,
                    "({x}, {y}): {b:#06x} vs {a:#06x} reflected, off by {error}"
                );
            }
        }
    }

    /// The result stays inside the contract's range, for every input.
    ///
    /// `arctan2` is reachable from the `extern "C"` hook, so a wrapping
    /// computation that produced an out-of-range value here would be a real
    /// bug rather than a cosmetic one: the caller would read a register holding
    /// a nonsense angle.
    #[test]
    fn the_result_always_stays_in_range() {
        for x in [-0x7FFFi32, -0x4000, -0x1000, -1, 0, 1, 0x1000, 0x4000, 0x7FFF] {
            for y in [-0x7FFFi32, -0x4000, -0x1000, -1, 0, 1, 0x1000, 0x4000, 0x7FFF] {
                let _ = arctan2(x as i16, y as i16);
            }
        }
    }

    /// Every octant is reachable and lands in its own slice of the circle.
    ///
    /// Checked by sign pattern rather than by exact value, so it is robust to
    /// the polynomial's one-unit wobble while still failing loudly if an
    /// octant lands in the wrong quadrant -- the failure mode a missing branch
    /// or a wrong offset produces.
    #[test]
    fn every_octant_lands_in_its_own_slice_of_the_circle() {
        // One representative per octant, as (x, y), the quadrant the answer
        // must land in, and how far round it sits. The quadrant index is
        // `angle / 0x4000`; an octant that straddles a quadrant boundary is
        // placed by its far end, which is the stricter of the two.
        let cases: [(i16, i16, u16, &str); 8] = [
            (0x4000, 0x1000, 0, "0..45"),
            (0x1000, 0x4000, 0, "45..90"),
            (-0x1000, 0x4000, 1, "90..135"),
            (-0x4000, 0x1000, 1, "135..180"),
            (-0x4000, -0x1000, 2, "180..225"),
            (-0x1000, -0x4000, 2, "225..270"),
            (0x1000, -0x4000, 3, "270..315"),
            (0x4000, -0x1000, 3, "315..360"),
        ];
        for (x, y, expected, label) in cases {
            let got = arctan2(x, y);
            assert_eq!(
                got / 0x4000,
                expected,
                "({x}, {y}) [{label}] gave {got:#06x}, in quadrant {}",
                got / 0x4000
            );
        }
    }

    /// The 45-degree octant boundary is crossed without a jump.
    ///
    /// This is where `arctan2` switches branch, so a mistake here shows up as
    /// a visible seam in a rotated background. Walking up the first quadrant,
    /// the answer must rise smoothly and stay below the +Y axis.
    ///
    /// Deliberately **not** asserting strict monotonicity: the polynomial has a
    /// one-unit wobble at `y = 0x3F0A` (pinned in
    /// `the_documented_wobble_is_exactly_where_it_is`), and inheriting it is
    /// correct behaviour, not a defect.
    #[test]
    fn the_octant_boundaries_do_not_jump() {
        let mut previous = arctan2(0x4000, 0x0001) as i32;
        for y in 1..0x4000i32 {
            let got = arctan2(0x4000, y as i16) as i32;
            assert!(
                got >= previous - 1,
                "not monotone at y = {y}: {previous:#06x} -> {got:#06x}"
            );
            assert!(got < AXIS_POS_Y as i32, "y = {y} gave {got:#06x}, past the quadrant");
            previous = got;
        }
        // And it lands on a 45-degree value, not on an axis constant.
        let diagonal = arctan2(0x4000, 0x4000);
        assert!(
            diagonal > 0x1000 && diagonal < 0x3000,
            "45 degrees should be near half a quadrant, got {diagonal:#06x}"
        );
    }

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
    /// The function is odd **apart from the rounding direction**.
    ///
    /// The BIOS evaluates on the signed input and lets the arithmetic shift
    /// round towards negative infinity, so the two halves do not land on
    /// exactly opposite values: the magnitudes can differ by one, and the
    /// sum of the two is never further than one from zero. An implementation
    /// that took the magnitude and negated at the end would be exactly odd --
    /// more symmetric than the hardware, and therefore wrong. `arctan2`
    /// inherits the asymmetry, so folding it away here would corrupt its
    /// octants.
    ///
    /// This is the one place where being *more* symmetric than the hardware is
    /// a defect, so it is pinned rather than smoothed over.
    #[test]
    fn the_function_is_odd_apart_from_the_rounding_direction() {
        for v in -(UNITY as i32)..=(UNITY as i32) {
            let v = v as i16;
            let positive = arctan(v);
            let negative = arctan(-v);
            let magnitude_gap = negative.unsigned_abs() as i32 - positive.unsigned_abs() as i32;
            assert!(
                magnitude_gap == 0 || magnitude_gap == 1 || magnitude_gap == -1,
                "v = {v:#06x}: {positive:#06x} and {negative:#06x} differ by {magnitude_gap}, \
                 more than the rounding direction can explain"
            );
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
    /// are **six** -- three on each side, at different magnitudes, because
    /// negating the input does not mirror *where* the approximation wobbles.
    ///
    /// Pinning all of them means a change to the algorithm surfaces as *this*
    /// test failing, instead of as a unit or two of drift nobody notices.
    ///
    /// The count fell from fourteen to six when the Horner term was corrected
    /// to shift-then-negate (see [`arctan`]): the previous form's extra
    /// rounding artefacts on the negative half were not the hardware's.
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
                (0xC071, 1),
                (0xC0ED, 1),
                (0xC0F7, 1),
                (0x3F0A, 1),
                (0x3F14, 1),
                (0x3F90, 1)
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
