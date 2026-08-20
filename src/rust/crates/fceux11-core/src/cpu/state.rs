//! CPU state — registers, flags, and the 64-byte X6502-compatible layout.
//!
//! The X6502 layout is the **savestate binary contract** with the legacy
//! C++ implementation. We mirror its field-for-field offsets under
//! `#[repr(C, align(64))]` so a savestate written by `Cpu::snapshot()` can
//! be loaded by the old `X6502` blob (and vice versa) without byte drift.
//!
//! See `src/x6502struct.h` for the authoritative C++ definition; see
//! `src/cpu.cpp:11–22` for the `static_assert` that pins the 64-byte size.
//!
//! ## Layout
//!
//! ```text
//! offset  size  field
//! 0       4     tcount     (i32, temporary cycle counter)
//! 4       2     pc         (u16)
//! 6       1     a
//! 7       1     x
//! 8       1     y
//! 9       1     s
//! 10      1     p
//! 11      1     moo_pi
//! 12      1     jammed
//! 13      3     _pad0      (align next i32)
//! 16      4     count      (i32)
//! 20      4     irq_low    (u32 — see [`IrqSource`])
//! 24      1     db
//! 25      3     _pad1      (align next i32)
//! 28      4     preexec    (i32)
//! 32      …     [debugger hooks — Release build pads to 64]
//! ```
//!
//! Total payload: 32 bytes (Release) or 56 bytes (Debugger). With
//! `align(64)`, both round up to 64 bytes, satisfying the C++ `static_assert`.

use bitflags::bitflags;
use core::mem::offset_of;

/// Raw, FFI-stable X6502-compatible layout. Do **not** add fields, reorder
/// fields, or change field types — doing so breaks savestate compatibility.
///
/// Mirrors `src/x6502struct.h:7–32`.
#[repr(C, align(64))]
#[derive(Copy, Clone)]
pub struct X6502Layout {
    /// Temporary cycle counter (per-instruction scratch).
    pub tcount: i32,
    /// Program counter (16-bit, masked to 0xFFFF on every increment).
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub p: u8,
    /// Mirror of `p` that records the state observed at the start of the
    /// previous instruction. Used by IRQ-line sampling.
    pub moo_pi: u8,
    /// 1 = CPU is jammed (STP/KIL); cleared by reset.
    pub jammed: u8,
    _pad0: [u8; 3],
    /// Cumulative cycle counter (× 1/16-dot units, see `X6502_RunDebug`).
    pub count: i32,
    /// IRQ source bitmask (see [`IrqSource`]).
    pub irq_low: u32,
    /// Data-bus "cache" for reads from certain areas (cart mappers peek
    /// at this for open-bus behaviour).
    pub db: u8,
    _pad1: [u8; 3],
    /// Pre-execution bookkeeping for debug breakpoints.
    pub preexec: i32,
    /// 32 bytes of trailing padding so `align(64)` holds and
    /// `size_of::<X6502Layout>() == 64`.
    _tail_pad: [u8; 32],
}

// Compile-time guarantee that the layout matches the C++ side. The C++
// `static_assert` lives in `src/cpu.cpp:11–22`; we mirror it here so any
// drift fails at Rust build time too.
const _: () = {
    assert!(offset_of!(X6502Layout, tcount) == 0);
    assert!(offset_of!(X6502Layout, pc) == 4);
    assert!(offset_of!(X6502Layout, a) == 6);
    assert!(offset_of!(X6502Layout, x) == 7);
    assert!(offset_of!(X6502Layout, y) == 8);
    assert!(offset_of!(X6502Layout, s) == 9);
    assert!(offset_of!(X6502Layout, p) == 10);
    assert!(offset_of!(X6502Layout, moo_pi) == 11);
    assert!(offset_of!(X6502Layout, jammed) == 12);
    assert!(offset_of!(X6502Layout, count) == 16);
    assert!(offset_of!(X6502Layout, irq_low) == 20);
    assert!(offset_of!(X6502Layout, db) == 24);
    assert!(offset_of!(X6502Layout, preexec) == 28);
    assert!(core::mem::size_of::<X6502Layout>() == 64);
};

