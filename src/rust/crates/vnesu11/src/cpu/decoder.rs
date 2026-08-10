//! Decoder — 256-opcode dispatch.
//!
//! The compiler lowers `match opcode` to a jump table (or series of
//! compares). Every opcode handler is a `CpuCore` method that decrements
//! `count` for page-cross / RMW / taken-branch extras.

use super::BusContext;
use super::CpuCore;

/// Execute one opcode (already fetched; PC advanced past the opcode byte).
///
/// Note: some opcodes that need operands (e.g., the "unofficial NOP with
/// operand") consume the operand via their addressing method. Pure NOPs
/// with unused operand bytes just leave PC pointing at the operand —
/// the next fetch consumes it, which is the classic "2-byte NOP" behavior.
#[inline(always)]
pub fn execute<BC: BusContext>(cpu: &mut CpuCore, op: u8, bus: &mut BC) {
    match op {
        // ---- Load / store ----
        0xA9 => cpu.lda_imm(bus),
        0xA5 => cpu.lda_zp(bus),
        0xB5 => cpu.lda_zpx(bus),
        0xAD => cpu.lda_abs(bus),
        0xBD => cpu.lda_absx(bus),
        0xB9 => cpu.lda_absy(bus),
        0xA1 => cpu.lda_izx(bus),
        0xB1 => cpu.lda_izy(bus),

        0xA2 => cpu.ldx_imm(bus),
        0xA6 => cpu.ldx_zp(bus),
        0xB6 => cpu.ldx_zpy(bus),
        0xAE => cpu.ldx_abs(bus),
        0xBE => cpu.ldx_absy(bus),

        0xA0 => cpu.ldy_imm(bus),
        0xA4 => cpu.ldy_zp(bus),
        0xB4 => cpu.ldy_zpx(bus),
        0xAC => cpu.ldy_abs(bus),
        0xBC => cpu.ldy_absx(bus),

        0x85 => cpu.sta_zp(bus),
        0x95 => cpu.sta_zpx(bus),
        0x8D => cpu.sta_abs(bus),
        0x9D => cpu.sta_absx(bus),
        0x99 => cpu.sta_absy(bus),
        0x81 => cpu.sta_izx(bus),
        0x91 => cpu.sta_izy(bus),

        0x86 => cpu.stx_zp(bus),
        0x96 => cpu.stx_zpy(bus),
        0x8E => cpu.stx_abs(bus),

        0x84 => cpu.sty_zp(bus),
        0x94 => cpu.sty_zpx(bus),
        0x8C => cpu.sty_abs(bus),

        // ---- Arithmetic ----
        0x69 => cpu.adc_imm(bus),
        0x65 => cpu.adc_zp(bus),
        0x75 => cpu.adc_zpx(bus),
        0x6D => cpu.adc_abs(bus),
        0x7D => cpu.adc_absx(bus),
        0x79 => cpu.adc_absy(bus),
        0x61 => cpu.adc_izx(bus),
        0x71 => cpu.adc_izy(bus),

        0xE9 => cpu.sbc_imm(bus),
        0xE5 => cpu.sbc_zp(bus),
        0xF5 => cpu.sbc_zpx(bus),
        0xED => cpu.sbc_abs(bus),
        0xFD => cpu.sbc_absx(bus),
        0xF9 => cpu.sbc_absy(bus),
        0xE1 => cpu.sbc_izx(bus),
        0xF1 => cpu.sbc_izy(bus),

        0x29 => cpu.and_imm(bus),
        0x25 => cpu.and_zp(bus),
        0x35 => cpu.and_zpx(bus),
        0x2D => cpu.and_abs(bus),
        0x3D => cpu.and_absx(bus),
        0x39 => cpu.and_absy(bus),
        0x21 => cpu.and_izx(bus),
        0x31 => cpu.and_izy(bus),

        0x09 => cpu.ora_imm(bus),
        0x05 => cpu.ora_zp(bus),
        0x15 => cpu.ora_zpx(bus),
        0x0D => cpu.ora_abs(bus),
        0x1D => cpu.ora_absx(bus),
        0x19 => cpu.ora_absy(bus),
        0x01 => cpu.ora_izx(bus),
        0x11 => cpu.ora_izy(bus),

        0x49 => cpu.eor_imm(bus),
        0x45 => cpu.eor_zp(bus),
        0x55 => cpu.eor_zpx(bus),
        0x4D => cpu.eor_abs(bus),
        0x5D => cpu.eor_absx(bus),
        0x59 => cpu.eor_absy(bus),
        0x41 => cpu.eor_izx(bus),
        0x51 => cpu.eor_izy(bus),

        0xC9 => cpu.cmp_imm(bus),
        0xC5 => cpu.cmp_zp(bus),
        0xD5 => cpu.cmp_zpx(bus),
        0xCD => cpu.cmp_abs(bus),
        0xDD => cpu.cmp_absx(bus),
        0xD9 => cpu.cmp_absy(bus),
        0xC1 => cpu.cmp_izx(bus),
        0xD1 => cpu.cmp_izy(bus),

        // ---- CPX / CPY ----
        0xE0 => cpu.cpx_imm(bus),
        0xE4 => cpu.cpx_zp(bus),
        0xEC => cpu.cpx_abs(bus),
        0xC0 => cpu.cpy_imm(bus),
        0xC4 => cpu.cpy_zp(bus),
        0xCC => cpu.cpy_abs(bus),

        // ---- Branches ----
        0x90 => cpu.bcc(bus),
        0xB0 => cpu.bcs(bus),
        0xF0 => cpu.beq(bus),
        0x30 => cpu.bmi(bus),
        0xD0 => cpu.bne(bus),
        0x10 => cpu.bpl(bus),
        0x50 => cpu.bvc(bus),
        0x70 => cpu.bvs(bus),

        // ---- Transfers ----
        0xAA => cpu.tax(),
        0x8A => cpu.txa(),
        0xA8 => cpu.tay(),
        0x98 => cpu.tya(),
        0xBA => cpu.tsx(),
        0x9A => cpu.txs(),

        // ---- Stack ----
        0x48 => cpu.pha(bus),
        0x08 => cpu.php(bus),
        0x68 => cpu.pla(bus),
        0x28 => cpu.plp(bus),
        0x20 => cpu.jsr(bus),
        0x60 => cpu.rts(bus),
        0x40 => cpu.rti(bus),
        0x4C => cpu.jmp_abs(bus),
        0x6C => cpu.jmp_ind(bus),

        // ---- Increment / decrement ----
        0xE6 => cpu.inc_zp(bus),
        0xF6 => cpu.inc_zpx(bus),
        0xEE => cpu.inc_abs(bus),
        0xFE => cpu.inc_absx(bus),

        0xC6 => cpu.dec_zp(bus),
        0xD6 => cpu.dec_zpx(bus),
        0xCE => cpu.dec_abs(bus),
        0xDE => cpu.dec_absx(bus),

        0xE8 => cpu.inx(),
        0xC8 => cpu.iny(),
        0xCA => cpu.dex(),
        0x88 => cpu.dey(),

        // ---- Shifts ----
        0x0A => cpu.asl_a(),
        0x06 => cpu.asl_zp(bus),
        0x16 => cpu.asl_zpx(bus),
        0x0E => cpu.asl_abs(bus),
        0x1E => cpu.asl_absx(bus),

        0x2A => cpu.rol_a(),
        0x26 => cpu.rol_zp(bus),
        0x36 => cpu.rol_zpx(bus),
        0x2E => cpu.rol_abs(bus),
        0x3E => cpu.rol_absx(bus),

        0x4A => cpu.lsr_a(),
        0x46 => cpu.lsr_zp(bus),
        0x56 => cpu.lsr_zpx(bus),
        0x4E => cpu.lsr_abs(bus),
        0x5E => cpu.lsr_absx(bus),

        0x6A => cpu.ror_a(),
        0x66 => cpu.ror_zp(bus),
        0x76 => cpu.ror_zpx(bus),
        0x6E => cpu.ror_abs(bus),
        0x7E => cpu.ror_absx(bus),

        // ---- Flags ----
        0x18 => cpu.clc(),
        0x38 => cpu.sec(),
        0xD8 => cpu.cld(),
        0xF8 => cpu.sed(),
        0x58 => cpu.cli(),
        0x78 => cpu.sei(),
        0xB8 => cpu.clv(),

        // ---- System ----
        0x00 => cpu.brk(bus),
        0xEA => cpu.nop(),
        0x24 => cpu.bit_zp(bus),
        0x2C => cpu.bit_abs(bus),

        // =========================================================
        // Undocumented (unofficial) opcodes
        // =========================================================
        // LAX (load A & X)
        0xA7 => cpu.lax_zp(bus),
        0xB7 => cpu.lax_zpy(bus),
        0xAF => cpu.lax_abs(bus),
        0xBF => cpu.lax_absy(bus),
        0xA3 => cpu.lax_izx(bus),
        0xB3 => cpu.lax_izy(bus),

        // SAX (store A & X)
        0x87 => cpu.sax_zp(bus),
        0x97 => cpu.sax_zpy(bus),
        0x8F => cpu.sax_abs(bus),
        0x83 => cpu.sax_izx(bus),

        // DCP (DEC + CMP)
        0xC7 => cpu.dcp_zp(bus),
        0xD7 => cpu.dcp_zpx(bus),
        0xCF => cpu.dcp_abs(bus),
        0xDF => cpu.dcp_absx(bus),
        0xDB => cpu.dcp_absy(bus),
        0xC3 => cpu.dcp_izx(bus),
        0xD3 => cpu.dcp_izy(bus),

        // ISB / ISC (INC + SBC)
        0xE7 => cpu.isb_zp(bus),
        0xF7 => cpu.isb_zpx(bus),
        0xEF => cpu.isb_abs(bus),
        0xFF => cpu.isb_absx(bus),
        0xFB => cpu.isb_absy(bus),
        0xE3 => cpu.isb_izx(bus),
        0xF3 => cpu.isb_izy(bus),

        // SLO (ASL + ORA)
        0x07 => cpu.slo_zp(bus),
        0x17 => cpu.slo_zpx(bus),
        0x0F => cpu.slo_abs(bus),
        0x1F => cpu.slo_absx(bus),
        0x1B => cpu.slo_absy(bus),
        0x03 => cpu.slo_izx(bus),
        0x13 => cpu.slo_izy(bus),

        // RLA (ROL + AND)
        0x27 => cpu.rla_zp(bus),
        0x37 => cpu.rla_zpx(bus),
        0x2F => cpu.rla_abs(bus),
        0x3F => cpu.rla_absx(bus),
        0x3B => cpu.rla_absy(bus),
        0x23 => cpu.rla_izx(bus),
        0x33 => cpu.rla_izy(bus),

        // SRE (LSR + EOR)
        0x47 => cpu.sre_zp(bus),
        0x57 => cpu.sre_zpx(bus),
        0x4F => cpu.sre_abs(bus),
        0x5F => cpu.sre_absx(bus),
        0x5B => cpu.sre_absy(bus),
        0x43 => cpu.sre_izx(bus),
        0x53 => cpu.sre_izy(bus),

        // RRA (ROR + ADC)
        0x67 => cpu.rra_zp(bus),
        0x77 => cpu.rra_zpx(bus),
        0x6F => cpu.rra_abs(bus),
        0x7F => cpu.rra_absx(bus),
        0x7B => cpu.rra_absy(bus),
        0x63 => cpu.rra_izx(bus),
        0x73 => cpu.rra_izy(bus),

        // ANC / ARR / XAA / LAS / AXS / LAX-imm / SBC-imm(unofficial)
        0x0B | 0x2B => cpu.anc(bus),
        0x4B | 0x6B => cpu.arr(bus),
        0x8B => cpu.xaa(bus),
        0xBB => cpu.las(bus),
        0xCB => cpu.axs(bus),
        0xAB => cpu.lax_imm(bus),
        0xEB => cpu.sbc_imm(bus),

        // SHA / SHX / SHY / TAS
        0x9F => cpu.sha_absy(bus),
        0x93 => cpu.sha_izy(bus),
        0x9E => cpu.shx_absy(bus),
        0x9C => cpu.shy_absx(bus),
        0x9B => cpu.tas_absy(bus),

        // KIL (jam)
        0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72
        | 0x92 | 0xB2 | 0xD2 | 0xF2 => cpu.kil(),

        // Unofficial NOPs — consume operand bytes, do nothing:
        //   2-byte (1 operand byte): 0x04 0x44 0x64 0x0C 0x14 0x34 0x54
        //     0x74 0xD4 0xF4
        //   3-byte (2 operand bytes): 0x1A 0x3A 0x5A 0x7A 0xDA 0xFA
        //     0x80 0x82 0x89 0xC2 0xE2
        0x04 | 0x44 | 0x64 | 0x0C | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
            cpu.nop_imm(bus); // 1 operand byte
        }
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA | 0x80 | 0x82 | 0x89 | 0xC2 | 0xE2
        | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
            cpu.nop_imm(bus);
            cpu.nop_imm(bus); // 2 operand bytes (3-byte NOP)
        }

        // Fallback (should never happen for a valid 6502 opcode —
        // every 0x00-0xFF is assigned above).
        _ => unreachable!("unhandled opcode 0x{op:02X}"),
    }
}
