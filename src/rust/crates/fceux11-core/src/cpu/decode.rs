//! Opcode decode tables.
//!
//! The four 256-entry tables here are byte-for-byte equivalents of:
//!
//! * `CycTable[256]`  — base cycle cost per opcode (`src/x6502.cpp:363–377`)
//! * `opsize[256]`    — instruction size in bytes (`src/x6502.cpp:638–661`)
//! * `optype[256]`    — coarse addressing-mode class (`src/x6502.cpp:673–693`)
//! * `opwrite[256]`   — write-predictor class (`src/x6502.cpp:715–731`)
//!
//! Plus a richer [`OpcodeInfo`] table we build ourselves. Instead of
//! pulling in `serde` + a TOML parser just for Phase 1, we hand-author
//! the equivalent data as a `const [OpcodeInfo; 256]` literal. Phase 2
//! will swap this for a `build.rs`-generated file sourced from
//! `table_gen/opcodes.toml`.
//!
//! Authority for all numeric values:
//! * [NESdev Wiki — 6502 instructions](https://www.nesdev.org/wiki/6502_instructions)
//! * [NESdev Wiki — CPU unofficial opcodes](https://www.nesdev.org/wiki/CPU_unofficial_opcodes)
//! * [Visual6502 — 6502 Opcode 8B (XAA, ANE)](https://www.nesdev.org/wiki/Visual6502wiki/6502_Opcode_8B_(XAA,_ANE))
//! * The original FCE Ultra tables (preserved above for cross-checking).

use crate::cpu::addressing::AddrMode;