impl X6502Layout {
    /// Zero-init the layout (matches `X6502_Init`'s `memset`).
    #[inline]
    pub const fn zeroed() -> Self {
        Self {
            tcount: 0,
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            s: 0,
            p: 0,
            moo_pi: 0,
            jammed: 0,
            _pad0: [0; 3],
            count: 0,
            irq_low: 0,
            db: 0,
            _pad1: [0; 3],
            preexec: 0,
            _tail_pad: [0; 32],
        }
    }
}

impl Default for X6502Layout {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl core::fmt::Debug for X6502Layout {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("X6502Layout")
            .field("pc", &format_args!("${:04X}", self.pc))
            .field("a", &format_args!("${:02X}", self.a))
            .field("x", &format_args!("${:02X}", self.x))
            .field("y", &format_args!("${:02X}", self.y))
            .field("s", &format_args!("${:02X}", self.s))
            .field("p", &format_args!("${:02X}", self.p))
            .field("moo_pi", &format_args!("${:02X}", self.moo_pi))
            .field("jammed", &self.jammed)
            .field("tcount", &self.tcount)
            .field("count", &self.count)
            .field("irq_low", &format_args!("${:04X}", self.irq_low))
            .field("db", &format_args!("${:02X}", self.db))
            .field("preexec", &self.preexec)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Flags (P register bitmasks). Numeric values MUST match the C++
// `inline constexpr uint8_t` constants in `src/x6502.h:75–82` exactly.
// ---------------------------------------------------------------------------
bitflags! {
    /// 6502 P (status) register flags.
    #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
    pub struct Flags: u8 {
        const CARRY     = 0x01;
        const ZERO      = 0x02;
        const IRQ_DIS   = 0x04; // I flag — interrupt disable
        const DECIMAL   = 0x08;
        const BREAK     = 0x10; // B flag — software-set on BRK/PHP push
        const UNUSED    = 0x20; // U flag — always reads as 1 inside the CPU
        const OVERFLOW   = 0x40;
        const NEGATIVE  = 0x80;
    }
}

impl Flags {
    /// P register value after a 6502 reset / power-on, matching the C++
    /// reference exactly (`src/x6502.cpp` RESET dispatch: `_PI=_P=I_FLAG`).
    /// Only I is set; B, U and all arithmetic flags are cleared.
    pub const RESET: Flags = Flags::from_bits_truncate(Self::IRQ_DIS.bits());

    /// Flags observed on the **stack** during a push (BRK / PHP / IRQ entry):
    /// U flag always set, B flag set when pushed from BRK / IRQ.
    pub fn push_value(self, b_flag: bool) -> u8 {
        let mut v = self.bits() | Self::UNUSED.bits();
        if b_flag {
            v |= Self::BREAK.bits();
        } else {
            v &= !Self::BREAK.bits();
        }
        v
    }

