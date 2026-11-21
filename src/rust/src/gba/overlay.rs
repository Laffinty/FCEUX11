//! Frame dimension constants and the overlay flag.
//!
//! # History: the `BETA` watermark
//!
//! From v2.0 (plan section 7.1) until v2.0.7 this module drew a `BETA`
//! watermark into every frame that left for the caller — one synthesis point,
//! top-left corner, generated glyphs, semi-transparent white, and a
//! `gba_set_overlay` FFI whose release builds refused to turn it off (see git
//! history for that machinery and its tests). **Removed in v2.0.7 by owner
//! decision**: the GBA compatibility work (batch 1: the OBJ window-span fix,
//! the per-title verifications) justified an initial compatibility
//! recognition, so the mark came off the screen while the project's BETA
//! *status* itself remains unchanged.
//!
//! What is left here, on purpose:
//!
//! * the frame dimension constants every frame consumer needs;
//! * the [`Overlay`] flag and its `gba_set_overlay` semantics — kept so the
//!   C ABI surface and the savestate flag field do not change shape; the flag
//!   is bookkeeping only now (nothing reads it on the frame path), and the
//!   release-build refusal answers exactly as it always did.
//!
//! The behavioural lock for the removal lives in `frame.rs`
//! (`the_watermark_no_longer_reaches_the_caller_buffer`): reintroducing any
//! draw on the frame path turns it red.

/// Width and height of the 240x160 GBA screen.
pub const SCREEN_WIDTH: usize = 240;
/// See [`SCREEN_WIDTH`].
pub const SCREEN_HEIGHT: usize = 160;

/// Whether the watermark would be drawn, and whether the caller is allowed to
/// change that.
///
/// Since v2.0.7 nothing reads [`Overlay::enabled`] on the frame path — the
/// flag is kept so the ABI and the savestate field do not change shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overlay {
    enabled: bool,
    /// False in a release build. A release `gba_set_overlay(0)` must fail
    /// rather than be quietly ignored — a caller that is told "off" and is not
    /// off has been lied to, and that is worse than an error code. The
    /// refusal semantics are retained even though nothing draws any more.
    mutable: bool,
}

impl Overlay {
    /// The state a build starts in.
    #[must_use]
    pub const fn for_build(release: bool) -> Self {
        Self {
            enabled: true,
            mutable: !release,
        }
    }

    /// Whether the watermark would have been drawn.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Ask for the watermark to be off.
    ///
    /// Returns the state to keep, and whether the request was allowed. A
    /// release build always refuses — the same answer it gave while the mark
    /// was on screen, so no caller sees its ABI change shape under it.
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

#[cfg(test)]
mod tests {
    use super::{FRAME_BYTES, Overlay, SCREEN_HEIGHT, SCREEN_WIDTH};

    /// A black frame of the right size.
    fn blank() -> Vec<u8> {
        vec![0u8; FRAME_BYTES]
    }

    /// The frame is exactly 240x160 RGBA.
    #[test]
    fn the_frame_is_two_hundred_forty_by_one_sixty_rgba() {
        assert_eq!(SCREEN_WIDTH, 240);
        assert_eq!(SCREEN_HEIGHT, 160);
        assert_eq!(FRAME_BYTES, 240 * 160 * 4);
        assert_eq!(blank().len(), FRAME_BYTES);
    }

    /// A test build can be told to turn the flag off, and back on.
    ///
    /// The flag is bookkeeping since v2.0.7 (nothing draws), but its state
    /// machine is ABI-visible and must keep working.
    #[test]
    fn a_test_build_may_turn_the_watermark_off() {
        let mut overlay = Overlay::for_build(false);
        let (after, allowed) = overlay.set_enabled(false);
        assert!(allowed, "a test build must be able to disable it");
        assert!(!after.enabled());
        overlay = after;
        assert!(overlay.set_enabled(true).1);
    }

    /// A release build refuses, exactly as it always did.
    ///
    /// The mark is gone, but the ABI answer must not change shape under a
    /// caller that still checks the return value.
    #[test]
    fn a_release_build_refuses_to_turn_the_watermark_off() {
        let mut overlay = Overlay::for_build(true);
        let (after, allowed) = overlay.set_enabled(false);
        assert!(!allowed, "a release build must not accept the request");
        assert!(after.enabled(), "and must stay in the enabled state");
    }
}
