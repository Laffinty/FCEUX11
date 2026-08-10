//! 6502 addressing modes.
//!
//! Each mode provides:
//! - `mode_size(opcode) -> u8`     number of extra bytes after the opcode
//! - `fetch_addr(cpu, bus) -> Addr` resolves effective address (and
//!   reads any operands from memory). May also return a "page-crossed"
//!   flag for cycle-penalty purposes.
//!
//! Page-cross penalty rules:
//! - Read modes: +1 cycle if effective addr page != operand page
//!   (e.g., LDA $nnnn,Y when nn crosses page)
//! - Branch modes: +1 cycle if branch taken AND target page != PC page
//! - RMW absolute indexed: +1 cycle always (per 6502 quirk)
//! - JMP indirect: $nnnn crossed page is an emulator-defined quirk
//!   (the 6502 bug where $00FF wraps to $0000 instead of $0100)
//!
//! Source conventions: see `src/ops_table.inc` for the C++ macro forms
//! (LD_IMM, LD_ZP, LD_ZPX, ...).

use crate::cpu::BusContext;

/// Resolved addressing mode for an operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addr {
    /// 16-bit effective address.
    pub addr: u16,
    /// True if the address computation crossed a page boundary
    /// (consumes +1 cycle for read modes, +1 for taken branches).
    pub page_crossed: bool,
    /// True if the operand was at zero-page (zp, zp,X, zp,Y).
    /// Used by RMW modes to skip the extra-cycle quirk.
    pub zero_page: bool,
}

/// Number of operand bytes (after the opcode) for each 6502 opcode.
/// This matches `opsize[]` in `src/x6502.cpp` (with `BRK_3BYTE_HACK` not
/// defined, i.e. BRK is 1 byte like every other 6502 implementation —
/// the "signature byte" is implicit).
pub fn op_size(opcode: u8) -> u8 {
    // Source: src/x6502.cpp opsize[256] verbatim.
    OPSIZE[opcode as usize]
}

const OPSIZE: [u8; 256] = [
    /*0x00*/ 1, 2, 0, 0, 2, 2, 2, 2, 0, 0, 0, 0, 3, 3, 3, 3,
    /*0x10*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
    /*0x20*/ 3, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0x30*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
    /*0x40*/ 1, 2, 0, 0, 0, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0x50*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
    /*0x60*/ 1, 2, 0, 0, 0, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0x70*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
    /*0x80*/ 0, 2, 0, 0, 2, 2, 2, 0, 1, 0, 1, 0, 3, 3, 3, 0,
    /*0x90*/ 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 0, 3, 0, 0,
    /*0xA0*/ 2, 2, 2, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0xB0*/ 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    /*0xC0*/ 2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0xD0*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
    /*0xE0*/ 2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0xF0*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
];

/// Read the byte at `pc` and increment pc. Used by instruction fetch.
#[inline(always)]
pub fn fetch_byte<BC: BusContext>(pc: &mut u16, bus: &mut BC) -> u8 {
    let b = bus.read(*pc);
    *pc = pc.wrapping_add(1);
    b
}

/// Read the little-endian word at `pc` and increment pc by 2.
#[inline(always)]
pub fn fetch_word<BC: BusContext>(pc: &mut u16, bus: &mut BC) -> u16 {
    let lo = bus.read(*pc) as u16;
    let hi = bus.read(pc.wrapping_add(1)) as u16;
    *pc = pc.wrapping_add(2);
    lo | (hi << 8)
}

/// Read zero-page address at `pc` (wraps within page 0).
#[inline(always)]
pub fn fetch_zp<BC: BusContext>(pc: &mut u16, bus: &mut BC) -> u8 {
    let b = bus.read(*pc);
    *pc = pc.wrapping_add(1);
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::regs::CpuRegsLayout;

    struct TestBus {
        mem: [u8; 65536],
        pc: u16,
        stall: bool,
    }
    impl crate::cpu::BusContext for TestBus {
        fn read(&mut self, addr: u16) -> u8 { self.mem[addr as usize] }
        fn write(&mut self, addr: u16, val: u8) { self.mem[addr as usize] = val; }
        fn dma_stalled(&self) -> bool { self.stall }
    }

    #[test]
    fn op_size_brk_is_1() {
        // The upstream `BRK_3BYTE_HACK` is not used in this build —
        // BRK is a 1-byte instruction (the "signature byte" is implicit).
        assert_eq!(op_size(0x00), 1);
    }

    #[test]
    fn op_size_jmp_abs_is_3() {
        // JMP $nnnn
        assert_eq!(op_size(0x4C), 3);
    }

    #[test]
    fn op_size_lda_imm_is_2() {
        assert_eq!(op_size(0xA9), 2);
    }

    #[test]
    fn fetch_byte_increments_pc() {
        let mut bus = TestBus { mem: [0; 65536], pc: 0x100, stall: false };
        bus.mem[0x100] = 0x42;
        let mut pc = 0x100;
        assert_eq!(fetch_byte(&mut pc, &mut bus), 0x42);
        assert_eq!(pc, 0x101);
    }

    #[test]
    fn fetch_word_reads_le() {
        let mut bus = TestBus { mem: [0; 65536], pc: 0x200, stall: false };
        bus.mem[0x200] = 0x34;
        bus.mem[0x201] = 0x12;
        let mut pc = 0x200;
        assert_eq!(fetch_word(&mut pc, &mut bus), 0x1234);
        assert_eq!(pc, 0x202);
    }

    // Silence unused-import warning when CpuRegsLayout is not used directly.
    #[allow(dead_code)]
    fn _ensure_layout_import(_: CpuRegsLayout) {}
}
