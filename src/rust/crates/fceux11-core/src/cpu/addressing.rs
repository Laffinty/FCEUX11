//! 6502 addressing-mode implementations.
//!
//! Each mode is a pure function over the CPU state and a bus. Phase 1
//! builds out all 13 modes per the
//! [NESdev Wiki — CPU addressing modes](https://www.nesdev.org/wiki/CPU_addressing_modes)
//! reference; phase 2 will wire them into `execute.rs`.
//!
//! ## Function signature
//!
//! ```text
//! fn mode_xxx(state: &mut CpuState, bus: &mut dyn Bus) -> ModeResult
//! ```
//!
//! `ModeResult` carries the effective address (for branch / read /
//! write-modify targets) and any *extra* cycles the mode may impose
//! beyond the base cycle count (e.g. page-crossing on a,x / a,y / (d),y
//! reads, or the dummy-read cycle on store-indexed).
//!
//! Phase 1 uses a synchronous mock bus; the real [`Bus`] trait lives in
//! [`crate::traits`]. Phase 5 wires the FFI boundary against the real
//! C++ `g_bus` singleton.

use crate::cpu::state::X6502Layout;

/// CPU bus trait. For Phase 1 we only need reads/writes of single bytes
/// at 16-bit addresses; Phase 2 may add DMA hooks + open-bus fallbacks.
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);

    /// Synchronise the pending-IRQ bitmask from the host (C++) side
    /// into the Rust CPU state. The host's `X6502_IRQBegin` /
    /// `X6502_IRQEnd` mutate the C++ `X6502::IRQlow` blob (e.g. from
    /// mapper hooks and the APU frame-counter IRQ fired via the tick
    /// bridge), which the Rust state does NOT see because it was
    /// snapshotted at call start. Called at the top of every
    /// dispatch loop iteration before `dispatch_irq` reads
    /// `state.regs.irq_low`.
    ///
    /// Default: no-op (pure-Rust buses have no host side).
    fn sync_irq_from_host(&mut self, _state: &mut CpuState) {}

    /// Push the Rust CPU state's pending-IRQ bitmask back to the host
    /// (C++) side after dispatch has consumed bits (e.g. NMI). Without
    /// this the C++ blob keeps a stale `IRQlow` and the next call's
    /// snapshot re-introduces already-consumed interrupts.
    ///
    /// Default: no-op.
    fn sync_irq_to_host(&mut self, _state: &mut CpuState) {}
}

/// CPU state wrapper that carries the [`X6502Layout`] together with
/// mutable side-channels (cycle counter, NMI-freshness flag). Phase 1
/// keeps everything inline; Phase 5 will split this into a thin FFI
/// handle around `fceu11::Cpu`.
#[derive(Copy, Clone, Debug, Default)]
pub struct CpuState {
    pub regs: X6502Layout,
    /// `tcount` mirror. We keep both `regs.tcount` (savestate-compatible)
    /// and this to avoid `&mut` issues when the dispatch helper wants to
    /// observe the cycle residual.
    pub nmi_fresh: bool,
    /// Total CPU cycles consumed since the last reset of this
    /// accumulator. The FFI shim zeroes it at the start of
    /// `fceux11_cpu_run` and reads it at the end so the C++ caller
    /// can advance `Cpu::timestamp_` / `sound_timestamp_` without
    /// having to know about the 1/16-dot unit semantics.
    pub cycles_in_run: i32,
}

