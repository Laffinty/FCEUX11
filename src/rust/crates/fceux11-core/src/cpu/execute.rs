//! Top-level instruction execution loop.
//!
//! Phase 2 implements the real per-opcode semantics for all 256 entries
//! of [`crate::cpu::decode::OPCODE_TABLE`]. The dispatch is a single
//! `match` on `OpKind`, with each arm calling the appropriate ALU /
//! branch / jump helper.
//!
//! Gate test (Phase 2): `nestest.nes` PC + register state + cycle
//! trace matches the published `nestest.log` line-by-line for the
//! first 5,000 instructions.

use crate::cpu::addressing::{
    abs_x_read, abs_x_write, abs_y_read, abs_y_write, absolute, imm, implied, ind_x,
    ind_y_read, ind_y_write, indirect, zp, zpx, zpy, AddrMode, Bus, CpuState,
};
use crate::cpu::alu::{
    adc, and, asl, bit, cmp, eor, load_reg, lsr, ora, rol, ror, sbc, LoadReg,
};
use crate::cpu::decode::{info, OpKind};
use crate::cpu::state::{Flags, IrqSource};

/// Cycles per CPU cycle in 1/16-dot units. The C++ loop uses
/// `_count = cycles * (PAL ? 15 : 16)`. We hardcode NTSC = 16.
pub const CYCLES_PER_CPU_CYCLE: i32 = 16;

/// One CPU cycle in dot-clock units. The `CycTable` is in CPU cycles.
#[inline]
const fn dot(cpu_cycles: u8) -> i32 {
    (cpu_cycles as i32) * CYCLES_PER_CPU_CYCLE
}

// ---------------------------------------------------------------------------
// IRQ / RESET / NMI dispatch (Phase 2 minimal version; Phase 3 expands it).
// Mirrors the loop-top of `X6502_RunDebug` in `src/x6502.cpp:519-577`.
// ---------------------------------------------------------------------------

/// Consume any pending IRQ / NMI / RESET at the instruction boundary.
/// Returns the cycle cost of the *dispatch* itself:
/// * `0` if no interrupt was pending
/// * `0` for RESET (the C++ does not `ADDCYC` inside the RESET
///   dispatch — the cycle cost comes from the subsequent
///   instruction that runs at the reset vector)
/// * `0` for the NMI-fresh deferral (deferral eats no cycles, just
///   clears `nmi_fresh` and the pending NMI bit)
/// * `7` for a normal NMI (push 3 bytes, jump via `$FFFA`/`$FFFB`)
/// * `7` for a maskable IRQ (push 3 bytes, jump via `$FFFE`/`$FFFF`)
///
/// Mirrors the loop-top of `X6502_RunDebug` in `src/x6502.cpp:519-577`.
/// The C++ structure is `if-else if-else if-else` with priority
/// RESET > NMI2 > NMI > maskable IRQ; we mirror that exactly.
fn dispatch_irq<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {
    let irq = state.regs.irq_low;
    if irq == 0 {
        return 0;
    }
    if irq & IrqSource::RESET.bits() != 0 {
        let lo = state.rd(bus, 0xFFFC);
        let hi = state.rd(bus, 0xFFFD);
        state.regs.pc = ((hi as u16) << 8) | lo as u16;
        state.regs.jammed = 0;
        let mut p = state.regs.p;
        p &= !(Flags::BREAK.bits() | Flags::UNUSED.bits());
        // Per 6502 datasheet: on RESET the P register is
        // I_FLAG | U_FLAG (the BREAK flag is cleared and U is set).
        // The C++ sets only I_FLAG, but the missing U_FLAG never
        // matters in practice because the RESET handler does not
        // push P; we use the datasheet-correct value to stay safe
        // for any future caller that observes P after RESET.
        p |= Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
        state.regs.p = p;
        state.regs.irq_low &= !IrqSource::RESET.bits();
        state.regs.irq_low &= !IrqSource::TEMP.bits();
        // RESET dispatch itself eats 0 cycles — the cycle cost
        // comes from the next instruction (JMP or whatever the
        // reset vector points at) which step() will fetch+execute
        // in this same iteration. See `X6502_RunDebug` lines
        // 519-535 (RESET branch has no ADDCYC).
        return 0;
    }
    if irq & IrqSource::NMI2.bits() != 0 {
        state.regs.irq_low &= !IrqSource::NMI2.bits();
        state.regs.irq_low |= IrqSource::NMI.bits();
    }
    if irq & IrqSource::NMI.bits() != 0 {
        if state.nmi_fresh {
            // Latch was set at/after this boundary; defer to next boundary.
            // Per `X6502_RunDebug` (src/x6502.cpp:548-555) we clear
            // only `g_e1_nmi_fresh` here, NOT `_IRQlow`'s NMI bit — the
            // dispatch runs at the NEXT boundary against the still-set
            // NMI bit. Clearing the NMI bit on defer (the previous
            // Rust behaviour) makes the NMI never fire, which broke
            // any code path that triggered an NMI between instructions.
            state.nmi_fresh = false;
            return 0;
        }
        if state.regs.jammed == 0 {
            // NMI: push PCH, PCL, P|U (no B), set I, jump via $FFFA.
            let pc = state.regs.pc;
            state.push(bus, (pc >> 8) as u8);
            state.push(bus, pc as u8);
            let push_p = (state.regs.p | Flags::UNUSED.bits()) & !Flags::BREAK.bits();
            state.push(bus, push_p);
            state.regs.p |= Flags::IRQ_DIS.bits();
            let lo = state.rd(bus, 0xFFFA);
            let hi = state.rd(bus, 0xFFFB);
            state.regs.pc = ((hi as u16) << 8) | lo as u16;
            state.regs.irq_low &= !IrqSource::NMI.bits();
            return 7;
        }
    }
    // Maskable IRQ: only if I flag clear and not jammed.
    let pi = state.regs.moo_pi;
    if pi & Flags::IRQ_DIS.bits() == 0 && state.regs.jammed == 0 {
        let pc = state.regs.pc;
        state.push(bus, (pc >> 8) as u8);
        state.push(bus, pc as u8);
        let push_p = (state.regs.p | Flags::UNUSED.bits()) & !Flags::BREAK.bits();
        state.push(bus, push_p);
        state.regs.p |= Flags::IRQ_DIS.bits();
        let lo = state.rd(bus, 0xFFFE);
        let hi = state.rd(bus, 0xFFFF);
        state.regs.pc = ((hi as u16) << 8) | lo as u16;
        state.regs.irq_low &= !IrqSource::TEMP.bits();
        return 7;
    }
    // No dispatch fired at this boundary (I flag blocked the maskable
    // IRQ, or the CPU was jammed, or the only pending source was
    // something we don't model). Return 0 so step() takes the regular
    // fetch+execute path instead of treating a blocked dispatch as a
    // successful 7-cycle push+jump. Mirrors `X6502_RunDebug`'s ADDCYC
    // being inside the `if(!(_PI&I_FLAG) && !_jammed)` block.
    state.regs.irq_low &= !IrqSource::TEMP.bits();
    0
}

