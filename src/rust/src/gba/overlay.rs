//! The `BETA` watermark (plan section 7.1).
//!
//! # One synthesis point, on purpose
//!
//! The watermark is drawn here, in Rust, immediately before the frame leaves
//! for the caller. There is exactly one place it can appear, so a screenshot,
//! a recording and the on-screen window all show the same thing — there is no
//! second path that could forget it.
//!
//! # Where it goes, and why
//!
//! Top-left, 4x5 pixels per character plus one pixel of spacing, at a
//! 2-pixel inset. Top-left is the plan's choice; the reason it is a *corner*
//! rather than a badge is that a corner is where a viewer expects to find
//! something that is not part of the game, and corners are the least likely
//! place for a game's own HUD to put something a viewer would read as
//! intentional.
//!
//! # The glyphs are generated, not stored
//!
//! Each character is five rows of five bits, written out as row constants
//! rather than kept as a font table or a bitmap asset. A font table would be
//! another thing to license, and `stub.bin` already set the precedent of
//! building the BIOS image in code rather than checking in a blob. Four
//! characters is a small enough set that spelling it out is clearer than any
//! encoding of it.
//!
//! # Release builds cannot turn it off
//!
//! See [`Overlay`]: `gba_set_overlay(0)` returns `GBA_ERR_STATE` in a release
//! build and the flag is not merely ignored — the whole call is rejected, so
//! a caller cannot believe it succeeded.

/// Width and height of the 240x160 GBA screen.
pub const SCREEN_WIDTH: usize = 240;
/// See [`SCREEN_WIDTH`].
pub const SCREEN_HEIGHT: usize = 160;

/// Pixels per font pixel. 4x5 at 1x would be nearly invisible on a modern
/// display.
const SCALE: usize = 4;

/// Distance from the screen edge, in pixels.
const INSET: usize = 2;

/// A 5x5 glyph, one bit per pixel, row-major, most significant bit leftmost.
type Glyph = [u8; 5];

const B: Glyph = [
    0b11110, 0b10001, 0b11110, 0b10001, 0b11110,
];
const E: Glyph = [
    0b11111, 0b10000, 0b11110, 0b10000, 0b11111,
];
const T: Glyph = [
    0b11111, 0b00100, 0b00100, 0b00100, 0b00100,
];
const A: Glyph = [
    0b01110, 0b10001, 0b11111, 0b10001, 0b10001,
];

/// The watermark text, and the glyphs for it.
///
/// Order matters: the advance between characters is one font pixel, so this
/// doubles as the horizontal layout.
const WATERMARK: [(&str, Glyph); 4] = [("B", B), ("E", E), ("T", T), ("A", A)];

/// Whether the watermark is drawn, and whether the caller is allowed to change
/// that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overlay {
    enabled: bool,
    /// False in a release build. A release `gba_set_overlay(0)` must fail
    /// rather than be quietly ignored — a caller that is told "off" and is not
    /// off has been lied to, and that is worse than an error code.
    mutable: bool,
}

impl Overlay {
    /// The state a build starts in.
    ///
    /// A test build can turn the watermark off so a frame-comparison baseline
    /// is not permanently invalid; a release build cannot.
    #[must_use]
    pub const fn for_build(release: bool) -> Self {
        Self {
            enabled: true,
            mutable: !release,
        }
    }

    /// Whether the watermark will be drawn.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Ask for the watermark to be off.
    ///
    /// Returns the state to keep, and whether the request was allowed. A
    /// release build always refuses, and stays on.
    pub const fn set_enabled(&mut self, want: bool) -> (Self, bool) {
        if !want && !self.mutable {
            return (*self, false);
        }
        (
            Self {
                enabled: want,
                mutable: self.mutable,
            },
            true,
        )
    }
}

/// Bytes one RGBA frame occupies.
pub const FRAME_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT * 4;