impl CpuState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply the post-power-on state (mirrors `X6502_Power`).
    pub fn power(&mut self) {
        self.regs = X6502Layout::zeroed();
        self.regs.s = 0xFD;
        self.regs.irq_low = super::state::IrqSource::RESET.bits();
        self.nmi_fresh = false;
    }

    /// Read a byte via the bus and cache it into `db` (matches
    /// `RdMem` / `RdRAM` side-effect in the C++ code).
    #[inline]
    pub fn rd<B: Bus + ?Sized>(&mut self, bus: &mut B, addr: u16) -> u8 {
        let v = bus.read(addr);
        self.regs.db = v;
        v
    }

    /// Write a byte via the bus and cache the value into `db` (matches
    /// `WrMem` / `WrRAM` side-effect).
    #[inline]
    pub fn wr<B: Bus + ?Sized>(&mut self, bus: &mut B, addr: u16, val: u8) {
        bus.write(addr, val);
        self.regs.db = val;
    }

    /// Push a byte on the hardware stack at `$0100 + S`.
    #[inline]
    pub fn push<B: Bus + ?Sized>(&mut self, bus: &mut B, val: u8) {
        self.wr(bus, 0x0100 | self.regs.s as u16, val);
        self.regs.s = self.regs.s.wrapping_sub(1);
    }

    /// Pop a byte from the hardware stack at `$0100 + S + 1`.
    #[inline]
    pub fn pop<B: Bus + ?Sized>(&mut self, bus: &mut B) -> u8 {
        self.regs.s = self.regs.s.wrapping_add(1);
        self.rd(bus, 0x0100 | self.regs.s as u16)
    }
}

/// Outcome of an addressing-mode decode. `addr` is the effective address
/// (only meaningful for `Abs`, `AbsX`, `AbsY`, `Ind`, etc.; for `Imm`
/// it carries the raw fetched value, for `Rel` it carries the absolute
/// branch target). `extra_cycles` is *added to* `CycTable[opcode]`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ModeResult {
    pub addr: u16,
    /// Extra CPU cycles imposed by the addressing mode beyond the base
    /// `CycTable` value. 0 for modes that don't add cycles; 1 for
    /// page-cross / dummy-read penalty.
    pub extra_cycles: u8,
}

/// All 13 addressing modes enumerated. The numeric values are an
/// internal ordering — they don't appear on the bus and don't need to
/// match the C++ `optype` table.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AddrMode {
    Implied = 0,
    Accum = 1,
    Imm = 2,
    ZP = 3,
    ZPX = 4,
    ZPY = 5,
    Abs = 6,
    AbsX = 7,
    AbsY = 8,
    Rel = 9,
    Ind = 10,
    /// (Indirect,X) — Indexed Indirect
    IndX = 11,
    /// (Indirect),Y — Indirect Indexed
    IndY = 12,
}

impl AddrMode {
    /// Human-readable assembly-syntax abbreviation (NESdev convention).
    pub const fn as_str(self) -> &'static str {
        match self {
            AddrMode::Implied => "",
            AddrMode::Accum => "A",
            AddrMode::Imm => "#",
            AddrMode::ZP => "d",
            AddrMode::ZPX => "d,x",
            AddrMode::ZPY => "d,y",
            AddrMode::Abs => "a",
            AddrMode::AbsX => "a,x",
            AddrMode::AbsY => "a,y",
            AddrMode::Rel => "r",
            AddrMode::Ind => "(a)",
            AddrMode::IndX => "(d,x)",
            AddrMode::IndY => "(d),y",
        }
    }
}

// ---------------------------------------------------------------------------
// Addressing-mode implementations. Each function follows the same shape as
// the corresponding `GetXXX` macro in `x6502.cpp`. The implementation
// translates the macro line-by-line; do not "improve" it without also
// checking the C++ side.
// ---------------------------------------------------------------------------

/// Implied / accumulator / branch: no operand fetch (or the operand is
/// the A register / a relative offset). Returns `addr = 0`,
/// `extra_cycles = 0`. The branch / accumulator logic lives in the
/// caller (`execute.rs`); this function only consumes operand bytes
/// for branches.
#[inline]
pub fn implied<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let _ = (s, bus);
    ModeResult::default()
}

/// Immediate: the next byte **is** the operand. Stores the byte in
/// `addr` so the caller can read it.
#[inline]
pub fn imm<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let v = s.rd(bus, pc);
    s.regs.pc = pc.wrapping_add(1);
    ModeResult {
        addr: v as u16,
        extra_cycles: 0,
    }
}