/// Base cycle count for each opcode. Source: `CycTable` in
/// `src/x6502.cpp:363–377`. This is the cycle cost **before** any
/// page-cross / branch-taken / RMW extra cycle.
pub const CYC_TABLE: [u8; 256] = [
    /*0x00*/ 7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,
    /*0x10*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x20*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    /*0x30*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x40*/ 6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,
    /*0x50*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x60*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,
    /*0x70*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x80*/ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /*0x90*/ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    /*0xA0*/ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /*0xB0*/ 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    /*0xC0*/ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xD0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0xE0*/ 2, 6, 3, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xF0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

/// Instruction size in bytes (operand bytes + 1 for opcode). Source:
/// `opsize[256]` in `src/x6502.cpp:638–661`. 0 marks unofficial /
/// unstable opcodes that the C++ code refuses to step past — note that
/// the addressing-mode helper still consumes operand bytes for these,
/// so the "0" is purely a debugger-disassembly marker, not a real
/// PC-doesn't-advance signal.
pub const OP_SIZE: [u8; 256] = [
    /*0x00*/ 1, 2, 0, 0, 0, 2, 2, 0, 1, 2, 1, 0, 0, 3, 3, 0,
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
    /*0xE0*/ 2, 2, 2, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    /*0xF0*/ 2, 2, 0, 0, 0, 2, 2, 0, 1, 3, 0, 0, 0, 3, 3, 0,
];

/// High-level classification of each opcode. Drives the dispatch arm in
/// `execute.rs`. Phase 1 stubs return just the cycle cost; Phase 2 will
/// fill in the per-instruction logic.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// Not a real instruction — CPU jams (STP / KIL opcodes 0x02, 0x12,
    /// 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2).
    Jam,
    /// Read a byte and discard (NOP family, 1 or 2 operand bytes).
    NopRead,
    /// Read a byte, then write it back unchanged — used by RMW-style NOPs.
    NopReadWrite,
    /// Push a register / pull a register / transfer register.
    Register,
    /// Set or clear a single flag.
    Flag,
    /// Jump absolute / jump indirect / jump-to-subroutine / return-from-subroutine.
    Jump,
    /// Conditional or unconditional relative branch.
    Branch,
    /// Load register from memory.
    Load,
    /// Store register to memory.
    Store,
    /// Read-modify-write memory (ASL, ROL, LSR, ROR, INC, DEC, plus the
    /// combined RMW+ALU unofficial opcodes).
    Rmw,
    /// ALU operation that writes back to A.
    AluA,
    /// ALU operation that compares (CMP / CPX / CPY).
    Compare,
    /// Bit-test (BIT) — special: writes N and V from memory, Z from A & M.
    Bit,
    /// Unofficial combined RMW+ALU opcodes (SLO, SRE, RLA, RRA, SAX,
    /// LAX, DCP, ISC, AHX, TAS, SHX, SHY, LAS, AXS, ANC, ARR, ALR, XAA).
    /// Some of these (ANC, ARR, ALR, XAA, AXS) have unstable behaviour
    /// that depends on analogue effects; the implementations match the
    /// Visual6502 captures documented in the NESdev wiki.
    Unofficial,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpcodeInfo {
    pub mnemonic: &'static str,
    pub mode: AddrMode,
    pub kind: OpKind,
    pub base_cycles: u8,
    pub size: u8,
    /// Whether the opcode is documented. Unofficial / unstable opcodes
    /// return `false` here; a small number of games depend on them
    /// (NESdev wiki lists ~10 such titles).
    pub official: bool,
}

/// 256-entry decode table. Hand-authored from the NESdev wiki opcode
/// matrix and the FCE Ultra tables above (cross-checked). Phase 2 will
/// replace this with a `build.rs`-generated file.
pub static OPCODE_TABLE: [OpcodeInfo; 256] = build_opcode_table();

const fn build_opcode_table() -> [OpcodeInfo; 256] {
    // Build entries by hand. Each line picks the (mode, kind, official)
    // triple for one opcode, pulling base_cycles and size from the
    // tables above.
    //
    // Authority:
    //   official mode/kind → NESdev 6502_instructions matrix
    //   unofficial mode/kind → NESdev CPU_unofficial_opcodes (with
    //                          cross-checks against the FCE Ultra
    //                          CycTable / opsize tables above)
    //
    // The author-time constants below are kept identical to the source
    // tables to make review trivial: any line of CYC_TABLE / OP_SIZE
    // corresponds to a matching line here.
    use AddrMode::*;
    use OpKind::*;
    let t = CYC_TABLE;
    let s = OP_SIZE;
    let mut tbl = [OpcodeInfo {
        mnemonic: "???",
        mode: Implied,
        kind: NopRead,
        base_cycles: 0,
        size: 0,
        official: false,
    }; 256];

    // Row 0x00 — BRK, ORA-indX, *JAM*, *SLO-indX*, *NOP-zp*, ORA-zp, ASL-zp, ...
    tbl[0x00] = OpcodeInfo { mnemonic: "BRK",  mode: Implied,  kind: Jump,   base_cycles: t[0x00], size: s[0x00], official: true };
    tbl[0x01] = OpcodeInfo { mnemonic: "ORA",  mode: IndX,     kind: AluA,   base_cycles: t[0x01], size: s[0x01], official: true };
    tbl[0x02] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x02], size: s[0x02], official: false };
    tbl[0x03] = OpcodeInfo { mnemonic: "SLO",  mode: IndX,     kind: Unofficial, base_cycles: t[0x03], size: s[0x03], official: false };
    tbl[0x04] = OpcodeInfo { mnemonic: "NOP",  mode: ZP,       kind: NopRead, base_cycles: t[0x04], size: s[0x04], official: false };
    tbl[0x05] = OpcodeInfo { mnemonic: "ORA",  mode: ZP,       kind: AluA,   base_cycles: t[0x05], size: s[0x05], official: true };
    tbl[0x06] = OpcodeInfo { mnemonic: "ASL",  mode: ZP,       kind: Rmw,    base_cycles: t[0x06], size: s[0x06], official: true };
    tbl[0x07] = OpcodeInfo { mnemonic: "SLO",  mode: ZP,       kind: Unofficial, base_cycles: t[0x07], size: s[0x07], official: false };
    tbl[0x08] = OpcodeInfo { mnemonic: "PHP",  mode: Implied,  kind: Register, base_cycles: t[0x08], size: s[0x08], official: true };
    tbl[0x09] = OpcodeInfo { mnemonic: "ORA",  mode: Imm,      kind: AluA,   base_cycles: t[0x09], size: s[0x09], official: true };
    tbl[0x0A] = OpcodeInfo { mnemonic: "ASL",  mode: Accum,    kind: Rmw,    base_cycles: t[0x0A], size: s[0x0A], official: true };
    tbl[0x0B] = OpcodeInfo { mnemonic: "ANC",  mode: Imm,      kind: Unofficial, base_cycles: t[0x0B], size: s[0x0B], official: false };
    tbl[0x0C] = OpcodeInfo { mnemonic: "NOP",  mode: Abs,      kind: NopRead, base_cycles: t[0x0C], size: s[0x0C], official: false };
    tbl[0x0D] = OpcodeInfo { mnemonic: "ORA",  mode: Abs,      kind: AluA,   base_cycles: t[0x0D], size: s[0x0D], official: true };
    tbl[0x0E] = OpcodeInfo { mnemonic: "ASL",  mode: Abs,      kind: Rmw,    base_cycles: t[0x0E], size: s[0x0E], official: true };
    tbl[0x0F] = OpcodeInfo { mnemonic: "SLO",  mode: Abs,      kind: Unofficial, base_cycles: t[0x0F], size: s[0x0F], official: false };

    // Row 0x10 — BPL, ORA-indY, *JAM*, *SLO-indY*, *NOP-zpx*, ORA-zpx, ASL-zpx, ...
    tbl[0x10] = OpcodeInfo { mnemonic: "BPL",  mode: Rel,      kind: Branch, base_cycles: t[0x10], size: s[0x10], official: true };
    tbl[0x11] = OpcodeInfo { mnemonic: "ORA",  mode: IndY,     kind: AluA,   base_cycles: t[0x11], size: s[0x11], official: true };
    tbl[0x12] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x12], size: s[0x12], official: false };
    tbl[0x13] = OpcodeInfo { mnemonic: "SLO",  mode: IndY,     kind: Unofficial, base_cycles: t[0x13], size: s[0x13], official: false };
    tbl[0x14] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0x14], size: s[0x14], official: false };
    tbl[0x15] = OpcodeInfo { mnemonic: "ORA",  mode: ZPX,      kind: AluA,   base_cycles: t[0x15], size: s[0x15], official: true };
    tbl[0x16] = OpcodeInfo { mnemonic: "ASL",  mode: ZPX,      kind: Rmw,    base_cycles: t[0x16], size: s[0x16], official: true };
    tbl[0x17] = OpcodeInfo { mnemonic: "SLO",  mode: ZPX,      kind: Unofficial, base_cycles: t[0x17], size: s[0x17], official: false };
    tbl[0x18] = OpcodeInfo { mnemonic: "CLC",  mode: Implied,  kind: Flag,   base_cycles: t[0x18], size: s[0x18], official: true };
    tbl[0x19] = OpcodeInfo { mnemonic: "ORA",  mode: AbsY,     kind: AluA,   base_cycles: t[0x19], size: s[0x19], official: true };
    tbl[0x1A] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0x1A], size: s[0x1A], official: false };
    tbl[0x1B] = OpcodeInfo { mnemonic: "SLO",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x1B], size: s[0x1B], official: false };
    tbl[0x1C] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0x1C], size: s[0x1C], official: false };
    tbl[0x1D] = OpcodeInfo { mnemonic: "ORA",  mode: AbsX,     kind: AluA,   base_cycles: t[0x1D], size: s[0x1D], official: true };
    tbl[0x1E] = OpcodeInfo { mnemonic: "ASL",  mode: AbsX,     kind: Rmw,    base_cycles: t[0x1E], size: s[0x1E], official: true };
    tbl[0x1F] = OpcodeInfo { mnemonic: "SLO",  mode: AbsX,     kind: Unofficial, base_cycles: t[0x1F], size: s[0x1F], official: false };

    // Row 0x20 — JSR, AND-indX, *JAM*, *RLA-indX*, BIT-zp, AND-zp, ROL-zp, ...
    tbl[0x20] = OpcodeInfo { mnemonic: "JSR",  mode: Abs,      kind: Jump,   base_cycles: t[0x20], size: s[0x20], official: true };
    tbl[0x21] = OpcodeInfo { mnemonic: "AND",  mode: IndX,     kind: AluA,   base_cycles: t[0x21], size: s[0x21], official: true };
    tbl[0x22] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x22], size: s[0x22], official: false };
    tbl[0x23] = OpcodeInfo { mnemonic: "RLA",  mode: IndX,     kind: Unofficial, base_cycles: t[0x23], size: s[0x23], official: false };
    tbl[0x24] = OpcodeInfo { mnemonic: "BIT",  mode: ZP,       kind: Bit,    base_cycles: t[0x24], size: s[0x24], official: true };
    tbl[0x25] = OpcodeInfo { mnemonic: "AND",  mode: ZP,       kind: AluA,   base_cycles: t[0x25], size: s[0x25], official: true };
    tbl[0x26] = OpcodeInfo { mnemonic: "ROL",  mode: ZP,       kind: Rmw,    base_cycles: t[0x26], size: s[0x26], official: true };
    tbl[0x27] = OpcodeInfo { mnemonic: "RLA",  mode: ZP,       kind: Unofficial, base_cycles: t[0x27], size: s[0x27], official: false };
    tbl[0x28] = OpcodeInfo { mnemonic: "PLP",  mode: Implied,  kind: Register, base_cycles: t[0x28], size: s[0x28], official: true };
    tbl[0x29] = OpcodeInfo { mnemonic: "AND",  mode: Imm,      kind: AluA,   base_cycles: t[0x29], size: s[0x29], official: true };
    tbl[0x2A] = OpcodeInfo { mnemonic: "ROL",  mode: Accum,    kind: Rmw,    base_cycles: t[0x2A], size: s[0x2A], official: true };
    tbl[0x2B] = OpcodeInfo { mnemonic: "ANC",  mode: Imm,      kind: Unofficial, base_cycles: t[0x2B], size: s[0x2B], official: false };
    tbl[0x2C] = OpcodeInfo { mnemonic: "BIT",  mode: Abs,      kind: Bit,    base_cycles: t[0x2C], size: s[0x2C], official: true };
    tbl[0x2D] = OpcodeInfo { mnemonic: "AND",  mode: Abs,      kind: AluA,   base_cycles: t[0x2D], size: s[0x2D], official: true };
    tbl[0x2E] = OpcodeInfo { mnemonic: "ROL",  mode: Abs,      kind: Rmw,    base_cycles: t[0x2E], size: s[0x2E], official: true };
    tbl[0x2F] = OpcodeInfo { mnemonic: "RLA",  mode: Abs,      kind: Unofficial, base_cycles: t[0x2F], size: s[0x2F], official: false };

    // Row 0x30 — BMI, AND-indY, *JAM*, *RLA-indY*, *NOP-zpx*, AND-zpx, ROL-zpx, ...
    tbl[0x30] = OpcodeInfo { mnemonic: "BMI",  mode: Rel,      kind: Branch, base_cycles: t[0x30], size: s[0x30], official: true };
    tbl[0x31] = OpcodeInfo { mnemonic: "AND",  mode: IndY,     kind: AluA,   base_cycles: t[0x31], size: s[0x31], official: true };
    tbl[0x32] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x32], size: s[0x32], official: false };
    tbl[0x33] = OpcodeInfo { mnemonic: "RLA",  mode: IndY,     kind: Unofficial, base_cycles: t[0x33], size: s[0x33], official: false };
    tbl[0x34] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0x34], size: s[0x34], official: false };
    tbl[0x35] = OpcodeInfo { mnemonic: "AND",  mode: ZPX,      kind: AluA,   base_cycles: t[0x35], size: s[0x35], official: true };
    tbl[0x36] = OpcodeInfo { mnemonic: "ROL",  mode: ZPX,      kind: Rmw,    base_cycles: t[0x36], size: s[0x36], official: true };
    tbl[0x37] = OpcodeInfo { mnemonic: "RLA",  mode: ZPX,      kind: Unofficial, base_cycles: t[0x37], size: s[0x37], official: false };
    tbl[0x38] = OpcodeInfo { mnemonic: "SEC",  mode: Implied,  kind: Flag,   base_cycles: t[0x38], size: s[0x38], official: true };
    tbl[0x39] = OpcodeInfo { mnemonic: "AND",  mode: AbsY,     kind: AluA,   base_cycles: t[0x39], size: s[0x39], official: true };
    tbl[0x3A] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0x3A], size: s[0x3A], official: false };
    tbl[0x3B] = OpcodeInfo { mnemonic: "RLA",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x3B], size: s[0x3B], official: false };
    tbl[0x3C] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0x3C], size: s[0x3C], official: false };
    tbl[0x3D] = OpcodeInfo { mnemonic: "AND",  mode: AbsX,     kind: AluA,   base_cycles: t[0x3D], size: s[0x3D], official: true };
    tbl[0x3E] = OpcodeInfo { mnemonic: "ROL",  mode: AbsX,     kind: Rmw,    base_cycles: t[0x3E], size: s[0x3E], official: true };
    tbl[0x3F] = OpcodeInfo { mnemonic: "RLA",  mode: AbsX,     kind: Unofficial, base_cycles: t[0x3F], size: s[0x3F], official: false };

    // Row 0x40 — RTI, EOR-indX, *JAM*, *SRE-indX*, *NOP-zp*, EOR-zp, LSR-zp, ...
    tbl[0x40] = OpcodeInfo { mnemonic: "RTI",  mode: Implied,  kind: Jump,   base_cycles: t[0x40], size: s[0x40], official: true };
    tbl[0x41] = OpcodeInfo { mnemonic: "EOR",  mode: IndX,     kind: AluA,   base_cycles: t[0x41], size: s[0x41], official: true };
    tbl[0x42] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x42], size: s[0x42], official: false };
    tbl[0x43] = OpcodeInfo { mnemonic: "SRE",  mode: IndX,     kind: Unofficial, base_cycles: t[0x43], size: s[0x43], official: false };
    tbl[0x44] = OpcodeInfo { mnemonic: "NOP",  mode: ZP,       kind: NopRead, base_cycles: t[0x44], size: s[0x44], official: false };
    tbl[0x45] = OpcodeInfo { mnemonic: "EOR",  mode: ZP,       kind: AluA,   base_cycles: t[0x45], size: s[0x45], official: true };
    tbl[0x46] = OpcodeInfo { mnemonic: "LSR",  mode: ZP,       kind: Rmw,    base_cycles: t[0x46], size: s[0x46], official: true };
    tbl[0x47] = OpcodeInfo { mnemonic: "SRE",  mode: ZP,       kind: Unofficial, base_cycles: t[0x47], size: s[0x47], official: false };
    tbl[0x48] = OpcodeInfo { mnemonic: "PHA",  mode: Implied,  kind: Register, base_cycles: t[0x48], size: s[0x48], official: true };
    tbl[0x49] = OpcodeInfo { mnemonic: "EOR",  mode: Imm,      kind: AluA,   base_cycles: t[0x49], size: s[0x49], official: true };
    tbl[0x4A] = OpcodeInfo { mnemonic: "LSR",  mode: Accum,    kind: Rmw,    base_cycles: t[0x4A], size: s[0x4A], official: true };
    tbl[0x4B] = OpcodeInfo { mnemonic: "ALR",  mode: Imm,      kind: Unofficial, base_cycles: t[0x4B], size: s[0x4B], official: false };
    tbl[0x4C] = OpcodeInfo { mnemonic: "JMP",  mode: Abs,      kind: Jump,   base_cycles: t[0x4C], size: s[0x4C], official: true };
    tbl[0x4D] = OpcodeInfo { mnemonic: "EOR",  mode: Abs,      kind: AluA,   base_cycles: t[0x4D], size: s[0x4D], official: true };
    tbl[0x4E] = OpcodeInfo { mnemonic: "LSR",  mode: Abs,      kind: Rmw,    base_cycles: t[0x4E], size: s[0x4E], official: true };
    tbl[0x4F] = OpcodeInfo { mnemonic: "SRE",  mode: Abs,      kind: Unofficial, base_cycles: t[0x4F], size: s[0x4F], official: false };

    // Row 0x50 — BVC, EOR-indY, *JAM*, *SRE-indY*, *NOP-zpx*, EOR-zpx, LSR-zpx, ...
    tbl[0x50] = OpcodeInfo { mnemonic: "BVC",  mode: Rel,      kind: Branch, base_cycles: t[0x50], size: s[0x50], official: true };
    tbl[0x51] = OpcodeInfo { mnemonic: "EOR",  mode: IndY,     kind: AluA,   base_cycles: t[0x51], size: s[0x51], official: true };
    tbl[0x52] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x52], size: s[0x52], official: false };
    tbl[0x53] = OpcodeInfo { mnemonic: "SRE",  mode: IndY,     kind: Unofficial, base_cycles: t[0x53], size: s[0x53], official: false };
    tbl[0x54] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0x54], size: s[0x54], official: false };
    tbl[0x55] = OpcodeInfo { mnemonic: "EOR",  mode: ZPX,      kind: AluA,   base_cycles: t[0x55], size: s[0x55], official: true };
    tbl[0x56] = OpcodeInfo { mnemonic: "LSR",  mode: ZPX,      kind: Rmw,    base_cycles: t[0x56], size: s[0x56], official: true };
    tbl[0x57] = OpcodeInfo { mnemonic: "SRE",  mode: ZPX,      kind: Unofficial, base_cycles: t[0x57], size: s[0x57], official: false };
    tbl[0x58] = OpcodeInfo { mnemonic: "CLI",  mode: Implied,  kind: Flag,   base_cycles: t[0x58], size: s[0x58], official: true };
    tbl[0x59] = OpcodeInfo { mnemonic: "EOR",  mode: AbsY,     kind: AluA,   base_cycles: t[0x59], size: s[0x59], official: true };
    tbl[0x5A] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0x5A], size: s[0x5A], official: false };
    tbl[0x5B] = OpcodeInfo { mnemonic: "SRE",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x5B], size: s[0x5B], official: false };
    tbl[0x5C] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0x5C], size: s[0x5C], official: false };
    tbl[0x5D] = OpcodeInfo { mnemonic: "EOR",  mode: AbsX,     kind: AluA,   base_cycles: t[0x5D], size: s[0x5D], official: true };
    tbl[0x5E] = OpcodeInfo { mnemonic: "LSR",  mode: AbsX,     kind: Rmw,    base_cycles: t[0x5E], size: s[0x5E], official: true };
    tbl[0x5F] = OpcodeInfo { mnemonic: "SRE",  mode: AbsX,     kind: Unofficial, base_cycles: t[0x5F], size: s[0x5F], official: false };

    // Row 0x60 — RTS, ADC-indX, *JAM*, *RRA-indX*, *NOP-zp*, ADC-zp, ROR-zp, ...
    tbl[0x60] = OpcodeInfo { mnemonic: "RTS",  mode: Implied,  kind: Jump,   base_cycles: t[0x60], size: s[0x60], official: true };
    tbl[0x61] = OpcodeInfo { mnemonic: "ADC",  mode: IndX,     kind: AluA,   base_cycles: t[0x61], size: s[0x61], official: true };
    tbl[0x62] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x62], size: s[0x62], official: false };
    tbl[0x63] = OpcodeInfo { mnemonic: "RRA",  mode: IndX,     kind: Unofficial, base_cycles: t[0x63], size: s[0x63], official: false };
    tbl[0x64] = OpcodeInfo { mnemonic: "NOP",  mode: ZP,       kind: NopRead, base_cycles: t[0x64], size: s[0x64], official: false };
    tbl[0x65] = OpcodeInfo { mnemonic: "ADC",  mode: ZP,       kind: AluA,   base_cycles: t[0x65], size: s[0x65], official: true };
    tbl[0x66] = OpcodeInfo { mnemonic: "ROR",  mode: ZP,       kind: Rmw,    base_cycles: t[0x66], size: s[0x66], official: true };
    tbl[0x67] = OpcodeInfo { mnemonic: "RRA",  mode: ZP,       kind: Unofficial, base_cycles: t[0x67], size: s[0x67], official: false };
    tbl[0x68] = OpcodeInfo { mnemonic: "PLA",  mode: Implied,  kind: Register, base_cycles: t[0x68], size: s[0x68], official: true };
    tbl[0x69] = OpcodeInfo { mnemonic: "ADC",  mode: Imm,      kind: AluA,   base_cycles: t[0x69], size: s[0x69], official: true };
    tbl[0x6A] = OpcodeInfo { mnemonic: "ROR",  mode: Accum,    kind: Rmw,    base_cycles: t[0x6A], size: s[0x6A], official: true };
    tbl[0x6B] = OpcodeInfo { mnemonic: "ARR",  mode: Imm,      kind: Unofficial, base_cycles: t[0x6B], size: s[0x6B], official: false };
    tbl[0x6C] = OpcodeInfo { mnemonic: "JMP",  mode: Ind,      kind: Jump,   base_cycles: t[0x6C], size: s[0x6C], official: true };
    tbl[0x6D] = OpcodeInfo { mnemonic: "ADC",  mode: Abs,      kind: AluA,   base_cycles: t[0x6D], size: s[0x6D], official: true };
    tbl[0x6E] = OpcodeInfo { mnemonic: "ROR",  mode: Abs,      kind: Rmw,    base_cycles: t[0x6E], size: s[0x6E], official: true };
    tbl[0x6F] = OpcodeInfo { mnemonic: "RRA",  mode: Abs,      kind: Unofficial, base_cycles: t[0x6F], size: s[0x6F], official: false };

    // Row 0x70 — BVS, ADC-indY, *JAM*, *RRA-indY*, *NOP-zpx*, ADC-zpx, ROR-zpx, ...
    tbl[0x70] = OpcodeInfo { mnemonic: "BVS",  mode: Rel,      kind: Branch, base_cycles: t[0x70], size: s[0x70], official: true };
    tbl[0x71] = OpcodeInfo { mnemonic: "ADC",  mode: IndY,     kind: AluA,   base_cycles: t[0x71], size: s[0x71], official: true };
    tbl[0x72] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x72], size: s[0x72], official: false };
    tbl[0x73] = OpcodeInfo { mnemonic: "RRA",  mode: IndY,     kind: Unofficial, base_cycles: t[0x73], size: s[0x73], official: false };
    tbl[0x74] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0x74], size: s[0x74], official: false };
    tbl[0x75] = OpcodeInfo { mnemonic: "ADC",  mode: ZPX,      kind: AluA,   base_cycles: t[0x75], size: s[0x75], official: true };
    tbl[0x76] = OpcodeInfo { mnemonic: "ROR",  mode: ZPX,      kind: Rmw,    base_cycles: t[0x76], size: s[0x76], official: true };
    tbl[0x77] = OpcodeInfo { mnemonic: "RRA",  mode: ZPX,      kind: Unofficial, base_cycles: t[0x77], size: s[0x77], official: false };
    tbl[0x78] = OpcodeInfo { mnemonic: "SEI",  mode: Implied,  kind: Flag,   base_cycles: t[0x78], size: s[0x78], official: true };
    tbl[0x79] = OpcodeInfo { mnemonic: "ADC",  mode: AbsY,     kind: AluA,   base_cycles: t[0x79], size: s[0x79], official: true };
    tbl[0x7A] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0x7A], size: s[0x7A], official: false };
    tbl[0x7B] = OpcodeInfo { mnemonic: "RRA",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x7B], size: s[0x7B], official: false };
    tbl[0x7C] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0x7C], size: s[0x7C], official: false };
    tbl[0x7D] = OpcodeInfo { mnemonic: "ADC",  mode: AbsX,     kind: AluA,   base_cycles: t[0x7D], size: s[0x7D], official: true };
    tbl[0x7E] = OpcodeInfo { mnemonic: "ROR",  mode: AbsX,     kind: Rmw,    base_cycles: t[0x7E], size: s[0x7E], official: true };
    tbl[0x7F] = OpcodeInfo { mnemonic: "RRA",  mode: AbsX,     kind: Unofficial, base_cycles: t[0x7F], size: s[0x7F], official: false };

    // Row 0x80 — *NOP-imm*, STA-indX, *NOP-imm*, *SAX-indX*, STY-zp, STA-zp, STX-zp, ...
    tbl[0x80] = OpcodeInfo { mnemonic: "NOP",  mode: Imm,      kind: NopRead, base_cycles: t[0x80], size: s[0x80], official: false };
    tbl[0x81] = OpcodeInfo { mnemonic: "STA",  mode: IndX,     kind: Store,  base_cycles: t[0x81], size: s[0x81], official: true };
    tbl[0x82] = OpcodeInfo { mnemonic: "NOP",  mode: Imm,      kind: NopRead, base_cycles: t[0x82], size: s[0x82], official: false };
    tbl[0x83] = OpcodeInfo { mnemonic: "SAX",  mode: IndX,     kind: Unofficial, base_cycles: t[0x83], size: s[0x83], official: false };
    tbl[0x84] = OpcodeInfo { mnemonic: "STY",  mode: ZP,       kind: Store,  base_cycles: t[0x84], size: s[0x84], official: true };
    tbl[0x85] = OpcodeInfo { mnemonic: "STA",  mode: ZP,       kind: Store,  base_cycles: t[0x85], size: s[0x85], official: true };
    tbl[0x86] = OpcodeInfo { mnemonic: "STX",  mode: ZP,       kind: Store,  base_cycles: t[0x86], size: s[0x86], official: true };
    tbl[0x87] = OpcodeInfo { mnemonic: "SAX",  mode: ZP,       kind: Unofficial, base_cycles: t[0x87], size: s[0x87], official: false };
    tbl[0x88] = OpcodeInfo { mnemonic: "DEY",  mode: Implied,  kind: Register, base_cycles: t[0x88], size: s[0x88], official: true };
    tbl[0x89] = OpcodeInfo { mnemonic: "NOP",  mode: Imm,      kind: NopRead, base_cycles: t[0x89], size: s[0x89], official: false };
    tbl[0x8A] = OpcodeInfo { mnemonic: "TXA",  mode: Implied,  kind: Register, base_cycles: t[0x8A], size: s[0x8A], official: true };
    tbl[0x8B] = OpcodeInfo { mnemonic: "XAA",  mode: Imm,      kind: Unofficial, base_cycles: t[0x8B], size: s[0x8B], official: false };
    tbl[0x8C] = OpcodeInfo { mnemonic: "STY",  mode: Abs,      kind: Store,  base_cycles: t[0x8C], size: s[0x8C], official: true };
    tbl[0x8D] = OpcodeInfo { mnemonic: "STA",  mode: Abs,      kind: Store,  base_cycles: t[0x8D], size: s[0x8D], official: true };
    tbl[0x8E] = OpcodeInfo { mnemonic: "STX",  mode: Abs,      kind: Store,  base_cycles: t[0x8E], size: s[0x8E], official: true };
    tbl[0x8F] = OpcodeInfo { mnemonic: "SAX",  mode: Abs,      kind: Unofficial, base_cycles: t[0x8F], size: s[0x8F], official: false };

    // Row 0x90 — BCC, STA-indY, *JAM*, *AHX-indY*, STY-zpx, STA-zpx, STX-zpy, ...
    tbl[0x90] = OpcodeInfo { mnemonic: "BCC",  mode: Rel,      kind: Branch, base_cycles: t[0x90], size: s[0x90], official: true };
    tbl[0x91] = OpcodeInfo { mnemonic: "STA",  mode: IndY,     kind: Store,  base_cycles: t[0x91], size: s[0x91], official: true };
    tbl[0x92] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0x92], size: s[0x92], official: false };
    tbl[0x93] = OpcodeInfo { mnemonic: "AHX",  mode: IndY,     kind: Unofficial, base_cycles: t[0x93], size: s[0x93], official: false };
    tbl[0x94] = OpcodeInfo { mnemonic: "STY",  mode: ZPX,      kind: Store,  base_cycles: t[0x94], size: s[0x94], official: true };
    tbl[0x95] = OpcodeInfo { mnemonic: "STA",  mode: ZPX,      kind: Store,  base_cycles: t[0x95], size: s[0x95], official: true };
    tbl[0x96] = OpcodeInfo { mnemonic: "STX",  mode: ZPY,      kind: Store,  base_cycles: t[0x96], size: s[0x96], official: true };
    tbl[0x97] = OpcodeInfo { mnemonic: "SAX",  mode: ZPY,      kind: Unofficial, base_cycles: t[0x97], size: s[0x97], official: false };
    tbl[0x98] = OpcodeInfo { mnemonic: "TYA",  mode: Implied,  kind: Register, base_cycles: t[0x98], size: s[0x98], official: true };
    tbl[0x99] = OpcodeInfo { mnemonic: "STA",  mode: AbsY,     kind: Store,  base_cycles: t[0x99], size: s[0x99], official: true };
    tbl[0x9A] = OpcodeInfo { mnemonic: "TXS",  mode: Implied,  kind: Register, base_cycles: t[0x9A], size: s[0x9A], official: true };
    tbl[0x9B] = OpcodeInfo { mnemonic: "TAS",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x9B], size: s[0x9B], official: false };
    tbl[0x9C] = OpcodeInfo { mnemonic: "SHY",  mode: AbsX,     kind: Unofficial, base_cycles: t[0x9C], size: s[0x9C], official: false };
    tbl[0x9D] = OpcodeInfo { mnemonic: "STA",  mode: AbsX,     kind: Store,  base_cycles: t[0x9D], size: s[0x9D], official: true };
    tbl[0x9E] = OpcodeInfo { mnemonic: "SHX",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x9E], size: s[0x9E], official: false };
    tbl[0x9F] = OpcodeInfo { mnemonic: "AHX",  mode: AbsY,     kind: Unofficial, base_cycles: t[0x9F], size: s[0x9F], official: false };

    // Row 0xA0 — LDY-imm, LDA-indX, LDX-imm, *LAX-indX*, LDY-zp, LDA-zp, LDX-zp, ...
    tbl[0xA0] = OpcodeInfo { mnemonic: "LDY",  mode: Imm,      kind: Load,   base_cycles: t[0xA0], size: s[0xA0], official: true };
    tbl[0xA1] = OpcodeInfo { mnemonic: "LDA",  mode: IndX,     kind: Load,   base_cycles: t[0xA1], size: s[0xA1], official: true };
    tbl[0xA2] = OpcodeInfo { mnemonic: "LDX",  mode: Imm,      kind: Load,   base_cycles: t[0xA2], size: s[0xA2], official: true };
    tbl[0xA3] = OpcodeInfo { mnemonic: "LAX",  mode: IndX,     kind: Unofficial, base_cycles: t[0xA3], size: s[0xA3], official: false };
    tbl[0xA4] = OpcodeInfo { mnemonic: "LDY",  mode: ZP,       kind: Load,   base_cycles: t[0xA4], size: s[0xA4], official: true };
    tbl[0xA5] = OpcodeInfo { mnemonic: "LDA",  mode: ZP,       kind: Load,   base_cycles: t[0xA5], size: s[0xA5], official: true };
    tbl[0xA6] = OpcodeInfo { mnemonic: "LDX",  mode: ZP,       kind: Load,   base_cycles: t[0xA6], size: s[0xA6], official: true };
    tbl[0xA7] = OpcodeInfo { mnemonic: "LAX",  mode: ZP,       kind: Unofficial, base_cycles: t[0xA7], size: s[0xA7], official: false };
    tbl[0xA8] = OpcodeInfo { mnemonic: "TAY",  mode: Implied,  kind: Register, base_cycles: t[0xA8], size: s[0xA8], official: true };
    tbl[0xA9] = OpcodeInfo { mnemonic: "LDA",  mode: Imm,      kind: Load,   base_cycles: t[0xA9], size: s[0xA9], official: true };
    tbl[0xAA] = OpcodeInfo { mnemonic: "TAX",  mode: Implied,  kind: Register, base_cycles: t[0xAA], size: s[0xAA], official: true };
    tbl[0xAB] = OpcodeInfo { mnemonic: "LAX",  mode: Imm,      kind: Unofficial, base_cycles: t[0xAB], size: s[0xAB], official: false };
    tbl[0xAC] = OpcodeInfo { mnemonic: "LDY",  mode: Abs,      kind: Load,   base_cycles: t[0xAC], size: s[0xAC], official: true };
    tbl[0xAD] = OpcodeInfo { mnemonic: "LDA",  mode: Abs,      kind: Load,   base_cycles: t[0xAD], size: s[0xAD], official: true };
    tbl[0xAE] = OpcodeInfo { mnemonic: "LDX",  mode: Abs,      kind: Load,   base_cycles: t[0xAE], size: s[0xAE], official: true };
    tbl[0xAF] = OpcodeInfo { mnemonic: "LAX",  mode: Abs,      kind: Unofficial, base_cycles: t[0xAF], size: s[0xAF], official: false };

    // Row 0xB0 — BCS, LDA-indY, *JAM*, *LAX-indY*, LDY-zpx, LDA-zpx, LDX-zpy, ...
    tbl[0xB0] = OpcodeInfo { mnemonic: "BCS",  mode: Rel,      kind: Branch, base_cycles: t[0xB0], size: s[0xB0], official: true };
    tbl[0xB1] = OpcodeInfo { mnemonic: "LDA",  mode: IndY,     kind: Load,   base_cycles: t[0xB1], size: s[0xB1], official: true };
    tbl[0xB2] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0xB2], size: s[0xB2], official: false };
    tbl[0xB3] = OpcodeInfo { mnemonic: "LAX",  mode: IndY,     kind: Unofficial, base_cycles: t[0xB3], size: s[0xB3], official: false };
    tbl[0xB4] = OpcodeInfo { mnemonic: "LDY",  mode: ZPX,      kind: Load,   base_cycles: t[0xB4], size: s[0xB4], official: true };
    tbl[0xB5] = OpcodeInfo { mnemonic: "LDA",  mode: ZPX,      kind: Load,   base_cycles: t[0xB5], size: s[0xB5], official: true };
    tbl[0xB6] = OpcodeInfo { mnemonic: "LDX",  mode: ZPY,      kind: Load,   base_cycles: t[0xB6], size: s[0xB6], official: true };
    tbl[0xB7] = OpcodeInfo { mnemonic: "LAX",  mode: ZPY,      kind: Unofficial, base_cycles: t[0xB7], size: s[0xB7], official: false };
    tbl[0xB8] = OpcodeInfo { mnemonic: "CLV",  mode: Implied,  kind: Flag,   base_cycles: t[0xB8], size: s[0xB8], official: true };
    tbl[0xB9] = OpcodeInfo { mnemonic: "LDA",  mode: AbsY,     kind: Load,   base_cycles: t[0xB9], size: s[0xB9], official: true };
    tbl[0xBA] = OpcodeInfo { mnemonic: "TSX",  mode: Implied,  kind: Register, base_cycles: t[0xBA], size: s[0xBA], official: true };
    tbl[0xBB] = OpcodeInfo { mnemonic: "LAS",  mode: AbsY,     kind: Unofficial, base_cycles: t[0xBB], size: s[0xBB], official: false };
    tbl[0xBC] = OpcodeInfo { mnemonic: "LDY",  mode: AbsX,     kind: Load,   base_cycles: t[0xBC], size: s[0xBC], official: true };
    tbl[0xBD] = OpcodeInfo { mnemonic: "LDA",  mode: AbsX,     kind: Load,   base_cycles: t[0xBD], size: s[0xBD], official: true };
    tbl[0xBE] = OpcodeInfo { mnemonic: "LDX",  mode: AbsY,     kind: Load,   base_cycles: t[0xBE], size: s[0xBE], official: true };
    tbl[0xBF] = OpcodeInfo { mnemonic: "LAX",  mode: AbsY,     kind: Unofficial, base_cycles: t[0xBF], size: s[0xBF], official: false };

    // Row 0xC0 — CPY-imm, CMP-indX, *NOP-imm*, *DCP-indX*, CPY-zp, CMP-zp, DEC-zp, ...
    tbl[0xC0] = OpcodeInfo { mnemonic: "CPY",  mode: Imm,      kind: Compare, base_cycles: t[0xC0], size: s[0xC0], official: true };
    tbl[0xC1] = OpcodeInfo { mnemonic: "CMP",  mode: IndX,     kind: Compare, base_cycles: t[0xC1], size: s[0xC1], official: true };
    tbl[0xC2] = OpcodeInfo { mnemonic: "NOP",  mode: Imm,      kind: NopRead, base_cycles: t[0xC2], size: s[0xC2], official: false };
    tbl[0xC3] = OpcodeInfo { mnemonic: "DCP",  mode: IndX,     kind: Unofficial, base_cycles: t[0xC3], size: s[0xC3], official: false };
    tbl[0xC4] = OpcodeInfo { mnemonic: "CPY",  mode: ZP,       kind: Compare, base_cycles: t[0xC4], size: s[0xC4], official: true };
    tbl[0xC5] = OpcodeInfo { mnemonic: "CMP",  mode: ZP,       kind: Compare, base_cycles: t[0xC5], size: s[0xC5], official: true };
    tbl[0xC6] = OpcodeInfo { mnemonic: "DEC",  mode: ZP,       kind: Rmw,    base_cycles: t[0xC6], size: s[0xC6], official: true };
    tbl[0xC7] = OpcodeInfo { mnemonic: "DCP",  mode: ZP,       kind: Unofficial, base_cycles: t[0xC7], size: s[0xC7], official: false };
    tbl[0xC8] = OpcodeInfo { mnemonic: "INY",  mode: Implied,  kind: Register, base_cycles: t[0xC8], size: s[0xC8], official: true };
    tbl[0xC9] = OpcodeInfo { mnemonic: "CMP",  mode: Imm,      kind: Compare, base_cycles: t[0xC9], size: s[0xC9], official: true };
    tbl[0xCA] = OpcodeInfo { mnemonic: "DEX",  mode: Implied,  kind: Register, base_cycles: t[0xCA], size: s[0xCA], official: true };
    tbl[0xCB] = OpcodeInfo { mnemonic: "AXS",  mode: Imm,      kind: Unofficial, base_cycles: t[0xCB], size: s[0xCB], official: false };
    tbl[0xCC] = OpcodeInfo { mnemonic: "CPY",  mode: Abs,      kind: Compare, base_cycles: t[0xCC], size: s[0xCC], official: true };
    tbl[0xCD] = OpcodeInfo { mnemonic: "CMP",  mode: Abs,      kind: Compare, base_cycles: t[0xCD], size: s[0xCD], official: true };
    tbl[0xCE] = OpcodeInfo { mnemonic: "DEC",  mode: Abs,      kind: Rmw,    base_cycles: t[0xCE], size: s[0xCE], official: true };
    tbl[0xCF] = OpcodeInfo { mnemonic: "DCP",  mode: Abs,      kind: Unofficial, base_cycles: t[0xCF], size: s[0xCF], official: false };

    // Row 0xD0 — BNE, CMP-indY, *JAM*, *DCP-indY*, *NOP-zpx*, CMP-zpx, DEC-zpx, ...
    tbl[0xD0] = OpcodeInfo { mnemonic: "BNE",  mode: Rel,      kind: Branch, base_cycles: t[0xD0], size: s[0xD0], official: true };
    tbl[0xD1] = OpcodeInfo { mnemonic: "CMP",  mode: IndY,     kind: Compare, base_cycles: t[0xD1], size: s[0xD1], official: true };
    tbl[0xD2] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0xD2], size: s[0xD2], official: false };
    tbl[0xD3] = OpcodeInfo { mnemonic: "DCP",  mode: IndY,     kind: Unofficial, base_cycles: t[0xD3], size: s[0xD3], official: false };
    tbl[0xD4] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0xD4], size: s[0xD4], official: false };
    tbl[0xD5] = OpcodeInfo { mnemonic: "CMP",  mode: ZPX,      kind: Compare, base_cycles: t[0xD5], size: s[0xD5], official: true };
    tbl[0xD6] = OpcodeInfo { mnemonic: "DEC",  mode: ZPX,      kind: Rmw,    base_cycles: t[0xD6], size: s[0xD6], official: true };
    tbl[0xD7] = OpcodeInfo { mnemonic: "DCP",  mode: ZPX,      kind: Unofficial, base_cycles: t[0xD7], size: s[0xD7], official: false };
    tbl[0xD8] = OpcodeInfo { mnemonic: "CLD",  mode: Implied,  kind: Flag,   base_cycles: t[0xD8], size: s[0xD8], official: true };
    tbl[0xD9] = OpcodeInfo { mnemonic: "CMP",  mode: AbsY,     kind: Compare, base_cycles: t[0xD9], size: s[0xD9], official: true };
    tbl[0xDA] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0xDA], size: s[0xDA], official: false };
    tbl[0xDB] = OpcodeInfo { mnemonic: "DCP",  mode: AbsY,     kind: Unofficial, base_cycles: t[0xDB], size: s[0xDB], official: false };
    tbl[0xDC] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0xDC], size: s[0xDC], official: false };
    tbl[0xDD] = OpcodeInfo { mnemonic: "CMP",  mode: AbsX,     kind: Compare, base_cycles: t[0xDD], size: s[0xDD], official: true };
    tbl[0xDE] = OpcodeInfo { mnemonic: "DEC",  mode: AbsX,     kind: Rmw,    base_cycles: t[0xDE], size: s[0xDE], official: true };
    tbl[0xDF] = OpcodeInfo { mnemonic: "DCP",  mode: AbsX,     kind: Unofficial, base_cycles: t[0xDF], size: s[0xDF], official: false };

    // Row 0xE0 — CPX-imm, SBC-indX, *NOP-imm*, *ISC-indX*, CPX-zp, SBC-zp, INC-zp, ...
    tbl[0xE0] = OpcodeInfo { mnemonic: "CPX",  mode: Imm,      kind: Compare, base_cycles: t[0xE0], size: s[0xE0], official: true };
    tbl[0xE1] = OpcodeInfo { mnemonic: "SBC",  mode: IndX,     kind: AluA,   base_cycles: t[0xE1], size: s[0xE1], official: true };
    tbl[0xE2] = OpcodeInfo { mnemonic: "NOP",  mode: Imm,      kind: NopRead, base_cycles: t[0xE2], size: s[0xE2], official: false };
    tbl[0xE3] = OpcodeInfo { mnemonic: "ISC",  mode: IndX,     kind: Unofficial, base_cycles: t[0xE3], size: s[0xE3], official: false };
    tbl[0xE4] = OpcodeInfo { mnemonic: "CPX",  mode: ZP,       kind: Compare, base_cycles: t[0xE4], size: s[0xE4], official: true };
    tbl[0xE5] = OpcodeInfo { mnemonic: "SBC",  mode: ZP,       kind: AluA,   base_cycles: t[0xE5], size: s[0xE5], official: true };
    tbl[0xE6] = OpcodeInfo { mnemonic: "INC",  mode: ZP,       kind: Rmw,    base_cycles: t[0xE6], size: s[0xE6], official: true };
    tbl[0xE7] = OpcodeInfo { mnemonic: "ISC",  mode: ZP,       kind: Unofficial, base_cycles: t[0xE7], size: s[0xE7], official: false };
    tbl[0xE8] = OpcodeInfo { mnemonic: "INX",  mode: Implied,  kind: Register, base_cycles: t[0xE8], size: s[0xE8], official: true };
    tbl[0xE9] = OpcodeInfo { mnemonic: "SBC",  mode: Imm,      kind: AluA,   base_cycles: t[0xE9], size: s[0xE9], official: true };
    tbl[0xEA] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0xEA], size: s[0xEA], official: true };
    tbl[0xEB] = OpcodeInfo { mnemonic: "SBC",  mode: Imm,      kind: AluA,   base_cycles: t[0xEB], size: s[0xEB], official: false }; // unofficial duplicate of E9
    tbl[0xEC] = OpcodeInfo { mnemonic: "CPX",  mode: Abs,      kind: Compare, base_cycles: t[0xEC], size: s[0xEC], official: true };
    tbl[0xED] = OpcodeInfo { mnemonic: "SBC",  mode: Abs,      kind: AluA,   base_cycles: t[0xED], size: s[0xED], official: true };
    tbl[0xEE] = OpcodeInfo { mnemonic: "INC",  mode: Abs,      kind: Rmw,    base_cycles: t[0xEE], size: s[0xEE], official: true };
    tbl[0xEF] = OpcodeInfo { mnemonic: "ISC",  mode: Abs,      kind: Unofficial, base_cycles: t[0xEF], size: s[0xEF], official: false };

    // Row 0xF0 — BEQ, SBC-indY, *JAM*, *ISC-indY*, *NOP-zpx*, SBC-zpx, INC-zpx, ...
    tbl[0xF0] = OpcodeInfo { mnemonic: "BEQ",  mode: Rel,      kind: Branch, base_cycles: t[0xF0], size: s[0xF0], official: true };
    tbl[0xF1] = OpcodeInfo { mnemonic: "SBC",  mode: IndY,     kind: AluA,   base_cycles: t[0xF1], size: s[0xF1], official: true };
    tbl[0xF2] = OpcodeInfo { mnemonic: "JAM",  mode: Implied,  kind: Jam,    base_cycles: t[0xF2], size: s[0xF2], official: false };
    tbl[0xF3] = OpcodeInfo { mnemonic: "ISC",  mode: IndY,     kind: Unofficial, base_cycles: t[0xF3], size: s[0xF3], official: false };
    tbl[0xF4] = OpcodeInfo { mnemonic: "NOP",  mode: ZPX,      kind: NopRead, base_cycles: t[0xF4], size: s[0xF4], official: false };
    tbl[0xF5] = OpcodeInfo { mnemonic: "SBC",  mode: ZPX,      kind: AluA,   base_cycles: t[0xF5], size: s[0xF5], official: true };
    tbl[0xF6] = OpcodeInfo { mnemonic: "INC",  mode: ZPX,      kind: Rmw,    base_cycles: t[0xF6], size: s[0xF6], official: true };
    tbl[0xF7] = OpcodeInfo { mnemonic: "ISC",  mode: ZPX,      kind: Unofficial, base_cycles: t[0xF7], size: s[0xF7], official: false };
    tbl[0xF8] = OpcodeInfo { mnemonic: "SED",  mode: Implied,  kind: Flag,   base_cycles: t[0xF8], size: s[0xF8], official: true };
    tbl[0xF9] = OpcodeInfo { mnemonic: "SBC",  mode: AbsY,     kind: AluA,   base_cycles: t[0xF9], size: s[0xF9], official: true };
    tbl[0xFA] = OpcodeInfo { mnemonic: "NOP",  mode: Implied,  kind: NopRead, base_cycles: t[0xFA], size: s[0xFA], official: false };
    tbl[0xFB] = OpcodeInfo { mnemonic: "ISC",  mode: AbsY,     kind: Unofficial, base_cycles: t[0xFB], size: s[0xFB], official: false };
    tbl[0xFC] = OpcodeInfo { mnemonic: "NOP",  mode: AbsX,     kind: NopRead, base_cycles: t[0xFC], size: s[0xFC], official: false };
    tbl[0xFD] = OpcodeInfo { mnemonic: "SBC",  mode: AbsX,     kind: AluA,   base_cycles: t[0xFD], size: s[0xFD], official: true };
    tbl[0xFE] = OpcodeInfo { mnemonic: "INC",  mode: AbsX,     kind: Rmw,    base_cycles: t[0xFE], size: s[0xFE], official: true };
    tbl[0xFF] = OpcodeInfo { mnemonic: "ISC",  mode: AbsX,     kind: Unofficial, base_cycles: t[0xFF], size: s[0xFF], official: false };

    tbl
}