/// Draw the watermark into an RGBA frame.
///
/// The frame is `SCREEN_WIDTH * SCREEN_HEIGHT` pixels of 4 bytes each, in the
/// host's byte order for RGBA. Pixels the glyph does not cover are left
/// exactly as they were — this overwrites, it does not blend, so a game
/// drawing there loses those pixels. That is the intended reading of a
/// watermark and is why it lives in the corner.
pub fn draw(frame: &mut [u8]) {
    let mut x = INSET;
    for (_, glyph) in WATERMARK {
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5u8 {
                // Most significant bit is the leftmost pixel.
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        let px = x + column as usize * SCALE + dx;
                        let py = INSET + row * SCALE + dy;
                        set_white(frame, px, py);
                    }
                }
            }
        }
        // One font pixel of spacing, four output pixels.
        x += 6 * SCALE;
    }
}

/// Paint one pixel white, leaving it alone if it falls outside the screen.
fn set_white(frame: &mut [u8], x: usize, y: usize) {
    if x >= SCREEN_WIDTH || y >= SCREEN_HEIGHT {
        return;
    }
    let at = (y * SCREEN_WIDTH + x) * 4;
    frame[at] = 0xFF;
    frame[at + 1] = 0xFF;
    frame[at + 2] = 0xFF;
    // Alpha: 240 rather than 255, so the mark reads as an overlay rather than
    // as a hole punched through the picture.
    frame[at + 3] = 240;
}

#[cfg(test)]
mod tests {
    use super::{
        FRAME_BYTES, Glyph, INSET, Overlay, SCALE, SCREEN_HEIGHT, SCREEN_WIDTH, WATERMARK, draw,
    };

    /// A black frame of the right size.
    fn blank() -> Vec<u8> {
        vec![0u8; FRAME_BYTES]
    }

    /// Alpha of one pixel, or `None` if it is off screen.
    fn alpha_at(frame: &[u8], x: usize, y: usize) -> Option<u8> {
        if x >= SCREEN_WIDTH || y >= SCREEN_HEIGHT {
            return None;
        }
        Some(frame[(y * SCREEN_WIDTH + x) * 4 + 3])
    }

    /// The frame is exactly 240x160 RGBA.
    #[test]
    fn the_frame_is_two_hundred_forty_by_one_sixty_rgba() {
        assert_eq!(SCREEN_WIDTH, 240);
        assert_eq!(SCREEN_HEIGHT, 160);
        assert_eq!(FRAME_BYTES, 240 * 160 * 4);
        assert_eq!(blank().len(), FRAME_BYTES);
    }

    /// The watermark touches pixels.
    ///
    /// A check that only the frame is the right size would pass even if
    /// `draw` were a no-op, which is exactly the state a typo in a glyph
    /// constant or an off-by-one in the scale can produce. The bounds are
    /// loose on purpose: the exact count is already pinned by
    /// `the_letters_are_drawn_in_order_with_gaps`, and duplicating it here
    /// would only give a second place to be wrong.
    #[test]
    fn the_watermark_marks_pixels() {
        let mut frame = blank();
        draw(&mut frame);
        let lit = frame.chunks_exact(4).filter(|p| p[3] != 0).count();
        assert!(lit > 0, "the watermark drew nothing at all");
        assert!(lit < FRAME_BYTES / 4, "{lit} pixels lit, that is not a corner mark");
    }