/// Zero Page.
#[inline]
pub fn zp<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    s.regs.pc = pc.wrapping_add(1);
    let a = s.rd(bus, pc);
    ModeResult {
        addr: a as u16,
        extra_cycles: 0,
    }
}

/// Zero Page,X — index wraps within the zero page (mod 256).
#[inline]
pub fn zpx<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    s.regs.pc = pc.wrapping_add(1);
    let a = s.rd(bus, pc).wrapping_add(s.regs.x);
    ModeResult {
        addr: a as u16,
        extra_cycles: 0,
    }
}

/// Zero Page,Y — index wraps within the zero page (mod 256).
#[inline]
pub fn zpy<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    s.regs.pc = pc.wrapping_add(1);
    let a = s.rd(bus, pc).wrapping_add(s.regs.y);
    ModeResult {
        addr: a as u16,
        extra_cycles: 0,
    }
}

/// Absolute.
#[inline]
pub fn absolute<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    ModeResult {
        addr: (hi << 8) | lo,
        extra_cycles: 0,
    }
}

/// Absolute,X — adds 1 cycle if the page crosses (read path).
///
/// Mirrors `GetABIRD(target, i)` in `x6502.cpp` exactly: it computes the
/// target by adding `i`, then if the high byte changed (carry-out from
/// the low-byte add), it does a dummy read at the pre-carry target and
/// adds 1 cycle.
#[inline]
pub fn abs_x_read<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    let base = (hi << 8) | lo;
    let target = base.wrapping_add(s.regs.x as u16);
    let extra = if (target ^ base) & 0x0100 != 0 {
        let _ = s.rd(bus, target ^ 0x0100);
        1
    } else {
        0
    };
    ModeResult {
        addr: target,
        extra_cycles: extra,
    }
}

/// Absolute,Y — same as AbsX but indexed by Y.
#[inline]
pub fn abs_y_read<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    let base = (hi << 8) | lo;
    let target = base.wrapping_add(s.regs.y as u16);
    let extra = if (target ^ base) & 0x0100 != 0 {
        let _ = s.rd(bus, target ^ 0x0100);
        1
    } else {
        0
    };
    ModeResult {
        addr: target,
        extra_cycles: extra,
    }
}

/// Absolute,X — write / RMW variant. Always does a dummy read at the
/// partially-added address before the final access.
#[inline]
pub fn abs_x_write<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    let base = (hi << 8) | lo;
    let target = base.wrapping_add(s.regs.x as u16) & 0xFFFF;
    let _ = s.rd(bus, (target & 0x00FF) | (base & 0xFF00));
    ModeResult {
        addr: target,
        extra_cycles: 0,
    }
}

/// Absolute,Y — write / RMW variant.
#[inline]
pub fn abs_y_write<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    let base = (hi << 8) | lo;
    let target = base.wrapping_add(s.regs.y as u16) & 0xFFFF;
    let _ = s.rd(bus, (target & 0x00FF) | (base & 0xFF00));
    ModeResult {
        addr: target,
        extra_cycles: 0,
    }
}

/// Relative (branch). Fetches the signed 8-bit offset and computes the
/// absolute branch target as `(PC_after_offset + offset) & 0xFFFF`. The
/// branch condition itself is evaluated by the caller.
#[inline]
pub fn relative<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let off = s.rd(bus, pc) as i8 as i16 as u16;
    s.regs.pc = pc.wrapping_add(1);
    let target = s.regs.pc.wrapping_add(off);
    // Branch base cycle already includes 1 cycle for the offset fetch.
    // `extra_cycles = 1` is added by the caller when the branch is taken
    // AND the page crosses — see NESdev branch_timing_tests.
    ModeResult {
        addr: target,
        extra_cycles: 0,
    }
}

