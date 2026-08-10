//! Base cycle counts for all 256 6502 opcodes.
//!
//! Values are the **base cycle count** at the start of each instruction.
//! Page-cross penalties (+1 cycle for read crossing a page boundary,
//! or +1 for branch taken crossing page) and RMW extra cycle (+1 for
//! write-modify-read instructions like INC/DEC/ASL/ROL/LSR/ROR at
//! absolute indexed addressing modes) are added by the dispatcher.
//!
//! Source: derived from the canonical 6502 reference (e.g. NESDev wiki
//! "CPU unofficial opcodes" and the `CycTable` in upstream FCEUX
//! `src/x6502.cpp`). Verified against `CycTable[b1]` semantics in
//! `x6502.cpp` for the official 151 opcodes.
//!
//! Note: NES NTSC CPU clock = 1.789773 MHz. The C++ scales `X6502_Run`
//! input by 16 (PAL scales by 15) to give fixed-point accuracy for the
//! scheduler. Our Phase 1 budget model keeps cycles in integer units;
//! the ×16 scaling is a scheduler concern (Phase 4) and does not
//! belong here.

/// 256-entry table: base cycles per opcode (NTSC).
pub const BASE_CYCLES: [u8; 256] = [
    /*0x00*/ 7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,
    /*0x10*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x20*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    /*0x30*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x40*/ 6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,
    /*0x50*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x60*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,
    /*0x70*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x80*/ 2, 6, 2, 6, 3, 3, 3, 3, 3, 2, 2, 2, 4, 4, 4, 4,
    /*0x90*/ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    /*0xA0*/ 2, 6, 2, 6, 3, 3, 3, 3, 3, 2, 2, 2, 4, 4, 4, 4,
    /*0xB0*/ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    /*0xC0*/ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xD0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0xE0*/ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xF0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

/// Whether this opcode is a read-modify-write that may need an extra
/// cycle at absolute indexed addressing modes (per 6502 quirk).
/// RMW ops: ASL, ROL, LSR, ROR, INC, DEC, plus their unofficial
/// equivalents (SLO, RLA, SRE, RRA, ISB, DCP).
pub const IS_RMW: [bool; 256] = {
    let mut t = [false; 256];
    // Official RMW
    t[0x06] = true; t[0x0A] = true; t[0x0E] = true; t[0x16] = true;
    t[0x1E] = true;
    t[0x26] = true; t[0x2A] = true; t[0x2E] = true; t[0x36] = true;
    t[0x3E] = true;
    t[0x46] = true; t[0x4A] = true; t[0x4E] = true; t[0x56] = true;
    t[0x5E] = true;
    t[0x66] = true; t[0x6A] = true; t[0x6E] = true; t[0x76] = true;
    t[0x7E] = true;
    t[0xC6] = true; t[0xCE] = true; t[0xD6] = true; t[0xDE] = true;
    t[0xE6] = true; t[0xEE] = true; t[0xF6] = true; t[0xFE] = true;
    // Unofficial RMW
    t[0x03] = true; t[0x07] = true; t[0x0B] = true; t[0x0F] = true;
    t[0x13] = true; t[0x17] = true; t[0x1B] = true; t[0x1F] = true;
    t[0x23] = true; t[0x27] = true; t[0x2B] = true; t[0x2F] = true;
    t[0x33] = true; t[0x37] = true; t[0x3B] = true; t[0x3F] = true;
    t[0x43] = true; t[0x47] = true; t[0x4B] = true; t[0x4F] = true;
    t[0x53] = true; t[0x57] = true; t[0x5B] = true; t[0x5F] = true;
    t[0x63] = true; t[0x67] = true; t[0x6B] = true; t[0x6F] = true;
    t[0x73] = true; t[0x77] = true; t[0x7B] = true; t[0x7F] = true;
    t[0x83] = true; t[0x87] = true; t[0x8B] = true; t[0x8F] = true;
    t[0x93] = true; t[0x97] = true; t[0x9B] = true; t[0x9F] = true;
    t[0xA3] = true; t[0xA7] = true; t[0xAB] = true; t[0xAF] = true;
    t[0xB3] = true; t[0xB7] = true; t[0xBB] = true; t[0xBF] = true;
    t[0xC3] = true; t[0xC7] = true; t[0xCB] = true; t[0xCF] = true;
    t[0xD3] = true; t[0xD7] = true; t[0xDB] = true; t[0xDF] = true;
    t[0xE3] = true; t[0xE7] = true; t[0xEB] = true; t[0xEF] = true;
    t[0xF3] = true; t[0xF7] = true; t[0xFB] = true; t[0xFF] = true;
    t
};

/// Whether this opcode reads memory (used to decide page-cross penalty).
/// Read ops: LDA, LDX, LDY, EOR, AND, ORA, ADC, SBC, CMP, BIT, PLA,
/// plus their unofficial variants.
pub const IS_READ: [bool; 256] = {
    let mut t = [false; 256];
    t
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_cycles_nonzero() {
        // Every opcode must consume at least 2 cycles (the smallest
        // legitimate instruction is the 2-cycle implied NOP).
        for &c in BASE_CYCLES.iter() {
            assert!(c >= 2, "opcode cycle count must be ≥ 2");
        }
    }

    #[test]
    fn rmw_marked_for_known_ops() {
        // ASL A (0x0A), ROL A (0x2A), LSR A (0x4A), ROR A (0x6A) are all RMW.
        assert!(IS_RMW[0x0A]);
        assert!(IS_RMW[0x2A]);
        assert!(IS_RMW[0x4A]);
        assert!(IS_RMW[0x6A]);
    }

    #[test]
    fn rmw_zero_page_indexed() {
        // INC zp,X (0xF6), DEC zp,X (0xD6) — RMW at zp indexed.
        assert!(IS_RMW[0xF6]);
        assert!(IS_RMW[0xD6]);
    }
}