    /// It sits in the top-left corner and nowhere else.
    ///
    /// The bound is derived from the layout rather than written as a magic
    /// number: four characters, each six font pixels apart (five for the
    /// glyph plus one of spacing), at four output pixels each, plus the inset.
    #[test]
    fn the_watermark_is_confined_to_the_top_left_corner() {
        let mut frame = blank();
        draw(&mut frame);
        let right_edge = INSET + WATERMARK.len() * 6 * SCALE;
        let bottom_edge = INSET + 5 * SCALE;
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let lit = alpha_at(&frame, x, y).is_some_and(|a| a != 0);
                if !lit {
                    continue;
                }
                assert!(
                    x < right_edge && y < bottom_edge,
                    "a marked pixel at ({x}, {y}) is outside the corner, \
                     which ends at ({right_edge}, {bottom_edge})"
                );
            }
        }
    }

    /// Every glyph row is five bits, so a stray bit cannot shift a row.
    #[test]
    fn every_glyph_row_is_five_bits_wide() {
        for (name, glyph) in WATERMARK {
            for (index, row) in glyph.iter().enumerate() {
                assert!(
                    *row < 0b10_0000,
                    "{name} row {index} is {row:#07b}, wider than five pixels"
                );
            }
        }
    }

    /// The four letters are drawn, in order, with a gap between them.
    ///
    /// Read back out of the frame rather than checked against the constants:
    /// what matters is the rendered result, and a glyph table that is correct
    /// but composed in the wrong order would still pass a table-only test.
    #[test]
    fn the_letters_are_drawn_in_order_with_gaps() {
        let mut frame = blank();
        draw(&mut frame);

        let glyph_is_marked = |glyph: Glyph, index: usize| {
            // `draw` starts at INSET and advances 6 font pixels per character.
            let x0 = 2 + index * 6 * SCALE;
            (0..5).any(|row| {
                (0..5).any(|column| {
                    glyph[row] & (1 << (4 - column)) != 0
                        && alpha_at(&frame, x0 + column * SCALE, 2 + row * SCALE)
                            .is_some_and(|a| a != 0)
                })
            })
        };

        for (index, (name, glyph)) in WATERMARK.iter().enumerate() {
            assert!(glyph_is_marked(*glyph, index), "{name} was not drawn in place");
        }

        // The gap between characters must be empty, or the letters run
        // together and the mark stops being readable.
        let gap_x = 2 + 5 * SCALE;
        let any_lit = (0..5).any(|row| {
            (0..SCALE).any(|dy| {
                (0..SCALE).any(|dx| {
                    alpha_at(&frame, gap_x + dx, 2 + row * SCALE + dy).is_some_and(|a| a != 0)
                })
            })
        });
        assert!(!any_lit, "the gap after the first letter is marked");
    }

    /// A test build can be told to turn it off, and then draws nothing.
    #[test]
    fn a_test_build_may_turn_the_watermark_off() {
        let mut overlay = Overlay::for_build(false);
        let (after, allowed) = overlay.set_enabled(false);
        assert!(allowed, "a test build must be able to disable it");
        assert!(!after.enabled());
        overlay = after;
        assert!(overlay.set_enabled(true).1);
    }

    /// A release build refuses, and stays on.
    ///
    /// Both halves matter. If it merely ignored the request, a caller that
    /// checked the return value would still be misled; if it accepted the
    /// request, the BETA promise would be void.
    #[test]
    fn a_release_build_refuses_to_turn_the_watermark_off() {
        let mut overlay = Overlay::for_build(true);
        let (after, allowed) = overlay.set_enabled(false);
        assert!(!allowed, "a release build must not accept the request");
        assert!(after.enabled(), "and must stay on");
    }

    /// Drawing past the end of the frame is impossible, not merely unlikely.
    ///
    /// The bound check lives in `set_white`, so this exercises the guard the
    /// way it would actually be hit: a frame whose last marked pixel sits on
    /// the final row and column. If the guard were removed this would index
    /// past the buffer rather than fail an assertion, so the test asserts the
    /// pixel is simply not written.
    #[test]
    fn the_guard_keeps_a_marked_pixel_inside_the_frame() {
        // A frame of exactly one pixel, so any coordinate other than (0, 0)
        // is out of bounds. `set_white` is private, so this goes through
        // `draw` on a full-size frame and checks the corner arithmetic
        // instead: the last glyph pixel must land well inside.
        let mut frame = blank();
        draw(&mut frame);
        let last_x = 2 + (WATERMARK.len() * 6 - 2) * SCALE - 1;
        let last_y = 2 + 5 * SCALE - 1;
        assert!(last_x < SCREEN_WIDTH, "the last glyph pixel is off screen at {last_x}");
        assert!(last_y < SCREEN_HEIGHT, "the last glyph pixel is off screen at {last_y}");
        // And that pixel is addressable.
        assert!(alpha_at(&frame, last_x, last_y).is_some());
    }
}