/// Indirect (`JMP (addr)`). The 6502 has a famous page-boundary bug at
/// `$xxFF`: the high byte is read from `$xx00` instead of `$(xx+1)00`.
/// Mirrors `case 0x6C` in `x6502.cpp` exactly.
#[inline]
pub fn indirect<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let lo = s.rd(bus, pc) as u16;
    let hi = s.rd(bus, pc.wrapping_add(1)) as u16;
    s.regs.pc = pc.wrapping_add(2);
    let ptr = (hi << 8) | lo;
    let target_lo = s.rd(bus, ptr) as u16;
    // Bug: if low byte is $FF, the high byte is read from $xx00 not $(xx+1)00.
    let target_hi_addr = if lo == 0xFF { ptr & 0xFF00 } else { ptr.wrapping_add(1) };
    let target_hi = s.rd(bus, target_hi_addr) as u16;
    ModeResult {
        addr: (target_hi << 8) | target_lo,
        extra_cycles: 0,
    }
}

/// (Indirect,X) — Indexed Indirect. Reads a zero-page address, adds X
/// (with zero-page wrap), then reads two bytes to form the effective
/// address. The pointer reads go via the **internal** zero-page bus
/// (i.e. `RdRAM`), matching `GetIX` in `x6502.cpp`.
#[inline]
pub fn ind_x<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let tmp = s.rd(bus, pc).wrapping_add(s.regs.x);
    s.regs.pc = pc.wrapping_add(1);
    let lo = s.rd(bus, tmp as u16);
    let hi = s.rd(bus, tmp.wrapping_add(1) as u16);
    ModeResult {
        addr: ((hi as u16) << 8) | lo as u16,
        extra_cycles: 0,
    }
}

/// (Indirect),Y — Indirect Indexed, read path. Adds 1 cycle on page cross.
#[inline]
pub fn ind_y_read<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let tmp = s.rd(bus, pc);
    s.regs.pc = pc.wrapping_add(1);
    let lo = s.rd(bus, tmp as u16);
    let hi = s.rd(bus, tmp.wrapping_add(1) as u16);
    let base = ((hi as u16) << 8) | lo as u16;
    let target = base.wrapping_add(s.regs.y as u16);
    let extra = if (target ^ base) & 0x0100 != 0 {
        let _ = s.rd(bus, target ^ 0x0100);
        1
    } else {
        0
    };
    ModeResult {
        addr: target,
        extra_cycles: extra,
    }
}

