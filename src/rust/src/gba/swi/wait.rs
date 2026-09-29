//! SWI 0x02-0x05: the wait family.
//!
//! Per GBATEK the GBA BIOS keeps four "BIOS interrupt flags" at
//! 03007FF8h bits 0-3, one per hardware interrupt group (VBlank, HBlank,
//! VCount match, DMA/keypad). `IntrWait` reads and clears them. This module
//! owns that bit arithmetic because it is pure logic over the caller's
//! registers -- no core state -- and because it is the part that can be
//! exhaustively unit-tested without a running CPU.
//!
//! # Why nothing here is wired up yet
//!
//! Measured 2026-09-28 against the vendored baseline `ee77922d`:
//! `Arm7tdmi` (`cpu/arm7tdmi.rs:144-169`) has **no halted or stopped field**
//! at all, and `step()` (`:705` onward, once for ARM and once for Thumb) has
//! **no halt branch** -- every step checks IRQ and executes.
//!
//! So the core's `0x02` and `0x03..=0x05` arms are not merely shells waiting
//! for a body: the mechanism they would drive does not exist. Implementing
//! Halt/Stop requires new CPU state plus a halt check in both execution
//! paths, which is a CPU-model change, not a match arm. See the v2.0 plan
//! section 5.3 T1-b and the S1a re-estimate (1 week -> 2-3 weeks).

/// BIOS interrupt flags word at 03007FF8h.
pub const BIOS_FLAGS_ADDR: u32 = 0x0300_7FF8;

/// Mask of the four defined BIOS interrupt flags (bits 0-3).
pub const BIOS_FLAGS_MASK: u8 = 0x0F;

/// VBlank (the one every commercial game waits on).
pub const VBLANK_FLAG: u8 = 1 << 0;
/// HBlank.
pub const HBALANK_FLAG: u8 = 1 << 1;
/// VCount match.
pub const VCOUNT_FLAG: u8 = 1 << 2;
/// DMA0/1/2 and keypad share the last defined flag.
pub const DMA_FLAG: u8 = 1 << 3;

/// What `IntrWait` should do, from its second argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitMode {
    /// r0 == 0: return at once if the flag is already set, otherwise block.
    ReturnIfSet,
    /// r0 != 0: always block until set, then clear it.
    /// `VBlankIntrWait` is exactly `IntrWait(1, 1)`, i.e. this mode.
    AlwaysWait,
}

/// A decoded `IntrWait` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntrWaitRequest {
    /// Which of the four BIOS flag bits to wait for.
    pub wanted: u8,
    pub mode: WaitMode,
}

impl IntrWaitRequest {
    /// Decode the raw r0/r1 pair the caller passed.
    ///
    /// GBATEK has callers pass `1` for "VBlank" and `0x11` for "DMA0 and
    /// DMA1", so the argument is a *mask*, not an index. Bits outside the
    /// defined four are dropped rather than honoured: waiting on a reserved
    /// bit could never complete.
    pub fn decode(wanted_raw: u32, discard: u32) -> Self {
        Self {
            wanted: (wanted_raw as u8) & BIOS_FLAGS_MASK,
            mode: if discard & 1 == 0 {
                WaitMode::ReturnIfSet
            } else {
                WaitMode::AlwaysWait
            },
        }
    }

    /// Whether the current BIOS flags word already satisfies this request.
    pub fn satisfied_by(&self, flags: u8) -> bool {
        let f = flags & BIOS_FLAGS_MASK;
        match self.mode {
            // "always wait" must not short-circuit even if already set --
            // that is what distinguishes it from the BIOS's own fast path.
            WaitMode::AlwaysWait => false,
            WaitMode::ReturnIfSet => f & self.wanted == self.wanted,
        }
    }

    /// Whether the flag this request waits for is present, in either mode.
    ///
    /// This is the *wake-up* test, and it has to ignore the mode: `AlwaysWait`
    /// refuses to short-circuit before sleeping, but a sleeping CPU that
    /// refuses to wake up would hang the machine forever.
    pub fn is_set(&self, flags: u8) -> bool {
        flags & BIOS_FLAGS_MASK & self.wanted == self.wanted
    }

    /// The flags word after this wait completes. Only the wanted bits are
    /// cleared; the BIOS never touches bits the caller did not ask about.
    pub fn flags_after(&self, flags: u8) -> u8 {
        (flags & BIOS_FLAGS_MASK) & !self.wanted
    }

    /// Whether the request can never complete, because no defined bit was
    /// requested. The BIOS would block forever; we reject instead.
    pub fn is_unsatisfiable(&self) -> bool {
        self.wanted == 0
    }
}

/// `VBlankIntrWait` is `IntrWait(1, 1)`.
pub const fn vblank_intr_wait() -> IntrWaitRequest {
    IntrWaitRequest {
        wanted: VBLANK_FLAG,
        mode: WaitMode::AlwaysWait,
    }
}

/// The two low-power modes.
///
/// They differ in what resumes the CPU: `Halt` returns on any enabled
/// interrupt, `Stop` waits for the LCD controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowPower {
    Halt,
    Stop,
}

impl LowPower {
    /// The SWI number that requests this mode.
    pub const fn swi_number(self) -> u32 {
        match self {
            LowPower::Halt => 0x02,
            LowPower::Stop => 0x03,
        }
    }
}
