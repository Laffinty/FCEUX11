//! Savestate-compatible snapshot / restore helpers (Phase 5).
//!
//! The 64-byte `X6502Layout` is the savestate contract with the C++
//! `X6502` struct (pinned by `state.rs`'s `offset_of!` asserts and the
//! C++ `static_assert`s in `src/cpu.cpp`). These helpers serialize and
//! deserialize that blob so CPU state can be captured, stored and
//! resumed byte-for-byte - the same semantics as the FFI
//! `fceux11_cpu_snapshot` / `fceux11_cpu_restore` (pure
//! `copy_nonoverlapping`); [`tests::snapshot_matches_ffi_copy_semantics`]
//! cross-checks the two paths.

use crate::cpu::addressing::CpuState;
use crate::cpu::state::X6502Layout;

/// Size of the savestate blob (must stay 64).
pub const SNAPSHOT_SIZE: usize = core::mem::size_of::<X6502Layout>();

const _: () = assert!(SNAPSHOT_SIZE == 64, "X6502Layout must stay 64 bytes");

impl CpuState {
    /// Serialize the 64-byte CPU register blob (the savestate payload).
    pub fn snapshot_bytes(&self) -> [u8; SNAPSHOT_SIZE] {
        let mut out = [0u8; SNAPSHOT_SIZE];
        unsafe {
            core::ptr::copy_nonoverlapping(
                &self.regs as *const X6502Layout as *const u8,
                out.as_mut_ptr(),
                SNAPSHOT_SIZE,
            );
        }
        out
    }

    /// Overwrite the CPU register blob from a snapshot. Side channels
    /// (`nmi_fresh`) are cleared, matching `fceux11_cpu_restore`.
    pub fn restore_bytes(&mut self, data: &[u8; SNAPSHOT_SIZE]) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                &mut self.regs as *mut X6502Layout as *mut u8,
                SNAPSHOT_SIZE,
            );
        }
        self.nmi_fresh = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::addressing::Bus;
    use crate::cpu::state::{Flags, IrqSource};

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

    fn cpu_at(pc: u16) -> CpuState {
        let mut cpu = CpuState::new();
        cpu.regs.pc = pc;
        cpu.regs.s = 0xFD;
        cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
        cpu.regs.moo_pi = cpu.regs.p;
        cpu
    }

    #[test]
    fn snapshot_size_is_64() {
        assert_eq!(SNAPSHOT_SIZE, 64);
    }

    #[test]
    fn round_trip_restores_registers_byte_for_byte() {
        let mut cpu = cpu_at(0x4321);
        cpu.regs.a = 0xAB;
        cpu.regs.x = 0xCD;
        cpu.regs.y = 0xEF;
        cpu.regs.p = Flags::IRQ_DIS.bits();
        cpu.regs.moo_pi = 0x24;
        cpu.regs.count = 12345;
        cpu.regs.irq_low = IrqSource::NMI.bits();
        cpu.regs.db = 0x7F;
        cpu.nmi_fresh = true;

        let snap = cpu.snapshot_bytes();

        // Mutate everything, then restore.
        cpu.regs = X6502Layout::zeroed();
        cpu.nmi_fresh = true;
        cpu.restore_bytes(&snap);

        assert_eq!(cpu.regs.pc, 0x4321);
        assert_eq!(cpu.regs.a, 0xAB);
        assert_eq!(cpu.regs.x, 0xCD);
        assert_eq!(cpu.regs.y, 0xEF);
        assert_eq!(cpu.regs.s, 0xFD);
        assert_eq!(cpu.regs.p, Flags::IRQ_DIS.bits());
        assert_eq!(cpu.regs.moo_pi, 0x24);
        assert_eq!(cpu.regs.count, 12345);
        assert_eq!(cpu.regs.irq_low, IrqSource::NMI.bits());
        assert_eq!(cpu.regs.db, 0x7F);
        assert!(!cpu.nmi_fresh, "restore clears the NMI-fresh side flag");
    }

    #[test]
    fn snapshot_matches_ffi_copy_semantics() {
        // The FFI snapshot is a pure 64-byte copy; our helper must
        // produce identical bytes for the same layout.
        let mut cpu = cpu_at(0xC000);
        cpu.regs.a = 0x42;
        cpu.regs.count = -112i32;
        let bytes = cpu.snapshot_bytes();
        let layout = cpu.regs;
        let mut via_ptr = [0u8; 64];
        unsafe {
            core::ptr::copy_nonoverlapping(
                &layout as *const X6502Layout as *const u8,
                via_ptr.as_mut_ptr(),
                64,
            );
        }
        assert_eq!(bytes, via_ptr);
    }

    #[test]
    fn restore_resumes_execution_deterministically() {
        // NOP sled: interrupt a run with a snapshot + restore and verify
        // the restored CPU continues exactly as an uninterrupted run.
        fn step_n(mut cpu: CpuState, n: usize, bus: &mut FlatBus) -> CpuState {
            for _ in 0..n {
                crate::cpu::step(&mut cpu, bus);
            }
            cpu
        }

        let mut bus = FlatBus::new();
        bus.mem[0x4000..0x4020].fill(0xEA); // NOP sled

        let base = cpu_at(0x4000);
        let after4 = step_n(base, 4, &mut bus);
        let snap = after4.snapshot_bytes();

        // Run 2 more steps (state moves on), then restore the snapshot.
        let mut interrupted = step_n(after4, 2, &mut bus);
        interrupted.restore_bytes(&snap);

        // One more step from the restored state...
        let resumed = step_n(interrupted, 1, &mut bus);

        // ...must equal an uninterrupted run of 5 steps.
        let fresh = step_n(base, 5, &mut bus);
        assert_eq!(resumed.regs.pc, fresh.regs.pc);
        assert_eq!(resumed.regs.s, fresh.regs.s);
        assert_eq!(resumed.regs.p, fresh.regs.p);
        assert_eq!(resumed.snapshot_bytes(), fresh.snapshot_bytes());
    }
}