    /// Update Z and N from an 8-bit value (most ALU helpers).
    /// `X_ZN(zort)` in C++.
    #[inline]
    pub fn set_zn(&mut self, val: u8) {
        let zn = ZN_TABLE[val as usize];
        // Clear Z and N, then OR in the precomputed bits. We use the
        // bitflags 2.x `from_bits_retain` so unknown bits stay 0.
        let mut v = self.bits();
        v &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
        v |= zn & (Flags::ZERO.bits() | Flags::NEGATIVE.bits());
        *self = Flags::from_bits_retain(v);
    }
}

// ---------------------------------------------------------------------------
// IRQ source bitmask values. MUST match C++ `inline constexpr uint32_t`
// constants in `src/x6502.h:96–103` exactly.
// ---------------------------------------------------------------------------
bitflags! {
    /// Mask of active IRQ sources. Multiple sources may be asserted
    /// simultaneously; the CPU handles them in priority order.
    #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
    pub struct IrqSource: u32 {
        const EXTERNAL   = 0x001;
        const EXTERNAL2  = 0x002;
        const RESET      = 0x020;
        const NMI2       = 0x040; // Delayed NMI; converted to NMI on the next boundary
        const NMI        = 0x080;
        const DPCM       = 0x100;
        const FRAME      = 0x200;
        const TEMP       = 0x800;
    }
}

// ---------------------------------------------------------------------------
// Precomputed Z/N flag table. ZNTable[byte] returns the Z_FLAG / N_FLAG
// bits to OR into P. Mirrors `ZNTable` initialised in `X6502_Init()`
// (x6502.cpp:471–487).
// ---------------------------------------------------------------------------

/// ZNTable[byte] → `Flags::ZERO | Flags::NEGATIVE` bits for that byte.
pub static ZN_TABLE: [u8; 256] = build_zn_table();

const fn build_zn_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    table[0] = Flags::ZERO.bits();
    let mut i = 1;
    while i < 256 {
        table[i] = if i & 0x80 != 0 {
            Flags::NEGATIVE.bits()
        } else {
            0
        };
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_64_bytes() {
        assert_eq!(core::mem::size_of::<X6502Layout>(), 64);
        assert_eq!(core::mem::align_of::<X6502Layout>(), 64);
    }

    #[test]
    fn layout_field_offsets_match_cpp() {
        // Cross-checked against `src/x6502struct.h`.
        assert_eq!(offset_of!(X6502Layout, tcount), 0);
        assert_eq!(offset_of!(X6502Layout, pc), 4);
        assert_eq!(offset_of!(X6502Layout, a), 6);
        assert_eq!(offset_of!(X6502Layout, s), 9);
        assert_eq!(offset_of!(X6502Layout, irq_low), 20);
        assert_eq!(offset_of!(X6502Layout, db), 24);
        assert_eq!(offset_of!(X6502Layout, preexec), 28);
    }

    #[test]
    fn zn_table_zero_only_for_zero() {
        assert_eq!(ZN_TABLE[0x00], Flags::ZERO.bits());
        for &b in &[0x01u8, 0x7F, 0x80, 0xFF] {
            let bits = ZN_TABLE[b as usize];
            if b == 0 {
                assert_eq!(bits & Flags::ZERO.bits(), Flags::ZERO.bits());
            } else {
                assert_eq!(bits & Flags::ZERO.bits(), 0);
            }
            if b & 0x80 != 0 {
                assert_eq!(bits & Flags::NEGATIVE.bits(), Flags::NEGATIVE.bits());
            } else {
                assert_eq!(bits & Flags::NEGATIVE.bits(), 0);
            }
        }
    }

    #[test]
    fn reset_flags_match_cpp() {
        // X6502_Power() / X6502_RunDebug(): after consuming RESET,
        // `_PI=_P=I_FLAG` — P is wiped to exactly 0x04 (I only).
        let reset = Flags::RESET;
        assert_eq!(reset.bits(), 0x04);
        assert!(reset.contains(Flags::IRQ_DIS));
        assert!(!reset.contains(Flags::UNUSED));
        assert!(!reset.contains(Flags::BREAK));
    }

    #[test]
    fn irq_source_values_match_cpp() {
        // Numeric values MUST equal C++ inline constexpr constants.
        assert_eq!(IrqSource::EXTERNAL.bits(), 0x001);
        assert_eq!(IrqSource::EXTERNAL2.bits(), 0x002);
        assert_eq!(IrqSource::RESET.bits(), 0x020);
        assert_eq!(IrqSource::NMI2.bits(), 0x040);
        assert_eq!(IrqSource::NMI.bits(), 0x080);
        assert_eq!(IrqSource::DPCM.bits(), 0x100);
        assert_eq!(IrqSource::FRAME.bits(), 0x200);
        assert_eq!(IrqSource::TEMP.bits(), 0x800);
    }

    #[test]
    fn push_value_applies_u_and_b_flags() {
        let p = Flags::from_bits_truncate(0xC0); // N + V
        assert_eq!(p.push_value(true), 0xF0); // N+V+B+U
        assert_eq!(p.push_value(false), 0xE0); // N+V+U (no B)
    }
}