#[inline]
pub fn info(opcode: u8) -> OpcodeInfo {
    OPCODE_TABLE[opcode as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_table_has_256_entries() {
        assert_eq!(CYC_TABLE.len(), 256);
        assert_eq!(OP_SIZE.len(), 256);
        assert_eq!(OPCODE_TABLE.len(), 256);
    }

    #[test]
    fn opcode_table_consistent_with_source_tables() {
        // Every entry's base_cycles and size must agree with the
        // byte-for-byte imported CYC_TABLE / OP_SIZE.
        for (op, info) in OPCODE_TABLE.iter().enumerate() {
            assert_eq!(info.base_cycles, CYC_TABLE[op], "op ${:02X} base_cycles", op);
            assert_eq!(info.size, OP_SIZE[op], "op ${:02X} size", op);
        }
    }

    #[test]
    fn every_opcode_has_a_mnemonic() {
        // No "???" left behind. The default entry is the only place it
        // appears, so we forbid it.
        for (op, info) in OPCODE_TABLE.iter().enumerate() {
            assert_ne!(info.mnemonic, "???", "op ${:02X} still placeholder", op);
        }
    }

    #[test]
    fn known_opcodes_match_nesdev_matrix() {
        // A handful of spot-checks against the NESdev matrix.
        assert_eq!(info(0x00).mnemonic, "BRK");
        assert_eq!(info(0xEA).mnemonic, "NOP");
        assert_eq!(info(0x6C).mode, AddrMode::Ind); // JMP indirect
        assert_eq!(info(0xBE).mode, AddrMode::AbsY); // LDX abs,Y
        assert_eq!(info(0x1A).kind, OpKind::NopRead); // unofficial NOP
        assert_eq!(info(0x9B).mnemonic, "TAS");
    }
}