/// Fetch one byte at PC and advance PC.
#[inline]
fn fetch<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {
    let pc = state.regs.pc;
    let op = state.rd(bus, pc);
    state.regs.pc = pc.wrapping_add(1);
    op
}

/// Branch: relative addressing with page-cross cycle penalty.
fn do_branch<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    cond: bool,
) -> u8 {
    let pc = state.regs.pc;
    let disp = state.rd(bus, pc) as i8 as i16 as u16;
    state.regs.pc = pc.wrapping_add(1);
    if !cond {
        return 0; // not taken: extra cycles = 0
    }
    let pre = state.regs.pc;
    let target = pre.wrapping_add(disp);
    state.regs.pc = target;
    // +1 for taking the branch, +1 more if page-cross.
    let mut extra = 1u8;
    if (pre ^ target) & 0x0100 != 0 {
        extra += 1;
    }
    extra
}

/// Step one instruction. Returns total *CPU* cycles consumed.
///
/// Per the C++ `X6502_RunDebug` loop, `step()` does both:
/// 1. Process any pending IRQ / NMI / RESET (no instruction executed).
/// 2. If the cycle budget isn't exhausted, fetch + execute the next
///    instruction.
///
/// If the IRQ dispatch consumed all remaining budget, returns the IRQ
/// cost (typically 7) without executing a follow-up instruction. If
/// the IRQ dispatch was a no-op (no pending IRQ), the function falls
/// through to the fetch+execute path and returns the instruction cost.
pub fn step<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {

    // IRQ / NMI / RESET dispatch at this boundary.
    //
    // The C++ `X6502_RunDebug` body has two paths:
    //   1. `_IRQlow` set → dispatch (RESET/NMI2/NMI/maskable IRQ) →
    //      check `_count <= 0` → if not exhausted, fall through and
    //      fetch+execute the next instruction.
    //   2. `_IRQlow` clear → just fetch+execute the next instruction.
    //
    // The Rust mirrors both paths. `dispatch_irq` returns 0 for RESET
    // and the NMI-fresh deferral (no cycles consumed by the dispatch
    // itself), and 7 for NMI / maskable IRQ (cycles consumed by the
    // push-and-jump sequence inside the dispatch). The follow-up
    // fetch+execute then runs for the instruction at the new PC —
    // whatever it is. For `nestest.nes` this is `JMP $C5F5` at the
    // reset vector, but for arbitrary ROMs it can be any opcode.
    let irq_cycles = dispatch_irq(state, bus);

    // C++ sets `_PI = _P` AFTER the IRQ dispatch (so the NEXT
    // iteration's maskable-IRQ check sees the post-dispatch I flag).
    // We mirror that here so dispatch_irq's I-flag set takes effect
    // immediately on the next boundary. Setting moo_pi BEFORE the
    // dispatch (as we did originally) is wrong — the C++ semantics
    // are "use the I flag from the END of the previous instruction".
    state.regs.moo_pi = state.regs.p;

    // Accumulate the dispatch cycle cost (0 or 7). Done BEFORE the
    // instruction's own cycles so the residual ordering matches the
    // C++ loop (`ADDCYC(7)` inside the dispatch, then `ADDCYC(CycTable)`
    // for the follow-up instruction). The `* 3` matches the C++
    // `CycTable * 48 = 3 * 16` per-instruction delta.
    if irq_cycles != 0 {
        state.regs.count = state.regs.count.saturating_add(dot(irq_cycles) * 3);
    }

    // Always fetch + execute exactly one instruction. PC has either
    // been left at the current PC (no dispatch) or moved to the
    // post-dispatch address (dispatch fired) by dispatch_irq.
    let opcode = fetch(state, bus);
    let op_info = info(opcode);

    let mut cycles = irq_cycles + op_info.base_cycles;

    match op_info.kind {
        OpKind::Jam => {
            // STP / KIL �?halt CPU, roll PC back. Per C++ 0x02 handler.
            state.regs.jammed = 1;
            state.regs.pc = state.regs.pc.wrapping_sub(1);
        }
        OpKind::NopRead => {
            // 1-byte NOP (Implied) or 2-byte read-NOP �?just consume operand.
            let _ = match op_info.mode {
                AddrMode::Implied => implied(state, bus),
                AddrMode::ZP => zp(state, bus),
                AddrMode::ZPX => zpx(state, bus),
                AddrMode::Abs => absolute(state, bus),
                AddrMode::AbsX => {
                    cycles += abs_x_read(state, bus).extra_cycles;
                    abs_x_read(state, bus)
                }
                _ => implied(state, bus),
            };
        }
        OpKind::NopReadWrite => {
            // RMW-style NOP �?read M, write it back unchanged. 2A (NOP).
            // The 0xDA / 0xFA / 0xEA NOPs are single-byte NOPs; this
            // kind is reserved for the 0x04 / 0x44 / 0x64 family that
            // does a true read+write (used by some test ROMs).
            let addr = zp(state, bus);
            let m = state.rd(bus, addr.addr);
            state.wr(bus, addr.addr, m);
        }
        OpKind::Register => {
            // TAX, TAY, TSX, TXA, TYA, TXS, PHA, PHP, PLA, PLP, INX, INY, DEX, DEY.
            do_register_op(state, bus, opcode, op_info.mode);
        }
        OpKind::Flag => {
            // CLC, SEC, CLD, SED, CLI, SEI, CLV.
            do_flag_op(state, opcode);
        }
        OpKind::Branch => {
            // BCC, BCS, BEQ, BNE, BMI, BPL, BVC, BVS.
            cycles += do_branch(state, bus, branch_cond(state, opcode));
        }
        OpKind::Load => {
            // LDA, LDX, LDY. The mode carries the operand:
            // * Imm.addr is the immediate byte value (not a memory address).
            // * Everything else: .addr is a memory address; read once more.
            let (mode_result, m) = match op_info.mode {
                AddrMode::Imm => {
                    let r = imm(state, bus);
                    let val = r.addr as u8; // immediate value, not a read
                    (r, val)
                }
                AddrMode::ZP => {
                    let r = zp(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::ZPX => {
                    let r = zpx(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::ZPY => {
                    let r = zpy(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::Abs => {
                    let r = absolute(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::AbsX => {
                    let r = abs_x_read(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::AbsY => {
                    let r = abs_y_read(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::IndX => {
                    let r = ind_x(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                AddrMode::IndY => {
                    let r = ind_y_read(state, bus);
                    let v = state.rd(bus, r.addr);
                    (r, v)
                }
                _ => unreachable!("Load with non-load mode: {:?}", op_info.mode),
            };
            cycles += mode_result.extra_cycles;
            let reg = load_reg_for_load(opcode);
            load_reg(state, bus, reg, m);
        }
        OpKind::Store => {
            // STA, STX, STY �?write the register to memory.
            // For STX/STY with ZP / ZP,Y / Abs we can use the same read
            // helper (it just reads the operand byte); the actual write
            // happens below.
            let mode_result = match op_info.mode {
                AddrMode::ZP => zp(state, bus),
                AddrMode::ZPX => zpx(state, bus),
                AddrMode::ZPY => zpy(state, bus),
                AddrMode::Abs => absolute(state, bus),
                AddrMode::AbsX => abs_x_write(state, bus),
                AddrMode::AbsY => abs_y_write(state, bus),
                AddrMode::IndX => ind_x(state, bus),
                AddrMode::IndY => ind_y_write(state, bus),
                _ => unreachable!("Store with non-store mode: {:?}", op_info.mode),
            };
            cycles += mode_result.extra_cycles;
            let v = store_reg(state, opcode);
            state.wr(bus, mode_result.addr, v);
        }
        OpKind::Rmw => {
            // ASL, ROL, LSR, ROR, INC, DEC �?memory RMW.
            cycles += do_rmw(state, bus, op_info.mode, opcode);
        }
        OpKind::AluA => {
            // ORA, AND, EOR, ADC, SBC (also covers LDA-style via Load).
            cycles += do_alu_a(state, bus, op_info.mode, opcode);
        }
        OpKind::Compare => {
            // CMP, CPX, CPY.
            cycles += do_compare(state, bus, op_info.mode, opcode);
        }
        OpKind::Bit => {
            // BIT.
            cycles += do_bit(state, bus, op_info.mode);
        }
        OpKind::Jump => {
            // JMP abs / ind, JSR, RTS, RTI, BRK.
            cycles += do_jump(state, bus, opcode, op_info.mode);
        }
        OpKind::Unofficial => {
            // All 109 unofficial opcodes.
            cycles += do_unofficial(state, bus, opcode, op_info.mode);
        }
    }

    // Add the instruction's own cycle cost (excluding the dispatch
    // portion, which was already added above). The `* 3` matches
    // C++ `CycTable * 48 = 3 * 16`.
    state.regs.count = state.regs.count.saturating_add(dot(cycles - irq_cycles) * 3);
    state.cycles_in_run = state.cycles_in_run.saturating_add((cycles - irq_cycles) as i32);
    cycles
}

// ---------------------------------------------------------------------------
// Per-opcode helpers. Each one updates CPU state for the matching
// opcode(s) and returns any extra cycles beyond the base.
// ---------------------------------------------------------------------------

fn do_register_op<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    opcode: u8,
    _mode: AddrMode,
) {
    match opcode {
        // TAX
        0xAA => {
            let v = state.regs.a;
            state.regs.x = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // TAY
        0xA8 => {
            let v = state.regs.a;
            state.regs.y = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // TSX
        0xBA => {
            let v = state.regs.s;
            state.regs.x = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // TXA
        0x8A => {
            let v = state.regs.x;
            state.regs.a = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // TYA
        0x98 => {
            let v = state.regs.y;
            state.regs.a = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // TXS �?does NOT update flags.
        0x9A => {
            state.regs.s = state.regs.x;
        }
        // PHA
        0x48 => {
            state.push(bus, state.regs.a);
        }
        // PHP
        0x08 => {
            // Push P with B|U set.
            let v = (state.regs.p | Flags::BREAK.bits() | Flags::UNUSED.bits()) & 0xFF;
            state.push(bus, v);
        }
        // PLA
        0x68 => {
            let v = state.pop(bus);
            state.regs.a = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // PLP
        0x28 => {
            // Pop P but ignore B (the 6502 always sets B to 0 on PLP �?it
            // cannot be cleared by software via this instruction).
            // Also clear U then OR U bit from popped value (some docs say
            // U stays set; the C++ implementation just assigns directly).
            let v = state.pop(bus);
            // Bits 4 and 5 are not affected by PLP �?keep them from P.
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits() | Flags::OVERFLOW.bits()
                | Flags::DECIMAL.bits() | Flags::IRQ_DIS.bits() | Flags::CARRY.bits());
            p |= v & !(Flags::BREAK.bits() | Flags::UNUSED.bits());
            state.regs.p = p;
        }
        // INX
        0xE8 => {
            let v = state.regs.x.wrapping_add(1);
            state.regs.x = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // INY
        0xC8 => {
            let v = state.regs.y.wrapping_add(1);
            state.regs.y = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // DEX
        0xCA => {
            let v = state.regs.x.wrapping_sub(1);
            state.regs.x = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        // DEY
        0x88 => {
            let v = state.regs.y.wrapping_sub(1);
            state.regs.y = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        _ => unreachable!("Register op ${:02X} not handled", opcode),
    }
}

#[inline]
fn zn_table_lookup(v: u8) -> u8 {
    crate::cpu::state::ZN_TABLE[v as usize]
}

fn do_flag_op(state: &mut CpuState, opcode: u8) {
    match opcode {
        // CLC
        0x18 => state.regs.p &= !Flags::CARRY.bits(),
        // SEC
        0x38 => state.regs.p |= Flags::CARRY.bits(),
        // CLD
        0xD8 => state.regs.p &= !Flags::DECIMAL.bits(),
        // SED
        0xF8 => state.regs.p |= Flags::DECIMAL.bits(),
        // CLI
        0x58 => state.regs.p &= !Flags::IRQ_DIS.bits(),
        // SEI
        0x78 => state.regs.p |= Flags::IRQ_DIS.bits(),
        // CLV
        0xB8 => state.regs.p &= !Flags::OVERFLOW.bits(),
        _ => unreachable!("Flag op ${:02X} not handled", opcode),
    }
}

fn branch_cond(state: &CpuState, opcode: u8) -> bool {
    let p = state.regs.p;
    match opcode {
        0x90 => p & Flags::CARRY.bits() == 0, // BCC
        0xB0 => p & Flags::CARRY.bits() != 0, // BCS
        0xF0 => p & Flags::ZERO.bits() != 0,  // BEQ
        0xD0 => p & Flags::ZERO.bits() == 0,  // BNE
        0x30 => p & Flags::NEGATIVE.bits() != 0, // BMI
        0x10 => p & Flags::NEGATIVE.bits() == 0, // BPL
        0x50 => p & Flags::OVERFLOW.bits() == 0, // BVC
        0x70 => p & Flags::OVERFLOW.bits() != 0, // BVS
        _ => unreachable!("Branch op ${:02X} not handled", opcode),
    }
}

fn load_reg_for_load(opcode: u8) -> LoadReg {
    match opcode {
        0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => LoadReg::A,
        0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => LoadReg::X,
        0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => LoadReg::Y,
        _ => unreachable!("Load reg opcode ${:02X}", opcode),
    }
}

fn store_reg(state: &CpuState, opcode: u8) -> u8 {
    match opcode {
        // STA family
        0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => state.regs.a,
        // STX family
        0x86 | 0x96 | 0x8E => state.regs.x,
        // STY family
        0x84 | 0x94 | 0x8C => state.regs.y,
        _ => unreachable!("Store reg opcode ${:02X}", opcode),
    }
}

fn do_rmw<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    mode: AddrMode,
    opcode: u8,
) -> u8 {
    let addr = match mode {
        AddrMode::ZP => zp(state, bus).addr,
        AddrMode::ZPX => zpx(state, bus).addr,
        AddrMode::Abs => absolute(state, bus).addr,
        AddrMode::AbsX => abs_x_write(state, bus).addr,
        AddrMode::AbsY => abs_y_write(state, bus).addr,
        AddrMode::IndX => ind_x(state, bus).addr,
        AddrMode::IndY => ind_y_write(state, bus).addr,
        AddrMode::Accum => {
            // ASL A / ROL A / LSR A / ROR A operate on the accumulator.
            let m = state.regs.a;
            let r = match opcode {
                0x0A => asl(state, m),
                0x2A => rol(state, m),
                0x4A => lsr(state, m),
                0x6A => ror(state, m),
                _ => unreachable!("Accumulator RMW ${:02X}", opcode),
            };
            state.regs.a = r;
            return 0;
        }
        _ => unreachable!("RMW mode {:?}", mode),
    };
    let m = state.rd(bus, addr);
    let r = match opcode {
        0x06 | 0x0E | 0x16 | 0x1E => asl(state, m),
        0x26 | 0x2E | 0x36 | 0x3E => rol(state, m),
        0x46 | 0x4E | 0x56 | 0x5E => lsr(state, m),
        0x66 | 0x6E | 0x76 | 0x7E => ror(state, m),
        0xC6 | 0xCE | 0xD6 | 0xDE => {
            // DEC
            let v = m.wrapping_sub(1);
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
            v
        }
        0xE6 | 0xEE | 0xF6 | 0xFE => {
            // INC
            let v = m.wrapping_add(1);
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
            v
        }
        _ => unreachable!("RMW opcode ${:02X}", opcode),
    };
    state.wr(bus, addr, r);
    0
}

fn do_alu_a<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    mode: AddrMode,
    opcode: u8,
) -> u8 {
    // Immediate mode: the value IS the byte at PC (no memory re-read).
    // All other modes: the addressing helper returns a memory address.
    let m = match mode {
        AddrMode::Imm => imm(state, bus).addr as u8,
        AddrMode::ZP => {
            let r = zp(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::ZPX => {
            let r = zpx(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::Abs => {
            let r = absolute(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::AbsX => {
            let r = abs_x_read(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::AbsY => {
            let r = abs_y_read(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::IndX => {
            let r = ind_x(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::IndY => {
            let r = ind_y_read(state, bus);
            state.rd(bus, r.addr)
        }
        _ => unreachable!("AluA mode {:?}", mode),
    };
    if matches!(opcode, 0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 | 0xEB) {
        sbc(state, bus, m);
    } else {
        match opcode {
            0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => ora(state, bus, m),
            0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => and(state, bus, m),
            0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => eor(state, bus, m),
            0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => adc(state, bus, m),
            _ => unreachable!("AluA opcode ${:02X}", opcode),
        }
    }
    0
}


fn do_compare<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    mode: AddrMode,
    opcode: u8,
) -> u8 {
    // Immediate mode: the value IS the byte at PC (no memory re-read).
    // All other modes: the addressing helper returns a memory address.
    let m = match mode {
        AddrMode::Imm => imm(state, bus).addr as u8,
        AddrMode::ZP => {
            let r = zp(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::ZPX => {
            let r = zpx(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::Abs => {
            let r = absolute(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::AbsX => {
            let r = abs_x_read(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::AbsY => {
            let r = abs_y_read(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::IndX => {
            let r = ind_x(state, bus);
            state.rd(bus, r.addr)
        }
        AddrMode::IndY => {
            let r = ind_y_read(state, bus);
            state.rd(bus, r.addr)
        }
        _ => unreachable!("Compare mode {:?}", mode),
    };
    let r = match opcode {
        0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => state.regs.a,
        0xE0 | 0xE4 | 0xEC => state.regs.x,
        0xC0 | 0xC4 | 0xCC => state.regs.y,
        _ => unreachable!("Compare opcode ${:02X}", opcode),
    };
    cmp(state, bus, r, m);
    0
}


fn do_bit<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B, mode: AddrMode) -> u8 {
    let addr = match mode {
        AddrMode::ZP => zp(state, bus).addr,
        AddrMode::Abs => absolute(state, bus).addr,
        _ => unreachable!("BIT mode {:?}", mode),
    };
    let m = state.rd(bus, addr);
    bit(state, bus, m);
    0
}

fn do_jump<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    opcode: u8,
    mode: AddrMode,
) -> u8 {
    match opcode {
        // JMP absolute ($4C).
        0x4C => {
            let r = absolute(state, bus);
            state.regs.pc = r.addr;
        }
        // JMP indirect ($6C) �?has the $xxFF page bug.
        0x6C => {
            let r = indirect(state, bus);
            state.regs.pc = r.addr;
        }
        // JSR absolute ($20).
        0x20 => {
            let pc = state.regs.pc;
            // Read the low byte of the target first.
            let lo = state.rd(bus, pc) as u16;
            // (replaced below)
            // Push PCH of PC+1 (the address after the JSR �?note +1 here
            // because the JSR has a 1-byte opcode followed by 2-byte
            // operand, and we already fetched the opcode; PC points at
            // the low operand byte).
            // The C++ implementation pushes PC after it has been
            // incremented past both operand bytes:
            //   PUSH(_PC >> 8); PUSH(_PC);
            //   _PC = npc;
            // but the operand reads advance PC inside GetAB. We mimic by
            // pushing PC+1 then reading the high byte.
            state.push(bus, ((pc + 1) >> 8) as u8);
            state.push(bus, (pc + 1) as u8);
            let hi = state.rd(bus, pc.wrapping_add(1)) as u16;
            state.regs.pc = (hi << 8) | lo;
        }
        // RTS ($60).
        0x60 => {
            let lo = state.pop(bus) as u16;
            let hi = state.pop(bus) as u16;
            state.regs.pc = ((hi << 8) | lo).wrapping_add(1);
        }
        // RTI ($40).
        0x40 => {
            let v = state.pop(bus);
            // Bits 4 (B) and 5 (U) are not settable by RTI.
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits() | Flags::OVERFLOW.bits()
                | Flags::DECIMAL.bits() | Flags::IRQ_DIS.bits() | Flags::CARRY.bits());
            p |= v & !(Flags::BREAK.bits() | Flags::UNUSED.bits());
            state.regs.p = p;
            let lo = state.pop(bus) as u16;
            let hi = state.pop(bus) as u16;
            state.regs.pc = (hi << 8) | lo;
        }
        // BRK ($00).
        0x00 => {
            let pc = state.regs.pc;
            // C++: _PC++; PUSH(_PC>>8); PUSH(_PC); PUSH(_P|U_FLAG|B_FLAG);
            state.push(bus, ((pc + 1) >> 8) as u8);
            state.push(bus, (pc + 1) as u8);
            let push_p = state.regs.p | Flags::UNUSED.bits() | Flags::BREAK.bits();
            state.push(bus, push_p);
            state.regs.p |= Flags::IRQ_DIS.bits();
            let lo = state.rd(bus, 0xFFFE);
            let hi = state.rd(bus, 0xFFFF);
            state.regs.pc = (hi as u16) << 8 | lo as u16;
        }
        _ => unreachable!("Jump op ${:02X} mode {:?}", opcode, mode),
    }
    0
}

fn do_unofficial<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    opcode: u8,
    mode: AddrMode,
) -> u8 {
    match opcode {
        // ----- read-NOPs (imm / zp / zpx / abs / absx) -----
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => {
            // Implied 1-byte NOPs.
            let _ = implied(state, bus);
        }
        0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
            // Immediate 2-byte NOPs (read & discard).
            let _ = imm(state, bus);
        }
        0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 | 0x0C
        | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
            // ZP / Abs / ZPX / AbsX read-NOPs (2 or 3 bytes).
            // (0x9C is excluded - it's SHY, handled separately below.)
            match mode {
                AddrMode::ZP => {
                    let _ = zp(state, bus);
                }
                AddrMode::ZPX => {
                    let _ = zpx(state, bus);
                }
                AddrMode::Abs => {
                    let _ = absolute(state, bus);
                }
                AddrMode::AbsX => {
                    let _ = abs_x_read(state, bus);
                }
                _ => unreachable!("NOP mode {:?}", mode),
            }
        }
        // ----- SLO / RLA / SRE / RRA (RMW then ALU) -----
        0x03 | 0x07 | 0x0F | 0x13 | 0x17 | 0x1B | 0x1F => {
            // SLO: M <<= 1; A |= M
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let shifted = asl(state, m);
            state.wr(bus, addr, shifted);
            ora(state, bus, shifted);
        }
        0x23 | 0x27 | 0x2F | 0x33 | 0x37 | 0x3B | 0x3F => {
            // RLA: M = ROL(M); A &= M
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let rotated = rol(state, m);
            state.wr(bus, addr, rotated);
            and(state, bus, rotated);
        }
        0x43 | 0x47 | 0x4F | 0x53 | 0x57 | 0x5B | 0x5F => {
            // SRE: M >>= 1; A ^= M
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let shifted = lsr(state, m);
            state.wr(bus, addr, shifted);
            eor(state, bus, shifted);
        }
        0x63 | 0x67 | 0x6F | 0x73 | 0x77 | 0x7B | 0x7F => {
            // RRA: M = ROR(M); A = ADC(A, M)
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let rotated = ror(state, m);
            state.wr(bus, addr, rotated);
            adc(state, bus, rotated);
        }
        // ----- SAX (store A & X) -----
        0x83 | 0x87 | 0x8F | 0x97 => {
            let addr = match mode {
                AddrMode::IndX => ind_x(state, bus).addr,
                AddrMode::ZP => zp(state, bus).addr,
                AddrMode::ZPY => zpy(state, bus).addr,
                AddrMode::Abs => absolute(state, bus).addr,
                _ => unreachable!("SAX mode {:?}", mode),
            };
            state.wr(bus, addr, state.regs.a & state.regs.x);
        }
        // ----- LAX (load A and X) -----
        0xA3 | 0xA7 | 0xAF | 0xB3 | 0xB7 | 0xBF | 0xAB => {
            let addr = match mode {
                AddrMode::Imm => imm(state, bus).addr,
                AddrMode::IndX => ind_x(state, bus).addr,
                AddrMode::ZP => zp(state, bus).addr,
                AddrMode::ZPY => zpy(state, bus).addr,
                AddrMode::Abs => absolute(state, bus).addr,
                AddrMode::IndY => ind_y_read(state, bus).addr,
                AddrMode::AbsY => abs_y_read(state, bus).addr,
                _ => unreachable!("LAX mode {:?}", mode),
            };
            let m = state.rd(bus, addr);
            state.regs.a = m;
            state.regs.x = m;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(m);
            state.regs.p = p;
        }
        // ----- DCP (DEC then CMP) -----
        0xC3 | 0xC7 | 0xCF | 0xD3 | 0xD7 | 0xDB | 0xDF => {
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let new = m.wrapping_sub(1);
            state.wr(bus, addr, new);
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(new);
            state.regs.p = p;
            let t = (state.regs.a as i16) - (new as i16);
            let result = (t & 0xFF) as u8;
            let mut p = state.regs.p;
            p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= (((t >> 8) & 1) as u8) ^ Flags::CARRY.bits();
            p |= zn_table_lookup(result);
            state.regs.p = p;
        }
        // ----- ISC / ISB (INC then SBC) -----
        0xE3 | 0xE7 | 0xEF | 0xF3 | 0xF7 | 0xFB | 0xFF => {
            let addr = rmw_addr(state, bus, mode);
            let m = state.rd(bus, addr);
            let new = m.wrapping_add(1);
            state.wr(bus, addr, new);
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(new);
            state.regs.p = p;
            sbc(state, bus, new);
        }
        // ----- ANC (AND imm, then copy bit 7 of A to C) -----
        0x0B | 0x2B => {
            let imm_addr = imm(state, bus).addr;
            let m = state.rd(bus, imm_addr);
            and(state, bus, m);
            // C = A >> 7.
            if state.regs.a & 0x80 != 0 {
                state.regs.p |= Flags::CARRY.bits();
            } else {
                state.regs.p &= !Flags::CARRY.bits();
            }
        }
        // ----- ALR (AND imm, then LSR A) -----
        0x4B => {
            let imm_addr = imm(state, bus).addr;
            let m = state.rd(bus, imm_addr);
            and(state, bus, m);
            let r = lsr(state, state.regs.a);
            state.regs.a = r;
        }
        // ----- ARR (AND imm, then ROR A, then special V/C) -----
        0x6B => {
            let imm_addr = imm(state, bus).addr;
            let m = state.rd(bus, imm_addr);
            and(state, bus, m);
            // ARR's behaviour depends on the state of the decimal flag and
            // is notoriously hard to capture exactly. The Visual6502 wiki
            // documents the binary (decimal-flag clear) case:
            //   A = (A >> 1) | (C << 7)
            //   C = A >> 6 ^ A >> 5 (i.e. bit 6 of the unrotated result)
            //   V = A >> 6 ^ A >> 5 (same XOR)
            let c_in = if state.regs.p & Flags::CARRY.bits() != 0 { 1 } else { 0 };
            let a = state.regs.a;
            let bit7 = a >> 7;
            let bit6 = (a >> 6) & 1;
            let bit5 = (a >> 5) & 1;
            let result = (a >> 1) | (c_in << 7);
            state.regs.a = result;
            // C = bit 6 of A before rotation (i.e. (a >> 6) & 1).
            let mut p = state.regs.p;
            p &= !(Flags::CARRY.bits() | Flags::OVERFLOW.bits() | Flags::ZERO.bits()
                | Flags::NEGATIVE.bits());
            p |= bit6;
            p |= (bit6 ^ bit5) << 6; // V
            p |= zn_table_lookup(result);
            state.regs.p = p;
            // Suppress unused warning for bit7.
            let _ = bit7;
        }
        // ----- XAA (TXA then AND imm) �?unstable.  Use TXA result.
        0x8B => {
            let tx = state.regs.x;
            state.regs.a = tx;
            let imm_addr = imm(state, bus).addr;
            let m = state.rd(bus, imm_addr);
            and(state, bus, m);
        }
        // ----- AXS (X = (A & X) - imm, sets NZC) -----
        0xCB => {
            let imm_addr = imm(state, bus).addr;
            let m = state.rd(bus, imm_addr);
            let t = ((state.regs.a & state.regs.x) as i16) - (m as i16);
            let result = (t & 0xFF) as u8;
            state.regs.x = result;
            let mut p = state.regs.p;
            p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= (((t >> 8) & 1) as u8) ^ Flags::CARRY.bits();
            p |= zn_table_lookup(result);
            state.regs.p = p;
        }
        // ----- AHX (AbsY / IndY): store A & X & (high byte of addr + 1) -----
        // Visual6502 captures: A & X & H, where H = high byte of the
        // **bank-determined** address (in NES this is always $01 for the
        // stack / $00 / $80 etc. depending on mapper). The published
        // behaviour for nestest testing uses H = (addr >> 8) + 1, but
        // some emulators use high byte of effective addr. We follow the
        // common Mesen2 / nestest-passing formula:
        0x93 | 0x9F => {
            let addr = match mode {
                AddrMode::IndY => ind_y_write(state, bus).addr,
                AddrMode::AbsY => abs_y_write(state, bus).addr,
                _ => unreachable!("AHX mode {:?}", mode),
            };
            let h = (addr >> 8).wrapping_add(1) as u8;
            state.wr(bus, addr, state.regs.a & state.regs.x & h);
        }
        // ----- SHX (AbsX): store X & (high byte of addr + 1) -----
        0x9E => {
            let addr = abs_x_write(state, bus).addr;
            let h = (addr >> 8).wrapping_add(1) as u8;
            state.wr(bus, addr, state.regs.x & h);
        }
        // ----- SHY (AbsX): store Y & (high byte of addr + 1) -----
        0x9C => {
            let addr = abs_x_write(state, bus).addr;
            let h = (addr >> 8).wrapping_add(1) as u8;
            state.wr(bus, addr, state.regs.y & h);
        }
        // ----- TAS (AbsY): S = A & X; store S & (high byte of addr + 1) -----
        0x9B => {
            let addr = abs_y_write(state, bus).addr;
            let s = state.regs.a & state.regs.x;
            state.regs.s = s;
            let h = (addr >> 8).wrapping_add(1) as u8;
            state.wr(bus, addr, s & h);
        }
        // ----- LAS (AbsY): A = X = S = M & S -----
        0xBB => {
            let addr = abs_y_read(state, bus).addr;
            let m = state.rd(bus, addr);
            let v = m & state.regs.s;
            state.regs.a = v;
            state.regs.x = v;
            state.regs.s = v;
            let mut p = state.regs.p;
            p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
            p |= zn_table_lookup(v);
            state.regs.p = p;
        }
        _ => unreachable!("Unofficial op ${:02X} mode {:?}", opcode, mode),
    }
    0
}

#[inline]
fn rmw_addr<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B, mode: AddrMode) -> u16 {
    match mode {
        AddrMode::ZP => zp(state, bus).addr,
        AddrMode::ZPX => zpx(state, bus).addr,
        AddrMode::Abs => absolute(state, bus).addr,
        AddrMode::AbsX => abs_x_write(state, bus).addr,
        AddrMode::AbsY => abs_y_write(state, bus).addr,
        AddrMode::IndX => ind_x(state, bus).addr,
        AddrMode::IndY => ind_y_write(state, bus).addr,
        _ => unreachable!("RMW mode {:?}", mode),
    }
}

// ---------------------------------------------------------------------------
// Top-level run loop. Mirrors `X6502_RunDebug`.
// ---------------------------------------------------------------------------

/// Run the CPU for `cycles` 1/16-dot units. Returns the total CPU
/// cycles consumed during this run (sum of every opcode's base cycle
/// cost plus page-cross / branch-taken extras). The C++ FFI shim
/// uses this to advance `Cpu::timestamp_` / `sound_timestamp_`,
/// which live on the Cpu object outside the 64-byte X6502 layout.
///
/// The C++ `X6502_RunDebug` always does at least one full loop-body
/// iteration per call (dispatch, fetch opcode, ADDCYC), even when
/// the cycle budget is zero — that's how the C++ tests that call
/// `X6502_Run(1)` consume exactly one instruction. The Rust mirrors
/// this by entering the loop at least once, then running until the
/// `count` cap is reached, then breaking on `[trapped]` (jammed) state.
pub fn run<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B, cycles: i32) -> i32 {
    let mut executed_cycles = 0i32;
    let target = state.regs.count.saturating_add(cycles);
    loop {
        let c = step(state, bus);
        executed_cycles = executed_cycles.saturating_add(c as i32);
        if state.regs.jammed != 0 {
            break;
        }
        if state.regs.count >= target {
            break;
        }
    }
    executed_cycles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::addressing::Bus;

    struct FlatBus {
        mem: [u8; 0x10000],
    }
    impl FlatBus {
        fn new() -> Self {
            Self { mem: [0; 0x10000] }
        }
    }
    impl Bus for FlatBus {
        fn read(&mut self, addr: u16) -> u8 {
            self.mem[addr as usize]
        }
        fn write(&mut self, addr: u16, val: u8) {
            self.mem[addr as usize] = val;
        }
    }

fn cpu_no_irq() -> CpuState {
        let mut s = CpuState::new();
        s.regs.s = 0xFD;
        s.regs.p = Flags::UNUSED.bits() | Flags::IRQ_DIS.bits();
        s
    }

        #[test]
    fn lda_imm_loads_a_and_sets_flags() {
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        bus.mem[0] = 0xA9;
        bus.mem[1] = 0x80;
        s.regs.pc = 0;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2);
        assert_eq!(s.regs.a, 0x80);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
        assert_eq!(s.regs.p & Flags::ZERO.bits(), 0);
    }

    #[test]
    fn sta_zp_writes_memory() {
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        s.regs.a = 0x42;
        bus.mem[0] = 0x85;
        bus.mem[1] = 0x10;
        s.regs.pc = 0;
        step(&mut s, &mut bus);
        assert_eq!(bus.mem[0x10], 0x42);
    }

    #[test]
    fn branch_taken_adds_cycle() {
        let mut s = CpuState::new();
        s.regs.p = Flags::ZERO.bits(); // BEQ will take
        let mut bus = FlatBus::new();
        bus.mem[0] = 0xF0;
        bus.mem[1] = 0x05; // forward 5
        s.regs.pc = 0;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2 + 1); // base 2 + taken
        assert_eq!(s.regs.pc, 7);
    }

    #[test]
    fn branch_not_taken_no_extra_cycle() {
        let mut s = CpuState::new();
        s.regs.p = 0; // BEQ will not take
        let mut bus = FlatBus::new();
        bus.mem[0] = 0xF0;
        bus.mem[1] = 0x05;
        s.regs.pc = 0;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2);
        assert_eq!(s.regs.pc, 2);
    }

    #[test]
        #[test]
    fn branch_page_cross_adds_two_cycles() {
        // BEQ at $10FE with offset $FF (-1). After fetch+offset, PC=$1100.
        // Target = $1100 + (-1) = $10FF (page $10), pre = $1100 (page $11)
        // so this IS a page cross, +1 cycle.
        let mut s = cpu_no_irq();
        s.regs.p = Flags::ZERO.bits();
        let mut bus = FlatBus::new();
        bus.mem[0x10FE] = 0xF0;
        bus.mem[0x10FF] = 0xFF;
        s.regs.pc = 0x10FE;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2 + 2); // base 2 + taken + page-cross
        assert_eq!(s.regs.pc, 0x10FF);
    }

    #[test]
    fn jsr_pushes_pc_and_jumps() {
        let mut s = CpuState::new();
        s.regs.s = 0xFD;
        let mut bus = FlatBus::new();
        bus.mem[0] = 0x20;
        bus.mem[1] = 0x00;
        bus.mem[2] = 0xC0;
        s.regs.pc = 0;
        step(&mut s, &mut bus);
        assert_eq!(s.regs.pc, 0xC000);
        assert_eq!(s.regs.s, 0xFB);
        assert_eq!(bus.mem[0x01FD], 0x00); // PCH
        assert_eq!(bus.mem[0x01FC], 0x02); // PCL (PC+1 = 2)
    }

    #[test]
    fn rts_returns() {
        let mut s = CpuState::new();
        s.regs.s = 0xFC;
        let mut bus = FlatBus::new();
        // RTS pops at 0xFD, 0xFE (after wrapping S++).
        bus.mem[0x01FD] = 0x34;
        bus.mem[0x01FE] = 0x12;
        s.regs.s = 0xFC;
        bus.mem[0] = 0x60;
        s.regs.pc = 0;
        step(&mut s, &mut bus);
        assert_eq!(s.regs.pc, 0x1235); // RTS adds 1 to popped address
    }

    #[test]
    fn inc_zp_increments_memory() {
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        bus.mem[0] = 0xE6;
        bus.mem[1] = 0x10;
        bus.mem[0x10] = 0xFE;
        s.regs.pc = 0;
        step(&mut s, &mut bus);
        assert_eq!(bus.mem[0x10], 0xFF);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
        assert_eq!(s.regs.p & Flags::ZERO.bits(), 0);
    }

    #[test]
    fn asl_accumulator_shifts_a() {
        let mut s = CpuState::new();
        s.regs.a = 0x40;
        let mut bus = FlatBus::new();
        bus.mem[0] = 0x0A;
        s.regs.pc = 0;
        step(&mut s, &mut bus);
        assert_eq!(s.regs.a, 0x80);
        assert_eq!(s.regs.p & Flags::CARRY.bits(), 0);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
    }
}