/// (Indirect),Y — write / RMW variant.
#[inline]
pub fn ind_y_write<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> ModeResult {
    let pc = s.regs.pc;
    let tmp = s.rd(bus, pc);
    s.regs.pc = pc.wrapping_add(1);
    let lo = s.rd(bus, tmp as u16);
    let hi = s.rd(bus, tmp.wrapping_add(1) as u16);
    let base = ((hi as u16) << 8) | lo as u16;
    let target = base.wrapping_add(s.regs.y as u16) & 0xFFFF;
    let _ = s.rd(bus, (target & 0x00FF) | (base & 0xFF00));
    ModeResult {
        addr: target,
        extra_cycles: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial bus: addressable 64 KiB + open-bus zero on out-of-range
    /// writes. Phase 2 swaps in the real `Bus` trait from
    /// `crate::traits`.
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

    fn setup(code: &[u8], load_addr: u16) -> (CpuState, FlatBus) {
        let mut bus = FlatBus::new();
        for (i, b) in code.iter().enumerate() {
            bus.mem[(load_addr as usize).wrapping_add(i) & 0xFFFF] = *b;
        }
        let mut s = CpuState::new();
        s.regs.pc = load_addr;
        (s, bus)
    }

    #[test]
    fn imm_reads_next_byte_and_advances_pc() {
        // imm() expects the *immediate operand* to be the byte at PC.
        // (The opcode itself is consumed by fetch() upstream of imm().)
        let (mut s, mut bus) = setup(&[0x42], 0x8000);
        let r = imm(&mut s, &mut bus);
        assert_eq!(r.addr, 0x42);
        assert_eq!(s.regs.pc, 0x8001);
        assert_eq!(s.regs.db, 0x42);
    }

    #[test]
    fn zp_reads_zero_page_address() {
        let (mut s, mut bus) = setup(&[0x10], 0x8000);
        bus.mem[0x10] = 0xAB;
        let r = zp(&mut s, &mut bus);
        assert_eq!(r.addr, 0x10);
        assert_eq!(r.extra_cycles, 0);
        assert_eq!(s.regs.pc, 0x8001);
    }

    #[test]
    fn zpx_wraps_in_zero_page() {
        // 0xFF + 0x05 should wrap to 0x04 (zero-page wrap).
        let (mut s, mut bus) = setup(&[0xFF], 0x8000);
        s.regs.x = 0x05;
        let r = zpx(&mut s, &mut bus);
        assert_eq!(r.addr, 0x04);
    }

    #[test]
    fn abs_x_read_adds_cycle_on_page_cross() {
        // base $10FF, X = $01 → target $1100 (page cross), +1 cycle.
        let (mut s, mut bus) = setup(&[0xFF, 0x10], 0x8000);
        s.regs.x = 0x01;
        let r = abs_x_read(&mut s, &mut bus);
        assert_eq!(r.addr, 0x1100);
        assert_eq!(r.extra_cycles, 1);
    }

    #[test]
    fn abs_x_read_no_cycle_on_same_page() {
        // base $1002, X = $05 → target $1007 (no page cross), +0 cycle.
        let (mut s, mut bus) = setup(&[0x02, 0x10], 0x8000);
        s.regs.x = 0x05;
        let r = abs_x_read(&mut s, &mut bus);
        assert_eq!(r.addr, 0x1007);
        assert_eq!(r.extra_cycles, 0);
    }

    #[test]
    fn ind_x_uses_zero_page_wrap() {
        // zero-page pointer $80 + X=$FF → $7F (wrap) → address at $7F/$80.
        let (mut s, mut bus) = setup(&[0x80], 0x8000);
        s.regs.x = 0xFF;
        bus.mem[0x7F] = 0x34;
        bus.mem[0x80] = 0x12;
        let r = ind_x(&mut s, &mut bus);
        assert_eq!(r.addr, 0x1234);
    }

    #[test]
    fn indirect_jmp_has_page_bug() {
        // JMP ($10FF) reads the high byte from $1000 (page bug) instead of $1100.
        let (mut s, mut bus) = setup(&[0xFF, 0x10], 0x8000);
        bus.mem[0x10FF] = 0x78;
        bus.mem[0x1000] = 0x56; // what gets read for the high byte
        bus.mem[0x1100] = 0xAB; // would be read if the bug were absent
        let r = indirect(&mut s, &mut bus);
        assert_eq!(r.addr, 0x5678); // bug: 1000 not 1100
    }

    #[test]
    fn indirect_jmp_no_bug_when_lo_not_ff() {
        let (mut s, mut bus) = setup(&[0xFE, 0x10], 0x8000);
        bus.mem[0x10FE] = 0x78;
        bus.mem[0x10FF] = 0x56;
        let r = indirect(&mut s, &mut bus);
        assert_eq!(r.addr, 0x5678);
    }

    #[test]
    fn push_pop_roundtrip() {
        let (mut s, mut bus) = setup(&[], 0x8000);
        s.regs.s = 0xFD;
        s.push(&mut bus, 0xA5);
        assert_eq!(s.regs.s, 0xFC);
        assert_eq!(bus.mem[0x01FD], 0xA5);
        let v = s.pop(&mut bus);
        assert_eq!(v, 0xA5);
        assert_eq!(s.regs.s, 0xFD);
    }

    #[test]
    fn all_modes_have_a_str() {
        for m in [
            AddrMode::Implied,
            AddrMode::Accum,
            AddrMode::Imm,
            AddrMode::ZP,
            AddrMode::ZPX,
            AddrMode::ZPY,
            AddrMode::Abs,
            AddrMode::AbsX,
            AddrMode::AbsY,
            AddrMode::Rel,
            AddrMode::Ind,
            AddrMode::IndX,
            AddrMode::IndY,
        ] {
            // Just exercise the function — coverage of all variants.
            let _ = m.as_str();
        }
    }
}